//! Migrações versionadas do `tako.db` (port de `SchemaMigrator.cs`). v1..v6 são as das alphas C#, idênticas.
//! Nunca edite uma migração publicada; acrescente outra.

use super::format_date;
use chrono::Utc;
use rusqlite::{Connection, params};

/// `(versão, SQL)`. O SQL é idempotente quando dá (`IF NOT EXISTS`) para tolerar uma migração interrompida.
pub const MIGRATIONS: &[(u32, &str)] = &[
    (
        1,
        "CREATE TABLE IF NOT EXISTS manual_games (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            path TEXT NOT NULL,
            arguments TEXT,
            working_directory TEXT,
            cover_path TEXT,
            hero_path TEXT,
            added_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS game_state (
            game_id TEXT PRIMARY KEY,
            is_favorite INTEGER NOT NULL DEFAULT 0,
            last_played_at TEXT,
            play_minutes REAL NOT NULL DEFAULT 0
        );",
    ),
    (
        2,
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
    ),
    // ADD COLUMN não aceita IF NOT EXISTS, mas roda na transação que registra a versão.
    (3, "ALTER TABLE manual_games ADD COLUMN track_by TEXT;"),
    (
        4,
        "CREATE TABLE IF NOT EXISTS collections (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS collection_games (
            collection_id TEXT NOT NULL,
            game_id TEXT NOT NULL,
            added_at TEXT NOT NULL,
            PRIMARY KEY (collection_id, game_id)
        );
        CREATE TABLE IF NOT EXISTS game_settings (
            game_id TEXT PRIMARY KEY,
            use_game_screen INTEGER,
            screen_mode TEXT,
            hdr INTEGER,
            audio_device_id TEXT,
            audio_device_name TEXT,
            fps_limit INTEGER,
            show_fps INTEGER,
            extra_arguments TEXT
        );",
    ),
    (5, "ALTER TABLE manual_games ADD COLUMN platform TEXT;"),
    (
        6,
        "CREATE TABLE IF NOT EXISTS game_art (
            game_id TEXT NOT NULL,
            kind TEXT NOT NULL,
            url TEXT NOT NULL,
            PRIMARY KEY (game_id, kind)
        );
        CREATE TABLE IF NOT EXISTS game_source_ids (
            game_id TEXT NOT NULL,
            source TEXT NOT NULL,
            source_id TEXT NOT NULL,
            PRIMARY KEY (game_id, source)
        );",
    ),
    // Primeira versão só do Rust: pasta de trabalho por jogo (PRD §6).
    (
        7,
        "ALTER TABLE game_settings ADD COLUMN working_directory TEXT;",
    ),
    // Status do jogo (LIB-001): jogando, zerado, pausado, abandonado, quero jogar.
    (8, "ALTER TABLE game_state ADD COLUMN status TEXT;"),
    // Histórico de sessões (SESSION-002); a biblioteca guarda só as últimas por jogo.
    (
        9,
        "CREATE TABLE IF NOT EXISTS play_sessions (
            game_id TEXT NOT NULL,
            started_at TEXT NOT NULL,
            seconds INTEGER NOT NULL
        );",
    ),
    // Tags do usuário (LIB-002), por jogo de qualquer loja.
    (
        10,
        "CREATE TABLE IF NOT EXISTS game_tags (
            game_id TEXT NOT NULL,
            tag TEXT NOT NULL,
            PRIMARY KEY (game_id, tag)
        );",
    ),
    // Variáveis de ambiente e comandos antes e depois de jogar, com o texto permitido (SESSION-004).
    (
        11,
        "ALTER TABLE game_settings ADD COLUMN environment TEXT;
         ALTER TABLE game_settings ADD COLUMN before_command TEXT;
         ALTER TABLE game_settings ADD COLUMN after_command TEXT;
         ALTER TABLE game_settings ADD COLUMN approved_commands TEXT;",
    ),
    // Tela própria de cada jogo (PERF-002).
    (
        12,
        "ALTER TABLE game_settings ADD COLUMN screen_id TEXT;
         ALTER TABLE game_settings ADD COLUMN screen_name TEXT;",
    ),
    // VRR de cada jogo (PERF-002).
    (13, "ALTER TABLE game_settings ADD COLUMN vrr INTEGER;"),
    // Ferramentas abertas junto do jogo e se fecham no fim (EXT-001).
    (
        14,
        "ALTER TABLE game_settings ADD COLUMN tools TEXT;
         ALTER TABLE game_settings ADD COLUMN close_tools INTEGER;",
    ),
];

