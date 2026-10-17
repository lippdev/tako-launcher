//! Biblioteca do modo desktop (ADR-0027): abas Todos/Favoritos/Coleções, busca, filtros e ordenação, a lista de
//! jogos à esquerda e o destaque do jogo selecionado à direita. Mouse primeiro (clique seleciona, duplo clique joga,
//! botão direito abre as ações); o teclado anda pelas zonas com Tab e pelas linhas com as setas. As regras são as
//! da biblioteca da tela cheia (`query`, `dialogs`, `details::primary_action`); aqui só muda a apresentação.

use super::library::Criteria;
use super::{Cmd, Ctx, Effect, Nav, Out, Page, details, dialogs, keys, smart, tiles};
use crate::model::{DeskGame, DeskGameView, Hint, NavItem, PanelRow, Stat};
use crate::values::parse_color;
use std::cell::RefCell;
use std::rc::Rc;
use tako_core::input::shortcut::KeyModifiers;
use tako_core::library::{
    Game, LibraryCriteria, LibraryFacts, LibraryFilter, query, smart::SmartCollection,
};
use tako_core::presentation::{game_text, library_text};

/// Rota da biblioteca desktop.
pub const ROUTE: &str = "desk:library";

/// Miniatura da lista (2:3, pequena: a lista inteira cabe no cache).
const THUMB: (u32, u32) = (64, 96);
/// Linhas em volta da primeira visível que pedem miniatura.
const NEAR: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    All,
    Favorites,
    Collections,
}

const TABS: [Tab; 3] = [Tab::All, Tab::Favorites, Tab::Collections];

/// Coleção escolhida na aba Coleções: do usuário ou automática (LIB-003).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Pick {
    Manual(String),
    Smart(String),
}

/// Onde está o foco do teclado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    Tabs,
    Collections,
    List,
    Actions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Play,
    Favorite,
    More,
    End,
}

pub struct DesktopLibrary {
    tab: Tab,
    pick: Option<Pick>,
    /// Busca e filtros do usuário, por cima da aba (o painel de filtros é o mesmo da tela cheia).
    pub criteria: Criteria,
    selected: Option<String>,
    zone: Zone,
    /// Aba, coleção ou ação com o foco do teclado (conforme a zona).
    index: usize,
}

impl Default for DesktopLibrary {
    fn default() -> Self {
        Self::new()
    }
}

impl DesktopLibrary {
    pub fn new() -> Self {
        DesktopLibrary {
            tab: Tab::All,
            pick: None,
            criteria: Rc::new(RefCell::new(LibraryCriteria::default())),
            selected: None,
            zone: Zone::List,
            index: 0,
        }
    }

    pub fn tab(&self) -> Tab {
        self.tab
    }

    pub fn zone(&self) -> Zone {
        self.zone
    }

    /// Coleções da aba: as do usuário e as automáticas.
    fn picks(ctx: &Ctx) -> Vec<(Pick, String, usize)> {
        let mut v: Vec<(Pick, String, usize)> = ctx
            .collections
            .collections()
            .iter()
            .map(|c| {
                let n = ctx
                    .library
                    .games()
                    .iter()
                    .filter(|g| c.contains(&g.id))
                    .count();
                (Pick::Manual(c.id.clone()), c.name.clone(), n)
            })
            .collect();
        v.extend(smart::list(ctx).into_iter().map(|c| {
            let n = smart::count(ctx, &c);
            (Pick::Smart(c.id.clone()), c.name, n)
        }));
        v
    }

    fn smart_of(ctx: &Ctx, id: &str) -> Option<SmartCollection> {
        smart::list(ctx).into_iter().find(|c| c.id == id)
    }

    /// Filtro da aba (o que o painel de filtros conta e o que uma coleção automática salva).
    fn route_filter(&self) -> LibraryFilter {
        match self.tab {
            Tab::Favorites => LibraryFilter::Favorites,
            _ => LibraryFilter::All,
        }
    }

