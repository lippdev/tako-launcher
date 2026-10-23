//! Perfil do modo desktop (TelaPerfil, UX-004 / SESSION-002): a identidade do perfil em uso, as guias Visão geral ·
//! Sessões · Coleções e os blocos com os números que o Tako registra (sessões e tempo de jogo). A guia Família
//! (limites por perfil) e o bloco Conquistas ficam de fora até existir o que os alimenta: sem dado inventado.

use super::desktop::{self, THUMB};
use super::{Cmd, Ctx, Nav, Out, Page, dialogs, keys, profiles, smart, tiles};
use crate::model::{DeskBar, DeskCard, DeskEntry, DeskGameView, DeskInfo, DeskProfile, Hint};
use crate::values::parse_color;
use chrono::{Datelike, Local, NaiveDate};
use std::time::Duration;
use tako_core::i18n;
use tako_core::input::shortcut::KeyModifiers;
use tako_core::library::{Game, LibraryCriteria, LibraryFilter, PlaySession, query};
use tako_core::presentation::game_text;

pub const ROUTE: &str = "desk:profile";
pub const KIND: &str = "desk-profile";

/// Sessão com menos de 1 minuto: "Abriu e fechou", fora dos totais (PerfilSessoes).
const MIN_SESSION: Duration = Duration::from_secs(60);
/// Sessões recentes na Visão geral.
const RECENT: usize = 5;
// shortcut: a guia Sessões mostra as 300 mais recentes do período, sem carregar mais ao rolar; virtualizar se faltar.
const MAX_ROWS: usize = 300;
/// Colunas da grade de coleções (as setas ↑ ↓ andam por elas).
const COLUMNS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Overview,
    Sessions,
    Collections,
}

const TABS: [Tab; 3] = [Tab::Overview, Tab::Sessions, Tab::Collections];

/// Período da guia Sessões (Esta semana · Este mês · Tudo).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    Week,
    Month,
    All,
}

const PERIODS: [Period; 3] = [Period::Week, Period::Month, Period::All];

/// Onde está o foco do teclado (a ordem do Tab segue a leitura: barra → guias → ações → blocos → lista).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    Bar,
    Tabs,
    Actions,
    Tiles,
    Periods,
    List,
}

/// Ações em ícone: Trocar perfil e Editar perfil (Perfis e família só com os limites).
const ACTIONS: usize = 2;
/// Blocos da Visão geral: Tempo jogado, Mais jogado, Jogos na biblioteca.
const TILES: usize = 3;

/// Segunda-feira da semana de `day`.
fn monday(day: NaiveDate) -> NaiveDate {
    day - chrono::Days::new(u64::from(day.weekday().num_days_from_monday()))
}

fn local_day(s: &PlaySession) -> NaiveDate {
    s.started_at.with_timezone(&Local).date_naive()
}

/// O tempo que conta nos totais (sessão de menos de 1 minuto não soma).
fn counted(s: &PlaySession) -> Duration {
    if s.duration < MIN_SESSION {
        Duration::ZERO
    } else {
        s.duration
    }
}

/// Tempo de cada dia da semana de `today`, de segunda a domingo.
pub fn week<'a>(
    sessions: impl IntoIterator<Item = &'a PlaySession>,
    today: NaiveDate,
) -> [Duration; 7] {
    let start = monday(today);
    let mut days = [Duration::ZERO; 7];
    for s in sessions {
        let i = (local_day(s) - start).num_days();
        if (0..7).contains(&i) {
            days[i as usize] += counted(s);
        }
    }
    days
}

/// O jogo com mais tempo nas sessões desde `since` (o fundo é o mais jogado do mês); empate fica com o primeiro.
pub fn top_since<'a>(
    played: impl IntoIterator<Item = (&'a str, &'a PlaySession)>,
    since: NaiveDate,
) -> Option<&'a str> {
    let mut totals: Vec<(&str, Duration)> = Vec::new();
    for (id, s) in played {
        if local_day(s) < since || counted(s).is_zero() {
            continue;
        }
        match totals.iter_mut().find(|(g, _)| *g == id) {
            Some((_, t)) => *t += s.duration,
            None => totals.push((id, s.duration)),
        }
    }
    totals
        .into_iter()
        .fold(None, |best: Option<(&str, Duration)>, (id, t)| match best {
            Some((_, b)) if b >= t => best,
            _ => Some((id, t)),
        })
        .map(|(id, _)| id)
}

/// "42 min", "2 h", "6 h 37 min".
fn hm(d: Duration) -> String {
    let m = d.as_secs() / 60;
    match (m / 60, m % 60) {
        (0, m) => i18n::tf("profile.duration.minutes", &[("m", &m.to_string())]),
        (h, 0) => i18n::tf("profile.duration.hours", &[("h", &h.to_string())]),
        (h, m) => i18n::tf(
            "profile.duration.hours_minutes",
            &[("h", &h.to_string()), ("m", &format!("{m:02}"))],
        ),
    }
}

