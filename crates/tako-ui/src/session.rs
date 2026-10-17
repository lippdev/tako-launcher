//! Sessão de jogo (MIG-024, ADR-0026). Começa na interface: resolve as configurações, grava o estado de
//! telas e áudio em disco, aplica a tela de jogo, a saída de áudio e o cursor, e abre o jogo. Daí em diante
//! o acompanhamento ([`Running`]) pode rodar no supervisor, que restaura tudo a partir do disco no fim.

use crate::shell::Ctx;
use crate::shell::ctx::keys;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tako_core::devices::DisplayMode;
use tako_core::library::GameTrackingStrategy;
use tako_core::paths::DataPaths;
use tako_core::session::audio::AudioOutputChoice;
use tako_core::session::restore::{Env, PendingEnvironmentStore, SessionEnvironmentGuard};
use tako_core::session::screen::{GameScreenOutcome, GameScreenSession};
use tako_core::session::settings::{
    EffectiveSessionSettings, GameSettingsOverride, GlobalSessionSettings, resolve,
};
use tako_core::session::tracking::{GameRunStatus, GameRunTracker, TrackingPlan, plan};
use tako_platform::Platform;

/// O que o supervisor precisa para acompanhar o jogo depois que a interface fecha.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackRequest {
    pub game_id: String,
    pub title: String,
    pub root_pid: Option<u32>,
    pub strategy: GameTrackingStrategy,
    pub folder: Option<String>,
    pub steam_app_id: Option<u32>,
    pub launched_at: DateTime<Utc>,
    pub hide_cursor: bool,
    pub controller_mouse: bool,
    /// Telas e áudio foram registrados em disco e precisam voltar no fim.
    pub restore: bool,
    /// Mostrar o jogo no Discord (EXT-004).
    #[serde(default)]
    pub discord: bool,
}

/// Resultado de abrir um jogo.
pub enum Started {
    /// O jogo abriu e há como saber quando termina.
    Tracking(TrackRequest),
    /// Abriu, mas não dá para acompanhar (URI sem processo nem pasta): só abre, como o C#.
    Untracked,
}

pub fn global_settings(ctx: &Ctx) -> GlobalSessionSettings {
    let s = &ctx.settings;
    let audio_id = s.string(keys::GAME_AUDIO_ID);
    GlobalSessionSettings {
        game_screen_enabled: s.bool(keys::GAME_SCREEN_ENABLED, false),
        game_screen_id: s.string(keys::GAME_SCREEN_ID),
        game_screen_mode: s.get(keys::GAME_SCREEN_MODE).and_then(DisplayMode::parse),
        game_screen_hdr: s.bool(keys::GAME_SCREEN_HDR, false),
        game_audio: (!audio_id.trim().is_empty()).then(|| {
            let name = s.string(keys::GAME_AUDIO_NAME);
            AudioOutputChoice::new(&audio_id, if name.is_empty() { &audio_id } else { &name })
        }),
    }
}

pub fn effective(ctx: &Ctx, game_id: &str) -> EffectiveSessionSettings {
    let game: Option<GameSettingsOverride> = ctx
        .game_settings
        .as_ref()
        .and_then(|s| s.load(game_id).ok());
    resolve(&global_settings(ctx), game.as_ref())
}

fn guard(paths: &DataPaths) -> SessionEnvironmentGuard {
    SessionEnvironmentGuard::new(PendingEnvironmentStore::new(paths.session_environment()))
}

