//! Sessão de jogo pela ponte (WEB-20): `game.play` → `HostRequest::Launch` → `launch_game`, leitura sem bloquear,
//! `game.running`, `game.resume`, `game.end`, `game.started`, `game.ended` e a recuperação depois de uma queda. Só
//! funções de plataforma em memória: nenhum jogo abre e nada muda na tela ou no áudio desta máquina.

#![allow(clippy::unwrap_used)]

use serde_json::{Value, json};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};
use tako_app::protocol::{
    Call, ErrorCode, Event, GameEndReason, GameEndRequest, GameRequest, Reply, RunningState,
};
use tako_app::{AppService, Core, HostRequest, SessionUpdate};
use tako_core::devices::VrrStatus;
use tako_core::devices::fake::{FakeDisplays, FakeProcesses};
use tako_core::library::{Game, GameStore};
use tako_core::paths::DataPaths;
use tako_core::persistence::{SettingsStore, TakoDatabase};
use tako_platform::Platform;
use tako_sources::{LibraryScan, LibrarySource};

const ID: &str = "steam:10";

struct Stub(Vec<Game>);

impl LibrarySource for Stub {
    fn store(&self) -> GameStore {
        GameStore::Steam
    }
    fn scan(&self) -> LibraryScan {
        LibraryScan {
            games: self.0.clone(),
            client_installed: true,
            notice: None,
        }
    }
}

fn game() -> Game {
    Game {
        id: ID.into(),
        title: "Árvore".into(),
        store: Some(GameStore::Steam),
        is_installed: true,
        launch_target: Some("/jogos/arvore.exe".into()),
        ..Default::default()
    }
}

struct Rig {
    app: AppService,
    host: mpsc::Receiver<HostRequest>,
    events: mpsc::Receiver<Event>,
    processes: Arc<FakeProcesses>,
    displays: Arc<FakeDisplays>,
    platform: Platform,
    paths: DataPaths,
    _dir: Option<tempfile::TempDir>,
}

fn platform() -> (Platform, Arc<FakeProcesses>, Arc<FakeDisplays>) {
    let processes = Arc::new(FakeProcesses::default());
    let displays = Arc::new(FakeDisplays::desk());
    let platform = Platform {
        processes: processes.clone(),
        displays: displays.clone(),
        ..Platform::simulated()
    };
    (platform, processes, displays)
}

fn open(paths: DataPaths, platform: Platform, simulate: bool) -> AppService {
    let core = Core::open_with_sources(
        paths,
        platform,
        simulate,
        vec![Box::new(Stub(vec![game()]))],
    )
    .unwrap();
    let mut app = AppService::with_core("test", core);
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let Reply::LibrarySources(state) = app.handle(Call::LibrarySources).unwrap() else {
            panic!("relatório esperado");
        };
        let id = if simulate { "steam:demo0" } else { ID };
        if !state.scanning && app.handle(Call::GameDetails(request(id))).is_ok() {
            return app;
        }
        assert!(Instant::now() < deadline, "leitura não terminou");
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// Núcleo de verdade (banco numa pasta temporária) com tela de jogo e VRR ligados: o VRR da TV simulada é o sinal
/// de que a tela do jogo foi aplicada e restaurada.
fn rig() -> Rig {
    rig_with(|p| p)
}

fn rig_with(change: impl FnOnce(Platform) -> Platform) -> Rig {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path());
    let database = TakoDatabase::new(paths.database());
    database.open().unwrap();
    let settings = SettingsStore::new(database);
    for (key, value) in [
        ("GameScreenEnabled", "true"),
        ("GameScreenId", "DISPLAY2"),
        ("GameScreenVrr", "true"),
        ("HideCursorDuringGame", "true"),
    ] {
        settings.set(key, value).unwrap();
    }
    let (platform, processes, displays) = platform();
    let platform = change(platform);
    let mut rig = attach(open(paths.clone(), platform.clone(), false));
    rig.processes = processes;
    rig.displays = displays;
    rig.platform = platform;
    rig.paths = paths;
    rig._dir = Some(dir);
    rig
}

fn attach(mut app: AppService) -> Rig {
    let (host_tx, host) = mpsc::channel();
    let (event_tx, events) = mpsc::channel();
    app.set_host_sink(host_tx);
    app.set_event_sink(event_tx);
    let (platform, processes, displays) = platform();
    Rig {
        app,
        host,
        events,
        processes,
        displays,
        platform,
        paths: DataPaths::at(std::env::temp_dir()),
        _dir: None,
    }
}

fn request(id: &str) -> GameRequest {
    GameRequest { game_id: id.into() }
}

