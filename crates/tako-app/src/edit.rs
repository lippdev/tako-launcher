//! Editar jogo, adicionar jogo, remover da biblioteca e "Fontes e IDs" (WEB-12c). As regras ficam no `tako-core`
//! (`library::manual_edit`, `library::manual_add`, `metadata`); aqui só validamos o contrato e gravamos.
//!
//! Buscar e conferir ids falam com a rede, então rodam em threads próprias: a chamada responde na hora com um
//! número e o resultado volta como evento (`game.source_ids.found` e `.checked`).

use crate::organize::{bad, internal};
use crate::protocol::*;
use crate::{AppService, Core, HostRequest};
use chrono::Datelike;
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use tako_core::library::manual_add::{self, AddKind};
use tako_core::library::manual_edit::{EditDraft, EditError};
use tako_core::library::{Game, GameTrackingStrategy, LibraryError, ManualGameRecord};
use tako_core::metadata::agent::{MetadataAgent, PinnedIds};
use tako_core::metadata::art::{parse_source_id, source_id_example};
use tako_core::metadata::art_catalog::known_ids;
use tako_core::metadata::cache::MetadataCache;
use tako_core::metadata::igdb::IgdbSettings;
use tako_core::metadata::screenscraper::ScreenScraperSettings;
use tako_core::metadata::screenscraper_parse::{
    EmulatedSystem, SYSTEMS, candidates_for, rom_title,
};
use tako_core::metadata::steamgriddb::SteamGridDbSettings;
use tako_core::metadata::title_matcher;
use tako_core::metadata::{MetadataMatch, SecretStore};
use tako_core::paths::DataPaths;

const MAX_TITLE_CHARS: usize = 200;
const MAX_ARGUMENTS_CHARS: usize = 2048;
const MAX_FOLDER_CHARS: usize = 1024;
const MAX_PATH_CHARS: usize = 1024;
const MAX_ID_CHARS: usize = 128;
const MAX_TERM_CHARS: usize = 200;

fn missing(what: &str) -> BridgeError {
    BridgeError::new(ErrorCode::NotFound, what)
}

/// Texto de uma linha só, sem caractere de controle, dentro do limite.
fn one_line(value: &str, max: usize, what: &str) -> Result<(), BridgeError> {
    if value.chars().count() > max || value.chars().any(char::is_control) {
        return Err(bad(&format!("{what} inválido")));
    }
    Ok(())
}

fn game_id(value: &str) -> Result<(), BridgeError> {
    if value.trim().is_empty() || value.chars().count() > 128 || value.contains('\0') {
        Err(bad("id inválido"))
    } else {
        Ok(())
    }
}

// ---------- Editar e remover ----------

fn tracking_of(track_by: Option<GameTrackingStrategy>) -> GameTracking {
    match track_by {
        Some(GameTrackingStrategy::Process) => GameTracking::Process,
        Some(GameTrackingStrategy::InstallFolder) => GameTracking::InstallFolder,
        Some(GameTrackingStrategy::None) => GameTracking::None,
        // O sinal da Steam não existe para jogo adicionado à mão: vale o automático.
        Some(GameTrackingStrategy::SteamApp) | None => GameTracking::Auto,
    }
}

fn strategy_of(tracking: GameTracking) -> Option<GameTrackingStrategy> {
    match tracking {
        GameTracking::Auto => None,
        GameTracking::Process => Some(GameTrackingStrategy::Process),
        GameTracking::InstallFolder => Some(GameTrackingStrategy::InstallFolder),
        GameTracking::None => Some(GameTrackingStrategy::None),
    }
}

/// O registro do jogo adicionado à mão; jogo de loja não se edita (`conflict`).
fn manual(core: &Core, id: &str) -> Result<ManualGameRecord, BridgeError> {
    game_id(id)?;
    if core.library.find(id).is_none() {
        return Err(missing("jogo não encontrado"));
    }
    core.library
        .find_manual(id)
        .cloned()
        .ok_or_else(|| BridgeError::new(ErrorCode::Conflict, "só jogos adicionados à mão têm isto"))
}

fn edit_view(record: &ManualGameRecord) -> GameEdit {
    let draft = EditDraft::of(record);
    GameEdit {
        game_id: record.id.clone(),
        title: draft.title,
        target: record.path.clone(),
        arguments: draft.arguments,
        working_directory: draft.folder,
        default_folder: tako_core::library::manual_edit::default_folder(&record.path),
        tracking: tracking_of(record.track_by),
    }
}