/// Última versão das alphas C#: banco abaixo da v7 veio delas (ou de antes).
pub const LAST_ALPHA_VERSION: u32 = 6;

pub fn latest_version() -> u32 {
    MIGRATIONS.last().map_or(0, |m| m.0)
}

/// Versão atual do banco (0 quando ainda não foi migrado).
pub fn current_version(conn: &Connection) -> rusqlite::Result<u32> {
    ensure_version_table(conn)?;
    conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations;",
        [],
        |r| r.get(0),
    )
}

/// Aplica as migrações pendentes, em ordem, até `target` (padrão: a mais recente), cada uma na sua transação.
pub fn migrate(conn: &Connection, target: Option<u32>) -> rusqlite::Result<()> {
    let current = current_version(conn)?;
    let target = target.unwrap_or_else(latest_version);
    for (version, sql) in MIGRATIONS
        .iter()
        .filter(|(v, _)| *v > current && *v <= target)
    {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (?1, ?2);",
            params![version, format_date(Utc::now())],
        )?;
        tx.commit()?;
    }
    Ok(())
}

fn ensure_version_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);",
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn table_exists(conn: &Connection, table: &str) -> bool {
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1;",
            [table],
            |r| r.get::<_, i64>(0),
        )
        .unwrap()
            == 1
    }

    pub(crate) fn count(conn: &Connection, sql: &str) -> i64 {
        conn.query_row(sql, [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn migrator_upgrades_a_v1_database_keeping_its_data() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn, Some(1)).unwrap();
        assert_eq!(current_version(&conn).unwrap(), 1);
        conn.execute_batch("INSERT INTO game_state (game_id, is_favorite, play_minutes) VALUES ('steam:570', 1, 12.5);").unwrap();
        assert!(!table_exists(&conn, "settings"));

        migrate(&conn, None).unwrap();

        assert_eq!(current_version(&conn).unwrap(), latest_version());
        assert!(table_exists(&conn, "settings"));
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM game_state;"), 1);

        // Idempotente: rodar de novo não reaplica nada.
        migrate(&conn, None).unwrap();
        assert_eq!(
            count(&conn, "SELECT COUNT(*) FROM schema_migrations;"),
            MIGRATIONS.len() as i64
        );
    }

    #[test]
    fn migrations_are_in_strictly_increasing_order() {
        let versions: Vec<u32> = MIGRATIONS.iter().map(|m| m.0).collect();
        assert_eq!(versions, (1..=MIGRATIONS.len() as u32).collect::<Vec<_>>());
    }

    #[test]
    fn migration_4_adds_collections_and_game_settings_to_a_v3_database() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn, Some(3)).unwrap();
        conn.execute_batch("INSERT INTO manual_games (id, title, path, added_at, track_by) VALUES ('manual:a', 'A', 'C:/a.exe', '2026-10-15T00:00:00Z', 'Process');").unwrap();
        assert!(!table_exists(&conn, "collections"));

        migrate(&conn, None).unwrap();

        assert!(current_version(&conn).unwrap() >= 4);
        for t in ["collections", "collection_games", "game_settings"] {
            assert!(table_exists(&conn, t), "{t}");
        }
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM manual_games;"), 1);
    }

    #[test]
    fn migration_7_adds_the_working_directory_to_game_settings() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn, Some(LAST_ALPHA_VERSION)).unwrap();
        migrate(&conn, None).unwrap();
        conn.execute_batch(
            "INSERT INTO game_settings (game_id, working_directory) VALUES ('steam:1', 'D:/x');",
        )
        .unwrap();
    }
}
