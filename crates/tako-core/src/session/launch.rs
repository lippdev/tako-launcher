//! Abrir o jogo e devolver o ambiente (MIG-024, ADR-0026), sem interface: o que a `tako-ui` (Slint) e a `tako-app`
//! (versão web) fazem igual. Resolve as configurações, grava o estado de telas e áudio em disco, aplica a tela de
//! jogo e a saída de áudio e abre o jogo. Daí em diante o acompanhamento ([`super::running::GameSession`]) restaura
//! tudo a partir do disco no fim, e [`recover_after_crash`] restaura o que uma queda deixou.
//!
//! O que depende de quem chama fica fora: achar o emulador de uma ROM (`tako-sources`), preparar o Wine ou o Proton
//! (`tako-platform`, só Linux), mostrar os avisos e guardar as configurações. Os textos já saem no idioma atual.

use super::audio::AudioOutputChoice;
use super::events::{self, SessionEvent, SessionGame};
use super::restore::{Env, PendingEnvironmentStore, SessionEnvironmentGuard};
use super::screen::{GameScreenOutcome, GameScreenSession};
use super::settings::{
    EffectiveSessionSettings, GameSettingsOverride, GlobalSessionSettings, MonitorPresets, resolve,
};
use super::tracking::{plan, processes_of};
use crate::devices::{
    AudioOutputs, ControllerHiding, CursorControl, DisplayMode, Displays, PointerInjector,
    ProcessLauncher,
};
use crate::i18n;
use crate::library::{Game, GameLibrary, GameStore, GameTrackingStrategy, PlayStatus};
use crate::paths::DataPaths;
use crate::persistence::{GameSettingsStore, parse_bool};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configurações que a sessão lê (as mesmas chaves do app Slint, `tako_ui::shell::ctx::keys`).
pub mod keys {
    pub const RESTORE_ENVIRONMENT: &str = "RestoreEnvironmentAfterGame";
    pub const GAME_SCREEN_ENABLED: &str = "GameScreenEnabled";
    pub const GAME_SCREEN_ID: &str = "GameScreenId";
    pub const GAME_SCREEN_MODE: &str = "GameScreenMode";
    pub const GAME_SCREEN_HDR: &str = "GameScreenHdr";
    pub const GAME_SCREEN_VRR: &str = "GameScreenVrr";
    pub const GAME_SCREEN_PRESETS: &str = "GameScreenPresets";
    pub const GAME_SCREEN_CONFIRMED: &str = "GameScreenConfirmedSetup";
    pub const GAME_AUDIO_ID: &str = "GameAudioDeviceId";
    pub const GAME_AUDIO_NAME: &str = "GameAudioDeviceName";
    pub const HIDE_CURSOR: &str = "HideCursorDuringGame";
    pub const CONTROLLER_MOUSE: &str = "ControllerMouseDuringGame";
    pub const DISCORD_PRESENCE: &str = "DiscordRichPresence";
}

/// As funções de plataforma que a sessão usa. Quem chama monta a partir da `tako_platform::Platform` (ou das
/// versões em memória de [`crate::devices::fake`], na demonstração e nos testes).
#[derive(Clone, Copy)]
pub struct SessionDevices<'a> {
    pub displays: &'a dyn Displays,
    pub audio: &'a dyn AudioOutputs,
    pub processes: &'a dyn ProcessLauncher,
    pub cursor: &'a dyn CursorControl,
    pub pointer: &'a dyn PointerInjector,
    pub controller_hiding: &'a dyn ControllerHiding,
}

impl SessionDevices<'_> {
    fn env(&self) -> Env<'_> {
        Env {
            displays: self.displays,
            audio: self.audio,
        }
    }
}

/// O que o acompanhamento precisa para seguir o jogo (no supervisor, na própria interface ou no núcleo web).
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
    pub store: Option<GameStore>,
    /// Ferramentas permitidas para fechar quando o jogo terminar (EXT-001).
    #[serde(default)]
    pub close_tools: Vec<String>,
    /// Processos dessas ferramentas que já rodavam antes do jogo: o usuário abriu, não são fechados.
    #[serde(default)]
    pub keep_pids: Vec<u32>,
}

