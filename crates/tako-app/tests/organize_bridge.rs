//! Contrato de organização exercitado pelo mesmo parser JSON usado pelo hospedeiro.
#![allow(clippy::unwrap_used)]

use serde_json::{Value, json};
use tako_app::{
    AppService, Core,
    protocol::{self, Call, ErrorCode, Event, Response},
};
use tako_core::{
    paths::DataPaths,
    persistence::{SettingsStore, TakoDatabase},
};
use tako_platform::Platform;

fn open(dir: &tempfile::TempDir, simulate: bool) -> AppService {
    // Sem lojas, e esperando a leitura de abertura acabar: ela terminaria vazia por cima dos jogos de teste e
    // mandaria eventos no meio das asserções.
    let core = Core::open_with_sources(
        DataPaths::at(dir.path()),
        Platform::simulated(),
        simulate,
        Vec::new(),
    )
    .unwrap();
    let mut app = AppService::with_core("test", core);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while matches!(
        app.handle(tako_app::protocol::Call::LibrarySources),
        Ok(tako_app::protocol::Reply::LibrarySources(s)) if s.scanning
    ) {
        assert!(
            std::time::Instant::now() < deadline,
            "leitura de abertura não terminou"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    app
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
    serde_json::to_value(response).unwrap()
}
fn ok(app: &mut AppService, name: &str, payload: Option<Value>) -> Value {
    let reply = wire(app, name, payload);
    assert_eq!(reply["ok"], true, "{reply}");
    reply["reply"]["data"].clone()
}
fn err(app: &mut AppService, name: &str, payload: Value, code: &str) {
    assert_eq!(
        wire(app, name, Some(payload))["error"]["code"],
        code,
        "{name}"
    );
}

#[test]
fn settings_use_an_allowlist_and_never_return_legacy_secrets() {
    let dir = tempfile::tempdir().unwrap();
    let store = SettingsStore::new(TakoDatabase::new(DataPaths::at(dir.path()).database()));
    for key in [
        "LocalApiToken",
        "SteamApiKey",
        "SteamGridDbApiKey",
        "IgdbClientSecret",
        "UnknownFutureToken",
    ] {
        store.set(key, "SECRET_SENTINEL").unwrap();
    }
    let mut app = open(&dir, false);
    ok(
        &mut app,
        "settings.set",
        Some(json!({"key":"ReducedMotion","value":"true"})),
    );
    let all = ok(&mut app, "settings.get", Some(json!({})));
    assert_eq!(all, json!({"values":{"ReducedMotion":"true"}}));
    assert!(!all.to_string().contains("SECRET_SENTINEL"));
    for key in [
        "LocalApiToken",
        "SteamApiKey",
        "UnknownFutureToken",
        "SmartCollections",
    ] {
        err(
            &mut app,
            "settings.get",
            json!({"keys":[key]}),
            "bad_request",
        );
        err(
            &mut app,
            "settings.set",
            json!({"key":key,"value":"x"}),
            "bad_request",
        );
    }
    for (key, value) in [
        ("ReducedMotion", "maybe"),
        ("Language", "invalid"),
        ("ControlCenterChord", "-1"),
    ] {
        err(
            &mut app,
            "settings.set",
            json!({"key":key,"value":value}),
            "bad_request",
        );
    }
    drop(app);
    assert_eq!(
        ok(
            &mut open(&dir, false),
            "settings.get",
            Some(json!({"keys":["ReducedMotion"]}))
        ),
        all
    );
}

#[test]
fn organize_persists_and_emits_events_for_every_library_command() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = open(&dir, false);
    let fixture = vec![
        tako_core::library::Game {
            id: "steam:1".into(),
            title: "Um".into(),
            ..Default::default()
        },
        tako_core::library::Game {
            id: "gog:2".into(),
            title: "Dois".into(),
            ..Default::default()
        },
    ];
    app.reload_library(fixture.clone()).unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    app.set_event_sink(tx);
    let c = ok(
        &mut app,
        "collections.create",
        Some(json!({"name":"  Sofá  "})),
    );
    assert_eq!(rx.try_recv().unwrap(), Event::LibraryChanged);
    let id = c["id"].as_str().unwrap();
    for (name, payload) in [
        (
            "collections.rename",
            json!({"collectionId":id,"name":"Dupla"}),
        ),
        (
            "collections.add_games",
            json!({"collectionId":id,"gameIds":["steam:1","gog:2","steam:1"]}),
        ),
        (
            "collections.add_games",
            json!({"collectionId":id,"gameIds":["steam:1"]}),
        ),
        (
            "collections.remove_games",
            json!({"collectionId":id,"gameIds":["gog:2"]}),
        ),
        (
            "game.tags.set",
            json!({"gameIds":["steam:1","gog:2"],"tags":["Sofá","Coop"],"on":true}),
        ),
        (
            "game.tags.set",
            json!({"gameIds":["steam:1"],"tags":["sofa"],"on":true}),
        ),
        (
            "game.status.set",
            json!({"gameIds":["steam:1","gog:2"],"status":"playing"}),
        ),
        (
            "game.favorite.set",
            json!({"gameIds":["steam:1","gog:2","steam:1"],"favorite":true}),
        ),
    ] {
        ok(&mut app, name, Some(payload));
        assert_eq!(rx.try_recv().unwrap(), Event::LibraryChanged, "{name}");
    }
    drop(app);
    let mut app = open(&dir, false);
    app.reload_library(fixture).unwrap();
    let list = ok(&mut app, "collections.list", None);
    assert_eq!(list[0]["gameIds"], json!(["steam:1"]));
    assert_eq!(list[0]["name"], "Dupla");
    let tags = ok(&mut app, "tags.list", None);
    assert_eq!(
        tags,
        json!([{"name":"Coop","count":2},{"name":"Sofá","count":2}])
    );
    let page = ok(
        &mut app,
        "library.list",
        Some(json!({"filter":"favorites","statuses":["playing"]})),
    );
    assert_eq!(page["total"], 2);
    ok(
        &mut app,
        "game.status.set",
        Some(json!({"gameIds":["steam:1","gog:2"],"status":null})),
    );
    ok(
        &mut app,
        "game.favorite.set",
        Some(json!({"gameIds":["steam:1","gog:2"],"favorite":false})),
    );
    ok(
        &mut app,
        "game.tags.set",
        Some(json!({"gameIds":["steam:1","gog:2"],"tags":["Coop","Sofá"],"on":false})),
    );
    assert_eq!(ok(&mut app, "tags.list", None), json!([]));
    ok(
        &mut app,
        "collections.delete",
        Some(json!({"collectionId":id})),
    );
    assert_eq!(ok(&mut app, "collections.list", None), json!([]));
}

#[test]
fn smart_collections_resolve_saved_criteria_and_intersect_live_filters() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = open(&dir, true);
    let smart = ok(
        &mut app,
        "collections.smart.set",
        Some(
            json!({"name":"Steam RPG","criteria":{"library":{"stores":["steam"]},"genres":["RPG"],"playTime":"over_ten_hours"}}),
        ),
    );
    let id = &smart["id"];
    let page = ok(&mut app, "library.list", Some(json!({"collection":id})));
    assert_eq!(page["total"], 1);
    assert_eq!(page["rows"][0]["title"], "Celeste");
    assert_eq!(
        ok(
            &mut app,
            "library.list",
            Some(json!({"collection":id,"stores":["gog"]}))
        )["total"],
        0
    );
    ok(
        &mut app,
        "collections.smart.set",
        Some(json!({"collectionId":id,"name":"Epic","criteria":{"library":{"stores":["epic"]}}})),
    );
    assert_eq!(
        ok(&mut app, "library.list", Some(json!({"collection":id})))["total"],
        2
    );
    ok(
        &mut app,
        "collections.rename",
        Some(json!({"collectionId":id,"name":"Épicos"})),
    );
    ok(
        &mut app,
        "collections.delete",
        Some(json!({"collectionId":id})),
    );
    assert_eq!(
        ok(&mut app, "library.list", Some(json!({"collection":id})))["total"],
        0
    );
    assert!(!DataPaths::at(dir.path()).database().exists());
}

