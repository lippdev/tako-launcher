//! Estado da interface (o `ShellViewModel` do WPF): páginas com histórico, barra lateral, camadas, dicas e
//! avisos. Puro Rust e testável sem janela: `render` produz as propriedades do tema, o `bridge` as aplica.

pub mod add_game;
pub mod art;
pub(crate) mod art_review;
pub mod closing_game;
pub mod control_center;
pub mod ctx;
pub mod desktop;
pub mod details;
pub mod dialogs;
pub mod edit_game;
pub mod first_run;
pub mod game_settings;
pub mod home;
pub mod keyboard;
pub mod keys;
pub mod launch_splash;
pub mod layer;
pub mod library;
mod links;
pub mod performance;
pub mod profiles;
pub mod settings;
pub mod sidebar;
pub mod smart;
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
    /// Tecla crua antes de virar comando (modo desktop: Tab entre as zonas); `Some` = tratou.
    fn key(
        &mut self,
        _text: &str,
        _mods: tako_core::input::shortcut::KeyModifiers,
        _ctx: &mut Ctx,
    ) -> Option<Nav> {
        None
    }
    /// Campo de texto do modo desktop editado (a busca da biblioteca).
    fn edited(&mut self, _zone: &str, _text: &str, _ctx: &mut Ctx) {}
    /// A página é a biblioteca desktop (testes e o tema leem o estado dela).
    fn desktop(&self) -> Option<&desktop::DesktopLibrary> {
        None
    }
}

pub const TOAST_TIME: Duration = Duration::from_millis(2600);

pub struct Shell {
    pub(crate) pending_link: Option<tako_core::links::Link>,
    pub ctx: Ctx,
    pages: Vec<Box<dyn Page>>,
    layers: Vec<Box<dyn Layer>>,
    sidebar: sidebar::Sidebar,
    /// Central rápida aberta (durante um jogo).
    cc: Option<control_center::ControlCenter>,
    /// Modo desktop aberto sem argumento: perguntar pela tela cheia se um controle aparecer logo (ADR-0027).
    pub ask_big_picture: bool,
}

impl Shell {
    pub fn new(ctx: Ctx) -> Shell {
        let mut shell = Shell {
            pending_link: None,
            ctx,
            pages: vec![Box::new(home::HomePage::new())],
            layers: Vec::new(),
            sidebar: sidebar::Sidebar::default(),
            cc: None,
            ask_big_picture: false,
        };
        shell.ctx.dirty = true;
        shell
    }

    /// Passa a apresentar no modo desktop (ADR-0027): a biblioteca desktop, com o jogo em foco selecionado.
    pub fn enter_desktop(&mut self) {
        let focus = self.focused_game();
        self.ctx.desktop = true;
        self.cc = None;
        self.layers.clear();
        self.sidebar.focus = None;
        self.ctx.pointer = false;
        let mut page = desktop::DesktopLibrary::new();
        if let Some(id) = focus {
            page.focus_game(&id, &self.ctx);
        }
        self.pages = vec![Box::new(page)];
        self.ctx.dirty = true;
    }

    /// Passa a apresentar na tela cheia: a Início, com o jogo selecionado no desktop em foco.
    pub fn enter_big_picture(&mut self) {
        let focus = self.focused_game();
        self.ctx.desktop = false;
        self.ask_big_picture = false;
        self.layers.clear();
        self.pages = vec![Box::new(home::HomePage::new())];
        self.sidebar.focus = None;
        if let Some(id) = focus {
            self.focus_game(&id);
        }
        self.ctx.dirty = true;
    }

    /// A biblioteca do modo desktop (sempre a página de baixo no desktop).
    pub fn desktop_library(&self) -> Option<&desktop::DesktopLibrary> {
        if !self.ctx.desktop {
            return None;
        }
        self.pages.first()?.desktop()
    }

    /// Barra superior do desktop: Biblioteca, Configurações e Tela cheia.
    fn desktop_nav(&mut self, index: usize) {
        self.layers.clear();
        match index {
            0 => self.pages.truncate(1),
            1 if self.page().kind() != "settings" => {
                self.pages.truncate(1);
                self.pages.push(Box::new(settings::SettingsPage::new()));
            }
            2 => self
                .ctx
                .effect(Effect::SwitchMode(crate::mode::UiMode::BigPicture)),
            _ => {}
        }
        self.ctx.dirty = true;
    }

