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
use tako_core::session::events::{self, SessionEvent, SessionGame};
use tako_core::session::restore::{Env, PendingEnvironmentStore, SessionEnvironmentGuard};
use tako_core::session::screen::{GameScreenOutcome, GameScreenSession};
use tako_core::session::settings::{
    EffectiveSessionSettings, GameSettingsOverride, GlobalSessionSettings, MonitorPresets, resolve,
};
use tako_core::session::tracking::{
    GameRunStatus, GameRunTracker, TrackingPlan, plan, processes_of,
};
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
    /// Comando permitido para quando o jogo terminar (SESSION-004).
    #[serde(default)]
    pub after_command: Option<String>,
    /// Loja do jogo, para os eventos de sessão (EXT-001).
    #[serde(default)]
    pub store: Option<tako_core::library::GameStore>,
    /// Ferramentas permitidas para fechar quando o jogo terminar (EXT-001).
    #[serde(default)]
    pub close_tools: Vec<String>,
    /// Processos dessas ferramentas que já rodavam antes do jogo: o usuário abriu, não são fechados.
    #[serde(default)]
    pub keep_pids: Vec<u32>,
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
        game_screen_vrr: s.bool(keys::GAME_SCREEN_VRR, false),
        monitor_presets: MonitorPresets::parse(&s.string(keys::GAME_SCREEN_PRESETS)),
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

/// A ROM com o comando do emulador; sem emulador para o sistema, o motivo.
fn with_emulator(
    ctx: &Ctx,
    mut game: tako_core::library::Game,
) -> Result<tako_core::library::Game, String> {
    let rom = game.rom_path.clone().unwrap_or_default();
    let config = crate::sources::emulator_config(&ctx.settings);
    let Some((target, args, dir)) = tako_sources::emulators::command_for(
        std::path::Path::new(&rom),
        game.platform.as_deref(),
        &config,
    ) else {
        return Err(ctx.t("session.no_emulator"));
    };
    game.launch_target = Some(target);
    // Os argumentos extras das opções do jogo continuam valendo (entram depois, pelo `eff.arguments`).
    game.launch_arguments = Some(args);
    game.working_directory = dir.or(game.working_directory);
    Ok(game)
}

