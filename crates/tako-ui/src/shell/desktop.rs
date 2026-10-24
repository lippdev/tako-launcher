//! Biblioteca do modo desktop (ADR-0027): abas Todos/Favoritos/Coleções, busca, filtros e ordenação, a lista de
//! jogos à esquerda e o destaque do jogo selecionado à direita. Mouse primeiro (clique seleciona, duplo clique joga,
//! botão direito abre as ações); o teclado anda pelas zonas com Tab e pelas linhas com as setas. As regras são as
//! da biblioteca da tela cheia (`query`, `dialogs`, `details::primary_action`); aqui só muda a apresentação.

use super::ctx::keys::TITLE_LOGO;
use super::library::Criteria;
use super::overview::{self, Target};
use super::{
    Cmd, Ctx, Effect, InputKind, Nav, Out, Page, bar, details, dialogs, keys, sidebar, smart, tiles,
};
use crate::model::{
    DeskBlock, DeskCategory, DeskGame, DeskGameView, DeskInfo, DeskOverview, DeskPage, Hint,
    NavItem, PanelRow,
};
use crate::values::parse_color;
use std::cell::RefCell;
use std::rc::Rc;
use tako_core::input::shortcut::KeyModifiers;
use tako_core::library::{
    Game, GameStore, InstallFilter, LibraryCriteria, LibraryFacts, LibraryFilter, PlayStatus,
    query, smart::SmartCollection, tags,
};
use tako_core::presentation::{game_text, library_text};

/// Ícone da loja de origem na linha (ListaDeJogos › Loja de origem): o Xbox tem o logo próprio aqui.
pub(super) fn store_icon(store: GameStore) -> &'static str {
    match store {
        GameStore::Xbox => "store-xbox",
        GameStore::Manual => "store-manual",
        s => sidebar::store_icon(s),
    }
}

/// Rota da biblioteca desktop.
pub const ROUTE: &str = "desk:library";

/// Miniatura de capa do perfil (o quadrado do topo da capa 2:3, a 40 × 40; 2× para telas densas).
pub(crate) const THUMB: (u32, u32) = (80, 120);
/// Ícone da linha da lista (28 na tela, 2× para telas densas) e, com a lista recolhida, o de 46 (92 px; 96 aqui).
const ICON: u32 = 56;
const ICON_COLLAPSED: u32 = 96;
/// Linhas em volta da primeira visível que pedem miniatura.
const NEAR: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    All,
    Favorites,
    Installed,
    /// Sem aba própria: entra pelo menu Ordenar, ver e filtrar (seção Coleção) e pelo Perfil.
    Collections,
}

const TABS: [Tab; 3] = [Tab::All, Tab::Favorites, Tab::Installed];

/// Modo de exibição da biblioteca (Lista · Grade · Categorias), salvo por perfil.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    List,
    Grid,
    Categories,
}

const VIEWS: [View; 3] = [View::List, View::Grid, View::Categories];
pub const VIEW_KEY: &str = "desktop.view";
/// ListaDeJogos recolhida (Ctrl + B): 76 de largura, só os ícones; salvo por perfil.
const COLLAPSED_KEY: &str = "desktop.list_collapsed";

fn collapsed(ctx: &Ctx) -> bool {
    ctx.settings.bool(COLLAPSED_KEY, false)
}

impl View {
    fn key(self) -> &'static str {
        match self {
            View::List => "list",
            View::Grid => "grid",
            View::Categories => "categories",
        }
    }
    fn of(ctx: &Ctx) -> View {
        let view = VIEWS
            .into_iter()
            .find(|v| ctx.settings.get(VIEW_KEY) == Some(v.key()))
            .unwrap_or(View::List);
        // Recolhida, a Grade vira a lista de ícones (como no protótipo).
        if view == View::Grid && collapsed(ctx) {
            View::List
        } else {
            view
        }
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

/// Lojas fechadas na seção NÃO INSTALADOS (ids separados por vírgula), guardadas por perfil como a escolha de modo.
pub const CLOSED_KEY: &str = "desktop.closed_stores";

/// Uma vaga da lista: jogo (índice na lista arrumada), a divisória NÃO INSTALADOS ou o cabeçalho de uma loja.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Entry {
    Game(usize),
    Section,
    Group(GameStore),
}

/// Jogos não instalados de uma mesma loja: `start..end` na lista arrumada.
struct Run {
    store: GameStore,
    start: usize,
    end: usize,
}

/// Ícone da linha e, se não houver, a inicial do nome. Carrega o ícone recortado via [`crate::images::icon`].
/// `load: false` = linha fora da área visível (não pede nada).
fn row_icon(g: &Game, load: bool, collapsed: bool) -> (slint::Image, String) {
    let uri = g.shown_icon();
    // Já recortado e com contorno no cache: 56 serve à tela normal e à de alta densidade; recolhida pede 96.
    let size = if collapsed { ICON_COLLAPSED } else { ICON };
    let icon = uri
        .filter(|_| load)
        .and_then(|u| crate::images::icon(u, size, !g.is_installed))
        .unwrap_or_default();
    // A inicial fica por baixo enquanto o ícone espera na fila (pode demorar: vem da rede); o ícone a cobre ao chegar.
    let initial = if icon.size().width > 0 {
        String::new()
    } else {
        g.title
            .trim()
            .chars()
            .next()
            .map(|c| c.to_uppercase().collect())
            .unwrap_or_default()
    };
    (icon, initial)
}

/// Entra na seção NÃO INSTALADOS: jogo de uma loja que ainda não está no computador. ROM ou arquivo que sumiu e
/// lançamento futuro não têm o que instalar e ficam onde estavam.
fn to_install(g: &Game) -> bool {
    !g.is_installed
        && !g.is_upcoming()
        && !matches!(g.store(), GameStore::Manual | GameStore::Emulators)
}

fn store_slot(store: GameStore) -> usize {
    GameStore::ALL
        .iter()
        .position(|s| *s == store)
        .unwrap_or_default()
}

/// Instalados primeiro; depois os não instalados, um trecho por loja, a com mais jogos na frente. A ordenação
/// escolhida vale dentro de cada trecho (a ordenação é estável).
fn arrange(mut games: Vec<&Game>) -> Vec<&Game> {
    let mut count = [0usize; GameStore::ALL.len()];
    for g in games.iter().filter(|g| to_install(g)) {
        count[store_slot(g.store())] += 1;
    }
    games.sort_by_key(|g| {
        let slot = store_slot(g.store());
        if !to_install(g) {
            (0, std::cmp::Reverse(0), 0)
        } else {
            (1, std::cmp::Reverse(count[slot]), slot)
        }
    });
    games
}

fn runs(games: &[&Game]) -> Vec<Run> {
    let mut out: Vec<Run> = Vec::new();
    for (i, g) in games.iter().enumerate().filter(|(_, g)| to_install(g)) {
        match out.last_mut() {
            Some(r) if r.store == g.store() => r.end = i + 1,
            _ => out.push(Run {
                store: g.store(),
                start: i,
                end: i + 1,
            }),
        }
    }
    out
}

fn group_closed(ctx: &Ctx, store: GameStore) -> bool {
    ctx.settings
        .string(CLOSED_KEY)
        .split(',')
        .any(|s| s == store.id_prefix())
}

/// Abre (`Some(true)`), fecha (`Some(false)`) ou alterna (`None`) a loja da seção.
fn set_group(ctx: &mut Ctx, store: GameStore, open: Option<bool>) {
    let key = store.id_prefix();
    let raw = ctx.settings.string(CLOSED_KEY);
    let mut closed: Vec<&str> = raw
        .split(',')
        .filter(|s| !s.is_empty() && *s != key)
        .collect();
    if !open.unwrap_or_else(|| group_closed(ctx, store)) {
        closed.push(key);
    }
    ctx.settings.set(CLOSED_KEY, &closed.join(","));
}

/// A lista na tela: os instalados, a divisória e, por loja, o cabeçalho com os jogos (se aberta). Sem instalados em
/// cima a divisória não separa nada e fica de fora (filtro "Só não instalados").
fn entries(ctx: &Ctx, games: &[&Game]) -> Vec<Entry> {
    let runs = runs(games);
    let installed = games.len() - runs.iter().map(|r| r.end - r.start).sum::<usize>();
    let mut out: Vec<Entry> = (0..installed).map(Entry::Game).collect();
    if installed > 0 && !runs.is_empty() {
        out.push(Entry::Section);
    }
    for r in &runs {
        out.push(Entry::Group(r.store));
        if !group_closed(ctx, r.store) {
            out.extend((r.start..r.end).map(Entry::Game));
        }
    }
    out
}

/// A grade em linhas de posições da lista: cada cabeçalho tem a sua e as capas quebram a cada `cols`.
fn grid_rows(list: &[Entry], cols: usize) -> Vec<Vec<usize>> {
    let mut rows: Vec<Vec<usize>> = Vec::new();
    for (i, e) in list.iter().enumerate() {
        match rows.last_mut() {
            Some(last)
                if matches!(e, Entry::Game(_))
                    && last.len() < cols
                    && matches!(list[last[0]], Entry::Game(_)) =>
            {
                last.push(i)
            }
            _ => rows.push(vec![i]),
        }
    }
    rows
}

/// A vaga vizinha que aceita foco (a divisória não aceita).
fn neighbor(entries: &[Entry], from: Option<usize>, forward: bool) -> Option<usize> {
    let focusable = |i: &usize| entries[*i] != Entry::Section;
    if forward {
        (from.map_or(0, |f| f + 1)..entries.len()).find(focusable)
    } else {
        (0..from.unwrap_or(0)).rev().find(focusable)
    }
}

/// Rota que abre a biblioteca numa coleção (`desk:collection:m:<id>` ou `s:<id>`, a chave do SeletorDeColecao).
pub const COLLECTION_ROUTE: &str = "desk:collection:";

/// Coleção escolhida na aba Coleções: do usuário ou automática (LIB-003).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Pick {
    Manual(String),
    Smart(String),
}

/// Última coleção da aba Coleções (SeletorDeColecao: voltar à aba reabre a última).
const COLLECTION_KEY: &str = "DesktopCollection";

impl Pick {
    fn key(&self) -> String {
        match self {
            Pick::Manual(id) => format!("m:{id}"),
            Pick::Smart(id) => format!("s:{id}"),
        }
    }
}

/// Onde está o foco do teclado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    /// Barra superior: marca (menu do Tako), guia Biblioteca, busca e Tela cheia.
    Bar,
    Tabs,
    /// O topo da lista: Ordenar, ver e filtrar e a linha Visão geral (ou o chip da coleção).
    Controls,
    List,
    Actions,
    /// O corpo da página do jogo (abaixo do destaque): ↓ a partir das ações entra e rola, ↑ rola de volta e, no topo, volta.
    Body,
}

/// Os controles da zona `Controls`, na ordem da tela (o índice da zona).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Control {
    Options,
    /// Lista recolhida: a lupa no lugar do botão de opções (expande a lista e foca a busca).
    Search,
    /// O botão de recolher e expandir a lista (Ctrl + B).
    Collapse,
    Overview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Play,
    Favorite,
    More,
    /// Configurações deste jogo (a engrenagem).
    Settings,
    End,
}

pub struct DesktopLibrary {
    tab: Tab,
    pick: Option<Pick>,
    /// Coleção escolhida no select (SeletorDeColecao): a camada do select deixa aqui e a página aplica.
    pick_request: Rc<RefCell<Option<Pick>>>,
    /// Busca e filtros do usuário, por cima da aba (o painel de filtros é o mesmo da tela cheia).
    pub criteria: Criteria,
    selected: Option<String>,
    selection_index: usize,
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
    /// TrocaDeJogo: o jogo no destaque e a seleção do teclado esperando parar (id, desde quando).
    hero: Option<String>,
    hero_wait: Option<(String, std::time::Instant)>,
    /// Page Down soma 1 e Page Up tira 1 (uma tela por passo); ↓ e ↑ no corpo somam e tiram `BODY_STEP`. O tema rola a
    /// página do jogo pela diferença.
    page_scroll: f32,
    /// Onde o tema diz que a página está (0 topo, 1 no meio, 2 no fim): ↑ no topo sai do corpo e ↓ no fim não rola.
    page_edge: usize,
    /// Cabeçalho de loja com o foco do teclado (o destaque segue no último jogo escolhido).
    group_focus: Option<GameStore>,
    /// Visão geral (TelaBiblioteca) no lugar do destaque do jogo: o primeiro lugar da biblioteca; escolher um jogo sai
    /// dela. Com ela aberta, a zona `Actions` é o conteúdo da Visão geral e `index` anda pelos alvos dela.
    overview: bool,
}

/// Page Up e Page Down como o Slint manda.
const PAGE_UP: char = '\u{F72C}';
const PAGE_DOWN: char = '\u{F72D}';

/// Quanto ↓ e ↑ rolam o corpo da página do jogo, em telas.
const BODY_STEP: f32 = 0.3;

/// Teclado rápido: só a seleção anda; arte e texto trocam quando ela para por este tempo.
const HERO_SETTLE: std::time::Duration = std::time::Duration::from_millis(150);

impl Default for DesktopLibrary {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn game_card_accent(_ctx: &Ctx, game_accent: &str) -> slint::Color {
    parse_color(game_accent)
}

impl DesktopLibrary {
    pub fn new() -> Self {
        DesktopLibrary {
            tab: Tab::All,
            pick: None,
            pick_request: Rc::default(),
            criteria: Rc::new(RefCell::new(LibraryCriteria::default())),
            selected: None,
            selection_index: 0,
            zone: Zone::Actions,
            index: 0,
            search_focus: 0,
            checked: Default::default(),
            anchor: None,
            cat_rows: Vec::new(),
            cat_focus: (0, 0),
            hero: None,
            hero_wait: None,
            page_scroll: 0.,
            page_edge: 0,
            group_focus: None,
            overview: true,
        }
    }

    /// O jogo do destaque: com o teclado, o anterior fica até a seleção parar por `HERO_SETTLE`; com o controle, um
    /// clique (`select_index` limpa `hero`) ou na primeira vez, na hora.
    fn hero_id(&mut self, ctx: &Ctx, selected: Option<String>) -> Option<String> {
        let now = std::time::Instant::now();
        let waiting = match &self.hero_wait {
            Some((id, since)) if Some(id) == selected.as_ref() => now - *since < HERO_SETTLE,
            _ => true,
        };
        if selected == self.hero
            || self.hero.is_none()
            || ctx.input != InputKind::Keyboard
            || !waiting
        {
            self.hero = selected;
            self.hero_wait = None;
            return self.hero.clone();
        }
        if self.hero_wait.as_ref().map(|(id, _)| id) != selected.as_ref() {
            self.hero_wait = selected.map(|id| (id, now));
            slint::Timer::single_shot(HERO_SETTLE, || {
                if let Some(shell) = crate::tasks::shell() {
                    shell.borrow_mut().ctx.dirty = true;
                }
            });
        }
        self.hero.clone()
    }

    pub fn tab(&self) -> Tab {
        self.tab
    }

    /// Volta à Visão geral, com o foco do teclado no destaque.
    pub fn show_overview(&mut self) {
        self.overview = true;
        self.zone = Zone::Actions;
        self.index = 0;
    }

    /// A Visão geral está na tela: na Lista e na Grade (as Categorias tomam a janela toda) e fora das coleções (o chip
    /// da coleção toma o lugar da linha).
    fn overview_on(&self, ctx: &Ctx) -> bool {
        self.overview && View::of(ctx) != View::Categories && self.tab != Tab::Collections
    }

    pub fn zone(&self) -> Zone {
        self.zone
    }

    /// O tema avisa onde a página do jogo está (`Shell::click` com "page-edge").
    pub fn set_page_edge(&mut self, edge: usize) {
        self.page_edge = edge;
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
            Tab::Installed => LibraryFilter::Installed,
            _ => LibraryFilter::All,
        }
    }

