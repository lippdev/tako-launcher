//! Estado da interface (o `ShellViewModel` do WPF): páginas com histórico, barra lateral, camadas, dicas e
//! avisos. Puro Rust e testável sem janela: `render` produz as propriedades do tema, o `bridge` as aplica.

pub mod art;
pub mod control_center;
pub mod ctx;
pub mod details;
pub mod dialogs;
pub mod edit_game;
pub mod game_settings;
pub mod home;
pub mod keyboard;
pub mod keys;
pub mod layer;
pub mod library;
pub mod settings;
pub mod sidebar;
pub mod source_ids;
pub mod tiles;

pub use ctx::{Ctx, Effect, InputKind, RunningGame, Settings};
pub use layer::{Flow, Layer, MenuItem, MenuLayer};

use crate::model::Hint;
use crate::values::ToValue;
use slint_interpreter::Value;
use std::time::{Duration, Instant};

/// Comandos da interface (os `AppCommand` do C#), vindos do controle, do teclado ou de um clique numa dica.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cmd {
    Up,
    Down,
    Left,
    Right,
    Accept,
    Back,
    Favorite,
    Search,
    PrevSection,
    NextSection,
    Menu,
    Guide,
}

/// Propriedades do tema calculadas num desenho.
#[derive(Default)]
pub struct Out {
    pub props: Vec<(&'static str, Value)>,
}

impl Out {
    pub fn set(&mut self, name: &'static str, v: impl ToValue) {
        self.props.push((name, v.to_value()));
    }
}

/// O que a página pede depois de um comando.
pub enum Nav {
    Stay,
    Push(Box<dyn Page>),
    Replace(Box<dyn Page>),
    Back,
    Layer(Box<dyn Layer>),
    /// O foco vai para a barra lateral (← na primeira coluna).
    Sidebar,
}

pub trait Page {
    /// Id do item da barra lateral desta página (`home`, `library:all`, `settings`...).
    fn route(&self) -> String;
    /// Nome da página no tema (`home`, `library`, `details`, `settings`).
    fn kind(&self) -> &'static str;
    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Nav;
    fn render(&mut self, ctx: &Ctx, out: &mut Out);
    fn hints(&self, ctx: &Ctx) -> Vec<Hint>;
    fn click(&mut self, zone: &str, index: usize, ctx: &mut Ctx) -> Nav {
        self.hover(zone, index, ctx);
        self.handle(Cmd::Accept, ctx)
    }
    fn hover(&mut self, _zone: &str, _index: usize, _ctx: &Ctx) {}
    /// Jogo em foco (fundo e cor da tela).
    fn focused_game(&self, _ctx: &Ctx) -> Option<String> {
        None
    }
    /// Voltou a ser a página do topo (depois de B numa página empilhada ou depois do jogo).
    fn resumed(&mut self, _ctx: &mut Ctx) {}
    fn focus_game(&mut self, _id: &str, _ctx: &Ctx) {}
}

pub const TOAST_TIME: Duration = Duration::from_millis(2600);

pub struct Shell {
    pub ctx: Ctx,
    pages: Vec<Box<dyn Page>>,
    layers: Vec<Box<dyn Layer>>,
    sidebar: sidebar::Sidebar,
    /// Central rápida aberta (durante um jogo).
    cc: Option<control_center::ControlCenter>,
}

impl Shell {
    pub fn new(ctx: Ctx) -> Shell {
        let mut shell = Shell {
            ctx,
            pages: vec![Box::new(home::HomePage::new())],
            layers: Vec::new(),
            sidebar: sidebar::Sidebar::default(),
            cc: None,
        };
        shell.ctx.dirty = true;
        shell
    }

    pub fn page(&self) -> &dyn Page {
        self.pages
            .last()
            .map(|p| p.as_ref())
            .unwrap_or_else(|| unreachable!("sempre há uma página"))
    }

    pub fn typing(&self) -> bool {
        self.layers.last().is_some_and(|l| l.typing())
    }

    pub fn has_layers(&self) -> bool {
        !self.layers.is_empty()
    }

    pub fn push_layer(&mut self, layer: Box<dyn Layer>) {
        self.layers.push(layer);
        self.ctx.dirty = true;
    }

    pub fn pop_layer(&mut self) {
        self.layers.pop();
        self.ctx.dirty = true;
    }

