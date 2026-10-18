//! Biblioteca do modo desktop (ADR-0027): abas Todos/Favoritos/Coleções, busca, filtros e ordenação, a lista de
//! jogos à esquerda e o destaque do jogo selecionado à direita. Mouse primeiro (clique seleciona, duplo clique joga,
//! botão direito abre as ações); o teclado anda pelas zonas com Tab e pelas linhas com as setas. As regras são as
//! da biblioteca da tela cheia (`query`, `dialogs`, `details::primary_action`); aqui só muda a apresentação.

use super::library::Criteria;
use super::{Cmd, Ctx, Effect, Nav, Out, Page, details, dialogs, keys, sidebar, smart, tiles};
use crate::model::{DeskCategory, DeskGame, DeskGameView, Hint, NavItem, PanelRow, Stat};
use crate::values::parse_color;
use std::cell::RefCell;
use std::rc::Rc;
use tako_core::input::shortcut::KeyModifiers;
use tako_core::library::{
    Game, GameStore, InstallFilter, LibraryCriteria, LibraryFacts, LibraryFilter, PlayStatus, query,
    smart::SmartCollection, tags,
};
use tako_core::presentation::{game_text, library_text};

/// Ícone da loja de origem na linha (ListaDeJogos › Loja de origem): o Xbox tem o logo próprio aqui.
fn store_icon(store: GameStore) -> &'static str {
    match store {
        GameStore::Xbox => "store-xbox",
        s => sidebar::store_icon(s),
    }
}

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

/// Modo de exibição da biblioteca (Lista · Grade · Categorias), salvo por perfil.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    List,
    Grid,
    Categories,
}

const VIEWS: [View; 3] = [View::List, View::Grid, View::Categories];
pub const VIEW_KEY: &str = "desktop.view";

impl View {
    fn key(self) -> &'static str {
        match self {
            View::List => "list",
            View::Grid => "grid",
            View::Categories => "categories",
        }
    }
    fn of(ctx: &Ctx) -> View {
        VIEWS
            .into_iter()
            .find(|v| ctx.settings.get(VIEW_KEY) == Some(v.key()))
            .unwrap_or(View::List)
    }
}

/// Como a BibliotecaCategorias agrupa as fileiras.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupBy {
    Status,
    Collection,
    Tag,
    Genre,
    Store,
    Install,
}

const GROUPS: [GroupBy; 6] = [
    GroupBy::Status,
    GroupBy::Collection,
    GroupBy::Tag,
    GroupBy::Genre,
    GroupBy::Store,
    GroupBy::Install,
];
pub const GROUP_KEY: &str = "desktop.group_by";

impl GroupBy {
    fn key(self) -> &'static str {
        match self {
            GroupBy::Status => "status",
            GroupBy::Collection => "collection",
            GroupBy::Tag => "tag",
            GroupBy::Genre => "genre",
            GroupBy::Store => "store",
            GroupBy::Install => "install",
        }
    }
    fn of(ctx: &Ctx) -> GroupBy {
        GROUPS
            .into_iter()
            .find(|g| ctx.settings.get(GROUP_KEY) == Some(g.key()))
            .unwrap_or(GroupBy::Status)
    }
}

/// "Ver todos ›" de uma fileira das Categorias: o filtro que a grade aplica.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SeeAll {
    Status(PlayStatus),
    Collection(String),
    Tag(String),
    Genre(String),
    Store(String),
    Install(InstallFilter),
}

/// Uma fileira das Categorias.
struct Group<'a> {
    title: String,
    icon: String,
    games: Vec<&'a Game>,
    see: SeeAll,
}

/// Capa da grade e das categorias (2:3; mostrada a 172 × 258).
const COVER: (u32, u32) = (344, 516);
/// Capas por fileira na BibliotecaCategorias; o resto fica no "Ver todos".
// shortcut: fileira sem virtualização horizontal, corta em 24 capas; virtualizar se pedirem a fileira inteira.
const ROW_COVERS: usize = 24;