    /// Os jogos da lista, na ordem da tela.
    pub fn games<'a>(&self, ctx: &'a Ctx) -> Vec<&'a Game> {
        let user = self.criteria.borrow();
        let games = ctx.library.games();
        arrange(match (&self.tab, &self.pick) {
            (Tab::All, _) => query::apply(games, LibraryFilter::All, &user),
            (Tab::Favorites, _) => query::apply(games, LibraryFilter::Favorites, &user),
            (Tab::Installed, _) => {
                let base = LibraryCriteria {
                    install: InstallFilter::Installed,
                    ..Default::default()
                };
                let base = query::apply(games, LibraryFilter::All, &base);
                query::apply_by(base, |g| LibraryFacts::of(g), &user)
            }
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
        })
    }

    fn active_filters(&self, ctx: &Ctx) -> Vec<PanelRow> {
        let c = self.criteria.borrow();
        let mut rows = Vec::new();
        let mut add = |id: String, label: String| {
            let remove = ctx.tf("desktop.remove_filter", &[("filter", &label)]);
            rows.push(PanelRow::button(id, label).desc(remove));
        };
        for status in &c.statuses {
            add(
                format!("status:{}", status.key()),
                library_text::status_label(ctx.lang(), Some(*status)),
            );
        }
        if c.no_status {
            add(
                "no-status".into(),
                library_text::status_label(ctx.lang(), None),
            );
        }
        if c.install != InstallFilter::Any {
            add(
                "install".into(),
                library_text::install_label(ctx.lang(), c.install),
            );
        }
        for (on, id, key) in [
            (c.favorites_only, "favorites", "filters.favorites_only"),
            (c.played_only, "played", "filters.played"),
            (c.broken_only, "broken", "filters.broken"),
        ] {
            if on {
                add(id.into(), ctx.t(key));
            }
        }
        if c.play_time != tako_core::library::criteria::PlayTimeFilter::Any {
            add("play-time".into(), ctx.t(c.play_time.key()));
        }
        for (kind, values) in [
            ("source", &c.sources),
            ("genre", &c.genres),
            ("tag", &c.tags),
            ("collection", &c.collections),
        ] {
            for value in values {
                let label = if kind == "collection" {
                    ctx.collections
                        .collections()
                        .iter()
                        .find(|col| col.id == *value)
                        .map(|col| col.name.clone())
                        .unwrap_or_else(|| value.clone())
                } else {
                    value.clone()
                };
                add(format!("{kind}:{value}"), label);
            }
        }
        rows
    }

    fn remove_filter(&mut self, ctx: &Ctx, index: usize) {
        let Some(row) = self.active_filters(ctx).get(index).cloned() else {
            return;
        };
        let mut c = self.criteria.borrow_mut();
        match row.key.as_str() {
            "no-status" => c.no_status = false,
            "install" => c.install = InstallFilter::Any,
            "favorites" => c.favorites_only = false,
            "played" => c.played_only = false,
            "broken" => c.broken_only = false,
            "play-time" => c.play_time = tako_core::library::criteria::PlayTimeFilter::Any,
            id => {
                if let Some((kind, value)) = id.split_once(':') {
                    match kind {
                        "status" => c.statuses.retain(|s| s.key() != value),
                        "source" => {
                            c.sources.remove(value);
                        }
                        "genre" => {
                            c.genres.remove(value);
                        }
                        "tag" => {
                            c.tags.remove(value);
                        }
                        "collection" => {
                            c.collections.remove(value);
                        }
                        _ => {}
                    }
                }
            }
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
            .or_else(|| {
                (!ids.is_empty()).then(|| {
                    if self.selected.is_some() {
                        self.selection_index.min(ids.len() - 1)
                    } else {
                        0
                    }
                })
            })
    }

    pub fn selected(&self, ctx: &Ctx) -> Option<String> {
        let ids = self.ids(ctx);
        self.current(&ids).map(|i| ids[i].clone())
    }

    fn select_index(&mut self, ctx: &Ctx, index: usize) {
        // Clique: o destaque troca na hora, sem a espera do teclado.
        self.hero = None;
        let ids = self.ids(ctx);
        if let Some(id) = ids.get(index) {
            self.selected = Some(id.clone());
        }
        self.group_focus = None;
        self.zone = Zone::List;
        self.overview = false;
    }

    /// Onde está o foco na lista: o cabeçalho de loja ou o jogo escolhido.
    fn focus_pos(&self, list: &[Entry], cur: Option<usize>) -> Option<usize> {
        let at = self
            .group_focus
            .map(Entry::Group)
            .or(cur.map(Entry::Game))?;
        list.iter().position(|e| *e == at)
    }

    fn focus(&mut self, entry: Entry, ids: &[String]) {
        match entry {
            Entry::Game(i) => {
                if let Some(id) = ids.get(i) {
                    self.selected = Some(id.clone());
                }
                self.group_focus = None;
                self.overview = false;
            }
            Entry::Group(store) => self.group_focus = Some(store),
            Entry::Section => {}
        }
    }

    /// Abre a aba Coleções na coleção `key` (o Perfil › Coleções manda para cá).
    pub fn open_collection(&mut self, key: &str, ctx: &mut Ctx) {
        ctx.settings.set(COLLECTION_KEY, key);
        self.pick = None;
        self.selected = None;
        self.open_tab(Tab::Collections, ctx);
        self.zone = Zone::List;
    }

    pub fn open_tab(&mut self, tab: Tab, ctx: &Ctx) {
        self.tab = tab;
        if tab == Tab::Collections && self.pick.is_none() {
            let saved = ctx.settings.string(COLLECTION_KEY);
            let picks = Self::picks(ctx);
            self.pick = picks
                .iter()
                .find(|(p, ..)| p.key() == saved)
                .or(picks.first())
                .map(|(p, ..)| p.clone());
        }
    }

    /// SeletorDeColecao: as coleções com a quantidade (e "Auto" nas automáticas) e ✓ na atual; depois "Nova
    /// coleção…" e "Organizar coleções".
    fn collection_select(&self, ctx: &Ctx) -> super::layer::MenuLayer {
        let request = self.pick_request.clone();
        let current = self.pick.clone();
        let focus = Self::picks(ctx)
            .iter()
            .position(|(p, ..)| Some(p) == current.as_ref())
            .unwrap_or(0);
        super::layer::MenuLayer::menu(String::new(), move |ctx| {
            collection_items(ctx, &request, current.as_ref())
        })
        .wide()
        .focus_at(focus)
    }

    /// Ordenar, ver e filtrar: o botão da busca reúne ordem, modo de exibição, filtros, coleções e as ações da
    /// biblioteca num menu só, com a contagem no cabeçalho.
    fn options(&self, ctx: &Ctx) -> super::layer::MenuLayer {
        use super::layer::{Flow, MenuItem, MenuLayer};
        let (request, current, criteria) = (
            self.pick_request.clone(),
            self.pick.clone(),
            self.criteria.clone(),
        );
        let route = self.route_filter();
        let title = format!(
            "{} · {}",
            ctx.t("desktop.library"),
            library_text::game_count(ctx.lang(), self.games(ctx).len())
        );
        MenuLayer::menu(String::new(), move |ctx| {
            let (sort, reversed, installed) = {
                let c = criteria.borrow();
                (c.sort, c.reversed, c.install == InstallFilter::Installed)
            };
            let mark = |on: bool, mut row: PanelRow| {
                if on {
                    row.value = "✓".into();
                }
                row
            };
            let mut v = vec![
                MenuItem::new(PanelRow::header(title.clone())),
                MenuItem::new(PanelRow::header(ctx.t("desktop.options_sort"))),
            ];
            // Os três do protótipo e, se a escolha atual for outra (Nota, Tamanho…), ela também; o resto fica nos filtros.
            for s in library_text::SORT_ORDER
                .into_iter()
                .enumerate()
                .filter(|(i, s)| *i < 3 || *s == sort)
                .map(|(_, s)| s)
            {
                let c = criteria.clone();
                v.push(
                    MenuItem::new(mark(
                        s == sort,
                        PanelRow::button(
                            format!("sort-{}", s as u8),
                            library_text::sort_label(ctx.lang(), s),
                        ),
                    ))
                    .on(move |ctx| {
                        c.borrow_mut().sort = s;
                        ctx.dirty = true;
                        Flow::Stay
                    }),
                );
            }
            let c = criteria.clone();
            v.push(
                MenuItem::new(mark(
                    reversed,
                    PanelRow::button("reverse", ctx.t("desktop.view.reverse")).icon("sort"),
                ))
                .on(move |ctx| {
                    let mut c = c.borrow_mut();
                    c.reversed = !c.reversed;
                    ctx.dirty = true;
                    Flow::Stay
                }),
            );
            v.push(MenuItem::new(PanelRow::separator()));
            v.push(MenuItem::new(PanelRow::header(ctx.t("desktop.view.menu"))));
            v.extend(view_items(ctx));
            v.push(MenuItem::new(PanelRow::separator()));
            v.push(MenuItem::new(PanelRow::header(
                ctx.t("desktop.options_show"),
            )));
            let (c, c2) = (criteria.clone(), criteria.clone());
            v.push(
                MenuItem::new(mark(
                    installed,
                    PanelRow::button(
                        "installed",
                        library_text::install_label(ctx.lang(), InstallFilter::Installed),
                    ),
                ))
                .on(move |ctx| {
                    let mut c = c.borrow_mut();
                    c.install = if c.install == InstallFilter::Installed {
                        InstallFilter::Any
                    } else {
                        InstallFilter::Installed
                    };
                    ctx.dirty = true;
                    Flow::Stay
                }),
            );
            v.push(
                MenuItem::new(
                    PanelRow::button("filter", format!("{}…", ctx.t("desktop.filter_sort")))
                        .icon("filter"),
                )
                .on(move |ctx| {
                    Flow::Replace(Box::new(dialogs::library_filters(ctx, c2.clone(), route)))
                }),
            );
            v.push(MenuItem::new(PanelRow::separator()));
            v.push(MenuItem::new(PanelRow::header(
                ctx.t("desktop.tab_collections"),
            )));
            v.extend(collection_items(ctx, &request, current.as_ref()));
            v.push(MenuItem::new(PanelRow::separator()));
            v.push(
                MenuItem::new(
                    PanelRow::button("add-game", ctx.t("desktop.menu.add_game")).icon("plus"),
                )
                .on(|ctx| {
                    ctx.effect(Effect::AddGame);
                    Flow::CloseAll
                }),
            );
            let label = if ctx.scanning {
                ctx.t("desktop.refresh_busy")
            } else {
                ctx.t("desktop.refresh_library")
            };
            v.push(
                MenuItem::new(PanelRow::button("rescan", label).icon("refresh")).on(|ctx| {
                    rescan(ctx);
                    Flow::CloseAll
                }),
            );
            v
        })
        .wide()
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
            1 => Nav::Layer(dialogs::tags_layer(ctx, ids, None)),
            2 => {
                // Algum fora dos favoritos: favorita todos; senão tira todos.
                let fav = ids
                    .iter()
                    .any(|id| ctx.library.find(id).is_some_and(|g| !g.is_favorite));
                let mut changed = Vec::new();
                for id in &ids {
                    if ctx.library.find(id).is_some_and(|g| g.is_favorite != fav) {
                        match ctx.library.toggle_favorite(id) {
                            Ok(()) => changed.push((id.clone(), !fav)),
                            Err(e) => tracing::error!("favoritar: {e}"),
                        }
                    }
                }
                let n = ids.len().to_string();
                let key = if fav {
                    "desktop.bulk.favorited"
                } else {
                    "desktop.bulk.unfavorited"
                };
                // Desfazer volta cada jogo ao que era antes.
                let before: Vec<(String, bool)> = changed;
                ctx.toast_with_undo(ctx.tf(key, &[("n", &n)]), move |ctx| {
                    for (id, was) in &before {
                        if ctx.library.find(id).is_some_and(|g| g.is_favorite != *was)
                            && let Err(e) = ctx.library.toggle_favorite(id)
                        {
                            tracing::error!("favoritar: {e}");
                        }
                    }
                });
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
                    push(title, "check", SeeAll::Status(s), &|g| {
                        g.play_status == Some(s)
                    });
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
                    push(t.name, "tag", SeeAll::Tag(name.clone()), &|g| {
                        g.tags.contains(&name)
                    });
                }
            }
            GroupBy::Genre => {
                let mut genres: Vec<String> = games
                    .iter()
                    .flat_map(|g| g.genres.iter().cloned())
                    .collect();
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
        self.overview = false;
        self.set_view(View::Grid, ctx);
    }

    fn set_view(&mut self, view: View, ctx: &mut Ctx) {
        ctx.settings.set(VIEW_KEY, view.key());
        self.zone = Zone::List;
        self.group_focus = None;
        self.cat_focus = (0, 0);
    }

    /// AcoesDoJogo: Jogar, favoritar, ⋯ e a engrenagem; com o jogo aberto, Voltar ao jogo, Encerrar e ⋯.
    fn actions(&self, ctx: &Ctx, g: &Game) -> Vec<Action> {
        if ctx.running.as_ref().is_some_and(|r| r.id == g.id) {
            // Abrindo: só o anel; o cancelar aparece depois de `LAUNCH_SLOW`.
            if ctx.launching && !ctx.launch_slow(std::time::Instant::now()) {
                return vec![Action::Play, Action::More];
            }
            vec![Action::Play, Action::End, Action::More]
        } else {
            vec![
                Action::Play,
                Action::Favorite,
                Action::More,
                Action::Settings,
            ]
        }
    }

    fn run(&mut self, action: Action, ctx: &mut Ctx) -> Nav {
        let Some(id) = self.selected(ctx) else {
            return Nav::Stay;
        };
        self.selected = Some(id.clone());
        self.run_on(&id, action, ctx)
    }

    /// A ação num jogo qualquer (a Visão geral age em jogos que podem estar fora da aba atual).
    fn run_on(&mut self, id: &str, action: Action, ctx: &mut Ctx) -> Nav {
        match action {
            Action::Play => {
                if ctx.restoring {
                    ctx.toast(ctx.t("settings.system.restore_busy"));
                    return Nav::Stay;
                }
                // Já abrindo: o clique repetido não abre outra vez.
                if ctx.launching {
                    return Nav::Stay;
                }
                details::primary_action(ctx, id)
            }
            Action::Favorite => {
                dialogs::toggle_favorite(ctx, id);
                Nav::Stay
            }
            Action::More => Nav::Layer(Box::new(dialogs::more_actions(ctx, id))),
            Action::Settings => Nav::Layer(dialogs::game_settings(ctx, id)),
            Action::End => {
                ctx.effect(Effect::EndGame);
                Nav::Stay
            }
        }
    }

    fn zones(&self) -> Vec<Zone> {
        vec![
            Zone::Bar,
            Zone::Tabs,
            Zone::Controls,
            Zone::List,
            Zone::Actions,
        ]
    }

    /// As Categorias ocupam a área toda: não têm a coluna (recolher, Visão geral); só a coleção ativa tem o seletor.
    fn controls(&self, ctx: &Ctx) -> Vec<Control> {
        if View::of(ctx) == View::Categories {
            let mut v = vec![Control::Options];
            v.extend((self.tab == Tab::Collections).then_some(Control::Overview));
            return v;
        }
        let first = if collapsed(ctx) {
            Control::Search
        } else {
            Control::Options
        };
        vec![first, Control::Collapse, Control::Overview]
    }

    /// Tab / Shift+Tab: próxima zona.
    fn cycle(&mut self, forward: bool) {
        let zones = self.zones();
        let from = if self.zone == Zone::Body {
            Zone::Actions
        } else {
            self.zone
        };
        let i = zones.iter().position(|z| *z == from).unwrap_or(0) as i32;
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

    /// Abre a página do jogo na lista, onde quer que ele esteja (volta para Todos sem a busca se a aba o esconde).
    fn open_game(&mut self, id: &str, ctx: &Ctx) {
        self.hero = None;
        self.focus_game(id, ctx);
    }

    /// Teclado e controle na Visão geral: os alvos vêm de `overview::targets`, na ordem da tela. A fileira Instalados
    /// anda com ←→ (↑ sai dela para a seção de cima); no resto, ↓→ vão para o próximo e ↑← para o anterior.
    fn overview_cmd(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Nav {
        let targets = overview::targets(ctx, ctx.overview_show_back);
        let last = targets.len().saturating_sub(1);
        let at = self.index.min(last);
        let Some(target) = targets.get(at).copied() else {
            self.zone = Zone::List;
            return Nav::Stay;
        };
        let first_cover = targets
            .iter()
            .position(|t| matches!(t, Target::Cover(_)))
            .unwrap_or(0);
        let in_row = matches!(target, Target::Cover(_));
        let id = overview::game_of(ctx, target).map(|g| g.id.clone());
        match cmd {
            Cmd::Right => self.index = (at + 1).min(last),
            Cmd::Down if !in_row => self.index = (at + 1).min(last),
            Cmd::Left if in_row => self.index = at.saturating_sub(1).max(first_cover),
            Cmd::Up if in_row => self.index = first_cover.saturating_sub(1),
            Cmd::Up | Cmd::Left if at > 0 => self.index = at - 1,
            // No primeiro alvo: ← vai para a lista; ↑ para a linha Visão geral, no topo dela.
            Cmd::Left => self.zone = Zone::List,
            Cmd::Up => {
                self.zone = Zone::Controls;
                self.index = self
                    .controls(ctx)
                    .iter()
                    .position(|c| *c == Control::Overview)
                    .unwrap_or(0);
            }
            // Enter: o destaque joga, Precisa de atenção faz a ação da linha e os outros abrem a página do jogo.
            Cmd::Accept => match (target, id) {
                (Target::Hero, Some(id)) => return self.run_on(&id, Action::Play, ctx),
                (Target::Attention(i), _) => return self.attention_act(i, ctx),
                (_, Some(id)) => self.open_game(&id, ctx),
                _ => {}
            },
            Cmd::Favorite => {
                if let Some(id) = id {
                    return self.run_on(&id, Action::Favorite, ctx);
                }
            }
            Cmd::Menu => {
                if let Some(id) = id {
                    return self.run_on(&id, Action::More, ctx);
                }
            }
            _ => {}
        }
        Nav::Stay
    }

    /// A ação de uma linha de Precisa de atenção: refazer o que falhou, escolher a arte ou abrir o jogo.
    fn attention_act(&mut self, index: usize, ctx: &mut Ctx) -> Nav {
        let Some(item) = overview::attention_of(ctx, index) else {
            return Nav::Stay;
        };
        match item.state.as_str() {
            "error" => {
                super::activities::run_action(ctx, &item.id);
                Nav::Stay
            }
            "art" => Nav::Layer(super::art::picker(ctx, &item.id)),
            _ => {
                self.open_game(&item.id, ctx);
                Nav::Stay
            }
        }
    }
}

/// "Hoje", "Ontem" ou o dia e o mês, para a última sessão e o cartão de Sessões.
pub(crate) fn day(ctx: &Ctx, at: chrono::DateTime<chrono::Local>) -> String {
    let today = chrono::Local::now().date_naive();
    let d = at.date_naive();
    if d == today {
        ctx.t("desktop.day.today")
    } else if today.pred_opt() == Some(d) {
        ctx.t("desktop.day.yesterday")
    } else {
        game_text::short_date(ctx.lang(), d)
    }
}

/// O botão principal do jogo (Jogar, Voltar ao jogo, Instalar…, ou o anel enquanto abre): o do destaque e o da
/// Visão geral.
pub(super) fn play_row(
    ctx: &Ctx,
    g: &Game,
    running: bool,
    install: &Option<details::InstallStage>,
) -> PanelRow {
    let mut row = if running && ctx.launching {
        // Botão travado com o anel girando (JogarEAbrir).
        let slow = ctx.launch_slow(std::time::Instant::now());
        PanelRow::button(
            "play",
            ctx.t(if slow {
                "desktop.opening_slow"
            } else {
                "desktop.opening"
            }),
        )
        .icon("ring")
        .enabled(false)
    } else {
        match (running, install) {
            (false, Some(i)) => {
                // Instalar: loja e tamanho na linha de baixo da pílula, ⌄ se houver mais de uma loja.
                let icon = if i.installing { "ring" } else { "download" };
                let mut row = PanelRow::button("play", i.label.clone())
                    .icon(icon)
                    .enabled(!i.installing)
                    .desc(i.sub.clone());
                row.on = i.split;
                row
            }
            (true, _) => PanelRow::button("play", ctx.t("game.resume")).icon("gamepad"),
            _ => PanelRow::button("play", tiles::play_label(ctx, g)).icon("play"),
        }
    };
    row.style = "primary".into();
    row
}

/// Faixa de estado do destaque: Última sessão e Tempo de jogo. Nuvem e Conquistas só entram quando o Tako tiver esses
/// dados (sem métricas inventadas).
pub(super) fn strip(ctx: &Ctx, g: &Game) -> Vec<DeskInfo> {
    let lang = ctx.lang();
    let running = ctx.running.as_ref().is_some_and(|r| r.id == g.id);
    let last = ctx.library.sessions(&g.id).last().map(|s| s.started_at);
    let session = if running {
        // "Sessão atual 42 min" (EstadoJogando); sem o início conhecido, "jogando agora".
        let started = ctx.running.as_ref().and_then(|r| r.started);
        DeskInfo {
            icon: "calendar".into(),
            label: ctx.t("desktop.strip.current_session"),
            value: started.map_or_else(
                || ctx.t("desktop.strip.playing_now"),
                |t| game_text::play_time(lang, t.elapsed().max(std::time::Duration::from_secs(60))),
            ),
            dot: true,
            ..Default::default()
        }
    } else {
        let value = match last.or(g.last_played_at) {
            Some(at) => {
                let at = at.with_timezone(&chrono::Local);
                format!("{}, {}", day(ctx, at), at.format("%H:%M"))
            }
            None => ctx.t("desktop.strip.never_played"),
        };
        DeskInfo {
            icon: "calendar".into(),
            label: ctx.t("desktop.strip.last_session"),
            value,
            ..Default::default()
        }
    };
    let time = DeskInfo {
        icon: "clock".into(),
        label: ctx.t("desktop.play_time"),
        // Jogo com sessão registrada nunca é "Ainda não jogado": vale a soma das sessões ou "Não registrado".
        value: match g
            .play_time
            .max(ctx.library.sessions(&g.id).iter().map(|s| s.duration).sum())
        {
            t if !t.is_zero() => game_text::play_time(lang, t),
            _ if last.or(g.last_played_at).is_some() => ctx.t("desktop.strip.time_unknown"),
            _ => ctx.t("desktop.strip.not_played"),
        },
        ..Default::default()
    };
    let mut items = vec![session, time];
    if let Some(score) = g.metacritic_score {
        let band = match score {
            90..=i32::MAX => "desktop.strip.metacritic_universal",
            75..=89 => "desktop.strip.metacritic_favorable",
            50..=74 => "desktop.strip.metacritic_mixed",
            20..=49 => "desktop.strip.metacritic_unfavorable",
            _ => "desktop.strip.metacritic_overwhelming",
        };
        items.push(DeskInfo {
            icon: "metacritic".into(),
            label: ctx.t("desktop.strip.metacritic"),
            value: ctx.t(band),
            score,
            ..Default::default()
        });
    }
    items
}

/// Descrição do CartaoSobre: até uns 240 caracteres, cortada na última palavra com "…". `true` quando cortou.
pub(super) fn about(description: &str) -> (String, bool) {
    const LIMIT: usize = 240;
    let clean = description.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.chars().count() <= LIMIT {
        return (clean, false);
    }
    let cut: String = clean.chars().take(LIMIT).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head);
    (
        format!("{}…", cut.trim_end_matches([',', '.', ';', ':'])),
        true,
    )
}

/// CartaoDetalhes: todas as informações que o jogo tem, uma linha por dado.
fn more_details(ctx: &Ctx, g: &Game) -> Vec<DeskInfo> {
    let lang = ctx.lang();
    let text = |v: &Option<String>| v.clone().unwrap_or_default();
    let tags: Vec<&str> = g.tags.iter().map(String::as_str).collect();
    let release = g.release_date.map(|d| game_text::date(lang, d));
    let score = g.metacritic_score.or(g.critic_score).map(|s| s.to_string());
    // A ordem do CartaoDetalhes: Desenvolvedor, Publicadora, Lançamento, o resto e, por último, Tamanho e Situação.
    [
        ("user", "details.developer", text(&g.developer)),
        ("user", "desktop.page.publisher", text(&g.publisher)),
        ("calendar", "details.release", text(&release)),
        ("tag", "details.genres", g.genres.join(" - ")),
        (
            store_icon(g.store()),
            "desktop.store",
            game_text::store_name(lang, g.store()),
        ),
        ("layers", "details.series", text(&g.franchise)),
        ("star", "desktop.info.critic", text(&score)),
        ("tag", "desktop.info.tags", tags.join(", ")),
        ("monitor", "details.system", text(&g.platform)),
        ("user", "details.players", text(&g.players)),
        ("folder", "desktop.page.folder", text(&g.install_path)),
        ("gauge", "desktop.size", text(&g.size_label)),
        ("check", "details.status", game_text::status(lang, g)),
    ]
    .into_iter()
    .filter(|(.., value)| !value.trim().is_empty())
    .map(|(icon, key, value)| DeskInfo {
        icon: icon.into(),
        label: ctx.t(key),
        value,
        // A situação ganha o ponto verde quando o jogo está instalado.
        dot: key == "details.status" && g.is_installed,
        ..Default::default()
    })
    .collect()
}

/// CartaoSobre: os parágrafos da descrição. É dado externo: no máximo 24 parágrafos e 6000 caracteres.
fn paragraphs(description: &str) -> Vec<String> {
    let mut left = 6000;
    description
        .split('\n')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .take(24)
        .map_while(|p| {
            let cut: String = p.chars().take(left).collect();
            left -= cut.chars().count();
            (!cut.is_empty()).then_some(cut)
        })
        .collect()
}

impl Page for DesktopLibrary {
    fn route(&self) -> String {
        ROUTE.into()
    }

    fn kind(&self) -> &'static str {
        "desk-library"
    }

    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Nav {
        let games = self.games(ctx);
        let ids: Vec<String> = games.iter().map(|g| g.id.clone()).collect();
        let list = entries(ctx, &games);
        let cur = self.current(&ids);
        let to_install_cur = cur.is_some_and(|i| to_install(games[i]));
        match (self.zone, cmd) {
            (_, Cmd::PrevSection) | (_, Cmd::NextSection) => {
                let next = cmd == Cmd::NextSection;
                // Numa coleção (sem aba) o ponto de partida é antes da primeira aba.
                let to = match TABS.iter().position(|t| *t == self.tab) {
                    Some(i) => (i + if next { 1 } else { TABS.len() - 1 }) % TABS.len(),
                    None if next => 0,
                    None => TABS.len() - 1,
                };
                self.open_tab(TABS[to], ctx);
            }
            // Visão geral: ←→↑↓ andam pelos alvos (destaque → Voltar a jogar → …) e Enter age no alvo.
            (
                Zone::Actions,
                Cmd::Left
                | Cmd::Right
                | Cmd::Up
                | Cmd::Down
                | Cmd::Accept
                | Cmd::Favorite
                | Cmd::Menu,
            ) if self.overview_on(ctx) => return self.overview_cmd(cmd, ctx),
            // Cabeçalho de loja (NaoInstalados): Enter alterna, → abre e ← fecha.
            (Zone::List, Cmd::Accept | Cmd::Left | Cmd::Right)
                if self.group_focus.is_some() && View::of(ctx) != View::Categories =>
            {
                if let Some(store) = self.group_focus {
                    let open = match cmd {
                        Cmd::Left => Some(false),
                        Cmd::Right => Some(true),
                        _ => None,
                    };
                    set_group(ctx, store, open);
                }
            }
            // Grade: as setas andam pelas capas e cabeçalhos (a coluna vem do tema); Enter instala o que falta
            // instalar e abre os outros no destaque.
            (Zone::List, Cmd::Left | Cmd::Right | Cmd::Up | Cmd::Down | Cmd::Accept)
                if View::of(ctx) == View::Grid =>
            {
                let at = self.focus_pos(&list, cur);
                match cmd {
                    Cmd::Left | Cmd::Right => match neighbor(&list, at, cmd == Cmd::Right) {
                        Some(i) => self.focus(list[i], &ids),
                        None if cmd == Cmd::Left => self.cycle(false),
                        None => {}
                    },
                    Cmd::Up | Cmd::Down => {
                        let rows = grid_rows(&list, ctx.grid_columns.max(1));
                        let here = at.and_then(|a| rows.iter().position(|r| r.contains(&a)));
                        let col = at
                            .zip(here)
                            .and_then(|(a, r)| rows[r].iter().position(|i| *i == a))
                            .unwrap_or(0);
                        let step = |r: usize| {
                            let next = if cmd == Cmd::Down {
                                r.checked_add(1)
                            } else {
                                r.checked_sub(1)
                            };
                            next.filter(|n| *n < rows.len())
                        };
                        // A divisória não recebe foco: a linha dela é pulada.
                        let mut target = here.map_or(Some(0), step);
                        while target.is_some_and(|t| list[rows[t][0]] == Entry::Section) {
                            target = step(target.unwrap_or_default());
                        }
                        match target {
                            Some(t) => {
                                let row = &rows[t];
                                self.focus(list[row[col.min(row.len() - 1)]], &ids);
                            }
                            None if cmd == Cmd::Up => self.cycle(false),
                            None => {}
                        }
                    }
                    _ if to_install_cur => return self.run(Action::Play, ctx),
                    _ => {}
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
                        self.overview = false;
                        self.set_view(View::List, ctx);
                        return Nav::Stay;
                    }
                    _ => (r, c),
                };
                if let Some(id) = self
                    .cat_rows
                    .get(self.cat_focus.0)
                    .and_then(|row| row.get(self.cat_focus.1))
                {
                    self.selected = Some(id.clone());
                }
            }
            // Lista: ↑↓ passam pelos jogos e pelos cabeçalhos de loja, não pela divisória.
            (Zone::List, Cmd::Up | Cmd::Down) => {
                let at = self.focus_pos(&list, cur);
                match neighbor(&list, at, cmd == Cmd::Down) {
                    Some(i) => self.focus(list[i], &ids),
                    None if cmd == Cmd::Up => self.cycle(false),
                    None => {}
                }
            }
            (Zone::List, Cmd::Right) => {
                if cur.is_some() {
                    self.zone = Zone::Actions;
                    self.index = 0;
                }
            }
            (Zone::List | Zone::Actions, Cmd::Favorite) if self.group_focus.is_none() => {
                return self.run(Action::Favorite, ctx);
            }
            (Zone::List | Zone::Actions, Cmd::Menu) if self.group_focus.is_none() => {
                return self.run(Action::More, ctx);
            }
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
            (Zone::Actions, Cmd::Up) => self.zone = Zone::List,
            // Desce para o corpo da página (Sobre, Detalhes…) e já rola o primeiro passo.
            (Zone::Actions, Cmd::Down) => {
                self.zone = Zone::Body;
                self.page_scroll += BODY_STEP;
            }
            (Zone::Body, Cmd::Down) if self.page_edge < 2 => self.page_scroll += BODY_STEP,
            (Zone::Body, Cmd::Up) if self.page_edge > 0 => self.page_scroll -= BODY_STEP,
            (Zone::Body, Cmd::Up) => {
                self.zone = Zone::Actions;
                self.index = 0;
            }
            (Zone::Body, Cmd::Left) => self.zone = Zone::List,
            (Zone::Actions, Cmd::Accept) => {
                let action = cur
                    .and_then(|i| ctx.library.find(&ids[i]))
                    .and_then(|g| self.actions(ctx, g).get(self.index).copied());
                if let Some(a) = action {
                    return self.run(a, ctx);
                }
            }
            (Zone::Bar, Cmd::Left) => self.index = self.index.saturating_sub(1),
            (Zone::Bar, Cmd::Right) => {
                self.index = (self.index + 1).min(bar::ITEMS.len() - 1);
            }
            // Menu no botão Downloads: o popover (o mesmo que Aceitar).
            (Zone::Bar, Cmd::Menu) if bar::item(self.index) == Some(bar::Item::Downloads) => {
                super::activities::open_list(ctx);
                return Nav::Stay;
            }
            (Zone::Bar, Cmd::Down) => self.zone = Zone::List,
            (Zone::Bar, Cmd::Accept) => match bar::item(self.index) {
                Some(bar::Item::Library) | None => self.zone = Zone::List,
                Some(item) => return bar::activate(item, ctx),
            },
            (Zone::Tabs, Cmd::Up) => {
                self.zone = Zone::Bar;
                self.index = 0;
            }
            (Zone::Tabs, Cmd::Left) => self.index = self.index.saturating_sub(1),
            (Zone::Tabs, Cmd::Right) => self.index = (self.index + 1).min(TABS.len() - 1),
            (Zone::Tabs, Cmd::Accept) | (Zone::Tabs, Cmd::Down) => {
                self.open_tab(TABS[self.index.min(TABS.len() - 1)], ctx);
                self.zone = Zone::List;
            }
            (Zone::Controls, Cmd::Left) => self.index = self.index.saturating_sub(1),
            (Zone::Controls, Cmd::Right) => {
                self.index = (self.index + 1).min(self.controls(ctx).len() - 1)
            }
            (Zone::Controls, Cmd::Accept) => match self.controls(ctx).get(self.index) {
                Some(Control::Options) => return Nav::Layer(Box::new(self.options(ctx))),
                // O tema expande a lista e foca a busca (a mesma conta do Ctrl + K).
                Some(Control::Search) => self.search_focus += 1,
                Some(Control::Collapse) => return self.click("collapse", 0, ctx),
                // No chip, Enter abre o seletor de coleção; na Visão geral, sai da busca e dos filtros.
                Some(Control::Overview) if self.tab == Tab::Collections => {
                    return Nav::Layer(Box::new(self.collection_select(ctx)));
                }
                Some(Control::Overview) => return self.click("overview", 0, ctx),
                // O índice pode ter sobrado de uma lista de controles maior (ex.: ao trocar para Categorias).
                None => self.index = self.controls(ctx).len() - 1,
            },
            (Zone::Controls, Cmd::Down) => self.zone = Zone::List,
            (Zone::Controls, Cmd::Up) => self.cycle(false),
            // Esc: sai da seleção múltipla, depois limpa a busca e os filtros, depois volta para a lista.
            (_, Cmd::Back) if !self.checked.is_empty() => {
                self.checked.clear();
                self.anchor = None;
            }
            // No chip da coleção, Esc faz o mesmo que o ×.
            (Zone::Controls, Cmd::Back)
                if self.tab == Tab::Collections
                    && self.controls(ctx).get(self.index) == Some(&Control::Overview) =>
            {
                return self.click("collection-clear", 0, ctx);
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
        if mods == KeyModifiers::CTRL
            && c.eq_ignore_ascii_case(&'a')
            && View::of(ctx) != View::Categories
        {
            self.checked = self.ids(ctx).into_iter().collect();
            return Some(Nav::Stay);
        }
        // Ctrl + B recolhe e expande a lista.
        if mods == KeyModifiers::CTRL && c.eq_ignore_ascii_case(&'b') {
            ctx.settings.set_bool(COLLAPSED_KEY, !collapsed(ctx));
            return Some(Nav::Stay);
        }
        // Shift+Tab chega como Tab com Shift ou como "back tab" (U+0019), conforme o sistema.
        if c == keys::TAB || c == '\u{0019}' {
            let forward = c == keys::TAB && mods != KeyModifiers::SHIFT;
            // Na barra, Tab anda item a item (marca → abas → … → janela) antes de ir para a zona seguinte.
            match bar::step(self.index, forward).filter(|_| self.zone == Zone::Bar) {
                Some(i) => self.index = i,
                None => self.cycle(forward),
            }
            return Some(Nav::Stay);
        }
        // Page Down e Page Up rolam a página do jogo (com o foco na lista, nas ações ou no corpo).
        if (c == PAGE_DOWN || c == PAGE_UP)
            && mods == KeyModifiers::NONE
            && matches!(self.zone, Zone::List | Zone::Actions | Zone::Body)
            && View::of(ctx) != View::Categories
            && !self.overview_on(ctx)
        {
            self.page_scroll += if c == PAGE_DOWN { 1. } else { -1. };
            // Com a página rolada, o foco das ações ficaria fora da tela: ele passa para o corpo.
            if self.zone == Zone::Actions {
                self.zone = Zone::Body;
            }
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
                    ctx.settings.set(COLLECTION_KEY, &p.key());
                    self.pick = Some(p);
                    self.tab = Tab::Collections;
                    self.selected = None;
                }
                self.zone = Zone::List;
            }
            "collection-select" => return Nav::Layer(Box::new(self.collection_select(ctx))),
            "game" => {
                self.checked.clear();
                self.anchor = Some(index);
                self.select_index(ctx, index);
            }
            // Cabeçalho de loja da seção NÃO INSTALADOS: o índice é a ordem do grupo.
            "group" => {
                let store = runs(&self.games(ctx)).get(index).map(|r| r.store);
                if let Some(store) = store {
                    set_group(ctx, store, None);
                    self.group_focus = Some(store);
                    self.zone = Zone::List;
                }
            }
            "game-toggle" | "game-check" => self.toggle_check(ctx, index),
            "game-range" => self.check_range(ctx, index),
            "bulk" => return self.bulk(index, ctx),
            "view" => {
                if let Some(v) = VIEWS.get(index) {
                    self.set_view(*v, ctx);
                }
            }
            "collapse" => ctx.settings.set_bool(COLLAPSED_KEY, !collapsed(ctx)),
            "options" => return Nav::Layer(Box::new(self.options(ctx))),
            // Visão geral: sai da coleção e da busca e mostra a TelaBiblioteca.
            "overview" => {
                self.clear();
                self.open_tab(Tab::All, ctx);
                self.selected = None;
                self.overview = true;
                self.zone = Zone::Actions;
                self.index = 0;
            }
            // O destaque e Voltar a jogar: o clique abre a página do jogo, o ▶ joga.
            "ov-open-hero" | "ov-play-hero" | "ov-back" | "ov-back-play" => {
                let target = match zone {
                    "ov-back" | "ov-back-play" => Target::Back(index),
                    _ => Target::Hero,
                };
                let Some(id) = overview::game_of(ctx, target).map(|g| g.id.clone()) else {
                    return Nav::Stay;
                };
                if matches!(zone, "ov-play-hero" | "ov-back-play") {
                    self.zone = Zone::Actions;
                    self.index = overview::targets(ctx, ctx.overview_show_back)
                        .iter()
                        .position(|t| *t == target)
                        .unwrap_or(0);
                    return self.run_on(&id, Action::Play, ctx);
                }
                self.open_game(&id, ctx);
            }
            // Sua semana, Precisa de atenção e a fileira Instalados.
            "ov-best" | "ov-att" | "ov-att-action" | "ov-cover" | "ov-cover-play" => {
                let target = match zone {
                    "ov-best" => Target::Best,
                    "ov-att" | "ov-att-action" => Target::Attention(index),
                    _ => Target::Cover(index),
                };
                if let Some(at) = overview::targets(ctx, ctx.overview_show_back)
                    .iter()
                    .position(|t| *t == target)
                {
                    self.zone = Zone::Actions;
                    self.index = at;
                }
                if zone == "ov-att-action" {
                    return self.attention_act(index, ctx);
                }
                let Some(id) = overview::game_of(ctx, target).map(|g| g.id.clone()) else {
                    return Nav::Stay;
                };
                if zone == "ov-cover-play" {
                    return self.run_on(&id, Action::Play, ctx);
                }
                self.open_game(&id, ctx);
            }
            "collection-clear" => {
                self.open_tab(Tab::All, ctx);
                self.selected = None;
                self.zone = Zone::List;
            }
            "group-by" => return Nav::Layer(Box::new(group_menu())),
            "reverse" => {
                let mut c = self.criteria.borrow_mut();
                c.reversed = !c.reversed;
            }
            // Grade: clique abre o jogo no destaque; ▶ joga; botão direito abre as ações.
            "cover" => {
                self.checked.clear();
                self.select_index(ctx, index);
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
                self.overview = false;
                match zone {
                    "cat-cover-play" => return self.run(Action::Play, ctx),
                    "cat-cover-menu" => return self.run(Action::More, ctx),
                    _ => self.set_view(View::List, ctx),
                }
            }
            "cat-all" => {
                let games = self.games(ctx);
                let see = Self::groups(ctx, &games)
                    .into_iter()
                    .nth(index)
                    .map(|g| g.see);
                if let Some(see) = see {
                    self.see_all(see, ctx);
                }
            }
            "rescan" => rescan(ctx),
            // O ← e a trilha "Biblioteca" da barra fina da página do jogo: voltam à Visão geral.
            "game-back" => {
                if self.tab == Tab::Collections {
                    self.open_tab(Tab::All, ctx);
                }
                self.show_overview();
            }
            // Os dados do destaque: o trailer e o histórico de sessões.
            "trailer" | "sessions" => {
                let Some(g) = self
                    .selected(ctx)
                    .and_then(|id| ctx.library.find(&id).cloned())
                else {
                    return Nav::Stay;
                };
                if zone == "trailer" {
                    if let Some(url) = g.media.get(tako_core::library::media::VIDEO) {
                        ctx.effect(Effect::OpenUrl(url.clone()));
                    }
                } else {
                    ctx.open_route = Some("desk:profile:sessions".into());
                    return Nav::Stay;
                }
            }
            "game-play" => {
                self.select_index(ctx, index);
                return self.run(Action::Play, ctx);
            }
            "game-menu" => {
                self.select_index(ctx, index);
                return self.run(Action::More, ctx);
            }
            "install-more" => {
                if let Some(id) = self.selected(ctx) {
                    return Nav::Layer(Box::new(dialogs::install_from(ctx, &id)));
                }
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
            "status" => {
                if let Some(id) = self.selected(ctx) {
                    return Nav::Layer(Box::new(dialogs::choose_status(ctx, &id)));
                }
            }
            "search-leave" => self.zone = Zone::List,
            "remove-filter" => self.remove_filter(ctx, index),
            "clear-filters" => {
                let mut c = self.criteria.borrow_mut();
                let reversed = c.reversed;
                *c = c.clear_filters();
                c.reversed = reversed;
            }
            "clear" => self.clear(),
            "add-game" => ctx.effect(Effect::AddGame),
            "group-store" => ctx.settings.set(GROUP_KEY, GroupBy::Store.key()),
            "new-collection" => {
                return Nav::Layer(Box::new(dialogs::new_collection(ctx, None)));
            }
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
        if let Some(p) = self.pick_request.borrow_mut().take() {
            self.pick = Some(p);
            self.tab = Tab::Collections;
            self.selected = None;
        }
        let lang = ctx.lang();
        let games = self.games(ctx);
        let ids: Vec<String> = games.iter().map(|g| g.id.clone()).collect();
        let cur = self.current(&ids);
        self.selection_index = cur.unwrap_or(0);
        let stores = runs(&games);
        if self
            .group_focus
            .is_some_and(|s| !stores.iter().any(|r| r.store == s))
        {
            self.group_focus = None;
        }
        let list = entries(ctx, &games);
        out.set(
            "desk-tabs",
            vec![
                ctx.t("desktop.tab_all"),
                ctx.t("desktop.tab_favorites"),
                ctx.t("desktop.tab_installed"),
            ],
        );
        // Coleção ativa: nenhuma aba escolhida (-1) e o chip no lugar da Visão geral.
        out.set(
            "desk-tab",
            TABS.iter()
                .position(|t| *t == self.tab)
                .map_or(-1, |i| i as i32),
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
        let current = chips
            .iter()
            .find(|c| c.selected)
            .cloned()
            .unwrap_or_default();
        out.set("desk-collections", chips);
        out.set("desk-collection", current);
        let c = self.criteria.borrow().clone();
        out.set("desk-search", c.search.clone());
        out.set("desk-search-hint", ctx.t("desktop.search"));
        out.set("desk-filter-label", ctx.t("desktop.filter_sort"));
        out.set("desk-filter-count", c.active_filter_count() as i32);
        let filters = self.active_filters(ctx);
        out.set(
            "desk-filter-chips",
            filters.chunks(2).map(|r| r.to_vec()).collect::<Vec<_>>(),
        );
        out.set("desk-clear-label", ctx.t("desktop.clear"));
        out.set("desk-filtered", c.has_search() || c.has_filters());
        let view = View::of(ctx);
        // Seleção múltipla: só os marcados que continuam na lista contam.
        self.checked.retain(|id| ids.contains(id));
        let checked = self.checked.len();
        let bulk = (checked > 0).then(|| {
            ctx.tf(
                "desktop.bulk.count",
                &[
                    ("n", &checked.to_string()),
                    ("total", &ids.len().to_string()),
                ],
            )
        });
        let total = library_text::game_count(lang, ids.len());
        let count_text = if ctx.scanning {
            ctx.t("desktop.updating")
        } else {
            bulk.clone()
                .unwrap_or_else(|| format!("{total} · {}", library_text::sort_label(lang, c.sort)))
        };
        let count_short_text = if ctx.scanning {
            ctx.t("desktop.updating")
        } else {
            bulk.unwrap_or(total)
        };
        out.set("desk-count", count_text);
        out.set("desk-count-short", count_short_text);
        out.set("desk-selecting", checked > 0);
        out.set("desk-reversed", c.reversed);
        out.set("desk-view", view.key());
        out.set("desk-collapsed", collapsed(ctx));
        // "Agrupar por **Status**": o nome do agrupamento em destaque.
        out.set("desk-group-prefix", ctx.t("desktop.group.by_prefix"));
        out.set(
            "desk-group-label",
            ctx.t(&format!("desktop.group.{}", GroupBy::of(ctx).key())),
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
        out.set(
            "desk-loading",
            ctx.scanning && ctx.library.games().is_empty(),
        );
        out.set("desk-rescan-busy", ctx.scanning);
        // Miniaturas e capas só perto da área visível (o tema avisa a primeira linha pelo `scrolled`).
        let first = ctx.list_first;
        let near = first.saturating_sub(NEAR / 3)..first + NEAR;
        let row = |i: usize, g: &Game, near: bool, cover: bool| {
            // Instalação pela loja em andamento: a fração vem da loja, então só vale se for um número de 0 a 1.
            let progress = g.installing.map(|p| {
                p.fraction
                    .filter(|f| f.is_finite())
                    .map_or(-1.0, |f| f.clamp(0.0, 1.0))
            });
            let percent = progress
                .filter(|p| *p >= 0.0)
                .map(|p| format!("{}%", (p * 100.0).round() as i32))
                .unwrap_or_default();
            let store = game_text::store_name(lang, g.store());
            let title = [("title", g.title.as_str())];
            let (label, install_label) = if !to_install(g) {
                (format!("{} · {store}", g.title), String::new())
            } else if progress.is_some() {
                let key = if percent.is_empty() {
                    "desktop.installing_game_unknown"
                } else {
                    "desktop.installing_game"
                };
                let args = [
                    ("title", g.title.as_str()),
                    ("store", store.as_str()),
                    ("percent", percent.as_str()),
                ];
                let label = if percent.is_empty() {
                    "desktop.game_label_missing"
                } else {
                    "desktop.game_label_installing"
                };
                (ctx.tf(label, &args), ctx.tf(key, &title))
            } else {
                let args = [("title", g.title.as_str()), ("store", store.as_str())];
                (
                    ctx.tf("desktop.game_label_missing", &args),
                    ctx.tf("desktop.install_game", &title),
                )
            };
            let (icon, initial) = row_icon(g, near && !cover, collapsed(ctx));
            DeskGame {
                id: g.id.clone(),
                title: g.title.clone(),
                icon,
                initial,
                cover: tiles::image(g.shown_cover().filter(|_| near && cover), COVER),
                accent: game_card_accent(ctx, &g.accent),
                favorite: g.is_favorite,
                running: ctx.running.as_ref().is_some_and(|r| r.id == g.id),
                installed: g.is_installed,
                store_icon: store_icon(g.store()).into(),
                store_name: store,
                checked: self.checked.contains(&g.id),
                index: i as i32,
                label,
                installing: progress.is_some(),
                progress: progress.unwrap_or_default(),
                percent,
                install_label,
                ..Default::default()
            }
        };
        let grid = view == View::Grid;
        let rows: Vec<DeskGame> = if view == View::Categories {
            Vec::new()
        } else {
            let pending: usize = stores.iter().map(|r| r.end - r.start).sum();
            list.iter()
                .enumerate()
                .map(|(pos, e)| match *e {
                    // O tema pede miniatura pela linha e capa pelo jogo (a grade conta jogos, a lista conta linhas).
                    Entry::Game(i) => row(
                        i,
                        games[i],
                        near.contains(if grid { &i } else { &pos }),
                        grid,
                    ),
                    Entry::Section => DeskGame {
                        kind: "section".into(),
                        title: ctx.t("desktop.group.not_installed").to_uppercase(),
                        count: pending.to_string(),
                        ..Default::default()
                    },
                    Entry::Group(store) => {
                        let order = stores
                            .iter()
                            .position(|r| r.store == store)
                            .unwrap_or_default();
                        let n = stores.get(order).map_or(0, |r| r.end - r.start);
                        let name = game_text::store_name(lang, store);
                        DeskGame {
                            kind: "group".into(),
                            label: ctx.tf(
                                "desktop.group_label",
                                &[("store", &name), ("count", &n.to_string())],
                            ),
                            title: name,
                            store_icon: store_icon(store).into(),
                            count: n.to_string(),
                            open: !group_closed(ctx, store),
                            index: order as i32,
                            ..Default::default()
                        }
                    }
                })
                .collect()
        };
        out.set("desk-games", rows);
        // Grade: a mesma lista em trechos (capas soltas, divisória, uma loja com as capas). O tema soma as alturas
        // pelas contagens `*_before` e só desenha as linhas visíveis de cada trecho.
        let cols = ctx.grid_columns.max(1);
        let focus = self
            .focus_pos(&list, cur)
            .map(|p| list[p])
            .filter(|_| view != View::Categories);
        let mut blocks: Vec<DeskBlock> = Vec::new();
        let mut focus_at: Vec<i32> = Vec::new();
        let (mut covered, mut sections, mut headers) = (0i32, 0i32, 0i32);
        let mut at = 0;
        while view != View::Categories && at < list.len() {
            let games_after = |from: usize| {
                list[from..]
                    .iter()
                    .take_while(|e| matches!(e, Entry::Game(_)))
                    .count()
            };
            let (mut block, own, used) = match list[at] {
                Entry::Section => (
                    DeskBlock {
                        kind: "section".into(),
                        title: ctx.t("desktop.group.not_installed").to_uppercase(),
                        count: stores
                            .iter()
                            .map(|r| r.end - r.start)
                            .sum::<usize>()
                            .to_string(),
                        ..Default::default()
                    },
                    0,
                    1,
                ),
                Entry::Group(store) => {
                    let order = stores
                        .iter()
                        .position(|r| r.store == store)
                        .unwrap_or_default();
                    let n = stores.get(order).map_or(0, |r| r.end - r.start);
                    let name = game_text::store_name(lang, store);
                    (
                        DeskBlock {
                            kind: "group".into(),
                            label: ctx.tf(
                                "desktop.group_label",
                                &[("store", &name), ("count", &n.to_string())],
                            ),
                            title: name,
                            count: n.to_string(),
                            icon: store_icon(store).into(),
                            open: !group_closed(ctx, store),
                            index: order as i32,
                            ..Default::default()
                        },
                        1,
                        1 + games_after(at + 1),
                    )
                }
                Entry::Game(_) => (
                    DeskBlock {
                        kind: "covers".into(),
                        ..Default::default()
                    },
                    0,
                    games_after(at),
                ),
            };
            let first = at + usize::from(block.kind != "covers");
            let idx: Vec<usize> = list[first..at + used]
                .iter()
                .filter_map(|e| match e {
                    Entry::Game(i) => Some(*i),
                    _ => None,
                })
                .collect();
            block.first = idx.first().map_or(0, |i| *i as i32);
            block.rows_before = covered;
            block.sections_before = sections;
            block.groups_before = headers;
            // Onde está o foco, em linhas e cabeçalhos antes dele (para o tema rolar até lá).
            match focus {
                Some(Entry::Group(s))
                    if block.kind == "group" && stores[block.index as usize].store == s =>
                {
                    focus_at = vec![covered, sections, headers, 1];
                }
                Some(Entry::Game(g)) => {
                    if let Some(k) = idx.iter().position(|i| *i == g) {
                        focus_at = vec![covered + (k / cols) as i32, sections, headers + own, 0];
                    }
                }
                _ => {}
            }
            block.games = idx
                .iter()
                .map(|i| row(*i, games[*i], near.contains(i), true))
                .collect();
            covered += idx.len().div_ceil(cols) as i32;
            sections += i32::from(block.kind == "section");
            headers += i32::from(block.kind == "group");
            blocks.push(block);
            at += used;
        }
        out.set("desk-blocks", blocks);
        out.set("desk-grid-extent", vec![covered, sections, headers]);
        out.set("desk-grid-focus", focus_at);
        out.set(
            "desk-focus-id",
            match focus {
                Some(Entry::Game(i)) => games[i].id.clone(),
                _ => String::new(),
            },
        );
        out.set(
            "desk-focus-group",
            self.group_focus
                .and_then(|s| stores.iter().position(|r| r.store == s))
                .map_or(-1, |p| p as i32),
        );
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
                self.cat_rows
                    .push(shown.iter().map(|g| g.id.clone()).collect());
            }
        }
        out.set("desk-categories", cats);
        out.set("desk-cat-focus-row", self.cat_focus.0 as i32);
        out.set("desk-cat-focus-col", self.cat_focus.1 as i32);
        out.set("desk-see-all", ctx.t("desktop.group.see_all"));
        // Há jogos, mas nenhum cabe no agrupamento (ex.: nenhum com status): diz por quê e oferece agrupar por loja.
        let stuck = !games.is_empty() && GroupBy::of(ctx) != GroupBy::Store;
        out.set(
            "desk-cat-empty-text",
            if stuck && GroupBy::of(ctx) == GroupBy::Status {
                ctx.t("desktop.group.empty_text")
            } else {
                String::new()
            },
        );
        out.set(
            "desk-cat-empty-action",
            if stuck {
                ctx.t("desktop.group.by_store")
            } else {
                String::new()
            },
        );
        out.set(
            "desk-cat-empty",
            ctx.tf(
                "desktop.group.empty",
                &[(
                    "group",
                    &ctx.t(&format!("desktop.group.{}", GroupBy::of(ctx).key())),
                )],
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
                ctx.t("desktop.overview"),
                ctx.t("desktop.options"),
                ctx.t("desktop.collection_clear"),
                ctx.t("desktop.list_search"),
                ctx.t("desktop.collapse"),
                ctx.t("desktop.expand"),
            ],
        );
        // Na Visão geral nenhum jogo está escolhido; só a lista com o foco do teclado mostra onde ele está.
        let overview = self.overview_on(ctx);
        let targets = if overview {
            overview::targets(ctx, ctx.overview_show_back)
        } else {
            Vec::new()
        };
        out.set(
            "desk-selected",
            self.focus_pos(&list, cur)
                .filter(|_| !overview || self.zone == Zone::List)
                .map_or(-1, |p| p as i32),
        );
        let hero = self.hero_id(ctx, cur.map(|i| ids[i].clone()));
        if overview && self.zone == Zone::Actions && targets.is_empty() {
            self.zone = Zone::List;
        }
        let focus = if overview && self.zone == Zone::Actions {
            self.index = self.index.min(targets.len().saturating_sub(1));
            targets.get(self.index).copied()
        } else {
            None
        };
        out.set(
            "desk-overview",
            if overview {
                overview::build(ctx, focus)
            } else {
                DeskOverview::default()
            },
        );
        let game = hero
            .and_then(|id| ctx.library.find(&id))
            .filter(|_| !overview);
        let (view, page, actions, stats, details) = match game {
            Some(g) => {
                let running = ctx.running.as_ref().is_some_and(|r| r.id == g.id);
                // Etiqueta de estado: JOGANDO, QUEBRADO, INSTALADO ou NÃO INSTALADO (e os casos de ROM e lançamento).
                let badge = if running {
                    ctx.t("desktop.badge.playing")
                } else if g.is_broken {
                    ctx.t("desktop.badge.broken")
                } else {
                    game_text::status(lang, g)
                };
                let install = details::install_stage(ctx, g);
                let actions: Vec<PanelRow> =
                    self.actions(ctx, g)
                        .into_iter()
                        .map(|a| match a {
                            Action::Play => play_row(ctx, g, running, &install),
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
                                PanelRow::button("more", ctx.t("desktop.more")).icon("dots")
                            }
                            Action::Settings => {
                                PanelRow::button("settings", ctx.t("details.game_settings"))
                                    .icon("gear")
                            }
                            // Abrindo há tempo demais: o mesmo encerrar, com o nome de cancelar.
                            Action::End if ctx.launching => {
                                PanelRow::button("end", ctx.t("common.cancel"))
                                    .icon("close")
                                    .danger()
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
                let description = about(&g.description).0;
                let logo = tiles::image(g.shown_logo(), tiles::LOGO);
                let show_logo = show_logo(ctx, g, &logo, &art_lum);
                let title_pending = title_pending(ctx, g, &logo, &art_lum);
                let page = DeskPage {
                    id: g.id.clone(),
                    cover: tiles::image(g.shown_cover(), THUMB),
                    labels: [
                        "desktop.page.back",
                        "desktop.library",
                        "desktop.about",
                        "desktop.details",
                        "desktop.install.more",
                    ]
                    .iter()
                    .map(|k| ctx.t(k))
                    .collect(),
                    about: paragraphs(&g.description),
                    on: running || (g.is_installed && !g.is_broken),
                    bright: overview::bright(&art_lum),
                };
                (
                    DeskGameView {
                        has_game: true,
                        title: g.title.clone(),
                        overline: badge.to_uppercase(),
                        description,
                        art: tiles::image(g.shown_hero(), tiles::hero(ctx)),
                        art_blur,
                        art_lum,
                        logo,
                        show_logo,
                        title_pending,
                        accent: game_card_accent(ctx, &g.accent),
                        hint,
                        install_note: install
                            .as_ref()
                            .filter(|_| !running)
                            .map(|i| i.note.clone())
                            .unwrap_or_default(),
                        install_warn: install.as_ref().is_some_and(|i| i.warn),
                        running,
                        trailer: g.media.contains_key(tako_core::library::media::VIDEO),
                        art_loading: g.shown_hero().is_some_and(crate::images::is_pending),
                        store_icon: store_icon(g.store()).into(),
                        store_name: game_text::store_name(lang, g.store()),
                        play_status: g.play_status.map_or("", PlayStatus::key).into(),
                        play_status_label: g.play_status.map_or_else(
                            || ctx.t("desktop.define_status"),
                            |s| library_text::status_label(lang, Some(s)),
                        ),
                    },
                    page,
                    actions,
                    strip(ctx, g),
                    more_details(ctx, g),
                )
            }
            None => (
                DeskGameView::default(),
                DeskPage::default(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
        };
        out.set("desk-game", view);
        out.set("desk-page", page);
        out.set("desk-scroll-page", self.page_scroll);
        out.set("desk-actions", actions);
        out.set("desk-stats", stats);
        out.set("desk-details", details);
        out.set(
            "desk-zone",
            match self.zone {
                Zone::Bar => "bar",
                Zone::Tabs => "tabs",
                Zone::Controls => "controls",
                Zone::List => "list",
                Zone::Actions if overview => "overview",
                Zone::Actions => "actions",
                Zone::Body => "body",
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
                format!("{}…", ctx.t("collections.new")),
                "new-collection".to_owned(),
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
        } else if self.tab == Tab::Favorites {
            (ctx.t("desktop.empty.favorites"), none(), none(), none())
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
        if self.overview_on(ctx) {
            // A cor da tela vem do jogo do destaque.
            return overview::hero(ctx).map(|g| g.id.clone());
        }
        self.selected(ctx)
    }

    fn desktop(&self) -> Option<&DesktopLibrary> {
        Some(self)
    }

    fn desktop_mut(&mut self) -> Option<&mut DesktopLibrary> {
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
        self.overview = false;
    }
}

/// Aberto no desktop com um controle conectado: pergunta uma vez se quer a tela cheia (ADR-0027). O SDL avisa
/// os controles já ligados nos primeiros quadros; depois de `ASK_WINDOW`, um controle novo não pergunta.
/// Lista · Grade · Categorias, com ✓ no modo atual.
fn view_items(ctx: &Ctx) -> Vec<super::layer::MenuItem> {
    use super::layer::{Flow, MenuItem};
    let current = View::of(ctx);
    [
        (View::List, "list"),
        (View::Grid, "grid"),
        (View::Categories, "layers"),
    ]
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
}

/// As coleções com a quantidade e ✓ na atual, depois "Nova coleção…" e "Organizar coleções".
fn collection_items(
    ctx: &Ctx,
    request: &Rc<RefCell<Option<Pick>>>,
    current: Option<&Pick>,
) -> Vec<super::layer::MenuItem> {
    use super::layer::{Flow, MenuItem};
    let mut v: Vec<MenuItem> = DesktopLibrary::picks(ctx)
        .into_iter()
        .enumerate()
        .map(|(i, (p, name, n))| {
            let note = if matches!(p, Pick::Smart(_)) {
                format!("{n} · {}", ctx.t("desktop.collection_auto"))
            } else {
                n.to_string()
            };
            let on = current == Some(&p);
            let request = request.clone();
            MenuItem::new(
                PanelRow::button(format!("pick-{i}"), name)
                    .icon(if on { "tick" } else { "" })
                    .value(note)
                    .emphasized(on),
            )
            .on(move |ctx| {
                ctx.settings.set(COLLECTION_KEY, &p.key());
                *request.borrow_mut() = Some(p.clone());
                Flow::Close
            })
        })
        .collect();
    if !v.is_empty() {
        v.push(MenuItem::new(PanelRow::separator()));
    }
    v.push(
        MenuItem::new(PanelRow::button(
            "new",
            format!("{}…", ctx.t("collections.new")),
        ))
        .on(|ctx| Flow::Replace(Box::new(super::dialogs::new_collection(ctx, None)))),
    );
    v.push(
        MenuItem::new(PanelRow::button(
            "manage",
            ctx.t("desktop.collections_manage"),
        ))
        .on(|ctx| Flow::Replace(Box::new(super::settings::library::collections_panel(ctx)))),
    );
    v
}

/// Atualizar biblioteca: um de cada vez e nunca durante uma restauração.
fn rescan(ctx: &mut Ctx) {
    if ctx.restoring {
        ctx.toast(ctx.t("settings.system.restore_busy"));
    } else if ctx.scanning {
        ctx.toast(ctx.t("library.rescan.already_running"));
    } else {
        ctx.effect(Effect::UserRescan);
    }
}

/// "Agrupar por" das Categorias.
fn group_menu() -> super::layer::MenuLayer {
    use super::layer::{Flow, MenuItem, MenuLayer};
    MenuLayer::menu(String::new(), |ctx| {
        let current = GroupBy::of(ctx);
        GROUPS
            .into_iter()
            .map(|g| {
                let mut row =
                    PanelRow::button(g.key(), ctx.t(&format!("desktop.group.{}", g.key())));
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

/// Perfil da barra: abre a página e reutiliza os fluxos persistidos de perfis locais.
pub fn profile_menu() -> super::layer::MenuLayer {
    use super::layer::{Flow, MenuItem, MenuLayer};
    MenuLayer::menu(String::new(), |ctx| {
        vec![
            MenuItem::new(
                PanelRow::button("view-profile", ctx.t("desktop.menu.view_profile")).icon("user"),
            )
            .on(|ctx| {
                ctx.open_route = Some(super::profile::ROUTE.into());
                Flow::CloseAll
            }),
            MenuItem::new(
                PanelRow::button("switch", ctx.t("desktop.menu.switch_profile")).icon("user"),
            )
            .on(|ctx| Flow::Replace(Box::new(super::profiles::panel(ctx)))),
            MenuItem::new(PanelRow::button("edit", ctx.t("profile.edit")).icon("edit"))
                .on(|ctx| Flow::Replace(Box::new(super::profiles::manage_active(ctx)))),
        ]
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
                PanelRow::button("add-game", ctx.t("desktop.menu.add_game")).icon("plus"),
            )
            .on(|ctx| {
                ctx.effect(Effect::AddGame);
                Flow::CloseAll
            }),
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
            MenuItem::new(crate::app_updates::menu_row(ctx)).on(|ctx| {
                ctx.effect(Effect::CheckUpdates);
                Flow::Stay
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
    .overline(crate::app_updates::BRAND_MENU)
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
    MenuLayer::dialog(ctx.t("desktop.ask.question"), |ctx| {
        let mut go = PanelRow::button("go", ctx.t("desktop.ask.go")).icon("tv");
        go.style = "primary".into();
        vec![
            MenuItem::new(go).on(|ctx| {
                ctx.effect(Effect::SwitchMode(UiMode::BigPicture));
                Flow::Close
            }),
            MenuItem::new(PanelRow::button("stay", ctx.t("desktop.ask.stay"))).on(|_| Flow::Close),
            // Opção solta, sem caixa: marcar fecha e não pergunta mais.
            MenuItem::new(PanelRow::check("never", ctx.t("desktop.ask.never"), false)).on(|ctx| {
                ctx.settings.set_bool(ASK_SETTING, false);
                Flow::Close
            }),
        ]
    })
    .overline(ctx.t("desktop.ask.title").to_uppercase())
    .text(ctx.t("desktop.ask.text"))
}

/// TituloDoJogo: o que vale para o jogo — "logo", "text" ou "" (usar o padrão das Configurações).
pub(crate) fn title_mode(ctx: &Ctx, id: &str) -> String {
    ctx.settings.string(&format!("{TITLE_LOGO}:{id}"))
}

/// O título vira o logo quando há logo, a pessoa não pediu texto e o contraste médio dele com a arte embaixo é de
/// pelo menos 3:1. "Logo" escolhido no jogo vale mesmo com pouco contraste (a aba Logo avisa).
pub(super) fn show_logo(ctx: &Ctx, g: &Game, logo: &slint::Image, art_lum: &[f32]) -> bool {
    let wanted = match title_mode(ctx, &g.id).as_str() {
        "logo" => return logo.size().width > 0,
        "text" => false,
        _ => ctx.settings.string(TITLE_LOGO) != "text",
    };
    wanted && logo.size().width > 0 && logo_contrast(g, logo, art_lum).is_none_or(|c| c >= 3.0)
}

/// O logo (ou a cópia desfocada da arte, que mede o contraste dele) ainda carrega: o título espera em vez de mostrar
/// o nome em texto e trocar para o logo um instante depois.
pub(super) fn title_pending(ctx: &Ctx, g: &Game, logo: &slint::Image, art_lum: &[f32]) -> bool {
    let mode = title_mode(ctx, &g.id);
    let wanted = mode == "logo" || (mode != "text" && ctx.settings.string(TITLE_LOGO) != "text");
    let Some(uri) = g.shown_logo().filter(|_| wanted) else {
        return false;
    };
    if logo.size().width == 0 {
        return crate::images::is_pending(uri);
    }
    // Logo pronto; no padrão, o contraste ainda depende da cópia desfocada da arte.
    mode != "logo" && art_lum.is_empty() && g.shown_hero().is_some_and(crate::images::is_pending)
}

/// Contraste (WCAG) entre o logo e a arte atrás do título; `None` sem como medir (sem arte: o fundo é o degradê).
pub(crate) fn logo_contrast(g: &Game, logo: &slint::Image, art_lum: &[f32]) -> Option<f64> {
    let (cols, rows) = crate::images::GLASS_GRID;
    if art_lum.len() != (cols * rows) as usize {
        return None;
    }
    // shortcut: o título fica nas colunas 4–7 e linhas 3–4 da grade numa janela 16:9 (centro da arte a 62%); numa
    // janela muito diferente o recorte desloca um pouco. Medir pela posição real exigiria a geometria do tema.
    let cells: Vec<f32> = (3..5)
        .flat_map(|r| (4..8).map(move |c| art_lum[(r * cols + c) as usize]))
        .collect();
    let art = f64::from(cells.iter().sum::<f32>() / cells.len() as f32);
    let logo = f64::from(crate::images::mean_luminance(g.shown_logo()?, logo)?);
    let (hi, lo) = if logo > art { (logo, art) } else { (art, logo) };
    Some((hi + 0.05) / (lo + 0.05))
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

    #[test]
    fn desktop_toast_time_keeps_action_time_and_short_confirmation() {
        let (_dir, mut shell) = shell();
        shell.ctx.toast("Salvo");
        assert_eq!(
            shell.ctx.toast_time(),
            std::time::Duration::from_millis(2300)
        );
        shell.ctx.toast_with_undo("Desfeito", |_| {});
        assert_eq!(shell.ctx.toast_time(), std::time::Duration::from_secs(8));
        shell.ctx.toast("Atualização falhou");
        shell.ctx.app_update.toast_label = "Tentar de novo".into();
        assert_eq!(shell.ctx.toast_time(), std::time::Duration::from_secs(8));
        shell.ctx.desktop = false;
        assert_eq!(shell.ctx.toast_time(), super::super::TOAST_TIME);
    }

    #[test]
    fn a_row_without_icon_shows_the_title_initial() {
        let game = |title: &str| Game {
            title: title.into(),
            ..Default::default()
        };
        assert_eq!(row_icon(&game("  ori"), true, false).1, "O");
        assert_eq!(row_icon(&game(""), true, false).1, "");
        // Com ícone ainda a caminho (ou fora da área visível), a linha mostra a inicial, nunca fica vazia.
        let mut far = game("Celeste");
        far.pinned_art.insert(
            tako_core::library::ArtKind::Icon,
            "steam-art://1/icon".into(),
        );
        assert_eq!(row_icon(&far, false, false).1, "C");
        assert_eq!(row_icon(&far, true, true).1, "C");
    }

    fn page(shell: &Shell) -> &DesktopLibrary {
        shell.desktop_library().unwrap()
    }

    fn open_collections_tab(shell: &mut Shell) {
        let Shell { pages, ctx, .. } = shell;
        pages[0]
            .desktop_mut()
            .unwrap()
            .open_tab(Tab::Collections, ctx);
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
        // "Adicionar jogo…" vem depois de "Trocar perfil…".
        shell.click("overlay-row", 2);
        assert_eq!(shell.ctx.effects.pop(), Some(Effect::AddGame));
        shell.click("desk-brand", 0);
        shell.click("overlay-row", 3);
        assert_eq!(
            shell.ctx.effects.pop(),
            Some(Effect::SwitchMode(crate::mode::UiMode::BigPicture))
        );
        // Separadores não recebem foco: ↓ pula de "Tela cheia" para "Procurar atualizações".
        let rows = crate::shell::layer::Layer::render(&brand_menu(), &shell.ctx).rows;
        assert_eq!(rows[4].kind, "separator");
        assert!(!rows[4].is_focusable());

        // Teclado: subir das abas chega à barra; Enter no perfil abre seu menu.
        shell.handle(Cmd::Up);
        shell.handle(Cmd::Up);
        while page(&shell).zone() != Zone::Tabs {
            shell.key("\t", KeyModifiers::NONE);
        }
        shell.handle(Cmd::Up);
        assert_eq!(page(&shell).zone(), Zone::Bar);
        shell.handle(Cmd::Right);
        shell.handle(Cmd::Right);
        shell.handle(Cmd::Right);
        shell.handle(Cmd::Accept);
        assert!(shell.has_layers());
        assert_eq!(page(&shell).zone(), Zone::Bar);
    }

    #[test]
    fn bar_keyboard_order_ends_in_the_window_buttons() {
        use bar::Item::*;
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        assert_eq!(
            bar::ITEMS,
            [
                Brand, Library, Downloads, Profile, BigPicture, Minimize, Maximize, Close
            ]
        );
        while page(&shell).zone() != Zone::Tabs {
            shell.key("\t", KeyModifiers::NONE);
        }
        shell.handle(Cmd::Up);
        // ← → andam pela barra até o fim, sem passar dele; Enter nos botões da janela pede a ação.
        for _ in 0..10 {
            shell.handle(Cmd::Right);
        }
        assert_eq!(page(&shell).index, 7);
        shell.handle(Cmd::Accept);
        shell.handle(Cmd::Left);
        shell.handle(Cmd::Accept);
        shell.handle(Cmd::Left);
        shell.handle(Cmd::Accept);
        assert_eq!(
            shell.ctx.effects,
            [
                Effect::Window("close"),
                Effect::Window("max"),
                Effect::Window("min")
            ]
        );
        // Tab percorre a barra do começo ao fim e só então sai dela.
        for _ in 0..5 {
            shell.handle(Cmd::Left);
        }
        assert_eq!(page(&shell).index, 0);
        for index in 1..=7 {
            shell.key("\t", KeyModifiers::NONE);
            assert_eq!(
                (page(&shell).zone(), page(&shell).index),
                (Zone::Bar, index)
            );
        }
        shell.key("\t", KeyModifiers::NONE);
        assert_eq!(page(&shell).zone(), Zone::Tabs);
    }

    #[test]
    fn downloads_button_is_in_the_keyboard_bar_order() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        crate::shell::downloads::demo(&mut shell.ctx);

        while page(&shell).zone() != Zone::Tabs {
            shell.key("\t", KeyModifiers::NONE);
        }
        shell.handle(Cmd::Up);
        // Marca, Biblioteca e então o botão Downloads.
        for _ in 0..2 {
            shell.handle(Cmd::Right);
        }
        assert_eq!(page(&shell).zone(), Zone::Bar);
        shell.handle(Cmd::Accept);

        assert!(shell.has_layers());
        assert_eq!(page(&shell).zone(), Zone::Bar);
        // Esc fecha e o foco continua no botão.
        shell.handle(Cmd::Back);
        assert!(!shell.has_layers());
        assert_eq!(page(&shell).zone(), Zone::Bar);
        assert_eq!(page(&shell).index, 2);
    }

    #[test]
    fn profile_menu_opens_real_profile_routes_and_editors() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        let rows = crate::shell::layer::Layer::render(&profile_menu(), &shell.ctx).rows;
        assert_eq!(
            rows.iter().map(|r| r.title.as_str()).collect::<Vec<_>>(),
            ["Ver perfil", "Trocar perfil…", "Editar perfil"]
        );
        shell.click("desk-profile-menu", 0);
        shell.click("overlay-row", 0);
        assert_eq!(shell.page().kind(), super::super::profile::KIND);
        assert!(!shell.has_layers());
        for action in [1, 2] {
            shell.click("desk-profile-menu", 0);
            shell.click("overlay-row", action);
            assert!(shell.has_layers());
            shell.close_layers();
        }
    }

    #[test]
    fn the_hero_cuts_long_text_and_the_page_lists_every_detail() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, shell) = shell();
        // Sobre o jogo: corta na palavra com "…" e avisa para mostrar "Mais".
        let long = "palavra ".repeat(60);
        let (text, more) = about(&long);
        assert!(more && text.ends_with('…') && text.chars().count() <= 241);
        assert_eq!(about("curta"), ("curta".to_owned(), false));
        // Detalhes: uma linha por dado que o jogo tem, o Status por último e com o ponto verde se instalado.
        let mut g = shell.ctx.library.games()[0].clone();
        g.release_date = chrono::NaiveDate::from_ymd_opt(2025, 9, 25);
        g.metacritic_score = Some(93);
        g.genres = vec!["Roguelike".into(), "Ação".into(), "Indie".into()];
        g.developer = Some("Supergiant Games, Outro".into());
        g.franchise = Some("Hades".into());
        g.tags = ["Co-op".to_owned()].into();
        g.is_installed = true;
        g.publisher = Some("Supergiant".into());
        let rows = more_details(&shell.ctx, &g);
        let last = rows.last().unwrap();
        assert_eq!(last.label, shell.ctx.t("details.status"));
        assert!(last.dot);
        assert!(rows.iter().any(|r| r.value == "Hades"));
        assert!(rows.iter().any(|r| r.value == "Roguelike - Ação - Indie"));
        assert!(rows.iter().any(|r| r.value == "Supergiant Games, Outro"));
        // Os parágrafos do Sobre vêm da descrição, sem linhas vazias e com limite.
        assert_eq!(paragraphs("um\n\n  dois \n"), ["um", "dois"]);
        assert_eq!(paragraphs(&"a\n".repeat(100)).len(), 24);
        assert_eq!(paragraphs(&"x".repeat(9000))[0].len(), 6000);
        // A faixa não inventa Nuvem nem Conquistas; o Metacritic entra por último quando o jogo tem nota.
        let items = strip(&shell.ctx, &g);
        assert_eq!(items.len(), 2 + usize::from(g.metacritic_score.is_some()));
        let mut sem_nota = g.clone();
        sem_nota.metacritic_score = None;
        assert_eq!(strip(&shell.ctx, &sem_nota).len(), 2);
        // Com uma sessão (última vez conhecida) e tempo 0, o tempo não diz "Ainda não jogado".
        let time = |g: &Game| strip(&shell.ctx, g)[1].value.to_string();
        sem_nota.play_time = std::time::Duration::ZERO;
        sem_nota.last_played_at = None;
        assert_eq!(time(&sem_nota), "Ainda não jogado");
        sem_nota.last_played_at = Some(chrono::Utc::now());
        assert_eq!(time(&sem_nota), "Não registrado");
    }

    #[test]
    fn the_title_is_the_logo_only_with_enough_contrast_or_when_chosen() {
        use slint::{Rgba8Pixel, SharedPixelBuffer};
        let (_dir, mut shell) = shell();
        let mut g = Game {
            id: "g".into(),
            ..Game::default()
        };
        g.media.insert(
            tako_core::library::media::LOGO.into(),
            "test://white-logo".into(),
        );
        let white = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&[255u8; 16], 2, 2);
        let logo = slint::Image::from_rgba8(white);
        let (cols, rows) = crate::images::GLASS_GRID;
        let dark = vec![0.02; (cols * rows) as usize];
        let light = vec![0.9; (cols * rows) as usize];
        assert!(show_logo(&shell.ctx, &g, &logo, &dark));
        // Logo e arte prontos: nada a esperar.
        assert!(!title_pending(&shell.ctx, &g, &logo, &dark));
        assert!(!show_logo(&shell.ctx, &g, &logo, &light));
        assert!(!show_logo(&shell.ctx, &g, &slint::Image::default(), &dark));
        shell.ctx.settings.set(TITLE_LOGO, "text");
        assert!(!show_logo(&shell.ctx, &g, &logo, &dark));
        // "Logo" escolhido no jogo vale mesmo com pouco contraste.
        shell.ctx.settings.set(&format!("{TITLE_LOGO}:g"), "logo");
        assert!(show_logo(&shell.ctx, &g, &logo, &light));
    }

    #[test]
    fn fast_keyboard_keeps_the_hero_until_the_selection_settles() {
        let (_dir, mut shell) = shell();
        let mut p = DesktopLibrary::new();
        let (a, b) = (Some("a".to_owned()), Some("b".to_owned()));
        shell.ctx.input = InputKind::Keyboard;
        assert_eq!(p.hero_id(&shell.ctx, a.clone()), a);
        // Seleção andou agora: o destaque fica no anterior.
        assert_eq!(p.hero_id(&shell.ctx, b.clone()), a);
        // Parou por 150 ms: troca.
        p.hero_wait = Some(("b".into(), std::time::Instant::now() - HERO_SETTLE));
        assert_eq!(p.hero_id(&shell.ctx, b.clone()), b);
        // Clique (mesmo depois de usar o teclado): na hora.
        p.hero = None;
        assert_eq!(p.hero_id(&shell.ctx, a.clone()), a);
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
        let fav_of = |shell: &Shell| -> Vec<bool> {
            ids[1..=3]
                .iter()
                .map(|id| shell.ctx.library.find(id).unwrap().is_favorite)
                .collect()
        };
        let before = fav_of(&shell);
        // Favoritar em massa: algum fora dos favoritos → todos ficam favoritos.
        shell.click("bulk", 2);
        assert!(
            ids[1..=3]
                .iter()
                .all(|id| shell.ctx.library.find(id).unwrap().is_favorite)
        );
        // "Desfazer" do aviso volta os favoritos ao que eram.
        shell.click("toast-undo", 0);
        assert_eq!(fav_of(&shell), before);
        assert!(shell.ctx.toast_undo.is_none());
        shell.click("bulk", 2);
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
        // Setas andam pelas 3 colunas da grade.
        shell.ctx.grid_columns = 3;
        // A grade vive na lista: clicar numa capa escolhe o jogo e fica na grade.
        shell.click("cover", 0);
        assert_eq!(View::of(&shell.ctx), View::Grid);
        assert_eq!(shell.focused_game().as_deref(), Some(ids[0].as_str()));
        shell.handle(Cmd::Down);
        assert_eq!(shell.focused_game().as_deref(), Some(ids[3].as_str()));
        // Categorias por loja: cada jogo em uma fileira; "Ver todos" abre a grade filtrada.
        shell.ctx.settings.set(GROUP_KEY, GroupBy::Store.key());
        shell.click("view", 2);
        shell.render(std::time::Instant::now());
        let rows = page(&shell).cat_rows.clone();
        assert_eq!(
            rows.iter().map(Vec::len).sum::<usize>(),
            ids.len().min(rows.len() * ROW_COVERS)
        );
        let first_store = shell.ctx.library.find(&rows[0][0]).unwrap().store();
        shell.click("cat-all", 0);
        assert_eq!(View::of(&shell.ctx), View::Grid);
        assert!(
            page(&shell)
                .games(&shell.ctx)
                .iter()
                .all(|g| g.store() == first_store)
        );
        // Inverter a ordem: a mesma lista de trás para a frente.
        page(&shell).criteria.borrow_mut().sources.clear();
        shell.click("reverse", 0);
        // Os não instalados continuam depois dos instalados em qualquer ordem: a inversão vale para cada trecho.
        let installed = |ids: Vec<String>| -> Vec<String> {
            ids.into_iter()
                .filter(|id| shell.ctx.library.find(id).is_some_and(|g| g.is_installed))
                .collect()
        };
        let mut back = installed(page(&shell).ids(&shell.ctx));
        back.reverse();
        assert_eq!(back, installed(ids));
    }

    /// A biblioteca de demonstração mais os não instalados de três lojas (um baixando).
    fn shell_with_store_games() -> (tempfile::TempDir, Shell) {
        let (dir, mut shell) = shell();
        shell
            .ctx
            .found
            .lock()
            .unwrap()
            .extend(crate::demo::not_installed());
        shell.ctx.library.refresh();
        (dir, shell)
    }

    use slint::Model;
    use slint_interpreter::Value;

    /// Uma linha de `desk-games` como o tema a recebe.
    struct Row(slint_interpreter::Struct);

    impl Row {
        fn text(&self, field: &str) -> String {
            match self.0.get_field(field) {
                Some(Value::String(s)) => s.to_string(),
                _ => String::new(),
            }
        }
        fn flag(&self, field: &str) -> bool {
            matches!(self.0.get_field(field), Some(Value::Bool(true)))
        }
        fn number(&self, field: &str) -> i32 {
            match self.0.get_field(field) {
                Some(Value::Number(n)) => *n as i32,
                _ => 0,
            }
        }
    }

    fn rows(shell: &mut Shell) -> Vec<Row> {
        model(shell, "desk-games")
    }

    fn model(shell: &mut Shell, prop: &str) -> Vec<Row> {
        let out = shell.render(std::time::Instant::now());
        out.props
            .iter()
            .find_map(|(n, v)| match (*n, v) {
                (name, Value::Model(m)) if name == prop => Some(m.clone()),
                _ => None,
            })
            .map(|m| {
                (0..m.row_count())
                    .filter_map(|i| match m.row_data(i)? {
                        Value::Struct(s) => Some(Row(s)),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn the_list_puts_installed_first_then_one_group_per_store_with_the_most_games_first() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell_with_store_games();
        let games = page(&shell).games(&shell.ctx);
        let kinds: Vec<(bool, GameStore)> =
            games.iter().map(|g| (to_install(g), g.store())).collect();
        let first_pending = kinds.iter().position(|k| k.0).unwrap();
        assert!(
            kinds[first_pending..].iter().all(|k| k.0),
            "instalados antes"
        );
        let stores: Vec<GameStore> = runs(&games).iter().map(|r| r.store).collect();
        // Steam 7 jogos, Epic 5 (4 + o da demonstração), GOG 3, Xbox 1: a com mais jogos primeiro.
        assert_eq!(
            stores[..3],
            [GameStore::Steam, GameStore::Epic, GameStore::Gog]
        );
        let list = entries(&shell.ctx, &games);
        assert_eq!(
            list.iter().filter(|e| matches!(e, Entry::Section)).count(),
            1
        );
        let rows = rows(&mut shell);
        let section = rows.iter().find(|r| r.text("kind") == "section").unwrap();
        assert_eq!(section.text("title"), "NÃO INSTALADOS");
        let steam = rows.iter().find(|r| r.text("kind") == "group").unwrap();
        assert_eq!(
            (
                steam.text("title").as_str(),
                steam.text("count").as_str(),
                steam.flag("open")
            ),
            ("Steam", "7", true)
        );
        let installing = rows.iter().find(|r| r.flag("installing")).unwrap();
        assert_eq!(installing.text("percent"), "42%");
        assert_eq!(installing.text("install-label"), "Baixando Disco Elysium");
        let waiting = rows.iter().find(|r| r.text("title") == "Tunic").unwrap();
        assert_eq!(waiting.text("install-label"), "Instalar Tunic");
    }

    #[test]
    fn groups_open_and_close_by_keyboard_and_click_and_the_choice_is_saved() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell_with_store_games();
        let total = rows(&mut shell).len();
        // ↓ do último instalado passa pelo cabeçalho (a divisória não recebe foco).
        let installed = rows(&mut shell)
            .iter()
            .position(|r| r.text("kind") == "section")
            .unwrap();
        shell.click("game", installed - 1);
        shell.handle(Cmd::Down);
        assert_eq!(page(&shell).group_focus, Some(GameStore::Steam));
        let selected = rows(&mut shell);
        assert_eq!(selected[installed + 1].text("kind"), "group");
        // Enter fecha o grupo, → abre outra vez.
        shell.handle(Cmd::Accept);
        assert_eq!(rows(&mut shell).len(), total - 7);
        assert!(group_closed(&shell.ctx, GameStore::Steam));
        shell.handle(Cmd::Right);
        assert_eq!(rows(&mut shell).len(), total);
        shell.handle(Cmd::Left);
        assert!(group_closed(&shell.ctx, GameStore::Steam));
        // ↓ segue para o próximo cabeçalho; clicar no cabeçalho alterna.
        shell.handle(Cmd::Down);
        assert_eq!(page(&shell).group_focus, Some(GameStore::Epic));
        shell.click("group", 0);
        assert!(!group_closed(&shell.ctx, GameStore::Steam));
        // Tirar o cabeçalho do foco volta ao jogo (clique num jogo).
        shell.click("game", 0);
        assert_eq!(page(&shell).group_focus, None);
    }

    #[test]
    fn the_install_filter_hides_the_section_or_shows_only_the_groups() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell_with_store_games();
        page(&shell).criteria.borrow_mut().install = InstallFilter::Installed;
        assert!(rows(&mut shell).iter().all(|r| r.text("kind").is_empty()));
        page(&shell).criteria.borrow_mut().install = InstallFilter::NotInstalled;
        let only = rows(&mut shell);
        assert_eq!(only[0].text("kind"), "group");
        assert!(only.iter().all(|r| r.text("kind") != "section"));
    }

    #[test]
    fn a_broken_progress_from_the_store_never_reaches_the_ring_as_garbage() {
        use tako_core::library::InstallProgress;
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        let mut bad = crate::demo::not_installed();
        bad.truncate(3);
        for (g, f) in bad.iter_mut().zip([f32::NAN, 7.0, -3.0]) {
            g.installing = Some(InstallProgress { fraction: Some(f) });
        }
        shell.ctx.found.lock().unwrap().extend(bad);
        shell.ctx.library.refresh();
        let rows = rows(&mut shell);
        let got: Vec<(f32, String)> = rows
            .iter()
            .filter(|r| r.flag("installing"))
            .map(|r| {
                let p = match r.0.get_field("progress") {
                    Some(Value::Number(n)) => *n as f32,
                    _ => f32::NAN,
                };
                (p, r.text("percent"))
            })
            .collect();
        assert_eq!(got.len(), 3);
        // Sem número válido o anel gira (negativo, sem %); fora de 0..1 trava nas pontas.
        assert!(got.contains(&(-1.0, String::new())));
        assert!(got.contains(&(1.0, "100%".into())));
        assert!(got.contains(&(0.0, "0%".into())));
    }

    #[test]
    fn the_grid_has_one_block_per_store_and_the_arrows_walk_covers_and_headers() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell_with_store_games();
        shell.ctx.grid_columns = 4;
        shell.click("view", 1);
        let blocks = model(&mut shell, "desk-blocks");
        let kinds: Vec<String> = blocks.iter().map(|b| b.text("kind")).collect();
        assert_eq!(kinds[..3], ["covers", "section", "group"]);
        // 10 instalados em 4 colunas = 3 linhas; a divisória vem depois delas, o primeiro grupo depois da divisória.
        assert_eq!(blocks[1].number("rows-before"), 3);
        assert_eq!(blocks[2].number("rows-before"), 3);
        assert_eq!(blocks[2].number("sections-before"), 1);
        assert_eq!(blocks[3].number("groups-before"), 1);
        // Do último instalado, ↓ cai no cabeçalho da Steam (a divisória não recebe foco); ↓ de novo, na primeira capa.
        let installed = page(&shell)
            .games(&shell.ctx)
            .iter()
            .filter(|g| !to_install(g))
            .count();
        shell.click("game", installed - 1);
        shell.handle(Cmd::Down);
        assert_eq!(page(&shell).group_focus, Some(GameStore::Steam));
        shell.handle(Cmd::Down);
        assert_eq!(page(&shell).group_focus, None);
        let focused = shell.focused_game().unwrap();
        assert!(
            shell
                .ctx
                .library
                .find(&focused)
                .is_some_and(|g| !g.is_installed)
        );
        // Enter numa capa que falta instalar abre a instalação na loja; o jogo não vai para o destaque da lista.
        shell.handle(Cmd::Accept);
        assert!(
            shell
                .ctx
                .install_queue
                .iter()
                .any(|(_, o)| o.install_uri.starts_with("demo://install/"))
        );
        assert_eq!(View::of(&shell.ctx), View::Grid);
        // ↑ volta ao cabeçalho; Enter fecha o grupo e o trecho fica sem capas.
        shell.handle(Cmd::Up);
        assert_eq!(page(&shell).group_focus, Some(GameStore::Steam));
        shell.handle(Cmd::Accept);
        let closed = model(&mut shell, "desk-blocks");
        assert_eq!(closed[2].text("count"), "7");
        assert!(!closed[2].flag("open"));
    }

    /// O contador de Page Up e Page Down que o tema lê para rolar a página do jogo.
    fn page_scroll(shell: &mut Shell) -> f64 {
        let out = shell.render(std::time::Instant::now());
        out.props
            .iter()
            .find_map(|(n, v)| match (*n, v) {
                ("desk-scroll-page", slint_interpreter::Value::Number(n)) => Some(*n),
                _ => None,
            })
            .unwrap_or_default()
    }

    #[test]
    fn page_keys_scroll_the_game_page_and_only_there() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        // A Visão geral não tem a página: as teclas não contam.
        shell.show_overview();
        shell.key("\u{F72D}", KeyModifiers::NONE);
        assert_eq!(page_scroll(&mut shell), 0.);
        // Escolhido um jogo, Page Down rola uma tela para baixo e Page Up uma para cima.
        shell.click("game", 0);
        shell.key("\u{F72D}", KeyModifiers::NONE);
        shell.key("\u{F72D}", KeyModifiers::NONE);
        assert_eq!(page_scroll(&mut shell), 2.);
        shell.key("\u{F72C}", KeyModifiers::NONE);
        assert_eq!(page_scroll(&mut shell), 1.);
        // Também com o foco nos botões de ação.
        for _ in 0..8 {
            if page(&shell).zone() == Zone::Actions {
                break;
            }
            shell.key("\u{0009}", KeyModifiers::NONE);
        }
        assert_eq!(page(&shell).zone(), Zone::Actions);
        shell.key("\u{F72D}", KeyModifiers::NONE);
        assert_eq!(page_scroll(&mut shell), 2.);
        // A página rolou: o foco das ações sairia da tela, então ele passa para o corpo.
        assert_eq!(page(&shell).zone(), Zone::Body);
        // Com Ctrl, não: o atalho é de outra coisa. As Categorias não têm a página.
        shell.key("\u{F72D}", KeyModifiers::CTRL);
        assert_eq!(page_scroll(&mut shell), 2.);
        shell.click("view", 2);
        shell.key("\u{F72D}", KeyModifiers::NONE);
        assert_eq!(page_scroll(&mut shell), 2.);
    }

    /// O foco do teclado e do controle da página do jogo: ↓ das ações entra no corpo e rola, ↑ rola de volta e, no topo,
    /// devolve às ações. O tema avisa onde a página está (`page-edge`).
    #[test]
    fn arrows_and_dpad_scroll_the_game_page_body() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        shell.click("game", 0);
        shell.key("\u{0009}", KeyModifiers::NONE);
        assert_eq!(page(&shell).zone(), Zone::Actions);
        // ↓ entra no corpo e já rola um passo; os seguintes rolam mais.
        shell.handle(Cmd::Down);
        assert_eq!(page(&shell).zone(), Zone::Body);
        assert_eq!(page_scroll(&mut shell), BODY_STEP as f64);
        shell.click("page-edge", 1);
        shell.handle(Cmd::Down);
        assert_eq!(page_scroll(&mut shell), 2. * BODY_STEP as f64);
        // No fim da página ↓ não rola mais.
        shell.click("page-edge", 2);
        shell.handle(Cmd::Down);
        assert_eq!(page_scroll(&mut shell), 2. * BODY_STEP as f64);
        // ↑ rola de volta enquanto a página não chegou ao topo…
        shell.handle(Cmd::Up);
        shell.click("page-edge", 1);
        shell.handle(Cmd::Up);
        assert_eq!(page_scroll(&mut shell), 0.);
        assert_eq!(page(&shell).zone(), Zone::Body);
        // …e no topo devolve o foco às ações.
        shell.click("page-edge", 0);
        shell.handle(Cmd::Up);
        assert_eq!(page(&shell).zone(), Zone::Actions);
        // O aviso da página não é gesto do usuário: o anel de foco continua.
        shell.key("\u{0009}", KeyModifiers::SHIFT);
        shell.handle(Cmd::Right);
        shell.handle(Cmd::Down);
        assert!(shell.ctx.focus_visible);
        shell.click("page-edge", 1);
        assert!(shell.ctx.focus_visible);
        // ← sai do corpo para a lista; Tab sai dele para a zona seguinte às ações.
        shell.handle(Cmd::Left);
        assert_eq!(page(&shell).zone(), Zone::List);
        shell.handle(Cmd::Right);
        shell.handle(Cmd::Down);
        assert_eq!(page(&shell).zone(), Zone::Body);
        shell.key("\u{0009}", KeyModifiers::NONE);
        assert_eq!(page(&shell).zone(), Zone::Bar);
    }

    #[test]
    fn game_details_follow_the_card_order_with_status_last() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, shell) = shell();
        let g = shell
            .ctx
            .library
            .games()
            .iter()
            .find(|g| g.developer.is_some() && g.publisher.is_some())
            .cloned()
            .or_else(|| shell.ctx.library.games().first().cloned())
            .unwrap();
        let mut g = g;
        g.developer = Some("Dev".into());
        g.publisher = Some("Pub".into());
        g.size_label = Some("23 GB".into());
        let labels: Vec<String> = more_details(&shell.ctx, &g)
            .into_iter()
            .map(|d| d.label.to_string())
            .collect();
        let at = |key: &str| labels.iter().position(|l| *l == shell.ctx.t(key)).unwrap();
        assert!(at("details.developer") < at("desktop.page.publisher"));
        assert!(at("desktop.page.publisher") < at("desktop.size"));
        assert_eq!(at("details.status"), labels.len() - 1);
        assert_eq!(at("desktop.size"), labels.len() - 2);
    }

    #[test]
    fn the_back_button_of_the_game_page_returns_to_the_overview() {
        let (_dir, mut shell) = shell();
        shell.click("game", 1);
        assert!(page(&shell).selected(&shell.ctx).is_some());
        shell.click("game-back", 0);
        let out = shell.render(std::time::Instant::now());
        let on = out.props.iter().find_map(|(n, v)| match (*n, v) {
            ("desk-overview", slint_interpreter::Value::Struct(s)) => s.get_field("on").cloned(),
            _ => None,
        });
        assert_eq!(on, Some(slint_interpreter::Value::Bool(true)));
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

    fn overview_prop(shell: &mut Shell, field: &str) -> Option<slint_interpreter::Value> {
        let out = shell.render(std::time::Instant::now());
        out.props.iter().find_map(|(n, v)| match (*n, v) {
            ("desk-overview", slint_interpreter::Value::Struct(s)) => s.get_field(field).cloned(),
            _ => None,
        })
    }

    #[test]
    fn the_app_opens_on_the_overview_and_choosing_a_game_leaves_it() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        shell.show_overview();
        assert_eq!(
            overview_prop(&mut shell, "on"),
            Some(slint_interpreter::Value::Bool(true))
        );
        let hero = overview::hero(&shell.ctx).map(|g| g.id.clone()).unwrap();
        // A cor da tela vem do jogo do destaque; a lista não marca nenhum jogo e o destaque comum fica de fora.
        assert_eq!(shell.focused_game(), Some(hero.clone()));
        let out = shell.render(std::time::Instant::now());
        let number = |name: &str| match out.props.iter().find(|(n, _)| *n == name) {
            Some((_, slint_interpreter::Value::Number(n))) => *n as i32,
            _ => panic!("sem {name}"),
        };
        assert_eq!(number("desk-selected"), -1);
        let has_game = out.props.iter().any(|(n, v)| match (*n, v) {
            ("desk-game", slint_interpreter::Value::Struct(s)) => {
                s.get_field("has-game") == Some(&slint_interpreter::Value::Bool(true))
            }
            _ => false,
        });
        assert!(!has_game);
        // Trocar de aba e buscar não saem dela; escolher um jogo sai.
        shell.click("tab", 1);
        shell.edited("search", "a");
        assert!(page(&shell).overview_on(&shell.ctx));
        shell.edited("search", "");
        shell.click("tab", 0);
        shell.click("game", 1);
        assert!(!page(&shell).overview_on(&shell.ctx));
        assert_eq!(
            overview_prop(&mut shell, "on"),
            Some(slint_interpreter::Value::Bool(false))
        );
        // A linha Visão geral volta, e Esc na coleção também não a deixa para trás.
        shell.click("overview", 0);
        assert!(page(&shell).overview_on(&shell.ctx));
        // A Grade da lista mantém a Visão geral; as Categorias, que tomam a janela toda, não.
        shell.click("view", 1);
        assert!(page(&shell).overview_on(&shell.ctx));
        shell.click("view", 2);
        assert!(!page(&shell).overview_on(&shell.ctx));
    }

    #[test]
    fn the_overview_opens_and_plays_games_outside_the_current_tab() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        shell.show_overview();
        let hero = overview::hero(&shell.ctx).map(|g| g.id.clone()).unwrap();
        let back = overview::game_of(&shell.ctx, Target::Back(0))
            .map(|g| g.id.clone())
            .unwrap();
        // Favoritos não tem o destaque: jogar nele joga o jogo certo, não o primeiro da aba.
        shell.click("tab", 1);
        assert!(!page(&shell).ids(&shell.ctx).contains(&hero));
        shell.click("ov-play-hero", 0);
        assert!(shell.ctx.effects.contains(&Effect::Launch(hero.clone())) || shell.has_layers());
        shell.close_layers();
        shell.ctx.effects.clear();
        // Abrir um de Voltar a jogar volta para Todos (sem a busca) e deixa o jogo escolhido.
        shell.click("ov-back", 0);
        assert!(!page(&shell).overview_on(&shell.ctx));
        assert_eq!(page(&shell).tab(), Tab::All);
        assert_eq!(shell.focused_game(), Some(back.clone()));
        // O ▶ de Voltar a jogar joga o jogo da linha.
        shell.click("overview", 0);
        let other = overview::game_of(&shell.ctx, Target::Back(1))
            .map(|g| g.id.clone())
            .unwrap();
        shell.click("ov-back-play", 1);
        assert!(shell.ctx.effects.contains(&Effect::Launch(other)) || shell.has_layers());
    }

    #[test]
    fn the_keyboard_walks_hero_then_back_to_play_and_leaves_through_the_list_and_the_top() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        shell.show_overview();
        // Só o destaque e Voltar a jogar: as seções vêm depois (outro teste).
        let n = overview::targets(&shell.ctx, shell.ctx.overview_show_back)
            .iter()
            .take_while(|t| matches!(t, Target::Hero | Target::Back(_)))
            .count();
        assert!(n >= 3);
        assert_eq!(page(&shell).zone(), Zone::Actions);
        // ↓ anda pelo destaque e por Voltar a jogar.
        for _ in 0..n - 1 {
            shell.handle(Cmd::Down);
        }
        assert_eq!(page(&shell).index, n - 1);
        shell.handle(Cmd::Up);
        assert_eq!(page(&shell).index, n - 2);
        // Enter num jogo de Voltar a jogar abre a página dele.
        let id = overview::game_of(&shell.ctx, Target::Back(n - 3))
            .map(|g| g.id.clone())
            .unwrap();
        shell.handle(Cmd::Accept);
        assert!(!page(&shell).overview_on(&shell.ctx));
        assert_eq!(shell.focused_game(), Some(id));
        // No destaque, Enter joga o jogo do destaque; ← vai para a lista e ↑ para a linha Visão geral.
        shell.click("overview", 0);
        let hero = overview::hero(&shell.ctx).map(|g| g.id.clone()).unwrap();
        shell.handle(Cmd::Accept);
        assert!(shell.ctx.effects.contains(&Effect::Launch(hero)) || shell.has_layers());
        shell.close_layers();
        shell.handle(Cmd::Left);
        assert_eq!(page(&shell).zone(), Zone::List);
        shell.handle(Cmd::Right);
        assert_eq!(page(&shell).zone(), Zone::Actions);
        shell.handle(Cmd::Up);
        assert_eq!(page(&shell).zone(), Zone::Controls);
        assert_eq!(
            page(&shell).controls(&shell.ctx).get(page(&shell).index),
            Some(&Control::Overview)
        );
        // F (favoritar) age no alvo com o foco, não no primeiro jogo da lista escondido.
        shell.click("overview", 0);
        shell.handle(Cmd::Down);
        let target = overview::game_of(&shell.ctx, Target::Back(0))
            .map(|g| g.id.clone())
            .unwrap();
        let before = shell.ctx.library.find(&target).unwrap().is_favorite;
        shell.handle(Cmd::Favorite);
        assert_ne!(shell.ctx.library.find(&target).unwrap().is_favorite, before);
    }

    #[test]
    fn hidden_back_column_is_skipped_by_keyboard_focus() {
        let (_dir, mut shell) = shell();
        shell.show_overview();
        shell.ctx.overview_show_back = false;
        let visible_targets = overview::targets(&shell.ctx, false);
        assert!(
            visible_targets
                .iter()
                .all(|target| !matches!(target, Target::Back(_)))
        );
        shell.handle(Cmd::Down);
        assert!(page(&shell).index < visible_targets.len());
        assert!(!matches!(
            visible_targets[page(&shell).index],
            Target::Back(_)
        ));
        shell.handle(Cmd::Accept);
        assert!(!shell
            .ctx
            .effects
            .iter()
            .any(|effect| matches!(effect, Effect::Launch(id) if overview::game_of(&shell.ctx, Target::Back(0)).is_some_and(|game| game.id == *id))));
        assert!(page(&shell).overview_on(&shell.ctx));
    }

    #[test]
    fn the_overview_sections_take_focus_in_order_and_the_cover_row_walks_sideways() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        crate::demo::sessions(&mut shell.ctx);
        shell.show_overview();
        let targets = overview::targets(&shell.ctx, shell.ctx.overview_show_back);
        let first_cover = targets
            .iter()
            .position(|t| matches!(t, Target::Cover(_)))
            .unwrap();
        assert!(
            first_cover > 4,
            "destaque, Voltar a jogar e as seções vêm antes das capas"
        );
        // A ordem da tela: destaque, Voltar a jogar, Jogo da semana, atenção, capas.
        let order: Vec<u8> = targets
            .iter()
            .map(|t| match t {
                Target::Hero => 0,
                Target::Back(_) => 1,
                Target::Best => 2,
                Target::Attention(_) => 3,
                Target::Cover(_) => 4,
            })
            .collect();
        assert!(order.windows(2).all(|w| w[0] <= w[1]));
        for _ in 0..first_cover {
            shell.handle(Cmd::Down);
        }
        assert_eq!(page(&shell).index, first_cover);
        // ← → andam só dentro da fileira: ← na primeira capa fica; ↓ no fim fica.
        shell.handle(Cmd::Left);
        assert_eq!(page(&shell).index, first_cover);
        shell.handle(Cmd::Right);
        shell.handle(Cmd::Right);
        assert_eq!(page(&shell).index, first_cover + 2);
        let out = shell.render(std::time::Instant::now());
        let focus = out.props.iter().find_map(|(n, v)| match (*n, v) {
            ("desk-overview", slint_interpreter::Value::Struct(s)) => Some((
                s.get_field("focus-kind").cloned(),
                s.get_field("focus-index").cloned(),
            )),
            _ => None,
        });
        assert_eq!(
            focus,
            Some((
                Some(slint_interpreter::Value::String("cover".into())),
                Some(slint_interpreter::Value::Number(2.))
            ))
        );
        shell.handle(Cmd::Down);
        assert_eq!(page(&shell).index, first_cover + 2);
        // ↑ sai da fileira para a seção de cima, não para a capa de cima.
        shell.handle(Cmd::Up);
        assert_eq!(page(&shell).index, first_cover - 1);
        // Enter numa capa abre a página do jogo; duplo clique (▶) joga.
        shell.handle(Cmd::Down);
        let id = overview::game_of(&shell.ctx, Target::Cover(0))
            .map(|g| g.id.clone())
            .unwrap();
        shell.handle(Cmd::Accept);
        assert!(!page(&shell).overview_on(&shell.ctx));
        assert_eq!(shell.focused_game(), Some(id));
        shell.click("overview", 0);
        let other = overview::game_of(&shell.ctx, Target::Cover(1))
            .map(|g| g.id.clone())
            .unwrap();
        shell.click("ov-cover-play", 1);
        assert!(shell.ctx.effects.contains(&Effect::Launch(other)) || shell.has_layers());
        shell.close_layers();
        // O Jogo da semana abre a página dele.
        shell.click("overview", 0);
        let best = overview::game_of(&shell.ctx, Target::Best)
            .map(|g| g.id.clone())
            .unwrap();
        shell.click("ov-best", 0);
        assert_eq!(shell.focused_game(), Some(best));
    }

    #[test]
    fn attention_rows_redo_the_failed_work_choose_art_or_open_the_game() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        let now = std::time::Instant::now();
        let list = &mut shell.ctx.activity.list;
        list.start(
            "download:7",
            tako_core::activities::Kind::Download,
            "x",
            "",
            now,
        );
        list.fail(
            "download:7",
            "Não baixou X",
            Some(tako_core::activities::Action::Retry),
            now,
        );
        shell.show_overview();
        let first = overview::attention_of(&shell.ctx, 0).unwrap();
        assert_eq!(first.state, "error");
        // O botão da linha tira o erro da fila (quem alimenta a fila começa outro trabalho).
        shell.click("ov-att-action", 0);
        assert!(shell.ctx.activity.list.get("download:7").is_none());
        // Sem a capa, a ação abre o seletor de imagens; o clique na linha abre a página do jogo.
        shell.ctx.found.lock().unwrap()[0].cover_uri = None;
        shell.ctx.library.refresh();
        let art = (0..)
            .map_while(|i| overview::attention_of(&shell.ctx, i))
            .position(|a| a.state == "art")
            .unwrap();
        shell.click("ov-att-action", art);
        assert!(shell.has_layers());
        shell.close_layers();
        let id = overview::attention_of(&shell.ctx, art).unwrap().id;
        shell.click("ov-att", art);
        assert!(!page(&shell).overview_on(&shell.ctx));
        assert_eq!(shell.focused_game(), Some(id));
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
    fn fixed_filter_chips_remove_each_criterion_without_touching_search_or_sort() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = crate::app::build_ctx(
            tako_core::paths::DataPaths::at(dir.path()),
            tako_platform::Platform::simulated(),
            true,
        );
        let mut page = DesktopLibrary::new();
        *page.criteria.borrow_mut() = LibraryCriteria {
            search: "hades".into(),
            install: InstallFilter::Installed,
            favorites_only: true,
            played_only: true,
            broken_only: true,
            play_time: tako_core::library::criteria::PlayTimeFilter::Unplayed,
            no_status: true,
            statuses: [PlayStatus::Playing].into(),
            sources: ["Steam".into()].into(),
            genres: ["Ação".into()].into(),
            tags: ["Sofá".into()].into(),
            collections: ["collection-id".into()].into(),
            reversed: true,
            ..Default::default()
        };
        let sort = page.criteria.borrow().sort;
        while page.criteria.borrow().has_filters() {
            let count = page.criteria.borrow().active_filter_count();
            assert_eq!(page.active_filters(&ctx).len(), count);
            page.remove_filter(&ctx, usize::MAX);
            assert_eq!(page.criteria.borrow().active_filter_count(), count);
            page.remove_filter(&ctx, 0);
            assert_eq!(page.criteria.borrow().active_filter_count(), count - 1);
            let c = page.criteria.borrow();
            assert_eq!(c.search, "hades");
            assert_eq!(c.sort, sort);
            assert!(c.reversed);
        }
        assert!(page.active_filters(&ctx).is_empty());
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
        // Ordenar, ver e filtrar: um menu só; Visão geral sai da busca e volta para Todos.
        shell.click("options", 0);
        assert!(shell.has_layers());
        shell.close_layers();
        shell.edited("search", &title);
        shell.click("overview", 0);
        assert_eq!(page(&shell).ids(&shell.ctx).len(), all);

        // Instalados: só o que está no PC.
        let installed = shell
            .ctx
            .library
            .games()
            .iter()
            .filter(|g| g.is_installed)
            .count();
        shell.click("tab", 2);
        assert_eq!(page(&shell).tab(), Tab::Installed);
        assert_eq!(page(&shell).ids(&shell.ctx).len(), installed);
        assert!(installed < all);
        shell.click("tab", 0);

        // Coleções (sem aba, pelo menu): vazia mostra o aviso; com uma do usuário e uma automática, as duas viram
        // escolhas.
        shell.click("collection-select", 0);
        shell.close_layers();
        open_collections_tab(&mut shell);
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
        // O × do chip volta para Todos.
        shell.click("collection-clear", 0);
        assert_eq!(page(&shell).tab(), Tab::All);
        assert_eq!(page(&shell).ids(&shell.ctx).len(), all);
    }

    #[test]
    fn keyboard_enters_a_collection_through_the_options_menu_and_leaves_with_esc() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        let first = shell.ctx.library.games()[0].id.clone();
        shell
            .ctx
            .collections
            .create("Sofá", Some(first.as_str()))
            .unwrap();
        shell.ctx.sync_collections();
        let all = page(&shell).ids(&shell.ctx).len();

        // Tab chega ao topo da lista; o primeiro controle é Ordenar, ver e filtrar.
        while page(&shell).zone() != Zone::Controls {
            shell.key("\t", KeyModifiers::NONE);
        }
        assert_eq!(page(&shell).index, 0);
        shell.handle(Cmd::Accept);
        assert!(shell.has_layers());
        for _ in 0..40 {
            let o = shell.layers.last().unwrap().render(&shell.ctx);
            if o.rows[o.sel as usize].key == "pick-0" {
                break;
            }
            shell.handle(Cmd::Down);
        }
        shell.handle(Cmd::Accept);
        shell.render(std::time::Instant::now());
        assert!(!shell.has_layers());
        assert_eq!(page(&shell).tab(), Tab::Collections);
        assert_eq!(page(&shell).ids(&shell.ctx), vec![first]);

        // No chip (depois do botão de recolher), Enter abre o seletor, sem trocar de coleção em silêncio.
        assert_eq!(page(&shell).zone(), Zone::Controls);
        shell.handle(Cmd::Right);
        shell.handle(Cmd::Right);
        assert_eq!(page(&shell).index, 2);
        shell.handle(Cmd::Accept);
        assert!(shell.has_layers());
        shell.close_layers();
        assert_eq!(page(&shell).tab(), Tab::Collections);
        // Esc no botão de recolher não sai da coleção; só no chip.
        shell.handle(Cmd::Left);
        shell.handle(Cmd::Back);
        assert_eq!(page(&shell).tab(), Tab::Collections);
        while page(&shell).zone() != Zone::Controls {
            shell.key("\t", KeyModifiers::NONE);
        }
        while page(&shell).index < 2 {
            shell.handle(Cmd::Right);
        }
        // Esc no chip faz o mesmo que o ×: volta para Todos.
        shell.handle(Cmd::Back);
        assert_eq!(page(&shell).tab(), Tab::All);
        assert_eq!(page(&shell).ids(&shell.ctx).len(), all);
    }

    #[test]
    fn section_buttons_from_a_collection_start_before_the_first_tab() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        open_collections_tab(&mut shell);
        shell.handle(Cmd::NextSection);
        assert_eq!(page(&shell).tab(), Tab::All);
        open_collections_tab(&mut shell);
        shell.handle(Cmd::PrevSection);
        assert_eq!(page(&shell).tab(), Tab::Installed);
    }

    #[test]
    fn installed_tab_counts_and_saves_only_installed_games() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        shell.click("tab", 2);
        assert_eq!(page(&shell).route_filter(), LibraryFilter::Installed);
    }

    #[test]
    fn ctrl_b_collapses_the_list_and_the_grid_becomes_the_icon_list() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        let flag = |shell: &mut Shell, name: &str| {
            let out = shell.render(std::time::Instant::now());
            out.props
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| v.clone())
        };
        shell.click("view", 1);
        assert_eq!(flag(&mut shell, "desk-collapsed"), Some(Value::Bool(false)));
        assert_eq!(
            flag(&mut shell, "desk-view"),
            Some(Value::String("grid".into()))
        );
        // Recolhida, a Grade vira a lista de ícones (a escolha da Grade fica salva).
        shell.key("b", KeyModifiers::CTRL);
        assert_eq!(flag(&mut shell, "desk-collapsed"), Some(Value::Bool(true)));
        assert_eq!(
            flag(&mut shell, "desk-view"),
            Some(Value::String("list".into()))
        );
        // O botão ao lado da busca (e a lupa) expande de novo.
        shell.click("collapse", 0);
        assert_eq!(flag(&mut shell, "desk-collapsed"), Some(Value::Bool(false)));
        assert_eq!(
            flag(&mut shell, "desk-view"),
            Some(Value::String("grid".into()))
        );
    }

    #[test]
    fn keyboard_reaches_the_search_and_collapse_buttons_of_the_collapsed_list() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        let prop = |shell: &mut Shell, name: &str| {
            let out = shell.render(std::time::Instant::now());
            out.props
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| v.clone())
        };
        shell.key("b", KeyModifiers::CTRL);
        assert_eq!(prop(&mut shell, "desk-collapsed"), Some(Value::Bool(true)));
        while page(&shell).zone() != Zone::Controls {
            shell.key("\t", KeyModifiers::NONE);
        }
        // A lupa pede o foco da busca (o tema expande a lista e foca); o botão ao lado expande.
        let before = prop(&mut shell, "desk-search-focus");
        shell.handle(Cmd::Accept);
        assert_ne!(prop(&mut shell, "desk-search-focus"), before);
        shell.handle(Cmd::Right);
        shell.handle(Cmd::Accept);
        assert_eq!(prop(&mut shell, "desk-collapsed"), Some(Value::Bool(false)));
        // Expandida, o primeiro controle volta a ser o botão de opções.
        shell.handle(Cmd::Left);
        shell.handle(Cmd::Accept);
        assert!(shell.has_layers());
    }

    #[test]
    fn rescan_busy_shows_busy_state_when_library_not_empty_and_toasts_if_clicked_again() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        assert!(!shell.ctx.library.games().is_empty());

        // Estado normal sem varredura: não ocupado e rótulo padrão
        let out = shell.render(std::time::Instant::now());
        let busy = out.props.iter().find(|(n, _)| *n == "desk-rescan-busy");
        assert_eq!(busy.map(|(_, v)| v), Some(&Value::Bool(false)));

        // Enquanto atualiza com a biblioteca preenchida:
        shell.ctx.scanning = true;
        let out = shell.render(std::time::Instant::now());
        let busy = out.props.iter().find(|(n, _)| *n == "desk-rescan-busy");
        assert_eq!(busy.map(|(_, v)| v), Some(&Value::Bool(true)));
        let count = out.props.iter().find(|(n, _)| *n == "desk-count");
        assert_eq!(
            count.map(|(_, v)| v),
            Some(&Value::String("Atualizando…".into()))
        );
        let count_short = out.props.iter().find(|(n, _)| *n == "desk-count-short");
        assert_eq!(
            count_short.map(|(_, v)| v),
            Some(&Value::String("Atualizando…".into()))
        );

        // Clicar novamente durante atualização exibe toast e não duplica efeito
        shell.click("rescan", 0);
        assert_eq!(
            shell.ctx.toast.as_ref().map(|(t, _)| t.as_str()),
            Some("A biblioteca já está sendo atualizada")
        );
        assert!(!shell.ctx.effects.contains(&Effect::UserRescan));

        // Clicar em repouso enfileira UserRescan
        shell.ctx.scanning = false;
        shell.click("rescan", 0);
        assert!(shell.ctx.effects.contains(&Effect::UserRescan));
    }
}
