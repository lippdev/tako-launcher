//! Arte e metadados fora da ponte: fila do Slint, cache compartilhado e avisos para o hospedeiro.

use crate::protocol::{
    ArtCandidateDto, ArtCandidatesReady, ArtCandidatesRequest, ArtCandidatesView, ArtChooseRequest,
    ArtConfidence, ArtDoubtDto, ArtKindDto, ArtResetRequest, ArtReviewItem, ArtReviewView,
    BridgeError, ErrorCode, Event, GameRequest,
};
use crate::{Core, HostRequest};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, mpsc};
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
type Key = (String, ArtKind);
type Job = Box<dyn FnOnce() + Send>;
const PENDING_LIMIT: usize = 16;

struct WorkQueue {
    sender: mpsc::SyncSender<Job>,
    stop: Arc<AtomicBool>,
}

impl WorkQueue {
    fn new() -> Self {
        let (sender, receiver) = mpsc::sync_channel::<Job>(PENDING_LIMIT);
        let receiver = Arc::new(Mutex::new(receiver));
        let stop = Arc::new(AtomicBool::new(false));
        for number in 0..2 {
            let receiver = receiver.clone();
            let stop = stop.clone();
            if let Err(error) = std::thread::Builder::new()
                .name(format!("art-requests-{number}"))
                .spawn(move || {
                    loop {
                        let job = receiver.lock().unwrap_or_else(|e| e.into_inner()).recv();
                        if stop.load(Ordering::Acquire) {
                            break;
                        }
                        let Ok(job) = job else { break };
                        job();
                    }
                })
            {
                tracing::error!("não foi possível iniciar worker de arte: {error}");
            }
        }
        Self { sender, stop }
    }

    fn submit(&self, job: impl FnOnce() + Send + 'static) -> Result<(), BridgeError> {
        self.sender
            .try_send(Box::new(job))
            .map_err(|error| match error {
                // Preserva pedidos aceitos: descartar o mais antigo poderia perder uma escolha do usuário.
                mpsc::TrySendError::Full(_) => {
                    BridgeError::new(ErrorCode::Conflict, "fila de arte cheia")
                }
                mpsc::TrySendError::Disconnected(_) => {
                    BridgeError::new(ErrorCode::Internal, "workers de arte indisponíveis")
                }
            })
    }
}

