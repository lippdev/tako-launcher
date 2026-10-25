//! Processo da interface (`tako --web-ui`, WEB-02b): CEF Views no Windows. Objetos CEF de janela ficam
//! exclusivamente na thread UI. Não há `AppService` aqui: a ponte vai e volta pelo transporte de `ipc` até o
//! processo do núcleo, que abriu este.

use crate::bridge;
use crate::ipc::{self, CoreMsg, Page, UiMsg};
use crate::resources::{CSP, Resource, Resources};
use crate::target::{APP_URL, dev_origin};
use cef::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use tako_app::protocol::WindowAction;
use tako_core::paths::DataPaths;

const POST: &str = "tako.post";
const CONTEXT: &str = "tako.context";

/// Depois do fim do transporte, quanto esperar a janela fechar antes de sair à força (sem órfão).
const ORPHAN_GRACE: Duration = Duration::from_secs(5);

/// O núcleo pediu para fechar (ou terminou): o fechamento da última janela não é do usuário.
static CLOSING: AtomicBool = AtomicBool::new(false);
/// A janela não pôde ser criada: o processo sai com erro, e o núcleo trata como queda.
static FAILED: AtomicBool = AtomicBool::new(false);

fn cef_text(value: CefStringUserfree) -> String {
    CefString::from(&value).to_string()
}

thread_local! {
    // Só a thread UI acessa objetos Views. As tarefas vindas do núcleo carregam apenas IDs e texto.
    static WINDOWS: RefCell<HashMap<i32, BrowserEntry>> = RefCell::new(HashMap::new());
}

struct BrowserEntry {
    browser: Browser,
    page: Option<Page>,
}

#[derive(Clone)]
struct Config {
    url: String,
    origin: String,
    resources: Arc<Resources>,
    /// Para o núcleo, pela saída padrão. Limite de 64 mensagens na fila da página.
    requests: mpsc::SyncSender<UiMsg>,
    processes: Arc<dyn tako_core::devices::ProcessLauncher>,
}

fn trusted(url: &str, origin: &str) -> bool {
    let normalized = url.to_ascii_lowercase();
    normalized
        .strip_prefix(origin)
        .is_some_and(|rest| rest.starts_with(['/', '?', '#']) || rest.is_empty())
}

fn renderer_origin() -> String {
    if cfg!(debug_assertions)
        && let Ok(url) = std::env::var("TAKO_WEB_DEV_URL")
        && let Some(origin) = dev_origin(&url)
    {
        return origin;
    }
    crate::target::APP_ORIGIN.into()
}

pub fn execute_subprocess() -> i32 {
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);
    let mut app = TakoCefApp::new(None, Arc::new(AtomicU64::new(0)));
    execute_process(
        Some(args::Args::new().as_main_args()),
        Some(&mut app),
        std::ptr::null_mut(),
    )
}

pub fn run(paths: DataPaths) -> Result<(), String> {
    crate::check_runtime(&std::env::current_exe().map_err(|e| e.to_string())?)?;
    let target = crate::target::resolve()?;
    let resources = Arc::new(Resources::new(target.root, &paths));
    if cfg!(debug_assertions)
        && let Some(art) = std::env::var_os("TAKO_WEB_TEST_ART")
    {
        resources.install_test_art(&PathBuf::from(art))?;
    }
    let (requests, outgoing) = mpsc::sync_channel::<UiMsg>(64);
    // Um só escritor na saída padrão: as mensagens chegam ao núcleo na ordem em que a página mandou.
    std::thread::spawn(move || {
        let mut out = std::io::stdout().lock();
        for message in outgoing {
            if ipc::write_message(&mut out, &message).is_err() {
                break;
            }
        }
    });
    let platform = tako_platform::Platform::native(&paths.machine);
    let config = Config {
        processes: platform.processes.clone(),
        url: target.url,
        origin: target.origin,
        resources,
        requests,
    };
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);
    let mut app = TakoCefApp::new(Some(config), Arc::new(AtomicU64::new(0)));
    let cache = paths.cache().join("cef");
    std::fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
    let cache = std::path::absolute(cache).map_err(|e| e.to_string())?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let settings = cef_settings(
        &exe,
        &cache,
        &paths.logs().join("cef.log"),
        cfg!(debug_assertions),
    );
    if initialize(
        Some(args::Args::new().as_main_args()),
        Some(&settings),
        Some(&mut app),
        std::ptr::null_mut(),
    ) != 1
    {
        return Err("CEF não inicializou; confira runtime e logs/cef.log".into());
    }
    tracing::info!(
        pid = std::process::id(),
        "CEF inicializado no processo da interface"
    );
    std::thread::spawn(read_core);
    run_message_loop();
    drop(app);
    shutdown();
    tracing::info!("CEF finalizado");
    if FAILED.load(Ordering::Relaxed) {
        return Err("a janela da interface não abriu; veja logs/cef.log".into());
    }
    Ok(())
}

