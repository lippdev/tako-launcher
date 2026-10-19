use super::*;
use rusqlite::Connection;
use std::fs;
use tempfile::tempdir;

fn setup_test_db(db_path: &Path, base_install_dir: &Path) {
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
            install_folder TEXT NOT NULL,
            install_location TEXT NOT NULL,
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
        "INSERT INTO caves (id, game_id, install_folder, install_location, installed_size, seconds_run, last_touched_at)
         VALUES ('cave-1', 42, 'celeste_classic', 'custom_loc', 10485760, 3600, '2026-05-10T14:30:00Z')",
        [],
    )
    .unwrap();
}

#[test]
fn reads_installed_games_from_butler_db() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("butler.db");
    let apps_dir = dir.path().join("apps");
    let game_dir = apps_dir.join("celeste_classic");
    fs::create_dir_all(&game_dir).unwrap();

    let exe_path = game_dir.join("celeste.x86_64");
    fs::write(&exe_path, b"fake binary").unwrap();

    setup_test_db(&db_path, &apps_dir);

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

    setup_test_db(&db_path, &apps_dir);

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

    setup_test_db(&db_path, &apps_dir);

    let games = read_games(&db_path).unwrap();
    assert_eq!(games.len(), 1);
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
    let home = Path::new("/home/usuario");
    let candidates = db_candidates(home, None);
    assert_eq!(candidates.len(), 2);
    assert_eq!(
        candidates[0],
        Path::new("/home/usuario/.config/itch/db/butler.db")
    );
    assert_eq!(
        candidates[1],
        Path::new("/home/usuario/.var/app/io.itch.itch/config/itch/db/butler.db")
    );

    let sandbox = Path::new("/home/usuario/.var/app/io.github.lippdev.Tako/config");
    let candidates = db_candidates(home, Some(sandbox.to_path_buf()));
    assert_eq!(candidates.len(), 3);
    assert_eq!(candidates[0], sandbox.join("itch/db/butler.db"));
    assert_eq!(
        candidates[2],
        Path::new("/home/usuario/.config/itch/db/butler.db")
    );
}
