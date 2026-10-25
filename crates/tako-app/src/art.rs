//! Arte e metadados fora da ponte: fila do Slint, cache compartilhado e avisos para o hospedeiro.

use crate::protocol::{
    ArtCandidateDto, ArtCandidatesReady, ArtCandidatesRequest, ArtCandidatesView, ArtChooseRequest,
    ArtConfidence, ArtDoubtDto, ArtKindDto, ArtResetRequest, ArtReviewItem, ArtReviewView,
    BridgeError, ErrorCode, Event, GameRequest,
};
use crate::{Core, HostRequest};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, mpsc};
use tako_core::library::{ArtKind, Game, GameLibrary};
use tako_core::metadata::agent::{MetadataAgent, PinnedIds};
use tako_core::metadata::art::ArtCandidate;
use tako_core::metadata::art_cache::ArtCache;
use tako_core::metadata::art_catalog::{ArtCatalog, known_ids};
use tako_core::metadata::cache::MetadataCache;
use tako_core::metadata::game_art::{ArtWorker, GameArt};
use tako_core::metadata::igdb::IgdbSettings;
use tako_core::metadata::review::Review;
use tako_core::metadata::screenscraper::ScreenScraperSettings;
use tako_core::metadata::steamgriddb::SteamGridDbSettings;
use tako_core::metadata::worker::MetadataUpdated;
use tako_core::paths::DataPaths;
use tako_core::persistence::{GameChoicesStore, TakoDatabase};
use tako_platform::Platform;

type Wake = Arc<Mutex<Option<mpsc::Sender<HostRequest>>>>;
type CandidatesMap = HashMap<(String, ArtKind), Vec<ArtCandidate>>;
type InFlightSet = HashSet<(String, ArtKind)>;

pub(crate) enum Update {
    Art(String),
    Metadata(Box<MetadataUpdated>, PinnedIds),
    CandidatesReady {
        game_id: String,
        kind: ArtKindDto,
        candidates: Vec<ArtCandidateDto>,
    },
}

pub(crate) struct ArtState {
    pub(crate) jobs: Option<Jobs>,
    pub(crate) updates_tx: mpsc::Sender<Update>,
    pub(crate) updates_rx: mpsc::Receiver<Update>,
    pub(crate) wake: Wake,
    version: Option<u64>,
    pub(crate) paths: Option<DataPaths>,
    pub(crate) cache: Option<Arc<ArtCache>>,
    pub(crate) game_art: Option<GameArt>,
    pub(crate) catalog: Option<Arc<ArtCatalog>>,
    pub(crate) candidates: Arc<Mutex<CandidatesMap>>,
    pub(crate) in_flight: Arc<Mutex<InFlightSet>>,
}

pub(crate) struct Jobs {
    art: ArtWorker,
    metadata: Option<crate::metadata::MetadataQueue>,
}

fn send(tx: &mpsc::Sender<Update>, wake: &Wake, update: Update) {
    if tx.send(update).is_ok()
        && let Some(host) = &*wake.lock().unwrap_or_else(|e| e.into_inner())
    {
        let _ = host.send(HostRequest::PollBackground);
    }
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => {
                use std::fmt::Write;
                let _ = write!(out, "%{:02X}", byte);
            }
        }
    }
    out
}

fn copy_chosen(
    dir: &std::path::Path,
    path: &std::path::Path,
) -> std::io::Result<std::path::PathBuf> {
    std::fs::create_dir_all(dir)?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png");
    let name = format!("{}.{}", uuid_v4_simple(), ext);
    let target = dir.join(name);
    std::fs::copy(path, &target)?;
    Ok(target)
}

fn uuid_v4_simple() -> String {
    use std::time::SystemTime;
    let t = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("art-{:x}", t)
}

impl ArtState {
    pub(crate) fn attach_metadata(
        &mut self,
        agent: Option<Arc<MetadataAgent>>,
        cache: std::path::PathBuf,
    ) {
        if let Some(jobs) = &mut self.jobs {
            let (tx, wake) = (self.updates_tx.clone(), self.wake.clone());
            jobs.metadata = agent.map(|agent| {
                crate::metadata::MetadataQueue::new(agent, cache, move |update, pins| {
                    send(&tx, &wake, Update::Metadata(Box::new(update), pins));
                })
            });
            self.version = None;
        }
    }

