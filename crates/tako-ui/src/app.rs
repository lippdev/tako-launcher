//! Laço da interface: carrega o tema, monta o contexto, liga entrada (controle e teclado) e desenha quando
//! algo muda. As tarefas longas (ler as lojas, metadados) rodam em threads e voltam pelo laço do Slint.

use crate::gamepad::{PadEvent, PadState, Pads};
use crate::shell::keys::{self, KeyAction};
use crate::shell::{Cmd, Ctx, Effect, InputKind, Settings, Shell};
use crate::{bridge, images, theme, values::ToValue};
use slint::ComponentHandle;
use slint_interpreter::{ComponentDefinition, ComponentInstance, Value};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tako_core::i18n::{self, Lang};
use tako_core::input::shortcut::{ButtonChordDetector, chord};
use tako_core::input::{AppCommand, GamepadButtons, GamepadInterpreter, GamepadSnapshot};
use tako_core::library::{CollectionLibrary, Game, GameLibrary, system_clock};
use tako_core::paths::DataPaths;
use tako_core::persistence::{self as db, TakoDatabase};
use tako_core::presentation::color::AccentPalette;
use tako_platform::Platform;

/// Como a interface foi aberta.
#[derive(Debug, Clone, Default)]
pub struct UiOptions {
    /// Sem lojas nem funções do SO (testes, demonstração e CI).
    pub simulate: bool,
    /// Voltou de um jogo: foco nele e "Bem-vindo de volta".
    pub after_game: Option<(String, Duration)>,
    /// Um jogo continua rodando (aberta pela Central ou "Início" durante o jogo): id e título.
    pub running: Option<(String, String)>,
    /// Abrir direto na Central rápida.
    pub control_center: bool,
    pub safe_mode: bool,
}

/// O que fazer com a janela depois de um pedido.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostAction {
    Stay,
    /// Fecha a interface (o supervisor segue com o jogo, ou o app sai).
    Close,
    /// Esconde a janela e segue rodando (jogo acompanhado aqui mesmo, sem supervisor).
    Hide,
    /// Mostra a janela de novo.
    Show,
    /// Recarrega o tema (troca em tempo de execução).
    ReloadTheme,
}

/// O que o app (binário) faz com os pedidos da interface que saem dela: abrir jogos, sair, atualizar.
pub trait Host {
    fn handle(&mut self, effect: &Effect, ctx: &mut Ctx) -> HostAction;
    /// A cada 16 ms com o estado do controle (sessão sem supervisor, heartbeat).
    fn tick(&mut self, _ctx: &mut Ctx, _pad: &PadState, _now: Duration) -> HostAction {
        HostAction::Stay
    }
}

#[derive(Clone)]
pub struct App {
    pub ui: Rc<RefCell<ComponentInstance>>,
    pub shell: Rc<RefCell<Shell>>,
    host: Rc<RefCell<dyn Host>>,
    builtin: PathBuf,
}

fn cmd_of(c: AppCommand) -> Cmd {
    match c {
        AppCommand::Up => Cmd::Up,
        AppCommand::Down => Cmd::Down,
        AppCommand::Left => Cmd::Left,
        AppCommand::Right => Cmd::Right,
        AppCommand::Accept => Cmd::Accept,
        AppCommand::Back => Cmd::Back,
        AppCommand::Favorite => Cmd::Favorite,
        AppCommand::Search => Cmd::Search,
        AppCommand::PreviousSection => Cmd::PrevSection,
        AppCommand::NextSection => Cmd::NextSection,
        AppCommand::Menu => Cmd::Menu,
        AppCommand::Guide => Cmd::Guide,
    }
}