/// Resultado de abrir um jogo.
#[derive(Debug, Clone, PartialEq)]
pub enum Started {
    /// O jogo abriu e há como saber quando termina.
    Tracking(TrackRequest),
    /// Abriu, mas não dá para acompanhar (URI sem processo nem pasta): só abre, como o C#.
    Untracked,
}

/// Tela e áudio do jogo nas configurações gerais. `get` lê uma configuração pela chave ([`keys`]).
pub fn global_settings(get: impl Fn(&str) -> Option<String>) -> GlobalSessionSettings {
    let flag = |key| get(key).as_deref().and_then(parse_bool).unwrap_or(false);
    let text = |key| get(key).unwrap_or_default();
    let audio_id = text(keys::GAME_AUDIO_ID);
    GlobalSessionSettings {
        game_screen_enabled: flag(keys::GAME_SCREEN_ENABLED),
        game_screen_id: text(keys::GAME_SCREEN_ID),
        game_screen_mode: get(keys::GAME_SCREEN_MODE)
            .as_deref()
            .and_then(DisplayMode::parse),
        game_screen_hdr: flag(keys::GAME_SCREEN_HDR),
        game_screen_vrr: flag(keys::GAME_SCREEN_VRR),
        monitor_presets: MonitorPresets::parse(&text(keys::GAME_SCREEN_PRESETS)),
        game_audio: (!audio_id.trim().is_empty()).then(|| {
            let name = text(keys::GAME_AUDIO_NAME);
            AudioOutputChoice::new(&audio_id, if name.is_empty() { &audio_id } else { &name })
        }),
    }
}

/// As configurações gerais com as do jogo por cima (sem banco, na demonstração, só as gerais).
pub fn effective(
    global: &GlobalSessionSettings,
    store: Option<&GameSettingsStore>,
    game_id: &str,
) -> EffectiveSessionSettings {
    let game: Option<GameSettingsOverride> = store.and_then(|s| s.load(game_id).ok());
    resolve(global, game.as_ref())
}

/// Linha já pronta para abrir o jogo por outro programa (Wine ou Proton, no Linux), preparada e validada por quem
/// chama antes de qualquer efeito.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedCommand {
    pub executable: std::path::PathBuf,
    pub arguments: Vec<String>,
    pub environment: Vec<(String, String)>,
}

/// O pedido de abertura. `game` já vem com o comando do emulador, quando é uma ROM.
#[derive(Debug, Clone)]
pub struct LaunchRequest {
    pub game: Game,
    pub settings: EffectiveSessionSettings,
    pub prepared: Option<PreparedCommand>,
    /// "Restaurar tela e áudio depois do jogo" ([`keys::RESTORE_ENVIRONMENT`]).
    pub restore_environment: bool,
    pub hide_cursor: bool,
    pub controller_mouse: bool,
    pub discord: bool,
    /// Marca "Jogando" quando o jogo não tem situação (ou está na lista de desejos); o app Slint faz isso só no modo
    /// desktop.
    pub mark_playing: bool,
    /// O valor atual de [`keys::GAME_SCREEN_CONFIRMED`].
    pub screen_confirmations: Option<String>,
}

impl LaunchRequest {
    /// Pedido com as preferências das configurações gerais (`get` como em [`global_settings`]).
    pub fn new(
        game: Game,
        settings: EffectiveSessionSettings,
        get: impl Fn(&str) -> Option<String>,
    ) -> LaunchRequest {
        let flag = |key, default| get(key).as_deref().and_then(parse_bool).unwrap_or(default);
        LaunchRequest {
            game,
            settings,
            prepared: None,
            restore_environment: flag(keys::RESTORE_ENVIRONMENT, true),
            hide_cursor: flag(keys::HIDE_CURSOR, false),
            controller_mouse: flag(keys::CONTROLLER_MOUSE, false),
            discord: flag(keys::DISCORD_PRESENCE, false),
            mark_playing: false,
            screen_confirmations: get(keys::GAME_SCREEN_CONFIRMED),
        }
    }
}