/// Abre o jogo com tela, áudio e cursor da sessão. Em erro, desfaz o que já mudou.
pub fn start(ctx: &mut Ctx, game_id: &str) -> Result<Started, String> {
    let game = ctx
        .library
        .find(game_id)
        .cloned()
        .ok_or_else(|| ctx.t("session.ui.not_found"))?;
    // ROM adicionada à mão: abre pelo emulador do sistema dela (SRC-001).
    let game = if game.store() == tako_core::library::GameStore::Manual && game.is_rom() {
        with_emulator(ctx, game)?
    } else {
        game
    };
    let target = game
        .launch_target
        .clone()
        .ok_or_else(|| ctx.t("game.missing_file"))?;
    let eff = effective(ctx, game_id);
    let session_game = SessionGame {
        game_id: game.id.clone(),
        title: game.title.clone(),
        store: game.store(),
    };
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
        // `session` acaba aqui: quem desliga o VRR no fim é o guard, pelo arquivo gravado antes de ligar.
        let result = session.begin_with(
            p.displays.as_ref(),
            screen,
            confirmed.as_deref(),
            &mut |id| g.remember_vrr_turned_on(id),
        );
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
    // Daqui em diante todo `GameStarting` tem o seu `GameExited` (a tela recusada acima não conta como sessão).
    events::publish(SessionEvent::GameStarting {
        game: session_game.clone(),
        at: Utc::now(),
    });
    let exited_now = |played: Option<Duration>| {
        events::publish(SessionEvent::GameExited {
            game: session_game.clone(),
            at: Utc::now(),
            played,
            exit_code: None,
        })
    };

    // Comandos e ferramentas só rodam com a permissão do usuário (a tela pergunta antes de chamar esta função).
    // Ferramentas só são abertas, sem esperar: não seguram a interface nem o heartbeat do supervisor.
    let tools: &[String] = if eff.commands_approved {
        &eff.tools
    } else {
        &[]
    };
    let keep_pids = if tools.is_empty() {
        Vec::new()
    } else {
        processes_of(&ctx.platform.processes.snapshot(), tools)
    };
    for tool in tools {
        if let Err(e) = ctx.platform.processes.launch(tool, None, None) {
            tracing::warn!("ferramenta do jogo {tool}: {e}");
            ctx.toast(ctx.tf(
                "session.tool_failed",
                &[("tool", tool), ("error", &e.to_string())],
            ));
        }
    }
    let p = &ctx.platform;
    let before_failed = eff
        .before_command
        .as_deref()
        .filter(|_| eff.commands_approved)
        .and_then(|c| {
            p.processes
                .run_command(c, Some(tako_core::session::settings::BEFORE_COMMAND_WAIT))
                .err()
        });
    let args = eff.arguments(game.launch_arguments.as_deref());
    let cwd = eff
        .working_directory
        .clone()
        .or(game.working_directory.clone());
    let (target_to_launch, args_to_launch, env_to_launch) = {
        let is_windows_exe = target.to_lowercase().ends_with(".exe");
        let is_manual = game.store() == tako_core::library::GameStore::Manual;
        let runner = eff
            .wine_runtime
            .as_deref()
            .map(str::trim)
            .filter(|r| !r.is_empty());
        if (cfg!(target_os = "linux") || cfg!(test))
            && is_manual
            && is_windows_exe
            && let Some(runner) = runner
        {
            let detected = tako_platform::linux_compatibility::detect_runtimes();
            let runtime = tako_platform::linux_compatibility::resolve_runtime(runner, &detected)
                .ok_or_else(|| ctx.tf("session.runtime_not_found", &[("runtime", runner)]))?;

            let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
            let prefix = if let Some(p) =
                eff.wine_prefix.as_deref().filter(|s| !s.trim().is_empty())
            {
                tako_platform::linux_compatibility::expand_and_validate_prefix(p, home.as_deref())
                    .map_err(|_| ctx.t("session.prefix_invalid"))?
            } else {
                let def = ctx.paths.wine_prefix(&game.id);
                tako_platform::linux_compatibility::validate_prefix_path(&def, home.as_deref())
                    .map_err(|_| ctx.t("session.prefix_invalid"))?;
                def
            };
            let _ = std::fs::create_dir_all(&prefix);

            let steam_root = tako_platform::linux_compatibility::find_steam_root(home.as_deref());

            let cfg = tako_platform::linux_compatibility::prepare_launch(
                &runtime,
                std::path::Path::new(&target),
                (!args.is_empty()).then_some(args.as_str()),
                &prefix,
                steam_root.as_deref(),
                &eff.environment,
            );
            let args_str = cfg.arguments_string();
            (
                cfg.executable.to_string_lossy().into_owned(),
                (!args_str.is_empty()).then_some(args_str),
                cfg.environment,
            )
        } else {
            (
                target,
                (!args.is_empty()).then_some(args),
                eff.environment.clone(),
            )
        }
    };
    let launched = p.processes.launch_with_env(
        &target_to_launch,
        args_to_launch.as_deref(),
        cwd.as_deref(),
        &env_to_launch,
    );
    if let Some(e) = before_failed {
        tracing::warn!("comando antes do jogo: {e}");
        ctx.toast(ctx.tf("session.before_failed", &[("error", &e.to_string())]));
    }
    let pid = match launched {
        Ok(pid) => {
            events::publish(SessionEvent::GameStarted {
                game: session_game.clone(),
                at: Utc::now(),
            });
            pid
        }
        Err(e) => {
            // O jogo não abriu: as ferramentas abertas para ele também fecham.
            close_tools(&ctx.platform, tools, &keep_pids);
            exited_now(Some(Duration::ZERO));
            restore_now(&ctx.platform, &ctx.paths, recorded);
            return Err(
                tako_core::presentation::library_text::client_missing(&game, false, None)
                    .unwrap_or(e.to_string()),
            );
        }
    };
    if ctx.desktop
        && matches!(
            game.play_status,
            None | Some(tako_core::library::PlayStatus::Wishlist)
        )
        && let Err(e) = ctx
            .library
            .set_status(&game.id, Some(tako_core::library::PlayStatus::Playing))
    {
        tracing::error!("status ao jogar: {e}");
    }
    let _ = ctx.library.record_launch(&game.id);
    let tp = plan(&game, pid.is_some());
    if tp.strategy == GameTrackingStrategy::None {
        restore_now(&ctx.platform, &ctx.paths, recorded);
        // Sem acompanhamento não se sabe quando termina: o fim sai agora, sem duração (e as ferramentas ficam).
        exited_now(None);
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
        after_command: eff.after_command.clone().filter(|_| eff.commands_approved),
        store: Some(game.store()),
        close_tools: if eff.close_tools {
            tools.to_vec()
        } else {
            Vec::new()
        },
        keep_pids,
    };
    Ok(Started::Tracking(req))
}

