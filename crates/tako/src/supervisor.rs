//! Supervisor (ADR-0026): processo leve, sem janela, que abre a interface, a reinicia se cair ou travar e,
//! durante o jogo, fica sozinho: acompanha o jogo, ouve o atalho da Central, move o ponteiro pelo controle
//! (quando ligado) e no fim restaura tela, áudio e cursor e reabre a interface no jogo.

use crate::protocol::{self, SupMsg, UiMsg};
use std::io::BufReader;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};
use tako_core::input::GamepadButtons;
use tako_core::input::shortcut::{ButtonChordDetector, chord};
use tako_core::paths::DataPaths;
use tako_core::persistence::{SettingsStore, TakoDatabase};
use tako_core::session::GameEnd;
use tako_core::supervision::{SupervisorAction, SupervisorOptions, UiHeartbeat, UiWatch, args};
use tako_platform::Platform;
use tako_ui::gamepad::Pads;
use tako_ui::session::{self, Running};

#[cfg(test)]
mod link_tests {
    use super::*;

    #[test]
    fn activation_is_bounded_validated_and_consumed_once() {
        let dir = std::env::temp_dir().join(format!("tako-links-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let paths = DataPaths::at(&dir);
        let supervisor = Supervisor::new(paths, Platform::simulated(), &["tako".into()]);
        let file = dir.join(crate::ACTIVATE_FILE);
        for url in ["", "tako://settings", "tako://game/steam:570"] {
            std::fs::write(&file, url).unwrap();
            assert_eq!(supervisor.activation_requested().as_deref(), Some(url));
            assert!(supervisor.activation_requested().is_none());
        }
        for url in [
            "https://example.com".to_owned(),
            "x".repeat(tako_core::links::MAX_URL + 1),
        ] {
            std::fs::write(&file, url).unwrap();
            assert!(supervisor.activation_requested().is_none());
            assert!(!file.exists());
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

/// Como abrir a próxima interface.
#[derive(Default, Clone)]
struct Launch {
    link: Option<String>,
    after_game: Option<(String, Duration, GameEnd)>,
    control_center: bool,
}

struct UiProc {
    child: Child,
    stdin: Option<ChildStdin>,
    rx: Receiver<UiMsg>,
    /// A interface avisou que vai sair de propósito.
    closing: Option<String>,
}

pub struct Supervisor {
    paths: DataPaths,
    platform: Platform,
    passthrough: Vec<String>,
    watch: UiWatch,
    started: Instant,
    running: Option<Running>,
    /// Encerrar jogo foi pedido: confere o fim a cada volta (0,5 s), a interface mostra "Fechando o jogo…".
    closing: bool,
    /// O jogo abriu com a interface em "Abrindo…": avisar quando a janela dele aparecer.
    launching: bool,
    pads: Option<Pads>,
    chord: ButtonChordDetector,
    key: tako_core::input::shortcut::KeyShortcut,
    initial_link: Option<String>,
}

impl Supervisor {
    pub fn new(paths: DataPaths, platform: Platform, all_args: &[String]) -> Supervisor {
        let settings = SettingsStore::new(TakoDatabase::new(paths.database()));
        let chord_value = settings.get("ControlCenterChord").ok().flatten();
        let key = settings.get("ControlCenterKey").ok().flatten();
        Supervisor {
            initial_link: tako_core::links::argument(all_args).map(str::to_owned),
            paths,
            platform,
            passthrough: args::passthrough(&all_args[1..]),
            watch: UiWatch::new(SupervisorOptions::default()),
            started: Instant::now(),
            running: None,
            closing: false,
            launching: false,
            pads: None,
            chord: ButtonChordDetector::new(chord::parse(chord_value.as_deref())),
            key: tako_core::input::shortcut::KeyShortcut::parse(key.as_deref()),
        }
    }

    /// Outro Tako foi aberto (ver `ACTIVATE_FILE`)? Consome o pedido.
    fn activation_requested(&self) -> Option<String> {
        use std::io::Read;
        let path = self.paths.root.join(crate::ACTIVATE_FILE);
        let file = std::fs::File::open(&path).ok()?;
        let mut url = String::new();
        let read = file
            .take(tako_core::links::MAX_URL as u64 + 1)
            .read_to_string(&mut url);
        if let Err(e) = std::fs::remove_file(path) {
            tracing::warn!("consumir ativação: {e}");
        }
        if read.is_err() || (!url.is_empty() && tako_core::links::parse(&url).is_none()) {
            tracing::warn!("pedido de ativação inválido ignorado");
            return None;
        }
        Some(url)
    }

    fn now(&self) -> Duration {
        self.started.elapsed()
    }

    fn spawn_ui(&mut self, safe: bool, launch: &Launch) -> Result<UiProc, String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let mode = if safe {
            tako_core::supervision::UiLaunchMode::SafeMode
        } else {
            tako_core::supervision::UiLaunchMode::Normal
        };
        let mut a = args::for_ui("stdio", mode, &self.passthrough);
        if let Some(url) = &launch.link {
            a.extend(["--open-url".into(), url.clone()]);
        }
        if let Some((id, played, end)) = &launch.after_game {
            a.extend([
                "--after-game".into(),
                id.clone(),
                "--played".into(),
                played.as_secs().to_string(),
                "--game-end".into(),
                end.key().into(),
            ]);
        }
        if let Some(r) = &self.running {
            a.extend([
                "--running".into(),
                r.req.game_id.clone(),
                "--running-title".into(),
                r.req.title.clone(),
            ]);
            if r.req.controller_mouse {
                a.push("--controller-mouse".into());
            }
        }
        if launch.control_center {
            a.push("--control-center".into());
        }
        let mut child = Command::new(exe)
            .args(&a)
            .env("TAKO_SUPERVISOR_PID", std::process::id().to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        let stdout = child.stdout.take().ok_or("sem saída da interface")?;
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            protocol::read_lines::<UiMsg>(BufReader::new(stdout), |m| drop(tx.send(m)))
        });
        self.watch.started(child.id(), self.now());
        let stdin = child.stdin.take();
        Ok(UiProc {
            child,
            stdin,
            rx,
            closing: None,
        })
    }

    /// Laço principal; devolve o código de saída do processo.
    pub fn run(mut self, safe_mode: bool) -> i32 {
        session::recover_after_crash(&self.platform, &self.paths);
        self.activation_requested(); // pedido velho, de antes desta execução
        let mut safe = matches!(
            self.watch.begin(safe_mode),
            tako_core::supervision::UiLaunchMode::SafeMode
        );
        let mut launch = Launch {
            link: self.initial_link.take(),
            ..Default::default()
        };
        loop {
            let mut ui = match self.spawn_ui(safe, &launch) {
                Ok(u) => u,
                Err(e) => {
                    tracing::error!("não foi possível abrir a interface: {e}");
                    return 1;
                }
            };
            launch = Launch::default();
            let code = self.watch_ui(&mut ui);
            // "Abrindo…" só vale para a interface que abriu o jogo.
            self.launching = false;
            if let Some(reason) = &ui.closing
                && reason == "update"
            {
                return 0;
            }
            if ui.closing.as_deref() == Some("normal") {
                // Saiu do modo seguro: reabre a interface normal.
                safe = false;
                continue;
            }
            if self.running.is_some() {
                // A interface fechou para o jogo (ou caiu durante ele): o supervisor segue com a sessão.
                launch = self.game_loop();
                continue;
            }
            if ui.closing.is_some() {
                return 0;
            }
            let decision = self.watch.exited(code, None, self.now());
            match decision.action {
                SupervisorAction::Exit => return 0,
                SupervisorAction::GiveUp => {
                    tracing::error!("{}", decision.reason);
                    return 1;
                }
                SupervisorAction::Restart => {
                    tracing::warn!("{}", decision.reason);
                    safe = decision.mode == tako_core::supervision::UiLaunchMode::SafeMode;
                    std::thread::sleep(SupervisorOptions::default().restart_delay);
                }
            }
        }
    }

    /// Enquanto a interface está aberta: mensagens, heartbeat e o jogo em segundo plano.
    fn watch_ui(&mut self, ui: &mut UiProc) -> i32 {
        let mut last_poll = Instant::now();
        loop {
            match ui.rx.recv_timeout(Duration::from_millis(500)) {
                Ok(msg) => self.on_message(msg, ui),
                Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => {}
            }
            if let Ok(Some(status)) = ui.child.try_wait() {
                // Mensagens que chegaram antes de a interface sair (ex.: o pedido de acompanhar o jogo).
                while let Ok(msg) = ui.rx.try_recv() {
                    self.on_message(msg, ui);
                }
                return status.code().unwrap_or(-1);
            }
            if let Some(url) = self.activation_requested()
                && let Some(stdin) = ui.stdin.as_mut()
            {
                let msg = if url.is_empty() {
                    SupMsg::Activate
                } else {
                    SupMsg::OpenLink { url }
                };
                let _ = protocol::write(stdin, &msg);
            }
            if let Some(reason) = self.watch.check(self.now()) {
                tracing::warn!("interface sem resposta ({reason:?}); encerrando");
                let _ = ui.child.kill();
            }
            // Jogo rodando com a interface aberta (Início durante o jogo).
            let every = if self.closing || self.launching {
                Duration::from_millis(500)
            } else {
                Running::POLL
            };
            if self.running.is_some() && last_poll.elapsed() >= every {
                last_poll = Instant::now();
                if let Some(played) = self.running.as_mut().and_then(|r| r.poll(&self.platform)) {
                    let (id, end) = self.finish_game(played);
                    if let Some(stdin) = ui.stdin.as_mut() {
                        let _ = protocol::write(
                            stdin,
                            &SupMsg::GameEnded {
                                game_id: id,
                                played_secs: played.as_secs(),
                                end: end.key().into(),
                            },
                        );
                    }
                } else if self.launching
                    && self
                        .running
                        .as_mut()
                        .is_some_and(|r| r.window_shown(&self.platform))
                {
                    self.launching = false;
                    tracing::info!("janela do jogo na tela");
                    if let Some(stdin) = ui.stdin.as_mut() {
                        let _ = protocol::write(stdin, &SupMsg::GameShown);
                    }
                }
            }
        }
    }

    fn on_message(&mut self, msg: UiMsg, ui: &mut UiProc) {
        match msg {
            UiMsg::Heartbeat { seq } => {
                let beat = UiHeartbeat {
                    process_id: ui.child.id(),
                    sequence: seq,
                };
                self.watch.heartbeat(&beat, self.now());
            }
            UiMsg::Track { req } => match Running::new(req) {
                Ok(mut r) => {
                    tracing::info!(
                        "acompanhando {} ({:?}, pasta: {}, steam: {:?}, pid: {:?})",
                        r.req.game_id,
                        r.req.strategy,
                        r.req.folder.is_some(),
                        r.req.steam_app_id,
                        r.req.root_pid
                    );
                    r.begin(&self.platform);
                    self.running = Some(r);
                    self.launching = true;
                }
                Err(e) => tracing::warn!("acompanhar o jogo: {e}"),
            },
            UiMsg::Resume => {}
            UiMsg::EndGame => {
                tracing::info!("encerrar o jogo pedido pela interface");
                if let Some(r) = self.running.as_mut() {
                    self.closing = true;
                    if let Err(e) = r.request_close(&self.platform) {
                        tracing::warn!("encerrar o jogo: {e}");
                    }
                }
            }
            UiMsg::ForceEndGame => {
                if let Some(r) = self.running.as_mut() {
                    self.closing = true;
                    if let Err(e) = r.force_close(&self.platform) {
                        tracing::warn!("forçar o encerramento do jogo: {e}");
                    }
                }
            }
            UiMsg::Pointer { hide, mouse } => {
                if let Some(r) = self.running.as_mut() {
                    r.set_pointer(&self.platform, hide, mouse);
                }
            }
            UiMsg::Closing { reason } => ui.closing = Some(reason),
        }
    }

    /// Restaura tudo e esquece o jogo. Devolve o id.
    fn finish_game(&mut self, played: Duration) -> (String, GameEnd) {
        let Some(mut r) = self.running.take() else {
            return (String::new(), GameEnd::Normal);
        };
        self.closing = false;
        let end = r.end(played);
        r.finish(&self.platform, &self.paths);
        tracing::info!("jogo terminou: {} ({})", r.req.game_id, end.key());
        (r.req.game_id.clone(), end)
    }

    /// Sem interface, durante o jogo. Sem polling ativo: espera eventos do controle por até 2 s (16 ms só com o
    /// ponteiro pelo controle ligado). Devolve como reabrir a interface.
    fn game_loop(&mut self) -> Launch {
        if self.pads.is_none() {
            self.pads = Pads::open()
                .map_err(|e| tracing::warn!("controles: {e}"))
                .ok();
        }
        // Atalho de teclado da Central (só Windows): marca e acorda a espera dos controles.
        let pressed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let _hotkey = {
            let pressed = pressed.clone();
            let wake = self.pads.as_ref().and_then(Pads::waker);
            tako_platform::global_hotkey(&self.key, move || {
                pressed.store(true, std::sync::atomic::Ordering::Relaxed);
                if let Some(w) = &wake {
                    w();
                }
            })
        };
        let mut last_poll = Instant::now();
        let mut last = Instant::now();
        let mut last_buttons = 0;
        loop {
            let mouse = self
                .running
                .as_ref()
                .is_some_and(|r| r.req.controller_mouse);
            // Segurando o atalho do controle não chega evento novo: sem acordar antes, ele só disparava depois
            // dos 2 s da espera (e nada, se soltasse antes).
            let wait = if mouse {
                Duration::from_millis(16)
            } else if self.chord.holding() {
                Duration::from_millis(50)
            } else {
                Running::POLL
            };
            if let Some(p) = self.pads.as_mut() {
                let _ = p.wait(wait);
            } else {
                std::thread::sleep(wait);
            }
            let now = self.now();
            let dt = last.elapsed().as_secs_f64();
            last = Instant::now();
            let pad = self.pads.as_ref().map(|p| p.state()).unwrap_or_default();
            if pad.buttons != last_buttons {
                tracing::debug!("controle durante o jogo: botões {:#x}", pad.buttons);
                last_buttons = pad.buttons;
            }
            // Atalho do controle, do teclado ou o Tako aberto de novo: a interface volta por cima do jogo.
            let activation = self.activation_requested();
            if self.chord.update(GamepadButtons(pad.buttons), now)
                || pressed.swap(false, std::sync::atomic::Ordering::Relaxed)
                || activation.is_some()
            {
                return Launch {
                    control_center: activation.as_ref().is_none_or(String::is_empty),
                    link: activation.filter(|s| !s.is_empty()),
                    ..Default::default()
                };
            }
            if let Some(r) = self.running.as_mut() {
                r.pointer_step(&self.platform, &pad, now, dt);
            }
            if last_poll.elapsed() >= Running::POLL {
                last_poll = Instant::now();
                if let Some(played) = self.running.as_mut().and_then(|r| r.poll(&self.platform)) {
                    let (id, end) = self.finish_game(played);
                    return Launch {
                        after_game: Some((id, played, end)),
                        control_center: false,
                        link: None,
                    };
                }
            }
            if self.running.is_none() {
                return Launch::default();
            }
        }
    }
}