fn library_error(error: LibraryError) -> BridgeError {
    match error {
        LibraryError::NotFound(_) | LibraryError::NotManual(_) => missing("jogo não encontrado"),
        error => internal(error),
    }
}

fn edit_set(core: &mut Core, request: GameEditSet) -> Result<GameEdit, BridgeError> {
    let record = manual(core, &request.game_id)?;
    one_line(&request.title, MAX_TITLE_CHARS, "nome")?;
    one_line(&request.arguments, MAX_ARGUMENTS_CHARS, "argumentos")?;
    one_line(
        &request.working_directory,
        MAX_FOLDER_CHARS,
        "pasta de trabalho",
    )?;
    let edited = record
        .edited(&EditDraft {
            title: request.title,
            arguments: request.arguments,
            folder: request.working_directory,
            track_by: strategy_of(request.tracking),
        })
        .map_err(|e| match e {
            EditError::NameRequired => bad("o jogo precisa de um nome"),
            EditError::FolderMissing(folder) => BridgeError::new(
                ErrorCode::NotFound,
                format!("pasta não encontrada: {folder}"),
            ),
        })?;
    core.library
        .update_manual(edited.clone())
        .map_err(library_error)?;
    Ok(edit_view(&edited))
}

fn remove(core: &mut Core, request: GameRequest) -> Result<(), BridgeError> {
    manual(core, &request.game_id)?;
    core.library
        .remove_manual(&request.game_id)
        .map_err(library_error)
}

// ---------- Adicionar ----------

fn added(game: &Game) -> GameAddResult {
    GameAddResult {
        status: GameAddStatus::Added,
        game_id: Some(game.id.clone()),
        title: Some(game.title.clone()),
        file_name: None,
        systems: Vec::new(),
        others: Vec::new(),
    }
}

fn plain(status: GameAddStatus, title: Option<String>) -> GameAddResult {
    GameAddResult {
        status,
        game_id: None,
        title,
        file_name: None,
        systems: Vec::new(),
        others: Vec::new(),
    }
}

fn rom_systems(list: impl IntoIterator<Item = &'static EmulatedSystem>) -> Vec<RomSystem> {
    list.into_iter()
        .map(|s| RomSystem {
            key: s.key.to_owned(),
            name: s.name.to_owned(),
        })
        .collect()
}

/// O título do jogo de PC: o nome do produto do `.exe` no Windows; senão o do arquivo.
fn title_of(path: &Path) -> String {
    let is_exe = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("exe"));
    #[cfg(windows)]
    let names = if is_exe {
        tako_platform::windows::version_info::names(path)
    } else {
        [None, None]
    };
    #[cfg(not(windows))]
    let names = {
        let _ = is_exe;
        [None, None]
    };
    manual_add::guess_title(path, names)
}

/// Pergunta o sistema da ROM: os prováveis pela extensão e, à parte, todos os outros.
fn needs_system(path: &Path, text: &str) -> GameAddResult {
    let probable = candidates_for(text);
    let (systems, others) = if probable.is_empty() {
        (rom_systems(SYSTEMS.iter()), Vec::new())
    } else {
        let others = SYSTEMS.iter().filter(|s| !probable.contains(s));
        (rom_systems(probable.iter().copied()), rom_systems(others))
    };
    GameAddResult {
        file_name: Some(
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
        ),
        systems,
        others,
        ..plain(GameAddStatus::NeedsSystem, None)
    }
}

