//! Laço da interface: carrega o tema, monta o contexto, liga entrada (controle e teclado) e desenha quando
//! algo muda. As tarefas longas (ler as lojas, metadados) rodam em threads e voltam pelo laço do Slint.

use crate::gamepad::{PadState, Pads};
use crate::mode::UiMode;
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
use tako_core::input::profiles::Profiles;
use tako_core::input::shortcut::{ButtonChordDetector, KeyModifiers, chord};
use tako_core::input::{AppCommand, GamepadInterpreter, GamepadSnapshot};
use tako_core::library::{CollectionLibrary, Game, GameLibrary, system_clock};
use tako_core::paths::DataPaths;
use tako_core::persistence::{self as db, TakoDatabase};
use tako_core::presentation::color::AccentPalette;
use tako_core::session::GameEnd;
use tako_platform::Platform;

/// Como a interface foi aberta.
#[derive(Debug, Clone, Default)]
pub struct UiOptions {
    pub link: Option<tako_core::links::Link>,
    /// Sem lojas nem funções do SO (testes, demonstração e CI).
    pub simulate: bool,
    /// Voltou de um jogo: foco nele e "Bem-vindo de volta".
    pub after_game: Option<(String, Duration, GameEnd)>,
    /// Um jogo continua rodando (aberta pela Central ou "Início" durante o jogo): id e título.
    pub running: Option<(String, String)>,
    /// Abrir direto na Central rápida.
    pub control_center: bool,
    pub safe_mode: bool,
    /// Início do processo (o `main`), para o tempo até a primeira tela (PERF-001).
    pub started: Option<Instant>,
    /// Desktop ou tela cheia (ADR-0027).
    pub mode: UiMode,
    /// Perguntar pela tela cheia se um controle aparecer logo depois de abrir.
    pub ask_big_picture: bool,
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
    /// Minimiza a janela (modo desktop durante o jogo, ADR-0027).
    Minimize,
    /// Troca entre o desktop e a tela cheia.
    SwitchMode(UiMode),
}

/// O que o app (binário) faz com os pedidos da interface que saem dela: abrir jogos, sair, atualizar.
pub trait Host {
    fn handle(&mut self, effect: &Effect, ctx: &mut Ctx) -> HostAction;
    /// A cada 16 ms com o estado do controle (sessão sem supervisor, heartbeat).
    fn tick(&mut self, _ctx: &mut Ctx, _pad: &PadState, _now: Duration) -> HostAction {
        HostAction::Stay
    }
    /// A interface abriu ou trocou de modo (o supervisor precisa saber para reabrir no mesmo modo).
    fn mode_changed(&mut self, _mode: UiMode) {}
}

#[derive(Clone)]
pub struct App {
    pub ui: Rc<RefCell<ComponentInstance>>,
    pub shell: Rc<RefCell<Shell>>,
    host: Rc<RefCell<dyn Host>>,
    builtin: PathBuf,
    mode: Rc<std::cell::Cell<UiMode>>,
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
            &paths.machine,
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
    let profiles = Profiles::load(|k| settings.get(k).map(str::to_owned));
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
        pad_guid: String::new(),
        pads: Vec::new(),
        profiles,
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
        open_route: None,
        toast_undo: None,
        leave_details: false,
        queued_launch: None,
        found,
        art,
        controller_mouse: false,
        welcome_back: None,
        installing: Vec::new(),
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
        desktop: false,
        list_first: 0,
        focus_visible: false,
    };
    ctx.sync_collections();
    ctx
}

