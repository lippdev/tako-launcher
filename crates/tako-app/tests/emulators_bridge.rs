//! Contrato dos emuladores e pastas de ROMs (`emulators.*`) e das chaves novas de aparência do desktop, pelo mesmo
//! parser JSON do hospedeiro.
#![allow(clippy::unwrap_used)]

use serde_json::{Value, json};
use std::path::Path;
use std::time::{Duration, Instant};
use tako_app::protocol::{self, Call, Reply, Response};
use tako_app::{AppService, Core};
use tako_core::library::{Game, GameStore};
use tako_core::paths::DataPaths;
use tako_core::persistence::{SettingsStore, TakoDatabase};
use tako_platform::Platform;
use tako_sources::emulators::Config;
use tako_sources::{LibraryScan, LibrarySource};

/// Uma loja que conta quantas vezes foi lida.
struct Counting(std::sync::Arc<std::sync::atomic::AtomicUsize>);

impl LibrarySource for Counting {
    fn store(&self) -> GameStore {
        GameStore::Gog
    }
    fn scan(&self) -> LibraryScan {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        LibraryScan {
            games: vec![Game {
                id: "gog:1".into(),
                title: "Jogo".into(),
                store: Some(GameStore::Gog),
                ..Default::default()
            }],
            ..Default::default()
        }
    }
}