    /// Campo de texto do desktop editado: a camada de cima (`overlay-…`) ou a página.
    pub fn edited(&mut self, zone: &str, text: &str) {
        self.ctx.dirty = true;
        if zone.starts_with("overlay-") {
            if let Some(top) = self.layers.last_mut() {
                top.replace_text(text, &mut self.ctx);
            }
            return;
        }
        let Shell { pages, ctx, .. } = self;
        let n = pages.len();
        pages[n - 1].edited(zone, text, ctx);
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

    /// A camada de cima mostra valores que mudam sozinhos (`Layer::live`): o app redesenha a cada segundo.
    pub fn live(&self) -> bool {
        self.layers.last().is_some_and(|l| l.live())
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
        // No desktop as rotas são as da barra de cima (links e atalhos da tela cheia caem aqui).
        if self.ctx.desktop {
            self.desktop_nav(usize::from(route == "settings"));
            return;
        }
        let page: Box<dyn Page> = match route {
            "home" => Box::new(home::HomePage::new()),
            "settings" => Box::new(settings::SettingsPage::new()),
            r if r.starts_with("library:") => Box::new(library::LibraryPage::open(r, &self.ctx)),
            _ => return,
        };
        self.pages = vec![page];
        self.layers.clear();
        self.sidebar.focus = None;
        self.ctx.dirty = true;
    }

    /// Mostra um jogo na tela onde ele está (volta do jogo: foco onde estava).
    pub fn focus_game(&mut self, id: &str) {
        self.ctx.pointer = false;
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

    /// A Central por cima do jogo (interface aberta pelo atalho durante o jogo).
    pub fn open_control_center_over_game(&mut self) {
        self.cc = Some(control_center::ControlCenter::over_game());
        self.ctx.dirty = true;
    }

    pub fn control_center_open(&self) -> bool {
        self.cc.is_some()
    }

    pub fn close_control_center(&mut self) {
        self.cc = None;
        self.ctx.dirty = true;
    }

    /// Encerrar jogo: "Fechando o jogo…" até o acompanhamento confirmar o fim (sem a Central por cima).
    pub fn begin_closing_game(&mut self) {
        self.cc = None;
        self.layers.retain(|l| !l.ends_with_game());
        let layer = closing_game::ClosingGame::new(&mut self.ctx);
        self.layers.push(Box::new(layer));
        self.ctx.dirty = true;
    }

    /// Jogar: "Abrindo…" com a arte do jogo até a janela dele aparecer.
    pub fn begin_launch_splash(&mut self) {
        self.cc = None;
        self.layers.retain(|l| !l.ends_with_game());
        let layer = launch_splash::LaunchSplash::new(&mut self.ctx);
        self.layers.push(Box::new(layer));
        self.ctx.dirty = true;
    }

    /// O jogo terminou: fecha as camadas que dependiam dele.
    pub fn close_game_layers(&mut self) {
        self.ctx.launching = false;
        self.layers.retain(|l| !l.ends_with_game());
        self.ctx.dirty = true;
    }

    pub fn handle(&mut self, cmd: Cmd) {
        self.ctx.dirty = true;
        self.ctx.pointer = false;
        self.ctx.focus_visible = true;
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

    /// Tecla crua para a camada de cima; `false` = segue o mapa de comandos.
    pub fn key(&mut self, text: &str, mods: tako_core::input::shortcut::KeyModifiers) -> bool {
        if self.cc.is_some() {
            return false;
        }
        // F11 no desktop: tela cheia (ADR-0027).
        if self.ctx.desktop
            && text == keys::F11
            && mods == tako_core::input::shortcut::KeyModifiers::NONE
        {
            self.ctx
                .effect(Effect::SwitchMode(crate::mode::UiMode::BigPicture));
            return true;
        }
        let Some(mut top) = self.layers.pop() else {
            if !self.ctx.desktop {
                return false;
            }
            let nav = {
                let Shell { pages, ctx, .. } = self;
                let n = pages.len();
                pages[n - 1].key(text, mods, ctx)
            };
            return match nav {
                Some(nav) => {
                    self.ctx.dirty = true;
                    self.apply_nav(nav);
                    true
                }
                None => false,
            };
        };
        match top.key(text, mods, &mut self.ctx) {
            Some(flow) => {
                self.ctx.dirty = true;
                self.apply_flow(top, flow);
                true
            }
            None => {
                self.layers.push(top);
                false
            }
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
        self.ctx.pointer = false;
        self.ctx.focus_visible = false;
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
        if zone == "desk-nav" {
            self.desktop_nav(index);
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
            } else if self.sidebar.focus.is_none() && !self.ctx.desktop {
                // No desktop o mouse por cima só destaca (no tema); a seleção é do clique.
                let Shell { pages, ctx, .. } = self;
                let n = pages.len();
                pages[n - 1].hover(zone, index, ctx);
            }
        }
        // O foco segue o mouse, mas a fileira/página não rola por isso (ver `Ui.pointer` no tema).
        self.ctx.pointer = true;
        self.ctx.focus_visible = false;
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

    /// Pedidos que camadas, efeitos e tarefas em segundo plano fazem às páginas (abrir um diálogo, abrir os
    /// detalhes, sair dos detalhes de um jogo removido).
    pub(crate) fn after_action(&mut self) {
        if let Some(l) = self.ctx.pending_layer.take() {
            self.layers.push(l);
        }
        if let Some(id) = self.ctx.open_details.take() {
            self.layers.clear();
            if self.ctx.desktop {
                // O desktop mostra os detalhes no destaque da biblioteca.
                self.pages.truncate(1);
                self.focus_game(&id);
            } else {
                self.pages.push(Box::new(details::DetailsPage::new(&id)));
            }
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
                } else if !self.ctx.desktop {
                    self.sidebar.focus =
                        Some(self.sidebar.index_of(&self.ctx, &self.page().route()));
                }
            }
            Nav::Layer(l) => self.layers.push(l),
            Nav::Sidebar => {
                if self.pages.len() == 1 && !self.ctx.desktop {
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
        // No desktop as seções são as abas da biblioteca e as categorias das Configurações (a página trata).
        if self.layers.is_empty()
            && self.cc.is_none()
            && !self.ctx.desktop
            && self.page().kind() != "settings"
        {
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
        self.ctx.check_broken();
        if self.ctx.toast_expired(now) {
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
            None => {
                out.set("control-center", false);
                out.set("transparent-background", false);
            }
        }
        // A janela do jogo apareceu: tudo some aos poucos e o jogo aparece pela janela transparente.
        out.set("leaving", self.ctx.leaving);
        if self.ctx.leaving {
            out.set("transparent-background", true);
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
        out.set(
            "pad-battery",
            match self.ctx.pad_battery {
                Some(p) if self.ctx.input == InputKind::Gamepad => format!("{p}%"),
                _ => String::new(),
            },
        );
        out.set("see-all-label", self.ctx.t("home.see_all"));
        if self.ctx.desktop {
            self.render_desktop_bar(&mut out);
        }
        out
    }

    /// Barra de cima do desktop: Biblioteca, Configurações e o botão da tela cheia.
    fn render_desktop_bar(&self, out: &mut Out) {
        let kind = self.page().kind();
        let item = |id: &str, label: &str, icon: &str, selected: bool| crate::model::NavItem {
            id: id.into(),
            label: self.ctx.t(label),
            icon: icon.into(),
            selected,
            ..Default::default()
        };
        out.set(
            "desk-nav",
            vec![
                item("library", "desktop.library", "grid", kind == "desk-library"),
                item("settings", "desktop.settings", "gear", kind == "settings"),
            ],
        );
        out.set("desk-big-picture-label", self.ctx.t("desktop.big_picture"));
        out.set("desk-focus-visible", self.ctx.focus_visible);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_focuses_without_reveal_and_commands_reveal_again() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = crate::app::build_ctx(
            tako_core::paths::DataPaths::at(dir.path()),
            tako_platform::Platform::simulated(),
            true,
        );
        let mut shell = Shell::new(ctx);
        // Mouse sobre a última capa visível: o tema destaca, mas a fileira não rola.
        shell.hover("row-0", 5);
        assert!(shell.ctx.pointer);
        // Direita/Baixo pelo comando voltam a revelar o item focado.
        shell.handle(Cmd::Right);
        assert!(!shell.ctx.pointer);
        shell.hover("row-0", 5);
        shell.handle(Cmd::Down);
        assert!(!shell.ctx.pointer);
        // Clique numa capa parcialmente visível pode trazê-la para a tela.
        shell.hover("row-0", 5);
        shell.click("row-0", 5);
        assert!(!shell.ctx.pointer);
    }
}
