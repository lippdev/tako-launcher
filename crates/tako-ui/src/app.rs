//! Laço da interface: carrega o tema, monta o contexto, liga entrada (controle e teclado) e desenha quando
//! algo muda. As tarefas longas (ler as lojas, metadados) rodam em threads e voltam pelo laço do Slint.

use crate::gamepad::{PadEvent, Pads};
use crate::shell::keys::{self, KeyAction};
use crate::shell::{Cmd, Ctx, Effect, InputKind, Settings, Shell};
use crate::{bridge, images, theme, values::ToValue};
use slint::ComponentHandle;
use slint_interpreter::{ComponentInstance, Value};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tako_core::i18n::{self, Lang};
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
    pub safe_mode: bool,
}

/// O que o app (binário) faz com os pedidos da interface que saem dela: abrir jogos, sair, atualizar.
pub trait Host {
    /// Devolve `true` se a interface deve fechar agora (o jogo vai rodar com o supervisor).
    fn handle(&mut self, effect: &Effect, ctx: &mut Ctx) -> bool;
}

pub struct App {
    pub ui: ComponentInstance,
    pub shell: Rc<RefCell<Shell>>,
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
    let paths_for_art = paths.clone();
    let _ = std::fs::create_dir_all(&paths.root);
    let database = (!simulate).then(|| TakoDatabase::new(paths.database()));
    let settings = Settings::new(database.clone().map(db::SettingsStore::new));
    let lang = settings
        .get(crate::shell::ctx::keys::LANGUAGE)
        .and_then(Lang::from_code)
        .unwrap_or_else(Lang::from_env);
    i18n::set_lang(lang);

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
        art: crate::art::cache(&paths_for_art),
        dirty: true,
    };
    ctx.sync_collections();
    ctx
}