/// Abre o jogo com tela, áudio e cursor da sessão. Em erro, desfaz o que já mudou.
pub fn start(ctx: &mut Ctx, game_id: &str) -> Result<Started, String> {
    let game = ctx
        .library
        .find(game_id)
        .cloned()
        .ok_or_else(|| ctx.t("session.ui.not_found"))?;
    let target = game
        .launch_target
        .clone()
        .ok_or_else(|| ctx.t("game.missing_file"))?;
    let eff = effective(ctx, game_id);
    if let Some(p) = eff.game_screen_problem {
        ctx.toast(p.message());
    }
    let p = &ctx.platform;
    let env = Env {
        displays: p.displays.as_ref(),
        audio: p.audio.as_ref(),
    };
    let restore = ctx.settings.bool(keys::RESTORE_ENVIRONMENT, true);
    let mut g = guard(&ctx.paths);
    let recorded = g
        .begin_session(env, &game.id, restore, Utc::now())
        .is_some();

    if let Some(screen) = &eff.game_screen {
        let confirmed = ctx
            .settings
            .get(keys::GAME_SCREEN_CONFIRMED)
            .map(str::to_owned);
        let confirmed = confirmed.as_deref().and_then(|c| {
            tako_core::session::settings::game_screen_confirmations::matching(
                Some(c),
                &screen.setup_key(),
            )
        });
        let mut session = GameScreenSession::default();
        let result = session.begin(p.displays.as_ref(), screen, confirmed.as_deref());
        if result.outcome == GameScreenOutcome::Reverted {
            g.discard();
            return Err(result
                .reason
                .unwrap_or_else(|| ctx.t("session.ui.screen_failed")));
        }
        if result.needs_confirmation && result.outcome == GameScreenOutcome::Applied {
            // Primeira vez com esta tela/modo: lembra que o usuário viu a imagem (a abertura do jogo confirma).
            let saved = tako_core::session::settings::game_screen_confirmations::remember(
                ctx.settings.get(keys::GAME_SCREEN_CONFIRMED),
                &screen.setup_key(),
            );
            ctx.settings.set(keys::GAME_SCREEN_CONFIRMED, &saved);
        }
    }
    if let Some(choice) = &eff.game_audio
        && let Err(e) = tako_core::session::audio::apply(p.audio.as_ref(), choice)
    {
        tracing::warn!("áudio do jogo: {e}");
    }

    let args = eff.arguments(game.launch_arguments.as_deref());
    let cwd = eff
        .working_directory
        .clone()
        .or(game.working_directory.clone());
    let pid = match p.processes.launch(
        &target,
        (!args.is_empty()).then_some(args.as_str()),
        cwd.as_deref(),
    ) {
        Ok(pid) => pid,
        Err(e) => {
            restore_now(&ctx.platform, &ctx.paths, recorded);
            return Err(
                tako_core::presentation::library_text::client_missing(&game, false, None)
                    .unwrap_or(e.to_string()),
            );
        }
    };
    let _ = ctx.library.record_launch(&game.id);
    let tp = plan(&game, pid.is_some());
    if tp.strategy == GameTrackingStrategy::None {
        restore_now(&ctx.platform, &ctx.paths, recorded);
        return Ok(Started::Untracked);
    }
    let req = TrackRequest {
        game_id: game.id.clone(),
        title: game.title.clone(),
        root_pid: pid,
        strategy: tp.strategy,
        folder: tp.folder,
        steam_app_id: tp.steam_app_id,
        launched_at: Utc::now(),
        hide_cursor: ctx.settings.bool(keys::HIDE_CURSOR, false),
        controller_mouse: ctx.settings.bool(keys::CONTROLLER_MOUSE, false),
        restore: recorded,
        discord: crate::discord::available() && ctx.settings.bool(keys::DISCORD_PRESENCE, false),
    };
    Ok(Started::Tracking(req))
}

/// Volta telas e áudio ao que estava antes do jogo (lido do disco) e mostra o cursor. Idempotente.
pub fn restore_now(platform: &Platform, paths: &DataPaths, recorded: bool) {
    if platform.cursor.is_hidden() {
        let _ = platform.cursor.set_hidden(false);
    }
    let store = PendingEnvironmentStore::new(paths.session_environment());
    if !recorded {
        store.delete();
        return;
    }
    let env = Env {
        displays: platform.displays.as_ref(),
        audio: platform.audio.as_ref(),
    };
    let g = guard(paths);
    if let Some(snapshot) = store.read() {
        let report = g.restore_from(env, &snapshot, Utc::now());
        if !report.succeeded() {
            tracing::warn!("restauração incompleta: {report:?}");
        }
    }
    store.delete();
}

/// Ao abrir: restaura o que uma sessão anterior deixou (queda do Tako ou do PC no meio do jogo).
pub fn recover_after_crash(platform: &Platform, paths: &DataPaths) {
    platform.cursor.recover_after_crash();
    let env = Env {
        displays: platform.displays.as_ref(),
        audio: platform.audio.as_ref(),
    };
    let g = guard(paths);
    if let Some(pending) = g.pending_from_previous_run(env, Utc::now()) {
        tracing::info!("restaurando telas e áudio de uma sessão que não terminou");
        let _ = g.restore_from(env, &pending, Utc::now());
        PendingEnvironmentStore::new(paths.session_environment()).delete();
    }
}

/// Acompanhamento do jogo (no supervisor ou na própria interface sem supervisor).
pub struct Running {
    pub req: TrackRequest,
    tracker: GameRunTracker,
    mouse: tako_core::input::controller_mouse::ControllerMouse,
    screen_height: u32,
    presence: Option<crate::discord::Presence>,
}

impl Running {
    pub const POLL: Duration = Duration::from_secs(2);

    pub fn new(req: TrackRequest) -> Result<Running, String> {
        let plan = TrackingPlan {
            strategy: req.strategy,
            folder: req.folder.clone(),
            steam_app_id: req.steam_app_id,
        };
        let tracker = GameRunTracker::new(plan, req.launched_at, req.root_pid)?;
        Ok(Running {
            req,
            tracker,
            mouse: Default::default(),
            screen_height: 1080,
            presence: None,
        })
    }