/// O que a abertura deixa para quem chamou mostrar e guardar, também quando ela falha.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct LaunchReport {
    /// Avisos para o usuário, na ordem em que aconteceram.
    pub notices: Vec<String>,
    /// Novo valor de [`keys::GAME_SCREEN_CONFIRMED`], quando a tela de jogo foi vista pela primeira vez.
    pub screen_confirmations: Option<String>,
}

fn guard(paths: &DataPaths) -> SessionEnvironmentGuard {
    SessionEnvironmentGuard::new(PendingEnvironmentStore::new(paths.session_environment()))
}

/// Abre o jogo com tela e áudio da sessão. Em erro, desfaz o que já mudou. Registra a abertura na biblioteca.
pub fn launch(
    devices: &SessionDevices,
    paths: &DataPaths,
    library: &mut GameLibrary,
    request: LaunchRequest,
    report: &mut LaunchReport,
) -> Result<Started, String> {
    let LaunchRequest {
        game,
        settings: eff,
        prepared,
        ..
    } = &request;
    let target = game
        .launch_target
        .clone()
        .ok_or_else(|| i18n::t("game.missing_file"))?;
    let session_game = SessionGame {
        game_id: game.id.clone(),
        title: game.title.clone(),
        store: game.store(),
    };
    let raw_cwd = eff
        .working_directory
        .clone()
        .or(game.working_directory.clone())
        .filter(|d| !d.trim().is_empty());
    let cwd = if prepared.is_some() {
        raw_cwd.or_else(|| {
            std::path::Path::new(&target)
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
        })
    } else {
        raw_cwd
    };
    if let Some(p) = eff.game_screen_problem {
        report.notices.push(p.message());
    }
    let mut g = guard(paths);
    let recorded = g
        .begin_session(
            devices.env(),
            &game.id,
            request.restore_environment,
            Utc::now(),
        )
        .is_some();

    if let Some(screen) = &eff.game_screen {
        let confirmed = request.screen_confirmations.as_deref().and_then(|c| {
            super::settings::game_screen_confirmations::matching(Some(c), &screen.setup_key())
        });
        let mut session = GameScreenSession::default();
        // `session` acaba aqui: quem desliga o VRR no fim é o guard, pelo arquivo gravado antes de ligar.
        let result =
            session.begin_with(devices.displays, screen, confirmed.as_deref(), &mut |id| {
                g.remember_vrr_turned_on(id)
            });
        if result.outcome == GameScreenOutcome::Reverted {
            g.discard();
            return Err(result
                .reason
                .unwrap_or_else(|| i18n::t("session.ui.screen_failed")));
        }
        if result.needs_confirmation && result.outcome == GameScreenOutcome::Applied {
            // Primeira vez com esta tela/modo: lembra que o usuário viu a imagem (a abertura do jogo confirma).
            report.screen_confirmations =
                Some(super::settings::game_screen_confirmations::remember(
                    request.screen_confirmations.as_deref(),
                    &screen.setup_key(),
                ));
        }
    }
    if let Some(choice) = &eff.game_audio
        && let Err(e) = super::audio::apply(devices.audio, choice)
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
    // Ferramentas só são abertas, sem esperar: não seguram quem chama nem o heartbeat do supervisor.
    let tools: &[String] = if eff.commands_approved {
        &eff.tools
    } else {
        &[]
    };
    let keep_pids = if tools.is_empty() {
        Vec::new()
    } else {
        processes_of(&devices.processes.snapshot(), tools)
    };
    for tool in tools {
        if let Err(e) = devices.processes.launch(tool, None, None) {
            tracing::warn!("ferramenta do jogo {tool}: {e}");
            report.notices.push(i18n::tf(
                "session.tool_failed",
                &[("tool", tool), ("error", &e.to_string())],
            ));
        }
    }
    let before_failed = eff
        .before_command
        .as_deref()
        .filter(|_| eff.commands_approved)
        .and_then(|c| {
            devices
                .processes
                .run_command(c, Some(super::settings::BEFORE_COMMAND_WAIT))
                .err()
        });
    let env: &[(String, String)] = if eff.commands_approved {
        &eff.environment
    } else {
        &[]
    };
    let launched = if let Some(cfg) = prepared {
        devices.processes.launch_args_with_env(
            &cfg.executable.to_string_lossy(),
            &cfg.arguments,
            cwd.as_deref(),
            &cfg.environment,
        )
    } else {
        let args = if eff.commands_approved {
            eff.arguments(game.launch_arguments.as_deref())
        } else {
            game.launch_arguments.clone().unwrap_or_default()
        };
        devices.processes.launch_with_env(
            &target,
            (!args.is_empty()).then_some(&args),
            cwd.as_deref(),
            env,
        )
    };
    if let Some(e) = before_failed {
        tracing::warn!("comando antes do jogo: {e}");
        report.notices.push(i18n::tf(
            "session.before_failed",
            &[("error", &e.to_string())],
        ));
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
            close_tools(devices.processes, tools, &keep_pids);
            exited_now(Some(Duration::ZERO));
            restore_now(devices, paths, recorded);
            return Err(
                crate::presentation::library_text::client_missing(game, false, None)
                    .unwrap_or(e.to_string()),
            );
        }
    };
    if request.mark_playing
        && matches!(game.play_status, None | Some(PlayStatus::Wishlist))
        && let Err(e) = library.set_status(&game.id, Some(PlayStatus::Playing))
    {
        tracing::error!("status ao jogar: {e}");
    }
    let _ = library.record_launch(&game.id);
    let tp = plan(game, pid.is_some());
    if tp.strategy == GameTrackingStrategy::None {
        restore_now(devices, paths, recorded);
        // Sem acompanhamento não se sabe quando termina: o fim sai agora, sem duração (e as ferramentas ficam).
        exited_now(None);
        return Ok(Started::Untracked);
    }
    Ok(Started::Tracking(TrackRequest {
        game_id: game.id.clone(),
        title: game.title.clone(),
        root_pid: pid,
        strategy: tp.strategy,
        folder: tp.folder,
        steam_app_id: tp.steam_app_id,
        launched_at: Utc::now(),
        hide_cursor: request.hide_cursor,
        controller_mouse: request.controller_mouse,
        restore: recorded,
        discord: request.discord,
        after_command: eff.after_command.clone().filter(|_| eff.commands_approved),
        store: Some(game.store()),
        close_tools: if eff.close_tools {
            tools.to_vec()
        } else {
            Vec::new()
        },
        keep_pids,
    }))
}

