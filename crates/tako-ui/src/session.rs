//! Sessão de jogo (MIG-024, ADR-0026) na interface Slint. A regra (abrir com tela e áudio, acompanhar, restaurar e
//! recuperar depois de uma queda) fica em `tako_core::session::{launch, running}`, a mesma da versão web; aqui fica
//! só o que depende da interface: achar o emulador da ROM, preparar o Wine ou o Proton, mostrar os avisos, guardar
//! as configurações, o Discord e o ponteiro pelo controle.

use crate::shell::Ctx;
use crate::shell::ctx::keys;
use std::time::Duration;
use tako_core::paths::DataPaths;
use tako_core::session::launch::{self, LaunchReport, LaunchRequest, PreparedCommand};
use tako_core::session::settings::{EffectiveSessionSettings, GlobalSessionSettings};
use tako_core::session::{GameSession, SessionDevices};
use tako_platform::Platform;

pub use tako_core::session::launch::{Started, TrackRequest};

/// As funções da plataforma que a sessão usa.
pub fn devices(platform: &Platform) -> SessionDevices<'_> {
    SessionDevices {
        displays: platform.displays.as_ref(),
        audio: platform.audio.as_ref(),
        processes: platform.processes.as_ref(),
        cursor: platform.cursor.as_ref(),
        pointer: platform.pointer.as_ref(),
        controller_hiding: platform.controller_hiding.as_ref(),
    }
}

pub fn global_settings(ctx: &Ctx) -> GlobalSessionSettings {
    launch::global_settings(|k| ctx.settings.get(k).map(str::to_owned))
}

