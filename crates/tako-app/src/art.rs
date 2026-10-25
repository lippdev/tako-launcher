//! Arte e metadados fora da ponte: fila do Slint, cache compartilhado e avisos para o hospedeiro.

use crate::protocol::{Event, GameRequest};
use crate::{Core, HostRequest};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc};
use tako_core::library::GameLibrary;
use tako_core::metadata::agent::{MetadataAgent, PinnedIds};
use tako_core::metadata::art_cache::ArtCache;
use tako_core::metadata::cache::MetadataCache;
use tako_core::metadata::game_art::{ArtWorker, GameArt};
use tako_core::metadata::igdb::IgdbSettings;
use tako_core::metadata::screenscraper::ScreenScraperSettings;
use tako_core::metadata::steamgriddb::SteamGridDbSettings;
use tako_core::metadata::worker::{MetadataUpdated, MetadataWorker, SPACING};
use tako_core::paths::DataPaths;
use tako_core::persistence::{GameChoicesStore, TakoDatabase};
use tako_platform::Platform;

type Wake = Arc<Mutex<Option<mpsc::Sender<HostRequest>>>>;
enum Update {
    Art(String),
    Metadata(Box<MetadataUpdated>),
}

pub(crate) struct ArtState {
    jobs: Option<Jobs>,
    version: Option<u64>,
    pins: HashMap<String, PinnedIds>,
}

struct Jobs {
    art: ArtWorker,
    metadata: MetadataWorker,
    updates: mpsc::Receiver<Update>,
    wake: Wake,
}

fn send(tx: &mpsc::Sender<Update>, wake: &Wake, update: Update) {
    if tx.send(update).is_ok()
        && let Some(host) = &*wake.lock().unwrap_or_else(|e| e.into_inner())
    {
        let _ = host.send(HostRequest::PollBackground);
    }
}

impl ArtState {
    pub(crate) fn open(
        paths: &DataPaths,
        platform: &Platform,
        database: Option<&TakoDatabase>,
        simulate: bool,
        library: &mut GameLibrary,
    ) -> Self {
        if simulate {
            return Self {
                jobs: None,
                version: None,
                pins: HashMap::new(),
            };
        }
        let choices = database.cloned().map(GameChoicesStore::new);
        let art_pins = choices
            .as_ref()
            .and_then(|c| c.load_all_art().ok())
            .unwrap_or_default();
        let pins = choices
            .as_ref()
            .and_then(|c| c.load_all_ids().ok())
            .unwrap_or_default()
            .into_iter()
            .map(|(id, pins)| (id, pins.into_iter().collect()))
            .collect();
        let cache = MetadataCache::new(paths.metadata_cache());
        let old = std::mem::replace(
            library,
            GameLibrary::in_memory(Vec::new(), tako_core::library::system_clock()),
        );
        *library = old.with_enrich(Box::new(move |game| {
            if let Some(metadata) = cache.read(&game.id) {
                metadata.apply_to(game);
            }
            game.pinned_art = art_pins.get(&game.id).cloned().unwrap_or_default();
        }));
        library.refresh();
        let vault = Core::metadata_vault(platform);
        let sources = tako_core::metadata::runtime::sources(
            paths,
            IgdbSettings::load(&vault),
            ScreenScraperSettings::load(&vault),
            SteamGridDbSettings::load(&vault),
            Vec::new(),
        );
        let cache = tako_core::metadata::runtime::art_cache(
            paths,
            tako_sources::steam::find_root(),
            Some(sources.screenscraper.clone()),
        );
        let mut state = Self::pipeline(paths, cache, sources.agent());
        state.pins = pins;
        state
    }

    fn pipeline(paths: &DataPaths, cache: Arc<ArtCache>, agent: MetadataAgent) -> Self {
        let (tx, updates) = mpsc::channel();
        let wake: Wake = Arc::default();
        let art_tx = tx.clone();
        let art_wake = wake.clone();
        let art = ArtWorker::spawn(cache, GameArt::new(paths), move |id| {
            send(&art_tx, &art_wake, Update::Art(id))
        });
        let (metadata, results) =
            MetadataWorker::spawn(agent, MetadataCache::new(paths.metadata_cache()), SPACING);
        let meta_wake = wake.clone();
        let spawned = std::thread::Builder::new()
            .name("art-metadata-relay".into())
            .spawn(move || {
                for update in results {
                    send(&tx, &meta_wake, Update::Metadata(Box::new(update)));
                }
            });
        if spawned.is_err() {
            tracing::error!("não foi possível iniciar os avisos de arte");
        }
        Self {
            jobs: Some(Jobs {
                art,
                metadata,
                updates,
                wake,
            }),
            version: None,
            pins: HashMap::new(),
        }
    }

    pub(crate) fn set_host_sink(&mut self, sink: mpsc::Sender<HostRequest>, has_games: bool) {
        if let Some(jobs) = &self.jobs {
            *jobs.wake.lock().unwrap_or_else(|e| e.into_inner()) = Some(sink.clone());
            if has_games {
                let _ = sink.send(HostRequest::PollBackground);
            }
        }
    }

    fn queue(&mut self, library: &GameLibrary) {
        if self.version == Some(library.version()) {
            return;
        }
        self.version = Some(library.version());
        if let Some(jobs) = &self.jobs {
            jobs.art.refresh(library.games().iter().cloned());
            jobs.metadata
                .refresh(library.games().to_vec(), false, self.pins.clone());
        }
    }
}

impl Core {
    /// Injeta os mesmos clientes por abstrações do núcleo (rede falsa nos testes, sem novos endpoints).
    pub fn with_art_pipeline(
        mut self,
        paths: &DataPaths,
        cache: Arc<ArtCache>,
        agent: MetadataAgent,
    ) -> Self {
        let pins = std::mem::take(&mut self.art.pins);
        self.art = ArtState::pipeline(paths, cache, agent);
        self.art.pins = pins;
        self
    }
}

impl crate::AppService {
    pub(crate) fn poll_art(&mut self) {
        let Some(core) = &mut self.core else { return };
        core.art.queue(&core.library);
        let mut events = Vec::new();
        if let Some(jobs) = &core.art.jobs {
            while let Ok(update) = jobs.updates.try_recv() {
                match update {
                    Update::Art(game_id) => events.push(Event::ArtChanged(GameRequest { game_id })),
                    Update::Metadata(update) => {
                        if let Some(game) = core
                            .library
                            .games_mut()
                            .iter_mut()
                            .find(|g| g.id == update.game_id)
                        {
                            update.metadata.apply_to(game);
                            jobs.art.refresh([game.clone()]);
                            events.push(Event::LibraryChanged);
                        }
                    }
                }
            }
        }
        for event in events {
            self.emit(event);
        }
    }
}
