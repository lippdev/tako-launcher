//! Acompanhamento do jogo aberto por [`super::launch::launch`] (ADR-0026): segue os processos, avisa como terminou e,
//! no fim, roda o comando de depois, fecha as ferramentas e restaura tela, áudio e cursor. Roda no supervisor e na
//! interface Slint sem supervisor (`tako_ui::session::Running`, que acrescenta o Discord e o controle) e no núcleo
//! da versão web (`tako-app`).

use super::events::{self, SessionEvent, SessionGame};
use super::launch::{SessionDevices, TrackRequest, close_tools, restore_now};
use super::tracking::{GameEnd, GameRunStatus, GameRunTracker, TrackingPlan};
use crate::devices::GameWindow;
use crate::input::controller_mouse::ControllerMouse;
use crate::library::{GameStore, GameTrackingStrategy};
use crate::paths::DataPaths;
use chrono::Utc;
use std::time::Duration;

/// Uma sessão de jogo em andamento.
pub struct GameSession {
    pub req: TrackRequest,
    pub tracker: GameRunTracker,
    mouse: ControllerMouse,
    screen_height: u32,
    /// Desde quando o jogo mostra uma janela que não cobre a tela (ver [`GameSession::window_shown`]).
    windowed_since: Option<std::time::Instant>,
    /// Últimos processos do jogo registrados no log (BUG-001), para logar de novo só quando mudarem.
    logged_paths: Vec<String>,
}

impl GameSession {
    pub const POLL: Duration = Duration::from_millis(250);
    pub const OPENING_POLL: Duration = Duration::from_millis(500);
    /// Jogo em janela: quanto esperar antes de sair de "Abrindo…".
    pub const WINDOWED_GRACE: Duration = Duration::from_secs(4);

    /// Só acelera a leitura depois da janela confirmada; não cria trabalho quando não há sessão.
    pub fn poll_interval(&self) -> Duration {
        if self.tracker.was_shown() {
            Self::POLL
        } else {
            Self::OPENING_POLL
        }
    }

    pub fn new(req: TrackRequest) -> Result<GameSession, String> {
        let plan = TrackingPlan {
            strategy: req.strategy,
            folder: req.folder.clone(),
            steam_app_id: req.steam_app_id,
        };
        let tracker = GameRunTracker::new(plan, req.launched_at, req.root_pid)?;
        Ok(GameSession {
            req,
            tracker,
            mouse: Default::default(),
            screen_height: 1080,
            windowed_since: None,
            logged_paths: Vec::new(),
        })
    }

    /// Cursor da sessão. Fica com quem acompanha o jogo: no X11 o cursor escondido volta quando a conexão de quem
    /// pediu fecha.
    pub fn begin(&mut self, devices: &SessionDevices) {
        // Segura o processo raiz para ler o código de saída no fim (SESSION-003).
        if let Some(pid) = self.req.root_pid {
            devices.processes.watch_exit(pid);
        }
        self.screen_height = devices
            .displays
            .list()
            .ok()
            .and_then(|d| d.into_iter().find(|d| d.is_primary))
            .and_then(|d| d.bounds)
            .map(|b| b.height.max(1) as u32)
            .unwrap_or(1080);
        if self.req.hide_cursor
            && let Err(e) = devices.cursor.set_hidden(true)
        {
            tracing::warn!("esconder o cursor: {e}");
        }
    }

    pub fn set_pointer(&mut self, devices: &SessionDevices, hide: bool, mouse: bool) {
        self.req.hide_cursor = hide;
        if devices.cursor.is_hidden() != hide {
            let _ = devices.cursor.set_hidden(hide);
        }
        if self.req.controller_mouse && !mouse {
            let _ = self.mouse.reset(devices.pointer);
        }
        self.req.controller_mouse = mouse;
    }

    /// Uma amostra do controle para o ponteiro (só com "Mover com o analógico" ligado). Analógicos em -1..1 com Y
    /// para cima; `a` = botão A apertado.
    pub fn pointer_step(
        &mut self,
        devices: &SessionDevices,
        left: (f64, f64),
        right: (f64, f64),
        a: bool,
        now: Duration,
        dt: f64,
    ) {
        if !self.req.controller_mouse {
            return;
        }
        if let Err(e) =
            self.mouse
                .step(devices.pointer, left, right, a, now, dt, self.screen_height)
        {
            tracing::warn!("ponteiro pelo controle: {e}");
            self.req.controller_mouse = false;
        }
    }