/// Monta o contexto: banco, biblioteca (estado salvo), coleções e plataforma. As lojas são lidas depois,
/// em segundo plano, para a primeira tela aparecer logo.
pub fn build_ctx(paths: DataPaths, platform: Platform, simulate: bool) -> Ctx {
    let _ = std::fs::create_dir_all(&paths.root);
    let database = (!simulate).then(|| TakoDatabase::new(paths.database()));
    if database.is_some() {
        // Chaves das alphas C# (DPAPI ou texto) vão para o cofre do SO uma vez; antes de ler as configurações.
        #[cfg(windows)]
        let decrypt = tako_platform::windows::dpapi::unprotect;
        #[cfg(not(windows))]
        let decrypt = |_: &[u8]| None;
        if let Err(e) = tako_core::persistence::alpha_migration::migrate_secrets(
            &paths.root,
            decrypt,
            &crate::shell::settings::metadata::Vault(platform.secrets.as_ref()),
        ) {
            tracing::warn!("segredos das alphas: {e}");
        }
    }
    let settings = Settings::new(database.clone().map(db::SettingsStore::new));
    let lang = settings
        .get(crate::shell::ctx::keys::LANGUAGE)
        .and_then(Lang::from_code)
        .unwrap_or_else(Lang::from_env);
    // Nos testes o idioma é global e quem precisa dele o define (sob `LANG_LOCK`), não o ambiente da máquina.
    if !cfg!(test) {
        i18n::set_lang(lang);
    }

    let store: Box<dyn tako_core::library::LibraryStore + Send> = match &database {
        Some(d) => Box::new(db::LibraryStore::new(
            d.clone(),
            Some(paths.root.join("library.json")),
        )),
        None => Box::new(tako_core::library::MemoryLibraryStore::default()),
    };
    let found: Arc<Mutex<Vec<Game>>> = Arc::new(Mutex::new(if simulate {
        crate::demo::games()
    } else {
        Vec::new()
    }));
    let discover_from = found.clone();
    let mut library = GameLibrary::new(
        Box::new(move || discover_from.lock().map(|g| g.clone()).unwrap_or_default()),
        store,
        system_clock(),
    );
    library.load();
    let col_store: Box<dyn tako_core::library::CollectionStore + Send> = match &database {
        Some(d) => Box::new(db::CollectionStore::new(d.clone())),
        None => Box::new(crate::demo::MemoryCollections::default()),
    };
    let mut collections = CollectionLibrary::new(col_store, system_clock());
    if let Err(e) = collections.load() {
        tracing::error!("coleções: {e}");
    }
    let art = crate::art::cache(&paths);
    let mut ctx = Ctx {
        paths,
        library,
        collections,
        settings,
        game_settings: database.clone().map(db::GameSettingsStore::new),
        choices: database.map(db::GameChoicesStore::new),
        platform,
        version: crate::version().to_owned(),
        pad: Default::default(),
        pad_name: String::new(),
        pads_connected: 0,
        input: InputKind::Keyboard,
        toast: None,
        effects: Vec::new(),
        running: None,
        sources: Vec::new(),
        update_status: String::new(),
        update_ready: None,
        safe_mode: false,
        scanning: false,
        grid_columns: 7,
        pending_layer: None,
        open_details: None,
        leave_details: false,
        queued_launch: None,
        found,
        art,
        controller_mouse: false,
        welcome_back: None,
        move_window: None,
        dirty: true,
        pressed_buttons: 0,
        pad_stick: (0.0, 0.0),
        live_pad: Default::default(),
        game_overrides: Default::default(),
        pinned_ids: Default::default(),
        art_pins: Default::default(),
        metadata: None,
    };
    ctx.sync_collections();
    ctx
}

/// Lê as lojas numa thread e atualiza a biblioteca quando terminar.
pub fn rescan(shell: &Rc<RefCell<Shell>>) {
    let (found, disabled, simulate) = {
        let mut s = shell.borrow_mut();
        if s.ctx.scanning {
            return;
        }
        s.ctx.scanning = true;
        (
            s.ctx.found.clone(),
            s.ctx
                .settings
                .string(crate::shell::ctx::keys::DISABLED_STORES),
            s.ctx.simulated(),
        )
    };
    std::thread::Builder::new()
        .name("tako-scan".into())
        .spawn(move || {
            let (games, reports) = if simulate {
                (crate::demo::games(), Vec::new())
            } else {
                crate::sources::scan(&disabled)
            };
            if let Ok(mut f) = found.lock() {
                *f = games;
            }
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(shell) = crate::tasks::shell() {
                    let mut s = shell.borrow_mut();
                    s.ctx.library.refresh();
                    s.ctx.sync_collections();
                    s.ctx.sources = reports;
                    s.ctx.scanning = false;
                    s.ctx.dirty = true;
                }
            });
        })
        .ok();
}