impl Drop for WorkQueue {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

#[derive(Default)]
struct SelectorState {
    candidates: HashMap<Key, Vec<ArtCandidate>>,
    in_flight: HashSet<Key>,
    versions: HashMap<Key, u64>,
    ids: HashMap<Key, BTreeMap<String, String>>,
    publications: HashMap<Key, Vec<String>>,
}

#[derive(Default)]
struct ReviewState {
    version: u64,
    loading: bool,
    publications: Vec<String>,
}

pub(crate) enum Update {
    Art(String),
    Metadata(Box<MetadataUpdated>, PinnedIds),
    CandidatesReady {
        game_id: String,
        kind: ArtKindDto,
        candidates: Vec<ArtCandidateDto>,
        version: u64,
    },
    Chosen {
        game_id: String,
        kind: ArtKind,
        version: u64,
        result: Result<(String, Option<std::path::PathBuf>), BridgeError>,
    },
    ReviewReady(ArtReviewView),
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
    pool: OnceLock<WorkQueue>,
    selector: Arc<Mutex<SelectorState>>,
    review: Arc<Mutex<ReviewState>>,
    choices_version: HashMap<Key, u64>,
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
    let bytes = preview_bytes(path)
        .ok_or_else(|| std::io::Error::other("imagem ausente ou grande demais"))?;
    std::fs::write(&target, bytes)?;
    Ok(target)
}

fn uuid_v4_simple() -> String {
    use std::time::SystemTime;
    let t = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "art-{:x}-{}-{}",
        t,
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

fn preview_bytes(path: &std::path::Path) -> Option<Vec<u8>> {
    tako_core::metadata::game_art::read_limited(
        path,
        tako_core::metadata::art_cache::MAX_IMAGE_BYTES,
    )
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
        self.clear_candidates(&game.id);
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
                pool: OnceLock::new(),
                selector: Arc::default(),
                review: Arc::default(),
                choices_version: HashMap::new(),
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
            pool: OnceLock::new(),
            selector: Arc::default(),
            review: Arc::default(),
            choices_version: HashMap::new(),
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
            pool: OnceLock::new(),
            selector: Arc::default(),
            review: Arc::default(),
            choices_version: HashMap::new(),
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

    fn submit(&self, job: impl FnOnce() + Send + 'static) -> Result<(), BridgeError> {
        self.pool.get_or_init(WorkQueue::new).submit(job)
    }

    fn clear_candidates(&self, game_id: &str) {
        let mut state = self.selector.lock().unwrap_or_else(|e| e.into_inner());
        state.versions.retain(|(id, _), _| id != game_id);
        state.ids.retain(|(id, _), _| id != game_id);
        state.candidates.retain(|(id, _), _| id != game_id);
        state.in_flight.retain(|(id, _)| id != game_id);
        state.publications.retain(|(id, kind), tokens| {
            if id != game_id {
                return true;
            }
            if let Some(art) = &self.game_art {
                for token in tokens {
                    let _ = art.candidates().clear(token, *kind);
                }
            }
            false
        });
    }

    fn close_review(&self) {
        let mut state = self.review.lock().unwrap_or_else(|e| e.into_inner());
        state.version = next_version();
        state.loading = false;
        if let Some(art) = &self.game_art {
            for token in state.publications.drain(..) {
                let _ = art.candidates().clear(&token, ArtKind::Cover);
            }
        }
    }
}

fn next_version() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn candidate_token(version: u64, index: usize) -> String {
    static SESSION: OnceLock<String> = OnceLock::new();
    format!(
        "{}-cand-{version}-{index}",
        SESSION.get_or_init(uuid_v4_simple)
    )
}

fn candidate_dtos(
    game: &Game,
    kind: ArtKind,
    version: u64,
    candidates: &[ArtCandidate],
) -> Vec<ArtCandidateDto> {
    candidates
        .iter()
        .enumerate()
        .map(|(idx, c)| {
            let token = candidate_token(version, idx);
            ArtCandidateDto {
                id: token.clone(),
                url: format!("tako-img://candidate-{}?id={token}", kind.key()),
                source: tako_core::metadata::art::source_label(&c.source),
                dimensions: c.size_text(),
                style: c.style.clone(),
                current: game.pinned_art.get(&kind).map_or_else(
                    || {
                        tako_core::metadata::art::pins::automatic(game, kind)
                            == Some(c.url.as_str())
                    },
                    |url| url == &c.url,
                ),
            }
        })
        .collect()
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

    pub(crate) fn art_candidates(
        &self,
        req: &ArtCandidatesRequest,
    ) -> Result<ArtCandidatesView, BridgeError> {
        let game = self
            .library
            .find(&req.game_id)
            .cloned()
            .ok_or_else(|| BridgeError::new(ErrorCode::NotFound, "jogo não encontrado"))?;
        let kind: ArtKind = req.kind.into();
        let key = (game.id.clone(), kind);
        let mut state = self.art.selector.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(list) = state.candidates.get(&key) {
            let version = state.versions.get(&key).copied().unwrap_or_default();
            return Ok(ArtCandidatesView {
                candidates: candidate_dtos(&game, kind, version, list),
                loading: false,
            });
        }
        if state.in_flight.contains(&key) {
            return Ok(ArtCandidatesView {
                candidates: Vec::new(),
                loading: true,
            });
        }
        let version = next_version();
        let tx = self.art.updates_tx.clone();
        let wake = self.art.wake.clone();
        let selector = self.art.selector.clone();
        let catalog = self.art.catalog.clone();
        let cache = self.art.cache.clone();
        let art = self.art.game_art.clone();
        let metadata_path = self.metadata_cache.clone();
        let pins = self.pinned_ids.get(&game.id).cloned().unwrap_or_default();
        let job_key = key.clone();
        self.art.submit(move || {
            // A geração também invalida trabalhos que ainda aguardavam na fila.
            if selector
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .versions
                .get(&job_key)
                != Some(&version)
            {
                return;
            }
            let metadata = MetadataCache::new(metadata_path);
            let ids = known_ids(
                &game,
                MetadataAgent::normalize(metadata.read(&game.id)).as_ref(),
                &pins,
            );
            {
                let mut state = selector.lock().unwrap_or_else(|e| e.into_inner());
                if state.versions.get(&job_key) != Some(&version) {
                    return;
                }
                state.ids.insert(job_key.clone(), ids.clone());
            }
            let raw: Vec<_> = if let Some(catalog) = &catalog {
                catalog
                    .sources(&game, &ids)
                    .into_iter()
                    .filter(|source| tako_core::metadata::art::offers(source, kind))
                    .flat_map(|source| {
                        catalog.list(
                            &tako_core::metadata::art_catalog::ArtQuery {
                                game: &game,
                                source,
                                kind,
                                filter: None,
                                steamgriddb_game: None,
                                page: 0,
                            },
                            &ids,
                        )
                    })
                    .take(50)
                    .collect()
            } else {
                ids.get("Steam")
                    .and_then(|id| id.parse().ok())
                    .map(tako_core::metadata::art::steam_candidates)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|c| c.kind == Some(kind))
                    .take(50)
                    .collect()
            };
            let mut files = Vec::with_capacity(raw.len());
            for candidate in &raw {
                if selector
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .versions
                    .get(&job_key)
                    != Some(&version)
                {
                    return;
                }
                let file = cache.as_ref().and_then(|cache| {
                    let file = cache.locate(candidate.preview())?;
                    preview_bytes(&file).and_then(|bytes| cache.import(candidate.preview(), &bytes))
                });
                files.push(file);
            }
            let current_ids = known_ids(
                &game,
                MetadataAgent::normalize(metadata.read(&game.id)).as_ref(),
                &pins,
            );
            let mut state = selector.lock().unwrap_or_else(|e| e.into_inner());
            if state.versions.get(&job_key) != Some(&version) {
                return;
            }
            state.in_flight.remove(&job_key);
            if ids != current_ids {
                send(
                    &tx,
                    &wake,
                    Update::CandidatesReady {
                        game_id: game.id,
                        kind: kind.into(),
                        candidates: Vec::new(),
                        version,
                    },
                );
                return;
            }
            let mut tokens = Vec::new();
            for (idx, file) in files.into_iter().enumerate() {
                if let (Some(art), Some(file)) = (&art, file) {
                    let token = candidate_token(version, idx);
                    if art.candidates().publish(&token, kind, &file).is_ok() {
                        tokens.push(token);
                    }
                }
            }
            state.publications.insert(job_key.clone(), tokens);
            state.ids.insert(job_key.clone(), ids);
            let dtos = candidate_dtos(&game, kind, version, &raw);
            state.candidates.insert(job_key, raw);
            send(
                &tx,
                &wake,
                Update::CandidatesReady {
                    game_id: game.id,
                    kind: kind.into(),
                    candidates: dtos,
                    version,
                },
            );
        })?;
        state.versions.insert(key.clone(), version);
        state.in_flight.insert(key);
        Ok(ArtCandidatesView {
            candidates: Vec::new(),
            loading: true,
        })
    }

    pub(crate) fn art_choose(&mut self, req: &ArtChooseRequest) -> Result<(), BridgeError> {
        let kind: ArtKind = req.kind.into();
        let key = (req.game_id.clone(), kind);
        if self.library.find(&req.game_id).is_none() {
            return Err(BridgeError::new(ErrorCode::NotFound, "jogo não encontrado"));
        }
        let state = self.art.selector.lock().unwrap_or_else(|e| e.into_inner());
        let generation = state.versions.get(&key).copied().unwrap_or_default();
        let candidate = state
            .candidates
            .get(&key)
            .and_then(|list| {
                list.iter()
                    .enumerate()
                    .find(|(idx, _)| candidate_token(generation, *idx) == req.candidate_id)
                    .map(|(_, c)| c.clone())
            })
            .ok_or_else(|| BridgeError::new(ErrorCode::NotFound, "candidato não encontrado"))?;
        drop(state);
        let version = next_version();
        let game_id = req.game_id.clone();
        let dir = self
            .art
            .paths
            .as_ref()
            .map(|p| p.art_choices())
            .ok_or_else(|| {
                BridgeError::new(ErrorCode::Internal, "pasta de imagens indisponível")
            })?;
        let tx = self.art.updates_tx.clone();
        let wake = self.art.wake.clone();
        let cache = self.art.cache.clone();
        self.art.submit(move || {
            let result = if candidate.source == tako_core::metadata::art::FILE_SOURCE {
                copy_chosen(&dir, std::path::Path::new(&candidate.url))
                    .map(|path| tako_core::library::manual::file_uri(&path.to_string_lossy()))
                    .map_err(|e| {
                        BridgeError::new(
                            ErrorCode::Internal,
                            format!("falha ao copiar imagem: {e}"),
                        )
                    })
            } else {
                Ok(candidate.url)
            };
            let result = result.and_then(|url| {
                let file = match &cache {
                    Some(cache) => {
                        let file = cache
                            .locate(&url)
                            .and_then(|file| preview_bytes(&file))
                            .and_then(|bytes| cache.import(&url, &bytes))
                            .ok_or_else(|| {
                                BridgeError::new(
                                    ErrorCode::Internal,
                                    "não foi possível preparar a imagem escolhida",
                                )
                            })?;
                        Some(file)
                    }
                    None => None,
                };
                Ok((url, file))
            });
            send(
                &tx,
                &wake,
                Update::Chosen {
                    game_id,
                    kind,
                    version,
                    result,
                },
            );
        })?;
        self.art.choices_version.insert(key, version);
        self.art.clear_candidates(&req.game_id);
        Ok(())
    }

    pub(crate) fn art_reset(&mut self, req: &ArtResetRequest) -> Result<(), BridgeError> {
        let game_id = &req.game_id;
        let kind: ArtKind = req.kind.into();
        self.art
            .choices_version
            .insert((game_id.clone(), kind), next_version());
        self.art.clear_candidates(game_id);

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
        let mut state = self.art.review.lock().unwrap_or_else(|e| e.into_inner());
        if state.loading {
            return Ok(ArtReviewView {
                total: self.library.games().len(),
                items: Vec::new(),
                loading: true,
                request_id: state.version,
            });
        }
        if let Some(art) = &self.art.game_art {
            for token in state.publications.drain(..) {
                let _ = art.candidates().clear(&token, ArtKind::Cover);
            }
        }
        let games = self.library.games().to_vec();
        let total = games.len();
        let version = next_version();
        let metadata_path = self.metadata_cache.clone();
        let review = self.art.review.clone();
        let art = self.art.game_art.clone();
        let image_cache = self.art.cache.clone();
        let tx = self.art.updates_tx.clone();
        let wake = self.art.wake.clone();
        self.art.submit(move || {
            let cache = MetadataCache::new(metadata_path);
            let total = games.len();
            let mut found: Vec<(Game, Review)> = games
                .into_iter()
                .filter_map(|g| {
                    if review.lock().unwrap_or_else(|e| e.into_inner()).version != version {
                        return None;
                    }
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
            for (idx, (game, r)) in found.into_iter().enumerate() {
                if review.lock().unwrap_or_else(|e| e.into_inner()).version != version {
                    return;
                }
                let current_cover = tako_core::metadata::art::pins::shown(&game, ArtKind::Cover)
                    .map(|_| format!("tako-img://cover?id={}", percent_encode(&game.id)));
                let proposed_cover =
                    r.doubt
                        .as_ref()
                        .and_then(|d| d.cover.as_ref())
                        .and_then(|uri| {
                            let (Some(cache), Some(art)) = (&image_cache, &art) else {
                                return None;
                            };
                            let file = cache
                                .locate(uri)
                                .and_then(|file| preview_bytes(&file))
                                .and_then(|bytes| cache.import(uri, &bytes))?;
                            let token = format!("review-{}", candidate_token(version, idx));
                            let mut state = review.lock().unwrap_or_else(|e| e.into_inner());
                            if state.version != version {
                                return None;
                            }
                            art.candidates()
                                .publish(&token, ArtKind::Cover, &file)
                                .ok()?;
                            state.publications.push(token.clone());
                            Some(format!("tako-img://candidate-cover?id={token}"))
                        });

                let confidence =
                    r.doubt
                        .as_ref()
                        .map_or(ArtConfidence::Low, |d| match d.confidence() {
                            tako_core::metadata::review::Confidence::High => ArtConfidence::High,
                            tako_core::metadata::review::Confidence::Medium => {
                                ArtConfidence::Medium
                            }
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

            let mut state = review.lock().unwrap_or_else(|e| e.into_inner());
            if state.version != version {
                return;
            }
            state.loading = false;
            send(
                &tx,
                &wake,
                Update::ReviewReady(ArtReviewView {
                    total,
                    items,
                    loading: false,
                    request_id: version,
                }),
            );
        })?;
        state.version = version;
        state.loading = true;
        Ok(ArtReviewView {
            total,
            items: Vec::new(),
            loading: true,
            request_id: version,
        })
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
    pub(crate) fn handle_async_art(
        &mut self,
        call: &crate::protocol::Call,
    ) -> Option<Result<crate::protocol::Reply, BridgeError>> {
        use crate::protocol::{Call, Reply};
        if !matches!(
            call,
            Call::ArtChoose(_) | Call::ArtClose(_) | Call::ArtReviewClose
        ) {
            return None;
        }
        let Some(core) = &mut self.core else {
            return Some(Err(BridgeError::new(
                ErrorCode::Internal,
                "núcleo não aberto",
            )));
        };
        Some(match call {
            Call::ArtChoose(request) => core.art_choose(request).map(|()| Reply::ArtChoose),
            Call::ArtClose(request) => {
                core.art.clear_candidates(&request.game_id);
                Ok(Reply::ArtClose)
            }
            Call::ArtReviewClose => {
                core.art.close_review();
                Ok(Reply::ArtReviewClose)
            }
            _ => unreachable!(),
        })
    }

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
                    if let Some(game) = core.library.find(&update.game_id) {
                        let ids = known_ids(game, Some(&update.metadata), &pins);
                        let changed = core
                            .art
                            .selector
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .ids
                            .iter()
                            .any(|((game_id, _), old)| game_id == &update.game_id && old != &ids);
                        if changed {
                            core.art.clear_candidates(&update.game_id);
                        }
                    }
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
                    version,
                } => {
                    if core
                        .art
                        .selector
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .versions
                        .get(&(game_id.clone(), kind.into()))
                        != Some(&version)
                    {
                        continue;
                    }
                    events.push(Event::ArtCandidatesReady(ArtCandidatesReady {
                        game_id,
                        kind,
                        candidates,
                    }));
                }
                Update::Chosen {
                    game_id,
                    kind,
                    version,
                    result,
                } => {
                    if core.art.choices_version.get(&(game_id.clone(), kind)) != Some(&version) {
                        continue;
                    }
                    match result {
                        Ok((url, file)) => {
                            if let Some(choices) = &core.choices
                                && let Err(error) = choices.set_art(&game_id, kind, Some(&url))
                            {
                                events.push(Event::ArtChooseFailed(
                                    crate::protocol::ArtChooseFailure {
                                        game_id,
                                        error: BridgeError::new(
                                            ErrorCode::Internal,
                                            error.to_string(),
                                        ),
                                    },
                                ));
                                continue;
                            }
                            if let Some(game) = core
                                .library
                                .games_mut()
                                .iter_mut()
                                .find(|g| g.id == game_id)
                            {
                                game.pinned_art = tako_core::metadata::art::pins::with(
                                    &game.pinned_art,
                                    kind,
                                    Some(&url),
                                );
                                if let (Some(art), Some(file)) = (&core.art.game_art, file)
                                    && let Err(error) = art.publish(&game_id, kind, &file)
                                {
                                    tracing::warn!("falha ao publicar imagem escolhida: {error}");
                                }
                                events.push(Event::ArtChanged(GameRequest {
                                    game_id: game_id.clone(),
                                }));
                                events.push(Event::ArtChooseReady(GameRequest { game_id }));
                                events.push(Event::LibraryChanged);
                            }
                        }
                        Err(error) => {
                            events.push(Event::ArtChooseFailed(crate::protocol::ArtChooseFailure {
                                game_id,
                                error,
                            }))
                        }
                    }
                }
                Update::ReviewReady(view) => {
                    if core
                        .art
                        .review
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .version
                        == view.request_id
                    {
                        events.push(Event::ArtReviewReady(view));
                    }
                }
            }
        }
        for event in events {
            self.emit(event);
        }
    }
}

#[cfg(test)]
mod regression_tests {
    use super::*;
    use std::time::Duration;
    use tako_core::metadata::http::{Http, HttpRequest, HttpResponse};
    use tako_core::metadata::steam_art::SteamArtResolver;

    struct GateImage {
        entered: mpsc::Sender<()>,
        release: Mutex<Option<mpsc::Receiver<()>>>,
    }

    impl Http for GateImage {
        fn send(&self, _: &HttpRequest) -> Result<HttpResponse, String> {
            let release = self.release.lock().unwrap().take();
            if let Some(release) = release {
                self.entered.send(()).unwrap();
                release.recv_timeout(Duration::from_secs(5)).unwrap();
            }
            Ok(HttpResponse {
                status: 200,
                body: b"\x89PNG\r\n\x1a\nproposed".to_vec(),
                content_type: Some("image/png".into()),
                retry_after: None,
            })
        }
    }

    fn image_cache(paths: &DataPaths, http: Arc<GateImage>) -> Arc<ArtCache> {
        Arc::new(ArtCache::new(
            paths.art_cache(),
            http.clone(),
            SteamArtResolver::new(http, None),
            None,
        ))
    }

    fn core() -> (tempfile::TempDir, Core) {
        let dir = tempfile::tempdir().unwrap();
        let core = Core::open(DataPaths::at(dir.path()), Platform::simulated(), true).unwrap();
        (dir, core)
    }

    fn candidate(core: &mut Core) {
        core.library = GameLibrary::in_memory(
            vec![Game {
                id: "g".into(),
                ..Default::default()
            }],
            tako_core::library::system_clock(),
        );
        core.art.selector.lock().unwrap().candidates.insert(
            ("g".into(), ArtKind::Cover),
            vec![ArtCandidate::new(
                "Steam",
                ArtKind::Cover,
                "https://cdn.steamstatic.com/a.png",
            )],
        );
    }

    #[test]
    fn preview_read_rejects_oversized_local_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large.png");
        std::fs::File::create(&path)
            .unwrap()
            .set_len(tako_core::metadata::art_cache::MAX_IMAGE_BYTES + 1)
            .unwrap();
        assert!(preview_bytes(&path).is_none());
    }

    #[test]
    fn choose_defers_mutation_to_background_completion() {
        let (_dir, mut core) = core();
        candidate(&mut core);
        core.art_choose(&ArtChooseRequest {
            game_id: "g".into(),
            kind: ArtKindDto::Cover,
            candidate_id: candidate_token(0, 0),
        })
        .unwrap();
        assert!(core.library.find("g").unwrap().pinned_art.is_empty());
    }

    #[test]
    fn local_choice_copies_on_worker_and_announces_only_after_completion() {
        let (dir, mut core) = core();
        candidate(&mut core);
        let source = dir.path().join("original.png");
        std::fs::write(&source, b"\x89PNG\r\n\x1a\nlocal").unwrap();
        core.art
            .selector
            .lock()
            .unwrap()
            .candidates
            .get_mut(&("g".into(), ArtKind::Cover))
            .unwrap()[0] = ArtCandidate::new(
            tako_core::metadata::art::FILE_SOURCE,
            ArtKind::Cover,
            source.to_string_lossy(),
        );
        let gate = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        let (entered, started) = mpsc::channel();
        for _ in 0..2 {
            let gate = gate.clone();
            let entered = entered.clone();
            core.art
                .submit(move || {
                    entered.send(()).unwrap();
                    let (lock, ready) = &*gate;
                    let mut open = lock.lock().unwrap();
                    while !*open {
                        open = ready.wait(open).unwrap();
                    }
                })
                .unwrap();
        }
        for _ in 0..2 {
            started.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        let choices = DataPaths::at(dir.path()).art_choices();
        let mut app = crate::AppService::with_core("test", core);
        let (events, received) = mpsc::channel();
        app.set_event_sink(events);
        let before = std::time::Instant::now();
        app.handle(crate::protocol::Call::ArtChoose(ArtChooseRequest {
            game_id: "g".into(),
            kind: ArtKindDto::Cover,
            candidate_id: candidate_token(0, 0),
        }))
        .unwrap();
        assert!(before.elapsed() < Duration::from_millis(400));
        assert!(!choices.exists());
        assert!(
            received
                .try_iter()
                .all(|event| !matches!(event, Event::ArtChanged(_) | Event::LibraryChanged))
        );
        *gate.0.lock().unwrap() = true;
        gate.1.notify_all();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            app.poll();
            if received
                .try_iter()
                .any(|event| matches!(event, Event::ArtChooseReady(_)))
            {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "faltou conclusão da cópia"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        let copied = std::fs::read_dir(choices)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        std::fs::remove_file(source).unwrap();
        assert_eq!(std::fs::read(copied).unwrap(), b"\x89PNG\r\n\x1a\nlocal");
    }

    #[test]
    fn review_returns_loading_before_reading_library_cache() {
        let (_dir, core) = core();
        let view = core.art_review().unwrap();
        assert_eq!(serde_json::to_value(view).unwrap()["loading"], true);
    }

    #[test]
    fn source_id_change_invalidates_cached_candidates() {
        let (_dir, mut core) = core();
        candidate(&mut core);
        core.pinned_ids
            .insert("g".into(), [("IGDB".into(), "2".into())].into());
        core.refresh_metadata("g");
        assert!(core.art.selector.lock().unwrap().candidates.is_empty());
    }

    #[test]
    fn choosing_releases_candidate_list() {
        let (dir, mut core) = core();
        candidate(&mut core);
        let paths = DataPaths::at(dir.path());
        std::fs::create_dir_all(paths.art_cache()).unwrap();
        let file = paths.art_cache().join("preview.png");
        std::fs::write(&file, b"\x89PNG\r\n\x1a\n").unwrap();
        let token = candidate_token(0, 0);
        let art = core.art.game_art.clone().unwrap();
        art.candidates()
            .publish(&token, ArtKind::Cover, &file)
            .unwrap();
        core.art
            .selector
            .lock()
            .unwrap()
            .publications
            .insert(("g".into(), ArtKind::Cover), vec![token.clone()]);
        core.art_choose(&ArtChooseRequest {
            game_id: "g".into(),
            kind: ArtKindDto::Cover,
            candidate_id: candidate_token(0, 0),
        })
        .unwrap();
        assert!(core.art.selector.lock().unwrap().candidates.is_empty());
        assert!(art.candidates().resolve(&token, ArtKind::Cover).is_none());
    }

    #[test]
    fn request_queue_limits_workers_and_rejects_excess_pending_jobs() {
        let (_dir, mut core) = core();
        candidate(&mut core);
        core.art.clear_candidates("g");
        let pool = core.art.pool.get_or_init(WorkQueue::new);
        let gate = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        let (entered, started) = mpsc::channel();
        for _ in 0..2 {
            let gate = gate.clone();
            let entered = entered.clone();
            pool.submit(move || {
                entered.send(()).unwrap();
                let (lock, ready) = &*gate;
                let mut open = lock.lock().unwrap();
                while !*open {
                    open = ready.wait(open).unwrap();
                }
            })
            .unwrap();
        }
        for _ in 0..2 {
            started
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
        }
        for _ in 0..PENDING_LIMIT {
            let gate = gate.clone();
            let entered = entered.clone();
            pool.submit(move || {
                entered.send(()).unwrap();
                let (lock, ready) = &*gate;
                let mut open = lock.lock().unwrap();
                while !*open {
                    open = ready.wait(open).unwrap();
                }
            })
            .unwrap();
        }
        let extra_worker = started.recv_timeout(Duration::from_millis(100));
        let rejected = pool.submit(|| {});
        let reply = core.art_candidates(&ArtCandidatesRequest {
            game_id: "g".into(),
            kind: ArtKindDto::Cover,
        });
        *gate.0.lock().unwrap() = true;
        gate.1.notify_all();
        assert!(extra_worker.is_err(), "mais de dois workers executando");
        assert_eq!(rejected.unwrap_err().code, ErrorCode::Conflict);
        assert_eq!(reply.unwrap_err().code, ErrorCode::Conflict);
        let state = core.art.selector.lock().unwrap();
        assert!(state.in_flight.is_empty());
        assert!(state.versions.is_empty());
    }

    #[test]
    fn review_publishes_the_proposed_image_and_releases_it_on_close() {
        let (dir, mut core) = core();
        let paths = DataPaths::at(dir.path());
        let (entered, _) = mpsc::channel();
        let cache = image_cache(
            &paths,
            Arc::new(GateImage {
                entered,
                release: Mutex::new(None),
            }),
        );
        let current = cache
            .import("current", b"\x89PNG\r\n\x1a\ncurrent")
            .unwrap();
        let art = core.art.game_art.clone().unwrap();
        art.publish("g", ArtKind::Cover, &current).unwrap();
        core.art.cache = Some(cache);
        core.library = GameLibrary::in_memory(
            vec![Game {
                id: "g".into(),
                title: "Different".into(),
                cover_uri: Some("current".into()),
                ..Default::default()
            }],
            tako_core::library::system_clock(),
        );
        MetadataCache::new(core.metadata_cache.clone())
            .write(
                "g",
                &tako_core::metadata::GameMetadata {
                    source_ids: [("IGDB".into(), "12".into())].into(),
                    title: Some("Other title".into()),
                    matched_by_title: true,
                    cover_url: Some("https://images.igdb.com/proposed.png".into()),
                    ..tako_core::metadata::GameMetadata::new("IGDB")
                },
            )
            .unwrap();
        let loading = core.art_review().unwrap();
        assert!(loading.loading);
        let Update::ReviewReady(view) = core
            .art
            .updates_rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
        else {
            panic!("faltou revisão");
        };
        let item = &view.items[0];
        let proposed = item.proposed_cover.as_ref().expect("faltou capa proposta");
        assert_ne!(Some(proposed), item.current_cover.as_ref());
        let token = proposed
            .strip_prefix("tako-img://candidate-cover?id=")
            .unwrap();
        let file = art.candidates().resolve(token, ArtKind::Cover).unwrap();
        assert_eq!(std::fs::read(file).unwrap(), b"\x89PNG\r\n\x1a\nproposed");
        assert_eq!(
            art.resolve("g", ArtKind::Cover),
            Some(current.canonicalize().unwrap())
        );
        core.art.close_review();
        assert!(art.candidates().resolve(token, ArtKind::Cover).is_none());
    }

    #[test]
    fn changed_source_ids_and_close_discard_pending_workers_and_old_events() {
        let (dir, mut core) = core();
        let (entered, started) = mpsc::channel();
        let (release, released) = mpsc::channel();
        core.art.cache = Some(image_cache(
            &DataPaths::at(dir.path()),
            Arc::new(GateImage {
                entered,
                release: Mutex::new(Some(released)),
            }),
        ));
        core.library = GameLibrary::in_memory(
            vec![Game {
                id: "g".into(),
                steam_app_id: Some(1),
                ..Default::default()
            }],
            tako_core::library::system_clock(),
        );
        let request = ArtCandidatesRequest {
            game_id: "g".into(),
            kind: ArtKindDto::Cover,
        };
        assert!(core.art_candidates(&request).unwrap().loading);
        started.recv_timeout(Duration::from_secs(5)).unwrap();
        core.pinned_ids
            .insert("g".into(), [("Steam".into(), "2".into())].into());
        core.refresh_metadata("g");
        assert!(core.art_candidates(&request).unwrap().loading);
        let Update::CandidatesReady { candidates, .. } = core
            .art
            .updates_rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
        else {
            panic!("faltaram candidatos novos");
        };
        let expected: Vec<_> = tako_core::metadata::art::steam_candidates(2)
            .into_iter()
            .filter(|c| c.kind == Some(ArtKind::Cover))
            .collect();
        assert_eq!(
            core.art.selector.lock().unwrap().candidates[&("g".into(), ArtKind::Cover)],
            expected
        );
        let art = core.art.game_art.clone().unwrap();
        for c in &candidates {
            assert!(art.candidates().resolve(&c.id, ArtKind::Cover).is_some());
        }
        core.art.clear_candidates("g");
        release.send(()).unwrap();
        assert!(
            core.art
                .updates_rx
                .recv_timeout(Duration::from_millis(150))
                .is_err()
        );
        for c in &candidates {
            assert!(art.candidates().resolve(&c.id, ArtKind::Cover).is_none());
        }
        assert!(core.art.selector.lock().unwrap().candidates.is_empty());
    }
}
