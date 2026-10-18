//! Amazon Games (port de `AmazonLibrarySource`): o banco SQLite do cliente
//! (`%LOCALAPPDATA%\Amazon Games\Data\Games\Sql\GameInstallInfo.sqlite`, tabela `DbSet`), aberto só
//! para leitura. Abre sempre por `amazon-games://play/<id>` (login e DRM ficam com o cliente). Os da conta que
//! faltam instalar vêm de [`owned`] (SRC-005).

pub mod owned;

use crate::fs::StoreFs;
use crate::registry::RegistryReader;
use crate::windows_path;
use crate::{LibraryScan, LibrarySource};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tako_core::i18n::t;
use tako_core::library::{Game, GameStore};

pub const PROTOCOL_KEY: &str = r"HKEY_CLASSES_ROOT\amazon-games\shell\open\command";
const ACCENT: &str = "#B35C00";

/// Uma linha instalada da tabela `DbSet`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmazonInstall {
    pub id: String,
    pub title: String,
    pub install_directory: String,
}

/// Linhas com `Installed = 1`. Banco inválido ou sem a tabela é `Err`.
pub fn read_installed(database: &Path) -> Result<Vec<AmazonInstall>, rusqlite::Error> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(database, flags)?;
    let mut stmt =
        conn.prepare("SELECT Id, ProductTitle, InstallDirectory FROM DbSet WHERE Installed = 1")?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, Option<String>>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, Option<String>>(2)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (id, title, folder) = row?;
        let (Some(id), Some(folder)) = (
            id.filter(|s| !s.trim().is_empty()),
            folder.filter(|s| !s.trim().is_empty()),
        ) else {
            continue;
        };
        let title = title
            .filter(|s| !s.trim().is_empty())
            .map_or_else(|| windows_path::last_name(&folder), |s| s.trim().to_owned());
        out.push(AmazonInstall {
            id: id.trim().to_owned(),
            title,
            install_directory: folder,
        });
    }
    Ok(out)
}

pub fn id_for(product_id: &str) -> String {
    format!("amazon:{product_id}")
}

pub fn launch_uri(product_id: &str) -> String {
    format!("amazon-games://play/{product_id}")
}

pub fn to_game(i: &AmazonInstall, added_at: Option<DateTime<Utc>>) -> Game {
    Game {
        id: id_for(&i.id),
        title: i.title.clone(),
        store: Some(GameStore::Amazon),
        accent: ACCENT.to_owned(),
        is_installed: true,
        added_at,
        launch_target: Some(launch_uri(&i.id)),
        install_path: Some(i.install_directory.clone()),
        store_product_id: Some(i.id.clone()),
        ..Default::default()
    }
}

pub struct AmazonLibrarySource {
    registry: Arc<dyn RegistryReader>,
    local_app_data: PathBuf,
    unprotect: Option<owned::Unprotect>,
    pub fs: StoreFs,
}

impl AmazonLibrarySource {
    pub fn new(registry: Arc<dyn RegistryReader>, local_app_data: impl AsRef<Path>) -> Self {
        AmazonLibrarySource {
            registry,
            local_app_data: local_app_data.as_ref().to_path_buf(),
            unprotect: None,
            fs: StoreFs::real(),
        }
    }

    /// Liga a leitura dos jogos da conta (o DPAPI do Windows); sem ela, só os instalados.
    pub fn with_unprotect(mut self, unprotect: Option<owned::Unprotect>) -> Self {
        self.unprotect = unprotect;
        self
    }

    pub fn entitlements_path(&self) -> PathBuf {
        self.local_app_data
            .join("Amazon Games/Data/Games/Sql/Entitlements.sqlite")
    }

    pub fn database_path(&self) -> PathBuf {
        self.local_app_data
            .join("Amazon Games/Data/Games/Sql/GameInstallInfo.sqlite")
    }

    /// Protocolo registrado ou o executável na pasta padrão.
    pub fn client_installed(&self) -> bool {
        self.registry.get_string(PROTOCOL_KEY, "").is_some()
            || (self.fs.exists)(
                &self
                    .local_app_data
                    .join("Amazon Games/App/Amazon Games.exe")
                    .to_string_lossy(),
            )
    }
}

impl LibrarySource for AmazonLibrarySource {
    fn store(&self) -> GameStore {
        GameStore::Amazon
    }

