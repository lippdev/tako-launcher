//! Exercita o protocolo e os serviços da tako-app sem importar o hospedeiro.

#![allow(clippy::unwrap_used)]

use chrono::{TimeDelta, Utc};
use serde_json::{Value, json};
use std::sync::mpsc;
use std::time::Duration;
use tako_app::protocol::{
    Call, ErrorCode, Event, GameRequest, LibraryListRequest, LibraryPage, LibraryTab,
    MAX_PAGE_LIMIT, Reply, Response, SortOrder, Store,
};
use tako_app::{AppService, Core, HostRequest};
use tako_core::i18n::Lang;
use tako_core::library::{Game, GameStore, PlaySession, PlayStatus};
use tako_core::paths::DataPaths;
use tako_core::persistence::{self as db, TakoDatabase};
use tako_platform::Platform;

fn game_request(id: &str) -> GameRequest {
    GameRequest { game_id: id.into() }
}

fn demo() -> (tempfile::TempDir, AppService) {
    let dir = tempfile::tempdir().unwrap();
    let core = Core::open(DataPaths::at(dir.path()), Platform::simulated(), true).unwrap();
    (dir, AppService::with_core("test", core))
}

fn list(app: &mut AppService, request: LibraryListRequest) -> LibraryPage {
    match app.handle(Call::LibraryList(request)).unwrap() {
        Reply::LibraryList(page) => page,
        other => panic!("resposta inesperada: {other:?}"),
    }
}

fn wire(app: &mut AppService, request: &str) -> Value {
    let response = match tako_app::protocol::parse_request(request) {
        Ok(request) => match app.handle(request.call) {
            Ok(reply) => Response::ok(request.id, reply),
            Err(error) => Response::err(request.id, error),
        },
        Err((id, error)) => Response::err(id, error),
    };
    let text = serde_json::to_string(&response).unwrap();
    // Confere também que a resposta pode voltar ao mesmo protocolo tipado.
    let response: Response = serde_json::from_str(&text).unwrap();
    assert_eq!(
        serde_json::to_value(response).unwrap(),
        serde_json::from_str::<Value>(&text).unwrap()
    );
    serde_json::from_str(&text).unwrap()
}

fn fixture_games() -> Vec<Game> {
    [
        ("steam:570", "Árvore", GameStore::Steam, true, 60),
        (
            "heroic:legendary:Fortnite",
            "Fortnite",
            GameStore::Heroic,
            false,
            120,
        ),
        ("gog:3", "Zelda", GameStore::Gog, true, 30),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (id, title, store, installed, seconds))| Game {
        id: id.into(),
        title: title.into(),
        store: Some(store),
        is_installed: installed,
        is_favorite: i == 0,
        tags: ["Sofá".into()].into(),
        play_status: Some(PlayStatus::Playing),
        play_time: Duration::from_secs(seconds),
        cover_uri: (i == 0).then(|| "file:///private/cover.png".into()),
        ..Default::default()
    })
    .collect()
}