pub fn effective(ctx: &Ctx, game_id: &str) -> EffectiveSessionSettings {
    launch::effective(&global_settings(ctx), ctx.game_settings.as_ref(), game_id)
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

/// Wine ou Proton escolhido para um `.exe` manual (Linux), validado antes de qualquer efeito (tela, áudio, guard,
/// eventos, ferramentas, comando antes). `None` = abre direto.
fn prepare_wine(
    ctx: &mut Ctx,
    game: &tako_core::library::Game,
    target: &str,
    eff: &EffectiveSessionSettings,
) -> Result<Option<PreparedCommand>, String> {
    let is_windows_exe = target.to_lowercase().ends_with(".exe");
    let is_manual = game.store() == tako_core::library::GameStore::Manual;
    let runner = if eff.commands_approved {
        eff.wine_runtime
            .as_deref()
            .map(str::trim)
            .filter(|r| !r.is_empty())
    } else {
        None
    };
    let Some(runner) = runner.filter(|_| {
        (cfg!(target_os = "linux") || cfg!(test))
            && !tako_platform::sunshine::is_flatpak()
            && is_manual
            && is_windows_exe
    }) else {
        return Ok(None);
    };
    let detected = if runner == "system-wine" {
        Vec::new()
    } else {
        tako_platform::linux_compatibility::detect_runtimes()
    };
    let runtime = tako_platform::linux_compatibility::resolve_runtime(runner, &detected)
        .ok_or_else(|| {
            let msg = ctx.tf("session.runtime_not_found", &[("runtime", runner)]);
            ctx.toast(&msg);
            msg
        })?;

    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    let prefix =
        if let Some(p) = eff.wine_prefix.as_deref().filter(|s| !s.trim().is_empty()) {
            tako_platform::linux_compatibility::expand_and_validate_prefix(p, home.as_deref())
                .map_err(|_| {
                    let msg = ctx.t("session.prefix_invalid");
                    ctx.toast(&msg);
                    msg
                })?
        } else {
            let def = ctx.paths.wine_prefix(&game.id);
            tako_platform::linux_compatibility::validate_prefix_path(&def, home.as_deref())
                .map_err(|_| {
                    let msg = ctx.t("session.prefix_invalid");
                    ctx.toast(&msg);
                    msg
                })?;
            def
        };
    if let Err(e) = std::fs::create_dir_all(&prefix) {
        let msg = ctx.tf("session.prefix_create_failed", &[("error", &e.to_string())]);
        ctx.toast(&msg);
        return Err(msg);
    }

    let steam_root = tako_platform::linux_compatibility::find_steam_root(home.as_deref());
    let args = eff.arguments(game.launch_arguments.as_deref());
    let cfg = tako_platform::linux_compatibility::prepare_launch(
        &runtime,
        std::path::Path::new(target),
        (!args.is_empty()).then_some(args.as_str()),
        &prefix,
        steam_root.as_deref(),
        &eff.environment,
    );
    Ok(Some(PreparedCommand {
        executable: cfg.executable,
        arguments: cfg.arguments,
        environment: cfg.environment,
    }))
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
    let prepared = prepare_wine(ctx, &game, &target, &eff)?;
    let mut request = LaunchRequest::new(game, eff, |k| ctx.settings.get(k).map(str::to_owned));
    request.prepared = prepared;
    request.discord = crate::discord::available() && request.discord;
    request.mark_playing = ctx.desktop;
    let mut report = LaunchReport::default();
    let result = launch::launch(
        &devices(&ctx.platform),
        &ctx.paths,
        &mut ctx.library,
        request,
        &mut report,
    );
    if let Some(saved) = &report.screen_confirmations {
        ctx.settings.set(keys::GAME_SCREEN_CONFIRMED, saved);
    }
    for notice in report.notices {
        ctx.toast(notice);
    }
    result
}

/// Volta telas e áudio ao que estava antes do jogo (lido do disco) e mostra o cursor. Idempotente.
pub fn restore_now(platform: &Platform, paths: &DataPaths, recorded: bool) {
    launch::restore_now(&devices(platform), paths, recorded);
}

/// Ao abrir: restaura o que uma sessão anterior deixou (queda do Tako ou do PC no meio do jogo).
pub fn recover_after_crash(platform: &Platform, paths: &DataPaths) {
    launch::recover_after_crash(&devices(platform), paths);
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

/// Acompanhamento do jogo (no supervisor ou na própria interface sem supervisor): a [`GameSession`] do núcleo com
/// a presença no Discord e o ponteiro pelo controle. Os campos da sessão (`req`, `tracker`) valem direto aqui.
pub struct Running {
    session: GameSession,
    presence: Option<crate::discord::Presence>,
}

impl std::ops::Deref for Running {
    type Target = GameSession;
    fn deref(&self) -> &GameSession {
        &self.session
    }
}

impl std::ops::DerefMut for Running {
    fn deref_mut(&mut self) -> &mut GameSession {
        &mut self.session
    }
}

impl Running {
    pub const POLL: Duration = GameSession::POLL;
    /// Jogo em janela: quanto esperar antes de sair de "Abrindo…".
    pub const WINDOWED_GRACE: Duration = GameSession::WINDOWED_GRACE;

    pub fn new(req: TrackRequest) -> Result<Running, String> {
        Ok(Running {
            session: GameSession::new(req)?,
            presence: None,
        })
    }

    /// Cursor e presença no Discord da sessão. Ficam com quem acompanha o jogo: no X11 o cursor escondido (e no
    /// Discord a atividade) some quando a conexão de quem pediu fecha.
    pub fn begin(&mut self, platform: &Platform) {
        self.session.begin(&devices(platform));
        if self.session.req.discord {
            self.presence = crate::discord::Presence::start(
                &self.session.req.title,
                self.session.req.launched_at,
            );
        }
    }

    pub fn set_pointer(&mut self, platform: &Platform, hide: bool, mouse: bool) {
        self.session.set_pointer(&devices(platform), hide, mouse);
    }

    /// Uma amostra do controle para o ponteiro (só com "Mover com o analógico" ligado).
    pub fn pointer_step(
        &mut self,
        platform: &Platform,
        pad: &crate::gamepad::PadState,
        now: Duration,
        dt: f64,
    ) {
        let a = pad.buttons & crate::gamepad::buttons::A != 0;
        self.session.pointer_step(
            &devices(platform),
            (pad.left_x as f64, -pad.left_y as f64),
            (pad.right_x as f64, -pad.right_y as f64),
            a,
            now,
            dt,
        );
    }

    /// Fim da sessão: solta o ponteiro e restaura telas, áudio e cursor.
    pub fn finish(&mut self, platform: &Platform, paths: &DataPaths) {
        self.presence = None;
        self.session.finish(&devices(platform), paths);
    }

    /// Uma leitura dos processos. `Some(tempo jogado)` quando o jogo terminou.
    pub fn poll(&mut self, platform: &Platform) -> Option<Duration> {
        self.session.poll(&devices(platform))
    }

    /// Como terminou, depois que [`Self::poll`] devolveu o tempo jogado (SESSION-003).
    pub fn end(&self, platform: &Platform, played: Duration) -> tako_core::session::GameEnd {
        self.session.end(&devices(platform), played)
    }

    /// O jogo não fechou com o pedido: encerra os processos dele à força.
    pub fn force_close(&mut self, platform: &Platform) -> Result<(), String> {
        self.session.force_close(&devices(platform))
    }

    /// A janela do jogo já está na tela (ver [`GameSession::window_shown`]).
    pub fn window_shown(&mut self, platform: &Platform) -> bool {
        self.session.window_shown(&devices(platform))
    }

    /// Pede para o jogo fechar (Central › Encerrar jogo).
    pub fn request_close(&mut self, platform: &Platform) -> Result<(), String> {
        self.session.request_close(&devices(platform))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::build_ctx;
    use chrono::Utc;
    use tako_core::session::GameEnd;
    use tako_core::session::events::{self, SessionEvent};

    #[test]
    fn the_session_reads_the_same_keys_as_the_slint_settings() {
        use tako_core::session::launch::keys as core;
        for (ui, core) in [
            (keys::RESTORE_ENVIRONMENT, core::RESTORE_ENVIRONMENT),
            (keys::GAME_SCREEN_ENABLED, core::GAME_SCREEN_ENABLED),
            (keys::GAME_SCREEN_ID, core::GAME_SCREEN_ID),
            (keys::GAME_SCREEN_MODE, core::GAME_SCREEN_MODE),
            (keys::GAME_SCREEN_HDR, core::GAME_SCREEN_HDR),
            (keys::GAME_SCREEN_VRR, core::GAME_SCREEN_VRR),
            (keys::GAME_SCREEN_PRESETS, core::GAME_SCREEN_PRESETS),
            (keys::GAME_SCREEN_CONFIRMED, core::GAME_SCREEN_CONFIRMED),
            (keys::GAME_AUDIO_ID, core::GAME_AUDIO_ID),
            (keys::GAME_AUDIO_NAME, core::GAME_AUDIO_NAME),
            (keys::HIDE_CURSOR, core::HIDE_CURSOR),
            (keys::CONTROLLER_MOUSE, core::CONTROLLER_MOUSE),
            (keys::DISCORD_PRESENCE, core::DISCORD_PRESENCE),
        ] {
            assert_eq!(ui, core);
        }
    }

    #[test]
    fn unapproved_commands_environment_and_args_are_not_applied_on_launch() {
        use std::sync::Arc;
        use tako_core::devices::fake::FakeProcesses;
        let dir = tempfile::tempdir().unwrap();
        let fake = Arc::new(FakeProcesses::default());
        let platform = Platform {
            processes: fake.clone(),
            ..Platform::simulated()
        };
        let mut ctx = build_ctx(DataPaths::at(dir.path()), platform.clone(), false);
        ctx.library.refresh();
        let exe = dir.path().join("jogo.exe");
        std::fs::write(&exe, "").unwrap();
        let record = tako_core::library::ManualGameRecord::create(
            "Jogo",
            &exe.to_string_lossy(),
            chrono::Utc::now(),
        );
        let id = record.id.clone();
        ctx.library.add_manual(record).unwrap();

        let store = ctx.game_settings.clone().unwrap();
        let mut o = store.load(&id).unwrap();
        o.extra_arguments = Some("--unapproved-arg".into());
        o.environment = Some("UNAPPROVED_VAR=1".into());
        o.before_command = Some("unapproved_script.sh".into());
        o.wine_runtime = Some("proton-ge".into());
        store.save(&id, &o).unwrap();

        assert!(!effective(&ctx, &id).commands_approved);

        let started = start(&mut ctx, &id).unwrap();
        if let Started::Tracking(req) = started {
            assert!(req.after_command.is_none());
            assert!(req.close_tools.is_empty());
        }

        let commands = fake.commands.lock().unwrap().clone();
        assert!(
            commands.is_empty(),
            "before_command não deve rodar sem aprovação"
        );

        let env = fake.env.lock().unwrap().clone();
        assert!(
            env.is_empty(),
            "variáveis de ambiente não aprovadas não devem ser aplicadas"
        );

        let launched = fake.launched.lock().unwrap().clone();
        assert!(!launched.is_empty());
        assert!(
            !launched[0].contains("--unapproved-arg"),
            "argumentos não aprovados não devem ser aplicados"
        );
    }

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
        // Outros testes abrem o mesmo jogo em paralelo e intercalam eventos: a ordem fica nos testes de `events`.
        for expected in [("starting", None), ("started", None), ("exited", Some(0))] {
            assert!(mine.contains(&expected), "{mine:?}");
        }
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
        o.approve_commands();
        store.save(&manual.id, &o).unwrap();

        let Started::Tracking(req) = start(&mut ctx, &manual.id).unwrap() else {
            panic!("deveria acompanhar")
        };
        let last = fake.launched.lock().unwrap().last().unwrap().clone();
        assert!(
            last.starts_with("wine /jogos/game.exe")
                || last.starts_with("/usr/bin/wine /jogos/game.exe")
        );
        assert_eq!(fake.cwd.lock().unwrap().as_deref(), Some("/jogos"));
        restore_now(&platform, &ctx.paths, req.restore);

        // 3. Com prefixo proibido (/ ou $HOME): recusa a abertura
        o.wine_prefix = Some("/".into());
        o.approve_commands();
        store.save(&manual.id, &o).unwrap();
        assert!(start(&mut ctx, &manual.id).is_err());
    }

    #[test]
    fn missing_runtime_produces_no_starting_event_no_tools_and_no_effects() {
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

        let mut o = store.load(&manual.id).unwrap();
        o.wine_runtime = Some("proton:inexistente".into());
        o.tools = Some("/Ferramentas/fps".into());
        o.approve_commands();
        store.save(&manual.id, &o).unwrap();

        let events = events::subscribe();

        let res = start(&mut ctx, &manual.id);
        assert!(res.is_err());

        // Prova: nenhum evento GameStarting publicado para este jogo
        let starting_events = events
            .try_iter()
            .filter(|e| matches!(e, SessionEvent::GameStarting { game, .. } if game.game_id == manual.id))
            .count();
        assert_eq!(starting_events, 0);

        // Prova: nenhuma ferramenta aberta
        let launched = fake.launched.lock().unwrap().clone();
        assert!(
            launched.is_empty(),
            "nenhuma ferramenta ou jogo deve ter sido lançado: {launched:?}"
        );

        // Prova: nenhum efeito gravado no guard de sessão
        assert!(
            !ctx.paths.session_environment().exists(),
            "guard de sessão não deve existir em falha prematura"
        );
    }
}
