//! Port de `SqliteLibraryStore.cs` (e da leitura de `Library/LibraryStore.cs`, o `library.json` antigo).
//!
//! Na primeira abertura importa o `library.json` e o renomeia para `library.json.importado`.

use super::database::{CORRUPTED_SUFFIX, with_suffix};
use super::{Result, TakoDatabase, format_date, parse_date, parse_date_or_epoch};
use crate::library::GameTrackingStrategy;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const IMPORTED_SUFFIX: &str = ".importado";

/// O `library.json` é pequeno; acima disto é lixo e conta como corrompido.
const MAX_LEGACY_JSON: u64 = 32 * 1024 * 1024;

/// Linha de `manual_games` (jogo adicionado pelo usuário). A conversão para `Game` é da biblioteca.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ManualGameRow {
    pub id: String,
    pub title: String,
    /// Executável, atalho (.lnk/.url) ou URI.
    pub path: String,
    pub arguments: Option<String>,
    pub working_directory: Option<String>,
    pub cover_path: Option<String>,
    pub hero_path: Option<String>,
    pub added_at: DateTime<Utc>,
    /// `None` = automático.
    pub track_by: Option<GameTrackingStrategy>,
    /// Sistema da ROM; `None` = PC.
    pub platform: Option<String>,
}

/// Linha de `game_state` (estado do usuário por jogo, de qualquer loja).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GameStateRow {
    pub is_favorite: bool,
    pub last_played_at: Option<DateTime<Utc>>,
    pub play_minutes: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LibraryData {
    pub manual: Vec<ManualGameRow>,
    pub state: HashMap<String, GameStateRow>,
}

#[derive(Clone)]
pub struct LibraryStore {
    db: TakoDatabase,
    legacy_json: Option<PathBuf>,
}

impl LibraryStore {
    pub fn new(db: TakoDatabase, legacy_json: Option<PathBuf>) -> LibraryStore {
        LibraryStore { db, legacy_json }
    }

    pub fn load(&self) -> Result<LibraryData> {
        let conn = self.db.open()?;
        if let Some(json) = self.legacy_json.as_deref().filter(|p| p.exists()) {
            import_legacy_json(&conn, json)?;
        }
        Ok(LibraryData {
            manual: read_manual(&conn)?,
            state: read_state(&conn)?,
        })
    }

    /// Regrava as duas tabelas numa transação: ou tudo é salvo, ou nada muda.
    pub fn save(&self, data: &LibraryData) -> Result<()> {
        let conn = self.db.open()?;
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch("DELETE FROM manual_games; DELETE FROM game_state;")?;
        write(&tx, data, "INSERT OR REPLACE")?;
        tx.commit()?;
        Ok(())
    }
}

fn import_legacy_json(conn: &Connection, json: &Path) -> Result<()> {
    let legacy = read_legacy_json(json)?;
    let tx = conn.unchecked_transaction()?;
    // INSERT OR IGNORE: reimportar (se a renomeação falhar) nunca sobrescreve dados mais novos do banco.
    write(&tx, &legacy, "INSERT OR IGNORE")?;
    tx.commit()?;
    std::fs::rename(json, with_suffix(json, IMPORTED_SUFFIX))?;
    tracing::info!(path = %json.display(), "library.json importado");
    Ok(())
}

/// Arquivo corrompido: guarda uma cópia `.corrompido` e devolve vazio, como o `LibraryStore` do C#.
fn read_legacy_json(path: &Path) -> Result<LibraryData> {
    let parsed = if std::fs::metadata(path)?.len() > MAX_LEGACY_JSON {
        None
    } else {
        serde_json::from_slice::<LegacyDocument>(&std::fs::read(path)?).ok()
    };
    let Some(doc) = parsed else {
        tracing::warn!(path = %path.display(), "library.json ilegível");
        std::fs::copy(path, with_suffix(path, CORRUPTED_SUFFIX))?;
        return Ok(LibraryData::default());
    };
    Ok(LibraryData {
        manual: doc
            .manual
            .unwrap_or_default()
            .into_iter()
            .map(|m| ManualGameRow {
                id: m.id,
                title: m.title,
                path: m.path,
                arguments: m.arguments,
                working_directory: m.working_directory,
                cover_path: m.cover_path,
                hero_path: m.hero_path,
                added_at: m
                    .added_at
                    .as_deref()
                    .map_or(DateTime::UNIX_EPOCH, parse_date_or_epoch),
                track_by: m.track_by.as_ref().and_then(legacy_strategy),
                platform: m.platform,
            })
            .collect(),
        state: doc
            .state
            .unwrap_or_default()
            .into_iter()
            .map(|(id, s)| {
                let row = GameStateRow {
                    is_favorite: s.is_favorite,
                    last_played_at: s.last_played_at.as_deref().and_then(parse_date),
                    play_minutes: s.play_minutes,
                };
                (id, row)
            })
            .collect(),
    })
}