const WEEKDAYS: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

/// Todas as sessões do perfil (o histórico é por jogo), da mais recente para a mais antiga.
fn all_sessions(ctx: &Ctx) -> Vec<(&Game, &PlaySession)> {
    let mut v: Vec<(&Game, &PlaySession)> = ctx
        .library
        .games()
        .iter()
        .flat_map(|g| ctx.library.sessions(&g.id).iter().map(move |s| (g, s)))
        .collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.1.started_at));
    v
}

fn thumb(g: &Game) -> slint::Image {
    tiles::image(g.shown_cover(), THUMB)
}

/// Linha de sessão: capa, jogo, quando (ou o horário), o total do jogo e a duração.
fn session_row(ctx: &Ctx, g: &Game, s: &PlaySession, overview: bool) -> DeskEntry {
    let start = s.started_at.with_timezone(&Local);
    let end = start + chrono::Duration::from_std(s.duration).unwrap_or_default();
    let detail = if overview {
        format!("{}, {}", desktop::day(ctx, start), start.format("%H:%M"))
    } else {
        format!("{} – {}", start.format("%H:%M"), end.format("%H:%M"))
    };
    DeskEntry {
        id: g.id.clone(),
        title: g.title.clone(),
        detail,
        note: if overview && !g.play_time.is_zero() {
            ctx.tf(
                "profile.in_game",
                &[("time", &game_text::play_time(ctx.lang(), g.play_time))],
            )
        } else {
            String::new()
        },
        value: if s.duration < MIN_SESSION {
            ctx.t("profile.quick")
        } else {
            hm(s.duration)
        },
        thumb: thumb(g),
        accent: parse_color(&g.accent),
        ..Default::default()
    }
}

/// A sessão aberta agora (ponto verde, "Agora · desde 14:02").
fn running_row(ctx: &Ctx) -> Option<DeskEntry> {
    let r = ctx.running.as_ref()?;
    let g = ctx.library.find(&r.id)?;
    let elapsed = r.started.map(|t| t.elapsed()).unwrap_or_default();
    let since = Local::now() - chrono::Duration::from_std(elapsed).unwrap_or_default();
    Some(DeskEntry {
        id: g.id.clone(),
        title: g.title.clone(),
        detail: ctx.tf(
            "profile.playing_since",
            &[("time", &since.format("%H:%M").to_string())],
        ),
        value: r.started.map(|_| hm(elapsed)).unwrap_or_default(),
        thumb: thumb(g),
        accent: parse_color(&g.accent),
        dot: true,
        ..Default::default()
    })
}

/// Coleções do perfil: as do usuário e as automáticas, com os jogos de cada uma (a chave é a do SeletorDeColecao).
fn collections(ctx: &Ctx) -> Vec<(String, String, bool, Vec<&Game>)> {
    let games = ctx.library.games();
    let mut v: Vec<(String, String, bool, Vec<&Game>)> = ctx
        .collections
        .collections()
        .iter()
        .map(|c| {
            let base = LibraryCriteria::default().toggle_collection(&c.id);
            (
                format!("m:{}", c.id),
                c.name.clone(),
                false,
                query::apply(games, LibraryFilter::All, &base),
            )
        })
        .collect();
    v.extend(smart::list(ctx).into_iter().map(|c| {
        let list = query::apply(games, c.filter, &c.criteria_in(ctx.lang()));
        (format!("s:{}", c.id), c.name, true, list)
    }));
    v
}

pub struct ProfilePage {
    query: String,
    tab: Tab,
    period: Period,
    zone: Zone,
    index: usize,
}

impl Default for ProfilePage {
    fn default() -> Self {
        Self::new()
    }
}

impl ProfilePage {
    pub fn new() -> Self {
        ProfilePage {
            query: String::new(),
            tab: Tab::Overview,
            period: Period::Week,
            zone: Zone::Tabs,
            index: 0,
        }
    }

    pub fn tab(&self) -> Tab {
        self.tab
    }

    pub fn zone(&self) -> Zone {
        self.zone
    }

    pub fn open_tab(&mut self, tab: Tab) {
        self.tab = tab;
        self.zone = Zone::Tabs;
        self.index = TABS.iter().position(|t| *t == tab).unwrap_or(0);
    }

    fn matches(&self, title: &str) -> bool {
        tako_core::library::search_text::matches(title, &self.query)
    }