impl Rig {
    /// `game.play` e o que o hospedeiro faz com o pedido.
    fn play(&mut self, id: &str) {
        assert_eq!(
            self.app.handle(Call::GamePlay(request(id))).unwrap(),
            Reply::GamePlay
        );
        let launch = self
            .host
            .try_iter()
            .find(|r| matches!(r, HostRequest::Launch { .. }))
            .unwrap();
        let HostRequest::Launch { game_id } = launch else {
            unreachable!()
        };
        self.app.launch_game(&game_id);
    }

    fn events(&self) -> Vec<Value> {
        self.events
            .try_iter()
            .map(|e| serde_json::to_value(e).unwrap())
            .collect()
    }

    fn running(&mut self) -> Value {
        let Reply::GameRunning(running) = self.app.handle(Call::GameRunning).unwrap() else {
            panic!("game.running");
        };
        serde_json::to_value(running).unwrap()
    }

    /// Lê até a sessão mudar, sem esperar o prazo de leitura (que é do hospedeiro). O fim de um jogo que fechou
    /// sozinho espera a folga do acompanhamento (4 s, troca de launcher para jogo).
    fn poll_until_update(&mut self) -> SessionUpdate {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            let at = self.app.session_deadline().expect("sem jogo");
            if let Some(update) = self.app.poll_session(at) {
                return update;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("a sessão não mudou");
    }

    fn vrr(&self) -> VrrStatus {
        tako_core::devices::Displays::list(self.displays.as_ref()).unwrap()[1].vrr
    }

    fn play_time(&mut self) -> u64 {
        let Reply::GameDetails(details) = self.app.handle(Call::GameDetails(request(ID))).unwrap()
        else {
            panic!("game.details");
        };
        details.game.play_time_seconds
    }
}

fn named<'a>(events: &'a [Value], name: &str) -> Vec<&'a Value> {
    events.iter().filter(|e| e["name"] == name).collect()
}

#[test]
fn a_game_opens_with_its_screen_hides_the_ui_and_comes_back_restored_with_the_time() {
    let mut rig = rig();
    assert_eq!(rig.running(), Value::Null);
    let played_before = rig.play_time();
    rig.play(ID);
    assert_eq!(rig.vrr(), VrrStatus::On, "tela do jogo aplicada");
    assert!(rig.platform.cursor.is_hidden(), "cursor escondido");
    assert!(rig.paths.session_environment().exists(), "diário da sessão");
    let events = rig.events();
    let started = named(&events, "game.started");
    assert_eq!(started.len(), 1, "{events:?}");
    assert_eq!(started[0]["payload"]["gameId"], ID);
    assert_eq!(started[0]["payload"]["tracked"], true);
    assert_eq!(rig.running()["state"], "launching");
    assert_eq!(rig.running()["title"], "Árvore");

    // Um jogo por vez.
    let busy = rig.app.handle(Call::GamePlay(request(ID))).unwrap_err();
    assert_eq!(busy.code, ErrorCode::Conflict);

    // A janela do jogo aparece: o hospedeiro fecha a interface.
    assert_eq!(rig.poll_until_update(), SessionUpdate::Shown);
    assert_eq!(rig.running()["state"], "playing");
    // Sem nada mudar, a leitura antes do prazo não faz nada.
    assert_eq!(rig.app.poll_session(Instant::now()), None);

    // O jogo cai sozinho logo depois de abrir, com um código de erro.
    let pid = rig.processes.snapshot_pids()[0];
    rig.processes.exit_with(pid, 0xC000_0005);
    assert_eq!(rig.poll_until_update(), SessionUpdate::Ended);
    assert_eq!(rig.vrr(), VrrStatus::Off, "tela restaurada");
    assert!(!rig.platform.cursor.is_hidden(), "cursor de volta");
    assert!(!rig.paths.session_environment().exists());
    assert_eq!(rig.running(), Value::Null);
    assert!(rig.app.session_deadline().is_none());
    let events = rig.events();
    let ended = named(&events, "game.ended");
    assert_eq!(ended.len(), 2, "o fim e a repetição depois do game.running");
    assert_eq!(ended[0], ended[1]);
    let ended = &ended[0]["payload"];
    assert_eq!(ended["gameId"], ID);
    // Fechou em menos de 10 s: "caiu ao abrir", com o código explicado.
    assert_eq!(ended["end"], "quick");
    assert_eq!(ended["exitCode"], 0xC000_0005u32);
    assert!(ended["exitCodeText"].as_str().unwrap().contains("C0000005"));
    assert!(!named(&events, "library.changed").is_empty());
    assert!(rig.play_time() >= played_before);
    // Já entregue: não repete.
    rig.running();
    assert!(named(&rig.events(), "game.ended").is_empty());
}