    /// Os jogos da lista, na ordem da tela.
    pub fn games<'a>(&self, ctx: &'a Ctx) -> Vec<&'a Game> {
        let user = self.criteria.borrow();
        let games = ctx.library.games();
        match (&self.tab, &self.pick) {
            (Tab::All, _) => query::apply(games, LibraryFilter::All, &user),
            (Tab::Favorites, _) => query::apply(games, LibraryFilter::Favorites, &user),
            (Tab::Collections, None) => Vec::new(),
            (Tab::Collections, Some(Pick::Manual(id))) => {
                let base = LibraryCriteria::default().toggle_collection(id);
                let base = query::apply(games, LibraryFilter::All, &base);
                query::apply_by(base, |g| LibraryFacts::of(g), &user)
            }
            (Tab::Collections, Some(Pick::Smart(id))) => match Self::smart_of(ctx, id) {
                Some(c) => {
                    let base = query::apply(games, c.filter, &c.criteria_in(ctx.lang()));
                    query::apply_by(base, |g| LibraryFacts::of(g), &user)
                }
                None => Vec::new(),
            },
        }
    }

    fn ids(&self, ctx: &Ctx) -> Vec<String> {
        self.games(ctx).into_iter().map(|g| g.id.clone()).collect()
    }

    /// O selecionado, se ainda está na lista; senão o primeiro.
    fn current(&self, ids: &[String]) -> Option<usize> {
        self.selected
            .as_ref()
            .and_then(|s| ids.iter().position(|i| i == s))
            .or((!ids.is_empty()).then_some(0))
    }

    pub fn selected(&self, ctx: &Ctx) -> Option<String> {
        let ids = self.ids(ctx);
        self.current(&ids).map(|i| ids[i].clone())
    }

    fn select_index(&mut self, ctx: &Ctx, index: usize) {
        let ids = self.ids(ctx);
        if let Some(id) = ids.get(index) {
            self.selected = Some(id.clone());
        }
        self.zone = Zone::List;
    }

    pub fn open_tab(&mut self, tab: Tab, ctx: &Ctx) {
        self.tab = tab;
        if tab == Tab::Collections && self.pick.is_none() {
            self.pick = Self::picks(ctx).into_iter().next().map(|(p, ..)| p);
        }
    }

    fn actions(&self, ctx: &Ctx, g: &Game) -> Vec<Action> {
        let mut v = vec![Action::Play, Action::Favorite, Action::More];
        if ctx.running.as_ref().is_some_and(|r| r.id == g.id) {
            v.push(Action::End);
        }
        v
    }

    fn run(&mut self, action: Action, ctx: &mut Ctx) -> Nav {
        let Some(id) = self.selected(ctx) else {
            return Nav::Stay;
        };
        self.selected = Some(id.clone());
        match action {
            Action::Play => {
                // Já abrindo: o clique repetido não abre outra vez.
                if ctx.launching {
                    return Nav::Stay;
                }
                details::primary_action(ctx, &id)
            }
            Action::Favorite => {
                dialogs::toggle_favorite(ctx, &id);
                Nav::Stay
            }
            Action::More => Nav::Layer(Box::new(dialogs::more_actions(ctx, &id))),
            Action::End => {
                ctx.effect(Effect::EndGame);
                Nav::Stay
            }
        }
    }

    fn zones(&self) -> Vec<Zone> {
        let mut v = vec![Zone::Tabs];
        if self.tab == Tab::Collections {
            v.push(Zone::Collections);
        }
        v.extend([Zone::List, Zone::Actions]);
        v
    }

    /// Tab / Shift+Tab: próxima zona.
    fn cycle(&mut self, forward: bool) {
        let zones = self.zones();
        let i = zones.iter().position(|z| *z == self.zone).unwrap_or(0) as i32;
        let n = zones.len() as i32;
        self.zone = zones[((i + if forward { 1 } else { -1 }).rem_euclid(n)) as usize];
        self.index = match self.zone {
            Zone::Tabs => TABS.iter().position(|t| *t == self.tab).unwrap_or(0),
            _ => 0,
        };
    }

    fn clear(&mut self) {
        let mut c = self.criteria.borrow_mut();
        let sort = c.sort;
        *c = LibraryCriteria {
            sort,
            ..Default::default()
        };
    }

    fn filtered(&self) -> bool {
        let c = self.criteria.borrow();
        c.has_search() || c.has_filters()
    }
}

