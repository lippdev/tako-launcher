//! Port de `GameChoicesStore.cs` (ADR-0023): a imagem fixada por tipo e o id fixado por fonte. Ficam fora do cache
//! (limpar o cache de imagens ou de metadados não as apaga).

use super::{Result, TakoDatabase, require};
use crate::library::ArtKind;
use std::collections::{BTreeMap, HashMap};

#[derive(Clone)]
pub struct GameChoicesStore {
    db: TakoDatabase,
}

impl GameChoicesStore {
    pub fn new(db: TakoDatabase) -> GameChoicesStore {
        GameChoicesStore { db }
    }

    /// Todas as imagens fixadas, numa consulta só. Tipo desconhecido ou URL sem esquema ficam de fora.
    pub fn load_all_art(&self) -> Result<HashMap<String, BTreeMap<ArtKind, String>>> {
        let conn = self.db.open()?;
        let mut stmt = conn.prepare("SELECT game_id, kind, url FROM game_art;")?;
        let mut rows = stmt.query([])?;
        let mut result: HashMap<String, BTreeMap<ArtKind, String>> = HashMap::new();
        while let Some(r) = rows.next()? {
            let kind = ArtKind::from_key(&r.get::<_, String>(1)?);
            let url: String = r.get(2)?;
            if let Some(kind) = kind.filter(|_| is_absolute_uri(&url)) {
                result.entry(r.get(0)?).or_default().insert(kind, url);
            }
        }
        Ok(result)
    }

    /// Fixa a imagem do tipo; `None` volta para a automática.
    pub fn set_art(&self, game_id: &str, kind: ArtKind, url: Option<&str>) -> Result<()> {
        require(game_id, "game_id")?;
        let conn = self.db.open()?;
        match url {
            None => conn.execute(
                "DELETE FROM game_art WHERE game_id = ?1 AND kind = ?2;",
                (game_id, kind.key()),
            )?,
            Some(url) => conn.execute(
                "INSERT OR REPLACE INTO game_art (game_id, kind, url) VALUES (?1, ?2, ?3);",
                (game_id, kind.key(), url.trim()),
            )?,
        };
        Ok(())
    }

    /// Ids fixados de todos os jogos (uma consulta só, para a rodada de metadados).
    pub fn load_all_ids(&self) -> Result<HashMap<String, HashMap<String, String>>> {
        let conn = self.db.open()?;
        let mut stmt = conn.prepare("SELECT game_id, source, source_id FROM game_source_ids;")?;
        let mut rows = stmt.query([])?;
        let mut result: HashMap<String, HashMap<String, String>> = HashMap::new();
        while let Some(r) = rows.next()? {
            result
                .entry(r.get(0)?)
                .or_default()
                .insert(r.get(1)?, r.get(2)?);
        }
        Ok(result)
    }

    /// Ids fixados por fonte; id vazio = "nenhum" (a fonte fica de fora deste jogo).
    pub fn load_ids(&self, game_id: &str) -> Result<HashMap<String, String>> {
        require(game_id, "game_id")?;
        let conn = self.db.open()?;
        let mut stmt =
            conn.prepare("SELECT source, source_id FROM game_source_ids WHERE game_id = ?1;")?;
        let rows = stmt.query_map([game_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Fixa o id na fonte (vazio = "nenhum"); `None` volta para a correspondência automática.
    pub fn set_id(&self, game_id: &str, source: &str, source_id: Option<&str>) -> Result<()> {
        require(game_id, "game_id")?;
        require(source, "source")?;
        let conn = self.db.open()?;
        match source_id {
            None => conn.execute(
                "DELETE FROM game_source_ids WHERE game_id = ?1 AND source = ?2;",
                (game_id, source),
            )?,
            Some(id) => conn.execute(
                "INSERT OR REPLACE INTO game_source_ids (game_id, source, source_id) VALUES (?1, ?2, ?3);",
                (game_id, source, id.trim()),
            )?,
        };
        Ok(())
    }
}

/// O C# só aceitava `Uri` absoluta: `esquema:` com letra inicial.
fn is_absolute_uri(url: &str) -> bool {
    url.split_once(':').is_some_and(|(scheme, _)| {
        scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choices_are_saved_in_the_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tako.db");
        let store = GameChoicesStore::new(TakoDatabase::new(&path));
        store
            .set_art(
                "epic:1",
                ArtKind::Cover,
                Some("https://cdn2.steamgriddb.com/grid/a.png"),
            )
            .unwrap();
        store
            .set_art(
                "epic:1",
                ArtKind::Logo,
                Some("https://cdn2.steamgriddb.com/logo/l.png"),
            )
            .unwrap();
        store.set_art("epic:1", ArtKind::Logo, None).unwrap();
        store.set_id("epic:1", "IGDB", Some("900001")).unwrap();
        store.set_id("epic:1", "SteamGridDB", Some("")).unwrap();
        store.set_id("epic:1", "Steam", Some("1")).unwrap();
        store.set_id("epic:1", "Steam", None).unwrap();

        let reopened = GameChoicesStore::new(TakoDatabase::new(&path));
        let art = &reopened.load_all_art().unwrap()["epic:1"];
        assert_eq!(art.keys().copied().collect::<Vec<_>>(), [ArtKind::Cover]);
        let expected = HashMap::from([
            ("IGDB".to_string(), "900001".to_string()),
            ("SteamGridDB".to_string(), String::new()),
        ]);
        assert_eq!(reopened.load_ids("epic:1").unwrap(), expected);
        assert_eq!(reopened.load_all_ids().unwrap()["epic:1"], expected);
        assert!(store.set_id("", "IGDB", None).is_err());
    }

    #[test]
    fn unreadable_art_rows_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let db = TakoDatabase::new(dir.path().join("tako.db"));
        db.open()
            .unwrap()
            .execute_batch(
                "INSERT INTO game_art VALUES ('a', 'banner', 'https://x/y.png');
                 INSERT INTO game_art VALUES ('a', 'cover', 'sem esquema');
                 INSERT INTO game_art VALUES ('b', 'hero', 'file:///C:/art.png');",
            )
            .unwrap();
        let art = GameChoicesStore::new(db).load_all_art().unwrap();
        assert!(!art.contains_key("a"));
        assert_eq!(art["b"][&ArtKind::Hero], "file:///C:/art.png");
    }
}