/// O tema escolhido ou, se ele falhar, o padrão (PRD §7: volta ao padrão sem loop de falha).
/// Também devolve a pasta do tema escolhido (para o `tokens.toml`); `None` = o padrão.
fn load_theme(
    ctx: &Ctx,
    builtin: &std::path::Path,
) -> Result<(ComponentDefinition, Option<std::path::PathBuf>), String> {
    let selected = tako_themes::selected(&ctx.paths.themes());
    if let Some(id) = selected.filter(|_| !ctx.safe_mode) {
        let dir = ctx.paths.themes().join(&id);
        match tako_themes::validate_package(&dir) {
            Ok(m) => match theme::compile(&dir.join(m.entry), builtin) {
                Ok(def) => return Ok((def, Some(dir))),
                Err(e) => tracing::error!("tema {id} não compilou, usando o padrão: {e}"),
            },
            Err(p) => tracing::error!("tema {id} inválido, usando o padrão: {p:?}"),
        }
    }
    theme::compile(&builtin.join("main.slint"), builtin)
        .map(|def| (def, None))
        .map_err(|e| e.to_string())
}

impl App {
    /// Carrega o tema (o escolhido ou o padrão) e liga tudo. `host` recebe os pedidos de sessão e saída.
    pub fn new(ctx: Ctx, opts: &UiOptions, host: Rc<RefCell<dyn Host>>) -> Result<App, String> {
        let builtin =
            theme::install_builtin(&ctx.paths.themes()).map_err(|e| format!("tema padrão: {e}"))?;
        let mut ctx = ctx;
        ctx.safe_mode = opts.safe_mode;
        let (def, dir) = load_theme(&ctx, &builtin)?;
        let ui = def.create().map_err(|e| e.to_string())?;
        theme::apply_tokens(&ui, dir.as_deref());
        let mut shell = Shell::new(ctx);
        if let Some((id, title)) = &opts.running {
            shell.ctx.running = Some(crate::shell::RunningGame {
                id: id.clone(),
                title: title.clone(),
            });
        }
        if opts.control_center {
            shell.open_control_center();
        }
        if let Some((id, played)) = &opts.after_game {
            if let Err(e) = shell.ctx.library.record_session(id, *played) {
                tracing::warn!("tempo de jogo de {id}: {e}");
            }
            shell.ctx.welcome_back = Some((id.clone(), *played));
        }
        let shell = Rc::new(RefCell::new(shell));
        crate::tasks::attach(&shell);
        crate::metadata::start_metadata(&shell);
        let app = App {
            ui: Rc::new(RefCell::new(ui)),
            shell,
            host,
            builtin,
        };
        app.wire_ui();
        app.start_input();
        {
            let mut s = app.shell.borrow_mut();
            if tako_core::updates::UpdatePreferences::load(&s.ctx.paths.updates()).auto_update {
                crate::effects::check_updates(&mut s.ctx, false);
            }
        }
        app.redraw();
        Ok(app)
    }

    /// Trata os pedidos pendentes e desenha se algo mudou.
    pub fn redraw(&self) {
        let action = pump_effects(&self.shell, &self.host);
        self.apply(action);
        render(&self.shell, &self.ui.borrow());
    }

    pub fn apply(&self, action: HostAction) {
        match action {
            HostAction::Stay => {}
            HostAction::Close => {
                let _ = self.ui.borrow().hide();
                let _ = slint::quit_event_loop();
            }
            HostAction::Hide => {
                let _ = self.ui.borrow().hide();
            }
            HostAction::Show => {
                let _ = self.ui.borrow().show();
                self.shell.borrow_mut().ctx.dirty = true;
            }
            HostAction::ReloadTheme => self.reload_theme(),
        }
    }