    /// Fim da sessão: roda o comando de depois, fecha as ferramentas, solta o ponteiro e restaura telas, áudio e
    /// cursor. Pode ser chamado de novo sem efeito a mais.
    pub fn finish(&mut self, devices: &SessionDevices, paths: &DataPaths) {
        if let Some(c) = self.req.after_command.take()
            && let Err(e) = devices.processes.run_command(&c, None)
        {
            tracing::warn!("comando depois do jogo: {e}");
        }
        let tools = std::mem::take(&mut self.req.close_tools);
        close_tools(devices.processes, &tools, &self.req.keep_pids);
        let _ = self.mouse.reset(devices.pointer);
        restore_now(devices, paths, self.req.restore);
    }

    /// Uma leitura dos processos. `Some(tempo jogado)` quando o jogo terminou.
    pub fn poll(&mut self, devices: &SessionDevices) -> Option<Duration> {
        let steam =
            (self.req.steam_app_id.is_some()).then(|| devices.processes.steam_running_app_id());
        let before = self.tracker.status;
        let status = self
            .tracker
            .update(&devices.processes.snapshot(), Utc::now(), steam);
        let paths = self.tracker.game_executable_paths();
        if status != before || paths != self.logged_paths.as_slice() {
            // BUG-001: mostra se a sessão acabou cedo demais (processos fora da pasta, Steam sem confirmar).
            // Display (join), não Debug: o Debug dobra as barras e o LogRedactor não reconheceria o caminho.
            tracing::info!(
                "acompanhamento de {}: {before:?} → {status:?} ({} s desde a abertura, processos {}, steam {:?}, confirmado pela steam {})",
                self.req.game_id,
                (Utc::now() - self.req.launched_at).num_seconds(),
                paths.join(", "),
                steam,
                self.tracker.steam_confirmed
            );
            self.logged_paths = paths.to_vec();
        }
        if !self.tracker.is_finished() {
            // Também confirma no Slint sem supervisor, que não consulta `window_shown` por conta própria.
            self.window_shown(devices);
        }
        match status {
            GameRunStatus::Exited => Some(self.tracker.play_time().to_std().unwrap_or_default()),
            GameRunStatus::TimedOut => Some(Duration::ZERO),
            _ => None,
        }
    }

    /// Como terminou, depois que [`Self::poll`] devolveu o tempo jogado (SESSION-003), e avisa `GameExited`
    /// (EXT-001). O código de saída é o do processo raiz, quando o Tako abriu o executável direto (jogo aberto pela
    /// loja não tem).
    pub fn end(&self, devices: &SessionDevices, played: Duration) -> GameEnd {
        let code = self
            .req
            .root_pid
            .and_then(|pid| devices.processes.exit_code(pid));
        let end = GameEnd::of(
            self.tracker.status,
            self.tracker.close_requested,
            played,
            code,
        );
        events::publish(SessionEvent::GameExited {
            game: SessionGame {
                game_id: self.req.game_id.clone(),
                title: self.req.title.clone(),
                store: self.req.store.unwrap_or(GameStore::Manual),
            },
            at: Utc::now(),
            played: Some(played),
            exit_code: code,
        });
        end
    }

    /// O jogo não fechou com o pedido: encerra os processos dele à força.
    pub fn force_close(&mut self, devices: &SessionDevices) -> Result<(), String> {
        self.tracker.exit_grace = chrono::TimeDelta::seconds(1);
        self.tracker.close_requested = true;
        devices
            .processes
            .force_close(&self.game_pids())
            .map_err(|e| e.to_string())
    }