#[test]
fn opens_real_paths_with_snapshot_settings_manual_games_state_and_collections() {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path());
    std::fs::create_dir_all(paths.cache()).unwrap();
    std::fs::write(
        paths.cache().join("found-games.json"),
        serde_json::to_vec(&fixture_games()).unwrap(),
    )
    .unwrap();
    let database = TakoDatabase::new(paths.database());
    db::SettingsStore::new(database.clone())
        .set("Language", "en-US")
        .unwrap();
    let store = db::LibraryStore::new(database.clone(), None);
    let mut data = store.load().unwrap();
    data.manual.push(db::ManualGameRow {
        id: "manual:test".into(),
        title: "Manual".into(),
        path: "C:/Games/test.exe".into(),
        ..Default::default()
    });
    let now = Utc::now();
    data.state.insert(
        "steam:570".into(),
        db::GameStateRow {
            is_favorite: true,
            play_status: Some(PlayStatus::Beaten),
            tags: ["Relaxar".into()].into(),
            last_played_at: Some(now),
            sessions: (0..12)
                .map(|i| PlaySession {
                    started_at: now - TimeDelta::days(i),
                    duration: Duration::from_millis(60_001 * (i as u64 + 1)),
                })
                .collect(),
            ..Default::default()
        },
    );
    store.save(&data).unwrap();
    let collections = db::CollectionStore::new(database);
    collections.insert("sofa", "Sofá", now).unwrap();
    collections.add_game("sofa", "steam:570", now).unwrap();
    let core = Core::open(paths.clone(), Platform::simulated(), false).unwrap();
    assert!(paths.database().is_file());
    assert_eq!(core.language(), Lang::EnUs);
    assert_eq!(core.setting("Language"), Some("en-US"));
    assert!(core.library().find("manual:test").is_some());
    let mut app = AppService::with_core("1", core);
    let page = list(
        &mut app,
        LibraryListRequest {
            collection: Some("sofa".into()),
            tags: vec!["relaxar".into()],
            statuses: vec![tako_app::protocol::GameStatus::Beaten],
            stores: vec![Store::Steam],
            installed: Some(true),
            ..Default::default()
        },
    );
    assert_eq!(page.total, 1);
    assert_eq!(page.rows[0].id, "steam:570");
    assert!(page.rows[0].favorite);
    let Reply::GameDetails(details) = app
        .handle(Call::GameDetails(game_request("steam:570")))
        .unwrap()
    else {
        panic!("detalhes esperados");
    };
    let sessions = details.recent_sessions.unwrap();
    assert_eq!(sessions.len(), 10);
    assert!(
        sessions
            .windows(2)
            .all(|w| w[0].started_at > w[1].started_at)
    );
    assert_eq!(details.game.play_time_seconds, 60); // A Steam já soma o tempo.
    assert_eq!(details.tags, Some(vec!["Relaxar".into()]));
    let expected_date = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    assert_eq!(
        details.game.last_played_at.as_deref(),
        Some(expected_date.as_str())
    );
    assert_eq!(sessions[0].started_at, expected_date);
    assert_eq!(sessions[0].duration_seconds, 60);
}

#[test]
fn simulate_does_not_touch_existing_database_snapshot_or_alpha_secrets() {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path());
    std::fs::write(paths.database(), "banco preservado").unwrap();
    std::fs::write(
        paths.machine.join("igdb.dat"),
        r#"{"Enabled":true,"ClientId":"ficticio"}"#,
    )
    .unwrap();
    std::fs::create_dir_all(paths.cache()).unwrap();
    std::fs::write(
        paths.cache().join("found-games.json"),
        "snapshot preservado",
    )
    .unwrap();
    let core = Core::open(paths.clone(), Platform::simulated(), true).unwrap();
    assert_eq!(core.library().games().len(), 12);
    assert!(core.setting("Language").is_none());
    let mut app = AppService::with_core("1", core);
    app.reload_library(fixture_games()).unwrap();
    assert_eq!(
        std::fs::read_to_string(paths.database()).unwrap(),
        "banco preservado"
    );
    assert_eq!(
        std::fs::read_to_string(paths.cache().join("found-games.json")).unwrap(),
        "snapshot preservado"
    );
    assert!(paths.machine.join("igdb.dat").exists());
    assert!(!paths.machine.join("igdb.dat.migrado").exists());
}

#[test]
fn real_boot_migrates_alpha_secrets_into_the_supplied_vault() {
    use tako_core::metadata::{SecretStore, igdb::IgdbSettings};
    struct Vault<'a>(&'a dyn tako_platform::secrets::Secrets);
    impl SecretStore for Vault<'_> {
        fn get(&self, key: &str) -> Option<String> {
            self.0.get(key)
        }
        fn set(&self, key: &str, value: &str) -> Result<(), String> {
            self.0.set(key, value)
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path());
    std::fs::write(
        paths.machine.join("igdb.dat"),
        r#"{"Enabled":true,"ClientId":"ficticio","ClientSecret":"teste"}"#,
    )
    .unwrap();
    let platform = Platform::simulated();
    let secrets = platform.secrets.clone();
    Core::open(paths.clone(), platform, false).unwrap();
    let migrated = IgdbSettings::load(&Vault(secrets.as_ref()));
    assert_eq!(migrated.client_id, "ficticio");
    assert!(migrated.enabled);
    assert!(paths.machine.join("igdb.dat.migrado").exists());
    assert!(!paths.machine.join("igdb.dat").exists());
}