/// Pede para fechar as ferramentas do jogo, menos os processos que já rodavam antes dele (`keep`).
pub(crate) fn close_tools(processes: &dyn ProcessLauncher, tools: &[String], keep: &[u32]) {
    if tools.is_empty() {
        return;
    }
    let mut pids = processes_of(&processes.snapshot(), tools);
    pids.retain(|p| !keep.contains(p));
    if !pids.is_empty()
        && let Err(e) = processes.request_close(&pids)
    {
        tracing::warn!("fechar as ferramentas do jogo: {e}");
    }
}

/// Volta telas e áudio ao que estava antes do jogo (lido do disco) e mostra o cursor. Idempotente.
pub fn restore_now(devices: &SessionDevices, paths: &DataPaths, recorded: bool) {
    if devices.cursor.is_hidden() {
        let _ = devices.cursor.set_hidden(false);
    }
    let store = PendingEnvironmentStore::new(paths.session_environment());
    if !recorded {
        store.delete();
        return;
    }
    let g = guard(paths);
    // `restore_from` apaga o arquivo (ou deixa só a marca do VRR de uma tela desconectada).
    match store.read() {
        Some(snapshot) => {
            let report = g.restore_from(devices.env(), &snapshot, Utc::now());
            if !report.succeeded() {
                tracing::warn!("restauração incompleta: {report:?}");
            }
        }
        None => store.delete(),
    }
}