    fn scan(&self) -> LibraryScan {
        let client = self.client_installed();
        let db = self.database_path();
        if !db.is_file() && !client {
            return LibraryScan::missing(t("sources.amazon.missing"));
        }
        let installs = if db.is_file() {
            match read_installed(&db) {
                Ok(i) => i,
                Err(e) => {
                    tracing::warn!(error = %e, "banco do Amazon Games ilegível");
                    return LibraryScan {
                        client_installed: client,
                        notice: Some(t("sources.amazon.unreadable")),
                        ..Default::default()
                    };
                }
            }
        } else {
            Vec::new()
        };
        let mut games: Vec<Game> = Vec::new();
        for mut i in installs {
            i.install_directory = windows_path::normalize(&i.install_directory);
            if (self.fs.exists)(&i.install_directory)
                && !games
                    .iter()
                    .any(|g| g.id.eq_ignore_ascii_case(&id_for(&i.id)))
            {
                games.push(to_game(&i, crate::install_dates::of(&i.install_directory)));
            }
        }
        // Os da conta que faltam instalar: só com o app, que é quem instala.
        if client && let Some(unprotect) = self.unprotect {
            let missing =
                owned::not_installed(&owned::load(&self.entitlements_path(), unprotect), &games);
            games.extend(missing);
        }
        crate::text::sort_by_title(&mut games);
        LibraryScan {
            games,
            client_installed: client,
            notice: (!client).then(|| t("sources.amazon.no_client")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::fake;
    use crate::registry::MemoryRegistry;

    #[test]
    fn reads_installed_rows_from_sqlite_and_launches_by_protocol() {
        let root = tempfile::tempdir().unwrap();
        let reg = MemoryRegistry::default().with(PROTOCOL_KEY, "", "x");
        let mut source = AmazonLibrarySource::new(Arc::new(reg), root.path());
        source.fs = fake::folders(&[
            r"C:\Amazon Games\Library\Ilha",
            r"C:\Amazon Games\Library\Sem Titulo",
        ]);
        let db = source.database_path();
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        Connection::open(&db)
            .unwrap()
            .execute_batch(
                r"CREATE TABLE DbSet (Id TEXT, ProductTitle TEXT, InstallDirectory TEXT, Installed INTEGER, ProductAsin TEXT);
                INSERT INTO DbSet VALUES ('amzn1.adg.product.ilha', 'Ilha Perdida', 'C:\Amazon Games\Library\Ilha', 1, 'B0');
                INSERT INTO DbSet VALUES ('amzn1.adg.product.semtitulo', NULL, 'C:\Amazon Games\Library\Sem Titulo\', 1, 'B1');
                INSERT INTO DbSet VALUES ('amzn1.adg.product.velho', 'Desinstalado', 'C:\Amazon Games\Library\Velho', 0, 'B2');
                INSERT INTO DbSet VALUES ('amzn1.adg.product.sumiu', 'Sumiu', 'C:\Amazon Games\Library\Sumiu', 1, 'B3');",
            )
            .unwrap();

        let scan = source.scan();

        assert!(scan.client_installed);
        let titles: Vec<_> = scan.games.iter().map(|g| g.title.as_str()).collect();
        assert_eq!(titles, ["Ilha Perdida", "Sem Titulo"]);
        assert_eq!(scan.games[0].id, "amazon:amzn1.adg.product.ilha");
        assert_eq!(
            scan.games[0].launch_target.as_deref(),
            Some("amazon-games://play/amzn1.adg.product.ilha")
        );
        assert_eq!(
            scan.games[1].install_path.as_deref(),
            Some(r"C:\Amazon Games\Library\Sem Titulo")
        );
    }

    #[test]
    fn source_adds_account_games_only_with_the_app_and_the_decryptor() {
        let root = tempfile::tempdir().unwrap();
        let reg = || Arc::new(MemoryRegistry::default().with(PROTOCOL_KEY, "", "x"));
        let mut source = AmazonLibrarySource::new(reg(), root.path())
            .with_unprotect(Some(owned::tests::fake_unprotect));
        source.fs = fake::folders(&[r"C:\Amazon Games\Library\Ilha"]);
        let db = source.database_path();
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        Connection::open(&db)
            .unwrap()
            .execute_batch(
                r"CREATE TABLE DbSet (Id TEXT, ProductTitle TEXT, InstallDirectory TEXT, Installed INTEGER);
                INSERT INTO DbSet VALUES ('amzn1.adg.product.ilha', 'Ilha Perdida', 'C:\Amazon Games\Library\Ilha', 1);",
            )
            .unwrap();
        owned::tests::entitlements_db(&source.entitlements_path());

        let scan = source.scan();
        let got: Vec<_> = scan
            .games
            .iter()
            .map(|g| (g.title.as_str(), g.is_installed))
            .collect();
        assert_eq!(
            got,
            [
                ("Farol do Norte", false),
                ("Ilha Perdida", true),
                ("Texto", false)
            ]
        );

        // Sem o decifrador (fora do Windows) ou sem o app: só os instalados.
        let mut plain = AmazonLibrarySource::new(reg(), root.path());
        plain.fs = fake::folders(&[r"C:\Amazon Games\Library\Ilha"]);
        assert_eq!(plain.scan().games.len(), 1);
        let mut no_app = AmazonLibrarySource::new(Arc::new(MemoryRegistry::default()), root.path())
            .with_unprotect(Some(owned::tests::fake_unprotect));
        no_app.fs = fake::folders(&[r"C:\Amazon Games\Library\Ilha"]);
        assert!(no_app.scan().games.iter().all(|g| g.is_installed));
    }

    #[test]
    fn invalid_database_warns_and_missing_client_warns() {
        let root = tempfile::tempdir().unwrap();
        let mut source = AmazonLibrarySource::new(Arc::new(MemoryRegistry::default()), root.path());
        source.fs = fake::folders(&[]);
        assert_eq!(
            source.scan().notice.as_deref(),
            Some("Amazon Games não encontrado neste PC")
        );

        let db = source.database_path();
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        std::fs::write(&db, "não é um banco").unwrap();
        let scan = source.scan();
        assert!(scan.games.is_empty());
        assert_eq!(
            scan.notice.as_deref(),
            Some("Não foi possível ler o banco do Amazon Games")
        );
    }
}
