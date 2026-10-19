//! Fontes de download e downloads (DL-001, ADR-0028 §1 e §3), só no banco local. O conteúdo das fontes fica em
//! cache no disco (`downloads::cache`), fora do banco.

use super::{Result, TakoDatabase, format_date, parse_date_or_epoch, require};
use crate::downloads::{Download, DownloadState};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRow {
    pub url: String,
    pub name: String,
    pub items: usize,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct DownloadStore {
    db: TakoDatabase,
}

impl DownloadStore {
    pub fn new(db: TakoDatabase) -> DownloadStore {
        DownloadStore { db }
    }

    pub fn sources(&self) -> Result<Vec<SourceRow>> {
        let conn = self.db.open()?;
        let mut stmt = conn
            .prepare("SELECT url, name, items, updated_at FROM download_sources ORDER BY name;")?;
        let rows = stmt.query_map([], |r| {
            Ok(SourceRow {
                url: r.get(0)?,
                name: r.get(1)?,
                items: r.get::<_, i64>(2)?.max(0) as usize,
                updated_at: parse_date_or_epoch(&r.get::<_, String>(3)?),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn save_source(&self, row: &SourceRow) -> Result<()> {
        require(&row.url, "url")?;
        self.db.open()?.execute(
            "INSERT OR REPLACE INTO download_sources (url, name, items, updated_at) VALUES (?1, ?2, ?3, ?4);",
            (&row.url, &row.name, row.items as i64, format_date(row.updated_at)),
        )?;
        Ok(())
    }

    pub fn remove_source(&self, url: &str) -> Result<()> {
        self.db
            .open()?
            .execute("DELETE FROM download_sources WHERE url = ?1;", [url])?;
        Ok(())
    }

    pub fn downloads(&self) -> Result<Vec<Download>> {
        let conn = self.db.open()?;
        let mut stmt = conn.prepare(
            "SELECT id, title, game_id, source, uri, folder, state, error, done, total FROM downloads ORDER BY id;",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Download::stored(
                r.get::<_, i64>(0)?.max(0) as u64,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get::<_, String>(5)?.into(),
                DownloadState::from_key(&r.get::<_, String>(6)?, &r.get::<_, String>(7)?),
                r.get::<_, i64>(8)?.max(0) as u64,
                r.get::<_, Option<i64>>(9)?.map(|t| t.max(0) as u64),
            ))
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Grava a lista inteira (é curta) numa transação.
    pub fn save_downloads(&self, list: &[Download]) -> Result<()> {
        let mut conn = self.db.open()?;
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM downloads;", [])?;
        for d in list {
            let error = match &d.state {
                DownloadState::Failed(e) => e.as_str(),
                _ => "",
            };
            tx.execute(
                "INSERT INTO downloads (id, title, game_id, source, uri, folder, state, error, done, total)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10);",
                rusqlite::params![
                    d.id as i64,
                    d.title,
                    d.game_id,
                    d.source,
                    d.uri,
                    d.folder.to_string_lossy(),
                    d.state.key(),
                    error,
                    d.done as i64,
                    d.total.map(|t| t as i64),
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::downloads::{Downloads, NewDownload, fake::MemoryDownloader};

    #[test]
    fn sources_and_downloads_survive_a_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let store = DownloadStore::new(TakoDatabase::new(dir.path().join("tako.db")));
        let row = SourceRow {
            url: "https://exemplo.invalid/fonte.json".into(),
            name: "Minha fonte".into(),
            items: 3,
            updated_at: DateTime::UNIX_EPOCH,
        };
        store.save_source(&row).unwrap();
        assert_eq!(store.sources().unwrap(), vec![row.clone()]);

        let mut d = Downloads::new(
            Box::new(MemoryDownloader::default()),
            dir.path().into(),
            vec![],
        );
        d.add(NewDownload {
            title: "Jogo".into(),
            game_id: Some("g1".into()),
            source: row.name.clone(),
            uri: "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567".into(),
        })
        .unwrap();
        store.save_downloads(d.list()).unwrap();
        let back = store.downloads().unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].game_id.as_deref(), Some("g1"));
        // Baixando volta como pausado.
        assert_eq!(back[0].state, DownloadState::Paused);

        store.remove_source(&row.url).unwrap();
        assert!(store.sources().unwrap().is_empty());
    }
}