    pub fn close_layers(&mut self) {
        self.layers.clear();
        self.ctx.dirty = true;
    }

    /// Abre uma página de rota (barra lateral), limpando o histórico.
    pub fn open_route(&mut self, route: &str) {
        let page: Box<dyn Page> = match route {
            "home" => Box::new(home::HomePage::new()),
            "settings" => Box::new(settings::SettingsPage::new()),
            r if r.starts_with("library:") => Box::new(library::LibraryPage::for_route(r)),
            _ => return,
        };
        self.pages = vec![page];
        self.layers.clear();
        self.sidebar.focus = None;
        self.ctx.dirty = true;
    }

    /// Mostra um jogo na tela onde ele está (volta do jogo: foco onde estava).
    pub fn focus_game(&mut self, id: &str) {
        let ctx = &self.ctx;
        if let Some(p) = self.pages.last_mut() {
            p.focus_game(id, ctx);
        }
        self.ctx.dirty = true;
    }

    pub fn open_control_center(&mut self) {
        self.cc = Some(control_center::ControlCenter::new());
        self.ctx.dirty = true;
    }

    pub fn control_center_open(&self) -> bool {
        self.cc.is_some()
    }

    pub fn close_control_center(&mut self) {
        self.cc = None;
        self.ctx.dirty = true;
    }

    pub fn handle(&mut self, cmd: Cmd) {
        self.ctx.dirty = true;
        if let Some(cc) = self.cc.as_mut() {
            if cc.handle(cmd, &mut self.ctx) {
                self.cc = None;
            }
            return;
        }
        if let Some(mut top) = self.layers.pop() {
            let flow = top.handle(cmd, &mut self.ctx);
            self.apply_flow(top, flow);
            return;
        }
        if self.sidebar.focus.is_some() {
            self.handle_sidebar(cmd);
            return;
        }
        if cmd == Cmd::Guide && self.ctx.running.is_some() {
            self.ctx.effect(Effect::ResumeGame);
            return;
        }
        let nav = {
            let Shell { pages, ctx, .. } = self;
            let n = pages.len();
            pages[n - 1].handle(cmd, ctx)
        };
        self.apply_nav(nav);
    }

    pub fn text(&mut self, text: &str) {
        if let Some(top) = self.layers.last_mut() {
            top.text(text, &mut self.ctx);
            self.ctx.dirty = true;
        }
    }

    pub fn erase(&mut self) {
        if let Some(top) = self.layers.last_mut() {
            top.erase(&mut self.ctx);
            self.ctx.dirty = true;
        }
    }

    pub fn click(&mut self, zone: &str, index: usize) {
        self.ctx.dirty = true;
        if let Some(cc) = self.cc.as_mut() {
            if let Some(z) = zone.strip_prefix("cc-")
                && cc.click(z, index, &mut self.ctx)
            {
                self.cc = None;
            }
            return;
        }
        if let Some(rest) = zone.strip_prefix("overlay-") {
            if let Some(mut top) = self.layers.pop() {
                let flow = top.click(rest, index, &mut self.ctx);
                self.apply_flow(top, flow);
            }
            return;
        }
        if !self.layers.is_empty() {
            return;
        }
        if zone == "sidebar" {
            self.sidebar.focus = Some(index);
            self.handle_sidebar(Cmd::Accept);
            return;
        }
        self.sidebar.focus = None;
        let nav = {
            let Shell { pages, ctx, .. } = self;
            let n = pages.len();
            pages[n - 1].click(zone, index, ctx)
        };
        self.apply_nav(nav);
    }

    pub fn hover(&mut self, zone: &str, index: usize) {
        if let Some(rest) = zone.strip_prefix("overlay-") {
            if let Some(top) = self.layers.last_mut() {
                top.hover(rest, index);
            }
        } else if self.layers.is_empty() {
            if zone == "sidebar" {
                if self.sidebar.focus.is_some() {
                    self.sidebar.focus = Some(index);
                }
            } else if self.sidebar.focus.is_none() {
                let Shell { pages, ctx, .. } = self;
                let n = pages.len();
                pages[n - 1].hover(zone, index, ctx);
            }
        }
        self.ctx.dirty = true;
    }

