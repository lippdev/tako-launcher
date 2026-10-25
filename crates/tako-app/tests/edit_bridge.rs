//! Editar jogo, adicionar jogo, remover e "Fontes e IDs" pelo mesmo parser JSON usado pelo hospedeiro.
#![allow(clippy::unwrap_used)]

use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};
use tako_app::protocol::{self, Event, Response};
use tako_app::{AppService, Core};
use tako_core::library::{Game, GameStore};
use tako_core::metadata::agent::MetadataAgent;
use tako_core::metadata::http::SystemClock;
use tako_core::metadata::{GameMetadata, MetadataLookup, MetadataMatch, MetadataProvider};
use tako_core::paths::DataPaths;
use tako_platform::Platform;

/// Uma fonte falsa: acha "Hades" e "Hades II" e só conhece o id 113112.
struct Fake;

impl MetadataProvider for Fake {
    fn name(&self) -> &str {
        "IGDB"
    }
    fn is_enabled(&self) -> bool {
        true
    }
    fn search(&self, _: &MetadataLookup) -> Vec<MetadataMatch> {
        vec![
            MetadataMatch {
                year: Some(2018),
                platforms: Some("PC".into()),
                ..MetadataMatch::new("IGDB", "113112", "Hades")
            },
            MetadataMatch::new("IGDB", "7", "Hades II"),
        ]
    }
    fn find_exact(&self, _: &MetadataLookup) -> Option<String> {
        None
    }
    fn fetch(&self, id: &str) -> Option<GameMetadata> {
        (id == "113112").then(|| GameMetadata {
            title: Some("Hades".into()),
            release_date: chrono::NaiveDate::from_ymd_opt(2018, 12, 6),
            ..GameMetadata::new("IGDB")
        })
    }
}

struct Harness {
    app: AppService,
    events: Receiver<Event>,
}

fn core(dir: &tempfile::TempDir) -> Core {
    Core::open_with_sources(
        DataPaths::at(dir.path()),
        Platform::simulated(),
        false,
        Vec::new(),
    )
    .unwrap()
}