    /// Troca de tema em tempo de execução: nova instância com as mesmas ligações; o estado continua na `Shell`.
    pub fn reload_theme(&self) {
        let (def, dir) = match load_theme(&self.shell.borrow().ctx, &self.builtin) {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("tema: {e}");
                return;
            }
        };
        let new = match def.create() {
            Ok(n) => n,
            Err(e) => {
                tracing::error!("tema: {e}");
                return;
            }
        };
        theme::apply_tokens(&new, dir.as_deref());
        new.window()
            .set_fullscreen(self.ui.borrow().window().is_fullscreen());
        let old = self.ui.replace(new);
        self.wire_ui();
        let _ = self.ui.borrow().show();
        let _ = old.hide();
        self.shell.borrow_mut().ctx.dirty = true;
        render(&self.shell, &self.ui.borrow());
    }

    fn on_cmd(&self, cmd: Cmd) {
        {
            let mut s = self.shell.borrow_mut();
            match cmd {
                Cmd::PrevSection => s.section(false),
                Cmd::NextSection => s.section(true),
                c => s.handle(c),
            }
        }
        self.redraw();
    }

    fn wire_ui(&self) {
        let ui = self.ui.borrow();
        {
            let app = self.clone();
            let _ = ui.set_callback("key", move |args| {
                let text = arg_str(args, 0);
                let ctrl = matches!(args.get(2), Some(Value::Bool(true)));
                let alt = matches!(args.get(4), Some(Value::Bool(true)));
                let action = {
                    let mut s = app.shell.borrow_mut();
                    s.ctx.input = InputKind::Keyboard;
                    keys::map(&text, ctrl, alt, s.typing())
                };
                match action {
                    KeyAction::Cmd(c) => app.on_cmd(c),
                    KeyAction::Text(t) => {
                        app.shell.borrow_mut().text(&t);
                        app.redraw();
                    }
                    KeyAction::Erase => {
                        app.shell.borrow_mut().erase();
                        app.redraw();
                    }
                    KeyAction::Ignore => return Value::Bool(false),
                }
                Value::Bool(true)
            });
        }
        {
            let app = self.clone();
            let _ = ui.set_callback("clicked", move |args| {
                app.shell
                    .borrow_mut()
                    .click(&arg_str(args, 0), arg_num(args, 1));
                app.redraw();
                Value::Void
            });
        }
        {
            let app = self.clone();
            let _ = ui.set_callback("hovered", move |args| {
                app.shell
                    .borrow_mut()
                    .hover(&arg_str(args, 0), arg_num(args, 1));
                app.redraw();
                Value::Void
            });
        }
        {
            let app = self.clone();
            let _ = ui.set_callback("hint-clicked", move |args| {
                app.shell.borrow_mut().hint_clicked(arg_num(args, 0));
                app.redraw();
                Value::Void
            });
        }
        let _ = ui.set_callback("key-released", |_| Value::Void);
        // Fechar a janela pelo sistema (Alt+F4, botão da janela) é o mesmo que "Sair para o desktop".
        let app = self.clone();
        ui.window().on_close_requested(move || {
            {
                let mut s = app.shell.borrow_mut();
                app.host
                    .borrow_mut()
                    .handle(&Effect::ExitToDesktop, &mut s.ctx);
            }
            let _ = slint::quit_event_loop();
            slint::CloseRequestResponse::HideWindow
        });
    }

    /// Controle (SDL lido a cada 16 ms enquanto a interface está aberta, como o XInput do C#), relógio,
    /// atalho da Central e o `tick` do host.
    fn start_input(&self) {
        {
            let app = self.clone();
            let resolver = crate::art::resolver(&self.shell.borrow().ctx);
            images::init(resolver, 2, move || {
                app.shell.borrow_mut().ctx.dirty = true;
                app.redraw();
            });
        }
        let mut pads = Pads::open()
            .map_err(|e| tracing::warn!("controles indisponíveis: {e}"))
            .ok();
        let mut interp = GamepadInterpreter::new();
        let chord_value = self
            .shell
            .borrow()
            .ctx
            .settings
            .get(crate::shell::ctx::keys::CONTROL_CENTER_CHORD)
            .map(str::to_owned);
        let mut chord = ButtonChordDetector::new(chord::parse(chord_value.as_deref()));
        let started = Instant::now();
        let app = self.clone();
        let mut last_clock = String::new();
        let timer = slint::Timer::default();
        timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(16),
            move || {
                let now = started.elapsed();
                let mut fired = Vec::new();
                let mut pad_state = PadState::default();
                let mut chord_fired = false;
                if let Some(p) = pads.as_mut() {
                    for ev in p.poll() {
                        let mut s = app.shell.borrow_mut();
                        if let PadEvent::Connected { name, family } = ev {
                            s.ctx.pad = family;
                            s.ctx.pad_name = name;
                            s.ctx.input = InputKind::Gamepad;
                        }
                        s.ctx.pads_connected = p.count();
                        s.ctx.dirty = true;
                    }
                    pad_state = p.state();
                    {
                        let mut s = app.shell.borrow_mut();
                        (s.ctx.pressed_buttons, s.ctx.pad_stick) =
                            (pad_state.buttons, (pad_state.left_x, pad_state.left_y));
                        s.ctx.dirty |= s.ctx.live_pad.take();
                    }
                    let pressed = GamepadButtons(pad_state.buttons);
                    chord_fired = chord.update(pressed, now);
                    let snap = GamepadSnapshot::new(
                        pressed,
                        pad_state.left_x as f64,
                        -pad_state.left_y as f64,
                    );
                    fired = interp.update(snap, now, chord.suppressed(pressed));
                    if !fired.is_empty() {
                        let mut s = app.shell.borrow_mut();
                        s.ctx.input = InputKind::Gamepad;
                        s.ctx.pad = p.family();
                        s.ctx.pad_name = p.name().to_owned();
                    }
                }
                if chord_fired {
                    let mut s = app.shell.borrow_mut();
                    if s.control_center_open() {
                        s.close_control_center();
                        if s.ctx.running.is_some() {
                            s.ctx.effect(Effect::ResumeGame);
                        }
                    } else if s.ctx.running.is_some() {
                        s.open_control_center();
                    }
                    fired.clear();
                }
                for c in fired {
                    let mut s = app.shell.borrow_mut();
                    match cmd_of(c) {
                        Cmd::PrevSection => s.section(false),
                        Cmd::NextSection => s.section(true),
                        c => s.handle(c),
                    }
                }
                let action = {
                    let mut s = app.shell.borrow_mut();
                    app.host.borrow_mut().tick(&mut s.ctx, &pad_state, now)
                };
                app.apply(action);
                // Relógio e avisos que expiram.
                let clock = chrono::Local::now().format("%H:%M").to_string();
                if clock != last_clock {
                    bridge::set(&app.ui.borrow(), "clock", clock.to_value());
                    last_clock = clock;
                }
                let expired = app
                    .shell
                    .borrow()
                    .ctx
                    .toast
                    .as_ref()
                    .is_some_and(|(_, at)| at.elapsed() >= crate::shell::TOAST_TIME);
                if expired {
                    app.shell.borrow_mut().ctx.dirty = true;
                }
                let pending = {
                    let s = app.shell.borrow();
                    s.ctx.dirty || !s.ctx.effects.is_empty()
                };
                if pending {
                    app.redraw();
                }
            },
        );
        std::mem::forget(timer);
    }

    pub fn run(&self) -> Result<(), String> {
        CURRENT.with(|c| *c.borrow_mut() = Some(self.clone()));
        let ui = self.ui.borrow();
        // Como o WPF (sem borda, maximizado). Builds de desenvolvimento abrem em janela; TAKO_WINDOWED força janela.
        ui.window()
            .set_fullscreen(!cfg!(debug_assertions) && std::env::var_os("TAKO_WINDOWED").is_none());
        ui.show().map_err(|e| e.to_string())?;
        drop(ui);
        rescan(&self.shell);
        slint::run_event_loop_until_quit().map_err(|e| e.to_string())
    }
}

