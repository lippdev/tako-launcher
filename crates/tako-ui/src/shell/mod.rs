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
pub mod profile;
pub mod profiles;
pub mod settings;
pub mod sidebar;
pub mod smart;
pub mod source_ids;
pub mod tags_editor;
pub mod theme_preview;
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
    fn desktop_mut(&mut self) -> Option<&mut desktop::DesktopLibrary> {
        None
    }
}

/// Abrindo o jogo há mais que isso: "Ainda abrindo…" e "Cancelar" (JogarEAbrir).
pub const LAUNCH_SLOW: Duration = Duration::from_secs(20);
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
        // A prévia é da tela cheia: sem a camada dela, o tick pediria o tema de volta e avisaria no desktop.
        self.ctx.theme_preview = None;
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

    /// Barra superior do desktop: Biblioteca, Configurações, Tela cheia e Perfil (3).
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
            3 if self.page().kind() != profile::KIND => {
                self.pages.truncate(1);
                self.pages.push(Box::new(profile::ProfilePage::new()));
            }
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

    /// Prévia de tema vencida, ou sem a camada dela (fechada por outro caminho): volta ao tema que estava valendo.
    pub fn tick_theme_preview(&mut self, now: Instant) {
        let due = self
            .ctx
            .theme_preview
            .as_ref()
            .is_some_and(|p| now >= p.until || !self.layers.iter().any(|l| l.theme_preview()));
        if due {
            self.end_theme_preview(true);
        }
    }

    /// O tema da prévia não carregou: a tela continua com o anterior e a pessoa é avisada.
    pub fn fail_theme_preview(&mut self) {
        if self.end_theme_preview(false) {
            let text = self.ctx.t("settings.appearance.preview_failed");
            self.ctx.toast(text);
        }
    }

    /// Encerra a prévia sem confirmar e devolve a lista de temas; `reload` = a tela já trocou e precisa voltar.
    /// Devolve `false` se não havia prévia.
    fn end_theme_preview(&mut self, reload: bool) -> bool {
        let Some(p) = self.ctx.theme_preview.take() else {
            return false;
        };
        let shown = self.layers.iter().any(|l| l.theme_preview());
        self.layers.retain(|l| !l.theme_preview());
        if shown {
            self.layers
                .push(Box::new(theme_preview::panel(&self.ctx, p.id.as_deref())));
        }
        if reload {
            theme_preview::reload_selected(&mut self.ctx);
        }
        self.ctx.dirty = true;
        true
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
            if let Some(key) = route.strip_prefix(desktop::COLLECTION_ROUTE) {
                self.desktop_nav(0);
                let Shell { pages, ctx, .. } = self;
                if let Some(lib) = pages[0].desktop_mut() {
                    lib.open_collection(key, ctx);
                }
                return;
            }
            if route == "desk:profile:sessions" {
                self.layers.clear();
                self.pages.truncate(1);
                let mut p = profile::ProfilePage::new();
                p.open_tab(profile::Tab::Sessions);
                self.pages.push(Box::new(p));
                self.ctx.dirty = true;
                return;
            }
            self.desktop_nav(match route {
                "settings" => 1,
                profile::ROUTE => 3,
                _ => 0,
            });
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
        self.cc = None;
        self.ctx.launching = false;
        self.ctx.leaving = false;
        self.ctx.launch_on_top_until = None;
        self.layers.retain(|l| !l.ends_with_game());
        self.ctx.dirty = true;
    }

    /// Quanto a entrada do app fica na tela se ninguém pular (BRAND-002; o PRD §8.1 pede a 1ª tela interativa em
    /// ≤ 1,5 s, então o total com a saída fica perto de 1 s).
    pub const INTRO_HOLD: Duration = Duration::from_millis(800);

    /// Entrada do app (BRAND-002): só numa abertura comum em tela cheia e sem "Reduzir animações".
    pub fn wants_intro(&self, big_picture: bool, plain_start: bool) -> bool {
        big_picture
            && plain_start
            && self.layers.is_empty()
            && !self.ctx.settings.bool(ctx::keys::REDUCED_MOTION, false)
    }

    /// Acaba a entrada; `true` se ela estava na tela. A entrada do usuário que pula não faz mais nada.
    pub fn end_intro(&mut self) -> bool {
        let was = std::mem::take(&mut self.ctx.intro);
        self.ctx.dirty |= was;
        was
    }

    pub fn handle(&mut self, cmd: Cmd) {
        if self.end_intro() {
            return;
        }
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
        if self.end_intro() {
            return;
        }
        if let Some(top) = self.layers.last_mut() {
            top.text(text, &mut self.ctx);
            self.ctx.dirty = true;
        }
    }

    /// Tecla crua para a camada de cima; `false` = segue o mapa de comandos.
    pub fn key(&mut self, text: &str, mods: tako_core::input::shortcut::KeyModifiers) -> bool {
        if self.end_intro() {
            return true;
        }
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
        if self.end_intro() {
            return;
        }
        if let Some(top) = self.layers.last_mut() {
            top.erase(&mut self.ctx);
            self.ctx.dirty = true;
        }
    }

    pub fn click(&mut self, zone: &str, index: usize) {
        if self.end_intro() {
            return;
        }
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
        if zone == "toast-undo" {
            if let Some(undo) = self.ctx.toast_undo.take() {
                undo(&mut self.ctx);
                self.ctx.toast = None;
                self.ctx.dirty = true;
            }
            return;
        }
        if self.ctx.desktop && zone == "desk-profile-menu" {
            self.layers.push(Box::new(desktop::profile_menu()));
            self.ctx.dirty = true;
            return;
        }
        if zone == "desk-brand" {
            self.layers.push(Box::new(desktop::brand_menu()));
            self.ctx.dirty = true;
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
        if zone == "toast" {
            self.ctx.toast_hovered(index.min(3), Instant::now());
            self.ctx.dirty = true;
            return;
        }
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
        if std::mem::take(&mut self.ctx.app_update.close_menu) {
            self.layers
                .retain(|l| l.render(&self.ctx).overline != crate::app_updates::BRAND_MENU);
        }
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
        if let Some(route) = self.ctx.open_route.take() {
            self.open_route(&route);
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
        if self.end_intro() {
            return;
        }
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
            self.ctx.toast_undo = None;
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
        out.set("intro", self.ctx.intro);
        if self.ctx.leaving {
            out.set("transparent-background", true);
        }
        let overlays: Vec<_> = self.layers.iter().map(|l| l.render(&self.ctx)).collect();
        let update_menu_open = overlays
            .iter()
            .any(|o| o.overline == crate::app_updates::BRAND_MENU);
        if self.ctx.app_update.menu_open
            && !update_menu_open
            && self.ctx.app_update.check == crate::app_updates::Check::Failed
        {
            self.ctx.app_update.check = crate::app_updates::Check::Idle;
        }
        self.ctx.app_update.menu_open = update_menu_open;
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

    /// Barra de cima do desktop: as guias das áreas (Biblioteca e Perfil; Loja quando existir), a busca e o botão
    /// da tela cheia. Configurações fica no menu da marca.
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
                item("profile", "profile.tab", "user", kind == profile::KIND),
            ],
        );
        out.set("desk-big-picture-label", self.ctx.t("desktop.big_picture"));
        out.set(
            "desk-window-labels",
            ["minimize", "maximize", "restore", "close"]
                .map(|k| self.ctx.t(&format!("desktop.window.{k}")))
                .to_vec(),
        );
        out.set(
            "desk-update-label",
            if self.ctx.update_ready.is_some() {
                self.ctx.t("desktop.window.update_ready")
            } else {
                String::new()
            },
        );
        let name = profiles::active_name(&self.ctx);
        let initial = name
            .chars()
            .next()
            .map(|c| c.to_uppercase().collect::<String>())
            .unwrap_or_default();
        out.set("desk-profile-initial", initial);
        out.set("desk-profile-name", name);
        out.set("desk-brand-label", self.ctx.t("desktop.menu.brand"));
        out.set("desk-search-key", self.ctx.t("desktop.search_key"));
        out.set("desk-search-esc", self.ctx.t("desktop.search_esc"));
        out.set(
            "desk-settings-search-hint",
            self.ctx.t("desktop.settings_search"),
        );
        out.set("desk-cancel-label", self.ctx.t("common.cancel"));
        out.set(
            "desk-toast-undo",
            if !self.ctx.app_update.toast_label.is_empty() {
                self.ctx.app_update.toast_label.clone()
            } else if self.ctx.toast_undo.is_some() {
                self.ctx.t("desktop.undo")
            } else {
                String::new()
            },
        );
        out.set("desk-toast-mascot", self.ctx.toast_mascot);
        out.set("desk-ok-label", self.ctx.t("common.ok"));
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

    #[test]
    fn hovered_toast_does_not_expire_and_restarts_on_leave() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = crate::app::build_ctx(
            tako_core::paths::DataPaths::at(dir.path()),
            tako_platform::Platform::simulated(),
            true,
        );
        let mut shell = Shell::new(ctx);
        shell.ctx.toast("oi");
        let late = Instant::now() + Duration::from_secs(60);
        assert!(shell.ctx.toast_expired(late));
        shell.hover("toast", 1);
        assert!(!shell.ctx.toast_expired(late));
        // Sair da pílula reinicia a contagem.
        shell.hover("toast", 0);
        assert!(!shell.ctx.toast_expired(Instant::now()));
        assert!(shell.ctx.toast_expired(late));
    }

    #[test]
    fn only_the_welcome_back_toast_carries_the_mascot() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = crate::app::build_ctx(
            tako_core::paths::DataPaths::at(dir.path()),
            tako_platform::Platform::simulated(),
            true,
        );
        let mut shell = Shell::new(ctx);
        shell.enter_desktop();
        let mascot = |s: &mut Shell| {
            s.render(Instant::now()).props.iter().any(|(n, v)| {
                *n == "desk-toast-mascot" && matches!(v, slint_interpreter::Value::Bool(true))
            })
        };
        shell.ctx.toast_welcome_back("Bem-vindo de volta");
        assert!(mascot(&mut shell));
        // Um aviso comum depois dele volta ao ✓.
        shell.ctx.toast("Cupom copiado");
        assert!(!mascot(&mut shell));
    }

    fn intro_shell(dir: &std::path::Path) -> Shell {
        let ctx = crate::app::build_ctx(
            tako_core::paths::DataPaths::at(dir),
            tako_platform::Platform::simulated(),
            true,
        );
        Shell::new(ctx)
    }

    #[test]
    fn intro_is_off_with_reduced_motion_and_outside_a_plain_big_picture_start() {
        let dir = tempfile::tempdir().unwrap();
        let mut shell = intro_shell(dir.path());
        assert!(shell.wants_intro(true, true));
        // Desktop, abertura vinda de um jogo ou link: sem entrada.
        assert!(!shell.wants_intro(false, true));
        assert!(!shell.wants_intro(true, false));
        // "Reduzir animações": a entrada nem aparece, então a primeira tela não espera nada.
        shell.ctx.settings.set_bool(ctx::keys::REDUCED_MOTION, true);
        assert!(!shell.wants_intro(true, true));
    }

    #[test]
    fn any_input_skips_the_intro_at_once_without_acting() {
        let dir = tempfile::tempdir().unwrap();
        let mut shell = intro_shell(dir.path());
        shell.ctx.library.refresh();
        let none = tako_core::input::shortcut::KeyModifiers::NONE;
        let first = shell.focused_game();
        for step in 0..4 {
            shell.ctx.intro = true;
            assert!(matches!(
                shell
                    .render(Instant::now())
                    .props
                    .iter()
                    .find(|(n, _)| *n == "intro"),
                Some((_, Value::Bool(true)))
            ));
            match step {
                0 => shell.handle(Cmd::Right),
                1 => assert!(shell.key("a", none)),
                2 => shell.click("row-0", 1),
                _ => shell.section(true),
            }
            // Pulou na hora: some do próximo desenho e a entrada do usuário não navegou.
            assert!(!shell.ctx.intro);
            assert!(shell.ctx.dirty);
            assert!(matches!(
                shell
                    .render(Instant::now())
                    .props
                    .iter()
                    .find(|(n, _)| *n == "intro"),
                Some((_, Value::Bool(false)))
            ));
            assert_eq!(shell.focused_game(), first);
        }
        assert!(!shell.end_intro());
    }
}