#[test]
fn smart_criteria_survive_database_reopening_and_language_changes() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = open(&dir, false);
    let c = ok(
        &mut app,
        "collections.smart.set",
        Some(json!({"name":"Steam","criteria":{"library":{"stores":["steam"]}}})),
    );
    ok(
        &mut app,
        "settings.set",
        Some(json!({"key":"Language","value":"en-US"})),
    );
    drop(app);
    let mut app = open(&dir, false);
    app.reload_library(vec![tako_core::library::Game {
        id: "steam:1".into(),
        store: Some(tako_core::library::GameStore::Steam),
        ..Default::default()
    }])
    .unwrap();
    assert_eq!(
        ok(
            &mut app,
            "library.list",
            Some(json!({"collection":c["id"]}))
        )["total"],
        1
    );
}

#[test]
fn missing_targets_and_malformed_payloads_do_not_mutate() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = open(&dir, true);
    let (tx, rx) = std::sync::mpsc::channel();
    app.set_event_sink(tx);
    for (name, payload) in [
        (
            "collections.rename",
            json!({"collectionId":"missing","name":"X"}),
        ),
        ("collections.delete", json!({"collectionId":"missing"})),
        (
            "collections.add_games",
            json!({"collectionId":"missing","gameIds":["steam:demo0"]}),
        ),
        (
            "collections.remove_games",
            json!({"collectionId":"missing","gameIds":["steam:demo0"]}),
        ),
        (
            "collections.smart.set",
            json!({"collectionId":"missing","name":"X","criteria":{}}),
        ),
        (
            "game.tags.set",
            json!({"gameIds":["steam:demo0","missing"],"tags":["X"],"on":true}),
        ),
        (
            "game.status.set",
            json!({"gameIds":["steam:demo0","missing"],"status":"playing"}),
        ),
        (
            "game.favorite.set",
            json!({"gameIds":["steam:demo0","missing"],"favorite":true}),
        ),
    ] {
        err(&mut app, name, payload, "not_found");
    }
    for (name, payload) in [
        ("collections.create", json!({"name":"x".repeat(41)})),
        ("collections.create", json!({"name":"   "})),
        (
            "game.tags.set",
            json!({"gameIds":["steam:demo0"],"tags":["x".repeat(41)],"on":true}),
        ),
        (
            "game.tags.set",
            json!({"gameIds":["steam:demo0"],"tags":vec!["x";65],"on":true}),
        ),
        (
            "game.favorite.set",
            json!({"gameIds":vec!["steam:demo0";201],"favorite":true}),
        ),
        ("game.favorite.set", json!({"gameIds":[],"favorite":true})),
        (
            "game.favorite.set",
            json!({"gameIds":["x".repeat(129)],"favorite":true}),
        ),
        (
            "game.status.set",
            json!({"gameIds":["steam:demo0"],"status":"future"}),
        ),
        (
            "settings.set",
            json!({"key":"Theme","value":"x".repeat(4097)}),
        ),
        ("settings.get", json!({"keys":vec!["Language";65]})),
        (
            "collections.smart.set",
            json!({"name":"X","criteria":{"genres":vec!["RPG";65]}}),
        ),
    ] {
        err(&mut app, name, payload, "bad_request");
    }
    for name in [
        "settings.get",
        "settings.set",
        "collections.create",
        "collections.rename",
        "collections.delete",
        "collections.add_games",
        "collections.remove_games",
        "collections.smart.set",
        "game.tags.set",
        "game.status.set",
        "game.favorite.set",
    ] {
        for payload in [
            json!([]),
            json!(null),
            json!("strange"),
            json!({"unknown":true}),
        ] {
            err(&mut app, name, payload, "bad_request");
        }
    }
    assert!(rx.try_recv().is_err());
    assert_eq!(ok(&mut app, "tags.list", None), json!([]));
    assert_eq!(
        app.handle(Call::GameFavoriteSet(protocol::GameFavoriteSet {
            game_ids: vec![],
            favorite: true
        }))
        .unwrap_err()
        .code,
        ErrorCode::BadRequest
    );
}