/// Entrada padrão ← núcleo. O fim dela é o núcleo que terminou: fecha a janela e, se não fechar, sai.
fn read_core() {
    let result =
        ipc::read_messages::<CoreMsg>(std::io::stdin().lock(), ipc::MAX_CORE_LINE, |message| {
            match message {
                CoreMsg::Deliver {
                    page,
                    messages,
                    actions,
                } => post_delivery(page, messages, actions),
                CoreMsg::Close => post_close(),
            }
            true
        });
    if let Err(error) = result {
        tracing::warn!(%error, "entrada do núcleo");
    }
    tracing::info!("transporte do núcleo fechado; fechando a interface");
    post_close();
    std::thread::sleep(ORPHAN_GRACE);
    tracing::warn!("a interface não fechou depois do fim do núcleo; saindo");
    std::process::exit(2);
}

fn post_close() {
    CLOSING.store(true, Ordering::Relaxed);
    let mut task = CloseAll::new();
    if post_task(ThreadId::UI, Some(&mut task)) == 0 {
        tracing::warn!("fila UI encerrada");
    }
}

/// Em release, `command_line_args_disabled` faz o próprio CEF descartar toda opção externa do Chromium antes de
/// aplicar estes ajustes. O debug aceita opções externas para DevTools e diagnóstico.
fn cef_settings(exe: &Path, cache: &Path, log: &Path, debug: bool) -> Settings {
    Settings {
        no_sandbox: 1,
        command_line_args_disabled: i32::from(!debug),
        browser_subprocess_path: CefString::from(exe.to_string_lossy().as_ref()),
        // absolute preserva UNC e evita o prefixo estendido de canonicalize, incompatível com LevelDB.
        root_cache_path: CefString::from(cache.to_string_lossy().as_ref()),
        log_file: CefString::from(log.to_string_lossy().as_ref()),
        log_severity: if debug {
            LogSeverity::WARNING
        } else {
            LogSeverity::DISABLE
        },
        ..Default::default()
    }
}

/// Só acrescenta. Quando o CEF chama este callback, a linha já tem `--no-sandbox` e os caminhos de
/// `cef_settings`; limpá-la impede o renderer de subir (página em branco no release).
fn configure_command_line(line: Option<&mut CommandLine>) {
    let Some(line) = line else {
        return;
    };
    line.append_switch_with_value(
        Some(&CefString::from("disable-features")),
        Some(&CefString::from("Gamepad,WebHID,WebUSB")),
    );
}

fn initial_size(display: Option<Display>) -> Size {
    let area = display.map(|display| display.work_area()).unwrap_or(Rect {
        width: 1600,
        height: 900,
        ..Default::default()
    });
    Size {
        width: area.width.min(1600),
        height: area.height.min(900),
    }
}

fn browser_created(browser: Option<&mut Browser>) {
    if let Some(browser) = browser {
        WINDOWS.with(|windows| {
            windows.borrow_mut().insert(
                browser.identifier(),
                BrowserEntry {
                    browser: browser.clone(),
                    page: None,
                },
            )
        });
        tracing::info!(
            browser = browser.identifier(),
            "navegador CEF aberto (inclui DevTools)"
        );
    }
}

fn browser_closed(config: &Config, browser: Option<&mut Browser>) {
    let Some(browser) = browser else {
        return;
    };
    WINDOWS.with(|windows| windows.borrow_mut().remove(&browser.identifier()));
    let _ = config.requests.try_send(UiMsg::Closed {
        browser: browser.identifier(),
    });
    tracing::info!(browser = browser.identifier(), "navegador CEF fechado");
    if WINDOWS.with(|windows| windows.borrow().is_empty()) {
        // Sem pedido do núcleo, quem fechou foi o usuário: o núcleo encerra o app. A fila espera vaga (o
        // escritor drena sempre) e falha na hora se o núcleo já terminou.
        if !CLOSING.load(Ordering::Relaxed) {
            let _ = config.requests.send(UiMsg::UserClosed);
        }
        quit_message_loop();
    }
}

fn accept_post(
    config: &Config,
    browser: Option<&mut Browser>,
    frame: Option<&mut Frame>,
    source: ProcessId,
    message: Option<&mut ProcessMessage>,
) -> i32 {
    let (Some(browser), Some(frame), Some(message)) = (browser, frame, message) else {
        return 0;
    };
    if source != ProcessId::RENDERER
        || frame.is_main() == 0
        || !trusted(&cef_text(frame.url()), &config.origin)
    {
        return 0;
    }
    let Some(args) = message.argument_list() else {
        return 0;
    };
    let name = cef_text(message.name());
    let (text, generation) = match name.as_str() {
        POST if args.size() == 2 => (cef_text(args.string(0)), cef_text(args.string(1))),
        CONTEXT if args.size() == 1 => (String::new(), cef_text(args.string(0))),
        _ => return 0,
    };
    if text.len() > bridge::MAX_MESSAGE_BYTES || generation.is_empty() || generation.len() > 64 {
        return 0;
    }
    let page = Page {
        browser: browser.identifier(),
        frame: cef_text(frame.identifier()),
        generation,
    };
    let current_frame = browser
        .main_frame()
        .map(|frame| cef_text(frame.identifier()));
    if current_frame.as_deref() != Some(&page.frame) {
        return 0;
    }
    WINDOWS.with(|windows| {
        if let Some(entry) = windows.borrow_mut().get_mut(&page.browser) {
            entry.page = Some(page.clone());
        }
    });
    let command = if name == CONTEXT {
        UiMsg::Page { page }
    } else {
        UiMsg::Request { page, text }
    };
    if let Err(error) = config.requests.try_send(command) {
        tracing::warn!("fila da ponte indisponível");
        let command = match error {
            mpsc::TrySendError::Full(command) | mpsc::TrySendError::Disconnected(command) => {
                command
            }
        };
        if let UiMsg::Request { page, text } = command {
            let id = tako_app::protocol::parse_request(&text)
                .map(|request| request.id)
                .unwrap_or_else(|(id, _)| id);
            let response = tako_app::protocol::Response::err(
                id,
                tako_app::protocol::BridgeError::new(
                    tako_app::protocol::ErrorCode::Internal,
                    "fila da ponte indisponível",
                ),
            );
            if let Ok(text) = serde_json::to_string(&response) {
                deliver(&page, &[text], &[]);
            }
        }
    }
    1
}