fn add_game(core: &mut Core, request: GameAdd) -> Result<(GameAddResult, bool), BridgeError> {
    let text = request.path.trim();
    if text.is_empty() || text.chars().count() > MAX_PATH_CHARS || text.contains('\0') {
        return Err(bad("caminho inválido"));
    }
    let path = Path::new(text);
    let exists = path.is_file()
        || (path.is_dir()
            && path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("app")));
    if !exists {
        return Err(missing("arquivo não encontrado"));
    }
    if let Some(known) = manual_add::find_by_target(core.library.games(), text) {
        return Ok((
            plain(GameAddStatus::Already, Some(known.title.clone())),
            false,
        ));
    }
    let kind = manual_add::kind_of(path);
    if request.system.is_some() && kind != AddKind::Rom {
        return Err(bad("sistema só vale para ROM"));
    }
    let now = chrono::Utc::now();
    let record = match kind {
        AddKind::Desktop => {
            let Some((entry, (target, args))) =
                tako_sources::desktop_entries::read_desktop_file(path)
                    .and_then(|e| e.launch().map(|l| (e, l)))
            else {
                return Ok((plain(GameAddStatus::Unreadable, None), false));
            };
            let mut record = ManualGameRecord::create(&entry.name, &target, now);
            record.arguments = args;
            if entry.working_dir.is_some() {
                record.working_directory = entry.working_dir;
            }
            record
        }
        AddKind::MacApp => {
            let name = tako_sources::mac_apps::read_app(path)
                .map(|a| a.name)
                .unwrap_or_else(|| title_of(path));
            ManualGameRecord::create(&name, text, now)
        }
        AddKind::Rom => {
            let Some(key) = request.system.as_deref() else {
                return Ok((needs_system(path, text), false));
            };
            let Some(system) = SYSTEMS.iter().find(|s| s.key == key) else {
                return Err(bad("sistema desconhecido"));
            };
            ManualGameRecord::create_rom(&rom_title(text), text, system.key, now)
        }
        AddKind::Program => ManualGameRecord::create(&title_of(path), text, now),
    };
    let game = core.library.add_manual(record).map_err(library_error)?;
    Ok((added(game), true))
}

// ---------- Fontes e IDs ----------

fn year_of(date: Option<chrono::NaiveDate>) -> Option<i32> {
    date.map(|d| d.year())
}

fn source_states(core: &Core, game: &Game) -> Vec<SourceIdState> {
    let Some(agent) = &core.metadata else {
        return Vec::new();
    };
    let cached =
        MetadataAgent::normalize(MetadataCache::new(core.metadata_cache.clone()).read(&game.id));
    let pinned = core.pinned_ids.get(&game.id).cloned().unwrap_or_default();
    let known = known_ids(game, cached.as_ref(), &pinned);
    agent
        .sources(game)
        .iter()
        .map(|p| {
            let name = p.name().to_owned();
            let part = cached
                .as_ref()
                .and_then(|c| c.parts.iter().find(|x| x.source == name));
            let none = pinned.get(&name).is_some_and(String::is_empty);
            SourceIdState {
                id: if none {
                    None
                } else {
                    known.get(&name).cloned()
                },
                title: part.and_then(|p| p.title.clone()),
                year: year_of(part.and_then(|p| p.release_date)),
                pinned: pinned.contains_key(&name),
                none,
                example: source_id_example(&name),
                source: name,
            }
        })
        .collect()
}

fn find_game(core: &Core, id: &str) -> Result<Game, BridgeError> {
    game_id(id)?;
    core.library
        .find(id)
        .cloned()
        .ok_or_else(|| missing("jogo não encontrado"))
}

fn source_ids_view(core: &Core, game: &Game) -> GameSourceIds {
    GameSourceIds {
        game_id: game.id.clone(),
        title: game.title.clone(),
        sources: source_states(core, game),
    }
}

/// A fonte tem que ser uma das ligadas que atendem o jogo.
fn check_source(core: &Core, game: &Game, source: &str) -> Result<(), BridgeError> {
    let known = core
        .metadata
        .as_ref()
        .is_some_and(|a| a.sources(game).iter().any(|p| p.name() == source));
    if known {
        Ok(())
    } else {
        Err(bad("fonte desconhecida"))
    }
}

/// Fixa o id da fonte (`Some("")` = "nenhum", `None` = volta ao automático). Grava no banco antes de mudar a memória.
fn source_ids_set(core: &mut Core, request: GameSourceIdSet) -> Result<GameSourceIds, BridgeError> {
    let game = find_game(core, &request.game_id)?;
    check_source(core, &game, &request.source)?;
    let value = match request.choice {
        SourceIdChoice::Id(id) => {
            let id = id.trim().to_owned();
            if id.is_empty() {
                return Err(bad("id vazio"));
            }
            one_line(&id, MAX_ID_CHARS, "id")?;
            Some(id)
        }
        SourceIdChoice::None => Some(String::new()),
        SourceIdChoice::Auto => None,
    };
    if let Some(choices) = &core.choices {
        choices
            .set_id(&game.id, &request.source, value.as_deref())
            .map_err(internal)?;
    }
    let ids = core.pinned_ids.entry(game.id.clone()).or_default();
    match value {
        None => ids.remove(&request.source),
        Some(id) => ids.insert(request.source, id),
    };
    core.refresh_metadata(&game.id);
    Ok(source_ids_view(core, &game))
}

type Results = Vec<(String, Vec<MetadataMatch>)>;