/// Itens da barra superior na ordem do Tab: marca, guia Biblioteca, busca, Tela cheia.
const BAR_ITEMS: usize = 4;
const BAR_SEARCH: usize = 2;

/// Coleção escolhida na aba Coleções: do usuário ou automática (LIB-003).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Pick {
    Manual(String),
    Smart(String),
}

/// Onde está o foco do teclado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    /// Barra superior: marca (menu do Tako), guia Biblioteca, busca e Tela cheia.
    Bar,
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
    /// Cresce a cada pedido de foco na busca pelo teclado (o campo é do tema).
    search_focus: i32,
    /// Seleção múltipla (Ctrl/Shift + clique, Ctrl + A): os jogos marcados e o último clicado, para o intervalo.
    checked: std::collections::BTreeSet<String>,
    anchor: Option<usize>,
    /// Categorias: os jogos de cada fileira mostrada (o tema avisa cliques pelo índice na lista achatada) e a
    /// capa com o foco do teclado (fileira, coluna).
    cat_rows: Vec<Vec<String>>,
    cat_focus: (usize, usize),
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
            search_focus: 0,
            checked: Default::default(),
            anchor: None,
            cat_rows: Vec::new(),
            cat_focus: (0, 0),
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

    // ── Seleção múltipla (SelecaoMultipla) ──

    /// Ctrl + clique: marca ou desmarca um jogo.
    fn toggle_check(&mut self, ctx: &Ctx, index: usize) {
        let ids = self.ids(ctx);
        if let Some(id) = ids.get(index) {
            if !self.checked.remove(id) {
                self.checked.insert(id.clone());
            }
            self.anchor = Some(index);
        }
    }

    /// Shift + clique: marca do último clicado até este.
    fn check_range(&mut self, ctx: &Ctx, index: usize) {
        let ids = self.ids(ctx);
        if ids.is_empty() {
            return;
        }
        let from = self.anchor.or_else(|| self.current(&ids)).unwrap_or(index);
        let last = ids.len() - 1;
        let (a, b) = (from.min(index).min(last), from.max(index).min(last));
        for id in &ids[a..=b] {
            self.checked.insert(id.clone());
        }
        self.anchor = Some(index);
    }

    /// Os marcados que ainda estão na lista, na ordem dela.
    fn checked_ids(&self, ctx: &Ctx) -> Vec<String> {
        self.ids(ctx)
            .into_iter()
            .filter(|id| self.checked.contains(id))
            .collect()
    }

    /// Barra de ações da seleção: Coleção, Etiqueta, Favoritar, ⋯ e sair.
    fn bulk(&mut self, action: usize, ctx: &mut Ctx) -> Nav {
        let ids = self.checked_ids(ctx);
        if ids.is_empty() || action > 3 {
            self.checked.clear();
            self.anchor = None;
            return Nav::Stay;
        }
        match action {
            0 => Nav::Layer(Box::new(dialogs::choose_collections_many(ctx, ids))),
            1 => Nav::Layer(Box::new(dialogs::choose_tags(ctx, ids, None))),
            2 => {
                // Algum fora dos favoritos: favorita todos; senão tira todos.
                let fav = ids
                    .iter()
                    .any(|id| ctx.library.find(id).is_some_and(|g| !g.is_favorite));
                for id in &ids {
                    if ctx.library.find(id).is_some_and(|g| g.is_favorite != fav)
                        && let Err(e) = ctx.library.toggle_favorite(id)
                    {
                        tracing::error!("favoritar: {e}");
                    }
                }
                let n = ids.len().to_string();
                let key = if fav {
                    "desktop.bulk.favorited"
                } else {
                    "desktop.bulk.unfavorited"
                };
                ctx.toast(ctx.tf(key, &[("n", &n)]));
                Nav::Stay
            }
            _ => Nav::Layer(Box::new(dialogs::bulk_more(ids))),
        }
    }

    // ── Categorias (BibliotecaCategorias) ──

    /// Fileiras do agrupamento escolhido, na ordem dele; categoria vazia não aparece.
    fn groups<'a>(ctx: &Ctx, games: &[&'a Game]) -> Vec<Group<'a>> {
        let lang = ctx.lang();
        let mut out: Vec<Group<'a>> = Vec::new();
        let mut push = |title: String, icon: &str, see: SeeAll, keep: &dyn Fn(&Game) -> bool| {
            let v: Vec<&'a Game> = games.iter().copied().filter(|g| keep(g)).collect();
            if !v.is_empty() {
                out.push(Group {
                    title,
                    icon: icon.into(),
                    games: v,
                    see,
                });
            }
        };
        match GroupBy::of(ctx) {
            GroupBy::Status => {
                for s in [
                    PlayStatus::Playing,
                    PlayStatus::Wishlist,
                    PlayStatus::Paused,
                    PlayStatus::Beaten,
                    PlayStatus::Abandoned,
                ] {
                    let title = library_text::status_label(lang, Some(s));
                    push(title, "check", SeeAll::Status(s), &|g| g.play_status == Some(s));
                }
            }
            GroupBy::Collection => {
                for c in ctx.collections.collections() {
                    let see = SeeAll::Collection(c.id.clone());
                    push(c.name.clone(), "grid", see, &|g| c.contains(&g.id));
                }
            }
            GroupBy::Tag => {
                for t in tags::all(ctx.library.games()) {
                    let name = t.name.clone();
                    push(t.name, "tag", SeeAll::Tag(name.clone()), &|g| g.tags.contains(&name));
                }
            }
            GroupBy::Genre => {
                let mut genres: Vec<String> =
                    games.iter().flat_map(|g| g.genres.iter().cloned()).collect();
                genres.sort();
                genres.dedup();
                for genre in genres {
                    let see = SeeAll::Genre(genre.clone());
                    push(genre.clone(), "tag", see, &|g| g.genres.contains(&genre));
                }
            }
            GroupBy::Store => {
                for store in GameStore::ALL {
                    let name = game_text::store_name(lang, store);
                    push(name.clone(), store_icon(store), SeeAll::Store(name), &|g| {
                        g.store() == store
                    });
                }
            }
            GroupBy::Install => {
                push(
                    ctx.t("desktop.group.installed"),
                    "check",
                    SeeAll::Install(InstallFilter::Installed),
                    &|g| g.is_installed,
                );
                push(
                    ctx.t("desktop.group.not_installed"),
                    "folder",
                    SeeAll::Install(InstallFilter::NotInstalled),
                    &|g| !g.is_installed,
                );
            }
        }
        out
    }

    /// "Ver todos ›": a grade com o filtro da fileira.
    fn see_all(&mut self, see: SeeAll, ctx: &mut Ctx) {
        {
            let mut c = self.criteria.borrow_mut();
            match &see {
                SeeAll::Status(s) => c.statuses = [*s].into(),
                SeeAll::Tag(t) => c.tags = [t.clone()].into(),
                SeeAll::Genre(g) => c.genres = [g.clone()].into(),
                SeeAll::Store(s) => c.sources = [s.clone()].into(),
                SeeAll::Install(i) => c.install = *i,
                SeeAll::Collection(_) => {}
            }
        }
        if let SeeAll::Collection(id) = see {
            self.tab = Tab::Collections;
            self.pick = Some(Pick::Manual(id));
        } else {
            self.tab = Tab::All;
        }
        self.set_view(View::Grid, ctx);
    }

    fn set_view(&mut self, view: View, ctx: &mut Ctx) {
        ctx.settings.set(VIEW_KEY, view.key());
        self.zone = Zone::List;
        self.cat_focus = (0, 0);
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
        let mut v = vec![Zone::Bar, Zone::Tabs];
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
            // Grade: as setas andam pelas capas (a coluna vem do tema); Enter abre o jogo no destaque.
            (Zone::List, Cmd::Left | Cmd::Right | Cmd::Up | Cmd::Down | Cmd::Accept)
                if View::of(ctx) == View::Grid =>
            {
                let cols = ctx.grid_columns.max(1) as i32;
                let i = cur.unwrap_or(0) as i32;
                let to = match cmd {
                    Cmd::Left => i - 1,
                    Cmd::Right => i + 1,
                    Cmd::Up => i - cols,
                    Cmd::Down => i + cols,
                    _ => {
                        self.set_view(View::List, ctx);
                        return Nav::Stay;
                    }
                };
                if to < 0 {
                    self.cycle(false);
                } else if let Some(id) = ids.get(to as usize) {
                    self.selected = Some(id.clone());
                }
            }
            // Categorias: ←→ dentro da fileira, ↑↓ trocam de fileira mantendo a coluna mais próxima.
            (Zone::List, Cmd::Left | Cmd::Right | Cmd::Up | Cmd::Down | Cmd::Accept)
                if View::of(ctx) == View::Categories =>
            {
                let (r, c) = self.cat_focus;
                let rows = &self.cat_rows;
                let len = |r: usize| rows.get(r).map_or(0, Vec::len);
                self.cat_focus = match cmd {
                    Cmd::Left => (r, c.saturating_sub(1)),
                    Cmd::Right => (r, (c + 1).min(len(r).saturating_sub(1))),
                    Cmd::Up if r == 0 => {
                        self.cycle(false);
                        (r, c)
                    }
                    Cmd::Up => (r - 1, c.min(len(r - 1).saturating_sub(1))),
                    Cmd::Down if r + 1 < rows.len() => (r + 1, c.min(len(r + 1).saturating_sub(1))),
                    Cmd::Accept => {
                        self.set_view(View::List, ctx);
                        return Nav::Stay;
                    }
                    _ => (r, c),
                };
                if let Some(id) = self.cat_rows.get(self.cat_focus.0).and_then(|row| row.get(self.cat_focus.1)) {
                    self.selected = Some(id.clone());
                }
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
            (Zone::Bar, Cmd::Left) => self.index = self.index.saturating_sub(1),
            (Zone::Bar, Cmd::Right) => self.index = (self.index + 1).min(BAR_ITEMS - 1),
            (Zone::Bar, Cmd::Down) => self.zone = Zone::List,
            (Zone::Bar, Cmd::Accept) => match self.index {
                0 => return Nav::Layer(Box::new(brand_menu())),
                BAR_SEARCH => {
                    self.search_focus += 1;
                    self.zone = Zone::List;
                }
                3 => ctx.effect(Effect::SwitchMode(crate::mode::UiMode::BigPicture)),
                _ => self.zone = Zone::List,
            },
            (Zone::Tabs, Cmd::Up) => {
                self.zone = Zone::Bar;
                self.index = 0;
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
            // Esc: sai da seleção múltipla, depois limpa a busca e os filtros, depois volta para a lista.
            (_, Cmd::Back) if !self.checked.is_empty() => {
                self.checked.clear();
                self.anchor = None;
            }
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

    fn key(&mut self, text: &str, mods: KeyModifiers, ctx: &mut Ctx) -> Option<Nav> {
        let c = text.chars().next()?;
        // Ctrl + A: marca a lista filtrada inteira (SelecaoMultipla).
        if mods == KeyModifiers::CTRL && c.eq_ignore_ascii_case(&'a') && View::of(ctx) != View::Categories {
            self.checked = self.ids(ctx).into_iter().collect();
            return Some(Nav::Stay);
        }
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
            "game" => {
                self.checked.clear();
                self.anchor = Some(index);
                self.select_index(ctx, index);
            }
            "game-toggle" | "game-check" => self.toggle_check(ctx, index),
            "game-range" => self.check_range(ctx, index),
            "bulk" => return self.bulk(index, ctx),
            "view" => {
                if let Some(v) = VIEWS.get(index) {
                    self.set_view(*v, ctx);
                }
            }
            "view-menu" => return Nav::Layer(Box::new(view_menu())),
            "group-by" => return Nav::Layer(Box::new(group_menu())),
            "reverse" => {
                let mut c = self.criteria.borrow_mut();
                c.reversed = !c.reversed;
            }
            // Grade: clique abre o jogo no destaque; ▶ joga; botão direito abre as ações.
            "cover" => {
                self.checked.clear();
                self.select_index(ctx, index);
                self.set_view(View::List, ctx);
            }
            "cover-play" => {
                self.select_index(ctx, index);
                return self.run(Action::Play, ctx);
            }
            "cover-menu" => {
                self.select_index(ctx, index);
                return self.run(Action::More, ctx);
            }
            // Categorias: o índice é a posição na lista achatada das fileiras.
            "cat-cover" | "cat-cover-play" | "cat-cover-menu" => {
                let Some(id) = self.cat_rows.iter().flatten().nth(index).cloned() else {
                    return Nav::Stay;
                };
                self.selected = Some(id);
                match zone {
                    "cat-cover-play" => return self.run(Action::Play, ctx),
                    "cat-cover-menu" => return self.run(Action::More, ctx),
                    _ => self.set_view(View::List, ctx),
                }
            }
            "cat-all" => {
                let games = self.games(ctx);
                let see = Self::groups(ctx, &games).into_iter().nth(index).map(|g| g.see);
                if let Some(see) = see {
                    self.see_all(see, ctx);
                }
            }
            "rescan" => ctx.effect(Effect::Rescan),
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
        let view = View::of(ctx);
        // Seleção múltipla: só os marcados que continuam na lista contam.
        self.checked.retain(|id| ids.contains(id));
        let checked = self.checked.len();
        out.set(
            "desk-count",
            if checked > 0 {
                ctx.tf(
                    "desktop.bulk.count",
                    &[("n", &checked.to_string()), ("total", &ids.len().to_string())],
                )
            } else {
                format!(
                    "{} · {}",
                    library_text::game_count(lang, ids.len()),
                    library_text::sort_label(lang, c.sort)
                )
            },
        );
        out.set("desk-selecting", checked > 0);
        out.set("desk-reversed", c.reversed);
        out.set("desk-view", view.key());
        out.set(
            "desk-group-label",
            ctx.tf(
                "desktop.group.by",
                &[("group", &ctx.t(&format!("desktop.group.{}", GroupBy::of(ctx).key())))],
            ),
        );
        out.set(
            "desk-bulk",
            vec![
                PanelRow::button("collection", ctx.t("desktop.bulk.collection")).icon("grid"),
                PanelRow::button("tag", ctx.t("desktop.bulk.tag")).icon("tag"),
                PanelRow::button("favorite", ctx.t("desktop.bulk.favorite")).icon("star"),
                PanelRow::button("more", ctx.t("desktop.more")).icon("dots"),
                PanelRow::button("exit", ctx.t("desktop.bulk.exit")).icon("close"),
            ],
        );
        out.set("desk-loading", ctx.scanning && ctx.library.games().is_empty());
        // Miniaturas e capas só perto da área visível (o tema avisa a primeira linha pelo `scrolled`).
        let first = ctx.list_first;
        let near = first.saturating_sub(NEAR / 3)..first + NEAR;
        let row = |i: usize, g: &Game, near: bool, cover: bool| DeskGame {
            id: g.id.clone(),
            title: g.title.clone(),
            thumb: tiles::image(g.shown_cover().filter(|_| near && !cover), THUMB),
            cover: tiles::image(g.shown_cover().filter(|_| near && cover), COVER),
            accent: parse_color(&g.accent),
            favorite: g.is_favorite,
            running: ctx.running.as_ref().is_some_and(|r| r.id == g.id),
            installed: g.is_installed,
            store_icon: store_icon(g.store()).into(),
            store_name: game_text::store_name(lang, g.store()),
            checked: self.checked.contains(&g.id),
            index: i as i32,
        };
        let rows: Vec<DeskGame> = if view == View::Categories {
            Vec::new()
        } else {
            games
                .iter()
                .enumerate()
                .map(|(i, g)| row(i, g, near.contains(&i), view == View::Grid))
                .collect()
        };
        out.set("desk-games", rows);
        // Categorias: as fileiras perto da área visível pedem capa (o `scrolled` avisa a primeira fileira).
        let mut cats: Vec<DeskCategory> = Vec::new();
        self.cat_rows.clear();
        if view == View::Categories {
            let mut flat = 0;
            for (r, grp) in Self::groups(ctx, &games).into_iter().enumerate() {
                let shown: Vec<&Game> = grp.games.iter().copied().take(ROW_COVERS).collect();
                let near_row = r + 3 >= first && r <= first + 4;
                cats.push(DeskCategory {
                    count: library_text::game_count(lang, grp.games.len()),
                    title: grp.title,
                    icon: grp.icon,
                    games: shown
                        .iter()
                        .enumerate()
                        .map(|(i, g)| row(flat + i, g, near_row, true))
                        .collect(),
                });
                flat += shown.len();
                self.cat_rows.push(shown.iter().map(|g| g.id.clone()).collect());
            }
        }
        out.set("desk-categories", cats);
        out.set("desk-cat-focus-row", self.cat_focus.0 as i32);
        out.set("desk-cat-focus-col", self.cat_focus.1 as i32);
        out.set("desk-see-all", ctx.t("desktop.group.see_all"));
        out.set(
            "desk-cat-empty",
            ctx.tf(
                "desktop.group.empty",
                &[("group", &ctx.t(&format!("desktop.group.{}", GroupBy::of(ctx).key())))],
            ),
        );
        out.set("desk-loading-label", ctx.t("desktop.empty.loading"));
        out.set(
            "desk-view-labels",
            vec![
                ctx.t("desktop.view.list"),
                ctx.t("desktop.view.grid"),
                ctx.t("desktop.view.categories"),
                ctx.t("desktop.view.reverse"),
                ctx.t("desktop.filter_sort"),
                ctx.t("desktop.view.menu"),
            ],
        );
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
                let (art_blur, art_lum) = g
                    .shown_hero()
                    .and_then(crate::images::glass)
                    .unwrap_or_default();
                (
                    DeskGameView {
                        has_game: true,
                        title: g.title.clone(),
                        overline: overline.to_uppercase(),
                        description: g.description.clone(),
                        art: tiles::image(g.shown_hero(), tiles::HERO),
                        art_blur,
                        art_lum,
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
                Zone::Bar => "bar",
                Zone::Tabs => "tabs",
                Zone::Collections => "collections",
                Zone::List => "list",
                Zone::Actions => "actions",
            },
        );
        out.set("desk-zone-index", self.index as i32);
        out.set("desk-search-focus", self.search_focus);
        // ListaVazia: diz por quê e oferece a saída (zona do clique e o rótulo do botão).
        let none = String::new;
        let (title, text, action, zone) = if self.tab == Tab::Collections && picks.is_empty() {
            (
                ctx.t("desktop.no_collections"),
                ctx.t("desktop.no_collections_text"),
                none(),
                none(),
            )
        } else if self.tab == Tab::Collections && self.pick.is_none() {
            (ctx.t("desktop.pick_collection"), none(), none(), none())
        } else if c.has_search() || c.has_filters() {
            let mut active = Vec::new();
            if c.has_search() {
                active.push(format!("“{}”", c.search));
            }
            if c.has_filters() {
                active.push(ctx.tf(
                    "desktop.empty.filters",
                    &[("n", &c.active_filter_count().to_string())],
                ));
            }
            (
                ctx.t("desktop.empty.none_found"),
                active.join(" · "),
                ctx.t("desktop.empty.clear"),
                "clear".to_owned(),
            )
        } else if self.tab == Tab::Collections {
            (
                ctx.t("desktop.empty.collection"),
                ctx.t("desktop.empty.collection_text"),
                ctx.t("desktop.empty.add_games"),
                "tab".to_owned(),
            )
        } else if ctx.library.games().is_empty() {
            (
                ctx.t("desktop.empty.library"),
                ctx.t("desktop.empty.library_text"),
                ctx.t("desktop.empty.rescan"),
                "rescan".to_owned(),
            )
        } else {
            (ctx.t("desktop.empty.tab"), none(), none(), none())
        };
        out.set("desk-empty-title", title);
        out.set("desk-empty-text", text);
        out.set("desk-empty-action", action);
        out.set("desk-empty-zone", zone);
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
/// Modo de exibição (Lista · Grade · Categorias), pelo ícone da linha de contagem.
fn view_menu() -> super::layer::MenuLayer {
    use super::layer::{Flow, MenuItem, MenuLayer};
    MenuLayer::menu(String::new(), |ctx| {
        let current = View::of(ctx);
        [(View::List, "list"), (View::Grid, "grid"), (View::Categories, "layers")]
            .into_iter()
            .map(|(v, icon)| {
                let mut row =
                    PanelRow::button(v.key(), ctx.t(&format!("desktop.view.{}", v.key()))).icon(icon);
                if v == current {
                    row.value = "✓".into();
                }
                MenuItem::new(row).on(move |ctx| {
                    ctx.settings.set(VIEW_KEY, v.key());
                    Flow::Close
                })
            })
            .collect()
    })
}

/// "Agrupar por" das Categorias.
fn group_menu() -> super::layer::MenuLayer {
    use super::layer::{Flow, MenuItem, MenuLayer};
    MenuLayer::menu(String::new(), |ctx| {
        let current = GroupBy::of(ctx);
        GROUPS
            .into_iter()
            .map(|g| {
                let mut row = PanelRow::button(g.key(), ctx.t(&format!("desktop.group.{}", g.key())));
                if g == current {
                    row.value = "✓".into();
                }
                MenuItem::new(row).on(move |ctx| {
                    ctx.settings.set(GROUP_KEY, g.key());
                    Flow::Close
                })
            })
            .collect()
    })
}

/// Menu da marca (MenuDoTako), como o menu "Steam": Configurações e as ações da conta e do app, fora das guias.
pub fn brand_menu() -> super::layer::MenuLayer {
    use super::layer::{Flow, MenuItem, MenuLayer};
    MenuLayer::menu(String::new(), |ctx| {
        vec![
            MenuItem::new(
                PanelRow::button("settings", ctx.t("desktop.menu.settings"))
                    .icon("gear")
                    .value(ctx.t("desktop.menu.settings_key")),
            )
            .on(|ctx| {
                ctx.open_route = Some("settings".into());
                Flow::CloseAll
            }),
            MenuItem::new(
                PanelRow::button("profile", ctx.t("desktop.menu.switch_profile")).icon("user"),
            )
            .on(|ctx| Flow::Replace(Box::new(super::profiles::panel(ctx)))),
            MenuItem::new(
                PanelRow::button("big-picture", ctx.t("desktop.menu.big_picture"))
                    .icon("tv")
                    .value(ctx.t("desktop.menu.big_picture_key")),
            )
            .on(|ctx| {
                ctx.effect(Effect::SwitchMode(crate::mode::UiMode::BigPicture));
                Flow::CloseAll
            }),
            MenuItem::new(PanelRow::separator()),
            MenuItem::new(
                PanelRow::button("updates", ctx.t("desktop.menu.check_updates")).icon("info"),
            )
            .on(|ctx| {
                ctx.effect(Effect::CheckUpdates);
                Flow::CloseAll
            }),
            MenuItem::new(PanelRow::separator()),
            MenuItem::new(
                PanelRow::button("quit", ctx.t("desktop.menu.quit"))
                    .icon("power")
                    .value(ctx.t("desktop.menu.quit_key")),
            )
            .on(|ctx| {
                ctx.effect(Effect::ExitToDesktop);
                Flow::CloseAll
            }),
        ]
    })
}

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
    fn the_brand_opens_the_tako_menu_and_the_bar_is_the_first_keyboard_zone() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        // Menu da marca: Configurações abre a página; Tela cheia troca de modo.
        shell.click("desk-brand", 0);
        assert!(shell.has_layers());
        shell.click("overlay-row", 0);
        assert!(!shell.has_layers());
        assert_eq!(shell.page().kind(), "settings");
        shell.click("desk-nav", 0);
        shell.click("desk-brand", 0);
        shell.click("overlay-row", 2);
        assert_eq!(
            shell.ctx.effects.pop(),
            Some(Effect::SwitchMode(crate::mode::UiMode::BigPicture))
        );
        // Separadores não recebem foco: ↓ pula de "Tela cheia" para "Procurar atualizações".
        let rows = crate::shell::layer::Layer::render(&brand_menu(), &shell.ctx).rows;
        assert_eq!(rows[3].kind, "separator");
        assert!(!rows[3].is_focusable());

        // Teclado: subir das abas chega à barra; Enter na busca pede o foco do campo.
        shell.handle(Cmd::Up);
        shell.handle(Cmd::Up);
        while page(&shell).zone() != Zone::Tabs {
            shell.key("\t", KeyModifiers::NONE);
        }
        shell.handle(Cmd::Up);
        assert_eq!(page(&shell).zone(), Zone::Bar);
        shell.handle(Cmd::Right);
        shell.handle(Cmd::Right);
        shell.handle(Cmd::Accept);
        assert_eq!(page(&shell).search_focus, 1);
        assert_eq!(page(&shell).zone(), Zone::List);
    }

    #[test]
    fn ctrl_and_shift_mark_games_and_the_bar_acts_on_all_of_them() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        let ids = page(&shell).ids(&shell.ctx);
        shell.click("game", 0);
        shell.click("game-toggle", 1);
        shell.click("game-range", 3);
        assert_eq!(page(&shell).checked_ids(&shell.ctx), ids[1..=3].to_vec());
        // Favoritar em massa: algum fora dos favoritos → todos ficam favoritos.
        shell.click("bulk", 2);
        assert!(ids[1..=3]
            .iter()
            .all(|id| shell.ctx.library.find(id).unwrap().is_favorite));
        // O destaque continua no último jogo clicado sem modificador.
        assert_eq!(shell.focused_game().as_deref(), Some(ids[0].as_str()));
        // Esc sai da seleção antes de qualquer outra coisa; Ctrl + A marca a lista inteira.
        shell.handle(Cmd::Back);
        assert!(page(&shell).checked_ids(&shell.ctx).is_empty());
        shell.key("a", KeyModifiers::CTRL);
        assert_eq!(page(&shell).checked_ids(&shell.ctx), ids);
        shell.click("bulk", 4);
        assert!(page(&shell).checked_ids(&shell.ctx).is_empty());
    }

    #[test]
    fn grid_and_categories_show_the_same_games_and_see_all_filters_the_grid() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        let ids = page(&shell).ids(&shell.ctx);
        shell.click("view", 1);
        assert_eq!(View::of(&shell.ctx), View::Grid);
        // Setas andam pelas colunas que o tema mede; Enter volta ao destaque com o jogo escolhido.
        shell.ctx.grid_columns = 4;
        shell.click("cover", 0);
        assert_eq!(View::of(&shell.ctx), View::List);
        shell.click("view", 1);
        shell.handle(Cmd::Down);
        assert_eq!(shell.focused_game().as_deref(), Some(ids[4].as_str()));
        // Categorias por loja: cada jogo em uma fileira; "Ver todos" abre a grade filtrada.
        shell.ctx.settings.set(GROUP_KEY, GroupBy::Store.key());
        shell.click("view", 2);
        shell.render(std::time::Instant::now());
        let rows = page(&shell).cat_rows.clone();
        assert_eq!(rows.iter().map(Vec::len).sum::<usize>(), ids.len().min(rows.len() * ROW_COVERS));
        let first_store = shell.ctx.library.find(&rows[0][0]).unwrap().store();
        shell.click("cat-all", 0);
        assert_eq!(View::of(&shell.ctx), View::Grid);
        assert!(page(&shell)
            .games(&shell.ctx)
            .iter()
            .all(|g| g.store() == first_store));
        // Inverter a ordem: a mesma lista de trás para a frente.
        page(&shell).criteria.borrow_mut().sources.clear();
        shell.click("reverse", 0);
        let mut back = page(&shell).ids(&shell.ctx);
        back.reverse();
        assert_eq!(back, ids);
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