    fn sessions<'a>(&self, ctx: &'a Ctx) -> Vec<(&'a Game, &'a PlaySession)> {
        all_sessions(ctx)
            .into_iter()
            .filter(|(g, _)| self.matches(&g.title))
            .collect()
    }

    fn live(&self, ctx: &Ctx) -> Option<DeskEntry> {
        running_row(ctx).filter(|r| self.matches(&r.title))
    }

    fn collections<'a>(&self, ctx: &'a Ctx) -> Vec<(String, String, bool, Vec<&'a Game>)> {
        collections(ctx)
            .into_iter()
            .filter(|(_, name, _, _)| self.matches(name))
            .collect()
    }

    /// Sessões da guia Sessões, no período escolhido.
    fn period_sessions<'a>(&self, ctx: &'a Ctx) -> Vec<(&'a Game, &'a PlaySession)> {
        let today = Local::now().date_naive();
        let since = match self.period {
            Period::Week => Some(monday(today)),
            Period::Month => today.with_day(1),
            Period::All => None,
        };
        self.sessions(ctx)
            .into_iter()
            .filter(|(_, s)| since.is_none_or(|d| local_day(s) >= d))
            .collect()
    }

    /// Linhas da lista da guia atual (sessões sem os cabeçalhos de dia, ou coleções + "Nova coleção").
    fn list_len(&self, ctx: &Ctx) -> usize {
        match self.tab {
            Tab::Overview => {
                self.sessions(ctx)
                    .len()
                    .min(RECENT - usize::from(ctx.running.is_some()))
                    + usize::from(self.live(ctx).is_some())
            }
            Tab::Sessions => {
                self.period_sessions(ctx).len().min(MAX_ROWS)
                    + usize::from(self.live(ctx).is_some())
            }
            Tab::Collections => self.collections(ctx).len() + 1,
        }
    }

    fn zones(&self, ctx: &Ctx) -> Vec<Zone> {
        let mut v = vec![Zone::Bar, Zone::Tabs, Zone::Actions];
        match self.tab {
            Tab::Overview => v.push(Zone::Tiles),
            Tab::Sessions => v.push(Zone::Periods),
            Tab::Collections => {}
        }
        if self.list_len(ctx) > 0 {
            v.push(Zone::List);
        }
        v
    }

    fn zone_len(&self, zone: Zone, ctx: &Ctx) -> usize {
        match zone {
            Zone::Bar => super::bar::items(ctx).len(),
            Zone::Tabs => TABS.len(),
            Zone::Actions => ACTIONS,
            Zone::Tiles => TILES,
            Zone::Periods => PERIODS.len(),
            Zone::List => self.list_len(ctx),
        }
    }

    /// Tab / Shift+Tab e ↑ ↓ entre as zonas.
    fn step_zone(&mut self, forward: bool, ctx: &Ctx) {
        let zones = self.zones(ctx);
        let i = zones.iter().position(|z| *z == self.zone).unwrap_or(0);
        let next = if forward {
            (i + 1).min(zones.len() - 1)
        } else {
            i.saturating_sub(1)
        };
        self.zone = zones[next];
        self.index = match self.zone {
            Zone::Tabs => TABS.iter().position(|t| *t == self.tab).unwrap_or(0),
            Zone::Periods => PERIODS.iter().position(|p| *p == self.period).unwrap_or(0),
            // O foco começa no botão do perfil (a página em que está).
            Zone::Bar => super::bar::items(ctx)
                .iter()
                .position(|i| *i == super::bar::Item::Profile)
                .unwrap_or(0),
            _ => 0,
        };
    }

    /// Enter (ou clique) no item com o foco.
    fn activate(&mut self, ctx: &mut Ctx) -> Nav {
        match self.zone {
            Zone::Bar => match super::bar::item(ctx, self.index) {
                Some(super::bar::Item::Library) => return Nav::Back,
                Some(item) => return super::bar::activate(item, ctx),
                None => {}
            },
            Zone::Tabs => self.open_tab(TABS[self.index.min(TABS.len() - 1)]),
            Zone::Actions => return self.action(self.index, ctx),
            Zone::Tiles => match self.index {
                0 => self.open_tab(Tab::Sessions),
                1 => {
                    if let Some(g) = most_played(ctx).first() {
                        ctx.open_details = Some(g.id.clone());
                    }
                }
                _ => return Nav::Back,
            },
            Zone::Periods => {
                self.period = PERIODS[self.index.min(PERIODS.len() - 1)];
            }
            Zone::List => return self.open_row(self.index, ctx),
        }
        Nav::Stay
    }

    fn action(&mut self, index: usize, ctx: &mut Ctx) -> Nav {
        match index {
            0 => Nav::Layer(Box::new(profiles::panel(ctx))),
            _ => Nav::Layer(Box::new(profiles::manage_active(ctx))),
        }
    }

    /// Linha da lista: a sessão leva ao jogo na biblioteca; o cartão abre a coleção nela.
    fn open_row(&mut self, index: usize, ctx: &mut Ctx) -> Nav {
        match self.tab {
            Tab::Overview | Tab::Sessions => {
                let running = self.live(ctx).map(|r| r.id);
                let id = if index == 0 && running.is_some() {
                    running
                } else {
                    let i = index - usize::from(running.is_some());
                    let list = if self.tab == Tab::Overview {
                        self.sessions(ctx)
                    } else {
                        self.period_sessions(ctx)
                    };
                    list.get(i).map(|(g, _)| g.id.clone())
                };
                ctx.open_details = id;
            }
            Tab::Collections => match self.collections(ctx).into_iter().nth(index) {
                Some((key, ..)) => {
                    ctx.open_route = Some(format!("{}{key}", desktop::COLLECTION_ROUTE));
                }
                None => return Nav::Layer(Box::new(dialogs::new_collection(ctx, None))),
            },
        }
        Nav::Stay
    }
}

