use super::*;
use rusqlite::Connection;
use std::fs;
use tempfile::tempdir;

/// Fixture de banco de dados reproduzindo o esquema real do butler:
/// Fonte: github.com/itchio/butler
/// - database/models/cave.go (tabela `caves`: id, game_id, install_location_id, install_folder_name, custom_install_folder, verdict, installed_size, seconds_run, last_touched_at)
/// - database/models/install_location.go (tabela `install_locations`: id, path)
fn setup_test_db(db_path: &Path, base_install_dir: &Path, verdict_json: Option<&str>) {
    let conn = Connection::open(db_path).unwrap();
    conn.execute_batch(
        "CREATE TABLE install_locations (
            id TEXT PRIMARY KEY,
            path TEXT NOT NULL
        );
        CREATE TABLE games (
            id INTEGER PRIMARY KEY,
            title TEXT,
            url TEXT,
            cover_url TEXT,
            still_cover_url TEXT
        );
        CREATE TABLE caves (
            id TEXT PRIMARY KEY,
            game_id INTEGER NOT NULL,
            install_location_id TEXT,
            install_folder_name TEXT,
            custom_install_folder TEXT,
            verdict TEXT,
            installed_size INTEGER,
            seconds_run INTEGER,
            last_touched_at TEXT
        );",
    )
    .unwrap();

    conn.execute(
        "INSERT INTO install_locations (id, path) VALUES ('custom_loc', ?1)",
        [base_install_dir.to_str().unwrap()],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO games (id, title, url, cover_url, still_cover_url)
         VALUES (42, 'Celeste Classic', 'https://maddymakesgames.itch.io/celeste', 'https://img.itch.zone/cover.gif', 'https://img.itch.zone/still.png')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO caves (id, game_id, install_location_id, install_folder_name, custom_install_folder, verdict, installed_size, seconds_run, last_touched_at)
         VALUES ('cave-1', 42, 'custom_loc', 'celeste_classic', NULL, ?1, 10485760, 3600, '2026-05-10T14:30:00Z')",
        [verdict_json],
    )
    .unwrap();
}

#[test]
fn reads_installed_games_from_butler_db_via_verdict() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("butler.db");
    let apps_dir = dir.path().join("apps");
    let game_dir = apps_dir.join("celeste_classic");
    fs::create_dir_all(&game_dir).unwrap();

    let exe_path = game_dir.join("celeste.x86_64");
    fs::write(&exe_path, b"fake binary").unwrap();

    let verdict = serde_json::json!({
        "basePath": game_dir.to_str().unwrap(),
        "candidates": [
            {
                "path": "celeste.x86_64",
                "flavor": "linux"
            }
        ]
    })
    .to_string();

    setup_test_db(&db_path, &apps_dir, Some(&verdict));

    let games = read_games(&db_path).unwrap();
    assert_eq!(games.len(), 1);

    let g = &games[0];
    assert_eq!(g.id, "itch:42");
    assert_eq!(g.title, "Celeste Classic");
    assert_eq!(g.store, Some(GameStore::Itch));
    assert_eq!(g.accent, "#FA5C5C");
    assert_eq!(
        g.cover_uri.as_deref(),
        Some("https://img.itch.zone/still.png")
    );
    assert_eq!(g.size_bytes, Some(10485760));
    assert_eq!(g.play_time, Duration::from_secs(3600));
    assert!(g.last_played_at.is_some());
    assert_eq!(g.store_product_id.as_deref(), Some("42"));

    assert_eq!(g.launch_target.as_deref(), Some(exe_path.to_str().unwrap()));
    assert_eq!(
        g.working_directory.as_deref(),
        Some(game_dir.to_str().unwrap())
    );
    assert_eq!(g.tracking, Some(GameTrackingStrategy::Process));
}

#[test]
fn falls_back_to_itch_protocol_when_executable_missing() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("butler.db");
    let apps_dir = dir.path().join("apps");

    setup_test_db(&db_path, &apps_dir, None);

    let games = read_games(&db_path).unwrap();
    assert_eq!(games.len(), 1);
    let g = &games[0];

    assert_eq!(
        g.launch_target.as_deref(),
        Some("itch://install?game_id=42&launch")
    );
    assert_eq!(g.working_directory, None);
    assert_eq!(g.tracking, None);
}