/// O System.Text.Json grava o enum como número (ordem da declaração); aceita o nome também.
fn legacy_strategy(value: &serde_json::Value) -> Option<GameTrackingStrategy> {
    use GameTrackingStrategy::*;
    match value {
        serde_json::Value::Number(n) => [None, Process, InstallFolder, SteamApp]
            .get(usize::try_from(n.as_u64()?).ok()?)
            .copied(),
        serde_json::Value::String(s) => GameTrackingStrategy::from_key(s),
        _ => Option::None,
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct LegacyDocument {
    manual: Option<Vec<LegacyManual>>,
    state: Option<HashMap<String, LegacyState>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct LegacyManual {
    id: String,
    title: String,
    path: String,
    arguments: Option<String>,
    working_directory: Option<String>,
    track_by: Option<serde_json::Value>,
    cover_path: Option<String>,
    hero_path: Option<String>,
    added_at: Option<String>,
    platform: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct LegacyState {
    #[serde(default)]
    is_favorite: bool,
    last_played_at: Option<String>,
    #[serde(default)]
    play_minutes: f64,
}

fn write(conn: &Connection, data: &LibraryData, verb: &str) -> Result<()> {
    let mut manual = conn.prepare(&format!(
        "{verb} INTO manual_games (id, title, path, arguments, working_directory, cover_path, hero_path, added_at, track_by, platform)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10);"
    ))?;
    for m in &data.manual {
        manual.execute(params![
            m.id,
            m.title,
            m.path,
            m.arguments,
            m.working_directory,
            m.cover_path,
            m.hero_path,
            format_date(m.added_at),
            m.track_by.map(GameTrackingStrategy::key),
            m.platform,
        ])?;
    }
    let mut state = conn.prepare(&format!(
        "{verb} INTO game_state (game_id, is_favorite, last_played_at, play_minutes) VALUES (?1, ?2, ?3, ?4);"
    ))?;
    for (id, s) in &data.state {
        state.execute(params![
            id,
            s.is_favorite,
            s.last_played_at.map(format_date),
            s.play_minutes,
        ])?;
    }
    Ok(())
}

fn read_manual(conn: &Connection) -> Result<Vec<ManualGameRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, path, arguments, working_directory, cover_path, hero_path, added_at, track_by, platform
         FROM manual_games ORDER BY rowid;",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(ManualGameRow {
            id: r.get(0)?,
            title: r.get(1)?,
            path: r.get(2)?,
            arguments: r.get(3)?,
            working_directory: r.get(4)?,
            cover_path: r.get(5)?,
            hero_path: r.get(6)?,
            added_at: parse_date_or_epoch(&r.get::<_, String>(7)?),
            // Valor desconhecido (versão mais nova gravou outro nome) volta ao automático.
            track_by: r
                .get::<_, Option<String>>(8)?
                .as_deref()
                .and_then(GameTrackingStrategy::from_key),
            platform: r.get(9)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn read_state(conn: &Connection) -> Result<HashMap<String, GameStateRow>> {
    let mut stmt =
        conn.prepare("SELECT game_id, is_favorite, last_played_at, play_minutes FROM game_state;")?;
    let rows = stmt.query_map([], |r| {
        let row = GameStateRow {
            is_favorite: r.get::<_, i64>(1)? != 0,
            last_played_at: r
                .get::<_, Option<String>>(2)?
                .as_deref()
                .and_then(parse_date),
            play_minutes: r.get(3)?,
        };
        Ok((r.get(0)?, row))
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, TimeZone};

    fn added() -> DateTime<Utc> {
        FixedOffset::west_opt(3 * 3600)
            .unwrap()
            .with_ymd_and_hms(2026, 10, 15, 12, 0, 0)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn manual(id: &str, title: &str, path: &str) -> ManualGameRow {
        ManualGameRow {
            id: id.into(),
            title: title.into(),
            path: path.into(),
            added_at: added(),
            ..Default::default()
        }
    }

    #[test]
    fn sqlite_store_round_trips_manual_games_and_state() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("tako.db");
        let store = LibraryStore::new(TakoDatabase::new(&db), None);
        let mut data = store.load().unwrap();
        data.manual.push(ManualGameRow {
            arguments: Some("-x".into()),
            track_by: Some(GameTrackingStrategy::InstallFolder),
            ..manual("manual:a", "Jogo A", "C:/a.exe")
        });
        data.manual
            .push(manual("manual:b", "Jogo B", "steam://run/1"));
        data.manual.push(ManualGameRow {
            platform: Some("snes".into()),
            ..manual("manual:rom", "Lanterna", "D:/roms/lanterna.sfc")
        });
        let state = GameStateRow {
            is_favorite: true,
            last_played_at: Some(added()),
            play_minutes: 42.5,
        };
        data.state.insert("steam:570".into(), state.clone());
        store.save(&data).unwrap();

        let mut reopened = LibraryStore::new(TakoDatabase::new(&db), None)
            .load()
            .unwrap();
        let ids: Vec<_> = reopened.manual.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["manual:a", "manual:b", "manual:rom"]);
        assert_eq!(reopened.manual[0].platform, None);
        assert_eq!(reopened.manual[2].platform.as_deref(), Some("snes"));
        assert_eq!(reopened.manual[0].arguments.as_deref(), Some("-x"));
        assert_eq!(reopened.manual[1].arguments, None);
        assert_eq!(
            reopened.manual[0].track_by,
            Some(GameTrackingStrategy::InstallFolder)
        );
        assert_eq!(reopened.manual[1].track_by, None);
        assert_eq!(reopened.manual[0].added_at, added());
        assert_eq!(reopened.state["steam:570"], state);

        reopened.manual.remove(0);
        reopened.state.clear();
        store.save(&reopened).unwrap();
        let third = LibraryStore::new(TakoDatabase::new(&db), None)
            .load()
            .unwrap();
        let ids: Vec<_> = third.manual.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["manual:b", "manual:rom"]);
        assert!(third.state.is_empty());
    }

    /// Como o `LibraryStore.Save` do C# grava (System.Text.Json, indentado, enum como número).
    const LEGACY_JSON: &str = r#"{
  "Version": 1,
  "Manual": [
    {
      "Id": "manual:velho",
      "Title": "Velho",
      "Path": "C:/velho.exe",
      "Arguments": null,
      "WorkingDirectory": "C:/",
      "TrackBy": 1,
      "CoverPath": null,
      "HeroPath": null,
      "AddedAt": "1970-01-01T00:00:00+00:00",
      "Platform": null
    }
  ],
  "State": {
    "steam:10": {
      "IsFavorite": true,
      "LastPlayedAt": null,
      "PlayMinutes": 3
    }
  }
}"#;

    #[test]
    fn first_run_imports_library_json_and_renames_it() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("tako.db");
        let json = dir.path().join("library.json");
        std::fs::write(&json, LEGACY_JSON).unwrap();

        let data = LibraryStore::new(TakoDatabase::new(&db), Some(json.clone()))
            .load()
            .unwrap();

        assert_eq!(data.manual.len(), 1);
        assert_eq!(data.manual[0].title, "Velho");
        assert_eq!(data.manual[0].track_by, Some(GameTrackingStrategy::Process));
        assert_eq!(data.manual[0].added_at, DateTime::UNIX_EPOCH);
        assert!(data.state["steam:10"].is_favorite);
        assert!(!json.exists());
        assert!(with_suffix(&json, IMPORTED_SUFFIX).exists());

        // Segunda abertura: nada a importar, os dados continuam no banco.
        let again = LibraryStore::new(TakoDatabase::new(&db), Some(json))
            .load()
            .unwrap();
        assert_eq!(again.manual.len(), 1);
    }

    #[test]
    fn reimport_never_overwrites_newer_data_in_the_database() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("tako.db");
        let json = dir.path().join("library.json");
        let store = LibraryStore::new(TakoDatabase::new(&db), None);
        let mut data = store.load().unwrap();
        data.state.insert(
            "steam:10".into(),
            GameStateRow {
                is_favorite: false,
                play_minutes: 99.0,
                ..Default::default()
            },
        );
        store.save(&data).unwrap();
        std::fs::write(
            &json,
            LEGACY_JSON.replace("\"PlayMinutes\": 3", "\"PlayMinutes\": 1"),
        )
        .unwrap();

        let importing = LibraryStore::new(TakoDatabase::new(&db), Some(json))
            .load()
            .unwrap();

        assert_eq!(importing.state["steam:10"].play_minutes, 99.0);
    }

    #[test]
    fn corrupted_legacy_json_is_still_moved_aside() {
        for broken in [
            "{ isto não é json",
            r#"{"Manual":[{"Title":"sem id"}]}"#,
            "[]",
        ] {
            let dir = tempfile::tempdir().unwrap();
            let json = dir.path().join("library.json");
            std::fs::write(&json, broken).unwrap();

            let data = LibraryStore::new(
                TakoDatabase::new(dir.path().join("tako.db")),
                Some(json.clone()),
            )
            .load()
            .unwrap();

            assert!(data.manual.is_empty());
            assert!(with_suffix(&json, CORRUPTED_SUFFIX).exists());
            assert!(with_suffix(&json, IMPORTED_SUFFIX).exists());
        }
    }

    #[test]
    fn unknown_strategy_falls_back_to_automatic() {
        assert_eq!(legacy_strategy(&serde_json::json!(9)), None);
        assert_eq!(legacy_strategy(&serde_json::json!(-1)), None);
        assert_eq!(
            legacy_strategy(&serde_json::json!("SteamApp")),
            Some(GameTrackingStrategy::SteamApp)
        );
    }
}