/// Tempo de jogo, última vez, tamanho e loja: a linha de informações ao lado de Jogar.
fn summary(ctx: &Ctx, g: &Game) -> Vec<Stat> {
    let lang = ctx.lang();
    let dash = || "—".to_owned();
    vec![
        Stat {
            label: ctx.t("desktop.play_time"),
            value: if g.play_time.is_zero() {
                dash()
            } else {
                game_text::play_time(lang, g.play_time)
            },
        },
        Stat {
            label: ctx.t("desktop.last_played"),
            value: if g.last_played_at.is_some() {
                game_text::last_played(lang, g.last_played_at, chrono::Utc::now())
            } else {
                ctx.t("desktop.never")
            },
        },
        Stat {
            label: ctx.t("desktop.size"),
            value: g.size_label.clone().unwrap_or_else(dash),
        },
        Stat {
            label: ctx.t("desktop.store"),
            value: game_text::store_name(lang, g.store()),
        },
    ]
}

impl Page for DesktopLibrary {
    fn route(&self) -> String {
        ROUTE.into()
    }

    fn kind(&self) -> &'static str {
        "desk-library"
    }

    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Nav {
        let ids = self.ids(ctx);
        let cur = self.current(&ids);
        let picks = Self::picks(ctx);
        match (self.zone, cmd) {
            (_, Cmd::PrevSection) | (_, Cmd::NextSection) => {
                let i = TABS.iter().position(|t| *t == self.tab).unwrap_or(0) as i32;
                let d = if cmd == Cmd::NextSection { 1 } else { -1 };
                self.open_tab(TABS[(i + d).rem_euclid(TABS.len() as i32) as usize], ctx);
            }
            (Zone::List, Cmd::Up) => match cur {
                Some(0) | None => self.cycle(false),
                Some(i) => self.selected = Some(ids[i - 1].clone()),
            },
            (Zone::List, Cmd::Down) => {
                if let Some(i) = cur.filter(|i| i + 1 < ids.len()) {
                    self.selected = Some(ids[i + 1].clone());
                }
            }
            (Zone::List, Cmd::Right) => {
                if cur.is_some() {
                    self.zone = Zone::Actions;
                    self.index = 0;
                }
            }
            (Zone::List | Zone::Actions, Cmd::Favorite) => return self.run(Action::Favorite, ctx),
            (Zone::List | Zone::Actions, Cmd::Menu) => return self.run(Action::More, ctx),
            (Zone::List, Cmd::Accept) => return self.run(Action::Play, ctx),
            (Zone::Actions, Cmd::Left) => {
                if self.index == 0 {
                    self.zone = Zone::List;
                } else {
                    self.index -= 1;
                }
            }
            (Zone::Actions, Cmd::Right) => {
                let n = cur
                    .and_then(|i| ctx.library.find(&ids[i]))
                    .map(|g| self.actions(ctx, g).len())
                    .unwrap_or(1);
                self.index = (self.index + 1).min(n - 1);
            }
            (Zone::Actions, Cmd::Up) | (Zone::Actions, Cmd::Down) => self.zone = Zone::List,
            (Zone::Actions, Cmd::Accept) => {
                let action = cur
                    .and_then(|i| ctx.library.find(&ids[i]))
                    .and_then(|g| self.actions(ctx, g).get(self.index).copied());
                if let Some(a) = action {
                    return self.run(a, ctx);
                }
            }
            (Zone::Tabs, Cmd::Left) => self.index = self.index.saturating_sub(1),
            (Zone::Tabs, Cmd::Right) => self.index = (self.index + 1).min(TABS.len() - 1),
            (Zone::Tabs, Cmd::Accept) | (Zone::Tabs, Cmd::Down) => {
                self.open_tab(TABS[self.index.min(TABS.len() - 1)], ctx);
                self.cycle(true);
            }
            (Zone::Collections, Cmd::Left) => self.index = self.index.saturating_sub(1),
            (Zone::Collections, Cmd::Right) => {
                self.index = (self.index + 1).min(picks.len().saturating_sub(1))
            }
            (Zone::Collections, Cmd::Accept) => {
                if let Some((p, ..)) = picks.get(self.index) {
                    self.pick = Some(p.clone());
                    self.selected = None;
                }
            }
            (Zone::Collections, Cmd::Down) => self.zone = Zone::List,
            (Zone::Collections, Cmd::Up) => self.cycle(false),
            // Esc: limpa a busca e os filtros, depois volta para a lista.
            (_, Cmd::Back) => {
                if self.zone == Zone::List && self.filtered() {
                    self.clear();
                }
                self.zone = Zone::List;
            }
            _ => {}
        }
        Nav::Stay
    }

    fn key(&mut self, text: &str, mods: KeyModifiers, _ctx: &mut Ctx) -> Option<Nav> {
        let c = text.chars().next()?;
        // Shift+Tab chega como Tab com Shift ou como "back tab" (U+0019), conforme o sistema.
        if c == keys::TAB || c == '\u{0019}' {
            self.cycle(c == keys::TAB && mods != KeyModifiers::SHIFT);
            return Some(Nav::Stay);
        }
        None
    }

    fn click(&mut self, zone: &str, index: usize, ctx: &mut Ctx) -> Nav {
        match zone {
            "tab" => {
                if let Some(t) = TABS.get(index) {
                    self.open_tab(*t, ctx);
                }
                self.zone = Zone::List;
            }
            "collection" => {
                if let Some((p, ..)) = Self::picks(ctx).into_iter().nth(index) {
                    self.pick = Some(p);
                    self.selected = None;
                }
                self.zone = Zone::List;
            }
            "game" => self.select_index(ctx, index),
            "game-play" => {
                self.select_index(ctx, index);
                return self.run(Action::Play, ctx);
            }
            "game-menu" => {
                self.select_index(ctx, index);
                return self.run(Action::More, ctx);
            }
            "action" => {
                let action = self
                    .selected(ctx)
                    .and_then(|id| ctx.library.find(&id).map(|g| self.actions(ctx, g)))
                    .and_then(|a| a.get(index).copied());
                if let Some(a) = action {
                    self.zone = Zone::Actions;
                    self.index = index;
                    return self.run(a, ctx);
                }
            }
            "filter" => {
                return Nav::Layer(Box::new(dialogs::library_filters(
                    ctx,
                    self.criteria.clone(),
                    self.route_filter(),
                )));
            }
            "clear" => self.clear(),
            "add-game" => ctx.effect(Effect::AddGame),
            _ => {}
        }
        Nav::Stay
    }

    fn edited(&mut self, zone: &str, text: &str, ctx: &mut Ctx) {
        if zone == "search" {
            // O mesmo limite do teclado virtual: a busca é dado do usuário, não cresce sem fim.
            self.criteria.borrow_mut().search = text.chars().take(128).collect();
            ctx.dirty = true;
        }
    }

    fn render(&mut self, ctx: &Ctx, out: &mut Out) {
        let lang = ctx.lang();
        let games = self.games(ctx);
        let ids: Vec<String> = games.iter().map(|g| g.id.clone()).collect();
        let cur = self.current(&ids);
        out.set(
            "desk-tabs",
            vec![
                ctx.t("desktop.tab_all"),
                ctx.t("desktop.tab_favorites"),
                ctx.t("desktop.tab_collections"),
            ],
        );
        out.set(
            "desk-tab",
            TABS.iter().position(|t| *t == self.tab).unwrap_or(0) as i32,
        );
        let picks = Self::picks(ctx);
        let chips: Vec<NavItem> = picks
            .iter()
            .map(|(p, name, n)| NavItem {
                id: match p {
                    Pick::Manual(id) => format!("collection:{id}"),
                    Pick::Smart(id) => smart::route_of(id),
                },
                label: name.clone(),
                icon: if matches!(p, Pick::Smart(_)) {
                    "filter"
                } else {
                    "grid"
                }
                .into(),
                count: n.to_string(),
                section: String::new(),
                selected: self.pick.as_ref() == Some(p),
            })
            .collect();
        out.set("desk-collections", chips);
        let c = self.criteria.borrow().clone();
        out.set("desk-search", c.search.clone());
        out.set("desk-search-hint", ctx.t("desktop.search"));
        out.set("desk-filter-label", ctx.t("desktop.filter_sort"));
        out.set("desk-filter-count", c.active_filter_count() as i32);
        out.set("desk-clear-label", ctx.t("desktop.clear"));
        out.set("desk-filtered", c.has_search() || c.has_filters());
        out.set(
            "desk-count",
            format!(
                "{} · {}",
                library_text::game_count(lang, ids.len()),
                library_text::sort_label(lang, c.sort)
            ),
        );
        // Miniaturas só perto da área visível da lista (o tema avisa a primeira linha pelo `scrolled`).
        let first = ctx.list_first;
        let near = first.saturating_sub(NEAR / 3)..first + NEAR;
        let rows: Vec<DeskGame> = games
            .iter()
            .enumerate()
            .map(|(i, g)| DeskGame {
                id: g.id.clone(),
                title: g.title.clone(),
                thumb: tiles::image(g.shown_cover().filter(|_| near.contains(&i)), THUMB),
                accent: parse_color(&g.accent),
                favorite: g.is_favorite,
                running: ctx.running.as_ref().is_some_and(|r| r.id == g.id),
                installed: g.is_installed,
            })
            .collect();
        out.set("desk-games", rows);
        out.set("desk-selected", cur.map(|i| i as i32).unwrap_or(-1));
        let game = cur.and_then(|i| ctx.library.find(&ids[i]));
        let (view, actions, stats, details) = match game {
            Some(g) => {
                let mut overline = game_text::status(lang, g);
                if g.play_status.is_some() {
                    overline = format!(
                        "{overline} · {}",
                        library_text::status_label(lang, g.play_status)
                    );
                }
                let running = ctx.running.as_ref().is_some_and(|r| r.id == g.id);
                let actions: Vec<PanelRow> =
                    self.actions(ctx, g)
                        .into_iter()
                        .map(|a| match a {
                            Action::Play if running && ctx.launching => {
                                PanelRow::button("play", ctx.t("desktop.opening"))
                                    .icon("play")
                                    .enabled(false)
                            }
                            Action::Play => {
                                let mut row = PanelRow::button("play", tiles::play_label(ctx, g))
                                    .icon("play");
                                row.style = "primary".into();
                                row
                            }
                            Action::Favorite => {
                                let mut row = PanelRow::button(
                                    "favorite",
                                    ctx.t(if g.is_favorite {
                                        "details.unfavorite"
                                    } else {
                                        "details.favorite"
                                    }),
                                )
                                .icon(if g.is_favorite { "starfill" } else { "star" });
                                row.on = g.is_favorite;
                                row
                            }
                            Action::More => {
                                PanelRow::button("more", ctx.t("desktop.more")).icon("more")
                            }
                            Action::End => PanelRow::button("end", ctx.t("desktop.end_game"))
                                .icon("power")
                                .danger(),
                        })
                        .collect();
                let hint = if g.is_broken {
                    ctx.t("game.broken_text")
                } else {
                    g.note.clone().unwrap_or_default()
                };
                (
                    DeskGameView {
                        has_game: true,
                        title: g.title.clone(),
                        overline: overline.to_uppercase(),
                        description: g.description.clone(),
                        art: tiles::image(g.shown_hero(), tiles::HERO),
                        logo: tiles::image(g.shown_logo(), tiles::LOGO),
                        accent: parse_color(&g.accent),
                        hint,
                    },
                    actions,
                    summary(ctx, g),
                    details::stats(ctx, g),
                )
            }
            None => (DeskGameView::default(), Vec::new(), Vec::new(), Vec::new()),
        };
        out.set("desk-game", view);
        out.set("desk-actions", actions);
        out.set("desk-stats", stats);
        out.set("desk-details", details);
        out.set("desk-about-label", ctx.t("desktop.about"));
        out.set("desk-details-label", ctx.t("desktop.details"));
        out.set(
            "desk-zone",
            match self.zone {
                Zone::Tabs => "tabs",
                Zone::Collections => "collections",
                Zone::List => "list",
                Zone::Actions => "actions",
            },
        );
        out.set("desk-zone-index", self.index as i32);
        let (title, text) = if self.tab == Tab::Collections && picks.is_empty() {
            (
                ctx.t("desktop.no_collections"),
                ctx.t("desktop.no_collections_text"),
            )
        } else if self.tab == Tab::Collections && self.pick.is_none() {
            (ctx.t("desktop.pick_collection"), String::new())
        } else if c.has_search() || c.has_filters() {
            (
                ctx.t("library.none_found"),
                ctx.t("library.none_found_text"),
            )
        } else {
            (ctx.t("library.empty"), ctx.t("library.empty_text"))
        };
        out.set("desk-empty-title", title);
        out.set("desk-empty-text", text);
        out.set(
            "desk-empty-action",
            if ctx.library.games().is_empty() {
                ctx.t("desktop.add_game")
            } else {
                String::new()
            },
        );
    }

    fn hints(&self, _ctx: &Ctx) -> Vec<Hint> {
        Vec::new()
    }

    fn focused_game(&self, ctx: &Ctx) -> Option<String> {
        self.selected(ctx)
    }

    fn desktop(&self) -> Option<&DesktopLibrary> {
        Some(self)
    }

    fn focus_game(&mut self, id: &str, ctx: &Ctx) {
        // Se o jogo não aparece na aba atual (filtro, coleção), volta para Todos sem a busca.
        if !self.ids(ctx).iter().any(|i| i == id) {
            self.tab = Tab::All;
            self.clear();
        }
        self.selected = Some(id.to_owned());
        self.zone = Zone::List;
    }
}