fn install_bridge(frame: Option<&mut Frame>, context: Option<&mut V8Context>, counter: &AtomicU64) {
    let (Some(frame), Some(context)) = (frame, context) else {
        return;
    };
    if frame.is_main() == 0 || !trusted(&cef_text(frame.url()), &renderer_origin()) {
        return;
    }
    let Some(global) = context.global() else {
        return;
    };
    let Some(mut host) = v8_value_create_object(None, None) else {
        return;
    };
    let generation = format!(
        "{}:{}",
        std::process::id(),
        counter.fetch_add(1, Ordering::Relaxed)
    );
    let mut handler = Post::new(generation.clone());
    let Some(mut function) =
        v8_value_create_function(Some(&CefString::from("post")), Some(&mut handler))
    else {
        return;
    };
    let readonly = sys::cef_v8_propertyattribute_t::V8_PROPERTY_ATTRIBUTE_READONLY.into();
    host.set_value_bykey(
        Some(&CefString::from("post")),
        Some(&mut function),
        readonly,
    );
    global.set_value_bykey(
        Some(&CefString::from("takoHost")),
        Some(&mut host),
        readonly,
    );
    if let Some(mut value) = v8_value_create_string(Some(&CefString::from(generation.as_str()))) {
        global.set_value_bykey(
            Some(&CefString::from("__takoGeneration")),
            Some(&mut value),
            readonly,
        );
    }
    if let Some(mut message) = process_message_create(Some(&CefString::from(CONTEXT)))
        && let Some(args) = message.argument_list()
    {
        args.set_string(0, Some(&CefString::from(generation.as_str())));
        frame.send_process_message(ProcessId::BROWSER, Some(&mut message));
    }
}

fn renderer_post(
    generation: &str,
    arguments: Option<&[Option<V8Value>]>,
    exception: Option<&mut CefString>,
) -> i32 {
    let input = arguments
        .filter(|args| args.len() == 1)
        .and_then(|args| args[0].as_ref())
        .filter(|value| value.is_string() != 0);
    let Some(input) = input else {
        if let Some(exception) = exception {
            *exception = CefString::from("post exige uma string JSON");
        }
        return 1;
    };
    let text = cef_text(input.string_value());
    if text.len() > bridge::MAX_MESSAGE_BYTES {
        if let Some(exception) = exception {
            *exception = CefString::from("mensagem acima de 1 MiB");
        }
        return 1;
    }
    let Some(frame) = v8_context_get_current_context().and_then(|context| context.frame()) else {
        return 1;
    };
    if frame.is_main() == 0 || !trusted(&cef_text(frame.url()), &renderer_origin()) {
        return 1;
    }
    if let Some(mut message) = process_message_create(Some(&CefString::from(POST)))
        && let Some(args) = message.argument_list()
    {
        args.set_string(0, Some(&CefString::from(text.as_str())));
        args.set_string(1, Some(&CefString::from(generation)));
        frame.send_process_message(ProcessId::BROWSER, Some(&mut message));
    }
    1
}

fn deliver_script(generation: &str, messages: &[String]) -> String {
    let generation = serde_json::json!(generation).to_string();
    let messages = serde_json::json!(messages).to_string();
    format!("if(window.__takoGeneration==={generation}){{for(const message of {messages})window.__takoDeliver?.(message);}}")
        .replace('\u{2028}', "\\u2028").replace('\u{2029}', "\\u2029")
}

fn deliver(page: &Page, messages: &[String], actions: &[WindowAction]) {
    let browser = WINDOWS.with(|windows| {
        windows
            .borrow()
            .get(&page.browser)
            .filter(|entry| entry.page.as_ref() == Some(page))
            .map(|entry| entry.browser.clone())
    });
    let Some(mut browser) = browser else {
        return;
    };
    let Some(frame) = browser
        .main_frame()
        .filter(|frame| cef_text(frame.identifier()) == page.frame)
    else {
        return;
    };
    frame.execute_java_script(
        Some(&CefString::from(
            deliver_script(&page.generation, messages).as_str(),
        )),
        Some(&CefString::from(APP_URL)),
        1,
    );
    for action in actions {
        window_action(&mut browser, *action);
    }
}

