//! Leitura assíncrona com fontes falsas: nenhuma loja instalada é necessária.

#![allow(clippy::unwrap_used)]

use serde_json::{Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::ThreadId;
use std::time::{Duration, Instant};
use tako_app::protocol::{Call, ErrorCode, Event, LibrarySources, Reply, Response};
use tako_app::{AppService, Core, HostRequest};
use tako_core::library::{Game, GameStore};
use tako_core::paths::DataPaths;
use tako_core::persistence::{SettingsStore, TakoDatabase};
use tako_platform::Platform;
use tako_sources::{LibraryScan, LibrarySource};

fn game(id: &str, store: GameStore) -> Game {
    Game {
        id: id.into(),
        title: id.into(),
        store: Some(store),
        is_installed: true,
        ..Default::default()
    }
}

struct Stub(GameStore, LibraryScan);

impl LibrarySource for Stub {
    fn store(&self) -> GameStore {
        self.0
    }
    fn scan(&self) -> LibraryScan {
        self.1.clone()
    }
}

struct Gated {
    entered: mpsc::Sender<ThreadId>,
    release: Mutex<mpsc::Receiver<()>>,
    calls: Arc<AtomicUsize>,
    games: Vec<Game>,
}

impl LibrarySource for Gated {
    fn store(&self) -> GameStore {
        GameStore::Steam
    }
    fn scan(&self) -> LibraryScan {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.entered.send(std::thread::current().id()).unwrap();
        self.release
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(10))
            .unwrap();
        LibraryScan {
            games: self.games.clone(),
            client_installed: true,
            notice: None,
        }
    }
}

fn state(app: &mut AppService) -> LibrarySources {
    let Reply::LibrarySources(state) = app.handle(Call::LibrarySources).unwrap() else {
        panic!("relatório esperado");
    };
    state
}