    /// Cursor e presença no Discord da sessão. Ficam com quem acompanha o jogo: no X11 o cursor escondido (e no
    /// Discord a atividade) some quando a conexão de quem pediu fecha.
    pub fn begin(&mut self, platform: &Platform) {
        if self.req.discord {
            self.presence = crate::discord::Presence::start(&self.req.title, self.req.launched_at);
        }
        self.screen_height = platform
            .displays
            .list()
            .ok()
            .and_then(|d| d.into_iter().find(|d| d.is_primary))
            .and_then(|d| d.bounds)
            .map(|b| b.height.max(1) as u32)
            .unwrap_or(1080);
        if self.req.hide_cursor
            && let Err(e) = platform.cursor.set_hidden(true)
        {
            tracing::warn!("esconder o cursor: {e}");
        }
    }

    pub fn set_pointer(&mut self, platform: &Platform, hide: bool, mouse: bool) {
        self.req.hide_cursor = hide;
        if platform.cursor.is_hidden() != hide {
            let _ = platform.cursor.set_hidden(hide);
        }
        if self.req.controller_mouse && !mouse {
            let _ = self.mouse.reset(platform.pointer.as_ref());
        }
        self.req.controller_mouse = mouse;
    }

    /// Uma amostra do controle para o ponteiro (só com "Mover com o analógico" ligado).
    pub fn pointer_step(
        &mut self,
        platform: &Platform,
        pad: &crate::gamepad::PadState,
        now: Duration,
        dt: f64,
    ) {
        if !self.req.controller_mouse {
            return;
        }
        let a = pad.buttons & crate::gamepad::buttons::A != 0;
        if let Err(e) = self.mouse.step(
            platform.pointer.as_ref(),
            (pad.left_x as f64, -pad.left_y as f64),
            (pad.right_x as f64, -pad.right_y as f64),
            a,
            now,
            dt,
            self.screen_height,
        ) {
            tracing::warn!("ponteiro pelo controle: {e}");
            self.req.controller_mouse = false;
        }
    }

    /// Fim da sessão: solta o ponteiro e restaura telas, áudio e cursor.
    pub fn finish(&mut self, platform: &Platform, paths: &DataPaths) {
        self.presence = None;
        let _ = self.mouse.reset(platform.pointer.as_ref());
        restore_now(platform, paths, self.req.restore);
    }

    /// Uma leitura dos processos. `Some(tempo jogado)` quando o jogo terminou.
    pub fn poll(&mut self, platform: &Platform) -> Option<Duration> {
        let steam =
            (self.req.steam_app_id.is_some()).then(|| platform.processes.steam_running_app_id());
        let status = self
            .tracker
            .update(&platform.processes.snapshot(), Utc::now(), steam);
        match status {
            GameRunStatus::Exited => Some(self.tracker.play_time().to_std().unwrap_or_default()),
            GameRunStatus::TimedOut => Some(Duration::ZERO),
            _ => None,
        }
    }

    /// O jogo não fechou com o pedido: encerra os processos dele à força.
    pub fn force_close(&mut self, platform: &Platform) -> Result<(), String> {
        self.tracker.exit_grace = chrono::TimeDelta::seconds(1);
        self.tracker.close_requested = true;
        platform
            .processes
            .force_close(&self.game_pids())
            .map_err(|e| e.to_string())
    }

    pub fn game_pids(&self) -> Vec<u32> {
        self.tracker.game_process_ids().to_vec()
    }

    /// Pede para o jogo fechar (Central › Encerrar jogo). Daí em diante o fim conta assim que os processos somem
    /// (sem a folga da troca de launcher para jogo).
    pub fn request_close(&mut self, platform: &Platform) -> Result<(), String> {
        self.tracker.exit_grace = chrono::TimeDelta::seconds(1);
        self.tracker.close_requested = true;
        platform
            .processes
            .request_close(&self.game_pids())
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::build_ctx;

    #[test]
    fn starts_tracks_and_restores() {
        let dir = tempfile::tempdir().unwrap();
        let platform = Platform::simulated();
        let mut ctx = build_ctx(DataPaths::at(dir.path()), platform.clone(), true);
        ctx.library.refresh();
        let id = ctx
            .library
            .games()
            .iter()
            .find(|g| g.is_installed)
            .unwrap()
            .id
            .clone();
        ctx.settings.set_bool(keys::HIDE_CURSOR, true);
        let Started::Tracking(req) = start(&mut ctx, &id).unwrap() else {
            panic!("deveria acompanhar")
        };
        let mut running = Running::new(req.clone()).unwrap();
        running.begin(&platform);
        assert!(platform.cursor.is_hidden());
        assert_eq!(running.poll(&platform), None);
        // O jogo fecha: depois da folga, termina.
        let pid = req.root_pid.unwrap();
        platform.processes.request_close(&[pid]).unwrap();
        running.tracker.exit_grace = chrono::TimeDelta::zero();
        assert!(running.poll(&platform).is_some());
        restore_now(&platform, &ctx.paths, req.restore);
        assert!(!platform.cursor.is_hidden());
        assert!(!ctx.paths.session_environment().exists());
    }
}