fn is_external(url: &str, origin: &str) -> bool {
    let normalized = url.to_ascii_lowercase();
    (normalized.starts_with("http://") || normalized.starts_with("https://"))
        && !trusted(url, origin)
}

fn open_external(url: &str, config: &Config) {
    if is_external(url, &config.origin)
        && let Err(error) = config.processes.launch(url, None, None)
    {
        tracing::warn!(%error, "não foi possível abrir o link externo");
    }
}

fn before_browse(
    config: &Config,
    browser: Option<&mut Browser>,
    frame: Option<&mut Frame>,
    request: Option<&mut Request>,
) -> i32 {
    if !frame.is_some_and(|frame| frame.is_main() != 0) {
        return 1;
    }
    let Some(request) = request else {
        return 1;
    };
    let url = cef_text(request.url());
    if trusted(&url, &config.origin) {
        if let Some(browser) = browser {
            WINDOWS.with(|windows| {
                if let Some(entry) = windows.borrow_mut().get_mut(&browser.identifier()) {
                    entry.page = None;
                }
            });
        }
        0
    } else {
        open_external(&url, config);
        1
    }
}

fn handle_key(config: &Config, browser: Option<&mut Browser>, event: Option<&KeyEvent>) -> i32 {
    let modifiers = (sys::cef_event_flags_t::EVENTFLAG_CONTROL_DOWN.0
        | sys::cef_event_flags_t::EVENTFLAG_SHIFT_DOWN.0) as u32;
    if cfg!(debug_assertions)
        && event.is_some_and(|event| {
            event.type_ == KeyEventType::RAWKEYDOWN
                && (event.windows_key_code == 123
                    || (event.windows_key_code == 73 && event.modifiers & modifiers == modifiers))
        })
        && let Some(host) = browser.and_then(|browser| browser.host())
    {
        let mut client = DevToolsClient::new(config.clone());
        host.show_dev_tools(
            Some(&WindowInfo::default()),
            Some(&mut client),
            Some(&BrowserSettings::default()),
            None,
        );
        return 1;
    }
    0
}

fn reject_file_drag(data: Option<&mut DragData>) -> i32 {
    let rejected = data.is_some_and(|data| data.is_file() != 0);
    if rejected {
        tracing::info!("arquivo arrastado recusado");
    }
    i32::from(rejected)
}

wrap_jsdialog_handler! {
    struct Dialogs;
    impl JsdialogHandler {
        fn on_jsdialog(&self, _browser: Option<&mut Browser>, _origin_url: Option<&CefString>, _dialog_type: JsdialogType, _message_text: Option<&CefString>, _default_prompt_text: Option<&CefString>, _callback: Option<&mut JsdialogCallback>, suppress_message: Option<&mut i32>) -> i32 {
            if let Some(suppress) = suppress_message { *suppress = 1; }
            0
        }
        fn on_before_unload_dialog(&self, _browser: Option<&mut Browser>, _message_text: Option<&CefString>, _is_reload: i32, callback: Option<&mut JsdialogCallback>) -> i32 {
            if let Some(callback) = callback { callback.cont(1, None); }
            1
        }
    }
}

fn post_delivery(page: Page, messages: Vec<String>, actions: Vec<WindowAction>) {
    let mut task = Deliver::new(page, messages, actions);
    if post_task(ThreadId::UI, Some(&mut task)) == 0 {
        tracing::warn!("fila UI encerrada");
    }
}

/// Pedido do núcleo (ou fim do transporte): fecha todos os navegadores; o último fecha o laço de mensagens.
fn close_all() {
    let browsers = WINDOWS.with(|windows| {
        windows
            .borrow()
            .values()
            .map(|entry| entry.browser.clone())
            .collect::<Vec<_>>()
    });
    if browsers.is_empty() {
        quit_message_loop();
    }
    for browser in browsers {
        if let Some(host) = browser.host() {
            host.close_browser(1);
        }
    }
}

wrap_task! {
    struct CloseAll;
    impl Task { fn execute(&self) { close_all(); } }
}

wrap_app! {
    struct TakoCefApp { config: Option<Config>, generation: Arc<AtomicU64> }
    impl App {
        fn on_register_custom_schemes(&self, registrar: Option<&mut SchemeRegistrar>) {
            if let Some(registrar) = registrar {
                let options = sys::cef_scheme_options_t::CEF_SCHEME_OPTION_STANDARD as i32
                    | sys::cef_scheme_options_t::CEF_SCHEME_OPTION_SECURE as i32
                    | sys::cef_scheme_options_t::CEF_SCHEME_OPTION_CORS_ENABLED as i32
                    | sys::cef_scheme_options_t::CEF_SCHEME_OPTION_FETCH_ENABLED as i32;
                for scheme in ["tako-app", "tako-img"] {
                    registrar.add_custom_scheme(Some(&CefString::from(scheme)), options);
                }
            }
        }
        fn on_before_command_line_processing(&self, _process_type: Option<&CefString>, command_line: Option<&mut CommandLine>) {
            configure_command_line(command_line);
        }
        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            self.config.clone().map(BrowserProcess::new)
        }
        fn render_process_handler(&self) -> Option<RenderProcessHandler> { Some(Renderer::new(self.generation.clone())) }
    }
}

