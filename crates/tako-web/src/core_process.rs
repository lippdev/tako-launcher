//! Processo do núcleo do `tako --web` (WEB-02b, D-10): tem o `AppService` e o `ui.state` e nunca carrega o CEF.
//! A interface é um processo filho descartável (`tako --web-ui`), que o núcleo abre, fecha e reabre.
//!
//! Ciclo de vida da interface:
//! - o usuário fechou a última janela → o app termina;
//! - o núcleo pediu (jogo, bandeja, `TAKO_WEB_RECYCLE_MS`) → `CoreMsg::Close`; sem sair em [`Timings::grace`],
//!   o processo é morto; o núcleo segue sem interface até alguém pedir para reabrir;
//! - queda → registra e reabre uma vez; outra queda até [`Timings::crash_window`] depois encerra com erro;
//! - com um jogo aberto, qualquer saída da interface só a deixa fechada: ela volta quando o jogo terminar.
//!
//! Sessão de jogo (WEB-20): `HostRequest::Launch` abre o jogo pelo `AppService`, que o acompanha nesta thread
//! (`poll_session` no prazo de `session_deadline`, sem bloquear). A interface fecha quando a janela do jogo
//! aparece ou com `game.resume`, e reabre onde estava quando o jogo termina.

use crate::bridge;
use crate::ipc::{self, CoreMsg, Page, UiMsg};
use std::collections::HashMap;
use std::io::{self, BufReader, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};
use tako_app::{AppService, HostRequest, SessionUpdate};

/// O processo da interface, visto do núcleo.
pub trait UiProcess: Send {
    fn id(&self) -> u32;
    /// `Some(código)` depois que o processo terminou.
    fn exit_code(&mut self) -> Option<i32>;
    fn kill(&mut self);
}

/// Um processo da interface recém-aberto: entrada padrão dele (`input`) e saída padrão (`output`).
pub struct UiChild {
    pub process: Box<dyn UiProcess>,
    pub input: Box<dyn Write + Send>,
    pub output: Box<dyn Read + Send>,
}

pub trait UiLauncher {
    fn launch(&mut self) -> io::Result<UiChild>;
}

#[derive(Clone, Copy, Debug)]
pub struct Timings {
    /// Prazo para a interface sair depois de `CoreMsg::Close`; depois disso, o núcleo mata o processo.
    pub grace: Duration,
    /// Uma queda até este tempo depois da reabertura por queda encerra o app (sem laço infinito).
    pub crash_window: Duration,
    /// Com a interface aberta, intervalo entre as conferências de saída do processo.
    pub tick: Duration,
}

impl Default for Timings {
    fn default() -> Self {
        Self {
            grace: Duration::from_secs(5),
            crash_window: Duration::from_secs(60),
            tick: Duration::from_millis(250),
        }
    }
}

