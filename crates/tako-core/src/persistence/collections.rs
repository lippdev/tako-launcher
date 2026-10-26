//! Port de `CollectionStore.cs`: coleções do usuário (UX-003); cada mudança grava na hora, numa transação.
//! As regras de nome e a lista em memória são da biblioteca (`CollectionLibrary`).

use super::{Result, TakoDatabase, format_date, parse_date_or_epoch};
use chrono::{DateTime, Utc};
use rusqlite::Params;
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CollectionRow {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub game_ids: BTreeSet<String>,
}

#[derive(Clone)]
pub struct CollectionStore {
    db: TakoDatabase,
}

impl CollectionStore {
    pub fn new(db: TakoDatabase) -> CollectionStore {
        CollectionStore { db }
    }

    /// Em ordem de criação.
    pub fn load_all(&self) -> Result<Vec<CollectionRow>> {
        let conn = self.db.open()?;
        let mut members: HashMap<String, BTreeSet<String>> = HashMap::new();
        let mut stmt = conn.prepare("SELECT collection_id, game_id FROM collection_games;")?;
        let mut rows = stmt.query([])?;
        while let Some(r) = rows.next()? {
            members.entry(r.get(0)?).or_default().insert(r.get(1)?);
        }
        let mut stmt = conn
            .prepare("SELECT id, name, created_at FROM collections ORDER BY created_at, rowid;")?;
        let rows = stmt.query_map([], |r| {
            Ok(CollectionRow {
                id: r.get(0)?,
                name: r.get(1)?,
                created_at: parse_date_or_epoch(&r.get::<_, String>(2)?),
                game_ids: BTreeSet::new(),
            })
        })?;
        let mut result: Vec<CollectionRow> = rows.collect::<rusqlite::Result<_>>()?;
        for c in &mut result {
            c.game_ids = members.remove(&c.id).unwrap_or_default();
        }
        Ok(result)
    }

    pub fn insert(&self, id: &str, name: &str, created_at: DateTime<Utc>) -> Result<()> {
        self.execute(
            "INSERT INTO collections (id, name, created_at) VALUES (?1, ?2, ?3);",
            (id, name, format_date(created_at)),
        )
    }

    pub fn rename(&self, id: &str, name: &str) -> Result<()> {
        self.execute(
            "UPDATE collections SET name = ?2 WHERE id = ?1;",
            (id, name),
        )
    }

    /// Apaga a coleção e a lista de jogos dela (os jogos continuam na biblioteca).
    pub fn delete(&self, id: &str) -> Result<()> {
        let conn = self.db.open()?;
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM collection_games WHERE collection_id = ?1;",
            [id],
        )?;
        tx.execute("DELETE FROM collections WHERE id = ?1;", [id])?;
        tx.commit()?;
        Ok(())
    }

    pub fn add_game(&self, collection_id: &str, game_id: &str, at: DateTime<Utc>) -> Result<()> {
        self.execute(
            "INSERT OR IGNORE INTO collection_games (collection_id, game_id, added_at) VALUES (?1, ?2, ?3);",
            (collection_id, game_id, format_date(at)),
        )
    }

    pub fn remove_game(&self, collection_id: &str, game_id: &str) -> Result<()> {
        self.execute(
            "DELETE FROM collection_games WHERE collection_id = ?1 AND game_id = ?2;",
            (collection_id, game_id),
        )
    }

    fn execute(&self, sql: &str, params: impl Params) -> Result<()> {
        self.db.open()?.execute(sql, params)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::migrator::tests::count;
    use chrono::Duration;

    #[test]
    fn collections_are_created_renamed_filled_and_deleted_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tako.db");
        let store = CollectionStore::new(TakoDatabase::new(&path));
        let t0 = Utc::now();
        store.insert("c:rpg", "RPG", t0).unwrap();
        store.add_game("c:rpg", "steam:570", t0).unwrap();
        store
            .insert("c:corrida", "Corrida", t0 + Duration::seconds(1))
            .unwrap();
        store.add_game("c:corrida", "steam:730", t0).unwrap();
        store.add_game("c:corrida", "steam:570", t0).unwrap();
        store.add_game("c:corrida", "steam:570", t0).unwrap(); // repetido: ignora
        store.remove_game("c:corrida", "steam:730").unwrap();
        store.rename("c:rpg", "Aventura").unwrap();

        let reopened = CollectionStore::new(TakoDatabase::new(&path))
            .load_all()
            .unwrap();
        let names: Vec<_> = reopened.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Aventura", "Corrida"]); // em ordem de criação
        assert_eq!(
            reopened[0].game_ids,
            BTreeSet::from(["steam:570".to_string()])
        );
        assert_eq!(
            reopened[1].game_ids,
            BTreeSet::from(["steam:570".to_string()])
        );
        assert_eq!(
            reopened[0].created_at.timestamp_micros(),
            t0.timestamp_micros()
        );

        store.delete("c:corrida").unwrap();
        let third = CollectionStore::new(TakoDatabase::new(&path))
            .load_all()
            .unwrap();
        assert_eq!(third.len(), 1);
        assert_eq!(third[0].name, "Aventura");
        let conn = TakoDatabase::new(&path).open().unwrap();
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM collection_games;"), 1);
    }
}