fn settle(app: &mut AppService) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while matches!(
        app.handle(Call::LibrarySources),
        Ok(Reply::LibrarySources(s)) if s.scanning
    ) {
        assert!(Instant::now() < deadline, "leitura não terminou");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn wire(app: &mut AppService, name: &str, payload: Option<Value>) -> Value {
    let mut value = json!({"v":1,"id":7,"name":name});
    if let Some(payload) = payload {
        value["payload"] = payload;
    }
    let response = match protocol::parse_request(&value.to_string()) {
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

fn ok(app: &mut AppService, name: &str, payload: Option<Value>) -> Value {
    let reply = wire(app, name, payload);
    assert_eq!(reply["ok"], true, "{name}: {reply}");
    assert_eq!(reply["reply"]["name"], name);
    reply["reply"]["data"].clone()
}

fn err(app: &mut AppService, name: &str, payload: Option<Value>, code: &str) {
    let reply = wire(app, name, payload);
    assert_eq!(reply["ok"], false, "{name}: {reply}");
    assert_eq!(reply["error"]["code"], code, "{name}: {reply}");
}

fn open_with(paths: DataPaths, simulate: bool) -> AppService {
    let core = Core::open_with_sources(
        paths,
        Platform::simulated(),
        simulate,
        vec![Box::new(Counting(Default::default()))],
    )
    .unwrap();
    let mut app = AppService::with_core("test", core);
    settle(&mut app);
    app
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn folders(view: &Value) -> Vec<(String, Value)> {
    view["romFolders"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| (f["path"].as_str().unwrap().to_owned(), f["system"].clone()))
        .collect()
}

/// O que o app Slint leria: as três chaves gravadas, no formato de `Config::save`.
fn saved_config(dir: &Path) -> Config {
    let all = SettingsStore::new(TakoDatabase::new(DataPaths::at(dir).database()))
        .load_all()
        .unwrap();
    let get = |key: &str| all.get(key).map(String::as_str);
    Config::parse(
        get("EmulatorRomFolders"),
        get("EmulatorPaths"),
        get("EmulatorBySystem"),
    )
}

#[test]
fn list_reports_every_emulator_the_folder_systems_and_no_folders_at_first() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = open_with(DataPaths::at(dir.path()), false);
    let view = ok(&mut app, "emulators.list", None);
    assert_eq!(folders(&view), []);
    assert_eq!(view["maxFolders"], 256);
    let keys: Vec<&str> = view["emulators"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["key"].as_str().unwrap())
        .collect();
    assert_eq!(keys.len(), 8);
    assert_eq!(keys[0], "retroarch");
    let retroarch = &view["emulators"][0];
    assert_eq!(retroarch["name"], "RetroArch");
    // O que foi achado depende do PC; a forma não.
    assert_eq!(
        retroarch["found"].as_bool(),
        Some(!retroarch["path"].is_null())
    );
    let systems = view["systems"].as_array().unwrap();
    assert!(systems.iter().any(|s| s["id"] == "snes"));
    assert!(systems.iter().any(|s| s["id"] == "ps3"));
    // Sistema sem extensão nem pasta própria não aparece como escolha.
    assert!(
        systems
            .iter()
            .all(|s| !s["name"].as_str().unwrap().is_empty())
    );
    // A lista não aceita argumentos.
    err(&mut app, "emulators.list", Some(json!({})), "bad_request");
}

#[test]
fn adding_a_folder_saves_the_slint_format_is_idempotent_and_survives_reopening() {
    let dir = tempfile::tempdir().unwrap();
    let roms = dir.path().join("roms");
    std::fs::create_dir(&roms).unwrap();
    let mut app = open_with(DataPaths::at(dir.path().join("data")), false);

    let view = ok(
        &mut app,
        "emulators.folder.add",
        Some(json!({"path": path_text(&roms)})),
    );
    assert_eq!(folders(&view), [(path_text(&roms), Value::Null)]);
    // Duas vezes a mesma pasta: continua uma só e não é erro.
    let again = ok(
        &mut app,
        "emulators.folder.add",
        Some(json!({"path": format!("  {}  ", path_text(&roms))})),
    );
    assert_eq!(folders(&again).len(), 1);

    let saved = saved_config(&dir.path().join("data"));
    assert_eq!(saved.rom_folders.len(), 1);
    assert_eq!(saved.rom_folders[0].path, roms);
    assert_eq!(saved.rom_folders[0].system, None);

    drop(app);
    let mut app = open_with(DataPaths::at(dir.path().join("data")), false);
    let view = ok(&mut app, "emulators.list", None);
    assert_eq!(folders(&view), [(path_text(&roms), Value::Null)]);
}

#[test]
fn removing_a_folder_and_choosing_its_system() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (dir.path().join("a"), dir.path().join("b"));
    std::fs::create_dir(&a).unwrap();
    std::fs::create_dir(&b).unwrap();
    let mut app = open_with(DataPaths::at(dir.path().join("data")), false);
    for folder in [&a, &b] {
        ok(
            &mut app,
            "emulators.folder.add",
            Some(json!({"path": path_text(folder)})),
        );
    }

    let view = ok(
        &mut app,
        "emulators.folder.system.set",
        Some(json!({"path": path_text(&a), "system": "snes"})),
    );
    assert_eq!(
        folders(&view),
        [(path_text(&a), json!("snes")), (path_text(&b), Value::Null)]
    );
    assert_eq!(
        saved_config(&dir.path().join("data")).rom_folders[0]
            .system
            .as_deref(),
        Some("snes")
    );
    // Voltar a "uma subpasta por sistema".
    let view = ok(
        &mut app,
        "emulators.folder.system.set",
        Some(json!({"path": path_text(&a), "system": null})),
    );
    assert_eq!(folders(&view)[0].1, Value::Null);

    // Tirar não olha o disco: a pasta pode já ter sumido.
    std::fs::remove_dir(&b).unwrap();
    let view = ok(
        &mut app,
        "emulators.folder.remove",
        Some(json!({"path": path_text(&b)})),
    );
    assert_eq!(folders(&view), [(path_text(&a), Value::Null)]);
    assert_eq!(saved_config(&dir.path().join("data")).rom_folders.len(), 1);
    // Os arquivos da pasta tirada da lista ficam onde estavam.
    assert!(a.exists());
}

#[test]
fn unknown_folders_are_not_found_and_invalid_input_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let roms = dir.path().join("roms");
    std::fs::create_dir(&roms).unwrap();
    let file = dir.path().join("arquivo.txt");
    std::fs::write(&file, "x").unwrap();
    let mut app = open_with(DataPaths::at(dir.path().join("data")), false);
    let missing = path_text(&dir.path().join("nao-existe"));

    err(
        &mut app,
        "emulators.folder.remove",
        Some(json!({"path": path_text(&roms)})),
        "not_found",
    );
    err(
        &mut app,
        "emulators.folder.system.set",
        Some(json!({"path": path_text(&roms), "system": "snes"})),
        "not_found",
    );

    for path in ["", "   ", "relativa/pasta", "a\u{0}b", missing.as_str()] {
        err(
            &mut app,
            "emulators.folder.add",
            Some(json!({"path": path})),
            "bad_request",
        );
    }
    // Um arquivo não é pasta.
    err(
        &mut app,
        "emulators.folder.add",
        Some(json!({"path": path_text(&file)})),
        "bad_request",
    );
    err(
        &mut app,
        "emulators.folder.add",
        Some(json!({"path": "x".repeat(5000)})),
        "bad_request",
    );
    // Campo desconhecido e payload ausente.
    err(
        &mut app,
        "emulators.folder.add",
        Some(json!({"path": path_text(&roms), "extra": 1})),
        "bad_request",
    );
    err(&mut app, "emulators.folder.add", None, "bad_request");

    ok(
        &mut app,
        "emulators.folder.add",
        Some(json!({"path": path_text(&roms)})),
    );
    for system in ["nao-existe", "", "SNES"] {
        err(
            &mut app,
            "emulators.folder.system.set",
            Some(json!({"path": path_text(&roms), "system": system})),
            "bad_request",
        );
    }
    // Nada do que foi recusado entrou na lista.
    let view = ok(&mut app, "emulators.list", None);
    assert_eq!(folders(&view), [(path_text(&roms), Value::Null)]);
}

#[test]
fn the_folder_list_has_a_ceiling_like_the_one_that_reads_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = open_with(DataPaths::at(dir.path().join("data")), true);
    for n in 0..256 {
        let path = path_text(&dir.path().join(format!("r{n}")));
        ok(
            &mut app,
            "emulators.folder.add",
            Some(json!({"path": path})),
        );
    }
    err(
        &mut app,
        "emulators.folder.add",
        Some(json!({"path": path_text(&dir.path().join("a-mais"))})),
        "conflict",
    );
    let view = ok(&mut app, "emulators.list", None);
    assert_eq!(folders(&view).len(), 256);
}