/// Pede para fechar as ferramentas do jogo, menos os processos que já rodavam antes dele (`keep`).
fn close_tools(platform: &Platform, tools: &[String], keep: &[u32]) {
    if tools.is_empty() {
        return;
    }
    let mut pids = processes_of(&platform.processes.snapshot(), tools);
    pids.retain(|p| !keep.contains(p));
    if !pids.is_empty()
        && let Err(e) = platform.processes.request_close(&pids)
    {
        tracing::warn!("fechar as ferramentas do jogo: {e}");
    }
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
    // `restore_from` apaga o arquivo (ou deixa só a marca do VRR de uma tela desconectada).
    match store.read() {
        Some(snapshot) => {
            let report = g.restore_from(env, &snapshot, Utc::now());
            if !report.succeeded() {
                tracing::warn!("restauração incompleta: {report:?}");
            }
        }
        None => store.delete(),
    }
}

/// Ao abrir: restaura o que uma sessão anterior deixou (queda do Tako ou do PC no meio do jogo).
pub fn recover_after_crash(platform: &Platform, paths: &DataPaths) {
    platform.cursor.recover_after_crash();
    // Controles escondidos do jogo (INPUT-005) por uma execução que caiu com a Central aberta.
    if let Err(e) = platform.controller_hiding.reveal() {
        tracing::warn!("não foi possível devolver os controles escondidos: {e}");
    }
    let env = Env {
        displays: platform.displays.as_ref(),
        audio: platform.audio.as_ref(),
    };
    let g = guard(paths);
    if let Some(pending) = g.pending_from_previous_run(env, Utc::now()) {
        tracing::info!("restaurando telas e áudio de uma sessão que não terminou");
        let _ = g.restore_from(env, &pending, Utc::now());
    }
}

/// No Linux e no macOS só quem abriu o jogo recebe o código de saída, e quem abriu foi a interface: com ela
/// ainda aberta (o jogo caiu ao abrir), completa o aviso do supervisor com o código. No Windows o supervisor já
/// manda o código; aqui só solta o processo guardado.
pub fn with_local_exit_code(
    ctx: &Ctx,
    pid: Option<u32>,
    end: tako_core::session::GameEnd,
) -> tako_core::session::GameEnd {
    use tako_core::session::GameEnd;
    let local = pid.and_then(|p| ctx.platform.processes.exit_code(p));
    match end {
        GameEnd::Quick(None) => GameEnd::Quick(local),
        other => other,
    }
}

/// Acompanhamento do jogo (no supervisor ou na própria interface sem supervisor).
pub struct Running {
    pub req: TrackRequest,
    tracker: GameRunTracker,
    mouse: tako_core::input::controller_mouse::ControllerMouse,
    screen_height: u32,
    presence: Option<crate::discord::Presence>,
    /// Desde quando o jogo mostra uma janela que não cobre a tela (ver [`Running::window_shown`]).
    windowed_since: Option<std::time::Instant>,
    /// Últimos processos do jogo registrados no log (BUG-001), para logar de novo só quando mudarem.
    logged_paths: Vec<String>,
}