fn open_with(dir: &tempfile::TempDir, core: Core) -> Harness {
    let _ = dir;
    let mut app = AppService::with_core("test", core);
    let (tx, events) = mpsc::channel();
    app.set_event_sink(tx);
    // A leitura de abertura (sem lojas) termina vazia; esperar evita eventos no meio das asserções.
    let deadline = Instant::now() + Duration::from_secs(10);
    while matches!(
        app.handle(protocol::Call::LibrarySources),
        Ok(protocol::Reply::LibrarySources(s)) if s.scanning
    ) {
        assert!(
            Instant::now() < deadline,
            "leitura de abertura não terminou"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    while events.try_recv().is_ok() {}
    Harness { app, events }
}

fn open(dir: &tempfile::TempDir) -> Harness {
    open_with(dir, core(dir))
}

fn open_with_agent(dir: &tempfile::TempDir) -> Harness {
    let mut core = core(dir);
    core.set_metadata_agent(Arc::new(MetadataAgent::new(
        vec![Arc::new(Fake)],
        Arc::new(SystemClock),
    )));
    open_with(dir, core)
}

impl Harness {
    fn wire(&mut self, name: &str, payload: Option<Value>) -> Value {
        let mut value = json!({"v":1,"id":7,"name":name});
        if let Some(payload) = payload {
            value["payload"] = payload;
        }
        let response = match protocol::parse_request(&value.to_string()) {
            Ok(request) => match self.app.handle(request.call) {
                Ok(reply) => Response::ok(request.id, reply),
                Err(error) => Response::err(request.id, error),
            },
            Err((id, error)) => Response::err(id, error),
        };
        serde_json::to_value(response).unwrap()
    }

    fn ok(&mut self, name: &str, payload: Value) -> Value {
        let reply = self.wire(name, Some(payload));
        assert_eq!(reply["ok"], true, "{name}: {reply}");
        reply["reply"]["data"].clone()
    }

    fn err(&mut self, name: &str, payload: Value, code: &str) -> String {
        let reply = self.wire(name, Some(payload));
        assert_eq!(reply["error"]["code"], code, "{name}: {reply}");
        reply["error"]["message"].as_str().unwrap().to_owned()
    }

    fn changed(&self) -> usize {
        self.events
            .try_iter()
            .filter(|e| *e == Event::LibraryChanged)
            .count()
    }

    /// Chama `poll` até chegar o evento pedido (as buscas rodam em thread).
    fn wait_event(&mut self, wanted: impl Fn(&Event) -> bool) -> Event {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            self.app.poll();
            if let Some(event) = self.events.try_iter().find(|e| wanted(e)) {
                return event;
            }
            assert!(Instant::now() < deadline, "evento não chegou");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn add(&mut self, path: &std::path::Path) -> String {
        let data = self.ok("library.add_game", json!({"path": path.to_string_lossy()}));
        assert_eq!(data["status"], "added", "{data}");
        data["gameId"].as_str().unwrap().to_owned()
    }
}

fn file(dir: &tempfile::TempDir, name: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, b"x").unwrap();
    path
}

fn store_game(id: &str) -> Game {
    Game {
        id: id.into(),
        title: "Da loja".into(),
        store: Some(GameStore::Steam),
        ..Game::default()
    }
}

#[test]
fn adding_a_program_guesses_the_title_once_and_announces_the_change() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = open(&dir);
    let exe = file(&dir, "hollow_knight.exe");

    let data = h.ok("library.add_game", json!({"path": exe.to_string_lossy()}));
    assert_eq!(data["status"], "added");
    assert_eq!(data["title"], "hollow knight");
    assert!(data["gameId"].as_str().unwrap().starts_with("manual:"));
    assert_eq!(h.changed(), 1);

    let list = h.ok("library.list", json!({}));
    assert_eq!(list["total"], 1);
    assert_eq!(list["rows"][0]["title"], "hollow knight");
    assert_eq!(list["rows"][0]["store"], "manual");

    // O mesmo arquivo de novo: já está lá, nada muda.
    let same = h.ok("library.add_game", json!({"path": exe.to_string_lossy()}));
    assert_eq!(same["status"], "already");
    assert_eq!(same["title"], "hollow knight");
    assert_eq!(h.changed(), 0);
    assert_eq!(h.ok("library.list", json!({}))["total"], 1);
}

#[test]
fn adding_rejects_bad_paths_and_systems() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = open(&dir);
    for path in ["", "   ", "a\u{0}b"] {
        h.err("library.add_game", json!({"path": path}), "bad_request");
    }
    h.err(
        "library.add_game",
        json!({"path": "a".repeat(2000)}),
        "bad_request",
    );
    h.err(
        "library.add_game",
        json!({"path": dir.path().join("nao-existe.exe").to_string_lossy()}),
        "not_found",
    );
    // Uma pasta qualquer não é um jogo.
    h.err(
        "library.add_game",
        json!({"path": dir.path().to_string_lossy()}),
        "not_found",
    );
    let exe = file(&dir, "jogo.exe");
    h.err(
        "library.add_game",
        json!({"path": exe.to_string_lossy(), "system": "snes"}),
        "bad_request",
    );
    h.err(
        "library.add_game",
        json!({"path": exe.to_string_lossy(), "extra": 1}),
        "bad_request",
    );
    assert_eq!(h.ok("library.list", json!({}))["total"], 0);
    assert_eq!(h.changed(), 0);
}

#[test]
fn roms_ask_for_the_system_and_then_are_added_with_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = open(&dir);
    let rom = file(&dir, "Super Mario World (USA).sfc");

    let ask = h.ok("library.add_game", json!({"path": rom.to_string_lossy()}));
    assert_eq!(ask["status"], "needs_system");
    assert_eq!(ask["fileName"], "Super Mario World (USA).sfc");
    assert_eq!(ask["systems"][0]["key"], "snes");
    assert_eq!(ask["systems"][0]["name"], "SNES");
    // "Outro sistema": os que não são prováveis, sem repetir os de cima.
    let others = ask["others"].as_array().unwrap();
    assert!(!others.is_empty());
    assert!(!others.iter().any(|s| s["key"] == "snes"));
    assert_eq!(h.ok("library.list", json!({}))["total"], 0);
    assert_eq!(h.changed(), 0);

    h.err(
        "library.add_game",
        json!({"path": rom.to_string_lossy(), "system": "naoexiste"}),
        "bad_request",
    );
    let done = h.ok(
        "library.add_game",
        json!({"path": rom.to_string_lossy(), "system": "snes"}),
    );
    assert_eq!(done["status"], "added");
    assert_eq!(done["title"], "Super Mario World");
    assert_eq!(h.changed(), 1);
}