thread_local! {
    static SHELL: RefCell<std::rc::Weak<RefCell<Shell>>> = const { RefCell::new(std::rc::Weak::new()) };
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
    // O laço do Slint roda na thread da UI: o resultado volta para a mesma `Shell` por um thread-local.
    SHELL.with(|s| *s.borrow_mut() = Rc::downgrade(shell));
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
                if let Some(shell) = SHELL.with(|s| s.borrow().upgrade()) {
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

impl App {
    /// Carrega o tema (o escolhido ou o padrão) e liga tudo. `host` recebe os pedidos de sessão e saída.
    pub fn new(ctx: Ctx, opts: &UiOptions, host: Rc<RefCell<dyn Host>>) -> Result<App, String> {
        let builtin =
            theme::install_builtin(&ctx.paths.themes()).map_err(|e| format!("tema padrão: {e}"))?;
        let def = load_theme(&ctx, &builtin)?;
        let ui = def.create().map_err(|e| e.to_string())?;
        let mut shell = Shell::new(ctx);
        shell.ctx.safe_mode = opts.safe_mode;
        if let Some((id, played)) = &opts.after_game {
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
                            &tako_core::presentation::game_text::session_played(
                                i18n::lang(),
                                *played,
                            ),
                        ),
                    ],
                )
            };
            shell.ctx.toast(text);
        }
        let shell = Rc::new(RefCell::new(shell));
        let app = App { ui, shell };
        app.wire(host);
        Ok(app)
    }

    fn wire(&self, host: Rc<RefCell<dyn Host>>) {
        let shell = self.shell.clone();
        let weak = self.ui.as_weak();
        let render_now = {
            let shell = shell.clone();
            let weak = weak.clone();
            let host = host.clone();
            Rc::new(move || {
                if let Some(ui) = weak.upgrade() {
                    pump_effects(&shell, &host, &ui);
                    render(&shell, &ui);
                }
            })
        };

        // Imagens: ao chegar uma, redesenha.
        {
            let shell = shell.clone();
            let resolver = crate::art::resolver(&shell.borrow().ctx);
            let r = render_now.clone();
            images::init(resolver, 2, move || {
                shell.borrow_mut().ctx.dirty = true;
                r();
            });
        }

        let on_cmd = {
            let shell = shell.clone();
            let r = render_now.clone();
            Rc::new(move |cmd: Cmd| {
                {
                    let mut s = shell.borrow_mut();
                    match cmd {
                        Cmd::PrevSection => s.section(false),
                        Cmd::NextSection => s.section(true),
                        c => s.handle(c),
                    }
                }
                r();
            })
        };

        {
            let shell = shell.clone();
            let r = render_now.clone();
            let on_cmd = on_cmd.clone();
            let _ = self.ui.set_callback("key", move |args| {
                let text = arg_str(args, 0);
                let ctrl = matches!(args.get(2), Some(Value::Bool(true)));
                let alt = matches!(args.get(4), Some(Value::Bool(true)));
                let action = {
                    let mut s = shell.borrow_mut();
                    s.ctx.input = InputKind::Keyboard;
                    keys::map(&text, ctrl, alt, s.typing())
                };
                match action {
                    KeyAction::Cmd(c) => on_cmd(c),
                    KeyAction::Text(t) => {
                        shell.borrow_mut().text(&t);
                        r();
                    }
                    KeyAction::Erase => {
                        shell.borrow_mut().erase();
                        r();
                    }
                    KeyAction::Ignore => return Value::Bool(false),
                }
                Value::Bool(true)
            });
        }
        {
            let shell = shell.clone();
            let r = render_now.clone();
            let _ = self.ui.set_callback("clicked", move |args| {
                shell
                    .borrow_mut()
                    .click(&arg_str(args, 0), arg_num(args, 1));
                r();
                Value::Void
            });
        }
        {
            let shell = shell.clone();
            let r = render_now.clone();
            let _ = self.ui.set_callback("hovered", move |args| {
                shell
                    .borrow_mut()
                    .hover(&arg_str(args, 0), arg_num(args, 1));
                r();
                Value::Void
            });
        }
        {
            let shell = shell.clone();
            let r = render_now.clone();
            let _ = self.ui.set_callback("hint-clicked", move |args| {
                shell.borrow_mut().hint_clicked(arg_num(args, 0));
                r();
                Value::Void
            });
        }
        let _ = self.ui.set_callback("key-released", |_| Value::Void);

        // Controle: o SDL é lido a cada 16 ms enquanto a interface está aberta (como o XInput do C#).
        let pads = Pads::open()
            .map_err(|e| tracing::warn!("controles indisponíveis: {e}"))
            .ok();
        let pads = Rc::new(RefCell::new(pads));
        let interp = Rc::new(RefCell::new(GamepadInterpreter::new()));
        let started = Instant::now();
        let timer = slint::Timer::default();
        {
            let shell = shell.clone();
            let r = render_now.clone();
            let on_cmd = on_cmd.clone();
            let mut last_clock = String::new();
            let weak = weak.clone();
            timer.start(
                slint::TimerMode::Repeated,
                Duration::from_millis(16),
                move || {
                    let mut fired = Vec::new();
                    if let Some(p) = pads.borrow_mut().as_mut() {
                        for ev in p.poll() {
                            let mut s = shell.borrow_mut();
                            match ev {
                                PadEvent::Connected { name, family } => {
                                    s.ctx.pad = family;
                                    s.ctx.pad_name = name;
                                    s.ctx.input = InputKind::Gamepad;
                                }
                                PadEvent::Disconnected { .. } => {}
                            }
                            s.ctx.pads_connected = p.count();
                            s.ctx.dirty = true;
                        }
                        let st = p.state();
                        let snap = GamepadSnapshot::new(
                            GamepadButtons(st.buttons),
                            st.left_x as f64,
                            -st.left_y as f64,
                        );
                        fired = interp.borrow_mut().update(
                            snap,
                            started.elapsed(),
                            GamepadButtons::NONE,
                        );
                        if !fired.is_empty() {
                            let mut s = shell.borrow_mut();
                            s.ctx.input = InputKind::Gamepad;
                            s.ctx.pad = p.family();
                            s.ctx.pad_name = p.name().to_owned();
                        }
                    }
                    for c in fired {
                        on_cmd(cmd_of(c));
                    }
                    // Relógio e avisos que expiram.
                    let clock = chrono::Local::now().format("%H:%M").to_string();
                    let toast_expired = shell
                        .borrow()
                        .ctx
                        .toast
                        .as_ref()
                        .is_some_and(|(_, at)| at.elapsed() >= crate::shell::TOAST_TIME);
                    if clock != last_clock {
                        if let Some(ui) = weak.upgrade() {
                            bridge::set(&ui, "clock", clock.to_value());
                        }
                        last_clock = clock;
                    }
                    if toast_expired || shell.borrow().ctx.dirty {
                        shell.borrow_mut().ctx.dirty = true;
                        r();
                    }
                },
            );
        }
        std::mem::forget(timer);
        render_now();
    }

    pub fn run(&self) -> Result<(), String> {
        self.ui.show().map_err(|e| e.to_string())?;
        rescan(&self.shell);
        slint::run_event_loop_until_quit().map_err(|e| e.to_string())
    }
}

fn load_theme(
    ctx: &Ctx,
    builtin: &std::path::Path,
) -> Result<slint_interpreter::ComponentDefinition, String> {
    let selected = tako_themes::selected(&ctx.paths.themes());
    if let Some(id) = selected.filter(|_| !ctx.safe_mode) {
        let dir = ctx.paths.themes().join(&id);
        let entry = tako_themes::validate_package(&dir).map(|m| dir.join(m.entry));
        match entry {
            Ok(entry) => match theme::compile(&entry, builtin) {
                Ok(def) => return Ok(def),
                Err(e) => tracing::error!("tema {id} não compilou, usando o padrão: {e}"),
            },
            Err(p) => tracing::error!("tema {id} inválido, usando o padrão: {p:?}"),
        }
    }
    theme::compile(&builtin.join("main.slint"), builtin).map_err(|e| e.to_string())
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

fn pump_effects(shell: &Rc<RefCell<Shell>>, host: &Rc<RefCell<dyn Host>>, ui: &ComponentInstance) {
    loop {
        let effect = {
            let mut s = shell.borrow_mut();
            if s.ctx.effects.is_empty() {
                return;
            }
            s.ctx.effects.remove(0)
        };
        if effect == Effect::Rescan || effect == Effect::ReloadLibrary {
            rescan(shell);
            continue;
        }
        let close = {
            let mut s = shell.borrow_mut();
            host.borrow_mut().handle(&effect, &mut s.ctx)
        };
        if close {
            let _ = ui.hide();
            let _ = slint::quit_event_loop();
            return;
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
