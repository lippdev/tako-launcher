//! Boot sem interface. Por enquanto duplica a montagem de build_ctx do Slint; não depende da tako-ui.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;
use tako_core::i18n::{self, Lang};
use tako_core::library::{
    CollectionLibrary, Game, GameLibrary, LibraryStore, MemoryLibraryStore, system_clock,
};
use tako_core::metadata::SecretStore;
use tako_core::metadata::agent::{MetadataAgent, PinnedIds};
use tako_core::paths::DataPaths;
use tako_core::persistence::{self as db, TakoDatabase};
use tako_platform::Platform;
use tako_platform::secrets::Secrets;

/// O snapshot pode ter milhares de jogos, mas nunca cresce sem limite.
const MAX_SNAPSHOT_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error(transparent)]
    Persistence(#[from] db::PersistenceError),
    #[error("não foi possível iniciar a leitura das lojas: {0}")]
    Sources(#[from] std::io::Error),
}

/// Dados que sobrevivem à janela. A tako-app lê as lojas pela tako-sources; a sessão de jogo também fica aqui,
/// e o hospedeiro só decide a janela.
pub struct Core {
    pub(crate) settings: HashMap<String, String>,
    pub(crate) lang: Lang,
    pub(crate) library: GameLibrary,
    pub(crate) collections: CollectionLibrary,
    pub(crate) settings_store: Option<db::SettingsStore>,
    found: Arc<Mutex<Arc<Vec<Game>>>>,
    snapshot: Option<SnapshotWriter>,
    pub(crate) sources: crate::sources::Scanner,
    pub(crate) devices: crate::system::Devices,
    pub(crate) downloads: crate::downloads::DownloadsHub,
    pub(crate) game_settings: crate::game_settings::Overrides,
    pub(crate) profiles: crate::profile::ProfileState,
    /// Sessão de jogo (WEB-20): abrir, acompanhar e restaurar, também sem interface aberta.
    pub(crate) session: crate::session::Sessions,
    /// Fontes de metadados para "Fontes e IDs"; sem elas (demonstração) a lista de fontes vai vazia.
    pub(crate) metadata: Option<Arc<MetadataAgent>>,
    pub(crate) metadata_cache: std::path::PathBuf,
    pub(crate) choices: Option<db::GameChoicesStore>,
    /// Ids que o usuário fixou, por jogo e fonte (vazio = "nenhum").
    pub(crate) pinned_ids: HashMap<String, PinnedIds>,
    pub(crate) jobs: crate::edit::Jobs,
    /// Pastas de ROMs e executáveis dos emuladores; a próxima leitura das lojas usa o que está aqui.
    pub(crate) emulators: Arc<Mutex<tako_sources::emulators::Config>>,
    /// Demonstração: nada é procurado nem verificado no disco.
    pub(crate) simulate: bool,
    pub(crate) art: crate::art::ArtState,
}

impl Core {
    pub(crate) fn metadata_vault(platform: &Platform) -> impl SecretStore + '_ {
        Vault(platform.secrets.as_ref())
    }

    /// Abre o banco e os dados locais; a primeira consulta usa o snapshot da última leitura das lojas.
    /// Em demonstração nada é lido ou gravado no banco, no snapshot ou no cofre.
    pub fn open(paths: DataPaths, platform: Platform, simulate: bool) -> Result<Self, CoreError> {
        Self::open_using(paths, platform, simulate, None, None)
    }

    /// Abre com fontes fornecidas pelo hospedeiro, sem descobrir lojas da máquina. Permite testes isolados
    /// do boot real (banco e snapshot); em demonstração nem estas fontes são executadas.
    pub fn open_with_sources(
        paths: DataPaths,
        platform: Platform,
        simulate: bool,
        sources: Vec<Box<dyn tako_sources::LibrarySource>>,
    ) -> Result<Self, CoreError> {
        let sources = Arc::new(sources);
        Self::open_using(
            paths,
            platform,
            simulate,
            Some(Box::new(move || Arc::clone(&sources))),
            None,
        )
    }

    /// Como [`Core::open_with_sources`], com o motor de downloads fornecido pelo hospedeiro (o falso, nos testes) e a
    /// pasta onde cada download ganha a sua subpasta. Sem isto o motor é o real, ou o falso na demonstração.
    pub fn open_with_downloads(
        paths: DataPaths,
        platform: Platform,
        simulate: bool,
        sources: Vec<Box<dyn tako_sources::LibrarySource>>,
        engine: impl Fn() -> Box<dyn tako_core::downloads::Downloader> + Send + 'static,
        root: std::path::PathBuf,
    ) -> Result<Self, CoreError> {
        let sources = Arc::new(sources);
        Self::open_using(
            paths,
            platform,
            simulate,
            Some(Box::new(move || Arc::clone(&sources))),
            Some((Box::new(engine), root)),
        )
    }

    fn open_using(
        paths: DataPaths,
        platform: Platform,
        simulate: bool,
        factory: Option<crate::sources::Factory>,
        engine: Option<(crate::downloads::EngineFactory, std::path::PathBuf)>,
    ) -> Result<Self, CoreError> {
        let database = if simulate {
            None
        } else {
            let database = TakoDatabase::new(paths.database());
            database.open()?;
            #[cfg(windows)]
            let decrypt = tako_platform::windows::dpapi::unprotect;
            #[cfg(not(windows))]
            let decrypt = |_: &[u8]| None;
            if let Err(e) = db::alpha_migration::migrate_secrets(
                &paths.machine,
                decrypt,
                &Vault(platform.secrets.as_ref()),
            ) {
                tracing::warn!("segredos das alphas: {e}");
            }
            Some(database)
        };
        let settings = database
            .as_ref()
            .map(|d| {
                db::SettingsStore::new(d.clone())
                    .load_all()
                    .unwrap_or_else(|e| {
                        tracing::error!("configurações: {e}");
                        HashMap::new()
                    })
            })
            .unwrap_or_default();
        let lang = settings
            .get("Language")
            .and_then(|s| Lang::from_code(s))
            .unwrap_or_else(Lang::from_env);
        if !cfg!(test) {
            i18n::set_lang(lang);
        }
        let store: Box<dyn LibraryStore + Send> = match &database {
            Some(d) => Box::new(db::LibraryStore::new(
                d.clone(),
                Some(paths.root.join("library.json")),
            )),
            None => Box::new(MemoryLibraryStore::default()),
        };
        let found = Arc::new(Mutex::new(Arc::new(if simulate {
            crate::demo::games()
        } else {
            load_found(&paths.cache().join("found-games.json"))
        })));
        let discover_from = Arc::clone(&found);
        let mut library = GameLibrary::new(
            Box::new(move || {
                discover_from
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .as_ref()
                    .clone()
            }),
            store,
            system_clock(),
        );
        library.load();
        let settings_store = database.clone().map(db::SettingsStore::new);
        let art = crate::art::ArtState::open(
            &paths,
            &platform,
            database.as_ref(),
            simulate,
            &mut library,
        );
        let game_settings =
            crate::game_settings::Overrides::new(database.clone().map(db::GameSettingsStore::new));
        // Antes de tudo que mexe na máquina: restaura tela e áudio de um jogo que o núcleo não viu terminar.
        let session =
            crate::session::Sessions::open(&platform, &paths, database.as_ref(), simulate);
        let choices = database.clone().map(db::GameChoicesStore::new);
        let metadata =
            (!simulate).then(|| crate::edit::real_agent(&paths, &Vault(platform.secrets.as_ref())));
        let downloads = crate::downloads::DownloadsHub::new(
            database.clone().map(db::DownloadStore::new),
            paths.root.join("download-sources"),
            simulate,
            engine,
        );
        let collections = database
            .map(|d| {
                let mut collections =
                    CollectionLibrary::new(Box::new(db::CollectionStore::new(d)), system_clock());
                if let Err(e) = collections.load() {
                    tracing::error!("coleções: {e}");
                }
                collections.apply_to(library.games_mut());
                collections
            })
            .unwrap_or_else(|| {
                CollectionLibrary::new(Box::new(crate::organize::MemoryCollections), system_clock())
            });
        let emulators = Arc::new(Mutex::new(crate::sources::emulator_config(&settings)));
        let sources = crate::sources::Scanner::open(
            factory.unwrap_or_else(|| crate::sources::factory(Arc::clone(&emulators))),
            settings.get("DisabledStores").cloned().unwrap_or_default(),
            lang,
            simulate,
        )?;
        let devices = crate::system::Devices::new(&platform, simulate);

        let mut core = Self {
            settings,
            settings_store,
            lang,
            library,
            collections,
            found,
            sources,
            devices,
            snapshot: (!simulate)
                .then(|| SnapshotWriter::open(paths.cache().join("found-games.json")))
                .flatten(),
            downloads,
            game_settings,
            profiles: crate::profile::ProfileState::open(&paths, simulate),
            session,
            metadata,
            metadata_cache: paths.metadata_cache(),
            pinned_ids: crate::edit::load_pinned(choices.as_ref()),
            choices,
            jobs: crate::edit::Jobs::new(),
            emulators,
            simulate,
            art,
        };
        // Ligado, o estado salvo volta já na abertura; desligado, o motor nem é criado.
        core.downloads.ensure(&core.settings);
        Ok(core)
    }

    pub fn language(&self) -> Lang {
        self.lang
    }

    pub fn setting(&self, key: &str) -> Option<&str> {
        self.settings.get(key).map(String::as_str)
    }

    /// Liga as fontes de metadados de "Fontes e IDs" (o hospedeiro, ou um teste com fontes falsas).
    pub fn set_metadata_agent(&mut self, agent: Arc<MetadataAgent>) {
        self.metadata = Some(agent);
    }

    pub fn library(&self) -> &GameLibrary {
        &self.library
    }

    /// O AppService emite library.changed depois desta troca. Não expõe mutação silenciosa à interface.
    pub(crate) fn reload_library(&mut self, games: Vec<Game>) {
        let games = Arc::new(games);
        *self.found.lock().unwrap_or_else(|e| e.into_inner()) = Arc::clone(&games);
        self.library.refresh();
        self.collections.apply_to(self.library.games_mut());
        if let Some(snapshot) = &self.snapshot
            && snapshot
                .sender
                .as_ref()
                .is_some_and(|sender| sender.send(games).is_err())
        {
            tracing::warn!("gravador do snapshot encerrado");
        }
    }
}

