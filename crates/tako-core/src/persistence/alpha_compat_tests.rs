//! Aceite do MIG-011: um `tako.db` montado como a alpha C# monta (esquema v6 literal, linhas como o C# grava)
//! abre no Rust sem perder nada.

use super::alpha_migration::ALPHA_BACKUP_SUFFIX;
use super::database::with_suffix;
use super::migrator::{self, tests::count};
use super::*;
use crate::library::{ArtKind, GameTrackingStrategy};
use crate::session::settings::GameSettingsOverride;
use chrono::TimeZone;
use rusqlite::Connection;

/// SQL do `SchemaMigrator.cs` v1..v6, copiado do C# (não do migrator Rust, para o teste valer algo).
const CSHARP_V6: &str = "
CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS manual_games (
    id TEXT PRIMARY KEY, title TEXT NOT NULL, path TEXT NOT NULL, arguments TEXT, working_directory TEXT,
    cover_path TEXT, hero_path TEXT, added_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS game_state (
    game_id TEXT PRIMARY KEY, is_favorite INTEGER NOT NULL DEFAULT 0, last_played_at TEXT,
    play_minutes REAL NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
ALTER TABLE manual_games ADD COLUMN track_by TEXT;
CREATE TABLE IF NOT EXISTS collections (id TEXT PRIMARY KEY, name TEXT NOT NULL, created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS collection_games (
    collection_id TEXT NOT NULL, game_id TEXT NOT NULL, added_at TEXT NOT NULL, PRIMARY KEY (collection_id, game_id));
CREATE TABLE IF NOT EXISTS game_settings (
    game_id TEXT PRIMARY KEY, use_game_screen INTEGER, screen_mode TEXT, hdr INTEGER, audio_device_id TEXT,
    audio_device_name TEXT, fps_limit INTEGER, show_fps INTEGER, extra_arguments TEXT);
ALTER TABLE manual_games ADD COLUMN platform TEXT;
CREATE TABLE IF NOT EXISTS game_art (game_id TEXT NOT NULL, kind TEXT NOT NULL, url TEXT NOT NULL, PRIMARY KEY (game_id, kind));
CREATE TABLE IF NOT EXISTS game_source_ids (
    game_id TEXT NOT NULL, source TEXT NOT NULL, source_id TEXT NOT NULL, PRIMARY KEY (game_id, source));
INSERT INTO schema_migrations VALUES
    (1, '2026-05-02T18:30:12.1234567+00:00'), (2, '2026-05-02T18:30:12.1300000+00:00'),
    (3, '2026-06-10T09:00:00.0000001+00:00'), (4, '2026-08-01T21:15:44.9876543+00:00'),
    (5, '2026-09-03T10:00:00.5000000+00:00'), (6, '2026-10-01T08:00:00.0000000+00:00');

INSERT INTO manual_games (id, title, path, arguments, working_directory, cover_path, hero_path, added_at, track_by, platform) VALUES
    ('manual:3f2a9c', 'Jogo Fictício', 'C:\\Jogos\\Ficticio\\jogo.exe', '-windowed', 'C:\\Jogos\\Ficticio',
     'C:\\Users\\a\\capa.png', NULL, '2026-10-15T12:00:00.0000000-03:00', 'InstallFolder', NULL),
    ('manual:rom01', 'Lanterna', 'D:\\roms\\lanterna.sfc', NULL, 'D:\\roms', NULL, NULL,
     '2026-10-16T01:02:03.4560000+00:00', NULL, 'snes'),
    ('manual:uri', 'Por URI', 'steam://rungameid/1', NULL, NULL, NULL, NULL, '2026-10-16T00:00:00Z', 'Futuro', NULL);
INSERT INTO game_state (game_id, is_favorite, last_played_at, play_minutes) VALUES
    ('steam:570', 1, '2026-10-14T23:59:59.9999999+00:00', 1234.5),
    ('manual:3f2a9c', 0, NULL, 0);
INSERT INTO settings (key, value) VALUES
    ('DynamicColor', 'false'), ('ControlCenterChord', '12288'), ('DisabledStores', 'Ea,Amazon'),
    ('GameScreenMode', '1920x1080@60');
INSERT INTO collections (id, name, created_at) VALUES
    ('c1', 'Aventura', '2026-09-01T10:00:00.0000000+00:00'), ('c2', 'Corrida', '2026-09-02T10:00:00.0000000+00:00');
INSERT INTO collection_games (collection_id, game_id, added_at) VALUES
    ('c1', 'steam:570', '2026-09-01T10:00:00.0000000+00:00'), ('c1', 'manual:3f2a9c', '2026-09-01T10:00:01.0000000+00:00'),
    ('c2', 'steam:570', '2026-09-02T10:00:00.0000000+00:00');
INSERT INTO game_settings VALUES
    ('steam:570', 1, '2560x1440@144', 0, '{0.0.0.00000000}.{fone}', 'Fone', 60, 1, '-dx11'),
    ('steam:730', NULL, NULL, NULL, NULL, NULL, -1, NULL, NULL);
INSERT INTO game_art VALUES
    ('steam:570', 'cover', 'https://cdn2.steamgriddb.com/grid/a.png'), ('steam:570', 'logo', 'file:///C:/Users/a/logo.png');
INSERT INTO game_source_ids VALUES ('steam:570', 'IGDB', '900001'), ('steam:570', 'SteamGridDB', '');
";

#[test]
fn csharp_v6_database_opens_without_data_loss() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tako.db");
    Connection::open(&path)
        .unwrap()
        .execute_batch(CSHARP_V6)
        .unwrap();
    let db = TakoDatabase::new(&path);

    // Biblioteca.
    let lib = LibraryStore::new(db.clone(), None).load().unwrap();
    assert!(!db.recovered_from_corruption());
    let ids: Vec<_> = lib.manual.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["manual:3f2a9c", "manual:rom01", "manual:uri"]);
    let a = &lib.manual[0];
    assert_eq!(a.path, r"C:\Jogos\Ficticio\jogo.exe");
    assert_eq!(a.arguments.as_deref(), Some("-windowed"));
    assert_eq!(a.working_directory.as_deref(), Some(r"C:\Jogos\Ficticio"));
    assert_eq!(a.cover_path.as_deref(), Some(r"C:\Users\a\capa.png"));
    assert_eq!(a.track_by, Some(GameTrackingStrategy::InstallFolder));
    assert_eq!(
        a.added_at,
        Utc.with_ymd_and_hms(2026, 10, 15, 15, 0, 0).unwrap()
    );
    assert_eq!(lib.manual[1].platform.as_deref(), Some("snes"));
    assert_eq!(lib.manual[1].added_at.timestamp_subsec_millis(), 456);
    assert_eq!(lib.manual[2].track_by, None); // nome desconhecido = automático
    let s = &lib.state["steam:570"];
    assert!(s.is_favorite);
    assert_eq!(s.play_minutes, 1234.5);
    assert_eq!(
        s.last_played_at.unwrap().timestamp_subsec_nanos(),
        999_999_900
    );
    assert_eq!(lib.state.len(), 2);

    // Configurações.
    let settings = SettingsStore::new(db.clone());
    assert_eq!(settings.load_all().unwrap().len(), 4);
    assert_eq!(settings.get_bool("DynamicColor").unwrap(), Some(false));
    assert_eq!(settings.get_i64("ControlCenterChord").unwrap(), Some(12288));
    assert_eq!(
        settings.get("DisabledStores").unwrap().as_deref(),
        Some("Ea,Amazon")
    );

    // Coleções.
    let cols = CollectionStore::new(db.clone()).load_all().unwrap();
    assert_eq!(cols.len(), 2);
    assert_eq!(cols[0].name, "Aventura");
    assert_eq!(cols[0].game_ids.len(), 2);
    assert!(cols[1].game_ids.contains("steam:570"));

    // Configurações por jogo (FPS fora do escopo, mas preservado).
    let gs = GameSettingsStore::new(db.clone());
    assert_eq!(
        gs.load("steam:570").unwrap(),
        GameSettingsOverride {
            use_game_screen: Some(true),
            screen_mode: Some("2560x1440@144".into()),
            hdr: Some(false),
            audio_device_id: Some("{0.0.0.00000000}.{fone}".into()),
            audio_device_name: Some("Fone".into()),
            fps_limit: Some(60),
            show_fps: Some(true),
            extra_arguments: Some("-dx11".into()),
            working_directory: None,
        }
    );
    let only_fps = gs.load("steam:730").unwrap();
    assert_eq!(only_fps.fps_limit, Some(-1));
    gs.save("steam:730", &only_fps).unwrap();
    assert_eq!(gs.load("steam:730").unwrap(), only_fps);

    // Escolhas.
    let choices = GameChoicesStore::new(db.clone());
    let art = &choices.load_all_art().unwrap()["steam:570"];
    assert_eq!(
        art[&ArtKind::Cover],
        "https://cdn2.steamgriddb.com/grid/a.png"
    );
    assert_eq!(art[&ArtKind::Logo], "file:///C:/Users/a/logo.png");
    let ids = choices.load_ids("steam:570").unwrap();
    assert_eq!(ids["IGDB"], "900001");
    assert_eq!(ids["SteamGridDB"], "");

    // Esquema: v7 em cima, histórico da alpha intacto, cópia de segurança feita.
    let conn = db.open().unwrap();
    assert_eq!(migrator::current_version(&conn).unwrap(), 7);
    let applied: String = conn
        .query_row(
            "SELECT applied_at FROM schema_migrations WHERE version = 1;",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(applied, "2026-05-02T18:30:12.1234567+00:00");
    let backup = Connection::open(with_suffix(&path, ALPHA_BACKUP_SUFFIX)).unwrap();
    assert_eq!(migrator::current_version(&backup).unwrap(), 6);
    for (table, n) in [
        ("manual_games", 3),
        ("game_state", 2),
        ("settings", 4),
        ("collections", 2),
        ("collection_games", 3),
        ("game_settings", 2),
        ("game_art", 2),
        ("game_source_ids", 2),
    ] {
        let sql = format!("SELECT COUNT(*) FROM {table};");
        assert_eq!(count(&conn, &sql), n, "{table}");
        assert_eq!(count(&backup, &sql), n, "backup {table}");
    }
}