    /// Clique numa dica do rodapé: o mesmo que o comando dela.
    pub fn hint_clicked(&mut self, index: usize) {
        let hints = self.hints();
        let Some(h) = hints.get(index) else { return };
        let all = [
            Cmd::Accept,
            Cmd::Back,
            Cmd::Favorite,
            Cmd::Search,
            Cmd::PrevSection,
            Cmd::NextSection,
            Cmd::Menu,
            Cmd::Guide,
        ];
        if let Some(cmd) = all.into_iter().find(|c| self.ctx.glyph(*c) == h.glyph) {
            self.handle(cmd);
        }
    }

    fn apply_flow(&mut self, top: Box<dyn Layer>, flow: Flow) {
        self.apply_flow_inner(top, flow);
        self.after_action();
    }

    /// Pedidos que camadas fazem às páginas (abrir os detalhes, sair dos detalhes de um jogo removido).
    fn after_action(&mut self) {
        if let Some(l) = self.ctx.pending_layer.take() {
            self.layers.push(l);
        }
        if let Some(id) = self.ctx.open_details.take() {
            self.layers.clear();
            self.pages.push(Box::new(details::DetailsPage::new(&id)));
        }
        if std::mem::take(&mut self.ctx.leave_details) && self.page().kind() == "details" {
            self.layers.clear();
            self.apply_nav_inner(Nav::Back);
        }
    }

    fn apply_flow_inner(&mut self, top: Box<dyn Layer>, flow: Flow) {
        match flow {
            Flow::Stay => self.layers.push(top),
            Flow::Close => {}
            Flow::CloseAll => self.layers.clear(),
            Flow::Replace(l) => self.layers.push(l),
            Flow::Push(l) => {
                self.layers.push(top);
                self.layers.push(l);
            }
        }
    }

    fn apply_nav(&mut self, nav: Nav) {
        self.apply_nav_inner(nav);
        self.after_action();
    }

    fn apply_nav_inner(&mut self, nav: Nav) {
        match nav {
            Nav::Stay => {}
            Nav::Push(p) => self.pages.push(p),
            Nav::Replace(p) => {
                self.pages.pop();
                self.pages.push(p);
            }
            Nav::Back => {
                if self.pages.len() > 1 {
                    self.pages.pop();
                    let Shell { pages, ctx, .. } = self;
                    let n = pages.len();
                    pages[n - 1].resumed(ctx);
                } else {
                    self.sidebar.focus =
                        Some(self.sidebar.index_of(&self.ctx, &self.page().route()));
                }
            }
            Nav::Layer(l) => self.layers.push(l),
            Nav::Sidebar => {
                if self.pages.len() == 1 {
                    self.sidebar.focus =
                        Some(self.sidebar.index_of(&self.ctx, &self.page().route()));
                }
            }
        }
    }

    fn handle_sidebar(&mut self, cmd: Cmd) {
        let items = sidebar::items(&self.ctx);
        let Some(cur) = self.sidebar.focus else {
            return;
        };
        let step = |from: usize, d: i32| -> usize {
            let mut i = from as i32;
            loop {
                i += d;
                if i < 0 || i >= items.len() as i32 {
                    return from;
                }
                if items[i as usize].id != "spacer" {
                    return i as usize;
                }
            }
        };
        match cmd {
            Cmd::Up => self.sidebar.focus = Some(step(cur, -1)),
            Cmd::Down => self.sidebar.focus = Some(step(cur, 1)),
            Cmd::Right | Cmd::Back => {
                if self.page().route() == items.get(cur).map(|i| i.id.as_str()).unwrap_or_default()
                    || cmd == Cmd::Right
                {
                    self.sidebar.focus = None;
                } else {
                    self.sidebar.focus =
                        Some(self.sidebar.index_of(&self.ctx, &self.page().route()));
                }
            }
            Cmd::Accept => {
                let Some(item) = items.get(cur) else { return };
                match item.id.as_str() {
                    "exit" => self.layers.push(Box::new(dialogs::confirm_exit(&self.ctx))),
                    id if id == self.page().route() && self.pages.len() == 1 => {
                        self.sidebar.focus = None
                    }
                    id => {
                        let id = id.to_owned();
                        self.open_route(&id);
                    }
                }
            }
            Cmd::PrevSection | Cmd::NextSection => self.cycle_section(cmd == Cmd::NextSection),
            _ => {}
        }
    }