/// Um único gravador preserva a ordem das recargas sem serializar na thread do núcleo.
struct SnapshotWriter {
    sender: Option<mpsc::Sender<Arc<Vec<Game>>>>,
    worker: Option<JoinHandle<()>>,
}

impl SnapshotWriter {
    fn open(path: std::path::PathBuf) -> Option<Self> {
        let (sender, receiver) = mpsc::channel::<Arc<Vec<Game>>>();
        match std::thread::Builder::new()
            .name("library-snapshot".into())
            .spawn(move || {
                for games in receiver {
                    let result = serde_json::to_vec(games.as_ref())
                        .map_err(std::io::Error::other)
                        .and_then(|bytes| {
                            tako_core::metadata::cache::write_cache_file(&path, &bytes)
                        });
                    if let Err(e) = result {
                        tracing::warn!("não foi possível guardar a última leitura das lojas: {e}");
                    }
                }
            }) {
            Ok(worker) => Some(Self {
                sender: Some(sender),
                worker: Some(worker),
            }),
            Err(e) => {
                tracing::warn!("não foi possível iniciar o gravador do snapshot: {e}");
                None
            }
        }
    }
}

impl Drop for SnapshotWriter {
    fn drop(&mut self) {
        self.sender.take();
        // Só o encerramento aguarda o cache; a recarga e seu evento não esperam pelo disco.
        if let Some(worker) = self.worker.take()
            && worker.join().is_err()
        {
            tracing::error!("gravador do snapshot terminou com pânico");
        }
    }
}

