//! Port de `TakoDatabase.cs`: entrega conexões já migradas. Arquivo corrompido não derruba o launcher: é renomeado
//! para `.corrompido` e o banco recomeça vazio.

use super::{PersistenceError, Result, alpha_migration, migrator};
use rusqlite::{Connection, ErrorCode, ffi};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub const CORRUPTED_SUFFIX: &str = ".corrompido";

/// Banco local (`tako.db`). Barato de clonar: os clones dividem o mesmo estado.
#[derive(Clone)]
pub struct TakoDatabase(Arc<Inner>);

struct Inner {
    path: PathBuf,
    ready: Mutex<bool>,
    recovered: AtomicBool,
}

impl TakoDatabase {
    pub fn new(path: impl Into<PathBuf>) -> TakoDatabase {
        TakoDatabase(Arc::new(Inner {
            path: path.into(),
            ready: Mutex::new(false),
            recovered: AtomicBool::new(false),
        }))
    }

    pub fn path(&self) -> &Path {
        &self.0.path
    }

    /// Verdadeiro quando o arquivo estava corrompido e foi trocado por um banco vazio nesta execução.
    pub fn recovered_from_corruption(&self) -> bool {
        self.0.recovered.load(Ordering::Relaxed)
    }

    /// Conexão aberta com o esquema na versão mais recente.
    pub fn open(&self) -> Result<Connection> {
        self.ensure_ready()?;
        Ok(Connection::open(&self.0.path)?)
    }

    fn ensure_ready(&self) -> Result<()> {
        let mut ready = self.0.ready.lock().unwrap_or_else(|e| e.into_inner());
        if *ready {
            return Ok(());
        }
        match self.initialize() {
            Err(e) if is_corrupt(&e) => {
                tracing::warn!(path = %self.0.path.display(), "tako.db corrompido; recomeçando vazio");
                self.quarantine()?;
                self.initialize()?;
            }
            other => other?,
        }
        *ready = true;
        Ok(())
    }

    fn initialize(&self) -> Result<()> {
        if let Some(dir) = self.0.path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(&self.0.path)?;
        // Página danificada nem sempre falha ao abrir; a verificação rápida percorre o arquivo (que é pequeno).
        let check: String = conn.query_row("PRAGMA quick_check;", [], |r| r.get(0))?;
        if check != "ok" {
            return Err(rusqlite::Error::SqliteFailure(
                ffi::Error::new(ffi::SQLITE_CORRUPT),
                Some(format!("quick_check: {check}")),
            )
            .into());
        }
        alpha_migration::backup_alpha_database(&conn, &self.0.path)?;
        migrator::migrate(&conn, None)?;
        Ok(())
    }

    fn quarantine(&self) -> Result<()> {
        let path = &self.0.path;
        std::fs::rename(path, with_suffix(path, CORRUPTED_SUFFIX))?;
        for suffix in ["-journal", "-wal", "-shm"] {
            match std::fs::remove_file(with_suffix(path, suffix)) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.into()),
                _ => {}
            }
        }
        self.0.recovered.store(true, Ordering::Relaxed);
        Ok(())
    }
}

fn is_corrupt(e: &PersistenceError) -> bool {
    matches!(
        e,
        PersistenceError::Sqlite(rusqlite::Error::SqliteFailure(f, _))
            if matches!(f.code, ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase)
    )
}

/// `tako.db` + `.corrompido` (não troca a extensão).
pub(crate) fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::{GameStateRow, LibraryStore};

    #[test]
    fn corrupted_database_is_renamed_and_starts_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tako.db");
        std::fs::write(
            &path,
            "isto não é um banco SQLite, só texto qualquer com tamanho suficiente para o cabeçalho",
        )
        .unwrap();
        std::fs::write(with_suffix(&path, "-journal"), "lixo").unwrap();
        let db = TakoDatabase::new(&path);
        let store = LibraryStore::new(db.clone(), None);

        let mut data = store.load().unwrap();

        assert!(data.manual.is_empty());
        assert!(db.recovered_from_corruption());
        assert!(with_suffix(&path, CORRUPTED_SUFFIX).exists());
        assert!(!with_suffix(&path, "-journal").exists());
        data.state.insert(
            "x".into(),
            GameStateRow {
                is_favorite: true,
                ..Default::default()
            },
        );
        store.save(&data).unwrap();
        let reopened = LibraryStore::new(TakoDatabase::new(&path), None)
            .load()
            .unwrap();
        assert!(reopened.state["x"].is_favorite);
    }

    #[test]
    fn open_creates_the_folder_and_the_latest_schema() {
        let dir = tempfile::tempdir().unwrap();
        let db = TakoDatabase::new(dir.path().join("sub").join("tako.db"));
        let conn = db.open().unwrap();
        assert_eq!(
            migrator::current_version(&conn).unwrap(),
            migrator::latest_version()
        );
        assert!(!db.recovered_from_corruption());
    }
}