#[test]
fn changing_a_folder_reads_the_stores_again() {
    let dir = tempfile::tempdir().unwrap();
    let roms = dir.path().join("roms");
    std::fs::create_dir(&roms).unwrap();
    let reads = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let core = Core::open_with_sources(
        DataPaths::at(dir.path().join("data")),
        Platform::simulated(),
        false,
        vec![Box::new(Counting(std::sync::Arc::clone(&reads)))],
    )
    .unwrap();
    let mut app = AppService::with_core("test", core);
    settle(&mut app);
    let count = || reads.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(count(), 1);

    ok(
        &mut app,
        "emulators.folder.add",
        Some(json!({"path": path_text(&roms)})),
    );
    settle(&mut app);
    assert_eq!(count(), 2);
    ok(
        &mut app,
        "emulators.folder.system.set",
        Some(json!({"path": path_text(&roms), "system": "psx"})),
    );
    settle(&mut app);
    assert_eq!(count(), 3);
    // Pedir o mesmo sistema de novo não muda nada: não lê de novo. A pasta repetida também não.
    ok(
        &mut app,
        "emulators.folder.system.set",
        Some(json!({"path": path_text(&roms), "system": "psx"})),
    );
    settle(&mut app);
    assert_eq!(count(), 3);
    ok(
        &mut app,
        "emulators.folder.remove",
        Some(json!({"path": path_text(&roms)})),
    );
    settle(&mut app);
    assert_eq!(count(), 4);
}

#[test]
fn a_new_folder_reaches_the_next_read_without_reopening() {
    let dir = tempfile::tempdir().unwrap();
    let data = DataPaths::at(dir.path().join("data"));
    // Só o "Emuladores" lê; o DuckStation vem de um executável escolhido, não do PC de quem roda o teste.
    let others =
        "Steam,Epic,Gog,Xbox,Ubisoft,Ea,BattleNet,Amazon,Heroic,Lutris,Bottles,Itch,Manual";
    let exe = dir.path().join("duckstation-qt");
    std::fs::write(&exe, "").unwrap();
    let roms = dir.path().join("roms");
    std::fs::create_dir(&roms).unwrap();
    let rom = roms.join("Fixture.chd");
    std::fs::write(&rom, "rom fictícia").unwrap();
    {
        let store = SettingsStore::new(TakoDatabase::new(data.database()));
        store.set("DisabledStores", others).unwrap();
        store
            .set(
                "EmulatorPaths",
                &json!({"duckstation": path_text(&exe)}).to_string(),
            )
            .unwrap();
    }
    let core = Core::open(data, Platform::simulated(), false).unwrap();
    let mut app = AppService::with_core("test", core);
    settle(&mut app);
    let id = tako_sources::emulators::game_id(&rom);
    let known = |app: &mut AppService| {
        wire(app, "game.details", Some(json!({"gameId": id.clone()})))["ok"] == true
    };
    assert!(!known(&mut app));

    // O Slint lê "uma subpasta por sistema" quando a pasta não tem sistema: a ROM solta na raiz fica de fora.
    ok(
        &mut app,
        "emulators.folder.add",
        Some(json!({"path": path_text(&roms)})),
    );
    settle(&mut app);
    assert!(!known(&mut app));
    ok(
        &mut app,
        "emulators.folder.system.set",
        Some(json!({"path": path_text(&roms), "system": "psx"})),
    );
    settle(&mut app);
    assert!(known(&mut app));

    let view = ok(&mut app, "emulators.list", None);
    let duckstation = view["emulators"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["key"] == "duckstation")
        .unwrap();
    assert_eq!(duckstation["found"], true);
    assert_eq!(duckstation["chosen"], true);
    assert_eq!(duckstation["path"], path_text(&exe));

    ok(
        &mut app,
        "emulators.folder.remove",
        Some(json!({"path": path_text(&roms)})),
    );
    settle(&mut app);
    assert!(!known(&mut app));
}