#[test]
fn game_end_closes_the_game_by_request_or_by_force_and_says_how_it_ended() {
    for force in [false, true] {
        let mut rig = rig();
        let no_game = rig
            .app
            .handle(Call::GameEnd(GameEndRequest::default()))
            .unwrap_err();
        assert_eq!(no_game.code, ErrorCode::Conflict);
        rig.play(ID);
        rig.poll_until_update();
        assert_eq!(
            rig.app
                .handle(Call::GameEnd(GameEndRequest { force }))
                .unwrap(),
            Reply::GameEnd
        );
        assert!(rig.processes.snapshot_pids().is_empty());
        assert_eq!(rig.running()["state"], "closing");
        assert_eq!(rig.poll_until_update(), SessionUpdate::Ended);
        // A interface estava aberta e viu o fim: o hospedeiro avisa, e o game.running não repete.
        rig.app.session_end_seen();
        assert_eq!(rig.running(), Value::Null);
        let events = rig.events();
        let ended = named(&events, "game.ended");
        assert_eq!(ended.len(), 1);
        assert_eq!(ended[0]["payload"]["end"], "normal");
        assert_eq!(ended[0]["payload"]["exitCode"], Value::Null);
        assert_eq!(rig.vrr(), VrrStatus::Off);
    }
}

#[test]
fn game_resume_asks_the_host_to_bring_the_game_forward() {
    let mut rig = rig();
    let no_game = rig.app.handle(Call::GameResume).unwrap_err();
    assert_eq!(no_game.code, ErrorCode::Conflict);
    rig.play(ID);
    assert_eq!(rig.app.handle(Call::GameResume).unwrap(), Reply::GameResume);
    assert_eq!(rig.host.try_recv().unwrap(), HostRequest::Resume);
    rig.app
        .handle(Call::GameEnd(GameEndRequest { force: true }))
        .unwrap();
    rig.poll_until_update();
}

#[test]
fn a_game_that_does_not_open_is_reported_and_changes_nothing() {
    let mut rig = rig_with(|p| Platform {
        processes: Arc::new(Failing),
        ..p
    });
    rig.play(ID);
    let events = rig.events();
    let ended = named(&events, "game.ended");
    assert_eq!(ended.len(), 1, "{events:?}");
    assert_eq!(ended[0]["payload"]["end"], "launch_failed");
    assert!(ended[0]["payload"]["message"].is_string());
    assert!(named(&events, "game.started").is_empty());
    assert_eq!(rig.vrr(), VrrStatus::Off, "a tela voltou");
    assert!(!rig.paths.session_environment().exists());
    assert_eq!(rig.running(), Value::Null);
}

#[test]
fn a_core_that_dies_in_the_middle_of_the_game_restores_the_screen_on_the_next_open() {
    let mut rig = rig();
    rig.play(ID);
    assert_eq!(rig.vrr(), VrrStatus::On);
    // Queda: o núcleo some sem `finish`. O diário fica no disco.
    let Rig {
        app,
        platform,
        paths,
        displays,
        _dir,
        ..
    } = rig;
    drop(app);
    assert!(paths.session_environment().exists());
    let _next = open(paths.clone(), platform, false);
    assert_eq!(displays.list_vrr(1), VrrStatus::Off, "restaurado ao abrir");
    assert!(!paths.session_environment().exists());
}

#[test]
fn the_demo_session_is_simulated_without_touching_the_system() {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path());
    // A plataforma "da máquina" recebida pelo núcleo: a demonstração não pode usá-la.
    let (machine, processes, displays) = platform();
    let mut rig = attach(open(paths.clone(), machine, true));
    let id = "steam:demo0";
    rig.play(id);
    assert!(processes.launched.lock().unwrap().is_empty(), "nada abriu");
    assert_eq!(displays.list_vrr(1), VrrStatus::Off);
    assert!(!paths.session_environment().exists());
    let events = rig.events();
    assert_eq!(named(&events, "game.started")[0]["payload"]["gameId"], id);
    assert_eq!(rig.running()["gameId"], id);
    assert_eq!(
        serde_json::from_value::<RunningState>(rig.running()["state"].clone()).unwrap(),
        RunningState::Launching
    );
    rig.app
        .handle(Call::GameEnd(GameEndRequest::default()))
        .unwrap();
    while rig.poll_until_update() != SessionUpdate::Ended {}
    let events = rig.events();
    let ended = named(&events, "game.ended");
    assert_eq!(
        serde_json::from_value::<GameEndReason>(ended[0]["payload"]["end"].clone()).unwrap(),
        GameEndReason::Normal
    );
    assert!(processes.launched.lock().unwrap().is_empty());
}