#[test]
fn library_list_json_round_trip_and_demo_examples() {
    let (_dir, mut app) = demo();
    let page = wire(
        &mut app,
        r#"{"v":1,"id":11,"name":"library.list","payload":{"sort":"name","limit":1}}"#,
    );
    assert_eq!(page["id"], 11);
    assert_eq!(page["reply"]["name"], "library.list");
    assert_eq!(page["reply"]["data"]["total"], 12);
    assert_eq!(page["reply"]["data"]["rows"][0]["title"], "Balatro");
    println!("library.list = {}", serde_json::to_string(&page).unwrap());
    let details = wire(
        &mut app,
        r#"{"v":1,"id":12,"name":"game.details","payload":{"gameId":"steam:demo0"}}"#,
    );
    assert_eq!(details["reply"]["data"]["game"]["title"], "Hollow Knight");
    assert_eq!(details["reply"]["data"]["developer"], "Estúdio");
    assert!(details["reply"]["data"]["publisher"].is_null());
    println!(
        "game.details = {}",
        serde_json::to_string(&details).unwrap()
    );
}

#[test]
fn core_filters_accent_insensitive_search_and_sorting_apply_before_pagination() {
    let (_dir, mut app) = demo();
    app.reload_library(fixture_games()).unwrap();
    let page = list(
        &mut app,
        LibraryListRequest {
            search: "ARVORE".into(),
            ..Default::default()
        },
    );
    assert_eq!(page.total, 1);
    assert_eq!(page.rows[0].title, "Árvore");
    let page = list(
        &mut app,
        LibraryListRequest {
            filter: LibraryTab::Favorites,
            ..Default::default()
        },
    );
    assert_eq!(page.total, 1);
    let page = list(
        &mut app,
        LibraryListRequest {
            filter: LibraryTab::Installed,
            stores: vec![Store::Heroic],
            ..Default::default()
        },
    );
    assert_eq!(page.total, 0);
    let page = list(
        &mut app,
        LibraryListRequest {
            installed: Some(false),
            ..Default::default()
        },
    );
    assert_eq!(page.rows[0].id, "heroic:legendary:Fortnite");
    for (sort, reversed, first) in [
        (SortOrder::Name, false, "Árvore"),
        (SortOrder::Name, true, "Zelda"),
        (SortOrder::PlayTime, false, "Fortnite"),
        (SortOrder::Size, false, "Árvore"),
    ] {
        let page = list(
            &mut app,
            LibraryListRequest {
                sort,
                reversed,
                limit: 1,
                ..Default::default()
            },
        );
        assert_eq!(page.total, 3);
        assert_eq!(page.rows[0].title, first);
    }
    let page = list(
        &mut app,
        LibraryListRequest {
            sort: SortOrder::Name,
            offset: 1,
            limit: 1,
            ..Default::default()
        },
    );
    assert_eq!(page.rows[0].title, "Fortnite");
    assert_eq!(page.total, 3);
    let page = list(
        &mut app,
        LibraryListRequest {
            tags: vec!["sofá".into()],
            statuses: vec![tako_app::protocol::GameStatus::Playing],
            ..Default::default()
        },
    );
    assert_eq!(page.total, 3);
    let page = list(
        &mut app,
        LibraryListRequest {
            no_status: true,
            ..Default::default()
        },
    );
    assert_eq!(page.total, 0);
    let page = list(
        &mut app,
        LibraryListRequest {
            tags: vec!["ausente".into()],
            ..Default::default()
        },
    );
    assert_eq!(page.total, 0);
}

#[test]
fn pagination_caps_large_lists_and_handles_zero_and_past_end() {
    let (_dir, mut app) = demo();
    app.reload_library(
        (0..MAX_PAGE_LIMIT + 15)
            .map(|i| Game {
                id: format!("steam:{i}"),
                title: format!("Jogo {i:04}"),
                ..Default::default()
            })
            .collect(),
    )
    .unwrap();
    let page = list(
        &mut app,
        LibraryListRequest {
            limit: u32::MAX,
            ..Default::default()
        },
    );
    assert_eq!(page.rows.len(), MAX_PAGE_LIMIT as usize);
    assert_eq!(page.total, MAX_PAGE_LIMIT as usize + 15);
    assert_eq!(list(&mut app, Default::default()).rows.len(), 50);
    for (offset, limit) in [(0, 0), (u32::MAX, 1), (MAX_PAGE_LIMIT + 15, 10)] {
        let page = list(
            &mut app,
            LibraryListRequest {
                offset,
                limit,
                ..Default::default()
            },
        );
        assert!(page.rows.is_empty());
        assert_eq!(page.total, MAX_PAGE_LIMIT as usize + 15);
    }
}