    /// LB/RB: Início → Todos → Instalados → Favoritos → Configurações.
    fn cycle_section(&mut self, forward: bool) {
        const ORDER: [&str; 5] = [
            "home",
            "library:all",
            "library:installed",
            "library:favorites",
            "settings",
        ];
        let route = self.page().route();
        let i = ORDER.iter().position(|r| *r == route).unwrap_or(0) as i32;
        let n = ORDER.len() as i32;
        let next = ORDER[((i + if forward { 1 } else { -1 }) % n + n) as usize % n as usize];
        let in_sidebar = self.sidebar.focus.is_some();
        self.open_route(next);
        if in_sidebar {
            self.sidebar.focus = Some(self.sidebar.index_of(&self.ctx, next));
        }
    }

    /// Seções com LB/RB valem em qualquer página de rota (a de Configurações usa para categorias).
    pub fn section(&mut self, forward: bool) {
        if self.layers.is_empty() && self.cc.is_none() && self.page().kind() != "settings" {
            self.cycle_section(forward);
        } else {
            self.handle(if forward {
                Cmd::NextSection
            } else {
                Cmd::PrevSection
            });
        }
    }

    pub fn hints(&self) -> Vec<Hint> {
        if let Some(cc) = &self.cc {
            return cc.hints(&self.ctx);
        }
        if let Some(l) = self.layers.last() {
            return l.hints(&self.ctx);
        }
        if self.sidebar.focus.is_some() {
            return vec![
                self.ctx.hint(Cmd::Accept, &self.ctx.t("hint.select")),
                self.ctx.hint(Cmd::PrevSection, &self.ctx.t("hint.section")),
            ];
        }
        let mut h = self.page().hints(&self.ctx);
        if self.ctx.running.is_some() {
            h.push(self.ctx.hint(Cmd::Guide, &self.ctx.t("hint.back_to_game")));
        }
        h
    }

    /// Calcula todas as propriedades do tema para este instante.
    pub fn render(&mut self, now: Instant) -> Out {
        let mut out = Out::default();
        if let Some((_, at)) = &self.ctx.toast
            && now.duration_since(*at) >= TOAST_TIME
        {
            self.ctx.toast = None;
        }
        let kind = self.page().kind();
        out.set("page", kind);
        let sidebar_items = sidebar::items(&self.ctx);
        let route = self.page().route();
        let items: Vec<_> = sidebar_items
            .iter()
            .map(|i| crate::model::NavItem {
                selected: i.id == route,
                ..i.clone()
            })
            .collect();
        out.set("sidebar", items);
        out.set(
            "sidebar-focus",
            self.sidebar.focus.map(|i| i as i32).unwrap_or(-1),
        );
        out.set(
            "sidebar-visible",
            self.pages.len() == 1 && kind != "details",
        );
        {
            let Shell { pages, ctx, .. } = self;
            let n = pages.len();
            pages[n - 1].render(ctx, &mut out);
        }
        match &self.cc {
            Some(cc) => cc.render(&self.ctx, &mut out),
            None => out.set("control-center", false),
        }
        let overlays: Vec<_> = self.layers.iter().map(|l| l.render(&self.ctx)).collect();
        out.set("overlays", overlays);
        out.set("hints", self.hints());
        out.set(
            "toast",
            self.ctx
                .toast
                .as_ref()
                .map(|(t, _)| t.clone())
                .unwrap_or_default(),
        );
        out.set(
            "input-icon",
            if self.ctx.input == InputKind::Keyboard {
                "keyboard"
            } else {
                "gamepad"
            },
        );
        out.set(
            "input-label",
            if self.ctx.input == InputKind::Keyboard {
                self.ctx.t("status.keyboard")
            } else {
                self.ctx.pad_name.clone()
            },
        );
        out.set("see-all-label", self.ctx.t("home.see_all"));
        out
    }

    /// Jogo em foco, para a cor e o fundo da tela.
    pub fn focused_game(&self) -> Option<String> {
        if self.sidebar.focus.is_some() && self.page().kind() == "library" {
            return None;
        }
        self.page().focused_game(&self.ctx)
    }

    /// Para a próxima volta do jogo: a página atual se atualiza (fileiras, "Continuar jogando").
    pub fn resumed(&mut self) {
        let Shell { pages, ctx, .. } = self;
        let n = pages.len();
        pages[n - 1].resumed(ctx);
        ctx.dirty = true;
    }
}