thread_local! {
    /// O app rodando nesta thread (a da interface), para pedidos que chegam de fora (outro Tako aberto).
    static CURRENT: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// Traz a janela para a frente (alguém abriu o Tako de novo). No Wayland o compositor pode só destacar a
/// janela em vez de focar: lá roubar o foco não é permitido.
pub fn raise_current() {
    CURRENT.with(|c| {
        let Some(app) = c.borrow().clone() else {
            return;
        };
        let ui = app.ui.borrow();
        let window = ui.window();
        let _ = window.show();
        window.set_minimized(false);
        use slint::winit_030::WinitWindowAccessor;
        window.with_winit_window(|w| w.focus_window());
    });
}

/// "Bem-vindo de volta" com o tempo jogado e o foco no jogo (o tempo já foi gravado por quem chama).
pub fn welcome_back(shell: &mut Shell, id: &str, played: Duration) {
    shell.ctx.running = None;
    shell.focus_game(id);
    let title = shell
        .ctx
        .library
        .find(id)
        .map(|g| g.title.clone())
        .unwrap_or_default();
    let text = if played.is_zero() {
        shell.ctx.tf("session.welcome_back", &[("game", &title)])
    } else {
        shell.ctx.tf(
            "session.welcome_back_played",
            &[
                ("game", &title),
                (
                    "time",
                    &tako_core::presentation::game_text::session_played(i18n::lang(), played),
                ),
            ],
        )
    };
    shell.ctx.toast(text);
}

fn arg_str(args: &[Value], i: usize) -> String {
    match args.get(i) {
        Some(Value::String(s)) => s.to_string(),
        _ => String::new(),
    }
}

fn arg_num(args: &[Value], i: usize) -> usize {
    match args.get(i) {
        Some(Value::Number(n)) if *n >= 0.0 => *n as usize,
        _ => 0,
    }
}

/// Trata os pedidos da interface: os comuns aqui, o resto com o host.
fn pump_effects(shell: &Rc<RefCell<Shell>>, host: &Rc<RefCell<dyn Host>>) -> HostAction {
    loop {
        let effect = {
            let mut s = shell.borrow_mut();
            if s.ctx.effects.is_empty() {
                return HostAction::Stay;
            }
            s.ctx.effects.remove(0)
        };
        if effect == Effect::Rescan || effect == Effect::ReloadLibrary {
            rescan(shell);
            continue;
        }
        let action = {
            let mut s = shell.borrow_mut();
            let a = match crate::effects::common(&effect, &mut s) {
                Some(a) => a,
                None => host.borrow_mut().handle(&effect, &mut s.ctx),
            };
            s.after_action();
            a
        };
        if action != HostAction::Stay {
            return action;
        }
    }
}

/// Desenha se algo mudou: propriedades da página, cor do jogo em foco e colunas da grade.
pub fn render(shell: &Rc<RefCell<Shell>>, ui: &ComponentInstance) {
    let mut s = shell.borrow_mut();
    if !s.ctx.dirty {
        return;
    }
    s.ctx.dirty = false;
    if let Some((id, played)) = s.ctx.welcome_back.take() {
        welcome_back(&mut s, &id, played);
    }
    if let Some(b) = s.ctx.move_window.take() {
        let w = ui.window();
        let full = w.is_fullscreen();
        w.set_fullscreen(false);
        w.set_position(slint::PhysicalPosition::new(b.x, b.y));
        w.set_size(slint::PhysicalSize::new(
            b.width.max(1) as u32,
            b.height.max(1) as u32,
        ));
        w.set_fullscreen(full);
    }
    if let Ok(Value::Number(n)) = ui.get_property("library-columns") {
        s.ctx.grid_columns = (n as usize).max(1);
    }
    let out = s.render(Instant::now());
    for (name, value) in out.props {
        bridge::set(ui, name, value);
    }
    let focused = s.focused_game();
    let game = focused.as_deref().and_then(|id| s.ctx.library.find(id));
    let dynamic = s
        .ctx
        .settings
        .bool(crate::shell::ctx::keys::DYNAMIC_COLOR, true);
    let palette = AccentPalette::for_accent(game.map(|g| g.accent.as_str()), dynamic);
    let color = |c: tako_core::presentation::color::Rgb| slint::Color::from_rgb_u8(c.r, c.g, c.b);
    bridge::set_global(ui, "Ui", "accent", color(palette.accent).to_value());
    bridge::set_global(ui, "Ui", "accent-deep", color(palette.deep).to_value());
    bridge::set_global(
        ui,
        "Ui",
        "reduce-motion",
        s.ctx
            .settings
            .bool(crate::shell::ctx::keys::REDUCED_MOTION, false)
            .to_value(),
    );
    let art = game
        .map(|g| crate::shell::tiles::image(g.shown_hero(), crate::shell::tiles::HERO))
        .unwrap_or_default();
    bridge::set(ui, "backdrop", art.to_value());
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoHost;
    impl Host for NoHost {
        fn handle(&mut self, _: &Effect, _: &mut Ctx) -> HostAction {
            HostAction::Stay
        }
    }

    #[test]
    fn dialog_opened_by_an_effect_shows_without_a_key() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = build_ctx(DataPaths::at(dir.path()), Platform::simulated(), true);
        let shell = Rc::new(RefCell::new(Shell::new(ctx)));
        let host: Rc<RefCell<dyn Host>> = Rc::new(RefCell::new(NoHost));
        shell.borrow_mut().ctx.effect(Effect::SaveDiagnostics);
        pump_effects(&shell, &host);
        assert!(
            shell.borrow().has_layers(),
            "o relatório do sistema abre um diálogo"
        );
        assert!(shell.borrow().ctx.pending_layer.is_none());
    }
}
