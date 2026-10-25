//! Sessão de jogo na versão web (WEB-20, ADR-0026, D-10). Vive no núcleo (`tako --web`), que sobrevive à interface
//! como o supervisor sobrevive à interface Slint. A regra é a de `tako_core::session::{launch, running}`, a mesma do
//! app Slint; aqui ficam o pedido vindo da ponte, as configurações do núcleo e os eventos.
//!
//! Quem chama é o hospedeiro, na thread do núcleo: [`AppService::launch_game`] ao receber `HostRequest::Launch`, e
//! [`AppService::poll_session`] até [`AppService::session_deadline`], sem bloquear. A janela fica com ele: fecha a
//! interface em [`SessionUpdate::Shown`] e em `HostRequest::Resume`, reabre em [`SessionUpdate::Ended`].
//!
//! Na demonstração (`simulate`) a sessão usa as funções em memória do núcleo e uma pasta temporária: não abre nada,
//! não mexe em tela, áudio nem cursor e não grava o diário da sessão.

use crate::protocol::*;
use crate::{AppService, HostRequest};
use chrono::SecondsFormat;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tako_core::devices::fake::{FakeAudio, FakeDisplays, FakePointer, FakeProcesses};
use tako_core::library::GameStore;
use tako_core::paths::DataPaths;
use tako_core::persistence::{GameSettingsStore, TakoDatabase};
use tako_core::session::launch::{self, LaunchReport, LaunchRequest, Started, TrackRequest, keys};
use tako_core::session::tracking::exit_code_text;
use tako_core::session::{GameEnd, GameSession, SessionDevices};
use tako_platform::Platform;

/// Com o jogo abrindo ou fechando, a leitura é mais frequente (como o supervisor).
const QUICK_POLL: Duration = Duration::from_millis(500);

/// Na demonstração o jogo simulado fecha sozinho depois deste tempo, para o ciclo inteiro aparecer (a interface
/// fecha, o jogo "termina", a interface volta com o tempo jogado).
const DEMO_PLAY: Duration = Duration::from_secs(15);

/// O que mudou numa leitura do jogo; o hospedeiro decide a janela.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionUpdate {
    /// A janela do jogo apareceu: a interface pode fechar (D-10).
    Shown,
    /// O jogo terminou: tela, áudio e tempo jogado já foram tratados; a interface volta.
    Ended,
}

struct Active {
    game: GameSession,
    state: RunningState,
    opened: Instant,
    last_poll: Instant,
}

/// O que a sessão precisa do núcleo, separado do resto do [`crate::Core`].
pub(crate) struct Sessions {
    platform: Platform,
    paths: DataPaths,
    store: Option<GameSettingsStore>,
    simulate: bool,
    active: Option<Active>,
    /// Fim que a interface fechada não viu: sai logo depois da próxima resposta de `game.running`.
    unseen_end: Option<GameEnded>,
    /// `game.play` aceito e o `HostRequest::Launch` ainda na fila: um segundo `game.play` dá `conflict`.
    launching: bool,
}

impl Sessions {
    /// Fora da demonstração, restaura antes de tudo o que uma queda do núcleo no meio do jogo deixou.
    pub(crate) fn open(
        platform: &Platform,
        paths: &DataPaths,
        database: Option<&TakoDatabase>,
        simulate: bool,
    ) -> Sessions {
        let sessions = if simulate {
            let pointer = Arc::new(FakePointer::default());
            Sessions {
                platform: Platform {
                    displays: Arc::new(FakeDisplays::desk()),
                    audio: Arc::new(FakeAudio::desk()),
                    cursor: pointer.clone(),
                    pointer,
                    processes: Arc::new(FakeProcesses::default()),
                    controller_hiding: Arc::new(tako_core::devices::NoHiding),
                    ..platform.clone()
                },
                paths: DataPaths::at(
                    std::env::temp_dir().join(format!("tako-demo-session-{}", std::process::id())),
                ),
                store: None,
                simulate,
                active: None,
                unseen_end: None,
                launching: false,
            }
        } else {
            Sessions {
                platform: platform.clone(),
                paths: paths.clone(),
                store: database.cloned().map(GameSettingsStore::new),
                simulate,
                active: None,
                unseen_end: None,
                launching: false,
            }
        };
        if !simulate && launch::recover_after_crash(&sessions.devices(), &sessions.paths) {
            tracing::info!("tela e áudio restaurados de um jogo que não terminou");
        }
        sessions
    }