struct Vault<'a>(&'a dyn Secrets);

impl SecretStore for Vault<'_> {
    fn get(&self, key: &str) -> Option<String> {
        self.0.get(key)
    }

    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        self.0.set(key, value)
    }
}

fn load_found(path: &Path) -> Vec<Game> {
    match read_found(path) {
        Ok(games) => games,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => {
            tracing::warn!("snapshot da biblioteca ignorado: {e}");
            Vec::new()
        }
    }
}

fn read_found(path: &Path) -> std::io::Result<Vec<Game>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(MAX_SNAPSHOT_BYTES + 1)
        .read_to_end(&mut bytes)?;
    let invalid = || std::io::Error::new(std::io::ErrorKind::InvalidData, "snapshot inválido");
    if bytes.len() as u64 > MAX_SNAPSHOT_BYTES {
        return Err(invalid());
    }
    serde_json::from_slice(&bytes).map_err(|_| invalid())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reload_increments_the_library_version() {
        let dir = tempfile::tempdir().unwrap();
        let mut core = Core::open(DataPaths::at(dir.path()), Platform::simulated(), true).unwrap();
        let before = core.library().version();
        core.reload_library(Vec::new());
        assert_eq!(core.library().version(), before + 1);
        assert!(core.library().games().is_empty());
    }
}