/// Os três jogos com mais tempo de jogo (o bloco Mais jogado).
fn most_played(ctx: &Ctx) -> Vec<&Game> {
    let mut v: Vec<&Game> = ctx
        .library
        .games()
        .iter()
        .filter(|g| !g.play_time.is_zero())
        .collect();
    v.sort_by_key(|g| std::cmp::Reverse(g.play_time));
    v.truncate(3);
    v
}

/// O fundo: o mais jogado do mês; sem sessões no mês, o último jogo jogado.
fn backdrop_game(ctx: &Ctx) -> Option<&Game> {
    let today = Local::now().date_naive();
    let sessions = all_sessions(ctx);
    let top = top_since(
        sessions.iter().map(|(g, s)| (g.id.as_str(), *s)),
        today.with_day(1).unwrap_or(today),
    );
    top.and_then(|id| ctx.library.find(id)).or_else(|| {
        ctx.library
            .games()
            .iter()
            .filter(|g| g.last_played_at.is_some())
            .max_by_key(|g| g.last_played_at)
    })
}

impl Page for ProfilePage {
    fn route(&self) -> String {
        ROUTE.into()
    }

    fn kind(&self) -> &'static str {
        KIND
    }

    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Nav {
        let len = self.zone_len(self.zone, ctx);
        let grid = self.zone == Zone::List && self.tab == Tab::Collections;
        let rows = self.zone == Zone::List && !grid;
        match cmd {
            Cmd::Left if !rows => self.index = self.index.saturating_sub(1),
            Cmd::Right if !rows => self.index = (self.index + 1).min(len.saturating_sub(1)),
            Cmd::Up if rows && self.index > 0 => self.index -= 1,
            Cmd::Down if rows => {
                if self.index + 1 < len {
                    self.index += 1;
                }
            }
            Cmd::Up if grid && self.index >= COLUMNS => self.index -= COLUMNS,
            Cmd::Down if grid => self.index = (self.index + COLUMNS).min(len.saturating_sub(1)),
            Cmd::Up => self.step_zone(false, ctx),
            Cmd::Down => self.step_zone(true, ctx),
            Cmd::Accept => return self.activate(ctx),
            Cmd::Back if self.zone != Zone::Tabs => {
                self.zone = Zone::Tabs;
                self.index = TABS.iter().position(|t| *t == self.tab).unwrap_or(0);
            }
            Cmd::Back => return Nav::Back,
            Cmd::PrevSection | Cmd::NextSection => {
                let i = TABS.iter().position(|t| *t == self.tab).unwrap_or(0);
                let n = if cmd == Cmd::NextSection {
                    (i + 1).min(TABS.len() - 1)
                } else {
                    i.saturating_sub(1)
                };
                self.open_tab(TABS[n]);
            }
            _ => {}
        }
        Nav::Stay
    }

    fn key(&mut self, text: &str, mods: KeyModifiers, ctx: &mut Ctx) -> Option<Nav> {
        let c = text.chars().next()?;
        // Shift+Tab chega como Tab com Shift ou como "back tab" (U+0019), conforme o sistema.
        if c == keys::TAB || c == '\u{0019}' {
            let forward = c == keys::TAB && mods != KeyModifiers::SHIFT;
            match super::bar::step(ctx, self.index, forward).filter(|_| self.zone == Zone::Bar) {
                Some(i) => self.index = i,
                None => self.step_zone(forward, ctx),
            }
            return Some(Nav::Stay);
        }
        None
    }

    fn click(&mut self, zone: &str, index: usize, ctx: &mut Ctx) -> Nav {
        match zone {
            "profile-tab" => {
                if let Some(t) = TABS.get(index) {
                    self.open_tab(*t);
                }
            }
            "profile-action" => return self.action(index, ctx),
            "profile-tile" => {
                self.zone = Zone::Tiles;
                self.index = index;
                return self.activate(ctx);
            }
            "profile-see-all" => self.open_tab(Tab::Sessions),
            "profile-period" => {
                if let Some(p) = PERIODS.get(index) {
                    self.period = *p;
                }
            }
            "profile-row" => {
                self.zone = Zone::List;
                self.index = index;
                return self.open_row(index, ctx);
            }
            _ => {}
        }
        Nav::Stay
    }

    fn edited(&mut self, zone: &str, text: &str, ctx: &mut Ctx) {
        if zone == "search" {
            self.query = text.chars().take(128).collect();
            self.index = 0;
            self.zone = Zone::List;
            ctx.dirty = true;
        }
    }

    fn render(&mut self, ctx: &Ctx, out: &mut Out) {
        out.set("desk-search", self.query.clone());
        out.set("desk-search-hint", ctx.t("desktop.profile_search"));
        let lang = ctx.lang();
        let today = Local::now().date_naive();
        let sessions = all_sessions(ctx);
        let active = tako_core::profiles::Profiles::load(&ctx.paths.machine).active();
        let name = profiles::name(ctx, &active);

        // Linha de estado: "Perfil principal | ● Jogando Hades II · 42 min" (sem jogo: "Jogou por último ontem").
        let mut line = Vec::new();
        if active.id == tako_core::profiles::MAIN {
            line.push(DeskInfo {
                label: ctx.t("profile.main"),
                ..Default::default()
            });
        }
        let running = ctx.running.as_ref();
        if let Some(r) = running {
            let time = r.started.map(|t| hm(t.elapsed()));
            line.push(DeskInfo {
                label: match time {
                    Some(t) => ctx.tf("profile.playing", &[("game", &r.title), ("time", &t)]),
                    None => ctx.tf("profile.playing_title", &[("game", &r.title)]),
                },
                dot: true,
                ..Default::default()
            });
        } else if let Some((_, s)) = sessions.first() {
            let when = desktop::day(ctx, s.started_at.with_timezone(&Local)).to_lowercase();
            line.push(DeskInfo {
                label: ctx.tf("profile.last", &[("when", &when)]),
                ..Default::default()
            });
        }

        // Tempo jogado: a semana em barras (segunda a domingo) e o total de tudo.
        let days = week(sessions.iter().map(|(_, s)| *s), today);
        let today_i = today.weekday().num_days_from_monday() as usize;
        let max = days.iter().max().copied().unwrap_or_default();
        let week_bars: Vec<DeskBar> = days
            .iter()
            .enumerate()
            .map(|(i, d)| DeskBar {
                label: if i == today_i {
                    ctx.t("profile.weekday.today")
                } else {
                    ctx.t(&format!("profile.weekday.{}", WEEKDAYS[i]))
                },
                value: if i == today_i && !d.is_zero() {
                    hm(*d)
                } else {
                    String::new()
                },
                fraction: if max.is_zero() {
                    0.0
                } else {
                    d.as_secs_f32() / max.as_secs_f32()
                },
                today: i == today_i,
                future: i > today_i,
            })
            .collect();
        let week_text = days
            .iter()
            .enumerate()
            .take(today_i + 1)
            .map(|(i, d)| {
                let day = ctx.t(&format!("profile.weekday_long.{}", WEEKDAYS[i]));
                let v = if d.is_zero() {
                    ctx.t("profile.nothing")
                } else {
                    hm(*d)
                };
                format!("{day} {v}")
            })
            .collect::<Vec<_>>()
            .join(", ");
        let week_total: Duration = days.iter().sum();
        let total: Duration = ctx.library.games().iter().map(|g| g.play_time).sum();

        let top = most_played(ctx);
        let top_max = top.first().map(|g| g.play_time).unwrap_or_default();
        let top_rows: Vec<DeskEntry> = top
            .iter()
            .map(|g| DeskEntry {
                id: g.id.clone(),
                title: g.title.clone(),
                value: game_text::play_time(lang, g.play_time),
                thumb: thumb(g),
                accent: parse_color(&g.accent),
                fraction: g.play_time.as_secs_f32() / top_max.as_secs_f32().max(1.0),
                ..Default::default()
            })
            .collect();
        // Jogos na biblioteca: leque com as três últimas capas, a mais recente na frente (a última da lista).
        let mut recent_games: Vec<&Game> = ctx.library.games().iter().collect();
        recent_games.sort_by_key(|g| std::cmp::Reverse(g.added_at));
        let fan: Vec<DeskEntry> = recent_games
            .iter()
            .take(3)
            .rev()
            .map(|g| DeskEntry {
                id: g.id.clone(),
                title: g.title.clone(),
                thumb: thumb(g),
                accent: parse_color(&g.accent),
                ..Default::default()
            })
            .collect();
        let tiles = vec![
            DeskInfo {
                icon: "clock".into(),
                label: ctx.t("profile.tile_time"),
                value: if total.is_zero() {
                    "—".into()
                } else {
                    game_text::play_time(lang, total)
                },
                ..Default::default()
            },
            DeskInfo {
                icon: "star".into(),
                label: ctx.t("profile.tile_top"),
                value: top
                    .first()
                    .map(|g| g.title.clone())
                    .unwrap_or_else(|| "—".into()),
                ..Default::default()
            },
            DeskInfo {
                icon: "grid".into(),
                label: ctx.t("profile.tile_games"),
                value: ctx.library.games().len().to_string(),
                ..Default::default()
            },
        ];

        let live = self.live(ctx);
        let recent: Vec<DeskEntry> = live
            .clone()
            .into_iter()
            .chain(
                self.sessions(ctx)
                    .iter()
                    .take(RECENT - usize::from(live.is_some()))
                    .map(|(g, s)| session_row(ctx, g, s, true)),
            )
            .collect();

        // Guia Sessões: agrupadas por dia (cabeçalho com o dia, a data e o total dele).
        let period = self.period_sessions(ctx);
        let period_total: Duration = period.iter().map(|(_, s)| counted(s)).sum();
        let mut rows: Vec<DeskEntry> = Vec::new();
        if let Some(r) = live.clone() {
            rows.push(r);
        }
        let shown = &period[..period.len().min(MAX_ROWS)];
        for (i, (g, s)) in shown.iter().enumerate() {
            let d = local_day(s);
            if i == 0 || local_day(shown[i - 1].1) != d {
                let day_total: Duration = shown
                    .iter()
                    .filter(|(_, x)| local_day(x) == d)
                    .map(|(_, x)| counted(x))
                    .sum();
                let at = s.started_at.with_timezone(&Local);
                let wd = WEEKDAYS[d.weekday().num_days_from_monday() as usize];
                let (title, detail) = if d == today || today.pred_opt() == Some(d) {
                    (desktop::day(ctx, at), game_text::short_date(lang, d))
                } else {
                    (
                        ctx.t(&format!("profile.weekday_long.{wd}")),
                        game_text::short_date(lang, d),
                    )
                };
                rows.push(DeskEntry {
                    title,
                    detail,
                    value: hm(day_total),
                    header: true,
                    ..Default::default()
                });
            }
            rows.push(DeskEntry {
                row: (live.is_some() as usize + i) as i32,
                ..session_row(ctx, g, s, false)
            });
        }
        let summary = i18n::tn(
            "profile.summary",
            period.len() as i64,
            &[("time", &hm(period_total))],
        );

        // Guia Coleções: cartões com o mosaico de até 4 capas.
        let cols = self.collections(ctx);
        let in_cols: std::collections::BTreeSet<&str> = cols
            .iter()
            .filter(|c| !c.2)
            .flat_map(|c| c.3.iter().map(|g| g.id.as_str()))
            .collect();
        let cards: Vec<DeskCard> = cols
            .iter()
            .map(|(_, name, auto, games)| DeskCard {
                title: name.clone(),
                count: if games.is_empty() {
                    ctx.t("profile.no_games")
                } else {
                    let installed = games.iter().filter(|g| g.is_installed).count();
                    format!(
                        "{} · {}",
                        i18n::tn("common.games", games.len() as i64, &[]),
                        i18n::tn("profile.installed", installed as i64, &[])
                    )
                },
                auto: *auto,
                covers: games
                    .iter()
                    .take(4)
                    .map(|g| DeskEntry {
                        id: g.id.clone(),
                        thumb: tiles::image(g.shown_cover(), tiles::COVER),
                        accent: parse_color(&g.accent),
                        ..Default::default()
                    })
                    .collect(),
            })
            .collect();
        let cards_summary = format!(
            "{} · {}",
            i18n::tn("profile.collections", cols.len() as i64, &[]),
            i18n::tn("common.games", in_cols.len() as i64, &[])
        );

        // O foco não fica numa linha que sumiu (a lista mudou).
        let len = self.zone_len(self.zone, ctx);
        if self.zone == Zone::List && len == 0 {
            self.zone = Zone::Tabs;
            self.index = TABS.iter().position(|t| *t == self.tab).unwrap_or(0);
        } else if self.index >= len.max(1) {
            self.index = len.saturating_sub(1);
        }
        let zone = match self.zone {
            Zone::Bar => "bar",
            Zone::Tabs => "tabs",
            Zone::Actions => "actions",
            Zone::Tiles => "tiles",
            Zone::Periods => "periods",
            Zone::List => "list",
        };

        let profile = DeskProfile {
            initial: name
                .chars()
                .next()
                .map(|c| c.to_uppercase().collect())
                .unwrap_or_default(),
            name,
            line,
            playing: running.is_some(),
            tabs: ["overview", "sessions", "collections"]
                .iter()
                .map(|k| ctx.t(&format!("profile.tab_{k}")))
                .collect(),
            tab: TABS.iter().position(|t| *t == self.tab).unwrap_or(0) as i32,
            actions: vec![
                DeskInfo {
                    icon: "user".into(),
                    label: ctx.t("profile.switch"),
                    ..Default::default()
                },
                DeskInfo {
                    icon: "pencil".into(),
                    label: ctx.t("profile.edit"),
                    ..Default::default()
                },
            ],
            tiles,
            week_label: ctx.t("profile.this_week"),
            week_total: hm(week_total),
            week: week_bars,
            week_text: format!("{}: {week_text}", ctx.t("profile.this_week")),
            top: top_rows,
            fan,
            recent_title: ctx.t("profile.recent"),
            see_all: ctx.t("desktop.sessions.see_all"),
            recent,
            empty: ctx.t("profile.empty"),
            periods: ["week", "month", "all"]
                .iter()
                .map(|k| ctx.t(&format!("profile.period_{k}")))
                .collect(),
            period: PERIODS.iter().position(|p| *p == self.period).unwrap_or(0) as i32,
            summary,
            sessions: rows,
            cards_summary,
            cards,
            auto_label: ctx.t("profile.auto"),
            new_label: ctx.t("collections.new"),
            new_text: ctx.t("profile.new_text"),
            zone: zone.into(),
            index: self.index as i32,
        };
        out.set("desk-profile", profile);
        // A barra de cima lê a zona e o índice do foco como na biblioteca.
        out.set("desk-zone", zone);
        out.set("desk-zone-index", self.index as i32);

        // Fundo: a arte do mais jogado do mês, nítida como na biblioteca; sem nenhum, o degradê do accent.
        let view = match backdrop_game(ctx) {
            Some(g) => {
                let (art_blur, art_lum) = g
                    .shown_hero()
                    .and_then(crate::images::glass)
                    .unwrap_or_default();
                DeskGameView {
                    has_game: true,
                    title: g.title.clone(),
                    art: tiles::image(g.shown_hero(), tiles::hero(ctx)),
                    art_blur,
                    art_lum,
                    accent: parse_color(&g.accent),
                    art_loading: g.shown_hero().is_some_and(crate::images::is_pending),
                    ..Default::default()
                }
            }
            None => DeskGameView::default(),
        };
        out.set("desk-game", view);
    }

    fn hints(&self, _ctx: &Ctx) -> Vec<Hint> {
        Vec::new()
    }

    /// A cor da tela é a do jogo do fundo.
    fn focused_game(&self, ctx: &Ctx) -> Option<String> {
        backdrop_game(ctx).map(|g| g.id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(day: NaiveDate, hour: u32, minutes: u64) -> PlaySession {
        let start = Local
            .from_local_datetime(&day.and_hms_opt(hour, 0, 0).unwrap())
            .unwrap();
        PlaySession {
            started_at: start.with_timezone(&chrono::Utc),
            duration: Duration::from_secs(minutes * 60),
        }
    }

    #[test]
    fn the_week_runs_monday_to_sunday_and_short_sessions_do_not_count() {
        // Quarta-feira, 14/10/2026.
        let today = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap();
        let s = [
            at(NaiveDate::from_ymd_opt(2026, 10, 12).unwrap(), 20, 40), // segunda
            at(NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(), 9, 60),
            at(NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(), 21, 35),
            at(today, 10, 0), // abriu e fechou
            at(NaiveDate::from_ymd_opt(2026, 10, 11).unwrap(), 22, 90), // domingo anterior
            at(NaiveDate::from_ymd_opt(2026, 10, 19).unwrap(), 10, 30), // próxima semana
        ];
        let w = week(&s, today);
        let min = |m: u64| Duration::from_secs(m * 60);
        assert_eq!(
            w,
            [min(40), min(95), min(0), min(0), min(0), min(0), min(0)]
        );
        // Domingo é o último dia da mesma semana.
        let sunday = NaiveDate::from_ymd_opt(2026, 10, 18).unwrap();
        assert_eq!(week(&s, sunday)[0], min(40));
        assert_eq!(
            week(&s, NaiveDate::from_ymd_opt(2026, 10, 11).unwrap())[6],
            min(90)
        );
    }

    #[test]
    fn the_backdrop_is_the_game_played_most_this_month() {
        let d = |day| NaiveDate::from_ymd_opt(2026, 10, day).unwrap();
        let a = [at(d(2), 10, 50), at(d(20), 10, 30)];
        let b = [at(d(3), 10, 60), at(d(4), 10, 15)];
        let old = [at(NaiveDate::from_ymd_opt(2026, 9, 28).unwrap(), 10, 600)];
        let played: Vec<(&str, &PlaySession)> = a
            .iter()
            .map(|s| ("a", s))
            .chain(b.iter().map(|s| ("b", s)))
            .chain(old.iter().map(|s| ("old", s)))
            .collect();
        assert_eq!(top_since(played.clone(), d(1)), Some("a"));
        // Do dia 3 em diante, "b" tem mais.
        assert_eq!(top_since(played.clone(), d(3)), Some("b"));
        // Sem sessões no período, nenhum (o fundo cai para o último jogo jogado).
        assert_eq!(top_since(played, d(25)), None);
    }

    #[test]
    fn search_filters_sessions_and_collections_without_changing_totals() {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = crate::app::build_ctx(
            tako_core::paths::DataPaths::at(dir.path()),
            tako_platform::Platform::simulated(),
            true,
        );
        ctx.library.refresh();
        let id = ctx.library.games()[0].id.clone();
        let title = ctx.library.games()[0].title.clone();
        ctx.library
            .record_session(&id, Duration::from_secs(600))
            .unwrap();
        ctx.collections.create("Sofá", Some(&id)).unwrap();
        let mut page = ProfilePage::new();
        page.edited("search", &title, &mut ctx);
        assert!(page.sessions(&ctx).iter().all(|(g, _)| g.title == title));
        page.edited("search", "sofa", &mut ctx);
        assert_eq!(page.collections(&ctx).len(), 1);
        page.edited("search", "inexistente 987654321", &mut ctx);
        assert!(page.sessions(&ctx).is_empty());
        assert!(page.collections(&ctx).is_empty());
        assert_eq!(all_sessions(&ctx).len(), 1);
        page.edited("search", "", &mut ctx);
        assert_eq!(page.sessions(&ctx).len(), 1);
    }

    #[test]
    fn durations_read_with_units() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        i18n::set_lang(i18n::Lang::PtBr);
        assert_eq!(hm(Duration::from_secs(42 * 60)), "42 min");
        assert_eq!(hm(Duration::from_secs(120 * 60)), "2 h");
        assert_eq!(hm(Duration::from_secs(397 * 60)), "6 h 37 min");
        assert_eq!(hm(Duration::from_secs(62 * 60)), "1 h 02 min");
    }

    #[test]
    fn the_profile_tab_routes_sessions_collections_and_back() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        i18n::set_lang(i18n::Lang::PtBr);
        let dir = tempfile::tempdir().unwrap();
        let ctx = crate::app::build_ctx(
            tako_core::paths::DataPaths::at(dir.path()),
            tako_platform::Platform::simulated(),
            true,
        );
        let mut shell = super::super::Shell::new(ctx);
        shell.enter_desktop();
        let id = shell.ctx.library.games()[2].id.clone();
        shell
            .ctx
            .library
            .record_session(&id, Duration::from_secs(45 * 60))
            .unwrap();
        // "Ver perfil" do menu do avatar abre a página; o botão da barra fica aceso e não há aba do perfil.
        shell.click("desk-profile-menu", 0);
        shell.click("overlay-row", 0);
        assert_eq!(shell.page().kind(), KIND);
        let out = shell.render(std::time::Instant::now());
        let prop = |name| &out.props.iter().find(|(n, _)| *n == name).unwrap().1;
        assert_eq!(
            *prop("desk-profile-active"),
            slint_interpreter::Value::Bool(true)
        );
        assert!(!format!("{:?}", prop("desk-nav")).contains("profile"));
        // O fundo é o jogo com a sessão do mês.
        assert_eq!(shell.focused_game(), Some(id.clone()));
        // Clicar na sessão recente leva ao jogo na biblioteca.
        shell.click("profile-row", 0);
        assert_eq!(shell.page().kind(), "desk-library");
        assert_eq!(shell.focused_game(), Some(id.clone()));

        // Coleções: o cartão abre a biblioteca filtrada na coleção.
        shell.ctx.collections.create("Sofá", Some(&id)).unwrap();
        shell.ctx.sync_collections();
        shell.open_route(ROUTE);
        shell.click("profile-tab", 2);
        shell.render(std::time::Instant::now());
        shell.click("profile-row", 0);
        let lib = shell.desktop_library().unwrap();
        assert_eq!(shell.page().kind(), "desk-library");
        assert_eq!(lib.tab(), desktop::Tab::Collections);
        assert_eq!(lib.games(&shell.ctx).len(), 1);

        // Teclado: Tab anda pelas zonas; Esc volta às guias e depois à biblioteca.
        shell.open_route(ROUTE);
        shell.key("\t", KeyModifiers::NONE);
        shell.key("\t", KeyModifiers::NONE);
        shell.key("\t", KeyModifiers::NONE);
        let page = |s: &super::super::Shell| s.page().kind();
        assert_eq!(page(&shell), KIND);
        shell.handle(Cmd::Back);
        shell.handle(Cmd::Back);
        assert_eq!(page(&shell), "desk-library");
    }
}