#[test]
fn print_real_demo_responses() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = open(&dir, true);
    ok(
        &mut app,
        "settings.set",
        Some(json!({"key":"ReducedMotion","value":"true"})),
    );
    ok(
        &mut app,
        "collections.create",
        Some(json!({"name":"Meus jogos"})),
    );
    println!(
        "collections.list: {}",
        wire(&mut app, "collections.list", None)
    );
    println!(
        "settings.get: {}",
        wire(&mut app, "settings.get", Some(json!({})))
    );
}

#[test]
fn exact_limits_are_accepted_and_manual_membership_filters_by_id() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = open(&dir, true);
    let c = ok(
        &mut app,
        "collections.create",
        Some(json!({"name":"é".repeat(40)})),
    );
    ok(
        &mut app,
        "collections.add_games",
        Some(json!({"collectionId":c["id"],"gameIds":vec!["steam:demo0";200]})),
    );
    assert_eq!(
        ok(
            &mut app,
            "library.list",
            Some(json!({"collection":c["id"]}))
        )["total"],
        1
    );
    ok(
        &mut app,
        "game.tags.set",
        Some(json!({"gameIds":vec!["steam:demo0";200],"tags":vec!["é".repeat(40);64],"on":true})),
    );
    ok(
        &mut app,
        "settings.set",
        Some(json!({"key":"GameScreenName","value":"x".repeat(4096)})),
    );
    ok(
        &mut app,
        "settings.get",
        Some(json!({"keys":vec!["GameScreenName";64]})),
    );
    err(
        &mut app,
        "collections.create",
        json!({"name":"É".repeat(40)}),
        "bad_request",
    );
}