    /// A janela do jogo já está na tela (a interface sai de "Abrindo…"): cobrindo o monitor, ou em janela há
    /// [`Self::WINDOWED_GRACE`] (muitos jogos abrem uma janela pequena antes de ir para tela cheia). Acompanhado só
    /// pela Steam (sem processos), vale a confirmação dela.
    pub fn window_shown(&mut self, devices: &SessionDevices) -> bool {
        if self.tracker.was_shown() {
            return true;
        }
        let pids = self.game_pids();
        let shown = if pids.is_empty() {
            self.req.strategy == GameTrackingStrategy::SteamApp && self.tracker.steam_confirmed
        } else {
            match devices.processes.game_window(&pids) {
                GameWindow::Fullscreen => true,
                GameWindow::Windowed => {
                    self.windowed_since
                        .get_or_insert_with(std::time::Instant::now)
                        .elapsed()
                        >= Self::WINDOWED_GRACE
                }
                GameWindow::None => {
                    self.windowed_since = None;
                    false
                }
            }
        };
        if shown {
            self.tracker.mark_shown();
        }
        shown
    }

    pub fn game_pids(&self) -> Vec<u32> {
        self.tracker.game_process_ids().to_vec()
    }

    /// Pede para o jogo fechar (Central › Encerrar jogo). Daí em diante o fim conta assim que os processos somem
    /// (sem a folga da troca de launcher para jogo).
    pub fn request_close(&mut self, devices: &SessionDevices) -> Result<(), String> {
        self.tracker.exit_grace = chrono::TimeDelta::seconds(1);
        self.tracker.close_requested = true;
        devices
            .processes
            .request_close(&self.game_pids())
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::launch::tests::{Fakes, game, library_with, request};
    use crate::session::launch::{LaunchReport, Started, keys, launch};

    fn started(fakes: &Fakes, paths: &DataPaths, hide_cursor: bool) -> TrackRequest {
        let mut library = library_with(game("/jogos/a.exe"));
        let get = move |key: &str| (key == keys::HIDE_CURSOR && hide_cursor).then(|| "true".into());
        let r = request(game("/jogos/a.exe"), get);
        let report = &mut LaunchReport::default();
        match launch(&fakes.devices(), paths, &mut library, r, report).unwrap() {
            Started::Tracking(req) => req,
            Started::Untracked => panic!("deveria acompanhar"),
        }
    }

    #[test]
    fn polling_only_accelerates_after_the_game_window_is_confirmed() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::at(dir.path());
        let fakes = Fakes::new();
        let req = started(&fakes, &paths, false);
        let mut session = GameSession::new(req).unwrap();
        assert_eq!(session.poll_interval(), GameSession::OPENING_POLL);
        assert!(!session.tracker.was_shown());
        // A confirmação acontece no próprio núcleo, inclusive sem supervisor Slint.
        assert_eq!(session.poll(&fakes.devices()), None);
        assert!(session.tracker.was_shown());
        assert_eq!(session.poll_interval(), Duration::from_millis(250));
    }

    #[test]
    fn follows_the_game_until_it_closes_and_restores_the_cursor() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::at(dir.path());
        let fakes = Fakes::new();
        let d = fakes.devices();
        let req = started(&fakes, &paths, true);
        let mut session = GameSession::new(req.clone()).unwrap();
        session.begin(&d);
        assert!(d.cursor.is_hidden());
        assert_eq!(session.poll(&d), None);
        // O jogo fecha: depois da folga, termina.
        fakes.processes.exit_with(req.root_pid.unwrap(), 0);
        session.tracker.exit_grace = chrono::TimeDelta::zero();
        let played = session.poll(&d).unwrap();
        assert_eq!(session.end(&d, played), GameEnd::Quick(Some(0)));
        session.finish(&d, &paths);
        assert!(!d.cursor.is_hidden());
        assert!(!paths.session_environment().exists());
    }

    #[test]
    fn closing_on_request_and_by_force_ends_as_normal() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::at(dir.path());
        let fakes = Fakes::new();
        let d = fakes.devices();
        for force in [false, true] {
            let req = started(&fakes, &paths, false);
            let mut session = GameSession::new(req).unwrap();
            session.begin(&d);
            assert!(session.poll(&d).is_none());
            if force {
                session.force_close(&d).unwrap();
            } else {
                session.request_close(&d).unwrap();
            }
            assert!(d.processes.snapshot().is_empty());
            session.tracker.exit_grace = chrono::TimeDelta::zero();
            let played = session.poll(&d).unwrap();
            assert_eq!(session.end(&d, played), GameEnd::Normal);
            session.finish(&d, &paths);
        }
    }
}