#[test]
fn reads_executable_from_itch_toml_manifest() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("butler.db");
    let apps_dir = dir.path().join("apps");
    let game_dir = apps_dir.join("celeste_classic");
    let bin_dir = game_dir.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let target_sh = bin_dir.join("start.sh");
    fs::write(&target_sh, b"#!/bin/sh\necho hi\n").unwrap();

    let manifest = game_dir.join(".itch.toml");
    fs::write(
        manifest,
        "[[actions]]\nname = 'Play'\npath = 'bin/start.sh'\n",
    )
    .unwrap();

    setup_test_db(&db_path, &apps_dir, None);

    let games = read_games(&db_path).unwrap();
    assert_eq!(games.len(), 1);
    assert_eq!(
        games[0].launch_target.as_deref(),
        Some(target_sh.to_str().unwrap())
    );
}

#[test]
fn itch_toml_multiple_actions_respects_platform() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("butler.db");
    let apps_dir = dir.path().join("apps");
    let game_dir = apps_dir.join("celeste_classic");
    fs::create_dir_all(&game_dir).unwrap();

    let win_exe = game_dir.join("celeste.exe");
    fs::write(&win_exe, b"windows exe").unwrap();

    let linux_sh = game_dir.join("celeste.sh");
    fs::write(&linux_sh, b"linux sh").unwrap();

    let manifest = game_dir.join(".itch.toml");
    fs::write(
        manifest,
        "[[actions]]\nname = 'Play Windows'\nplatform = 'windows'\npath = 'celeste.exe'\n\n[[actions]]\nname = 'Play Linux'\nplatform = 'linux'\npath = 'celeste.sh'\n",
    )
    .unwrap();

    setup_test_db(&db_path, &apps_dir, None);

    let games = read_games(&db_path).unwrap();
    assert_eq!(games.len(), 1);
    assert_eq!(
        games[0].launch_target.as_deref(),
        Some(linux_sh.to_str().unwrap())
    );
}

#[test]
fn itch_toml_url_falls_back_to_protocol() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("butler.db");
    let apps_dir = dir.path().join("apps");
    let game_dir = apps_dir.join("celeste_classic");
    fs::create_dir_all(&game_dir).unwrap();

    let manifest = game_dir.join(".itch.toml");
    fs::write(
        manifest,
        "[[actions]]\nname = 'Play Web'\npath = 'https://game.itch.zone/html5/index.html'\n",
    )
    .unwrap();

    setup_test_db(&db_path, &apps_dir, None);

    let games = read_games(&db_path).unwrap();
    assert_eq!(games.len(), 1);
    assert_eq!(
        games[0].launch_target.as_deref(),
        Some("itch://install?game_id=42&launch")
    );
}

#[test]
fn itch_toml_absolute_path_falls_back_to_protocol() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("butler.db");
    let apps_dir = dir.path().join("apps");
    let game_dir = apps_dir.join("celeste_classic");
    fs::create_dir_all(&game_dir).unwrap();

    let manifest = game_dir.join(".itch.toml");
    fs::write(manifest, "[[actions]]\nname = 'Play'\npath = '/bin/sh'\n").unwrap();

    setup_test_db(&db_path, &apps_dir, None);

    let games = read_games(&db_path).unwrap();
    assert_eq!(games.len(), 1);
    assert_eq!(
        games[0].launch_target.as_deref(),
        Some("itch://install?game_id=42&launch")
    );
}

#[test]
fn itch_toml_parent_traversal_falls_back_to_protocol() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("butler.db");
    let apps_dir = dir.path().join("apps");
    let game_dir = apps_dir.join("celeste_classic");
    fs::create_dir_all(&game_dir).unwrap();

    let manifest = game_dir.join(".itch.toml");
    fs::write(
        manifest,
        "[[actions]]\nname = 'Play'\npath = '../escape.sh'\n",
    )
    .unwrap();

    setup_test_db(&db_path, &apps_dir, None);

    let games = read_games(&db_path).unwrap();
    assert_eq!(games.len(), 1);
    assert_eq!(
        games[0].launch_target.as_deref(),
        Some("itch://install?game_id=42&launch")
    );
}

#[test]
fn verdict_multiple_linux_candidates_falls_back_to_protocol() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("butler.db");
    let apps_dir = dir.path().join("apps");
    let game_dir = apps_dir.join("celeste_classic");
    fs::create_dir_all(&game_dir).unwrap();

    let exe1 = game_dir.join("game1.x86_64");
    let exe2 = game_dir.join("game2.x86_64");
    fs::write(&exe1, b"bin1").unwrap();
    fs::write(&exe2, b"bin2").unwrap();

    let verdict = serde_json::json!({
        "basePath": game_dir.to_str().unwrap(),
        "candidates": [
            { "path": "game1.x86_64", "flavor": "linux" },
            { "path": "game2.x86_64", "flavor": "linux" }
        ]
    })
    .to_string();

    setup_test_db(&db_path, &apps_dir, Some(&verdict));

    let games = read_games(&db_path).unwrap();
    assert_eq!(games.len(), 1);
    assert_eq!(
        games[0].launch_target.as_deref(),
        Some("itch://install?game_id=42&launch")
    );
}