/// Busca o termo em todas as fontes ao mesmo tempo (cada uma no próprio ritmo), na ordem das fontes.
fn search_all(agent: &MetadataAgent, game: &Game, term: &str) -> Results {
    let sources: Vec<String> = agent
        .sources(game)
        .iter()
        .map(|p| p.name().to_owned())
        .collect();
    std::thread::scope(|scope| {
        let jobs: Vec<_> = sources
            .iter()
            .map(|source| {
                let name = source.clone();
                scope.spawn(move || (name.clone(), agent.search_source(game, &name, term)))
            })
            .collect();
        jobs.into_iter()
            .zip(&sources)
            .map(|(job, source)| job.join().unwrap_or_else(|_| (source.clone(), Vec::new())))
            .collect()
    })
}

fn found_event(request_id: u32, game: &Game, term: &str, results: Results) -> SourceIdsFound {
    SourceIdsFound {
        request_id,
        game_id: game.id.clone(),
        term: term.to_owned(),
        results: results
            .into_iter()
            .map(|(source, found)| {
                let best = found
                    .iter()
                    .find(|m| m.is_exact)
                    .or_else(|| title_matcher::best(term, &found))
                    .map(|m| m.id.clone());
                SourceIdResults {
                    source,
                    best,
                    matches: found
                        .into_iter()
                        .map(|m| SourceIdMatch {
                            id: m.id,
                            title: m.title,
                            year: m.year,
                            platforms: m.platforms.filter(|p| !p.is_empty()),
                        })
                        .collect(),
                }
            })
            .collect(),
    }
}

// ---------- Trabalho em segundo plano ----------

pub(crate) enum Done {
    Found(SourceIdsFound),
    Checked(SourceIdsChecked),
}

/// Buscas e conferências de ids em andamento. Nenhuma thread recebe referência ao núcleo.
pub(crate) struct Jobs {
    sender: Sender<Done>,
    receiver: Receiver<Done>,
    wake: Arc<Mutex<Option<Sender<HostRequest>>>>,
    next: u32,
}

impl Jobs {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            sender,
            receiver,
            wake: Arc::default(),
            next: 0,
        }
    }

    pub fn set_host_sink(&mut self, sink: Sender<HostRequest>) {
        *self.wake.lock().unwrap_or_else(|e| e.into_inner()) = Some(sink);
    }

    fn start(&mut self, work: impl FnOnce(u32) -> Done + Send + 'static) -> SourceIdsJob {
        self.next = self.next.wrapping_add(1);
        let request_id = self.next;
        let (sender, wake) = (self.sender.clone(), Arc::clone(&self.wake));
        let spawned = std::thread::Builder::new()
            .name("source-ids".into())
            .spawn(move || {
                let Ok(done) = catch_unwind(AssertUnwindSafe(|| work(request_id))) else {
                    tracing::error!("busca de ids encerrou com pânico");
                    return;
                };
                if sender.send(done).is_ok()
                    && let Some(host) = &*wake.lock().unwrap_or_else(|e| e.into_inner())
                {
                    let _ = host.send(HostRequest::PollBackground);
                }
            });
        if let Err(e) = spawned {
            tracing::error!("não foi possível iniciar a busca de ids: {e}");
        }
        SourceIdsJob { request_id }
    }
}

fn start_search(
    core: &mut Core,
    request: GameSourceIdsSearch,
) -> Result<SourceIdsJob, BridgeError> {
    let game = find_game(core, &request.game_id)?;
    let term = request.term.trim().to_owned();
    if term.is_empty() {
        return Err(bad("nome vazio"));
    }
    one_line(&term, MAX_TERM_CHARS, "nome")?;
    let agent = core.metadata.clone();
    Ok(core.jobs.start(move |request_id| {
        let results = agent
            .map(|agent| search_all(&agent, &game, &term))
            .unwrap_or_default();
        Done::Found(found_event(request_id, &game, &term, results))
    }))
}