/// Ao abrir: restaura o que uma sessão anterior deixou (queda do Tako ou do PC no meio do jogo). `true` = havia
/// telas ou áudio a restaurar.
pub fn recover_after_crash(devices: &SessionDevices, paths: &DataPaths) -> bool {
    devices.cursor.recover_after_crash();
    // Controles escondidos do jogo (INPUT-005) por uma execução que caiu com a Central aberta.
    if let Err(e) = devices.controller_hiding.reveal() {
        tracing::warn!("não foi possível devolver os controles escondidos: {e}");
    }
    let g = guard(paths);
    let Some(pending) = g.pending_from_previous_run(devices.env(), Utc::now()) else {
        return false;
    };
    tracing::info!("restaurando telas e áudio de uma sessão que não terminou");
    let _ = g.restore_from(devices.env(), &pending, Utc::now());
    true
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::devices::controller_hiding::NoHiding;
    use crate::devices::fake::{FakeAudio, FakeDisplays, FakePointer, FakeProcesses};
    use crate::library::{MemoryLibraryStore, system_clock};
    use std::sync::Arc;

    /// Plataforma em memória para os testes da sessão.
    #[derive(Clone)]
    pub(crate) struct Fakes {
        pub displays: Arc<FakeDisplays>,
        pub audio: Arc<FakeAudio>,
        pub processes: Arc<FakeProcesses>,
        pub pointer: Arc<FakePointer>,
        pub hiding: Arc<NoHiding>,
    }

    impl Fakes {
        pub fn new() -> Fakes {
            Fakes {
                displays: Arc::new(FakeDisplays::desk()),
                audio: Arc::new(FakeAudio::desk()),
                processes: Arc::new(FakeProcesses::default()),
                pointer: Arc::new(FakePointer::default()),
                hiding: Arc::new(NoHiding),
            }
        }

        pub fn devices(&self) -> SessionDevices<'_> {
            SessionDevices {
                displays: self.displays.as_ref(),
                audio: self.audio.as_ref(),
                processes: self.processes.as_ref(),
                cursor: self.pointer.as_ref(),
                pointer: self.pointer.as_ref(),
                controller_hiding: self.hiding.as_ref(),
            }
        }
    }

    pub(crate) fn library_with(game: Game) -> GameLibrary {
        let mut library = GameLibrary::new(
            Box::new(move || vec![game.clone()]),
            Box::new(MemoryLibraryStore::default()),
            system_clock(),
        );
        library.load();
        library
    }

    pub(crate) fn game(target: &str) -> Game {
        Game {
            id: "manual:teste".into(),
            title: "Teste".into(),
            store: Some(GameStore::Manual),
            is_installed: true,
            launch_target: Some(target.into()),
            ..Default::default()
        }
    }

    pub(crate) fn request(game: Game, get: impl Fn(&str) -> Option<String>) -> LaunchRequest {
        let eff = effective(&global_settings(&get), None, &game.id);
        LaunchRequest::new(game, eff, get)
    }

    fn settings(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect();
        move |key: &str| pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    }

    #[test]
    fn global_settings_and_preferences_follow_the_slint_defaults() {
        let none = settings(&[]);
        let global = global_settings(&none);
        assert!(!global.game_screen_enabled && global.game_audio.is_none());
        let r = request(game("/jogos/a.exe"), &none);
        assert!(r.restore_environment && !r.hide_cursor && !r.controller_mouse && !r.discord);
        let set = settings(&[
            (keys::GAME_SCREEN_ENABLED, "True"),
            (keys::GAME_SCREEN_ID, "DISPLAY2"),
            (keys::GAME_AUDIO_ID, "tv"),
            (keys::RESTORE_ENVIRONMENT, "false"),
            (keys::HIDE_CURSOR, "true"),
            (keys::GAME_SCREEN_CONFIRMED, "x"),
        ]);
        let global = global_settings(&set);
        assert!(global.game_screen_enabled);
        assert_eq!(global.game_screen_id, "DISPLAY2");
        // Sem nome guardado, o nome é o id.
        assert_eq!(global.game_audio.unwrap().name, "tv");
        let r = request(game("/jogos/a.exe"), &set);
        assert!(!r.restore_environment && r.hide_cursor);
        assert_eq!(r.screen_confirmations.as_deref(), Some("x"));
    }

    #[test]
    fn launch_tracks_records_and_restore_brings_the_screen_back() {
        use crate::devices::VrrStatus;
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::at(dir.path());
        let fakes = Fakes::new();
        let d = fakes.devices();
        let mut library = library_with(game("/jogos/a.exe"));
        // A TV do simulado tem VRR desligado.
        let get = settings(&[
            (keys::GAME_SCREEN_ENABLED, "true"),
            (keys::GAME_SCREEN_ID, "DISPLAY2"),
            (keys::GAME_SCREEN_VRR, "true"),
        ]);
        let vrr = || fakes.displays.list().unwrap()[1].vrr;
        let mut report = LaunchReport::default();
        let mut r = request(game("/jogos/a.exe"), &get);
        r.mark_playing = true;
        let Started::Tracking(req) = launch(&d, &paths, &mut library, r, &mut report).unwrap()
        else {
            panic!("deveria acompanhar")
        };
        assert_eq!(vrr(), VrrStatus::On);
        assert!(req.restore && paths.session_environment().exists());
        assert_eq!(req.strategy, GameTrackingStrategy::Process);
        let saved = library.find("manual:teste").unwrap();
        assert_eq!(saved.play_status, Some(PlayStatus::Playing));
        assert!(saved.last_played_at.is_some());
        restore_now(&d, &paths, req.restore);
        assert_eq!(vrr(), VrrStatus::Off);
        assert!(!paths.session_environment().exists());
        // Restaurar de novo não desfaz nada a mais.
        restore_now(&d, &paths, req.restore);
        assert_eq!(vrr(), VrrStatus::Off);
    }

    #[test]
    fn a_crash_in_the_middle_of_the_game_is_restored_on_the_next_run() {
        use crate::devices::VrrStatus;
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::at(dir.path());
        let fakes = Fakes::new();
        let d = fakes.devices();
        let mut library = library_with(game("/jogos/a.exe"));
        let get = settings(&[
            (keys::GAME_SCREEN_ENABLED, "true"),
            (keys::GAME_SCREEN_ID, "DISPLAY2"),
            (keys::GAME_SCREEN_VRR, "true"),
        ]);
        let r = request(game("/jogos/a.exe"), &get);
        launch(&d, &paths, &mut library, r, &mut LaunchReport::default()).unwrap();
        assert_eq!(fakes.displays.list().unwrap()[1].vrr, VrrStatus::On);
        // O processo caiu aqui: nada de `restore_now`. A próxima abertura acha o diário e restaura.
        assert!(recover_after_crash(&d, &paths));
        assert_eq!(fakes.displays.list().unwrap()[1].vrr, VrrStatus::Off);
        assert!(!recover_after_crash(&d, &paths), "uma vez só");
    }

    #[test]
    fn a_game_that_does_not_open_undoes_everything_and_reports() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::at(dir.path());
        let fakes = Fakes::new();
        let d = fakes.devices();
        let mut no_target = game("/x");
        no_target.launch_target = None;
        let mut library = library_with(no_target.clone());
        let r = request(no_target, settings(&[]));
        let error = launch(&d, &paths, &mut library, r, &mut LaunchReport::default());
        assert_eq!(error.unwrap_err(), i18n::t("game.missing_file"));
        assert!(fakes.processes.launched.lock().unwrap().is_empty());
        assert!(!paths.session_environment().exists());
    }
}