wrap_browser_process_handler! {
    struct BrowserProcess { config: Config }
    impl BrowserProcessHandler {
        fn on_context_initialized(&self) {
            for scheme in ["tako-app", "tako-img"] {
                let mut factory = ResourceFactory::new(self.config.resources.clone());
                register_scheme_handler_factory(Some(&CefString::from(scheme)), None, Some(&mut factory));
            }
            // Fechamento pedido antes de o CEF ficar pronto: não abre a janela.
            if CLOSING.load(Ordering::Relaxed) {
                quit_message_loop();
            } else {
                open_window(&self.config);
            }
        }
    }
}

fn open_window(config: &Config) {
    let mut client = HostClient::new(config.clone());
    let mut view_delegate = WebViewDelegate::new();
    let Some(view) = browser_view_create(
        Some(&mut client),
        Some(&CefString::from(config.url.as_str())),
        Some(&BrowserSettings::default()),
        None,
        None,
        Some(&mut view_delegate),
    ) else {
        tracing::error!("CEF não criou BrowserView");
        FAILED.store(true, Ordering::Relaxed);
        quit_message_loop();
        return;
    };
    let mut delegate = HostWindow::new(RefCell::new(Some(view)), false);
    if window_create_top_level(Some(&mut delegate)).is_none() {
        tracing::error!("CEF não criou a janela");
        FAILED.store(true, Ordering::Relaxed);
        quit_message_loop();
    }
}

fn window_icon(png_1x: &[u8], png_2x: &[u8]) -> Option<Image> {
    let Some(image) = image_create() else {
        tracing::error!("CEF não criou a imagem do ícone do Tako");
        return None;
    };
    for (scale, png) in [(1.0, png_1x), (2.0, png_2x)] {
        if image.add_png(scale, Some(png)) == 0 {
            tracing::error!(scale, "CEF não decodificou o ícone do Tako");
            return None;
        }
    }
    Some(image)
}

fn set_window_icons(window: &Window) {
    // PNGs extraídos de packaging/tako.ico, nas escalas 1x/2x, sem ampliar a arte original.
    if let Some(mut icon) = window_icon(
        include_bytes!("../assets/tako-16.png"),
        include_bytes!("../assets/tako-32.png"),
    ) {
        window.set_window_icon(Some(&mut icon));
    }
    if let Some(mut icon) = window_icon(
        include_bytes!("../assets/tako-128.png"),
        include_bytes!("../assets/tako-256.png"),
    ) {
        window.set_window_app_icon(Some(&mut icon));
    }
}

wrap_browser_view_delegate! {
    struct WebViewDelegate {}
    impl ViewDelegate {}
    impl BrowserViewDelegate {
        fn browser_runtime_style(&self) -> RuntimeStyle { RuntimeStyle::ALLOY }
        fn on_popup_browser_view_created(&self, _browser_view: Option<&mut BrowserView>, popup_browser_view: Option<&mut BrowserView>, is_devtools: i32) -> i32 {
            if is_devtools == 0 { return 0; }
            let Some(view) = popup_browser_view else { return 0; };
            let mut delegate = HostWindow::new(RefCell::new(Some(view.clone())), true);
            if window_create_top_level(Some(&mut delegate)).is_some() { return 1; }
            tracing::error!("CEF não criou a janela do DevTools com ícone do Tako");
            0
        }
    }
}

wrap_window_delegate! {
    struct HostWindow { view: RefCell<Option<BrowserView>>, is_devtools: bool }
    impl ViewDelegate {
        fn preferred_size(&self, _view: Option<&mut View>) -> Size { initial_size(display_get_primary()) }
        fn minimum_size(&self, _view: Option<&mut View>) -> Size { Size { width: 960, height: 600 } }
    }
    impl PanelDelegate {}
    impl WindowDelegate {
        fn on_window_created(&self, window: Option<&mut Window>) {
            if let (Some(window), Some(view)) = (window, self.view.borrow().as_ref()) {
                window.set_title(Some(&CefString::from("Tako")));
                set_window_icons(window);
                window.set_to_fill_layout();
                window.add_child_view(Some(&mut View::from(view)));
                window.center_window(Some(&initial_size(window.display())));
                window.show();
            }
        }
        fn on_window_destroyed(&self, _window: Option<&mut Window>) { self.view.borrow_mut().take(); }
        fn is_frameless(&self, _window: Option<&mut Window>) -> i32 { i32::from(!self.is_devtools) }
        fn can_resize(&self, _window: Option<&mut Window>) -> i32 { 1 }
        fn can_minimize(&self, _window: Option<&mut Window>) -> i32 { 1 }
        fn can_maximize(&self, _window: Option<&mut Window>) -> i32 { 1 }
        fn can_close(&self, _window: Option<&mut Window>) -> i32 {
            self.view.borrow().as_ref().and_then(|view| view.browser()).and_then(|browser| browser.host()).map_or(1, |host| { host.close_dev_tools(); host.try_close_browser() })
        }
        // O DevTools pode usar um estilo diferente do navegador inspecionado.
        fn window_runtime_style(&self) -> RuntimeStyle {
            self.view.borrow().as_ref().map_or(RuntimeStyle::ALLOY, |view| view.runtime_style())
        }
    }
}