    pub(crate) fn refresh_metadata(
        &self,
        game: tako_core::library::Game,
        pins: &HashMap<String, PinnedIds>,
    ) {
        if let Some(metadata) = self.jobs.as_ref().and_then(|jobs| jobs.metadata.as_ref()) {
            metadata.worker.refresh(vec![game], true, pins.clone());
        }
    }

    pub(crate) fn open(
        paths: &DataPaths,
        platform: &Platform,
        database: Option<&TakoDatabase>,
        simulate: bool,
        library: &mut GameLibrary,
    ) -> Self {
        let (tx, updates) = mpsc::channel();
        let wake: Wake = Arc::default();
        if simulate {
            return Self {
                jobs: None,
                updates_tx: tx,
                updates_rx: updates,
                wake,
                version: None,
                paths: Some(paths.clone()),
                cache: None,
                game_art: Some(GameArt::new(paths)),
                catalog: None,
                candidates: Arc::default(),
                in_flight: Arc::default(),
            };
        }
        let choices = database.cloned().map(GameChoicesStore::new);
        let art_pins = choices
            .as_ref()
            .and_then(|c| c.load_all_art().ok())
            .unwrap_or_default();
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
        let art_tx = tx.clone();
        let art_wake = wake.clone();
        let art = ArtWorker::spawn(cache.clone(), GameArt::new(paths), move |id| {
            send(&art_tx, &art_wake, Update::Art(id))
        });
        Self {
            jobs: Some(Jobs {
                art,
                metadata: None,
            }),
            updates_tx: tx,
            updates_rx: updates,
            wake,
            version: None,
            paths: Some(paths.clone()),
            cache: Some(cache),
            game_art: Some(GameArt::new(paths)),
            catalog: Some(Arc::new(sources.catalog)),
            candidates: Arc::default(),
            in_flight: Arc::default(),
        }
    }

    fn pipeline(paths: &DataPaths, cache: Arc<ArtCache>) -> Self {
        let (tx, updates) = mpsc::channel();
        let wake: Wake = Arc::default();
        let art_tx = tx.clone();
        let art_wake = wake.clone();
        let art = ArtWorker::spawn(cache.clone(), GameArt::new(paths), move |id| {
            send(&art_tx, &art_wake, Update::Art(id))
        });
        Self {
            jobs: Some(Jobs {
                art,
                metadata: None,
            }),
            updates_tx: tx,
            updates_rx: updates,
            wake,
            version: None,
            paths: Some(paths.clone()),
            cache: Some(cache),
            game_art: Some(GameArt::new(paths)),
            catalog: None,
            candidates: Arc::default(),
            in_flight: Arc::default(),
        }
    }

    pub(crate) fn set_host_sink(&mut self, sink: mpsc::Sender<HostRequest>, has_games: bool) {
        *self.wake.lock().unwrap_or_else(|e| e.into_inner()) = Some(sink.clone());
        if self.jobs.is_some() && has_games {
            let _ = sink.send(HostRequest::PollBackground);
        }
    }

