//! Laço da interface: carrega o tema, monta o contexto, liga entrada (controle e teclado) e desenha quando
//! algo muda. As tarefas longas (ler as lojas, metadados) rodam em threads e voltam pelo laço do Slint.

use crate::gamepad::{PadState, Pads};
use crate::shell::keys::{self, KeyAction};
use crate::shell::{Cmd, Ctx, Effect, InputKind, Settings, Shell};
use crate::{bridge, images, theme, values::ToValue};
use slint::ComponentHandle;
use slint_interpreter::{ComponentDefinition, ComponentInstance, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tako_core::i18n::{self, Lang};
use tako_core::input::shortcut::{ButtonChordDetector, KeyModifiers, chord};
use tako_core::input::{AppCommand, GamepadButtons, GamepadInterpreter, GamepadSnapshot};
use tako_core::library::{CollectionLibrary, Game, GameLibrary, system_clock};
use tako_core::paths::DataPaths;
use tako_core::persistence::{self as db, TakoDatabase};
use tako_core::presentation::color::AccentPalette;
use tako_core::session::GameEnd;
use tako_platform::Platform;

/// Como a interface foi aberta.
#[derive(Debug, Clone, Default)]
pub struct UiOptions {
    /// Sem lojas nem funções do SO (testes, demonstração e CI).
    pub simulate: bool,
    /// Voltou de um jogo: foco nele e "Bem-vindo de volta".
    pub after_game: Option<(String, Duration, GameEnd)>,
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
    /// O jogo abriu com acompanhamento: "Abrindo…" até a janela dele aparecer.
    Launching,
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
        pad_lost: false,
        pad_battery: None,
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
        pointer: false,
        pending_layer: None,
        open_details: None,
        leave_details: false,
        queued_launch: None,
        found,
        art,
        controller_mouse: false,
        welcome_back: None,
        redraw_at: None,
        launching: false,
        leaving: false,
        launch_on_top_until: None,
        window_on_top: false,
        move_window: None,
        dirty: true,
        pressed_buttons: 0,
        pad_stick: (0.0, 0.0),
        live_pad: Default::default(),
        game_overrides: Default::default(),
        pinned_ids: Default::default(),
        art_pins: Default::default(),
        metadata: None,
        broken_checked: None,
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
            let reports = scan_into(&found, &disabled, simulate);
            let to_measure: Vec<(String, String)> = found
                .lock()
                .map(|f| {
                    f.iter()
                        .filter(|g| tako_core::library::storage::needs_measure(g))
                        .filter_map(|g| Some((g.id.clone(), g.install_path.clone()?)))
                        .collect()
                })
                .unwrap_or_default();
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
            measure_sizes(&found, to_measure);
        })
        .ok();
}

/// Lê as lojas e troca os jogos encontrados. Em simulação a lista é a que o ctx já tem (a demonstração ou os
/// 1.000 jogos do bench): reler não pode trocá-la, senão o bench passa a medir 12 jogos.
fn scan_into(
    found: &Mutex<Vec<Game>>,
    disabled: &str,
    simulate: bool,
) -> Vec<tako_sources::report::LibrarySourceReport> {
    if simulate {
        return Vec::new();
    }
    let (games, reports) = crate::sources::scan(disabled);
    if let Ok(mut f) = found.lock() {
        *f = games;
    }
    reports
}