#[test]
fn session_calls_on_the_wire() {
    let mut rig = attach(open(
        DataPaths::at(tempfile::tempdir().unwrap().path()),
        Platform::simulated(),
        true,
    ));
    let wire = |app: &mut AppService, text: &str| -> Value {
        let response = match tako_app::protocol::parse_request(text) {
            Ok(request) => match app.handle(request.call) {
                Ok(reply) => tako_app::protocol::Response::ok(request.id, reply),
                Err(error) => tako_app::protocol::Response::err(request.id, error),
            },
            Err((id, error)) => tako_app::protocol::Response::err(id, error),
        };
        serde_json::to_value(response).unwrap()
    };
    assert_eq!(
        wire(&mut rig.app, r#"{"v":1,"id":1,"name":"game.running"}"#),
        json!({"v":1,"id":1,"ok":true,"reply":{"name":"game.running","data":null}})
    );
    // Sem argumentos para consulta e retomada; `game.end` exige um objeto.
    for text in [
        r#"{"v":1,"id":2,"name":"game.running","payload":{}}"#,
        r#"{"v":1,"id":2,"name":"game.resume","payload":{}}"#,
        r#"{"v":1,"id":2,"name":"game.end"}"#,
        r#"{"v":1,"id":2,"name":"game.end","payload":[]}"#,
        r#"{"v":1,"id":2,"name":"game.end","payload":{"force":true,"x":1}}"#,
    ] {
        assert_eq!(
            wire(&mut rig.app, text)["error"]["code"],
            "bad_request",
            "{text}"
        );
    }
    assert_eq!(
        wire(
            &mut rig.app,
            r#"{"v":1,"id":3,"name":"game.end","payload":{}}"#
        )["error"]["code"],
        "conflict"
    );
    assert_eq!(
        wire(&mut rig.app, r#"{"v":1,"id":4,"name":"game.resume"}"#)["error"]["code"],
        "conflict"
    );
    rig.play("steam:demo0");
    let running = wire(&mut rig.app, r#"{"v":1,"id":5,"name":"game.running"}"#);
    let data = &running["reply"]["data"];
    assert_eq!(data["gameId"], "steam:demo0");
    assert_eq!(data["state"], "launching");
    assert!(
        chrono::DateTime::parse_from_rfc3339(data["startedAt"].as_str().unwrap()).is_ok(),
        "{data}"
    );
    assert_eq!(
        wire(
            &mut rig.app,
            r#"{"v":1,"id":6,"name":"game.end","payload":{"force":true}}"#
        ),
        json!({"v":1,"id":6,"ok":true,"reply":{"name":"game.end"}})
    );
    assert_eq!(
        wire(&mut rig.app, r#"{"v":1,"id":7,"name":"game.resume"}"#),
        json!({"v":1,"id":7,"ok":true,"reply":{"name":"game.resume"}})
    );
}

trait Pids {
    fn snapshot_pids(&self) -> Vec<u32>;
}

impl Pids for FakeProcesses {
    fn snapshot_pids(&self) -> Vec<u32> {
        tako_core::devices::ProcessLauncher::snapshot(self)
            .iter()
            .map(|p| p.pid)
            .collect()
    }
}

trait Vrr {
    fn list_vrr(&self, i: usize) -> VrrStatus;
}

impl Vrr for FakeDisplays {
    fn list_vrr(&self, i: usize) -> VrrStatus {
        tako_core::devices::Displays::list(self).unwrap()[i].vrr
    }
}

/// Processos que nunca abrem (cliente da loja ausente, executável apagado).
struct Failing;

impl tako_core::devices::ProcessLauncher for Failing {
    fn launch(
        &self,
        _target: &str,
        _arguments: Option<&str>,
        _working_dir: Option<&str>,
    ) -> tako_core::devices::DeviceResult<Option<u32>> {
        Err(tako_core::devices::DeviceError::NotFound("jogo".into()))
    }
    fn snapshot(&self) -> Vec<tako_core::devices::ProcessEntry> {
        Vec::new()
    }
    fn request_close(&self, _pids: &[u32]) -> tako_core::devices::DeviceResult<()> {
        Ok(())
    }
    fn force_close(&self, _pids: &[u32]) -> tako_core::devices::DeviceResult<()> {
        Ok(())
    }
    fn steam_running_app_id(&self) -> u32 {
        0
    }
    fn open_folder(&self, _path: &str) -> tako_core::devices::DeviceResult<()> {
        Ok(())
    }
}