fn start_check(core: &mut Core, request: GameSourceIdsCheck) -> Result<SourceIdsJob, BridgeError> {
    let game = find_game(core, &request.game_id)?;
    check_source(core, &game, &request.source)?;
    one_line(&request.text, MAX_PATH_CHARS, "id")?;
    let input = parse_source_id(&request.source, &request.text);
    if let Some(problem) = input.problem.clone() {
        return Err(BridgeError::new(ErrorCode::BadRequest, problem));
    }
    let (agent, source) = (core.metadata.clone(), request.source);
    Ok(core.jobs.start(move |request_id| {
        let found = agent
            .and_then(|agent| agent.validate(&source, &input))
            .map(|(id, m)| {
                let mut name = m.title.clone().unwrap_or_else(|| id.clone());
                if let Some(year) = year_of(m.release_date) {
                    name.push_str(&format!(" ({year})"));
                }
                SourceIdFound { id, name }
            });
        Done::Checked(SourceIdsChecked {
            request_id,
            game_id: game.id,
            source,
            found,
        })
    }))
}

// ---------- Ligação com o serviço ----------

fn run(core: &mut Core, call: Call) -> Result<(Reply, bool), BridgeError> {
    Ok(match call {
        Call::GameEdit(request) => {
            let record = manual(core, &request.game_id)?;
            (Reply::GameEdit(edit_view(&record)), false)
        }
        Call::GameEditSet(request) => (Reply::GameEditSet(edit_set(core, request)?), true),
        Call::GameRemove(request) => {
            remove(core, request)?;
            (Reply::GameRemove, true)
        }
        Call::LibraryAddGame(request) => {
            let (result, changed) = add_game(core, request)?;
            (Reply::LibraryAddGame(result), changed)
        }
        Call::GameSourceIds(request) => {
            let game = find_game(core, &request.game_id)?;
            (Reply::GameSourceIds(source_ids_view(core, &game)), false)
        }
        Call::GameSourceIdsSet(request) => (
            Reply::GameSourceIdsSet(source_ids_set(core, request)?),
            false,
        ),
        Call::GameSourceIdsSearch(request) => (
            Reply::GameSourceIdsSearch(start_search(core, request)?),
            false,
        ),
        Call::GameSourceIdsCheck(request) => (
            Reply::GameSourceIdsCheck(start_check(core, request)?),
            false,
        ),
        _ => {
            return Err(BridgeError::new(
                ErrorCode::Internal,
                "chamada fora do módulo",
            ));
        }
    })
}

impl AppService {
    /// Chamadas de editar, adicionar, remover e "Fontes e IDs". Emite `library.changed` quando a biblioteca mudou.
    pub(crate) fn handle_edit(&mut self, call: Call) -> Result<Reply, BridgeError> {
        let core = self
            .core
            .as_mut()
            .ok_or_else(|| BridgeError::new(ErrorCode::Internal, "núcleo não aberto"))?;
        let result = run(core, call);
        // Coleções que citam um jogo removido deixam de valer; e o estado do usuário volta aos jogos novos.
        core.collections.apply_to(core.library.games_mut());
        let (reply, changed) = result?;
        if changed {
            self.emit(Event::LibraryChanged);
        }
        Ok(reply)
    }

    /// Parte do `AppService::poll`: buscas e conferências de ids que terminaram.
    pub(crate) fn poll_edit(&mut self) {
        while let Some(done) = self
            .core
            .as_ref()
            .and_then(|core| core.jobs.receiver.try_recv().ok())
        {
            self.emit(match done {
                Done::Found(found) => Event::GameSourceIdsFound(found),
                Done::Checked(checked) => Event::GameSourceIdsChecked(checked),
            });
        }
    }
}

/// As fontes de metadados ligadas pelas chaves e opções que as Configurações gravam no cofre (as mesmas da fila de
/// metadados da interface Slint). Nenhuma consulta de rede acontece aqui; só quando alguém busca ou confere.
pub(crate) fn real_agent(paths: &DataPaths, vault: &dyn SecretStore) -> Arc<MetadataAgent> {
    Arc::new(
        tako_core::metadata::runtime::sources(
            paths,
            IgdbSettings::load(vault),
            ScreenScraperSettings::load(vault),
            SteamGridDbSettings::load(vault),
            Vec::new(),
        )
        .agent(),
    )
}

/// Ids fixados de todos os jogos, lidos do banco uma vez (a fila de metadados os usa).
pub(crate) fn load_pinned(
    choices: Option<&tako_core::persistence::GameChoicesStore>,
) -> HashMap<String, PinnedIds> {
    let Some(choices) = choices else {
        return HashMap::new();
    };
    match choices.load_all_ids() {
        Ok(ids) => ids
            .into_iter()
            .map(|(game, ids)| (game, ids.into_iter().collect::<PinnedIds>()))
            .collect(),
        Err(e) => {
            tracing::warn!("ids fixados: {e}");
            HashMap::new()
        }
    }
}