fn finish(app: &mut AppService) -> LibrarySources {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let state = state(app);
        if !state.scanning {
            return state;
        }
        assert!(Instant::now() < deadline, "leitura não terminou");
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn wire(app: &mut AppService, request: Value) -> Value {
    let response = match tako_app::protocol::parse_request(&request.to_string()) {
        Ok(request) => match app.handle(request.call) {
            Ok(reply) => Response::ok(request.id, reply),
            Err(error) => Response::err(request.id, error),
        },
        Err((id, error)) => Response::err(id, error),
    };
    let text = serde_json::to_string(&response).unwrap();
    let decoded: Response = serde_json::from_str(&text).unwrap();
    assert_eq!(decoded, response);
    serde_json::from_str(&text).unwrap()
}

#[test]
fn boot_scan_is_off_core_thread_nonblocking_single_and_applied_only_by_poll() {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path());
    let (entered, started) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let calls = Arc::new(AtomicUsize::new(0));
    let mut discovered = game("steam:42", GameStore::Steam);
    discovered.install_path = Some(dir.path().join("install").to_string_lossy().into());
    std::fs::create_dir(dir.path().join("install")).unwrap();
    std::fs::write(dir.path().join("install/data"), vec![0; 1024]).unwrap();
    let core = Core::open_with_sources(
        paths.clone(),
        Platform::simulated(),
        false,
        vec![Box::new(Gated {
            entered,
            release: Mutex::new(gate),
            calls: Arc::clone(&calls),
            games: vec![discovered],
        })],
    )
    .unwrap();
    assert_ne!(
        started.recv_timeout(Duration::from_secs(5)).unwrap(),
        std::thread::current().id()
    );
    assert!(core.library().games().is_empty());
    let mut app = AppService::with_core("test", core);
    let (events, received) = mpsc::channel();
    let (host, wakes) = mpsc::channel();
    app.set_event_sink(events);
    app.set_host_sink(host);
    let before = Instant::now();
    for _ in 0..20 {
        assert_eq!(app.handle(Call::LibraryScan).unwrap(), Reply::LibraryScan);
        assert!(state(&mut app).scanning);
        assert!(matches!(
            app.handle(Call::AppInfo).unwrap(),
            Reply::AppInfo(_)
        ));
    }
    // O trabalhador continua preso na fonte, mas o núcleo respondeu a todas as chamadas.
    assert!(before.elapsed() < Duration::from_secs(1));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let mut delivered: Vec<_> = received.try_iter().collect();
    assert!(!delivered.contains(&Event::LibraryChanged));
    release.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert_eq!(
            wakes.recv_timeout(Duration::from_secs(5)).unwrap(),
            HostRequest::PollBackground
        );
        // Este é o caminho do hospedeiro sem uma chamada da interface.
        app.poll();
        delivered.extend(received.try_iter());
        if delivered.contains(&Event::LibraryChanged) {
            break;
        }
        assert!(Instant::now() < deadline);
    }
    let final_state = state(&mut app);
    assert!(delivered.iter().any(|event| matches!(event, Event::LibraryScanProgress(progress) if progress.measuring_sizes && progress.scanning)));
    assert!(
        matches!(delivered.last(), Some(Event::LibraryScanProgress(progress)) if !progress.scanning)
    );
    assert!(!final_state.scanning);
    assert!(!final_state.measuring_sizes);
    assert_eq!((final_state.completed, final_state.total), (1, 1));
    assert_eq!(final_state.sources[0].found, 1);
    assert!(final_state.sources[0].last_scanned_at.is_some());
    let Reply::GameDetails(details) = app
        .handle(Call::GameDetails(tako_app::protocol::GameRequest {
            game_id: "steam:42".into(),
        }))
        .unwrap()
    else {
        panic!("detalhes esperados");
    };
    assert_eq!(details.size_bytes, Some(1024));
    assert!(details.size_label.is_some());
    // Um pedido depois de terminar inicia exatamente outra leitura.
    app.handle(Call::LibraryScan).unwrap();
    started.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(state(&mut app).sources, final_state.sources);
    release.send(()).unwrap();
    finish(&mut app);
    drop(app); // drena só o gravador do snapshot, depois que a leitura terminou
    let saved: Vec<Game> =
        serde_json::from_slice(&std::fs::read(paths.cache().join("found-games.json")).unwrap())
            .unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].id, "steam:42");
    assert_eq!(saved[0].size_bytes, Some(1024));
}

struct Broken;

impl LibrarySource for Broken {
    fn store(&self) -> GameStore {
        GameStore::Epic
    }
    fn scan(&self) -> LibraryScan {
        panic!("fonte falsa quebrada")
    }
}

#[test]
fn errors_disabled_sources_and_duplicates_do_not_stop_other_stores() {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path());
    SettingsStore::new(TakoDatabase::new(paths.database()))
        .set("DisabledStores", " ePiC ")
        .unwrap();
    let good = game("gog:1", GameStore::Gog);
    let core = Core::open_with_sources(
        paths,
        Platform::simulated(),
        false,
        vec![
            Box::new(Stub(
                GameStore::Steam,
                LibraryScan::missing("Manifesto Steam inválido"),
            )),
            Box::new(Broken), // desligada: o scan nem é chamado
            Box::new(Stub(
                GameStore::Gog,
                LibraryScan {
                    games: vec![good.clone(), good],
                    ..Default::default()
                },
            )),
        ],
    )
    .unwrap();
    let mut app = AppService::with_core("test", core);
    let state = finish(&mut app);
    assert_eq!(
        state.sources[0].error.as_deref(),
        Some("Manifesto Steam inválido")
    );
    assert!(state.sources[0].last_scanned_at.is_some());
    assert!(!state.sources[1].enabled);
    assert!(state.sources[1].error.is_none());
    assert!(state.sources[1].last_scanned_at.is_none());
    assert_eq!(state.sources[2].found, 1);
    assert_eq!(
        wire(
            &mut app,
            json!({"v":1,"id":3,"name":"library.list","payload":{}})
        )["reply"]["data"]["total"],
        1
    );
}