    fn devices(&self) -> SessionDevices<'_> {
        devices(&self.platform)
    }
}

fn devices(platform: &Platform) -> SessionDevices<'_> {
    SessionDevices {
        displays: platform.displays.as_ref(),
        audio: platform.audio.as_ref(),
        processes: platform.processes.as_ref(),
        cursor: platform.cursor.as_ref(),
        pointer: platform.pointer.as_ref(),
        controller_hiding: platform.controller_hiding.as_ref(),
    }
}

fn iso(at: chrono::DateTime<chrono::Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn conflict(message: &str) -> BridgeError {
    BridgeError::new(ErrorCode::Conflict, message)
}

fn reason(end: GameEnd) -> GameEndReason {
    match end {
        GameEnd::Normal => GameEndReason::Normal,
        GameEnd::Quick(_) => GameEndReason::Quick,
        GameEnd::NeverStarted => GameEndReason::NeverStarted,
        GameEnd::Failed(_) => GameEndReason::Failed,
    }
}

fn ended(req: &TrackRequest, played: Duration, end: GameEnd) -> GameEnded {
    GameEnded {
        game_id: req.game_id.clone(),
        title: req.title.clone(),
        played_seconds: played.as_secs(),
        end: reason(end),
        exit_code: end.exit_code(),
        exit_code_text: end.exit_code().map(exit_code_text),
        message: None,
    }
}

fn launch_failed(game_id: &str, title: &str, message: String) -> GameEnded {
    GameEnded {
        game_id: game_id.into(),
        title: title.into(),
        played_seconds: 0,
        end: GameEndReason::LaunchFailed,
        exit_code: None,
        exit_code_text: None,
        message: Some(message),
    }
}

/// A ROM adicionada à mão com o comando do emulador do sistema dela (SRC-001), como no app Slint.
fn with_emulator(
    settings: &std::collections::HashMap<String, String>,
    mut game: tako_core::library::Game,
) -> Result<tako_core::library::Game, String> {
    use tako_sources::emulators::{Config, command_for, keys};
    let config = Config::parse(
        settings.get(keys::ROM_FOLDERS).map(String::as_str),
        settings.get(keys::PATHS).map(String::as_str),
        settings.get(keys::BY_SYSTEM).map(String::as_str),
    );
    let rom = game.rom_path.clone().unwrap_or_default();
    let Some((target, args, dir)) = command_for(
        std::path::Path::new(&rom),
        game.platform.as_deref(),
        &config,
    ) else {
        return Err(tako_core::i18n::t("session.no_emulator"));
    };
    game.launch_target = Some(target);
    game.launch_arguments = Some(args);
    game.working_directory = dir.or(game.working_directory);
    Ok(game)
}

impl AppService {
    /// `game.play` aceito: valida aqui, e a abertura acontece no hospedeiro (`HostRequest::Launch`), depois da
    /// resposta. Um jogo por vez.
    pub(crate) fn play(&mut self, request: GameRequest) -> Result<Reply, BridgeError> {
        if !self.find_game(&request.game_id)?.can_play() {
            return Err(conflict("jogo não pode ser iniciado"));
        }
        if self
            .core
            .as_ref()
            .is_some_and(|c| c.session.active.is_some() || c.session.launching)
        {
            return Err(conflict("já há um jogo aberto"));
        }
        self.request(HostRequest::Launch {
            game_id: request.game_id,
        })?;
        if let Some(core) = self.core.as_mut() {
            core.session.launching = true;
        }
        Ok(Reply::GamePlay)
    }

    /// Abre o jogo de `game.play` com tela, áudio e cursor da sessão (`HostRequest::Launch`). Avisa `game.started`,
    /// ou `game.ended` com `launch_failed` quando ele não abre.
    pub fn launch_game(&mut self, game_id: &str) {
        let Some(core) = self.core.as_mut() else {
            return;
        };
        core.session.launching = false;
        if core.session.active.is_some() {
            tracing::warn!(%game_id, "outro jogo já está aberto; pedido ignorado");
            return;
        }
        let Some(game) = core.library.find(game_id).cloned() else {
            let message = tako_core::i18n::t("session.ui.not_found");
            self.fail_launch(launch_failed(game_id, "", message));
            return;
        };
        let title = game.title.clone();
        let game = if game.store() == GameStore::Manual && game.is_rom() {
            match with_emulator(&core.settings, game) {
                Ok(game) => game,
                Err(message) => {
                    self.fail_launch(launch_failed(game_id, &title, message));
                    return;
                }
            }
        } else {
            game
        };
        let get = |k: &str| core.settings.get(k).cloned();
        let eff = launch::effective(
            &launch::global_settings(get),
            core.session.store.as_ref(),
            game_id,
        );
        let mut request = LaunchRequest::new(game, eff, get);
        // A página da versão web é a do modo desktop: marca "Jogando", como o Slint no desktop.
        request.mark_playing = true;
        // Ainda sem controle nem Discord no núcleo web (WEB-22): o ponteiro pelo analógico não teria quem o mova.
        request.controller_mouse = false;
        request.discord = false;
        if core.session.simulate {
            request.restore_environment = false;
        }
        let before = core.library.version();
        let mut report = LaunchReport::default();
        let sessions = &mut core.session;
        let result = launch::launch(
            &devices(&sessions.platform),
            &sessions.paths,
            &mut core.library,
            request,
            &mut report,
        );
        if let Some(saved) = &report.screen_confirmations
            && let Err(e) = crate::settings::save(core, keys::GAME_SCREEN_CONFIRMED, saved)
        {
            tracing::warn!("tela de jogo confirmada: {e}");
        }
        let changed = core.library.version() != before;
        let started = match result {
            Ok(Started::Tracking(req)) => match GameSession::new(req.clone()) {
                Ok(mut game) => {
                    game.begin(&core.session.devices());
                    tracing::info!(
                        "acompanhando {} ({:?}, pasta: {}, steam: {:?}, pid: {:?})",
                        req.game_id,
                        req.strategy,
                        req.folder.is_some(),
                        req.steam_app_id,
                        req.root_pid
                    );
                    core.session.active = Some(Active {
                        game,
                        state: RunningState::Launching,
                        opened: Instant::now(),
                        last_poll: Instant::now(),
                    });
                    Ok(GameStarted {
                        game_id: req.game_id,
                        title: req.title,
                        started_at: iso(req.launched_at),
                        tracked: true,
                        notices: report.notices,
                    })
                }
                Err(e) => {
                    tracing::warn!("acompanhar o jogo: {e}");
                    let s = &core.session;
                    launch::restore_now(&s.devices(), &s.paths, req.restore);
                    Err(e)
                }
            },
            Ok(Started::Untracked) => Ok(GameStarted {
                game_id: game_id.into(),
                title: title.clone(),
                started_at: iso(chrono::Utc::now()),
                tracked: false,
                notices: report.notices,
            }),
            Err(message) => Err(message),
        };
        if changed {
            self.emit(Event::LibraryChanged);
        }
        match started {
            Ok(started) => self.emit(Event::GameStarted(started)),
            Err(message) => {
                tracing::warn!(%game_id, "o jogo não abriu: {message}");
                self.fail_launch(launch_failed(game_id, &title, message));
            }
        }
    }

    /// Quando chamar [`Self::poll_session`] de novo; `None` sem jogo aberto.
    pub fn session_deadline(&self) -> Option<Instant> {
        let active = self.core.as_ref()?.session.active.as_ref()?;
        let every = match active.state {
            RunningState::Playing => GameSession::POLL,
            RunningState::Launching | RunningState::Closing => QUICK_POLL,
        };
        Some(active.last_poll + every)
    }

    /// Há um jogo aberto e acompanhado.
    pub fn game_active(&self) -> bool {
        self.core
            .as_ref()
            .is_some_and(|c| c.session.active.is_some())
    }

    /// Uma leitura do jogo, se já for hora ([`Self::session_deadline`]). No fim restaura tela e áudio, registra o
    /// tempo jogado e avisa `game.ended`.
    pub fn poll_session(&mut self, now: Instant) -> Option<SessionUpdate> {
        if self.session_deadline().is_none_or(|at| now < at) {
            return None;
        }
        let core = self.core.as_mut()?;
        let sessions = &mut core.session;
        let d = devices(&sessions.platform);
        let active = sessions.active.as_mut()?;
        active.last_poll = now;
        if sessions.simulate
            && active.state != RunningState::Closing
            && now.duration_since(active.opened) >= DEMO_PLAY
        {
            tracing::info!("demonstração: o jogo simulado fecha sozinho");
            let _ = active.game.request_close(&d);
            active.state = RunningState::Closing;
        }
        let Some(played) = active.game.poll(&d) else {
            if active.state == RunningState::Launching && active.game.window_shown(&d) {
                tracing::info!("janela do jogo na tela");
                active.state = RunningState::Playing;
                return Some(SessionUpdate::Shown);
            }
            return None;
        };
        let end = active.game.end(&d, played);
        active.game.finish(&d, &sessions.paths);
        let req = active.game.req.clone();
        sessions.active = None;
        tracing::info!(
            "jogo terminou: {} ({}, {} s jogados)",
            req.game_id,
            end.key(),
            played.as_secs()
        );
        if let Err(e) = core.library.record_session(&req.game_id, played) {
            tracing::warn!("tempo de jogo: {e}");
        }
        let event = ended(&req, played, end);
        core.session.unseen_end = Some(event.clone());
        self.emit(Event::LibraryChanged);
        self.emit(Event::GameEnded(event));
        Some(SessionUpdate::Ended)
    }

    /// `game.ended` com `launch_failed`, guardado também para a interface que reabrir (como o fim normal).
    fn fail_launch(&mut self, event: GameEnded) {
        if let Some(core) = self.core.as_mut() {
            core.session.unseen_end = Some(event.clone());
        }
        self.emit(Event::GameEnded(event));
    }

    /// O hospedeiro entregou o último `game.ended` a uma interface aberta: não repetir no próximo `game.running`.
    pub fn session_end_seen(&mut self) {
        if let Some(core) = self.core.as_mut() {
            core.session.unseen_end = None;
        }
    }

    pub(crate) fn running(&mut self) -> Result<Reply, BridgeError> {
        let core = self.require_core()?;
        let running = core.session.active.as_ref().map(|a| RunningGame {
            game_id: a.game.req.game_id.clone(),
            title: a.game.req.title.clone(),
            started_at: iso(a.game.req.launched_at),
            state: a.state,
        });
        // A resposta sai antes deste evento (mesma fila): quem consulta ao reabrir já está escutando.
        if let Some(end) = self.core.as_mut().and_then(|c| c.session.unseen_end.take()) {
            self.emit(Event::GameEnded(end));
        }
        Ok(Reply::GameRunning(running))
    }

    pub(crate) fn resume(&mut self) -> Result<Reply, BridgeError> {
        if !self.game_active() {
            return Err(conflict("nenhum jogo aberto"));
        }
        self.request(HostRequest::Resume)?;
        Ok(Reply::GameResume)
    }

    pub(crate) fn end_game(&mut self, request: GameEndRequest) -> Result<Reply, BridgeError> {
        let core = self
            .core
            .as_mut()
            .ok_or_else(|| BridgeError::new(ErrorCode::Internal, "núcleo não aberto"))?;
        let sessions = &mut core.session;
        let d = devices(&sessions.platform);
        let active = sessions
            .active
            .as_mut()
            .ok_or_else(|| conflict("nenhum jogo aberto"))?;
        tracing::info!(
            force = request.force,
            "encerrar o jogo pedido pela interface"
        );
        active.state = RunningState::Closing;
        let result = if request.force {
            active.game.force_close(&d)
        } else {
            active.game.request_close(&d)
        };
        // Pedido válido que não deu agora (a janela do jogo ainda não existe, o jogo não respondeu): a sessão segue em
        // "fechando" e a interface pode tentar de novo ou forçar.
        result.map_err(|e| conflict(&format!("encerrar o jogo: {e}")))?;
        Ok(Reply::GameEnd)
    }
}