/// LIB-005: mede a pasta dos jogos sem tamanho da loja (depois da biblioteca já aparecer) e atualiza a biblioteca
/// uma vez no fim. Junta pelo id: uma leitura nova no meio do caminho não perde nada.
fn measure_sizes(found: &Arc<Mutex<Vec<Game>>>, to_measure: Vec<(String, String)>) {
    use tako_core::library::storage;
    if to_measure.is_empty() {
        return;
    }
    let sizes: HashMap<String, u64> = to_measure
        .into_iter()
        .filter_map(|(id, path)| {
            let bytes = storage::folder_size(std::path::Path::new(&path), storage::MAX_ENTRIES)?;
            (bytes > 0).then_some((id, bytes))
        })
        .collect();
    tracing::info!(medidos = sizes.len(), "tamanho das pastas dos jogos");
    if sizes.is_empty() {
        return;
    }
    if let Ok(mut f) = found.lock() {
        for g in f.iter_mut().filter(|g| g.size_bytes.is_none()) {
            if let Some(&bytes) = sizes.get(&g.id) {
                g.size_bytes = Some(bytes);
                g.size_label = tako_sources::steam::size_label(bytes);
            }
        }
    }
    let _ = slint::invoke_from_event_loop(|| {
        if let Some(shell) = crate::tasks::shell() {
            let mut s = shell.borrow_mut();
            s.ctx.library.refresh();
            s.ctx.sync_collections();
            s.ctx.dirty = true;
        }
    });
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
            shell.open_control_center_over_game();
        }
        if let Some((id, played, end)) = &opts.after_game {
            if let Err(e) = shell.ctx.library.record_session(id, *played) {
                tracing::warn!("tempo de jogo de {id}: {e}");
            }
            shell.ctx.welcome_back = Some((id.clone(), *played, *end));
        } else if !opts.control_center {
            crate::shell::first_run::open_if_needed(&mut shell);
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
            HostAction::Launching => self.shell.borrow_mut().begin_launch_splash(),
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
                let shift = matches!(args.get(3), Some(Value::Bool(true)));
                let alt = matches!(args.get(4), Some(Value::Bool(true)));
                let action = {
                    let mut s = app.shell.borrow_mut();
                    s.ctx.input = InputKind::Keyboard;
                    let mods = [
                        (ctrl, KeyModifiers::CTRL),
                        (alt, KeyModifiers::ALT),
                        (shift, KeyModifiers::SHIFT),
                    ]
                    .into_iter()
                    .filter(|(on, _)| *on)
                    .fold(KeyModifiers::NONE, |a, (_, m)| a | m);
                    if s.key(&text, mods) {
                        drop(s);
                        app.redraw();
                        return Value::Bool(true);
                    }
                    let setting = s
                        .ctx
                        .settings
                        .get(crate::shell::ctx::keys::CONTROL_CENTER_KEY);
                    if !s.typing() && keys::is_control_center(setting, &text, mods) {
                        KeyAction::Cmd(Cmd::Guide)
                    } else {
                        keys::map(&text, ctrl, alt, s.typing())
                    }
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
        let mut next_battery = Duration::ZERO;
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
                        app.shell.borrow_mut().ctx.pad_event(ev, p.count());
                        // O que já estiver apertado no controle que voltou (ou no outro) só vale depois de solto.
                        interp.reset();
                        next_battery = now;
                    }
                    // Bateria a cada 30 s (e logo depois de um controle entrar ou sair), nunca por quadro.
                    if now >= next_battery {
                        next_battery = now + Duration::from_secs(30);
                        let (level, low) = p.battery();
                        let mut s = app.shell.borrow_mut();
                        if s.ctx.pad_battery != level {
                            s.ctx.pad_battery = level;
                            s.ctx.dirty = true;
                        }
                        for (pad, pct) in low {
                            let text = s.ctx.tf(
                                "status.battery_low",
                                &[("pad", &pad), ("percent", &pct.to_string())],
                            );
                            s.ctx.toast(text);
                        }
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
                {
                    let mut s = app.shell.borrow_mut();
                    if s.ctx.redraw_at.is_some_and(|t| Instant::now() >= t) {
                        s.ctx.redraw_at = None;
                        s.ctx.dirty = true;
                    }
                }
                let expired = app.shell.borrow().ctx.toast_expired(Instant::now());
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

/// Quanto dura o fade de "Abrindo…" para o jogo (o mesmo do tema).
const LEAVE_FADE: Duration = Duration::from_millis(450);

/// O supervisor viu a janela do jogo: com "Abrindo…" na tela, a interface some aos poucos e fecha. Se o usuário
/// já voltou ao Tako (B), fica aberta.
pub fn game_shown() {
    CURRENT.with(|c| {
        let Some(app) = c.borrow().clone() else {
            return;
        };
        {
            let mut s = app.shell.borrow_mut();
            if !s.ctx.launching {
                return;
            }
            s.ctx.leaving = true;
            s.ctx.dirty = true;
        }
        app.redraw();
        slint::Timer::single_shot(LEAVE_FADE, || {
            CURRENT.with(|c| {
                if let Some(app) = c.borrow().clone() {
                    app.apply(HostAction::Close);
                }
            });
        });
    });
}

/// "Bem-vindo de volta" com o tempo jogado e o foco no jogo (o tempo já foi gravado por quem chama). Se o jogo
/// fechou logo ao abrir ou nem abriu, um aviso no lugar (SESSION-003).
pub fn welcome_back(shell: &mut Shell, id: &str, played: Duration, end: GameEnd) {
    shell.ctx.running = None;
    shell.close_game_layers();
    shell.focus_game(id);
    if end != GameEnd::Normal {
        tracing::warn!("{id} terminou fora do normal: {} ({played:?})", end.key());
        let dialog = crate::shell::dialogs::game_ended_badly(&shell.ctx, id, end);
        shell.push_layer(Box::new(dialog));
        return;
    }
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
        if effect == Effect::EndGame {
            shell.borrow_mut().begin_closing_game();
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
    let on_top = s.ctx.launching
        && s.ctx
            .launch_on_top_until
            .is_some_and(|t| Instant::now() < t);
    if on_top != s.ctx.window_on_top {
        s.ctx.window_on_top = on_top;
        use slint::winit_030::{WinitWindowAccessor, winit::window::WindowLevel};
        ui.window().with_winit_window(|w| {
            w.set_window_level(if on_top {
                WindowLevel::AlwaysOnTop
            } else {
                WindowLevel::Normal
            })
        });
    }
    if let Some((id, played, end)) = s.ctx.welcome_back.take() {
        welcome_back(&mut s, &id, played, end);
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
    bridge::set_global(ui, "Ui", "pointer", s.ctx.pointer.to_value());
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
    images::frame_done();
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

    #[test]
    fn controller_drop_keeps_focus_and_reconnect_resumes() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        i18n::set_lang(Lang::PtBr);
        let dir = tempfile::tempdir().unwrap();
        let ctx = build_ctx(DataPaths::at(dir.path()), Platform::simulated(), true);
        let mut shell = Shell::new(ctx);
        shell.ctx.input = InputKind::Gamepad;
        for c in [Cmd::Down, Cmd::Right] {
            shell.handle(c);
        }
        let focus = shell.focused_game();
        assert!(focus.is_some());

        shell
            .ctx
            .pad_event(crate::gamepad::PadEvent::Disconnected { remaining: 0 }, 0);
        let later = Instant::now() + crate::shell::TOAST_TIME * 2;
        shell.render(later);
        assert_eq!(shell.focused_game(), focus);
        assert!(!shell.has_layers());
        let toast = |s: &Shell| s.ctx.toast.as_ref().map(|(t, _)| t.clone());
        assert_eq!(toast(&shell).as_deref(), Some("Controle desconectado"));

        shell.ctx.pad_event(
            crate::gamepad::PadEvent::Connected {
                name: "Outro".into(),
                family: Default::default(),
            },
            1,
        );
        assert_eq!(toast(&shell).as_deref(), Some("Controle conectado"));
        shell.render(later);
        assert_eq!(toast(&shell), None);
        shell.handle(Cmd::Right);
        assert_ne!(shell.focused_game(), focus);
    }

    #[test]
    fn simulated_rescan_keeps_the_library_it_was_given() {
        let bench: Vec<Game> = (0..1000)
            .map(|i| Game {
                id: format!("bench:{i}"),
                ..Default::default()
            })
            .collect();
        let found = Mutex::new(bench);
        assert!(scan_into(&found, "", true).is_empty());
        assert_eq!(found.lock().unwrap().len(), 1000);
    }

    #[test]
    fn a_game_that_closed_right_away_shows_a_warning_instead_of_welcome_back() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        i18n::set_lang(Lang::PtBr);
        let dir = tempfile::tempdir().unwrap();
        let ctx = build_ctx(DataPaths::at(dir.path()), Platform::simulated(), true);
        let mut shell = Shell::new(ctx);
        let game = shell.ctx.library.games()[0].clone();

        welcome_back(&mut shell, &game.id, Duration::from_secs(3), GameEnd::Quick);
        assert!(shell.ctx.toast.is_none());
        assert!(shell.has_layers());
        use crate::shell::Layer;
        let warning = crate::shell::dialogs::game_ended_badly(&shell.ctx, &game.id, GameEnd::Quick);
        let title = warning.render(&shell.ctx).title;
        assert_eq!(title, format!("{} fechou logo depois de abrir", game.title));

        shell.close_layers();
        welcome_back(
            &mut shell,
            &game.id,
            Duration::from_secs(600),
            GameEnd::Normal,
        );
        assert!(!shell.has_layers());
        assert!(
            shell
                .ctx
                .toast
                .as_ref()
                .unwrap()
                .0
                .starts_with("Bem-vindo de volta")
        );
    }
}
