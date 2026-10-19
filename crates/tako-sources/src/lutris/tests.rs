//! Banco fictício com o esquema real de `lutris/database/schema.py` (tabela `games`).

#![allow(clippy::unwrap_used)]

use super::*;
use std::fs;

const SCHEMA: &str = "CREATE TABLE games (
    id INTEGER PRIMARY KEY, name TEXT, sortname TEXT, slug TEXT, installer_slug TEXT,
    parent_slug TEXT, platform TEXT, runner TEXT, executable TEXT, directory TEXT,
    updated DATETIME, lastplayed INTEGER, installed INTEGER, installed_at INTEGER, year INTEGER,
    configpath TEXT, has_custom_banner INTEGER, has_custom_icon INTEGER,
    has_custom_coverart_big INTEGER, playtime REAL, service TEXT, service_id TEXT, discord_id TEXT)";

fn fake_lutris() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path().join("lutris");
    fs::create_dir_all(data.join("coverart")).unwrap();
    fs::create_dir_all(data.join("banners")).unwrap();
    fs::create_dir_all(tmp.path().join("icons/hicolor/128x128/apps")).unwrap();
    fs::write(data.join("coverart/jogo-wine.jpg"), b"jpg").unwrap();
    fs::write(data.join("banners/jogo-wine.png"), b"png").unwrap();
    fs::write(
        tmp.path()
            .join("icons/hicolor/128x128/apps/lutris_jogo-wine.png"),
        b"png",
    )
    .unwrap();

    let db = Connection::open(data.join("pga.db")).unwrap();
    db.execute_batch(SCHEMA).unwrap();
    db.execute_batch(
        "INSERT INTO games (id, name, slug, runner, directory, installed, playtime, lastplayed, installed_at)
         VALUES (7, 'Jogo Wine Fictício', 'jogo-wine', 'wine', '/home/a/Games/jogo-wine', 1, 1.5, 1767225600, 1764547200);
         INSERT INTO games (id, name, slug, runner, directory, installed, playtime)
         VALUES (8, 'Não Instalado', 'nao-instalado', 'linux', '', 0, 0);
         INSERT INTO games (id, name, slug, runner, directory, installed, playtime, lastplayed)
         VALUES (9, 'Emulado', 'emulado', 'mednafen', '', 1, NULL, NULL);
         INSERT INTO games (id, name, slug, installed) VALUES (10, 'Slug ruim', '../etc', 1);",
    )
    .unwrap();
    tmp
}

#[test]
fn reads_installed_games_with_art_and_playtime() {
    let tmp = fake_lutris();
    let mut games = read_games(&tmp.path().join("lutris")).unwrap();
    games.sort_by(|a, b| a.id.cmp(&b.id));
    assert_eq!(
        games.iter().map(|g| g.id.as_str()).collect::<Vec<_>>(),
        ["lutris:emulado", "lutris:jogo-wine"]
    );

    let g = &games[1];
    assert_eq!(g.title, "Jogo Wine Fictício");
    assert_eq!(g.store, Some(GameStore::Lutris));
    assert_eq!(g.launch_target.as_deref(), Some("lutris:rungameid/7"));
    assert_eq!(g.install_path.as_deref(), Some("/home/a/Games/jogo-wine"));
    assert_eq!(g.play_time, Duration::from_secs(5400));
    assert_eq!(g.last_played_at.unwrap().timestamp(), 1767225600);
    assert_eq!(g.added_at.unwrap().timestamp(), 1764547200);
    assert!(
        g.cover_uri
            .as_deref()
            .unwrap()
            .ends_with("/coverart/jogo-wine.jpg")
    );
    assert!(
        g.hero_uri
            .as_deref()
            .unwrap()
            .ends_with("/banners/jogo-wine.png")
    );
    assert!(g.media[media::ICON].ends_with("/lutris_jogo-wine.png"));
    assert!(g.has_fallback_art);

    let e = &games[0];
    assert_eq!(e.install_path, None);
    assert_eq!(e.cover_uri, None);
    assert_eq!(e.play_time, Duration::ZERO);
    assert_eq!(e.last_played_at, None);
}

#[test]
fn broken_database_is_an_error_not_a_panic() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("pga.db"), b"isto nao e sqlite").unwrap();
    assert!(read_games(tmp.path()).is_err());

    // Banco válido sem a tabela `games`.
    let other = tmp.path().join("vazio");
    fs::create_dir_all(&other).unwrap();
    Connection::open(other.join("pga.db"))
        .unwrap()
        .execute_batch("CREATE TABLE x (a INTEGER)")
        .unwrap();
    assert!(read_games(&other).is_err());

    let scan = LutrisSource::new(vec![tmp.path().to_path_buf()]).scan();
    assert!(scan.client_installed);
    assert!(scan.games.is_empty());
}

#[test]
fn source_merges_dirs_and_reports_missing() {
    let tmp = fake_lutris();
    let dir = tmp.path().join("lutris");
    let scan = LutrisSource::new(vec![dir.clone(), dir]).scan();
    assert_eq!(scan.games.len(), 2);
    assert_eq!(scan.games[0].title, "Emulado");

    let none = LutrisSource::new(vec![tmp.path().join("nada")]).scan();
    assert!(!none.client_installed);
    assert!(none.notice.is_some());
}

// Caminhos montados a partir do temporário: `is_absolute` depende do SO (no Windows "/home/u" não é absoluto).
#[test]
fn data_dirs_do_not_repeat_the_default_folder() {
    let home = std::env::temp_dir().join("u");
    let expected = [
        home.join(".local/share/lutris"),
        home.join(".var/app/net.lutris.Lutris/data/lutris"),
    ];
    for xdg in [
        None,
        Some(home.join(".local/share")),
        Some(PathBuf::from("rel")),
    ] {
        assert_eq!(data_dirs(&home, xdg), expected);
    }
}

#[test]
fn data_dirs_add_the_host_folder_when_xdg_is_the_sandbox() {
    let home = std::env::temp_dir().join("u");
    let sandbox = home.join(".var/app/io.github.lippdev.Tako/data");
    let dirs = data_dirs(&home, Some(sandbox.clone()));
    assert_eq!(dirs.len(), 3);
    assert_eq!(dirs[0], sandbox.join("lutris"));
    assert_eq!(dirs[2], home.join(".local/share/lutris"));
}