#[test]
fn a_panicking_source_is_reported_and_later_sources_still_run() {
    let dir = tempfile::tempdir().unwrap();
    let core = Core::open_with_sources(
        DataPaths::at(dir.path()),
        Platform::simulated(),
        false,
        vec![
            Box::new(Broken),
            Box::new(Stub(
                GameStore::Gog,
                LibraryScan {
                    games: vec![game("gog:2", GameStore::Gog)],
                    ..Default::default()
                },
            )),
        ],
    )
    .unwrap();
    let mut app = AppService::with_core("test", core);
    let state = finish(&mut app);
    assert!(state.sources[0].error.is_some());
    assert_eq!(state.sources[1].found, 1);
    assert!(state.error.is_none());
}

#[test]
fn json_scan_and_sources_validate_payload_and_require_core() {
    let dir = tempfile::tempdir().unwrap();
    let core = Core::open_with_sources(
        DataPaths::at(dir.path()),
        Platform::simulated(),
        false,
        vec![Box::new(Stub(
            GameStore::Steam,
            LibraryScan {
                games: vec![game("steam:1", GameStore::Steam)],
                ..Default::default()
            },
        ))],
    )
    .unwrap();
    let mut app = AppService::with_core("test", core);
    finish(&mut app);
    for name in ["library.scan", "library.sources"] {
        let reply = wire(&mut app, json!({"v":1,"id":7,"name":name}));
        assert_eq!(reply["ok"], true);
        assert_eq!(reply["id"], 7);
        assert_eq!(reply["reply"]["name"], name);
        for payload in [
            json!({}),
            json!([]),
            json!("oops"),
            json!(1),
            json!(null),
            json!({"extra":true}),
        ] {
            let response = wire(
                &mut app,
                json!({"v":1,"id":8,"name":name,"payload":payload}),
            );
            assert_eq!(response["id"], 8);
            assert_eq!(response["error"]["code"], "bad_request");
        }
        let error = wire(
            &mut AppService::new("test"),
            json!({"v":1,"id":9,"name":name}),
        );
        assert_eq!(error["error"]["code"], "internal");
    }
    let report = finish(&mut app);
    let event: Value = serde_json::to_value(tako_app::protocol::EventEnvelope::from(
        Event::LibraryScanProgress(report),
    ))
    .unwrap();
    assert_eq!(event["event"]["name"], "library.scan.progress");
    assert_eq!(event["event"]["payload"]["sources"][0]["store"], "steam");
    assert!(event["event"]["payload"]["sources"][0]["lastScannedAt"].is_string());
    assert_eq!(
        app.handle(Call::GameDetails(tako_app::protocol::GameRequest {
            game_id: "absent".into()
        }))
        .unwrap_err()
        .code,
        ErrorCode::NotFound
    );
}

#[test]
fn demonstration_never_scans_even_on_explicit_requests() {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path().join("untouched"));
    let core = Core::open_with_sources(
        paths.clone(),
        Platform::simulated(),
        true,
        vec![Box::new(Broken)],
    )
    .unwrap();
    let mut app = AppService::with_core("test", core);
    let (sink, events) = mpsc::channel();
    app.set_event_sink(sink);
    for _ in 0..3 {
        assert_eq!(app.handle(Call::LibraryScan).unwrap(), Reply::LibraryScan);
        assert_eq!(state(&mut app), LibrarySources::default());
    }
    assert!(events.try_recv().is_err());
    assert_eq!(
        wire(
            &mut app,
            json!({"v":1,"id":1,"name":"library.list","payload":{}})
        )["reply"]["data"]["total"],
        12
    );
    drop(app);
    assert!(!paths.root.exists());
}