wrap_client! {
    struct HostClient { config: Config }
    impl Client {
        fn life_span_handler(&self) -> Option<LifeSpanHandler> { Some(Lifetime::new(self.config.clone())) }
        fn request_handler(&self) -> Option<RequestHandler> { Some(Navigation::new(self.config.clone())) }
        fn drag_handler(&self) -> Option<DragHandler> { Some(DragRegions::new()) }
        fn keyboard_handler(&self) -> Option<KeyboardHandler> { Some(Keys::new(self.config.clone())) }
        fn jsdialog_handler(&self) -> Option<JsdialogHandler> { Some(Dialogs::new()) }
        fn context_menu_handler(&self) -> Option<ContextMenuHandler> { Some(ContextMenu::new()) }
        fn on_process_message_received(&self, browser: Option<&mut Browser>, frame: Option<&mut Frame>, source_process: ProcessId, message: Option<&mut ProcessMessage>) -> i32 {
            accept_post(&self.config, browser, frame, source_process, message)
        }
    }
}

wrap_life_span_handler! {
    struct Lifetime { config: Config }
    impl LifeSpanHandler {
        fn on_after_created(&self, browser: Option<&mut Browser>) {
            browser_created(browser);
        }
        fn on_before_close(&self, browser: Option<&mut Browser>) {
            browser_closed(&self.config, browser);
        }
        fn on_before_popup(&self, _browser: Option<&mut Browser>, _frame: Option<&mut Frame>, _popup_id: i32, _target_url: Option<&CefString>, _target_frame_name: Option<&CefString>, _target_disposition: WindowOpenDisposition, _user_gesture: i32, _popup_features: Option<&PopupFeatures>, _window_info: Option<&mut WindowInfo>, _client: Option<&mut Option<Client>>, _settings: Option<&mut BrowserSettings>, _extra_info: Option<&mut Option<DictionaryValue>>, _no_javascript_access: Option<&mut i32>) -> i32 {
            if let Some(url) = _target_url { open_external(&url.to_string(), &self.config); }
            1
        }
    }
}

wrap_request_handler! {
    struct Navigation { config: Config }
    impl RequestHandler {
        fn on_before_browse(&self, _browser: Option<&mut Browser>, frame: Option<&mut Frame>, request: Option<&mut Request>, _user_gesture: i32, _is_redirect: i32) -> i32 {
            before_browse(&self.config, _browser, frame, request)
        }
        fn on_open_urlfrom_tab(&self, _browser: Option<&mut Browser>, _frame: Option<&mut Frame>, _target_url: Option<&CefString>, _target_disposition: WindowOpenDisposition, _user_gesture: i32) -> i32 {
            if let Some(url) = _target_url { open_external(&url.to_string(), &self.config); }
            1
        }
    }
}

wrap_drag_handler! {
    struct DragRegions;
    impl DragHandler {
        fn on_drag_enter(&self, _browser: Option<&mut Browser>, drag_data: Option<&mut DragData>, _mask: DragOperationsMask) -> i32 {
            reject_file_drag(drag_data)
        }
        fn on_draggable_regions_changed(&self, browser: Option<&mut Browser>, _frame: Option<&mut Frame>, regions: Option<&[DraggableRegion]>) {
            if let Some(window) = browser_view_get_for_browser(browser).and_then(|view| view.window()) {
                window.set_draggable_regions(regions);
            }
        }
    }
}

wrap_keyboard_handler! {
    struct Keys { config: Config }
    impl KeyboardHandler {
        fn on_key_event(&self, browser: Option<&mut Browser>, event: Option<&KeyEvent>, _os_event: Option<&mut cef::sys::tagMSG>) -> i32 {
            handle_key(&self.config, browser, event)
        }
    }
}

// Cliente sem ponte, eventos ou trava de navegação do pacote. O BrowserHost inspecionado controla sua vida.
wrap_client! {
    struct DevToolsClient { config: Config }
    impl Client {
        fn life_span_handler(&self) -> Option<LifeSpanHandler> { Some(Lifetime::new(self.config.clone())) }
        fn jsdialog_handler(&self) -> Option<JsdialogHandler> { Some(Dialogs::new()) }
    }
}

wrap_context_menu_handler! {
    struct ContextMenu;
    impl ContextMenuHandler {
        fn on_before_context_menu(&self, _browser: Option<&mut Browser>, _frame: Option<&mut Frame>, _params: Option<&mut ContextMenuParams>, model: Option<&mut MenuModel>) {
            if !cfg!(debug_assertions) && let Some(model) = model { model.clear(); }
        }
    }
}