    fn queue(&mut self, library: &GameLibrary, pins: &HashMap<String, PinnedIds>) {
        if self.version == Some(library.version()) {
            return;
        }
        self.version = Some(library.version());
        if let Some(jobs) = &self.jobs {
            jobs.art.refresh(library.games().iter().cloned());
            if let Some(metadata) = &jobs.metadata {
                metadata
                    .worker
                    .refresh(library.games().to_vec(), false, pins.clone());
            }
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
        self.art = ArtState::pipeline(paths, cache);
        self.set_metadata_agent(Arc::new(agent));
        self
    }

    pub fn with_art_catalog(mut self, catalog: ArtCatalog) -> Self {
        self.art.catalog = Some(Arc::new(catalog));
        self
    }

    fn build_candidate_dtos(
        &self,
        game: &Game,
        kind: ArtKind,
        candidates: &[ArtCandidate],
    ) -> Vec<ArtCandidateDto> {
        let current_pin = game.pinned_art.get(&kind);
        let auto_uri = tako_core::metadata::art::pins::automatic(game, kind);

        let mut dtos = Vec::with_capacity(candidates.len());
        for (idx, c) in candidates.iter().enumerate() {
            let candidate_id = format!("cand-{}", idx);
            let pub_id = format!("{}-cand-{}", game.id, idx);

            let url = if self
                .art
                .game_art
                .as_ref()
                .is_some_and(|g| g.resolve(&pub_id, kind).is_some())
            {
                format!("tako-img://{}?id={}", kind.key(), percent_encode(&pub_id))
            } else {
                format!("tako-img://{}?id={}", kind.key(), percent_encode(&game.id))
            };

            let is_current = match current_pin {
                Some(pinned_url) => pinned_url == &c.url,
                None => auto_uri == Some(c.url.as_str()),
            };

            dtos.push(ArtCandidateDto {
                id: candidate_id,
                url,
                source: tako_core::metadata::art::source_label(&c.source),
                dimensions: c.size_text(),
                style: c.style.clone(),
                current: is_current,
            });
        }
        dtos
    }

    pub(crate) fn art_candidates(
        &self,
        req: &ArtCandidatesRequest,
    ) -> Result<ArtCandidatesView, BridgeError> {
        let game_id = &req.game_id;
        let kind: ArtKind = req.kind.into();
        let game = self
            .library
            .find(game_id)
            .cloned()
            .ok_or_else(|| BridgeError::new(ErrorCode::NotFound, "jogo não encontrado"))?;

        // 1. Responde na hora com o que houver em cache
        {
            let lock = self
                .art
                .candidates
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(candidates) = lock.get(&(game_id.clone(), kind)) {
                let dtos = self.build_candidate_dtos(&game, kind, candidates);
                return Ok(ArtCandidatesView {
                    candidates: dtos,
                    loading: false,
                });
            }
        }

        // 2. Se já há busca em andamento, responde vazio e carregando
        let mut in_flight = self.art.in_flight.lock().unwrap_or_else(|e| e.into_inner());
        if !in_flight.insert((game_id.clone(), kind)) {
            return Ok(ArtCandidatesView {
                candidates: Vec::new(),
                loading: true,
            });
        }

        // 3. Dispara a busca em thread separada
        let tx = self.art.updates_tx.clone();
        let wake = Arc::clone(&self.art.wake);
        let candidates_map = Arc::clone(&self.art.candidates);
        let in_flight_map = Arc::clone(&self.art.in_flight);
        let catalog = self.art.catalog.clone();
        let cache = self.art.cache.clone();
        let game_art = self.art.game_art.clone();
        let metadata_cache_path = self.metadata_cache.clone();
        let pinned = self.pinned_ids.get(game_id).cloned().unwrap_or_default();

        let spawned = std::thread::Builder::new()
            .name("art-candidates".into())
            .spawn(move || {
                let cached = MetadataAgent::normalize(
                    MetadataCache::new(metadata_cache_path).read(&game.id),
                );
                let ids = known_ids(&game, cached.as_ref(), &pinned);

                let raw_candidates = if let Some(catalog) = &catalog {
                    let sources = catalog.sources(&game, &ids);
                    let mut list = Vec::new();
                    for source in sources {
                        if !tako_core::metadata::art::offers(source, kind) {
                            continue;
                        }
                        let query = tako_core::metadata::art_catalog::ArtQuery {
                            game: &game,
                            source,
                            kind,
                            filter: None,
                            steamgriddb_game: None,
                            page: 0,
                        };
                        list.extend(catalog.list(&query, &ids));
                    }
                    list
                } else if let Some(app_id) = game.steam_app_id {
                    tako_core::metadata::art::steam_candidates(app_id)
                        .into_iter()
                        .filter(|c| c.kind == Some(kind))
                        .collect()
                } else {
                    Vec::new()
                };

                let candidates: Vec<_> = raw_candidates.into_iter().take(50).collect();

                // Baixar preview de cada candidato para o cache e publicar com pub_id próprio
                for (idx, c) in candidates.iter().enumerate() {
                    let pub_id = format!("{}-cand-{}", game.id, idx);
                    if let (Some(cache), Some(game_art)) = (&cache, &game_art) {
                        let preview = c.preview();
                        if let Some(file) = cache.locate(preview)
                            && game_art.publish(&pub_id, kind, &file).is_err()
                            && let Ok(b) = std::fs::read(&file)
                            && let Some(imported) = cache.import(preview, &b)
                        {
                            let _ = game_art.publish(&pub_id, kind, &imported);
                        }
                    }
                }

                let current_pin = game.pinned_art.get(&kind);
                let auto_uri = tako_core::metadata::art::pins::automatic(&game, kind);

                let mut dtos = Vec::with_capacity(candidates.len());
                for (idx, c) in candidates.iter().enumerate() {
                    let candidate_id = format!("cand-{}", idx);
                    let pub_id = format!("{}-cand-{}", game.id, idx);

                    let url = if game_art
                        .as_ref()
                        .is_some_and(|g| g.resolve(&pub_id, kind).is_some())
                    {
                        format!("tako-img://{}?id={}", kind.key(), percent_encode(&pub_id))
                    } else {
                        format!("tako-img://{}?id={}", kind.key(), percent_encode(&game.id))
                    };

                    let is_current = match current_pin {
                        Some(pinned_url) => pinned_url == &c.url,
                        None => auto_uri == Some(c.url.as_str()),
                    };

                    dtos.push(ArtCandidateDto {
                        id: candidate_id,
                        url,
                        source: tako_core::metadata::art::source_label(&c.source),
                        dimensions: c.size_text(),
                        style: c.style.clone(),
                        current: is_current,
                    });
                }

                candidates_map
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert((game.id.clone(), kind), candidates);

                in_flight_map
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(&(game.id.clone(), kind));

                send(
                    &tx,
                    &wake,
                    Update::CandidatesReady {
                        game_id: game.id,
                        kind: ArtKindDto::from(kind),
                        candidates: dtos,
                    },
                );
            });

        if let Err(e) = spawned {
            in_flight.remove(&(game_id.clone(), kind));
            return Err(BridgeError::new(
                ErrorCode::Internal,
                format!("não foi possível iniciar busca de candidatos: {e}"),
            ));
        }

        Ok(ArtCandidatesView {
            candidates: Vec::new(),
            loading: true,
        })
    }

    pub(crate) fn art_choose(&mut self, req: &ArtChooseRequest) -> Result<(), BridgeError> {
        let game_id = &req.game_id;
        let kind: ArtKind = req.kind.into();

        let candidate = {
            let lock = self
                .art
                .candidates
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let list = lock.get(&(game_id.clone(), kind)).ok_or_else(|| {
                BridgeError::new(ErrorCode::NotFound, "candidatos não encontrados")
            })?;
            list.iter()
                .enumerate()
                .find(|(i, c)| {
                    format!("cand-{}", i) == req.candidate_id || c.url == req.candidate_id
                })
                .map(|(_, c)| c.clone())
                .ok_or_else(|| BridgeError::new(ErrorCode::NotFound, "candidato não encontrado"))?
        };

        let url = if candidate.source == tako_core::metadata::art::FILE_SOURCE {
            let dir = self
                .art
                .paths
                .as_ref()
                .map(|p| p.art_choices())
                .unwrap_or_else(|| std::path::PathBuf::from("art-choices"));
            let path = std::path::PathBuf::from(&candidate.url);
            let target = copy_chosen(&dir, &path).unwrap_or(path);
            tako_core::library::manual::file_uri(&target.to_string_lossy())
        } else {
            candidate.url.clone()
        };

        if let Some(choices) = &self.choices
            && let Err(e) = choices.set_art(game_id, kind, Some(&url))
        {
            tracing::warn!("falha ao salvar arte no banco: {e}");
        }

        if let Some(g) = self
            .library
            .games_mut()
            .iter_mut()
            .find(|g| g.id == *game_id)
        {
            g.pinned_art = tako_core::metadata::art::pins::with(&g.pinned_art, kind, Some(&url));
            if let Some(jobs) = &self.art.jobs {
                jobs.art.refresh([g.clone()]);
            }
        }

        Ok(())
    }

    pub(crate) fn art_reset(&mut self, req: &ArtResetRequest) -> Result<(), BridgeError> {
        let game_id = &req.game_id;
        let kind: ArtKind = req.kind.into();

        if let Some(choices) = &self.choices
            && let Err(e) = choices.set_art(game_id, kind, None)
        {
            tracing::warn!("falha ao resetar arte no banco: {e}");
        }

        if let Some(g) = self
            .library
            .games_mut()
            .iter_mut()
            .find(|g| g.id == *game_id)
        {
            g.pinned_art = tako_core::metadata::art::pins::with(&g.pinned_art, kind, None);
            if let Some(jobs) = &self.art.jobs {
                jobs.art.refresh([g.clone()]);
            }
        }

        Ok(())
    }

    pub(crate) fn art_review(&self) -> Result<ArtReviewView, BridgeError> {
        let games = self.library.games().to_vec();
        let cache = MetadataCache::new(self.metadata_cache.clone());
        let total = games.len();
        let mut found: Vec<(Game, Review)> = games
            .into_iter()
            .filter_map(|g| {
                let cached = MetadataAgent::normalize(cache.read(&g.id));
                let r = tako_core::metadata::review::review(&g, cached.as_ref());
                if r.needs_attention() {
                    Some((g, r))
                } else {
                    None
                }
            })
            .collect();

        found.sort_by_key(|(_, r)| r.rank());

        let mut items = Vec::with_capacity(found.len());
        for (game, r) in found {
            let current_cover = tako_core::metadata::art::pins::shown(&game, ArtKind::Cover)
                .map(|_| format!("tako-img://cover?id={}", percent_encode(&game.id)));
            let proposed_cover = r
                .doubt
                .as_ref()
                .and_then(|d| d.cover.as_ref())
                .map(|_| format!("tako-img://cover?id={}", percent_encode(&game.id)));

            let confidence =
                r.doubt
                    .as_ref()
                    .map_or(ArtConfidence::Low, |d| match d.confidence() {
                        tako_core::metadata::review::Confidence::High => ArtConfidence::High,
                        tako_core::metadata::review::Confidence::Medium => ArtConfidence::Medium,
                        tako_core::metadata::review::Confidence::Low => ArtConfidence::Low,
                    });

            let confidence_percent = r.doubt.as_ref().map_or(0, |d| d.percent());

            let reason = if let Some(d) = &r.doubt {
                format!("Achado pelo nome no {} como \"{}\"", d.source, d.found)
            } else {
                let kinds: Vec<String> = r
                    .missing
                    .iter()
                    .map(|k| tako_core::metadata::art::kind_label(*k).to_lowercase())
                    .collect();
                format!("Sem {}", kinds.join(", "))
            };

            let doubt = r.doubt.as_ref().map(|d| ArtDoubtDto {
                source: d.source.clone(),
                id: d.id.clone(),
                found: d.found.clone(),
            });

            let missing_kinds = r.missing.iter().map(|k| ArtKindDto::from(*k)).collect();

            items.push(ArtReviewItem {
                store: game.store().into(),
                game_id: game.id,
                title: game.title,
                current_cover,
                proposed_cover,
                confidence,
                confidence_percent,
                reason,
                doubt,
                missing_kinds,
            });
        }

        Ok(ArtReviewView { total, items })
    }

    pub(crate) fn art_review_accept(&mut self, req: &GameRequest) -> Result<(), BridgeError> {
        let game_id = &req.game_id;
        let game = self
            .library
            .find(game_id)
            .cloned()
            .ok_or_else(|| BridgeError::new(ErrorCode::NotFound, "jogo não encontrado"))?;

        let cache = MetadataCache::new(self.metadata_cache.clone());
        let cached = MetadataAgent::normalize(cache.read(game_id));
        let r = tako_core::metadata::review::review(&game, cached.as_ref());

        if let Some(d) = r.doubt {
            let mut pins = self.pinned_ids.get(game_id).cloned().unwrap_or_default();
            pins.insert(d.source.clone(), d.id.clone());
            if let Some(choices) = &self.choices {
                let _ = choices.set_id(game_id, &d.source, Some(&d.id));
            }
            self.pinned_ids.insert(game_id.clone(), pins);
            self.refresh_metadata(game_id);
        }

        Ok(())
    }
}

impl crate::AppService {
    pub(crate) fn poll_art(&mut self) {
        let Some(core) = &mut self.core else { return };
        core.art.queue(&core.library, &core.pinned_ids);
        let mut events = Vec::new();
        while let Ok(update) = core.art.updates_rx.try_recv() {
            match update {
                Update::Art(game_id) => events.push(Event::ArtChanged(GameRequest { game_id })),
                Update::Metadata(update, pins) => {
                    if core
                        .pinned_ids
                        .get(&update.game_id)
                        .cloned()
                        .unwrap_or_default()
                        != pins
                    {
                        continue;
                    }
                    let base = core.metadata_base_game(&update.game_id);
                    if let Some(game) = core
                        .library
                        .games_mut()
                        .iter_mut()
                        .find(|g| g.id == update.game_id)
                    {
                        crate::metadata::apply(game, base.as_ref(), &update.metadata);
                        if let Some(jobs) = &core.art.jobs {
                            jobs.art.refresh([game.clone()]);
                        }
                        events.push(Event::LibraryChanged);
                    }
                }
                Update::CandidatesReady {
                    game_id,
                    kind,
                    candidates,
                } => {
                    events.push(Event::ArtCandidatesReady(ArtCandidatesReady {
                        game_id,
                        kind,
                        candidates,
                    }));
                }
            }
        }
        for event in events {
            self.emit(event);
        }
    }
}