#[test]
fn emulator_roms_are_discovered_from_a_temporary_environment() {
    use tako_sources::emulators::{Config, EmulatorSource, Fs, RomFolder, emulator::Env};
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("bin")).unwrap();
    std::fs::create_dir(dir.path().join("roms")).unwrap();
    std::fs::write(dir.path().join("bin/duckstation-qt"), "").unwrap();
    let rom = dir.path().join("roms/Fixture.chd");
    std::fs::write(&rom, "rom fictícia").unwrap();
    let source = EmulatorSource {
        env: Some(Env {
            os: Some(tako_core::paths::Os::Linux),
            home: dir.path().to_owned(),
            path: vec![dir.path().join("bin")],
            ..Default::default()
        }),
        config: Config {
            rom_folders: vec![RomFolder {
                path: dir.path().join("roms"),
                system: Some("psx".into()),
            }],
            ..Default::default()
        },
        fs: Fs::real(),
    };
    let core = Core::open_with_sources(
        DataPaths::at(dir.path().join("data")),
        Platform::simulated(),
        false,
        vec![Box::new(source)],
    )
    .unwrap();
    let mut app = AppService::with_core("test", core);
    let report = finish(&mut app);
    assert_eq!(
        report.sources[0].store,
        tako_app::protocol::Store::Emulators
    );
    assert_eq!(report.sources[0].found, 1);
    let id = tako_sources::emulators::game_id(&rom);
    let Reply::GameDetails(details) = app
        .handle(Call::GameDetails(tako_app::protocol::GameRequest {
            game_id: id,
        }))
        .unwrap()
    else {
        panic!("detalhes esperados");
    };
    assert!(details.can_play);
    assert_eq!(details.size_bytes, Some("rom fictícia".len() as u64));
}

#[test]
#[ignore = "Lê as lojas reais da máquina; executar manualmente com --ignored --nocapture"]
fn real_machine_scan() {
    let dir = tempfile::tempdir().unwrap();
    let before = Instant::now();
    let core = Core::open(DataPaths::at(dir.path()), Platform::simulated(), false).unwrap();
    let mut app = AppService::with_core("real-scan", core);
    let deadline = Instant::now() + Duration::from_secs(600);
    let report = loop {
        let report = state(&mut app);
        if !report.scanning {
            break report;
        }
        assert!(Instant::now() < deadline, "leitura real excedeu 10 minutos");
        std::thread::sleep(Duration::from_millis(10));
    };
    for source in report.sources {
        println!(
            "{}: {} jogos; {} ms; erro/aviso: {}",
            source.name,
            source.found,
            source.elapsed_ms.unwrap_or(0),
            source.error.as_deref().unwrap_or("nenhum")
        );
    }
    println!("Tempo total: {:.3} s", before.elapsed().as_secs_f64());
    assert!(report.error.is_none(), "{:?}", report.error);
}

#[test]
fn a_new_scan_uses_the_stores_disabled_after_opening() {
    let dir = tempfile::tempdir().unwrap();
    let core = Core::open_with_sources(
        DataPaths::at(dir.path()),
        Platform::simulated(),
        false,
        vec![Box::new(Stub(
            GameStore::Gog,
            LibraryScan {
                games: vec![game("gog:1", GameStore::Gog)],
                ..Default::default()
            },
        ))],
    )
    .unwrap();
    let mut app = AppService::with_core("test", core);
    assert!(finish(&mut app).sources[0].enabled);

    let set = wire(
        &mut app,
        json!({"v":1,"id":1,"name":"settings.set","payload":{"key":"DisabledStores","value":"gog"}}),
    );
    assert_eq!(set["ok"], true, "{set}");
    let scan = wire(&mut app, json!({"v":1,"id":2,"name":"library.scan"}));
    assert_eq!(scan["ok"], true, "{scan}");

    let state = finish(&mut app);
    assert!(!state.sources[0].enabled);
    assert_eq!(
        wire(
            &mut app,
            json!({"v":1,"id":3,"name":"library.list","payload":{}})
        )["reply"]["data"]["total"],
        0
    );
}