#[test]
fn game_details_nulls_absent_data_and_does_not_expose_source_uris_or_targets() {
    let (_dir, mut app) = demo();
    app.reload_library(fixture_games()).unwrap();
    let response = wire(
        &mut app,
        r#"{"v":1,"id":1,"name":"game.details","payload":{"gameId":"steam:570"}}"#,
    );
    assert!(response["ok"].as_bool().unwrap());
    let data = &response["reply"]["data"];
    for key in [
        "description",
        "developer",
        "publisher",
        "releaseDate",
        "genres",
        "metacriticScore",
        "criticScore",
        "recentSessions",
        "sizeBytes",
        "sizeLabel",
        "platform",
    ] {
        assert!(data[key].is_null(), "{key}");
    }
    assert_eq!(data["game"]["hasCover"], true);
    assert_eq!(data["canPlay"], true);
    assert_eq!(data["upcoming"], false);
    assert_eq!(data["game"]["hasHero"], false);
    assert!(data["game"]["accent"].is_null());
    assert!(!response.to_string().contains("private"));
}

#[test]
fn details_and_all_sort_orders_use_the_values_from_the_core() {
    let (_dir, mut app) = demo();
    let mut games = fixture_games();
    let now = Utc::now();
    for (i, game) in games.iter_mut().enumerate() {
        game.added_at = Some(now + TimeDelta::days(i as i64));
        game.last_played_at = Some(now - TimeDelta::days(i as i64));
        game.metacritic_score = Some(80 + i as i32);
        game.size_bytes = Some((i as u64 + 1) * 1024);
    }
    games[0].description = "Descrição do núcleo".into();
    games[0].developer = Some("Desenvolvedora".into());
    games[0].publisher = Some("Editora".into());
    games[0].release_date = Some("2020-01-02".parse().unwrap());
    games[0].genres = vec!["Ação".into()];
    games[0].critic_score = Some(85);
    app.reload_library(games).unwrap();
    for (sort, first) in [
        (SortOrder::RecentlyPlayed, "Árvore"),
        (SortOrder::Name, "Árvore"),
        (SortOrder::PlayTime, "Fortnite"),
        (SortOrder::RecentlyAdded, "Zelda"),
        (SortOrder::Rating, "Zelda"),
        (SortOrder::Size, "Zelda"),
    ] {
        let page = list(
            &mut app,
            LibraryListRequest {
                sort,
                ..Default::default()
            },
        );
        assert_eq!(page.rows[0].title, first);
    }
    let data = wire(
        &mut app,
        r#"{"v":1,"id":1,"name":"game.details","payload":{"gameId":"steam:570"}}"#,
    );
    let data = &data["reply"]["data"];
    assert_eq!(data["description"], "Descrição do núcleo");
    assert_eq!(data["developer"], "Desenvolvedora");
    assert_eq!(data["publisher"], "Editora");
    assert_eq!(data["releaseDate"], "2020-01-02");
    assert_eq!(data["genres"], json!(["Ação"]));
    assert_eq!(data["metacriticScore"], 80);
    assert_eq!(data["criticScore"], 85);
    assert_eq!(data["sizeBytes"], 1024);
}

#[test]
fn game_play_json_accepts_the_request_and_sends_only_the_id_to_the_host() {
    let (_dir, mut app) = demo();
    let request = game_request("steam:demo0");
    assert_eq!(
        app.handle(Call::GamePlay(request.clone()))
            .unwrap_err()
            .code,
        ErrorCode::Internal
    );
    let (tx, rx) = mpsc::channel();
    app.set_host_sink(tx);
    assert_eq!(
        app.handle(Call::GamePlay(request)).unwrap(),
        Reply::GamePlay
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        HostRequest::Launch {
            game_id: "steam:demo0".into()
        }
    );
    let reply = wire(
        &mut app,
        r#"{"v":1,"id":17,"name":"game.play","payload":{"gameId":"steam:demo0"}}"#,
    );
    assert_eq!(
        reply,
        json!({"v":1,"id":17,"ok":true,"reply":{"name":"game.play"}})
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        HostRequest::Launch {
            game_id: "steam:demo0".into()
        }
    );
    drop(rx);
    assert_eq!(
        app.handle(Call::GamePlay(game_request("steam:demo0")))
            .unwrap_err()
            .code,
        ErrorCode::Internal
    );
    // Aceitar um pedido não cria sessão nem altera as horas.
    let details = wire(
        &mut app,
        r#"{"v":1,"id":18,"name":"game.details","payload":{"gameId":"steam:demo0"}}"#,
    );
    assert!(details["reply"]["data"]["recentSessions"].is_null());
}