#[test]
fn the_demonstration_never_looks_at_the_disk() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = open_with(DataPaths::at(dir.path().join("data")), true);
    let view = ok(&mut app, "emulators.list", None);
    assert!(
        view["emulators"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["found"] == false && e["path"].is_null())
    );
    // Pasta que não existe vale, e nada é gravado no disco.
    let fake = path_text(&dir.path().join("so-na-memoria"));
    let view = ok(
        &mut app,
        "emulators.folder.add",
        Some(json!({"path": fake})),
    );
    assert_eq!(folders(&view).len(), 1);
    assert!(!dir.path().join("data").exists());
}

#[test]
fn emulators_calls_need_the_core() {
    let mut app = AppService::new("test");
    err(&mut app, "emulators.list", None, "internal");
    err(
        &mut app,
        "emulators.folder.remove",
        Some(json!({"path": "/x"})),
        "internal",
    );
    let call = protocol::parse_request(r#"{"v":1,"id":1,"name":"emulators.list"}"#)
        .unwrap()
        .call;
    assert_eq!(call, Call::EmulatorsList);
}

#[test]
fn desktop_appearance_settings_use_the_slint_names_and_formats() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = open_with(DataPaths::at(dir.path().join("data")), false);
    let set = |app: &mut AppService, key: &str, value: &str| {
        wire(
            app,
            "settings.set",
            Some(json!({"key": key, "value": value})),
        )
    };
    // Os valores que o Slint grava.
    for (key, value) in [
        ("DesktopAccent", "#FF8A3D"),
        ("DesktopAccent", "#d8c8bc"),
        ("DesktopGlassOpacity", "0"),
        ("DesktopGlassOpacity", "100"),
        ("DesktopGlassAmbient", "false"),
        ("DesktopGlassPill", "true"),
    ] {
        let reply = set(&mut app, key, value);
        assert_eq!(reply["ok"], true, "{key}={value}: {reply}");
    }
    let got = ok(
        &mut app,
        "settings.get",
        Some(
            json!({"keys": ["DesktopAccent", "DesktopGlassOpacity", "DesktopGlassAmbient", "DesktopGlassPill"]}),
        ),
    );
    assert_eq!(got["values"]["DesktopAccent"], "#d8c8bc");
    assert_eq!(got["values"]["DesktopGlassOpacity"], "100");
    assert_eq!(got["values"]["DesktopGlassAmbient"], "false");
    assert_eq!(got["values"]["DesktopGlassPill"], "true");

    for (key, value) in [
        ("DesktopAccent", ""),
        ("DesktopAccent", "FF8A3D"),
        ("DesktopAccent", "#FF8A3"),
        ("DesktopAccent", "#GG8A3D"),
        ("DesktopAccent", "laranja"),
        ("DesktopAccent", "#FF8A3D; x"),
        ("DesktopGlassOpacity", "101"),
        ("DesktopGlassOpacity", "-1"),
        ("DesktopGlassOpacity", "meio"),
        ("DesktopGlassAmbient", "sim"),
        ("DesktopGlassPill", "1"),
    ] {
        let reply = set(&mut app, key, value);
        assert_eq!(reply["ok"], false, "{key}={value} devia falhar");
        assert_eq!(reply["error"]["code"], "bad_request");
    }
    // O recusado não mexeu no que estava gravado.
    let got = ok(
        &mut app,
        "settings.get",
        Some(json!({"keys": ["DesktopAccent"]})),
    );
    assert_eq!(got["values"]["DesktopAccent"], "#d8c8bc");
}