#[test]
fn legacy_smart_criteria_round_trip_without_losing_core_filters() {
    use tako_core::library::{GameStore, LibraryCriteria, LibraryFilter, LibrarySort, smart};
    let dir = tempfile::tempdir().unwrap();
    let store = SettingsStore::new(TakoDatabase::new(DataPaths::at(dir.path()).database()));
    let saved = smart::SmartCollection {
        id: "legacy".into(),
        name: "Legada".into(),
        filter: LibraryFilter::Store(GameStore::Steam),
        criteria: LibraryCriteria {
            collections: ["a".into(), "b".into()].into(),
            favorites_only: true,
            played_only: true,
            broken_only: true,
            genres: ["RPG".into()].into(),
            sort: LibrarySort::Name,
            reversed: true,
            ..Default::default()
        },
    };
    store
        .set(
            "SmartCollections",
            &smart::to_json(std::slice::from_ref(&saved)),
        )
        .unwrap();
    let mut app = open(&dir, false);
    let list = ok(&mut app, "collections.list", None);
    ok(
        &mut app,
        "collections.smart.set",
        Some(json!({"collectionId":"legacy","name":"Legada","criteria":list[0]["criteria"]})),
    );
    let loaded = smart::parse(&store.get("SmartCollections").unwrap().unwrap());
    assert_eq!(loaded, vec![saved]);
}

#[test]
fn smart_filters_follow_game_changes_and_reject_nested_sequences() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = open(&dir, true);
    for criteria in [json!([]), json!({"library":[]}), json!({"library":null})] {
        err(
            &mut app,
            "collections.smart.set",
            json!({"name":"Inválida","criteria":criteria}),
            "bad_request",
        );
    }
    let c = ok(
        &mut app,
        "collections.smart.set",
        Some(
            json!({"name":"Em andamento","criteria":{"library":{"filter":"favorites","statuses":["playing"],"tags":["Sofá"],"sort":"name"}}}),
        ),
    );
    assert_eq!(
        ok(
            &mut app,
            "library.list",
            Some(json!({"collection":c["id"]}))
        )["total"],
        0
    );
    for (name, payload) in [
        (
            "game.favorite.set",
            json!({"gameIds":["steam:demo0","steam:demo1"],"favorite":true}),
        ),
        (
            "game.status.set",
            json!({"gameIds":["steam:demo0","steam:demo1"],"status":"playing"}),
        ),
        (
            "game.tags.set",
            json!({"gameIds":["steam:demo0","steam:demo1"],"tags":["Sofá"],"on":true}),
        ),
    ] {
        ok(&mut app, name, Some(payload));
    }
    let page = ok(
        &mut app,
        "library.list",
        Some(json!({"collection":c["id"]})),
    );
    assert_eq!(page["total"], 2);
    assert_eq!(page["rows"][0]["title"], "Celeste");
    ok(
        &mut app,
        "game.tags.set",
        Some(json!({"gameIds":["steam:demo1"],"tags":["sofa"],"on":false})),
    );
    assert_eq!(
        ok(
            &mut app,
            "library.list",
            Some(json!({"collection":c["id"]}))
        )["total"],
        1
    );
}

#[test]
fn a_storage_failure_is_reported_and_memory_changes_still_emit_an_event() {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path());
    let mut app = open(&dir, false);
    app.reload_library(vec![tako_core::library::Game {
        id: "steam:1".into(),
        ..Default::default()
    }])
    .unwrap();
    std::fs::rename(paths.database(), dir.path().join("saved.db")).unwrap();
    std::fs::create_dir(paths.database()).unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    app.set_event_sink(tx);
    err(
        &mut app,
        "settings.set",
        json!({"key":"ReducedMotion","value":"true"}),
        "internal",
    );
    assert_eq!(
        ok(&mut app, "settings.get", Some(json!({}))),
        json!({"values":{}})
    );
    assert!(rx.try_recv().is_err());
    err(
        &mut app,
        "game.favorite.set",
        json!({"gameIds":["steam:1"],"favorite":true}),
        "internal",
    );
    assert_eq!(rx.try_recv().unwrap(), Event::LibraryChanged);
}

#[test]
fn desktop_theme_is_the_slint_key_and_only_known_themes_are_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = open(&dir, false);
    let ok = wire(
        &mut app,
        "settings.set",
        Some(json!({"key":"DesktopTheme","value":"midnight"})),
    );
    assert_eq!(ok["ok"], true, "{ok}");
    let bad = wire(
        &mut app,
        "settings.set",
        Some(json!({"key":"DesktopTheme","value":"pink"})),
    );
    assert_eq!(bad["error"]["code"], "bad_request", "{bad}");
    let got = wire(
        &mut app,
        "settings.get",
        Some(json!({"keys":["DesktopTheme"]})),
    );
    assert_eq!(got["reply"]["data"]["values"]["DesktopTheme"], "midnight");
}