#[test]
fn missing_games_are_not_found_and_invalid_payloads_are_bad_request_through_the_bridge() {
    let (_dir, mut app) = demo();
    let (tx, rx) = mpsc::channel();
    app.set_host_sink(tx);
    for name in ["game.details", "game.play"] {
        let response = wire(
            &mut app,
            &json!({"v":1,"id":29,"name":name,"payload":{"gameId":"steam:missing"}}).to_string(),
        );
        assert_eq!(response["error"]["code"], "not_found");
        assert_eq!(response["id"], 29);
        for payload in [
            json!({}),
            json!(null),
            json!(["steam:demo0"]),
            json!({"gameId":2}),
            json!({"gameId":""}),
            json!({"gameId":"steam:demo0","extra":true}),
        ] {
            let response = wire(
                &mut app,
                &json!({"v":1,"id":30,"name":name,"payload":payload}).to_string(),
            );
            assert_eq!(response["error"]["code"], "bad_request", "{payload}");
        }
    }
    for payload in [
        json!(null),
        json!([]),
        json!({"limit":-1}),
        json!({"offset":1.5}),
        json!({"limit":4294967296_u64}),
        json!({"filter":"other"}),
        json!({"sort":"unknown"}),
        json!({"stores":["unknown"]}),
        json!({"statuses":["unknown"]}),
        json!({"tags":1}),
        json!({"extra":true}),
    ] {
        let response = wire(
            &mut app,
            &json!({"v":1,"id":31,"name":"library.list","payload":payload}).to_string(),
        );
        assert_eq!(response["error"]["code"], "bad_request", "{payload}");
    }
    assert!(rx.try_recv().is_err());
    for call in [
        Call::GameDetails(game_request("missing")),
        Call::GamePlay(game_request("missing")),
    ] {
        assert_eq!(app.handle(call).unwrap_err().code, ErrorCode::NotFound);
    }
}

#[test]
fn reload_notifies_after_new_data_is_visible_and_persists_the_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path());
    let core = Core::open(paths.clone(), Platform::simulated(), false).unwrap();
    let mut app = AppService::with_core("1", core);
    let (tx, rx) = mpsc::channel();
    app.set_event_sink(tx);
    app.reload_library(fixture_games()).unwrap();
    assert_eq!(rx.try_recv().unwrap(), Event::LibraryChanged);
    assert_eq!(list(&mut app, Default::default()).total, 3);
    // O encerramento drena o gravador; recargas sucessivas preservam a ordem.
    app.reload_library(fixture_games()[..1].to_vec()).unwrap();
    assert_eq!(rx.try_recv().unwrap(), Event::LibraryChanged);
    drop(app);
    let reopened = Core::open(paths.clone(), Platform::simulated(), false).unwrap();
    assert_eq!(reopened.library().games().len(), 1);
    drop(reopened);
    std::fs::remove_file(paths.cache().join("found-games.json")).unwrap();
    std::fs::create_dir(paths.cache().join("found-games.json")).unwrap();
    let core = Core::open(paths, Platform::simulated(), false).unwrap();
    let mut app = AppService::with_core("1", core);
    let (tx, rx) = mpsc::channel();
    app.set_event_sink(tx);
    app.reload_library(fixture_games()).unwrap();
    assert_eq!(rx.try_recv().unwrap(), Event::LibraryChanged);
    assert_eq!(list(&mut app, Default::default()).total, 3);
    app.reload_library(Vec::new()).unwrap();
    assert_eq!(rx.try_recv().unwrap(), Event::LibraryChanged);
    assert_eq!(list(&mut app, Default::default()).total, 0);
    drop(app);
}

#[test]
fn broken_and_oversized_snapshots_do_not_panic_or_prevent_boot() {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path());
    std::fs::create_dir_all(paths.cache()).unwrap();
    let snapshot = paths.cache().join("found-games.json");
    for content in [
        "{",
        "null",
        "[]",
        r#"[{"play_time":1e300}]"#,
        r#"[{"play_time":-1}]"#,
    ] {
        std::fs::write(&snapshot, content).unwrap();
        let core = Core::open(paths.clone(), Platform::simulated(), false).unwrap();
        assert!(core.library().games().is_empty());
    }
    let file = std::fs::File::create(snapshot).unwrap();
    file.set_len(64 * 1024 * 1024 + 1).unwrap();
    drop(file);
    assert!(
        Core::open(paths, Platform::simulated(), false)
            .unwrap()
            .library()
            .games()
            .is_empty()
    );
}