#[test]
fn editing_saves_through_the_core_and_validates() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = open(&dir);
    let exe = file(&dir, "jogo.exe");
    let id = h.add(&exe);
    h.changed();

    let view = h.ok("game.edit", json!({"gameId": id}));
    assert_eq!(view["title"], "jogo");
    assert_eq!(view["target"], exe.to_string_lossy().as_ref());
    assert_eq!(view["arguments"], "");
    assert_eq!(view["tracking"], "auto");
    assert_eq!(view["defaultFolder"], dir.path().to_string_lossy().as_ref());

    let saved = h.ok(
        "game.edit.set",
        json!({
            "gameId": id,
            "title": "  Meu Jogo ",
            "arguments": " -windowed ",
            "workingDirectory": "",
            "tracking": "process",
        }),
    );
    assert_eq!(saved["title"], "Meu Jogo");
    assert_eq!(saved["arguments"], "-windowed");
    assert_eq!(saved["tracking"], "process");
    assert_eq!(
        saved["workingDirectory"],
        dir.path().to_string_lossy().as_ref()
    );
    assert_eq!(h.changed(), 1);
    assert_eq!(
        h.ok("library.list", json!({}))["rows"][0]["title"],
        "Meu Jogo"
    );
    assert_eq!(h.ok("game.edit", json!({"gameId": id})), saved);

    // Entrada inválida não grava nada.
    let base = |title: &str, folder: &str| json!({"gameId": id, "title": title, "arguments": "", "workingDirectory": folder, "tracking": "auto"});
    h.err("game.edit.set", base("  ", ""), "bad_request");
    h.err("game.edit.set", base(&"a".repeat(201), ""), "bad_request");
    h.err("game.edit.set", base("a\nb", ""), "bad_request");
    let missing = dir.path().join("nao-existe");
    let message = h.err(
        "game.edit.set",
        base("X", &missing.to_string_lossy()),
        "not_found",
    );
    assert!(message.contains("nao-existe"), "{message}");
    h.err(
        "game.edit.set",
        json!({"gameId": id, "title": "X", "arguments": "", "workingDirectory": "", "tracking": "outro"}),
        "bad_request",
    );
    h.err(
        "game.edit.set",
        json!({"gameId": id, "title": "X", "arguments": "x".repeat(2049), "workingDirectory": "", "tracking": "auto"}),
        "bad_request",
    );
    assert_eq!(h.changed(), 0);
    assert_eq!(
        h.ok("game.edit", json!({"gameId": id}))["title"],
        "Meu Jogo"
    );

    // Uma pasta que existe vale.
    let folder = dir.path().join("pasta");
    std::fs::create_dir(&folder).unwrap();
    let with_folder = h.ok("game.edit.set", base("Meu Jogo", &folder.to_string_lossy()));
    assert_eq!(
        with_folder["workingDirectory"],
        folder.to_string_lossy().as_ref()
    );
}

#[test]
fn editing_and_removing_need_a_manual_game() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = open(&dir);
    h.app.reload_library(vec![store_game("steam:1")]).unwrap();
    h.changed();
    let edit = json!({"gameId": "steam:1", "title": "X", "arguments": "", "workingDirectory": "", "tracking": "auto"});

    h.err("game.edit", json!({"gameId": "steam:1"}), "conflict");
    h.err("game.edit.set", edit, "conflict");
    h.err("game.remove", json!({"gameId": "steam:1"}), "conflict");
    h.err(
        "game.edit",
        json!({"gameId": "manual:nao-existe"}),
        "not_found",
    );
    h.err(
        "game.remove",
        json!({"gameId": "manual:nao-existe"}),
        "not_found",
    );
    h.err("game.edit", json!({"gameId": ""}), "bad_request");
    assert_eq!(h.changed(), 0);
    assert_eq!(h.ok("library.list", json!({}))["total"], 1);
}

#[test]
fn removing_takes_the_game_out_and_keeps_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = open(&dir);
    let exe = file(&dir, "jogo.exe");
    let id = h.add(&exe);
    h.ok(
        "game.favorite.set",
        json!({"gameIds": [id], "favorite": true}),
    );
    h.changed();

    h.ok("game.remove", json!({"gameId": id}));
    assert_eq!(h.changed(), 1);
    assert_eq!(h.ok("library.list", json!({}))["total"], 0);
    assert!(exe.exists());
    h.err("game.remove", json!({"gameId": id}), "not_found");
    h.err("game.details", json!({"gameId": id}), "not_found");
}