/// Pedidos ao núcleo sobre a interface. `Hide` e `Show` são o gancho da sessão de jogo e da bandeja.
pub enum Command {
    Ui(u64, UiMsg),
    UiEnded(u64),
    Host(HostRequest),
    /// Fechar a interface sem encerrar o app.
    Hide,
    /// Reabrir a interface fechada a pedido.
    Show,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Exit {
    User,
    Requested,
    Crash,
}

fn classify(close_requested: bool, user_closed: bool, code: i32) -> Exit {
    if close_requested {
        Exit::Requested
    } else if user_closed || code == 0 {
        Exit::User
    } else {
        Exit::Crash
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Next {
    Quit,
    Hidden,
    Reopen,
    Fail(String),
}

/// Reabre só uma vez por janela de tempo: duas quedas seguidas param o app com um erro claro.
#[derive(Default)]
struct Restarts {
    last: Option<Instant>,
}

impl Restarts {
    fn after(&mut self, exit: Exit, code: i32, now: Instant, window: Duration) -> Next {
        match exit {
            Exit::User => Next::Quit,
            Exit::Requested => Next::Hidden,
            Exit::Crash if self.last.is_some_and(|last| now - last < window) => {
                Next::Fail(format!(
                    "a interface caiu de novo logo depois de reabrir (código {code}); veja os logs"
                ))
            }
            Exit::Crash => {
                self.last = Some(now);
                Next::Reopen
            }
        }
    }
}

struct Ui {
    serial: u64,
    process: Box<dyn UiProcess>,
    writer: mpsc::Sender<CoreMsg>,
    close_requested: bool,
    kill_at: Option<Instant>,
    user_closed: bool,
    ended: bool,
}

impl Ui {
    fn send(&self, message: CoreMsg) {
        // Erro só quando o escritor já parou: o processo terminou e o laço vai notar.
        let _ = self.writer.send(message);
    }

    fn request_close(&mut self, now: Instant, grace: Duration) {
        if !self.close_requested {
            tracing::info!(pid = self.process.id(), "pedindo à interface para fechar");
            self.close_requested = true;
            self.kill_at = Some(now + grace);
            self.send(CoreMsg::Close);
        }
    }
}

struct Host<L> {
    launcher: L,
    timings: Timings,
    tx: mpsc::Sender<Command>,
    ui: Option<Ui>,
    pages: HashMap<i32, Page>,
    serial: u64,
    restarts: Restarts,
    /// `Command::Show` pendente: abre assim que não houver interface (inclusive depois de uma que está fechando).
    show: bool,
}

impl<L: UiLauncher> Host<L> {
    fn open_ui(&mut self) -> Result<(), String> {
        let child = self
            .launcher
            .launch()
            .map_err(|e| format!("não foi possível abrir a interface: {e}"))?;
        self.serial += 1;
        let serial = self.serial;
        let tx = self.tx.clone();
        let output = child.output;
        std::thread::spawn(move || {
            let result =
                ipc::read_messages::<UiMsg>(BufReader::new(output), ipc::MAX_UI_LINE, |message| {
                    tx.send(Command::Ui(serial, message)).is_ok()
                });
            if let Err(error) = result {
                tracing::debug!("saída da interface: {error}");
            }
            let _ = tx.send(Command::UiEnded(serial));
        });
        // Fila sem limite: o núcleo nunca espera a interface. Ao soltar o `Ui`, o escritor termina e fecha a
        // entrada do filho, que sai sozinho.
        let (writer, queue) = mpsc::channel::<CoreMsg>();
        let mut input = child.input;
        std::thread::spawn(move || {
            for message in queue {
                if ipc::write_message(&mut input, &message).is_err() {
                    break;
                }
            }
        });
        tracing::info!(
            pid = child.process.id(),
            "interface aberta em processo próprio"
        );
        self.ui = Some(Ui {
            serial,
            process: child.process,
            writer,
            close_requested: false,
            kill_at: None,
            user_closed: false,
            ended: false,
        });
        Ok(())
    }

    /// Pede para a interface fechar, sem encerrar o app.
    fn hide(&mut self, now: Instant) {
        if let Some(ui) = &mut self.ui {
            ui.request_close(now, self.timings.grace);
        }
    }

    /// Há uma página aberta que recebe o que for entregue agora.
    fn listening(&self) -> bool {
        self.ui.as_ref().is_some_and(|ui| !ui.close_requested) && !self.pages.is_empty()
    }

    /// Confere o processo e os prazos. `Some` termina o app. Com `game` (um jogo aberto), a interface que sai por
    /// qualquer motivo fica fechada até o fim do jogo, como no supervisor do Slint.
    fn watch(&mut self, now: Instant, game: bool) -> Option<Result<(), String>> {
        let Some(ui) = &mut self.ui else {
            if std::mem::take(&mut self.show) {
                return self.open_ui().err().map(Err);
            }
            return None;
        };
        if !ui.close_requested {
            self.show = false;
        }
        if let Some(code) = ui.process.exit_code() {
            let exit = classify(ui.close_requested, ui.user_closed, code);
            match exit {
                Exit::Crash => tracing::error!(code, "a interface caiu"),
                _ => tracing::info!(code, ?exit, "interface fechada"),
            }
            self.ui = None;
            self.pages.clear();
            if game && exit != Exit::Requested {
                tracing::info!(
                    "interface fechada durante o jogo; ela volta quando o jogo terminar"
                );
                return None;
            }
            return match self
                .restarts
                .after(exit, code, now, self.timings.crash_window)
            {
                Next::Quit => Some(Ok(())),
                Next::Fail(error) => Some(Err(error)),
                Next::Reopen => self.open_ui().err().map(Err),
                Next::Hidden => None,
            };
        }
        if ui.kill_at.is_some_and(|at| now >= at) {
            tracing::warn!(
                pid = ui.process.id(),
                "a interface não fechou no prazo; matando"
            );
            ui.kill_at = None;
            ui.process.kill();
        }
        None
    }

    fn next_wake(&self, now: Instant) -> Option<Duration> {
        let deadlines = match &self.ui {
            Some(ui) => {
                // Fim da saída padrão: o processo está saindo; conferir mais vezes.
                let tick = if ui.ended {
                    self.timings.tick / 10
                } else {
                    self.timings.tick
                };
                [Some(now + tick), ui.kill_at]
            }
            None => [self.show.then_some(now), None],
        };
        deadlines
            .into_iter()
            .flatten()
            .min()
            .map(|at| at.saturating_duration_since(now))
    }

    fn current(&mut self, serial: u64) -> Option<&mut Ui> {
        self.ui.as_mut().filter(|ui| ui.serial == serial)
    }

    fn deliver_all(&self, target: Option<&Page>, messages: &[String]) {
        let Some(ui) = &self.ui else {
            // Sem interface, os eventos se perdem de propósito: ao reabrir, a página consulta o núcleo.
            return;
        };
        for page in self.pages.values() {
            // Resposta só para a página que pediu; eventos para todas.
            let batch = if target.is_none() || target == Some(page) {
                messages.to_vec()
            } else {
                messages.iter().skip(1).cloned().collect()
            };
            if !batch.is_empty() {
                ui.send(CoreMsg::Deliver {
                    page: page.clone(),
                    messages: batch,
                    actions: Vec::new(),
                });
            }
        }
    }

    fn shutdown_ui(&mut self) {
        let Some(mut ui) = self.ui.take() else {
            return;
        };
        let started = Instant::now();
        ui.request_close(started, self.timings.grace);
        while ui.process.exit_code().is_none() {
            if started.elapsed() >= self.timings.grace {
                ui.process.kill();
                break;
            }
            std::thread::sleep(self.timings.tick / 10);
        }
    }
}

/// Atende a interface até o app terminar. `open` abre o núcleo depois do primeiro filho, para a janela subir
/// enquanto o banco carrega. `control` recebe um emissor de [`Command`] (gancho para jogo e bandeja).
pub fn run<L: UiLauncher>(
    launcher: L,
    open: impl FnOnce() -> Result<AppService, String>,
    timings: Timings,
    control: impl FnOnce(mpsc::Sender<Command>),
) -> Result<(), String> {
    let (tx, rx) = mpsc::channel::<Command>();
    let mut host = Host {
        launcher,
        timings,
        tx: tx.clone(),
        ui: None,
        pages: HashMap::new(),
        serial: 0,
        restarts: Restarts::default(),
        show: false,
    };
    host.open_ui()?;
    let mut service = match open() {
        Ok(service) => service,
        Err(error) => {
            tracing::error!(%error, "não foi possível abrir o núcleo");
            host.shutdown_ui();
            return Err(error);
        }
    };
    let (host_tx, host_rx) = mpsc::channel();
    let (event_tx, events) = mpsc::channel();
    let host_wake = host_tx.clone();
    service.set_host_sink(host_tx);
    service.set_event_sink(event_tx);
    // Um só receptor acorda por pedidos da página e por resultados das lojas, inclusive sem interface.
    let relay_stop = Arc::new(AtomicBool::new(false));
    let stopped = relay_stop.clone();
    let relay_tx = tx.clone();
    let relay = std::thread::spawn(move || {
        for request in host_rx {
            if stopped.load(Ordering::Relaxed) || relay_tx.send(Command::Host(request)).is_err() {
                break;
            }
        }
    });
    control(tx);
    let result = loop {
        let now = Instant::now();
        let session = service
            .session_deadline()
            .map(|at| at.saturating_duration_since(now));
        let wake = match (host.next_wake(now), session) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        let command = match wake {
            Some(wait) => rx.recv_timeout(wait).ok(),
            // `host.tx` mantém o canal aberto: sem prazo, espera o próximo pedido.
            None => rx.recv().ok(),
        };
        let mut response = None;
        match command {
            // Mensagens de uma interface já substituída não valem para a atual.
            Some(Command::Ui(serial, message)) if host.current(serial).is_some() => match message {
                UiMsg::Request { page, text } => {
                    host.pages.insert(page.browser, page.clone());
                    response = Some((page, bridge::handle_message(&mut service, &text)));
                }
                UiMsg::Page { page } => {
                    host.pages.insert(page.browser, page);
                }
                UiMsg::Closed { browser } => {
                    host.pages.remove(&browser);
                }
                UiMsg::UserClosed => {
                    if let Some(ui) = host.current(serial) {
                        ui.user_closed = true;
                    }
                }
            },
            Some(Command::Ui(..)) => {}
            Some(Command::UiEnded(serial)) => {
                if let Some(ui) = host.current(serial) {
                    ui.ended = true;
                }
            }
            Some(Command::Host(HostRequest::PollBackground)) => service.poll(),
            Some(Command::Host(HostRequest::Window(action))) => {
                if let Some(ui) = &host.ui {
                    for page in host.pages.values() {
                        ui.send(CoreMsg::Deliver {
                            page: page.clone(),
                            messages: Vec::new(),
                            actions: vec![action],
                        });
                    }
                }
            }
            Some(Command::Host(HostRequest::Launch { game_id })) => {
                tracing::info!(%game_id, "abrindo o jogo");
                service.launch_game(&game_id);
            }
            Some(Command::Host(HostRequest::Resume)) => {
                tracing::info!("voltar ao jogo: fechando a interface");
                host.hide(now);
            }
            Some(Command::Hide) => host.hide(now),
            Some(Command::Show) => host.show = true,
            None => {}
        }
        // A interface fecha durante o jogo (D-10) e volta no fim, onde estava (`ui.state`).
        match service.poll_session(Instant::now()) {
            Some(SessionUpdate::Shown) => {
                tracing::info!("janela do jogo na tela: fechando a interface");
                host.hide(now);
            }
            Some(SessionUpdate::Ended) => {
                if host.listening() {
                    // O `game.ended` vai agora para a página aberta; sem ela, sai depois do próximo `game.running`.
                    service.session_end_seen();
                } else {
                    tracing::info!("jogo terminou: reabrindo a interface");
                }
                host.show = true;
            }
            None => {}
        }
        let (target, text) = response.unzip();
        let messages = bridge::drain_messages(text, &events);
        // Resposta e eventos causados por ela seguem juntos e nessa ordem pela mesma fila do filho.
        host.deliver_all(target.as_ref(), &messages);
        if let Some(result) = host.watch(Instant::now(), service.game_active()) {
            break result;
        }
    };
    host.shutdown_ui();
    drop(service);
    relay_stop.store(true, Ordering::Relaxed);
    let _ = host_wake.send(HostRequest::PollBackground);
    // Liberar o receptor antes do join evita reter um relay bloqueado.
    drop(rx);
    relay
        .join()
        .map_err(|_| "thread de avisos encerrou com pânico".to_owned())?;
    result
}

/// Abre `tako --web-ui` com pipes na entrada e na saída padrão; a saída de erro fica com a do núcleo.
#[cfg(all(windows, feature = "cef"))]
pub struct ExeLauncher {
    pub exe: std::path::PathBuf,
    pub args: Vec<String>,
}

#[cfg(all(windows, feature = "cef"))]
impl UiLauncher for ExeLauncher {
    fn launch(&mut self) -> io::Result<UiChild> {
        use std::process::{Command, Stdio};
        let mut child = Command::new(&self.exe)
            .args(&self.args)
            .env(ipc::CORE_PID_ENV, std::process::id().to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let input = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("stdin"))?;
        let output = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("stdout"))?;
        Ok(UiChild {
            process: Box::new(ExeProcess(child)),
            input: Box::new(input),
            output: Box::new(output),
        })
    }
}

#[cfg(all(windows, feature = "cef"))]
struct ExeProcess(std::process::Child);

#[cfg(all(windows, feature = "cef"))]
impl UiProcess for ExeProcess {
    fn id(&self) -> u32 {
        self.0.id()
    }

    fn exit_code(&mut self) -> Option<i32> {
        match self.0.try_wait() {
            Ok(Some(status)) => Some(status.code().unwrap_or(-1)),
            Ok(None) => None,
            Err(_) => Some(-1),
        }
    }

    fn kill(&mut self) {
        if let Err(error) = self.0.kill() {
            tracing::warn!(%error, "matar a interface");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufRead;
    use std::sync::Mutex;
    use tako_app::protocol::WindowAction;

    #[test]
    fn exit_kinds_and_restart_policy() {
        assert_eq!(classify(true, false, 1), Exit::Requested);
        assert_eq!(classify(false, true, 7), Exit::User);
        assert_eq!(classify(false, false, 0), Exit::User);
        assert_eq!(classify(false, false, 3), Exit::Crash);
        let window = Duration::from_secs(60);
        let start = Instant::now();
        let mut restarts = Restarts::default();
        assert_eq!(restarts.after(Exit::User, 0, start, window), Next::Quit);
        assert_eq!(
            restarts.after(Exit::Requested, 1, start, window),
            Next::Hidden
        );
        assert_eq!(restarts.after(Exit::Crash, 3, start, window), Next::Reopen);
        assert!(matches!(
            restarts.after(Exit::Crash, 3, start + Duration::from_secs(59), window),
            Next::Fail(_)
        ));
        // Uma queda depois de muito tempo estável reabre de novo.
        assert_eq!(
            restarts.after(Exit::Crash, 3, start + Duration::from_secs(61), window),
            Next::Reopen
        );
    }

    #[derive(Clone, Default)]
    struct FakeState {
        code: Arc<Mutex<Option<i32>>>,
        killed: Arc<AtomicBool>,
    }

    struct FakeProcess(FakeState);

    impl UiProcess for FakeProcess {
        fn id(&self) -> u32 {
            0
        }
        fn exit_code(&mut self) -> Option<i32> {
            *self.0.code.lock().unwrap()
        }
        fn kill(&mut self) {
            self.0.killed.store(true, Ordering::Relaxed);
            *self.0.code.lock().unwrap() = Some(1);
        }
    }

    /// O lado do filho: lê o que o núcleo manda e escreve como a interface.
    struct FakeUi {
        state: FakeState,
        from_core: BufReader<io::PipeReader>,
        to_core: io::PipeWriter,
    }

    impl FakeUi {
        fn send(&mut self, message: &UiMsg) {
            ipc::write_message(&mut self.to_core, message).unwrap();
        }
        fn request(&mut self, text: &str) {
            self.send(&UiMsg::Request {
                page: page(),
                text: text.into(),
            });
        }
        fn next(&mut self) -> Option<CoreMsg> {
            let mut line = String::new();
            (self.from_core.read_line(&mut line).unwrap() > 0)
                .then(|| serde_json::from_str(&line).unwrap())
        }
        fn exit(self, code: i32) {
            *self.state.code.lock().unwrap() = Some(code);
        }
    }

    fn page() -> Page {
        Page {
            browser: 1,
            frame: "F".into(),
            generation: "1:0".into(),
        }
    }

    struct FakeLauncher(mpsc::Sender<FakeUi>);

    impl UiLauncher for FakeLauncher {
        fn launch(&mut self) -> io::Result<UiChild> {
            let (from_core, input) = io::pipe()?;
            let (output, to_core) = io::pipe()?;
            let state = FakeState::default();
            self.0
                .send(FakeUi {
                    state: state.clone(),
                    from_core: BufReader::new(from_core),
                    to_core,
                })
                .unwrap();
            Ok(UiChild {
                process: Box::new(FakeProcess(state)),
                input: Box::new(input),
                output: Box::new(output),
            })
        }
    }

    fn fast() -> Timings {
        Timings {
            grace: Duration::from_millis(200),
            crash_window: Duration::from_secs(60),
            tick: Duration::from_millis(10),
        }
    }

    type Running = (
        mpsc::Receiver<FakeUi>,
        mpsc::Sender<Command>,
        std::thread::JoinHandle<Result<(), String>>,
    );

    fn start(timings: Timings) -> Running {
        let (children, launched) = mpsc::channel();
        let (control_tx, control) = mpsc::channel();
        let core = std::thread::spawn(move || {
            run(
                FakeLauncher(children),
                || Ok(AppService::new("9.9.9")),
                timings,
                |tx| control_tx.send(tx).unwrap(),
            )
        });
        (launched, control.recv().unwrap(), core)
    }

    fn reply(message: CoreMsg) -> serde_json::Value {
        let CoreMsg::Deliver { messages, .. } = message else {
            panic!("esperava entrega")
        };
        serde_json::from_str(&messages[0]).unwrap()
    }

    #[test]
    fn closing_on_request_and_reopening_restores_the_ui_state() {
        let (launched, control, core) = start(fast());
        let mut ui = launched.recv().unwrap();
        ui.request(
            r#"{"v":1,"id":1,"name":"ui.state.set","payload":{"json":"{\"screen\":\"game\"}"}}"#,
        );
        ui.request(r#"{"v":1,"id":2,"name":"window.action","payload":"toggle_maximize"}"#);
        assert_eq!(reply(ui.next().unwrap())["id"], 1);
        assert_eq!(reply(ui.next().unwrap())["id"], 2);
        // A ação chega depois da resposta e é executada no processo da interface.
        assert_eq!(
            ui.next(),
            Some(CoreMsg::Deliver {
                page: page(),
                messages: Vec::new(),
                actions: vec![WindowAction::ToggleMaximize],
            })
        );
        control.send(Command::Hide).unwrap();
        assert_eq!(ui.next(), Some(CoreMsg::Close));
        ui.exit(0);
        // Sem interface, o núcleo continua e não abre outra sozinho.
        std::thread::sleep(Duration::from_millis(100));
        assert!(launched.try_recv().is_err());
        control.send(Command::Show).unwrap();
        let mut ui = launched.recv().unwrap();
        ui.request(r#"{"v":1,"id":1,"name":"ui.state"}"#);
        assert_eq!(
            reply(ui.next().unwrap())["reply"]["data"]["json"],
            r#"{"screen":"game"}"#
        );
        ui.send(&UiMsg::UserClosed);
        ui.exit(0);
        assert_eq!(core.join().unwrap(), Ok(()));
    }

    #[test]
    fn ui_that_ignores_the_close_request_is_killed() {
        let (launched, control, core) = start(fast());
        let mut ui = launched.recv().unwrap();
        control.send(Command::Hide).unwrap();
        assert_eq!(ui.next(), Some(CoreMsg::Close));
        let state = ui.state.clone();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !state.killed.load(Ordering::Relaxed) {
            assert!(Instant::now() < deadline, "não matou");
            std::thread::sleep(Duration::from_millis(10));
        }
        control.send(Command::Show).unwrap();
        let ui = launched.recv().unwrap();
        ui.exit(0);
        assert_eq!(core.join().unwrap(), Ok(()));
    }

    #[test]
    fn a_crash_reopens_once_and_a_second_crash_ends_with_an_error() {
        let (launched, _control, core) = start(fast());
        launched.recv().unwrap().exit(3);
        launched.recv().unwrap().exit(3);
        let error = core.join().unwrap().unwrap_err();
        assert!(error.contains("caiu de novo"), "{error}");
        assert!(launched.try_recv().is_err(), "abriu uma terceira vez");
    }

    #[test]
    fn messages_from_a_replaced_ui_are_ignored() {
        let (launched, _control, core) = start(fast());
        let old = launched.recv().unwrap();
        let mut stale = old.to_core.try_clone().unwrap();
        old.exit(3);
        let mut ui = launched.recv().unwrap();
        // Se valesse para a nova, a queda abaixo viraria fechamento pelo usuário.
        ipc::write_message(&mut stale, &UiMsg::UserClosed).unwrap();
        ui.request(r#"{"v":1,"id":5,"name":"app.info"}"#);
        assert_eq!(reply(ui.next().unwrap())["id"], 5);
        ui.exit(3);
        assert!(core.join().unwrap().is_err());
    }

    /// Lê entregas até achar uma mensagem (resposta ou evento) que satisfaça `want`.
    fn until(ui: &mut FakeUi, want: impl Fn(&serde_json::Value) -> bool) -> serde_json::Value {
        loop {
            match ui.next() {
                Some(CoreMsg::Deliver { messages, .. }) => {
                    if let Some(found) = messages
                        .iter()
                        .map(|m| serde_json::from_str::<serde_json::Value>(m).unwrap())
                        .find(|m| want(m))
                    {
                        return found;
                    }
                }
                Some(CoreMsg::Close) => panic!("fechou antes da mensagem esperada"),
                None => panic!("canal fechado"),
            }
        }
    }

    fn event(name: &'static str) -> impl Fn(&serde_json::Value) -> bool {
        move |m| m["event"]["name"] == name
    }

    fn answer(id: u32) -> impl Fn(&serde_json::Value) -> bool {
        move |m| m["id"] == id
    }

    #[test]
    fn the_ui_closes_during_the_game_and_comes_back_at_the_end_with_the_session() {
        let dir = tempfile::tempdir().unwrap();
        let paths = tako_core::paths::DataPaths::at(dir.path());
        let (children, launched) = mpsc::channel();
        let (control_tx, control) = mpsc::channel();
        let core = std::thread::spawn(move || {
            run(
                FakeLauncher(children),
                move || {
                    // Demonstração: o jogo e a tela são simulados no núcleo.
                    tako_app::Core::open(paths, tako_platform::Platform::simulated(), true)
                        .map(|core| AppService::with_core("9.9.9", core))
                        .map_err(|e| e.to_string())
                },
                fast(),
                |tx| control_tx.send(tx).unwrap(),
            )
        });
        let control: mpsc::Sender<Command> = control.recv().unwrap();
        let mut ui = launched.recv().unwrap();
        ui.request(r#"{"v":1,"id":1,"name":"game.play","payload":{"gameId":"steam:demo0"}}"#);
        assert_eq!(until(&mut ui, answer(1))["ok"], true);
        let started = until(&mut ui, event("game.started"));
        assert_eq!(started["event"]["payload"]["gameId"], "steam:demo0");
        // A janela do jogo aparece: a interface recebe o pedido de fechar e sai.
        loop {
            match ui.next() {
                Some(CoreMsg::Close) => break,
                Some(_) => {}
                None => panic!("canal fechado"),
            }
        }
        ui.exit(0);
        // Fechada durante o jogo: nem encerra o app, nem reabre sozinha.
        std::thread::sleep(Duration::from_millis(100));
        assert!(launched.try_recv().is_err());
        assert!(!core.is_finished());

        // Reaberta por cima do jogo (o gancho da Central, WEB-22): sabe do jogo e pede para encerrá-lo.
        control.send(Command::Show).unwrap();
        let mut ui = launched.recv().unwrap();
        ui.request(r#"{"v":1,"id":2,"name":"game.running"}"#);
        let running = until(&mut ui, answer(2));
        assert_eq!(running["reply"]["data"]["state"], "playing");
        ui.request(r#"{"v":1,"id":3,"name":"game.end","payload":{}}"#);
        assert_eq!(until(&mut ui, answer(3))["ok"], true);
        // Fecha de novo antes de o fim ser visto (a folga do pedido é de 1 s).
        control.send(Command::Hide).unwrap();
        loop {
            match ui.next() {
                Some(CoreMsg::Close) => break,
                Some(_) => {}
                None => panic!("canal fechado"),
            }
        }
        ui.exit(0);

        // O jogo termina com a interface fechada: ela reabre e o aviso sai depois do primeiro game.running.
        let mut ui = launched.recv().unwrap();
        ui.request(r#"{"v":1,"id":4,"name":"game.running"}"#);
        let Some(CoreMsg::Deliver { messages, .. }) = ui.next() else {
            panic!("esperava entrega")
        };
        let messages: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| serde_json::from_str(m).unwrap())
            .collect();
        assert_eq!(messages[0]["id"], 4);
        assert_eq!(messages[0]["reply"]["data"], serde_json::Value::Null);
        let ended = &messages[1]["event"];
        assert_eq!(ended["name"], "game.ended");
        assert_eq!(ended["payload"]["gameId"], "steam:demo0");
        assert_eq!(ended["payload"]["end"], "normal");
        // Sem jogo, fechar a janela encerra o app.
        ui.send(&UiMsg::UserClosed);
        ui.exit(0);
        assert_eq!(core.join().unwrap(), Ok(()));
    }

    #[test]
    fn failing_to_open_the_core_closes_the_ui() {
        let (children, launched) = mpsc::channel();
        let core = std::thread::spawn(move || {
            run(
                FakeLauncher(children),
                || Err("banco".into()),
                fast(),
                |_| {},
            )
        });
        let mut ui = launched.recv().unwrap();
        assert_eq!(ui.next(), Some(CoreMsg::Close));
        ui.exit(0);
        assert_eq!(core.join().unwrap(), Err("banco".into()));
    }
}