#[test]
fn boot_propagates_filesystem_failures_and_queries_require_a_core() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("file");
    std::fs::write(&file, "arquivo").unwrap();
    assert!(Core::open(DataPaths::at(file), Platform::simulated(), false).is_err());
    let mut app = AppService::new("1");
    assert_eq!(
        app.handle(Call::LibraryList(Default::default()))
            .unwrap_err()
            .code,
        ErrorCode::Internal
    );
}

#[test]
fn unreadable_settings_and_collections_fall_back_without_preventing_boot() {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path());
    let database = TakoDatabase::new(paths.database());
    let connection = database.open().unwrap();
    // O banco continua íntegro e abre; só os SELECTs das duas tabelas falham.
    connection
        .execute_batch(
            "ALTER TABLE settings RENAME COLUMN value TO broken;
        ALTER TABLE collections RENAME COLUMN name TO broken;",
        )
        .unwrap();
    drop(connection);
    let core = Core::open(paths, Platform::simulated(), false).unwrap();
    assert!(core.setting("Language").is_none());
    assert_eq!(core.language(), Lang::from_env());
    assert!(core.library().games().is_empty());
    // A falha das coleções também não bloqueia a recarga.
    let mut app = AppService::with_core("1", core);
    app.reload_library(fixture_games()).unwrap();
    assert_eq!(list(&mut app, Default::default()).total, 3);
}

#[test]
fn reload_uses_loaded_state_and_collections_without_reading_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path());
    let database = TakoDatabase::new(paths.database());
    let store = db::LibraryStore::new(database.clone(), None);
    let mut data = store.load().unwrap();
    data.state.insert(
        "steam:570".into(),
        db::GameStateRow {
            is_favorite: true,
            ..Default::default()
        },
    );
    store.save(&data).unwrap();
    let collections = db::CollectionStore::new(database.clone());
    collections.insert("sofa", "Sofá", Utc::now()).unwrap();
    collections
        .add_game("sofa", "steam:570", Utc::now())
        .unwrap();
    let core = Core::open(paths, Platform::simulated(), false).unwrap();
    database
        .open()
        .unwrap()
        .execute_batch("DROP TABLE collections; DROP TABLE game_state;")
        .unwrap();
    let mut app = AppService::with_core("1", core);
    app.reload_library(fixture_games()).unwrap();
    let page = list(
        &mut app,
        LibraryListRequest {
            collection: Some("sofa".into()),
            ..Default::default()
        },
    );
    assert_eq!(page.total, 1);
    assert!(page.rows[0].favorite);
}

#[test]
fn library_criteria_enforce_character_and_item_limits() {
    let (_dir, mut app) = demo();
    for payload in [
        json!({"search":"é".repeat(256)}),
        json!({"tags":vec!["界".repeat(128); 64]}),
        json!({"statuses":vec!["playing"; 64]}),
        json!({"stores":vec!["steam"; 64]}),
        json!({"collection":"é".repeat(128)}),
    ] {
        assert_eq!(
            wire(
                &mut app,
                &json!({"v":1,"id":60,"name":"library.list","payload":payload}).to_string()
            )["ok"],
            true
        );
    }
    for payload in [
        json!({"search":"é".repeat(257)}),
        json!({"tags":["界".repeat(129)]}),
        json!({"tags":vec!["a";65]}),
        json!({"statuses":vec!["playing";65]}),
        json!({"stores":vec!["steam";65]}),
        json!({"collection":"é".repeat(129)}),
    ] {
        let result = wire(
            &mut app,
            &json!({"v":1,"id":61,"name":"library.list","payload":payload}).to_string(),
        );
        assert_eq!(result["error"]["code"], "bad_request");
        assert_eq!(result["id"], 61);
    }
    assert_eq!(
        app.handle(Call::LibraryList(LibraryListRequest {
            search: "x".repeat(257),
            ..Default::default()
        }))
        .unwrap_err()
        .code,
        ErrorCode::BadRequest
    );
}