#[test]
fn edits_and_additions_survive_reopening() {
    let dir = tempfile::tempdir().unwrap();
    let id = {
        let mut h = open(&dir);
        let id = h.add(&file(&dir, "jogo.exe"));
        h.ok(
            "game.edit.set",
            json!({"gameId": id, "title": "Salvo", "arguments": "-x", "workingDirectory": "", "tracking": "none"}),
        );
        id
    };
    let mut h = open(&dir);
    let view = h.ok("game.edit", json!({"gameId": id}));
    assert_eq!(view["title"], "Salvo");
    assert_eq!(view["arguments"], "-x");
    assert_eq!(view["tracking"], "none");
}

#[test]
fn source_ids_list_pin_and_unpin_each_source() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = open_with_agent(&dir);
    let id = h.add(&file(&dir, "Hades.exe"));

    let view = h.ok("game.source_ids", json!({"gameId": id}));
    assert_eq!(view["title"], "Hades");
    let sources = view["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0]["source"], "IGDB");
    assert_eq!(sources[0]["id"], Value::Null);
    assert_eq!(sources[0]["pinned"], false);
    assert_eq!(sources[0]["none"], false);
    assert!(!sources[0]["example"].as_str().unwrap().is_empty());

    let set = |h: &mut Harness, choice: Value| {
        h.ok(
            "game.source_ids.set",
            json!({"gameId": id, "source": "IGDB", "choice": choice}),
        )["sources"][0]
            .clone()
    };
    let pinned = set(&mut h, json!({"kind": "id", "value": " 113112 "}));
    assert_eq!(pinned["id"], "113112");
    assert_eq!(pinned["pinned"], true);
    let none = set(&mut h, json!({"kind": "none"}));
    assert_eq!(
        (
            none["id"].clone(),
            none["none"].clone(),
            none["pinned"].clone()
        ),
        (Value::Null, json!(true), json!(true))
    );
    let auto = set(&mut h, json!({"kind": "auto"}));
    assert_eq!(
        (auto["pinned"].clone(), auto["none"].clone()),
        (json!(false), json!(false))
    );

    // O id fixado vale depois de reabrir (banco).
    set(&mut h, json!({"kind": "id", "value": "7"}));
    drop(h);
    let mut h = open_with_agent(&dir);
    let again = h.ok("game.source_ids", json!({"gameId": id}));
    assert_eq!(again["sources"][0]["id"], "7");
    assert_eq!(again["sources"][0]["pinned"], true);
    assert_eq!(h.changed(), 0);
}

#[test]
fn source_ids_reject_bad_input() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = open_with_agent(&dir);
    let id = h.add(&file(&dir, "Hades.exe"));
    let set =
        |source: &str, choice: Value| json!({"gameId": id, "source": source, "choice": choice});
    h.err(
        "game.source_ids.set",
        set("Outra", json!({"kind": "none"})),
        "bad_request",
    );
    h.err(
        "game.source_ids.set",
        set("IGDB", json!({"kind": "id", "value": "  "})),
        "bad_request",
    );
    h.err(
        "game.source_ids.set",
        set("IGDB", json!({"kind": "id", "value": "1".repeat(129)})),
        "bad_request",
    );
    h.err(
        "game.source_ids.set",
        set("IGDB", json!({"kind": "talvez"})),
        "bad_request",
    );
    h.err(
        "game.source_ids.set",
        json!({"gameId": "manual:nada", "source": "IGDB", "choice": {"kind": "auto"}}),
        "not_found",
    );
    h.err(
        "game.source_ids",
        json!({"gameId": "manual:nada"}),
        "not_found",
    );
    h.err(
        "game.source_ids.search",
        json!({"gameId": id, "term": "  "}),
        "bad_request",
    );
    h.err(
        "game.source_ids.search",
        json!({"gameId": id, "term": "x".repeat(201)}),
        "bad_request",
    );
    h.err(
        "game.source_ids.search",
        json!({"gameId": "manual:nada", "term": "a"}),
        "not_found",
    );
    h.err(
        "game.source_ids.check",
        json!({"gameId": id, "source": "Outra", "text": "1"}),
        "bad_request",
    );
    let problem = h.err(
        "game.source_ids.check",
        json!({"gameId": id, "source": "IGDB", "text": "isso não é um id"}),
        "bad_request",
    );
    assert!(!problem.is_empty());
    let after = h.ok("game.source_ids", json!({"gameId": id}));
    assert_eq!(after["sources"][0]["pinned"], false);
}