/// Lê as lojas numa thread e atualiza a biblioteca quando terminar.
pub fn rescan(shell: &Rc<RefCell<Shell>>) {
    let (found, disabled, simulate, steam_key, steam_cache, emulators) = {
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
            s.ctx.platform.secrets.get(crate::steam_owned::SECRET),
            crate::steam_owned::cache(&s.ctx.paths),
            crate::sources::emulator_config(&s.ctx.settings),
        )
    };
    std::thread::Builder::new()
        .name("tako-scan".into())
        .spawn(move || {
            // Jogos da conta Steam (SRC-005): só com a chave; sem ela, nem o cache antigo entra.
            let steam_owned = steam_key.map(|key| {
                if !simulate && let Err(e) = crate::steam_owned::refresh(&key, &steam_cache, false)
                {
                    tracing::warn!("jogos da conta Steam: {e}");
                }
                steam_cache
            });
            let reports = scan_into(&found, &disabled, simulate, steam_owned, emulators);
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
                    s.apply_pending_link();
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
    steam_owned: Option<PathBuf>,
    emulators: tako_sources::emulators::Config,
) -> Vec<tako_sources::report::LibrarySourceReport> {
    if simulate {
        return Vec::new();
    }
    let (games, reports) = crate::sources::scan(disabled, steam_owned, emulators);
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

/// A janela do modo: a do desktop (visual do Tako, sem temas por enquanto) ou a do tema escolhido.
fn load_mode(
    ctx: &Ctx,
    builtin: &std::path::Path,
    mode: UiMode,
) -> Result<(ComponentDefinition, Option<std::path::PathBuf>), String> {
    match mode {
        UiMode::Desktop => theme::compile_desktop(builtin)
            .map(|def| (def, None))
            .map_err(|e| e.to_string()),
        UiMode::BigPicture => load_theme(ctx, builtin),
    }
}

/// A tela cheia é sem borda e maximizada, como no WPF. Builds de desenvolvimento abrem em janela; TAKO_WINDOWED
/// força janela.
fn big_picture_fullscreen() -> bool {
    !cfg!(debug_assertions) && std::env::var_os("TAKO_WINDOWED").is_none()
}

impl App {
    /// Carrega o tema (o escolhido ou o padrão) e liga tudo. `host` recebe os pedidos de sessão e saída.
    pub fn new(ctx: Ctx, opts: &UiOptions, host: Rc<RefCell<dyn Host>>) -> Result<App, String> {
        let builtin =
            theme::install_builtin(&ctx.paths.themes()).map_err(|e| format!("tema padrão: {e}"))?;
        crate::shell::performance::mark_start(opts.started.unwrap_or_else(Instant::now));
        let mut ctx = ctx;
        ctx.safe_mode = opts.safe_mode;
        // "Abrir em: tela cheia" (onboarding, Configurações › Geral) vale para a abertura sem argumentos.
        let mode = crate::mode::preferred(opts.mode, opts.ask_big_picture, &ctx.settings);
        let (def, dir) = load_mode(&ctx, &builtin, mode)?;
        let ui = def.create().map_err(|e| e.to_string())?;
        theme::apply_tokens(&ui, dir.as_deref());
        let mut shell = Shell::new(ctx);
        if mode == UiMode::Desktop {
            shell.enter_desktop();
        }
        shell.pending_link = opts.link.clone();
        if let Some((id, title)) = &opts.running {
            shell.ctx.running = Some(crate::shell::RunningGame {
                id: id.clone(),
                title: title.clone(),
                pid: None,
                started: None,
            });
        }
        if opts.control_center {
            if mode == UiMode::BigPicture {
                shell.open_control_center_over_game();
            } else {
                shell.open_control_center();
            }
        }
        if let Some((id, played, end)) = &opts.after_game {
            if let Err(e) = shell.ctx.library.record_session(id, *played) {
                tracing::warn!("tempo de jogo de {id}: {e}");
            }
            shell.ctx.welcome_back = Some((id.clone(), *played, *end));
        } else if !opts.control_center {
            crate::shell::first_run::open_if_needed(&mut shell);
        }
        // "Quem está jogando?" (UX-004) só numa abertura comum; fica por cima do assistente da primeira execução.
        let plain_start = opts.after_game.is_none()
            && opts.running.is_none()
            && !opts.control_center
            && opts.link.is_none()
            && !opts.simulate;
        if let Some(who) = crate::shell::profiles::chooser_on_start(&shell.ctx, plain_start) {
            shell.push_layer(Box::new(who));
        }
        shell.ask_big_picture = opts.ask_big_picture
            && mode == UiMode::Desktop
            && plain_start
            && shell.ctx.settings.bool(crate::mode::ASK_SETTING, true);
        let shell = Rc::new(RefCell::new(shell));
        crate::tasks::attach(&shell);
        crate::metadata::start_metadata(&shell);
        host.borrow_mut().mode_changed(mode);
        let app = App {
            ui: Rc::new(RefCell::new(ui)),
            shell,
            host,
            builtin,
            mode: Rc::new(std::cell::Cell::new(mode)),
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
            // No desktop a janela fica aberta durante o jogo, minimizada (ADR-0027).
            HostAction::Hide | HostAction::Minimize if self.mode.get() == UiMode::Desktop => {
                self.ui.borrow().window().set_minimized(true);
            }
            HostAction::Hide | HostAction::Minimize => {
                let _ = self.ui.borrow().hide();
            }
            HostAction::Show => {
                restore(&self.ui.borrow());
                self.shell.borrow_mut().ctx.dirty = true;
            }
            HostAction::ReloadTheme if self.mode.get() == UiMode::Desktop => {
                // O tema é da tela cheia: vale na próxima troca.
                let mut s = self.shell.borrow_mut();
                let text = s.ctx.t("desktop.theme_big_picture");
                s.ctx.toast(text);
            }
            HostAction::ReloadTheme => self.reload_theme(),
            HostAction::Launching if self.mode.get() == UiMode::Desktop => {
                // "Abrindo…" no botão Jogar até a janela do jogo aparecer.
                let mut s = self.shell.borrow_mut();
                s.ctx.launching = true;
                s.ctx.dirty = true;
            }
            HostAction::Launching => self.shell.borrow_mut().begin_launch_splash(),
            HostAction::SwitchMode(mode) => {
                // Fora do callback que pediu: a janela antiga ainda está entregando o evento.
                let app = self.clone();
                slint::Timer::single_shot(Duration::ZERO, move || app.switch_mode(mode));
            }
        }
    }

    /// Troca entre o desktop e a tela cheia (ADR-0027): outra janela com as mesmas ligações; o estado continua
    /// na `Shell`. A janela antiga é descartada (só uma fica na memória).
    pub fn switch_mode(&self, mode: UiMode) {
        if self.mode.get() == mode {
            return;
        }
        let new =
            load_mode(&self.shell.borrow().ctx, &self.builtin, mode).and_then(|(def, dir)| {
                let ui = def.create().map_err(|e| e.to_string())?;
                theme::apply_tokens(&ui, dir.as_deref());
                Ok(ui)
            });
        let new = match new {
            Ok(n) => n,
            Err(e) => {
                tracing::error!("trocar de modo: {e}");
                return;
            }
        };
        tracing::info!("modo da interface: {mode:?}");
        self.mode.set(mode);
        {
            let mut s = self.shell.borrow_mut();
            match mode {
                UiMode::Desktop => s.enter_desktop(),
                UiMode::BigPicture => s.enter_big_picture(),
            }
        }
        new.window()
            .set_fullscreen(mode == UiMode::BigPicture && big_picture_fullscreen());
        let old = self.ui.replace(new);
        self.wire_ui();
        let _ = self.ui.borrow().show();
        let _ = old.hide();
        drop(old);
        restore(&self.ui.borrow());
        self.host.borrow_mut().mode_changed(mode);
        self.shell.borrow_mut().ctx.dirty = true;
        self.redraw();
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

    /// O mesmo atalho global ou do controle abre a Central e, repetido, volta ao jogo.
    pub fn toggle_control_center(&self) {
        {
            let mut s = self.shell.borrow_mut();
            if s.ctx.running.is_none() {
                return;
            }
            if s.control_center_open() {
                s.close_control_center();
                s.ctx.effect(Effect::ResumeGame);
            } else {
                s.open_control_center();
            }
        }
        restore(&self.ui.borrow());
        self.redraw();
    }

    fn on_cmd(&self, cmd: Cmd) {
        if cmd == Cmd::Guide && self.shell.borrow().ctx.running.is_some() {
            self.toggle_control_center();
            return;
        }
        crate::navigation_feedback::play(&self.shell.borrow().ctx, cmd);
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
                crate::navigation_feedback::play(&app.shell.borrow().ctx, Cmd::Accept);
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
        {
            // Campos de texto do desktop (busca, nome de coleção): zona e o texto inteiro. O tema da tela cheia não tem.
            let app = self.clone();
            let _ = ui.set_callback("edited", move |args| {
                app.shell
                    .borrow_mut()
                    .edited(&arg_str(args, 0), &arg_str(args, 1));
                app.redraw();
                Value::Void
            });
        }
        {
            // A lista do desktop rolou: pede as miniaturas da área nova.
            let app = self.clone();
            let _ = ui.set_callback("scrolled", move |args| {
                let first = arg_num(args, 0);
                let changed = {
                    let mut s = app.shell.borrow_mut();
                    let changed = s.ctx.list_first != first;
                    s.ctx.list_first = first;
                    s.ctx.dirty |= changed;
                    changed
                };
                if changed {
                    app.redraw();
                }
                Value::Void
            });
        }
        {
            // BarraDaJanela (só o desktop tem): a janela não tem moldura do sistema.
            let app = self.clone();
            let _ = ui.set_callback("window-action", move |args| {
                let ui = app.ui.borrow();
                let window = ui.window();
                match arg_str(args, 0).as_str() {
                    "drag" => {
                        use slint::winit_030::WinitWindowAccessor;
                        window.with_winit_window(|w| {
                            let _ = w.drag_window();
                        });
                    }
                    "min" => window.set_minimized(true),
                    "max" => {
                        let max = !window.is_maximized();
                        window.set_maximized(max);
                        let _ = ui.set_property("window-maximized", Value::Bool(max));
                    }
                    "update" => {
                        drop(ui);
                        app.shell.borrow_mut().ctx.effect(Effect::InstallUpdate);
                        app.redraw();
                    }
                    menu if menu.starts_with("menu:") => {
                        use slint::winit_030::WinitWindowAccessor;
                        let mut xy = menu["menu:".len()..]
                            .split(',')
                            .map(|n| n.parse::<f64>().unwrap_or(0.));
                        let (x, y) = (xy.next().unwrap_or(0.), xy.next().unwrap_or(0.));
                        window.with_winit_window(|w| {
                            w.show_window_menu(slint::winit_030::winit::dpi::LogicalPosition::new(
                                x, y,
                            ));
                        });
                    }
                    "close" => {
                        drop(ui);
                        {
                            let mut s = app.shell.borrow_mut();
                            app.host
                                .borrow_mut()
                                .handle(&Effect::ExitToDesktop, &mut s.ctx);
                        }
                        let _ = slint::quit_event_loop();
                    }
                    _ => {}
                }
                Value::Void
            });
        }
        {
            // Win+↑/↓, o encaixe e a troca de foco mudam a janela por fora do Tako: a BarraDaJanela lê o estado.
            let app = self.clone();
            let timer = slint::Timer::default();
            timer.start(
                slint::TimerMode::Repeated,
                Duration::from_millis(250),
                move || {
                    use slint::winit_030::WinitWindowAccessor;
                    if !app.shell.borrow().ctx.desktop {
                        return;
                    }
                    let ui = app.ui.borrow();
                    let window = ui.window();
                    let active = window.with_winit_window(|w| w.has_focus()).unwrap_or(true);
                    bridge::set(&ui, "window-maximized", window.is_maximized().to_value());
                    bridge::set(&ui, "window-active", active.to_value());
                },
            );
            WINDOW_STATE.with(|t| *t.borrow_mut() = Some(timer));
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
        // A combinação vem do perfil de cada controle (INPUT-001); o detector só conta o tempo segurado.
        let mut chord = ButtonChordDetector::new(chord::DEFAULT);
        let started = Instant::now();
        let app = self.clone();
        let mut last_clock = String::new();
        let mut next_battery = Duration::ZERO;
        let mut next_install_check = Duration::ZERO;
        let mut next_live = Duration::ZERO;
        // Só registra enquanto há jogo; ao fechar a UI, o supervisor assume o mesmo atalho.
        let mut hotkey = None;
        let mut hotkey_game = false;
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
                        s.ctx.pad_event(ev, p.count());
                        s.ctx.pads = p.connected();
                        drop(s);
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
                    // Cada controle pelo seu perfil (INPUT-001): botões e combinação da Central.
                    let input = app.shell.borrow().ctx.profiles.read(p.buttons());
                    chord_fired = chord.update_held(input.central, now);
                    let snap = GamepadSnapshot::new(
                        input.buttons,
                        pad_state.left_x as f64,
                        -pad_state.left_y as f64,
                    );
                    fired = interp.update(snap, now, input.ignore);
                    if !fired.is_empty() {
                        let mut s = app.shell.borrow_mut();
                        s.ctx.input = InputKind::Gamepad;
                        s.ctx.pad = p.family();
                        s.ctx.pad_name = p.name().to_owned();
                        s.ctx.pad_guid = p.guid().to_owned();
                    }
                }
                let desktop = app.mode.get() == UiMode::Desktop;
                let running = app.shell.borrow().ctx.running.is_some();
                if running != hotkey_game {
                    hotkey_game = running;
                    hotkey = if running {
                        let key = tako_core::input::shortcut::KeyShortcut::parse(
                            app.shell
                                .borrow()
                                .ctx
                                .settings
                                .get(crate::shell::ctx::keys::CONTROL_CENTER_KEY),
                        );
                        tako_platform::global_hotkey(&key, || {
                            let _ = slint::invoke_from_event_loop(|| {
                                CURRENT.with(|c| {
                                    if let Some(app) = c.borrow().clone() {
                                        app.toggle_control_center();
                                    }
                                });
                            });
                        })
                    } else {
                        None
                    };
                }
                let _ = &hotkey;
                {
                    let mut s = app.shell.borrow_mut();
                    crate::shell::desktop::ask_big_picture(&mut s, now);
                    // No desktop, durante o jogo, o controle é do jogo (a janela minimizada não reage).
                    if desktop && s.ctx.running.is_some() && !s.control_center_open() {
                        fired.clear();
                    }
                }
                if chord_fired {
                    app.toggle_control_center();
                    fired.clear();
                }
                for c in fired {
                    let mut s = app.shell.borrow_mut();
                    let command = cmd_of(c);
                    crate::navigation_feedback::play(&s.ctx, command);
                    match command {
                        Cmd::PrevSection => s.section(false),
                        Cmd::NextSection => s.section(true),
                        c => s.handle(c),
                    }
                }
                // Instalando pela loja (SRC-005): relê a biblioteca a cada 30 s até o jogo aparecer instalado.
                if now >= next_install_check {
                    next_install_check = now + Duration::from_secs(30);
                    if app.shell.borrow_mut().ctx.check_installs() {
                        rescan(&app.shell);
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
                    if now >= next_live && s.live() {
                        next_live = now + Duration::from_secs(1);
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
        ui.window()
            .set_fullscreen(self.mode.get() == UiMode::BigPicture && big_picture_fullscreen());
        // Quem já observa os quadros (o benchmark) fica com o aviso; aqui é só a primeira tela.
        let _ = ui.window().set_rendering_notifier(|state, _| {
            if let slint::RenderingState::AfterRendering = state {
                crate::shell::performance::first_frame_drawn();
            }
        });
        ui.show().map_err(|e| e.to_string())?;
        drop(ui);
        rescan(&self.shell);
        slint::run_event_loop_until_quit().map_err(|e| e.to_string())
    }
}

thread_local! {
    /// Lê maximizada e em foco para a BarraDaJanela (desktop).
    static WINDOW_STATE: RefCell<Option<slint::Timer>> = const { RefCell::new(None) };
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
        restore(&app.ui.borrow());
    });
}

/// Mostra a janela, tira do minimizado e pede o foco.
fn restore(ui: &ComponentInstance) {
    let window = ui.window();
    let _ = window.show();
    window.set_minimized(false);
    use slint::winit_030::WinitWindowAccessor;
    window.with_winit_window(|w| w.focus_window());
}

/// Um segundo processo encaminhou um link à interface já aberta.
pub fn open_current_link(url: &str) {
    if let Some(link) = tako_core::links::parse(url) {
        if let Some(shell) = crate::tasks::shell() {
            shell.borrow_mut().open_link(link);
        }
        raise_current();
    }
}

/// Quanto dura o fade de "Abrindo…" para o jogo (o mesmo `Ui.leave-time` do tema): nada com "Reduzir animações".
fn leave_fade(settings: &crate::shell::Settings) -> Duration {
    if settings.bool(crate::shell::ctx::keys::REDUCED_MOTION, false) {
        Duration::ZERO
    } else {
        Duration::from_millis(450)
    }
}

/// O supervisor viu a janela do jogo: com "Abrindo…" na tela, a interface some aos poucos e fecha. Se o usuário
/// já voltou ao Tako (B), fica aberta.
pub fn game_shown() {
    CURRENT.with(|c| {
        if let Some(app) = c.borrow().clone() {
            app.game_shown();
        }
    });
}

impl App {
    /// Resposta do supervisor: desktop minimiza; tela cheia fecha depois do fade.
    pub fn game_shown(&self) {
        let fade = {
            let mut s = self.shell.borrow_mut();
            if !s.ctx.launching {
                return;
            }
            if self.mode.get() == UiMode::Desktop {
                s.ctx.launching = false;
                s.ctx.dirty = true;
                drop(s);
                self.apply(HostAction::Minimize);
                self.redraw();
                return;
            }
            s.ctx.leaving = true;
            s.ctx.dirty = true;
            leave_fade(&s.ctx.settings)
        };
        self.redraw();
        let app = self.clone();
        slint::Timer::single_shot(fade, move || {
            // O jogo pode ter acabado ou o usuário ter trocado de modo durante o fade.
            if app.mode.get() == UiMode::BigPicture && app.shell.borrow().ctx.leaving {
                app.apply(HostAction::Close);
            }
        });
    }
}

/// "Bem-vindo de volta" com o tempo jogado e o foco no jogo (o tempo já foi gravado por quem chama). Se o jogo
/// fechou logo ao abrir ou nem abriu, um aviso no lugar (SESSION-003).
pub fn welcome_back(shell: &mut Shell, id: &str, played: Duration, end: GameEnd) {
    shell.ctx.running = None;
    shell.close_game_layers();
    if shell.ctx.desktop {
        shell.open_route(crate::shell::desktop::ROUTE);
    }
    shell.focus_game(id);
    if end != GameEnd::Normal {
        let code = end
            .exit_code()
            .map(|c| format!(", código {c:#X}"))
            .unwrap_or_default();
        tracing::warn!(
            "{id} terminou fora do normal: {} ({played:?}{code})",
            end.key()
        );
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
    crate::navigation_feedback::sync(&s.ctx);
    if !s.ctx.dirty {
        return;
    }
    s.ctx.dirty = false;
    if let Some((id, played, end)) = s.ctx.welcome_back.take() {
        // Desktop: a janela minimizada durante o jogo volta quando ele fecha (ADR-0027).
        if s.ctx.desktop {
            restore(ui);
        }
        welcome_back(&mut s, &id, played, end);
    }
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
        "text-scale",
        (crate::navigation_feedback::text_size(&s.ctx.settings) as f32 / 100.).to_value(),
    );
    bridge::set_global(
        ui,
        "Ui",
        "reduce-transparency",
        s.ctx
            .settings
            .bool(crate::shell::ctx::keys::REDUCED_TRANSPARENCY, false)
            .to_value(),
    );
    for (name, key) in [
        ("silence", "accessibility.sound_silence"),
        ("max", "accessibility.sound_max"),
    ] {
        bridge::set_global(ui, "DeskText", name, s.ctx.t(key).to_value());
    }
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
    fn desktop_switches_windows_and_returns_from_the_minimized_game() {
        let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::thread::spawn(|| {
            use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
            struct Software;
            impl slint::platform::Platform for Software {
                fn create_window_adapter(
                    &self,
                ) -> Result<Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError>
                {
                    Ok(MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer))
                }
            }
            slint::platform::set_platform(Box::new(Software)).unwrap();
            struct DesktopHost;
            impl Host for DesktopHost {
                fn handle(&mut self, effect: &Effect, _: &mut Ctx) -> HostAction {
                    if *effect == Effect::ResumeGame {
                        HostAction::Minimize
                    } else {
                        HostAction::Stay
                    }
                }
            }
            i18n::set_lang(Lang::PtBr);
            let dir = tempfile::tempdir().unwrap();
            let mut ctx = build_ctx(DataPaths::at(dir.path()), Platform::simulated(), true);
            ctx.library.refresh();
            let builtin = theme::install_builtin(&ctx.paths.themes()).unwrap();
            let ui = theme::compile_desktop(&builtin).unwrap().create().unwrap();
            let mut shell = Shell::new(ctx);
            shell.enter_desktop();
            shell.click("game", 2);
            let id = shell.focused_game().unwrap();
            let app = App {
                ui: Rc::new(RefCell::new(ui)),
                shell: Rc::new(RefCell::new(shell)),
                host: Rc::new(RefCell::new(DesktopHost)),
                builtin,
                mode: Rc::new(std::cell::Cell::new(UiMode::Desktop)),
            };
            app.wire_ui();
            app.ui.borrow().show().unwrap();
            // O botão e F11 pedem o mesmo efeito; a troca preserva o jogo nos dois sentidos.
            app.shell.borrow_mut().click("desk-nav", 2);
            assert!(
                app.shell.borrow_mut().ctx.effects.remove(0)
                    == Effect::SwitchMode(UiMode::BigPicture)
            );
            app.switch_mode(UiMode::BigPicture);
            assert!(!app.shell.borrow().ctx.desktop);
            assert_eq!(app.shell.borrow().focused_game(), Some(id.clone()));
            app.switch_mode(UiMode::Desktop);
            assert!(app.shell.borrow().ctx.desktop);
            assert_eq!(app.shell.borrow().focused_game(), Some(id.clone()));
            assert!(
                app.shell
                    .borrow_mut()
                    .key(crate::shell::keys::F11, KeyModifiers::NONE)
            );
            assert_eq!(
                app.shell.borrow_mut().ctx.effects.remove(0),
                Effect::SwitchMode(UiMode::BigPicture)
            );

            app.shell.borrow_mut().ctx.running = Some(crate::shell::RunningGame {
                id: id.clone(),
                title: "Teste".into(),
                pid: Some(1234),
                started: None,
            });
            app.apply(HostAction::Launching);
            app.game_shown();
            assert!(!app.shell.borrow().ctx.launching);
            assert!(app.ui.borrow().window().is_minimized());
            assert!(
                app.ui.borrow().window().is_visible(),
                "a UI desktop continua viva"
            );
            app.toggle_control_center();
            assert!(app.shell.borrow().control_center_open());
            assert!(!app.ui.borrow().window().is_minimized());
            assert_eq!(
                app.ui.borrow().get_property("control-center").unwrap(),
                Value::Bool(true)
            );
            app.toggle_control_center();
            assert!(!app.shell.borrow().control_center_open());
            assert!(app.ui.borrow().window().is_minimized());
            app.toggle_control_center();
            app.shell.borrow_mut().click("cc-icon", 6);
            app.shell.borrow_mut().click("cc-panel", 0);
            assert!(app.shell.borrow().ctx.effects.contains(&Effect::EndGame));
            app.shell.borrow_mut().ctx.effects.clear();
            app.shell.borrow_mut().open_route("settings");
            for end in [GameEnd::Normal, GameEnd::Failed(3)] {
                {
                    let mut s = app.shell.borrow_mut();
                    s.ctx.welcome_back = Some((id.clone(), Duration::from_secs(120), end));
                    s.ctx.dirty = true;
                }
                app.redraw();
                assert!(!app.ui.borrow().window().is_minimized());
                assert!(app.shell.borrow().ctx.running.is_none());
                assert!(!app.shell.borrow().control_center_open());
                assert_eq!(app.shell.borrow().page().kind(), "desk-library");
                assert_eq!(app.shell.borrow().focused_game(), Some(id.clone()));
                assert_eq!(app.shell.borrow().has_layers(), end != GameEnd::Normal);
                app.shell.borrow_mut().close_layers();
            }
            app.ui.borrow().hide().unwrap();
        })
        .join()
        .unwrap();
    }

    #[test]
    fn reduced_motion_removes_the_leave_fade_and_every_theme_animation_follows_it() {
        let mut settings = crate::shell::Settings::new(None);
        assert_eq!(leave_fade(&settings), Duration::from_millis(450));
        settings.set(crate::shell::ctx::keys::REDUCED_MOTION, "true");
        assert_eq!(leave_fade(&settings), Duration::ZERO);
        // Duração fixa num `animate` escaparia de "Reduzir animações": todas vêm dos tokens `Ui.*-time`.
        for file in [
            include_str!("../theme/main.slint"),
            include_str!("../theme/widgets.slint"),
            include_str!("../theme/overlay.slint"),
            include_str!("../theme/desktop.slint"),
        ] {
            for line in file.lines().filter(|l| l.contains("animate ")) {
                assert!(line.contains("duration: Ui."), "{line}");
            }
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
                guid: String::new(),
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
        assert!(scan_into(&found, "", true, None, Default::default()).is_empty());
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

        welcome_back(
            &mut shell,
            &game.id,
            Duration::from_secs(3),
            GameEnd::Quick(None),
        );
        assert!(shell.ctx.toast.is_none());
        assert!(shell.has_layers());
        use crate::shell::Layer;
        let warning =
            crate::shell::dialogs::game_ended_badly(&shell.ctx, &game.id, GameEnd::Quick(None));
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

    #[test]
    fn the_warning_shows_the_exit_code_and_errors_warn_even_after_ten_seconds() {
        use crate::shell::Layer;
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        i18n::set_lang(Lang::PtBr);
        let dir = tempfile::tempdir().unwrap();
        let ctx = build_ctx(DataPaths::at(dir.path()), Platform::simulated(), true);
        let mut shell = Shell::new(ctx);
        let game = shell.ctx.library.games()[0].clone();
        let dialog = |shell: &Shell, end| {
            crate::shell::dialogs::game_ended_badly(&shell.ctx, &game.id, end).render(&shell.ctx)
        };

        let quick = dialog(&shell, GameEnd::Quick(Some(0xC000_0135)));
        assert_eq!(
            quick.title,
            format!("{} fechou logo depois de abrir", game.title)
        );
        assert!(
            quick
                .text
                .starts_with("Código de saída 0xC0000135 (3221225781): uma DLL"),
            "{}",
            quick.text
        );
        assert!(
            quick
                .text
                .contains("O jogo fechou sozinho em poucos segundos")
        );

        // Depois dos 10 s, só avisa com código de erro.
        welcome_back(
            &mut shell,
            &game.id,
            Duration::from_secs(600),
            GameEnd::Failed(0xC000_0005),
        );
        assert!(shell.has_layers());
        assert!(shell.ctx.toast.is_none());
        let failed = dialog(&shell, GameEnd::Failed(0xC000_0005));
        assert_eq!(failed.title, format!("{} fechou com um erro", game.title));
        assert!(
            failed.text.contains("0xC0000005 (3221225477)"),
            "{}",
            failed.text
        );
        assert!(failed.text.contains("memória"), "{}", failed.text);

        // Sem código (jogo aberto pela loja): o texto de antes, sem número.
        let store = dialog(&shell, GameEnd::Quick(None));
        assert!(!store.text.contains("Código"), "{}", store.text);
    }

    #[test]
    fn the_interface_fills_in_the_exit_code_of_the_game_it_opened() {
        use std::sync::Arc;
        use tako_core::devices::fake::FakeProcesses;
        let dir = tempfile::tempdir().unwrap();
        let fake = Arc::new(FakeProcesses::default());
        let platform = Platform {
            processes: fake.clone(),
            ..Platform::simulated()
        };
        let ctx = build_ctx(DataPaths::at(dir.path()), platform, true);
        let fill = crate::session::with_local_exit_code;
        fake.exit_with(1234, 3);
        // O supervisor (Linux e macOS) não tinha o código; a interface, que abriu o jogo, tem.
        assert_eq!(
            fill(&ctx, Some(1234), GameEnd::Quick(None)),
            GameEnd::Quick(Some(3))
        );
        // O do supervisor vale; sem processo, nada muda.
        fake.exit_with(1234, 3);
        assert_eq!(
            fill(&ctx, Some(1234), GameEnd::Quick(Some(7))),
            GameEnd::Quick(Some(7))
        );
        assert_eq!(fill(&ctx, None, GameEnd::Quick(None)), GameEnd::Quick(None));
        assert_eq!(fill(&ctx, Some(1), GameEnd::Normal), GameEnd::Normal);
    }
}