wrap_render_process_handler! {
    struct Renderer { generation: Arc<AtomicU64> }
    impl RenderProcessHandler {
        fn on_context_created(&self, _browser: Option<&mut Browser>, frame: Option<&mut Frame>, context: Option<&mut V8Context>) {
            install_bridge(frame, context, &self.generation);
        }
    }
}

wrap_v8_handler! {
    struct Post { generation: String }
    impl V8Handler {
        fn execute(&self, _name: Option<&CefString>, _object: Option<&mut V8Value>, arguments: Option<&[Option<V8Value>]>, _retval: Option<&mut Option<V8Value>>, exception: Option<&mut CefString>) -> i32 {
            renderer_post(&self.generation, arguments, exception)
        }
    }
}

wrap_task! {
    struct Deliver { page: Page, messages: Vec<String>, actions: Vec<WindowAction> }
    impl Task {
        fn execute(&self) { deliver(&self.page, &self.messages, &self.actions); }
    }
}

fn window_action(browser: &mut Browser, action: WindowAction) {
    let Some(window) = browser_view_get_for_browser(Some(browser)).and_then(|view| view.window())
    else {
        return;
    };
    match action {
        WindowAction::Minimize => window.minimize(),
        WindowAction::ToggleMaximize => {
            if window.is_maximized() != 0 {
                window.restore();
            } else {
                window.maximize();
            }
        }
        WindowAction::Close => {
            if let Some(host) = browser.host() {
                host.close_dev_tools();
            }
            window.close();
        }
        WindowAction::BigPicture => window.set_fullscreen(i32::from(window.is_fullscreen() == 0)),
    }
    tracing::info!(?action, "ação da janela executada");
}

wrap_scheme_handler_factory! {
    struct ResourceFactory { resources: Arc<Resources> }
    impl SchemeHandlerFactory {
        fn create(&self, _browser: Option<&mut Browser>, _frame: Option<&mut Frame>, _scheme_name: Option<&CefString>, request: Option<&mut Request>) -> Option<ResourceHandler> {
            let request = request?;
            Some(FileResource::new(self.resources.clone(), cef_text(request.url()), cef_text(request.method()), Arc::new(Mutex::new((Resource::not_found(), 0)))))
        }
    }
}

wrap_resource_handler! {
    struct FileResource { resources: Arc<Resources>, url: String, method: String, body: Arc<Mutex<(Resource, usize)>> }
    impl ResourceHandler {
        fn open(&self, _request: Option<&mut Request>, handle_request: Option<&mut i32>, _callback: Option<&mut Callback>) -> i32 {
            open_resource(&self.resources, &self.url, &self.method, &self.body, handle_request)
        }
        fn response_headers(&self, response: Option<&mut Response>, response_length: Option<&mut i64>, _redirect_url: Option<&mut CefString>) {
            resource_headers(&self.method, &self.body, response, response_length)
        }
        fn read(&self, data_out: *mut u8, bytes_to_read: i32, bytes_read: Option<&mut i32>, _callback: Option<&mut ResourceReadCallback>) -> i32 {
            read_resource(&self.method, &self.body, data_out, bytes_to_read, bytes_read)
        }
    }
}

fn open_resource(
    resources: &Resources,
    url: &str,
    method: &str,
    body: &Mutex<(Resource, usize)>,
    handle_request: Option<&mut i32>,
) -> i32 {
    // CEF chama open em worker: disco, decode e resize não ocupam a thread da janela nem a de IO.
    if let Ok(mut body) = body.lock() {
        body.0 = if matches!(method, "GET" | "HEAD") {
            resources.serve(url)
        } else {
            Resource::not_found()
        };
    }
    if let Some(handle_request) = handle_request {
        *handle_request = 1;
    }
    1
}

fn resource_headers(
    method: &str,
    body: &Mutex<(Resource, usize)>,
    response: Option<&mut Response>,
    response_length: Option<&mut i64>,
) {
    if let (Some(response), Ok(body)) = (response, body.lock()) {
        response.set_status(body.0.status);
        let mime = body.0.mime.split(';').next().unwrap_or(body.0.mime);
        response.set_mime_type(Some(&CefString::from(mime)));
        if body.0.mime.ends_with("charset=utf-8") {
            response.set_charset(Some(&CefString::from("utf-8")));
        }
        response.set_header_by_name(
            Some(&CefString::from("Content-Type")),
            Some(&CefString::from(body.0.mime)),
            1,
        );
        for (name, value) in [
            ("Content-Security-Policy", CSP),
            ("X-Content-Type-Options", "nosniff"),
            ("Cache-Control", "no-store"),
        ] {
            response.set_header_by_name(
                Some(&CefString::from(name)),
                Some(&CefString::from(value)),
                1,
            );
        }
        if let Some(length) = response_length {
            *length = if method == "HEAD" {
                0
            } else {
                body.0.bytes.len() as i64
            };
        }
    }
}