#[test]
fn search_and_check_answer_later_by_event() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = open_with_agent(&dir);
    let id = h.add(&file(&dir, "Hades.exe"));

    let job = h.ok(
        "game.source_ids.search",
        json!({"gameId": id, "term": " Hades "}),
    );
    let request_id = job["requestId"].as_u64().unwrap() as u32;
    let Event::GameSourceIdsFound(found) =
        h.wait_event(|e| matches!(e, Event::GameSourceIdsFound(_)))
    else {
        unreachable!()
    };
    assert_eq!(found.request_id, request_id);
    assert_eq!(found.game_id, id);
    assert_eq!(found.term, "Hades");
    assert_eq!(found.results.len(), 1);
    let igdb = &found.results[0];
    assert_eq!(igdb.source, "IGDB");
    assert_eq!(igdb.best.as_deref(), Some("113112"));
    assert_eq!(igdb.matches[0].title, "Hades");
    assert_eq!(igdb.matches[0].year, Some(2018));
    assert_eq!(igdb.matches[0].platforms.as_deref(), Some("PC"));
    assert_eq!(igdb.matches[1].id, "7");

    // Conferir um id: a fonte devolve o nome e o ano; nada é fixado até o usuário confirmar.
    let job = h.ok(
        "game.source_ids.check",
        json!({"gameId": id, "source": "IGDB", "text": "113112"}),
    );
    let Event::GameSourceIdsChecked(checked) =
        h.wait_event(|e| matches!(e, Event::GameSourceIdsChecked(_)))
    else {
        unreachable!()
    };
    assert_eq!(
        checked.request_id,
        job["requestId"].as_u64().unwrap() as u32
    );
    let found = checked.found.unwrap();
    assert_eq!(
        (found.id.as_str(), found.name.as_str()),
        ("113112", "Hades (2018)")
    );
    assert_eq!(
        h.ok("game.source_ids", json!({"gameId": id}))["sources"][0]["pinned"],
        false
    );

    // Id que a fonte não conhece.
    h.ok(
        "game.source_ids.check",
        json!({"gameId": id, "source": "IGDB", "text": "99"}),
    );
    let Event::GameSourceIdsChecked(unknown) =
        h.wait_event(|e| matches!(e, Event::GameSourceIdsChecked(_)))
    else {
        unreachable!()
    };
    assert_eq!(unknown.found, None);
}

#[test]
fn without_sources_the_list_is_empty_and_searches_find_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = open(&dir);
    // Sem fontes ligadas (nenhuma chave): a IGDB e as outras não atendem.
    let id = h.add(&file(&dir, "Hades.exe"));
    let view = h.ok("game.source_ids", json!({"gameId": id}));
    assert!(
        view["sources"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["source"] != "IGDB")
    );

    let dir2 = tempfile::tempdir().unwrap();
    let simulated = Core::open_with_sources(
        DataPaths::at(dir2.path()),
        Platform::simulated(),
        true,
        Vec::new(),
    )
    .unwrap();
    let mut h = open_with(&dir2, simulated);
    let id = h.add(&file(&dir2, "Hades.exe"));
    assert_eq!(
        h.ok("game.source_ids", json!({"gameId": id}))["sources"],
        json!([])
    );
    h.ok(
        "game.source_ids.search",
        json!({"gameId": id, "term": "Hades"}),
    );
    let Event::GameSourceIdsFound(found) =
        h.wait_event(|e| matches!(e, Event::GameSourceIdsFound(_)))
    else {
        unreachable!()
    };
    assert!(found.results.is_empty());
    h.err(
        "game.source_ids.set",
        json!({"gameId": id, "source": "IGDB", "choice": {"kind": "none"}}),
        "bad_request",
    );
}

#[test]
fn new_calls_reject_a_payload_that_is_not_an_object() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = open(&dir);
    for name in [
        "game.edit",
        "game.edit.set",
        "game.remove",
        "library.add_game",
        "game.source_ids",
        "game.source_ids.set",
        "game.source_ids.search",
        "game.source_ids.check",
    ] {
        let reply = h.wire(name, Some(json!([])));
        assert_eq!(reply["error"]["code"], "bad_request", "{name}");
        let reply = h.wire(name, None);
        assert_eq!(reply["error"]["code"], "bad_request", "{name} sem payload");
    }
}