impl Running {
    pub const POLL: Duration = Duration::from_secs(2);
    /// Jogo em janela: quanto esperar antes de sair de "Abrindo…".
    pub const WINDOWED_GRACE: Duration = Duration::from_secs(4);

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
            windowed_since: None,
            logged_paths: Vec::new(),
        })
    }

    /// Cursor e presença no Discord da sessão. Ficam com quem acompanha o jogo: no X11 o cursor escondido (e no
    /// Discord a atividade) some quando a conexão de quem pediu fecha.
    pub fn begin(&mut self, platform: &Platform) {
        // Segura o processo raiz para ler o código de saída no fim (SESSION-003).
        if let Some(pid) = self.req.root_pid {
            platform.processes.watch_exit(pid);
        }
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
        // Uma vez só, mesmo se `finish` for chamado de novo.
        if let Some(c) = self.req.after_command.take()
            && let Err(e) = platform.processes.run_command(&c, None)
        {
            tracing::warn!("comando depois do jogo: {e}");
        }
        let tools = std::mem::take(&mut self.req.close_tools);
        close_tools(platform, &tools, &self.req.keep_pids);
        let _ = self.mouse.reset(platform.pointer.as_ref());
        restore_now(platform, paths, self.req.restore);
    }

    /// Uma leitura dos processos. `Some(tempo jogado)` quando o jogo terminou.
    pub fn poll(&mut self, platform: &Platform) -> Option<Duration> {
        let steam =
            (self.req.steam_app_id.is_some()).then(|| platform.processes.steam_running_app_id());
        let before = self.tracker.status;
        let status = self
            .tracker
            .update(&platform.processes.snapshot(), Utc::now(), steam);
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
        match status {
            GameRunStatus::Exited => Some(self.tracker.play_time().to_std().unwrap_or_default()),
            GameRunStatus::TimedOut => Some(Duration::ZERO),
            _ => None,
        }
    }

    /// Como terminou, depois que [`Self::poll`] devolveu o tempo jogado (SESSION-003), e avisa `GameExited`
    /// (EXT-001). O código de saída é o do processo raiz, quando o Tako abriu o executável direto (jogo aberto pela
    /// loja não tem).
    pub fn end(&self, platform: &Platform, played: Duration) -> tako_core::session::GameEnd {
        let code = self
            .req
            .root_pid
            .and_then(|pid| platform.processes.exit_code(pid));
        let end = tako_core::session::GameEnd::of(
            self.tracker.status,
            self.tracker.close_requested,
            played,
            code,
        );
        events::publish(SessionEvent::GameExited {
            game: SessionGame {
                game_id: self.req.game_id.clone(),
                title: self.req.title.clone(),
                store: self
                    .req
                    .store
                    .unwrap_or(tako_core::library::GameStore::Manual),
            },
            at: Utc::now(),
            played: Some(played),
            exit_code: code,
        });
        end
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

    /// A janela do jogo já está na tela (a interface sai de "Abrindo…"): cobrindo o monitor, ou em janela há
    /// [`Self::WINDOWED_GRACE`] (muitos jogos abrem uma janela pequena antes de ir para tela cheia). Acompanhado só
    /// pela Steam (sem processos), vale a confirmação dela.
    pub fn window_shown(&mut self, platform: &Platform) -> bool {
        use tako_core::devices::GameWindow;
        let pids = self.game_pids();
        if pids.is_empty() {
            return self.req.strategy == GameTrackingStrategy::SteamApp
                && self.tracker.steam_confirmed;
        }
        match platform.processes.game_window(&pids) {
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
    use tako_core::session::GameEnd;

    #[test]
    fn successful_desktop_launch_marks_unset_or_wishlist_status_and_preserves_user_choices() {
        use tako_core::library::PlayStatus;
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = build_ctx(DataPaths::at(dir.path()), Platform::simulated(), true);
        ctx.library.refresh();
        let id = ctx
            .library
            .games()
            .iter()
            .find(|g| g.is_installed)
            .unwrap()
            .id
            .clone();
        ctx.desktop = true;
        for before in [
            None,
            Some(PlayStatus::Wishlist),
            Some(PlayStatus::Paused),
            Some(PlayStatus::Beaten),
            Some(PlayStatus::Abandoned),
        ] {
            ctx.library.set_status(&id, before).unwrap();
            let started = start(&mut ctx, &id).unwrap();
            let expected = match before {
                None | Some(PlayStatus::Wishlist) => Some(PlayStatus::Playing),
                other => other,
            };
            assert_eq!(ctx.library.find(&id).unwrap().play_status, expected);
            if let Started::Tracking(req) = started {
                restore_now(&ctx.platform, &ctx.paths, req.restore);
            }
        }
        ctx.desktop = false;
        ctx.library.set_status(&id, None).unwrap();
        start(&mut ctx, &id).unwrap();
        assert_eq!(ctx.library.find(&id).unwrap().play_status, None);
    }

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
        let played = running.poll(&platform).unwrap();
        // Sem código (o processo simulado não deixou): só o tempo decide.
        assert_eq!(running.end(&platform, played), GameEnd::Quick(None));
        restore_now(&platform, &ctx.paths, req.restore);
        assert!(!platform.cursor.is_hidden());
        assert!(!ctx.paths.session_environment().exists());
    }

    #[test]
    fn allowed_tools_open_with_the_game_close_at_the_end_and_the_session_is_announced() {
        use std::sync::Arc;
        use tako_core::devices::fake::FakeProcesses;
        let dir = tempfile::tempdir().unwrap();
        let fake = Arc::new(FakeProcesses::default());
        let platform = Platform {
            processes: fake.clone(),
            ..Platform::simulated()
        };
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
        let store = tako_core::persistence::GameSettingsStore::new(
            tako_core::persistence::TakoDatabase::new(dir.path().join("tako.db")),
        );
        ctx.game_settings = Some(store.clone());
        let mut o = store.load(&id).unwrap();
        o.tools = Some("/Ferramentas/fps;/Ferramentas/mods".into());
        o.close_tools = Some(true);
        store.save(&id, &o).unwrap();
        let events = events::subscribe();

        // Sem permissão: nenhuma ferramenta abre (a tela pergunta antes; aqui a sessão não confia nela).
        let Started::Tracking(req) = start(&mut ctx, &id).unwrap() else {
            panic!("deveria acompanhar")
        };
        assert!(req.close_tools.is_empty());
        assert!(
            !fake
                .launched
                .lock()
                .unwrap()
                .iter()
                .any(|l| l.starts_with("/Ferramentas"))
        );
        restore_now(&platform, &ctx.paths, req.restore);
        fake.exit(req.root_pid.unwrap());

        // O usuário já tinha o "fps" aberto (gravando, por exemplo): esse não fecha no fim.
        let user_fps = tako_core::devices::ProcessLauncher::launch(
            fake.as_ref(),
            "/Ferramentas/fps",
            None,
            None,
        )
        .unwrap()
        .unwrap();
        o.approve_commands();
        store.save(&id, &o).unwrap();
        let Started::Tracking(req) = start(&mut ctx, &id).unwrap() else {
            panic!("deveria acompanhar")
        };
        let launched = fake.launched.lock().unwrap().clone();
        assert_eq!(
            &launched[launched.len() - 3..launched.len() - 1],
            ["/Ferramentas/fps", "/Ferramentas/mods"]
        );
        let mut running = Running::new(req.clone()).unwrap();
        running.begin(&platform);
        fake.exit_with(req.root_pid.unwrap(), 0);
        running.tracker.exit_grace = chrono::TimeDelta::zero();
        let played = running.poll(&platform).unwrap();
        running.end(&platform, played);
        running.finish(&platform, &ctx.paths);
        let left: Vec<u32> = platform
            .processes
            .snapshot()
            .iter()
            .map(|p| p.pid)
            .collect();
        assert_eq!(
            left,
            [user_fps],
            "as ferramentas abertas pelo Tako fecham; a do usuário fica"
        );

        let mine: Vec<_> = events
            .try_iter()
            .filter_map(|e| match e {
                SessionEvent::GameStarting { game, .. } if game.game_id == id => {
                    Some(("starting", None))
                }
                SessionEvent::GameStarted { game, .. } if game.game_id == id => {
                    Some(("started", None))
                }
                SessionEvent::GameExited {
                    game, exit_code, ..
                } if game.game_id == id => Some(("exited", exit_code)),
                _ => None,
            })
            .collect();
        // Outros testes (e a primeira sessão deste) abrem o mesmo jogo: confere só a última sessão.
        assert!(
            mine.ends_with(&[("starting", None), ("started", None), ("exited", Some(0))]),
            "{mine:?}"
        );
    }

    #[test]
    fn vrr_turned_on_for_the_game_is_turned_off_after_it() {
        use tako_core::devices::VrrStatus;
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
        // A TV do simulado tem VRR desligado.
        ctx.settings.set_bool(keys::GAME_SCREEN_ENABLED, true);
        ctx.settings.set(keys::GAME_SCREEN_ID, "DISPLAY2");
        ctx.settings.set_bool(keys::GAME_SCREEN_VRR, true);
        let vrr = || platform.displays.list().unwrap()[1].vrr;
        let Started::Tracking(req) = start(&mut ctx, &id).unwrap() else {
            panic!("deveria acompanhar")
        };
        assert_eq!(vrr(), VrrStatus::On);
        restore_now(&platform, &ctx.paths, req.restore);
        assert_eq!(vrr(), VrrStatus::Off);
    }

    #[test]
    fn the_end_carries_the_exit_code_of_the_game_tako_opened() {
        use std::sync::Arc;
        use tako_core::devices::fake::FakeProcesses;
        let dir = tempfile::tempdir().unwrap();
        let fake = Arc::new(FakeProcesses::default());
        let platform = Platform {
            processes: fake.clone(),
            ..Platform::simulated()
        };
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
        for (played, code, expected) in [
            (3, 0xC000_0135, GameEnd::Quick(Some(0xC000_0135))),
            (600, 0xC000_0005, GameEnd::Failed(0xC000_0005)),
            (600, 0, GameEnd::Normal),
        ] {
            let Started::Tracking(req) = start(&mut ctx, &id).unwrap() else {
                panic!("deveria acompanhar")
            };
            let mut running = Running::new(req.clone()).unwrap();
            running.begin(&platform);
            fake.exit_with(req.root_pid.unwrap(), code);
            running.tracker.exit_grace = chrono::TimeDelta::zero();
            assert!(running.poll(&platform).is_some());
            let end = running.end(&platform, Duration::from_secs(played));
            assert_eq!(end, expected, "{played} s, {code:#X}");
            restore_now(&platform, &ctx.paths, req.restore);
        }
    }

    #[test]
    fn manual_windows_game_launches_with_proton_or_wine() {
        use std::sync::Arc;
        use tako_core::devices::fake::FakeProcesses;
        use tako_core::library::ManualGameRecord;
        let dir = tempfile::tempdir().unwrap();
        let fake = Arc::new(FakeProcesses::default());
        let platform = Platform {
            processes: fake.clone(),
            ..Platform::simulated()
        };
        let mut ctx = build_ctx(DataPaths::at(dir.path()), platform.clone(), true);
        let manual = ctx
            .library
            .add_manual(ManualGameRecord::create(
                "Windows Game",
                "/jogos/game.exe",
                Utc::now(),
            ))
            .unwrap()
            .clone();

        let store = tako_core::persistence::GameSettingsStore::new(
            tako_core::persistence::TakoDatabase::new(dir.path().join("tako.db")),
        );
        ctx.game_settings = Some(store.clone());

        // 1. Sem escolha de runtime (Sistema): abre direto sem Wine/Proton ("nunca rode nada sem o usuário escolher")
        let Started::Tracking(req) = start(&mut ctx, &manual.id).unwrap() else {
            panic!("deveria acompanhar")
        };
        assert_eq!(
            fake.launched.lock().unwrap().last().unwrap(),
            "/jogos/game.exe"
        );
        restore_now(&platform, &ctx.paths, req.restore);

        // 2. Com Wine do sistema e prefixo padrão
        let mut o = store.load(&manual.id).unwrap();
        o.wine_runtime = Some("system-wine".into());
        store.save(&manual.id, &o).unwrap();

        let Started::Tracking(req) = start(&mut ctx, &manual.id).unwrap() else {
            panic!("deveria acompanhar")
        };
        let last = fake.launched.lock().unwrap().last().unwrap().clone();
        assert!(
            last.starts_with("wine /jogos/game.exe")
                || last.starts_with("/usr/bin/wine /jogos/game.exe")
        );
        restore_now(&platform, &ctx.paths, req.restore);

        // 3. Com prefixo proibido (/ ou $HOME): recusa a abertura
        o.wine_prefix = Some("/".into());
        store.save(&manual.id, &o).unwrap();
        assert!(start(&mut ctx, &manual.id).is_err());
    }
}