#[allow(unsafe_code, clippy::not_unsafe_ptr_arg_deref)]
fn read_resource(
    method: &str,
    body: &Mutex<(Resource, usize)>,
    data_out: *mut u8,
    bytes_to_read: i32,
    bytes_read: Option<&mut i32>,
) -> i32 {
    let Some(bytes_read) = bytes_read else {
        return 0;
    };
    *bytes_read = 0;
    if data_out.is_null() || bytes_to_read <= 0 || method == "HEAD" {
        return 0;
    }
    let Ok(mut body) = body.lock() else {
        return 0;
    };
    let length = (body.0.bytes.len() - body.1).min(bytes_to_read as usize);
    // SAFETY: o CEF fornece bytes_to_read bytes graváveis durante este callback. O tamanho copiado
    // é limitado também ao restante do Vec, protegido pelo mutex; os buffers não se sobrepõem.
    unsafe {
        std::ptr::copy_nonoverlapping(body.0.bytes.as_ptr().add(body.1), data_out, length);
    }
    body.1 += length;
    *bytes_read = length as i32;
    i32::from(length > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_dragged_files_without_rejecting_text_links() {
        let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);
        let mut data = drag_data_create().unwrap();
        data.set_link_url(Some(&CefString::from("https://example.org/")));
        assert_eq!(reject_file_drag(Some(&mut data)), 0);
        data.add_file(Some(&CefString::from("C:\\fixture.png")), None);
        assert_eq!(reject_file_drag(Some(&mut data)), 1);
    }

    #[test]
    fn resource_reads_in_chunks_and_head_has_no_body() {
        let body = Mutex::new((
            Resource {
                status: 200,
                mime: "text/plain",
                bytes: b"abcde".to_vec(),
            },
            0,
        ));
        let mut out = [0u8; 3];
        let mut read = -1;
        let mut chunks = Vec::new();
        while read_resource("GET", &body, out.as_mut_ptr(), 3, Some(&mut read)) == 1 {
            chunks.push(out[..read as usize].to_vec());
        }
        assert_eq!(chunks, [b"abc".to_vec(), b"de".to_vec()]);
        assert_eq!(read, 0);
        body.lock().unwrap().1 = 0;
        assert_eq!(
            read_resource("HEAD", &body, out.as_mut_ptr(), 3, Some(&mut read)),
            0
        );
        assert_eq!(
            read_resource("GET", &body, std::ptr::null_mut(), 3, Some(&mut read)),
            0
        );
        assert_eq!(read, 0);
    }

    #[test]
    fn only_debug_accepts_external_chromium_options() {
        let path = Path::new("C:\tako");
        let release = cef_settings(path, path, path, false);
        assert_eq!(release.command_line_args_disabled, 1);
        assert_eq!(release.no_sandbox, 1);
        assert_eq!(
            cef_settings(path, path, path, true).command_line_args_disabled,
            0
        );
    }

    #[test]
    fn host_options_keep_what_cef_already_applied() {
        let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);
        let mut line = command_line_create().unwrap();
        line.init_from_string(Some(&CefString::from(
            "tako.exe --no-sandbox --lang=pt-BR --browser-subprocess-path=tako.exe",
        )));
        configure_command_line(Some(&mut line));
        for option in [
            "no-sandbox",
            "lang",
            "browser-subprocess-path",
            "disable-features",
        ] {
            assert_ne!(
                line.has_switch(Some(&CefString::from(option))),
                0,
                "{option}"
            );
        }
    }

    #[test]
    fn delivery_escapes_messages_and_generation_as_data() {
        let generation = "1:2\";throw new Error('generation');//";
        let message = "\"\\\n\r\t</script>\u{2028}\u{2029}');throw new Error('message');//";
        let script = deliver_script(generation, &[message.into()]);
        let prefix = "if(window.__takoGeneration===";
        let (encoded_generation, rest) = script
            .strip_prefix(prefix)
            .unwrap()
            .split_once("){for(const message of ")
            .unwrap();
        assert_eq!(
            serde_json::from_str::<String>(encoded_generation).unwrap(),
            generation
        );
        let encoded_messages = rest
            .strip_suffix(")window.__takoDeliver?.(message);}")
            .unwrap();
        assert_eq!(
            serde_json::from_str::<Vec<String>>(encoded_messages).unwrap(),
            [message]
        );
        assert!(!script.contains(['\u{2028}', '\u{2029}']));
    }

    #[test]
    fn only_external_http_links_open_in_the_system_browser() {
        for url in [
            "http://example.org/",
            "HTTPS://EXAMPLE.ORG/",
            "http://localhost:5174/",
        ] {
            assert!(is_external(url, "http://localhost:5173"));
        }
        for url in [
            "http://LOCALHOST:5173/path",
            "file:///secret",
            "javascript:alert(1)",
            "steam://run/570",
        ] {
            assert!(!is_external(url, "http://localhost:5173"));
        }
    }
    #[test]
    fn origin_is_exact() {
        assert!(trusted("tako-app://app/library", "tako-app://app"));
        for url in [
            "tako-app://app.evil/",
            "tako-app://app@evil/",
            "https://example.org/",
            "tako-app://app:123/",
        ] {
            assert!(!trusted(url, "tako-app://app"));
        }
        assert!(trusted(
            "HTTP://LOCALHOST:5173/path",
            "http://localhost:5173"
        ));
        assert!(!trusted("http://localhost:51730/", "http://localhost:5173"));
    }
}