#[test]
fn request_parser_distinguishes_unknown_calls_and_invalid_payload_enums() {
    use tako_app::protocol::{MAX_MESSAGE_BYTES, parse_request};
    for (text, id, code) in [
        (
            r#"{"v":1,"id":70,"name":"window.action","payload":"invalid"}"#,
            70,
            ErrorCode::BadRequest,
        ),
        (
            r#"{"v":1,"id":71,"name":"unknown.call"}"#,
            71,
            ErrorCode::UnknownCall,
        ),
        (
            r#"{"v":99,"id":72,"name":"app.info"}"#,
            72,
            ErrorCode::VersionMismatch,
        ),
        (
            r#"{"v":"1","id":73,"name":"app.info"}"#,
            73,
            ErrorCode::BadRequest,
        ),
        ("{", 0, ErrorCode::BadRequest),
        ("[]", 0, ErrorCode::BadRequest),
    ] {
        let (actual_id, error) = parse_request(text).unwrap_err();
        assert_eq!(actual_id, id);
        assert_eq!(error.code, code);
    }
    let oversized = format!(
        r#"{{"v":1,"id":1,"name":"app.info","x":"{}"}}"#,
        "x".repeat(MAX_MESSAGE_BYTES)
    );
    let (id, error) = parse_request(&oversized).unwrap_err();
    assert_eq!(id, 0);
    assert_eq!(error.code, ErrorCode::BadRequest);
    let defaults = parse_request(r#"{"v":1,"id":1,"name":"library.list","payload":{}}"#).unwrap();
    assert_eq!(defaults.call, Call::LibraryList(Default::default()));
}

#[test]
fn unplayable_games_conflict_without_sending_host_requests() {
    let (_dir, mut app) = demo();
    let mut games = fixture_games();
    games[0].upcoming_release = Some("2099-01-01".parse().unwrap());
    games[0].platform = Some("snes".into());
    app.reload_library(games).unwrap();
    let (tx, rx) = mpsc::channel();
    app.set_host_sink(tx);
    for (id, upcoming, platform) in [
        ("steam:570", true, json!("snes")),
        ("heroic:legendary:Fortnite", false, Value::Null),
    ] {
        let result = wire(
            &mut app,
            &json!({"v":1,"id":80,"name":"game.play","payload":{"gameId":id}}).to_string(),
        );
        assert_eq!(result["error"]["code"], "conflict");
        let details = wire(
            &mut app,
            &json!({"v":1,"id":81,"name":"game.details","payload":{"gameId":id}}).to_string(),
        );
        assert_eq!(details["reply"]["data"]["canPlay"], false);
        assert_eq!(details["reply"]["data"]["upcoming"], upcoming);
        assert_eq!(details["reply"]["data"]["platform"], platform);
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn accent_is_only_a_six_digit_hex_color_and_play_time_is_an_integer() {
    let (_dir, mut app) = demo();
    for (accent, valid) in [
        ("#aB012F", true),
        ("red", false),
        ("#123", false),
        ("#12345678", false),
        ("#GG0000", false),
        (" #123456", false),
        ("#é1234", false),
    ] {
        let mut games = fixture_games();
        games[0].accent = accent.into();
        games[0].play_time = Duration::from_millis(1500);
        app.reload_library(games).unwrap();
        let page = wire(
            &mut app,
            r#"{"v":1,"id":90,"name":"library.list","payload":{"search":"Árvore"}}"#,
        );
        let row = &page["reply"]["data"]["rows"][0];
        assert_eq!(
            row["accent"],
            if valid { json!(accent) } else { Value::Null }
        );
        assert_eq!(row["playTimeSeconds"].as_u64(), Some(1));
    }
}

#[test]
fn snapshot_durations_are_read_once_and_invalid_values_become_zero() {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path());
    std::fs::create_dir_all(paths.cache()).unwrap();
    for secs in [json!(1e300), json!(-1)] {
        let mut games = serde_json::to_value(fixture_games()).unwrap();
        games[0]["play_time"] = secs;
        std::fs::write(paths.cache().join("found-games.json"), games.to_string()).unwrap();
        let core = Core::open(paths.clone(), Platform::simulated(), false).unwrap();
        assert_eq!(core.library().games().len(), 3);
        assert_eq!(
            core.library().find("steam:570").unwrap().play_time,
            Duration::ZERO
        );
    }
}
