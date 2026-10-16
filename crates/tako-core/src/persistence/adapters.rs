//! Liga os armazenamentos SQLite aos contratos da biblioteca (`library::LibraryStore` e `CollectionStore`).

use super::collections::{CollectionRow, CollectionStore as SqlCollections};
use super::library::{
    GameStateRow, LibraryData as SqlData, LibraryStore as SqlLibrary, ManualGameRow,
};
use crate::library::{self, CollectionInfo, GameStateRecord, LibraryData, ManualGameRecord};
use chrono::{DateTime, Utc};

impl library::LibraryStore for SqlLibrary {
    fn load(&mut self) -> LibraryData {
        match SqlLibrary::load(self) {
            Ok(d) => LibraryData {
                manual: d.manual.into_iter().map(to_record).collect(),
                state: d.state.into_iter().map(|(k, s)| (k, to_state(s))).collect(),
            },
            Err(e) => {
                tracing::error!("não foi possível ler a biblioteca: {e}");
                LibraryData::default()
            }
        }
    }

    fn save(&mut self, data: &LibraryData) -> Result<(), String> {
        let rows = SqlData {
            manual: data.manual.iter().cloned().map(to_row).collect(),
            state: data
                .state
                .iter()
                .map(|(k, s)| {
                    (
                        k.clone(),
                        GameStateRow {
                            is_favorite: s.is_favorite,
                            last_played_at: s.last_played_at,
                            play_minutes: s.play_minutes,
                            sessions: s.sessions.clone(),
                        },
                    )
                })
                .collect(),
        };
        SqlLibrary::save(self, &rows).map_err(|e| e.to_string())
    }
}

fn to_record(r: ManualGameRow) -> ManualGameRecord {
    ManualGameRecord {
        id: r.id,
        title: r.title,
        path: r.path,
        arguments: r.arguments,
        working_directory: r.working_directory,
        track_by: r.track_by,
        cover_path: r.cover_path,
        hero_path: r.hero_path,
        added_at: r.added_at,
        platform: r.platform,
    }
}

fn to_row(r: ManualGameRecord) -> ManualGameRow {
    ManualGameRow {
        id: r.id,
        title: r.title,
        path: r.path,
        arguments: r.arguments,
        working_directory: r.working_directory,
        cover_path: r.cover_path,
        hero_path: r.hero_path,
        added_at: r.added_at,
        track_by: r.track_by,
        platform: r.platform,
    }
}

fn to_state(s: GameStateRow) -> GameStateRecord {
    GameStateRecord {
        is_favorite: s.is_favorite,
        last_played_at: s.last_played_at,
        play_minutes: s.play_minutes,
        sessions: s.sessions,
    }
}

impl library::CollectionStore for SqlCollections {
    fn load_all(&mut self) -> Result<Vec<CollectionInfo>, String> {
        let rows: Vec<CollectionRow> = SqlCollections::load_all(self).map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|r| CollectionInfo {
                id: r.id,
                name: r.name,
                created_at: r.created_at,
                game_ids: r.game_ids,
            })
            .collect())
    }
    fn insert(&mut self, c: &CollectionInfo) -> Result<(), String> {
        SqlCollections::insert(self, &c.id, &c.name, c.created_at).map_err(|e| e.to_string())
    }
    fn rename(&mut self, id: &str, name: &str) -> Result<(), String> {
        SqlCollections::rename(self, id, name).map_err(|e| e.to_string())
    }
    fn delete(&mut self, id: &str) -> Result<(), String> {
        SqlCollections::delete(self, id).map_err(|e| e.to_string())
    }
    fn add_game(
        &mut self,
        collection_id: &str,
        game_id: &str,
        at: DateTime<Utc>,
    ) -> Result<(), String> {
        SqlCollections::add_game(self, collection_id, game_id, at).map_err(|e| e.to_string())
    }
    fn remove_game(&mut self, collection_id: &str, game_id: &str) -> Result<(), String> {
        SqlCollections::remove_game(self, collection_id, game_id).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::TakoDatabase;

    #[test]
    fn round_trips_through_sqlite() {
        let dir = tempfile::tempdir().unwrap();
        let db = TakoDatabase::new(dir.path().join("tako.db"));
        let mut lib = SqlLibrary::new(db.clone(), None);
        let mut data = LibraryData::default();
        data.manual.push(ManualGameRecord {
            id: "manual:a".into(),
            title: "A".into(),
            path: "/games/a".into(),
            arguments: None,
            working_directory: None,
            track_by: None,
            cover_path: None,
            hero_path: None,
            added_at: Utc::now(),
            platform: None,
        });
        data.state.insert(
            "steam:1".into(),
            GameStateRecord {
                is_favorite: true,
                last_played_at: None,
                play_minutes: 3.0,
                ..Default::default()
            },
        );
        library::LibraryStore::save(&mut lib, &data).unwrap();
        let back = library::LibraryStore::load(&mut lib);
        assert_eq!(back.manual[0].title, "A");
        assert!(back.state["steam:1"].is_favorite);

        let mut cols = SqlCollections::new(db);
        let c = CollectionInfo {
            id: "c1".into(),
            name: "RPG".into(),
            created_at: Utc::now(),
            game_ids: Default::default(),
        };
        library::CollectionStore::insert(&mut cols, &c).unwrap();
        library::CollectionStore::add_game(&mut cols, "c1", "steam:1", Utc::now()).unwrap();
        assert!(library::CollectionStore::load_all(&mut cols).unwrap()[0].contains("steam:1"));
    }
}