/// Aberto no desktop com um controle conectado: pergunta uma vez se quer a tela cheia (ADR-0027). O SDL avisa
/// os controles já ligados nos primeiros quadros; depois de `ASK_WINDOW`, um controle novo não pergunta.
pub fn ask_big_picture(shell: &mut super::Shell, since_start: std::time::Duration) {
    use crate::mode::{ASK_SETTING, ASK_WINDOW};
    if !shell.ask_big_picture {
        return;
    }
    if since_start > ASK_WINDOW || !shell.ctx.desktop {
        shell.ask_big_picture = false;
        return;
    }
    if shell.ctx.pads_connected == 0 {
        return;
    }
    shell.ask_big_picture = false;
    if shell.ctx.settings.bool(ASK_SETTING, true) {
        let dialog = question(&shell.ctx);
        shell.push_layer(Box::new(dialog));
    }
}

/// "Controle conectado": ir para a tela cheia, continuar no desktop ou não perguntar de novo.
pub fn question(ctx: &Ctx) -> super::layer::MenuLayer {
    use super::layer::{Flow, MenuItem, MenuLayer};
    use crate::mode::{ASK_SETTING, UiMode};
    MenuLayer::dialog(ctx.t("desktop.ask.title"), |ctx| {
        let mut go = PanelRow::button("go", ctx.t("desktop.ask.go")).icon("gamepad");
        go.style = "primary".into();
        vec![
            MenuItem::new(go).on(|ctx| {
                ctx.effect(Effect::SwitchMode(UiMode::BigPicture));
                Flow::Close
            }),
            MenuItem::new(PanelRow::button("stay", ctx.t("desktop.ask.stay"))).on(|_| Flow::Close),
            MenuItem::new(PanelRow::button("never", ctx.t("desktop.ask.never"))).on(|ctx| {
                ctx.settings.set_bool(ASK_SETTING, false);
                Flow::Close
            }),
        ]
    })
    .text(ctx.t("desktop.ask.text"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::Shell;

    fn shell() -> (tempfile::TempDir, Shell) {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = crate::app::build_ctx(
            tako_core::paths::DataPaths::at(dir.path()),
            tako_platform::Platform::simulated(),
            true,
        );
        ctx.library.refresh();
        let mut shell = Shell::new(ctx);
        shell.enter_desktop();
        (dir, shell)
    }

    fn page(shell: &Shell) -> &DesktopLibrary {
        shell.desktop_library().unwrap()
    }

    #[test]
    fn click_selects_double_click_plays_and_the_keyboard_walks_the_list() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        assert_eq!(shell.page().kind(), "desk-library");
        let ids = page(&shell).ids(&shell.ctx);
        assert!(ids.len() > 3);
        // Sem nada escolhido, o primeiro jogo fica em destaque.
        assert_eq!(shell.focused_game().as_deref(), Some(ids[0].as_str()));
        shell.click("game", 2);
        assert_eq!(shell.focused_game().as_deref(), Some(ids[2].as_str()));
        // Passar o mouse não muda a seleção (só destaca, no tema).
        shell.hover("game", 0);
        assert_eq!(shell.focused_game().as_deref(), Some(ids[2].as_str()));
        shell.handle(Cmd::Down);
        assert_eq!(shell.focused_game().as_deref(), Some(ids[3].as_str()));
        shell.handle(Cmd::Up);
        shell.handle(Cmd::Up);
        assert_eq!(shell.focused_game().as_deref(), Some(ids[1].as_str()));
        // Tab vai para as ações; Enter na primeira joga.
        shell.key("\t", KeyModifiers::NONE);
        assert_eq!(page(&shell).zone(), Zone::Actions);
        let playable = shell
            .ctx
            .library
            .find(&ids[1])
            .is_some_and(|g| g.can_play() && !g.is_broken);
        shell.handle(Cmd::Accept);
        if playable {
            assert!(
                shell.ctx.effects.contains(&Effect::Launch(ids[1].clone())) || shell.has_layers(),
                "Jogar abre o jogo (ou a confirmação dele)"
            );
        }
        shell.close_layers();
        shell.ctx.effects.clear();
        // Duplo clique joga o jogo clicado.
        let installed = ids
            .iter()
            .position(|id| {
                shell
                    .ctx
                    .library
                    .find(id)
                    .is_some_and(|g| g.can_play() && !g.is_broken)
            })
            .unwrap();
        shell.click("game-play", installed);
        assert_eq!(
            shell.focused_game().as_deref(),
            Some(ids[installed].as_str())
        );
        assert!(
            shell
                .ctx
                .effects
                .contains(&Effect::Launch(ids[installed].clone()))
                || shell.has_layers()
        );
    }

    #[test]
    fn favorite_more_actions_and_the_context_menu() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        let id = shell.focused_game().unwrap();
        let before = shell.ctx.library.find(&id).unwrap().is_favorite;
        shell.click("action", 1);
        assert_ne!(shell.ctx.library.find(&id).unwrap().is_favorite, before);
        // ⋯: o menu com as Opções do jogo, mais coleções, tags e configurações deste jogo.
        shell.click("action", 2);
        assert!(shell.has_layers());
        let menu = dialogs::more_actions(&shell.ctx, &id);
        let keys: Vec<String> = (menu.build)(&shell.ctx)
            .into_iter()
            .map(|i| i.row.key)
            .collect();
        for k in ["status", "collections", "tags", "settings", "art", "ids"] {
            assert!(keys.iter().any(|x| x == k), "falta {k} em {keys:?}");
        }
        shell.close_layers();
        // Botão direito numa linha: seleciona e abre o mesmo menu.
        shell.click("game-menu", 3);
        assert!(shell.has_layers());
        assert_eq!(
            shell.focused_game(),
            Some(page(&shell).ids(&shell.ctx)[3].clone())
        );
    }

    #[test]
    fn tabs_search_filters_and_collections() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        let all = page(&shell).ids(&shell.ctx).len();
        let favorites = shell
            .ctx
            .library
            .games()
            .iter()
            .filter(|g| g.is_favorite)
            .count();
        shell.click("tab", 1);
        assert_eq!(page(&shell).tab(), Tab::Favorites);
        assert_eq!(page(&shell).ids(&shell.ctx).len(), favorites);
        shell.click("tab", 0);
        // Busca pelo campo de texto.
        let title = shell.ctx.library.games()[0].title.clone();
        shell.edited("search", &title);
        let found = page(&shell).ids(&shell.ctx);
        assert!(!found.is_empty() && found.len() < all);
        assert_eq!(shell.focused_game(), Some(found[0].clone()));
        // Esc limpa a busca.
        shell.handle(Cmd::Back);
        assert_eq!(page(&shell).ids(&shell.ctx).len(), all);
        // Filtrar e ordenar abre o painel de filtros da biblioteca.
        shell.click("filter", 0);
        assert!(shell.has_layers());
        shell.close_layers();

        // Coleções: vazia mostra o aviso; com uma do usuário e uma automática, as duas viram escolhas.
        shell.click("tab", 2);
        assert!(page(&shell).ids(&shell.ctx).is_empty());
        let first = shell.ctx.library.games()[0].id.clone();
        shell
            .ctx
            .collections
            .create("Sofá", Some(first.as_str()))
            .unwrap();
        shell.ctx.sync_collections();
        let mut list = smart::list(&shell.ctx);
        tako_core::library::smart::create(
            &mut list,
            "Favoritos",
            LibraryCriteria {
                favorites_only: true,
                ..Default::default()
            },
            LibraryFilter::All,
        )
        .unwrap();
        shell.ctx.settings.set(
            crate::shell::ctx::keys::SMART_COLLECTIONS,
            &tako_core::library::smart::to_json(&list),
        );
        shell.click("collection", 0);
        assert_eq!(page(&shell).ids(&shell.ctx), vec![first.clone()]);
        shell.click("collection", 1);
        assert_eq!(page(&shell).ids(&shell.ctx).len(), favorites);
    }
}