#[test]
fn verdict_no_linux_candidates_falls_back_to_protocol() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("butler.db");
    let apps_dir = dir.path().join("apps");
    let game_dir = apps_dir.join("celeste_classic");
    fs::create_dir_all(&game_dir).unwrap();

    let exe_win = game_dir.join("game.exe");
    fs::write(&exe_win, b"win bin").unwrap();

    let verdict = serde_json::json!({
        "basePath": game_dir.to_str().unwrap(),
        "candidates": [
            { "path": "game.exe", "flavor": "windows" }
        ]
    })
    .to_string();

    setup_test_db(&db_path, &apps_dir, Some(&verdict));

    let games = read_games(&db_path).unwrap();
    assert_eq!(games.len(), 1);
    assert_eq!(
        games[0].launch_target.as_deref(),
        Some("itch://install?game_id=42&launch")
    );
}

#[test]
fn reads_installed_games_custom_install_folder() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("butler.db");
    let custom_dir = dir.path().join("custom_folder");
    fs::create_dir_all(&custom_dir).unwrap();

    let target_sh = custom_dir.join("run.sh");
    fs::write(&target_sh, b"run").unwrap();

    let manifest = custom_dir.join(".itch.toml");
    fs::write(manifest, "[[actions]]\nname = 'Play'\npath = 'run.sh'\n").unwrap();

    let conn = Connection::open(&db_path).unwrap();
    conn.execute_batch(
        "CREATE TABLE install_locations (id TEXT PRIMARY KEY, path TEXT NOT NULL);
         CREATE TABLE games (id INTEGER PRIMARY KEY, title TEXT, url TEXT, cover_url TEXT, still_cover_url TEXT);
         CREATE TABLE caves (id TEXT PRIMARY KEY, game_id INTEGER NOT NULL, install_location_id TEXT, install_folder_name TEXT, custom_install_folder TEXT, verdict TEXT, installed_size INTEGER, seconds_run INTEGER, last_touched_at TEXT);",
    )
    .unwrap();

    conn.execute(
        "INSERT INTO games (id, title) VALUES (99, 'Custom Game')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO caves (id, game_id, install_location_id, install_folder_name, custom_install_folder, verdict, installed_size, seconds_run, last_touched_at)
         VALUES ('cave-custom', 99, NULL, NULL, ?1, NULL, 5000, 10, '2026-06-01T00:00:00Z')",
        [custom_dir.to_str().unwrap()],
    )
    .unwrap();

    let games = read_games(&db_path).unwrap();
    assert_eq!(games.len(), 1);
    assert_eq!(games[0].title, "Custom Game");
    assert_eq!(
        games[0].launch_target.as_deref(),
        Some(target_sh.to_str().unwrap())
    );
}

#[test]
fn broken_database_is_an_error_not_a_panic() {
    let dir = tempdir().unwrap();
    let broken = dir.path().join("butler.db");
    fs::write(&broken, b"nao e um sqlite valido").unwrap();

    assert!(read_games(&broken).is_err());
}

#[test]
fn missing_db_reports_notice() {
    let dir = tempdir().unwrap();
    let source = ItchSource::new(vec![dir.path().join("butler.db")]);

    let scan = source.scan();
    assert!(!scan.client_installed);
    assert!(scan.games.is_empty());
    assert!(scan.notice.is_some());
}

#[test]
fn db_candidates_deduplication() {
    let dir = tempdir().unwrap();
    let home = dir.path().join("usuario");
    let candidates = db_candidates(&home, None);
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0], home.join(".config/itch/db/butler.db"));
    assert_eq!(
        candidates[1],
        home.join(".var/app/io.itch.itch/config/itch/db/butler.db")
    );

    let sandbox = dir.path().join("sandbox");
    let candidates = db_candidates(&home, Some(sandbox.clone()));
    assert_eq!(candidates.len(), 3);
    assert_eq!(candidates[0], sandbox.join("itch/db/butler.db"));
    assert_eq!(candidates[2], home.join(".config/itch/db/butler.db"));
}
