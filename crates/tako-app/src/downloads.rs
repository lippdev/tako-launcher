//! Downloads por torrent pela ponte (DL-001, ADR-0028): liga a regra do núcleo (`tako_core::downloads`) às
//! chamadas `downloads.*`, com o mesmo comportamento do app Slint.
//!
//! - **Desligado por padrão.** Valem as configurações do app atual: `DownloadsEnabled` e `DownloadsLegalAccepted`
//!   (as duas verdadeiras). Desligado, toda chamada dá `conflict` e o motor nunca é criado; se o usuário desliga
//!   com downloads em andamento, o motor é solto (o que baixava volta pausado na próxima abertura).
//! - **Motor.** Real (`tako-downloads`, librqbit) fora da demonstração, falso (`MemoryDownloader`) na demonstração.
//!   Criado na abertura do núcleo se já estiver ligado, ou na primeira chamada depois de ligar; o estado salvo
//!   (banco) é restaurado nessa hora. O motor roda numa thread própria: nada aqui espera por rede ou disco do torrent.
//! - **Progresso.** O motor só é lido por `Downloads::poll`, que o `AppService::poll` chama (antes de cada chamada
//!   e a cada `HostRequest::PollBackground`). Para o progresso andar sem interface chamando, enquanto algo baixa uma
//!   thread pequena pede `PollBackground` ao hospedeiro uma vez por segundo; parado, não pede nada.
//! - **Fontes.** Só as que o usuário adicionou (banco + cache em disco). São lidas por uma thread, na primeira
//!   consulta, e a chamada responde `loading: true` até `downloads.changed` avisar que terminou. Nenhum endereço
//!   de item vem da interface: ela escolhe um item da fonte e o núcleo usa o endereço que leu.

use crate::protocol::{
    BridgeError, Call, DownloadAdd, DownloadCancel, DownloadEntry, DownloadSource,
    DownloadSourceItem, DownloadSources, DownloadSourcesRequest, DownloadStatus, DownloadsSummary,
    ErrorCode, Event, MAX_PAGE_LIMIT, Reply,
};
use crate::{AppService, Core, HostRequest};
use chrono::SecondsFormat;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use tako_core::downloads::fake::MemoryDownloader;
use tako_core::downloads::{
    Download, DownloadError, DownloadState, Downloader, Downloads, NewDownload, source,
    source::Source, suggest,
};
use tako_core::library::title_matcher;
use tako_core::persistence::{DownloadStore, SourceRow, parse_bool};

/// Mesmas chaves do app Slint (`tako-ui/src/shell/downloads.rs`).
const ENABLED: &str = "DownloadsEnabled";
const ACCEPTED: &str = "DownloadsLegalAccepted";
/// Limite de envio em KiB/s; 0 = sem limite.
const UPLOAD: &str = "DownloadsUploadLimitKiB";
const UPLOAD_DEFAULT: i64 = 256;
/// Teto da lista (concluídos ficam até o usuário tirar): mantém `downloads.list` pequena.
const MAX_DOWNLOADS: usize = 200;
const MAX_ID_CHARS: usize = 128;
const MAX_TEXT_CHARS: usize = 256;
/// Ritmo de leitura do motor pedido ao hospedeiro enquanto algo baixa.
const TICK: Duration = Duration::from_secs(1);

type Wake = Arc<Mutex<Option<mpsc::Sender<HostRequest>>>>;
type Loaded = Vec<(SourceRow, Source)>;
/// Quem fornece o motor no lugar do real (testes e hospedeiros de teste); `root` = pasta dos downloads.
pub(crate) type EngineFactory = Box<dyn Fn() -> Box<dyn Downloader> + Send>;

fn notify(wake: &Wake) {
    if let Some(host) = &*wake.lock().unwrap_or_else(|e| e.into_inner())
        && host.send(HostRequest::PollBackground).is_err()
    {
        tracing::debug!("downloads sem hospedeiro para despertar");
    }
}

/// Pede `PollBackground` a cada `TICK` enquanto `active`. Soltar o `Ticker` encerra a thread, sem esperá-la.
struct Ticker {
    active: Arc<AtomicBool>,
    _stop: mpsc::Sender<()>,
}

impl Ticker {
    fn spawn(wake: Wake) -> Option<Self> {
        let active = Arc::new(AtomicBool::new(false));
        let (stop, stopped) = mpsc::channel::<()>();
        let flag = Arc::clone(&active);
        let spawned = std::thread::Builder::new()
            .name("downloads-tick".into())
            .spawn(move || {
                while let Err(mpsc::RecvTimeoutError::Timeout) = stopped.recv_timeout(TICK) {
                    if flag.load(Ordering::Relaxed) {
                        notify(&wake);
                    }
                }
            });
        match spawned {
            Ok(_) => Some(Self {
                active,
                _stop: stop,
            }),
            Err(e) => {
                tracing::warn!("ritmo dos downloads: {e}");
                None
            }
        }
    }
}

/// Estado dos downloads dentro do `Core`.
pub(crate) struct DownloadsHub {
    store: Option<DownloadStore>,
    sources_dir: PathBuf,
    simulate: bool,
    injected: Option<(EngineFactory, PathBuf)>,
    manager: Option<Downloads>,
    /// Falha ao criar o motor: só tenta de novo na próxima abertura.
    engine_failed: bool,
    sources: Loaded,
    sources_loaded: bool,
    loading: Option<mpsc::Receiver<Loaded>>,
    wake: Wake,
    ticker: Option<Ticker>,
}

fn enabled(settings: &HashMap<String, String>) -> bool {
    let on = |key: &str| {
        settings
            .get(key)
            .and_then(|v| parse_bool(v))
            .unwrap_or(false)
    };
    on(ENABLED) && on(ACCEPTED)
}

/// `<Downloads do SO>/Tako`, como no app Slint.
fn default_root() -> PathBuf {
    tako_platform::downloads::downloads_dir().join("Tako")
}

impl DownloadsHub {
    pub(crate) fn new(
        store: Option<DownloadStore>,
        sources_dir: PathBuf,
        simulate: bool,
        injected: Option<(EngineFactory, PathBuf)>,
    ) -> Self {
        Self {
            store,
            sources_dir,
            simulate,
            injected,
            manager: None,
            engine_failed: false,
            sources: Vec::new(),
            sources_loaded: false,
            loading: None,
            wake: Arc::default(),
            ticker: None,
        }
    }

    pub(crate) fn set_host_sink(&mut self, sink: mpsc::Sender<HostRequest>) {
        *self.wake.lock().unwrap_or_else(|e| e.into_inner()) = Some(sink);
    }

    /// Cria o motor e restaura o salvo, se ligado. `None` = desligado ou motor indisponível.
    pub(crate) fn ensure(&mut self, settings: &HashMap<String, String>) -> Option<&mut Downloads> {
        if !enabled(settings) || self.engine_failed {
            return None;
        }
        if self.manager.is_none() {
            let (engine, root) = self.engine(settings)?;
            let saved = self
                .store
                .as_ref()
                .and_then(|s| s.downloads().ok())
                .unwrap_or_default();
            self.manager = Some(Downloads::new(engine, root, saved));
            if self.ticker.is_none() && (!self.simulate || self.injected.is_some()) {
                self.ticker = Ticker::spawn(Arc::clone(&self.wake));
            }
            self.refresh_ticker();
        }
        self.manager.as_mut()
    }

    fn engine(
        &mut self,
        settings: &HashMap<String, String>,
    ) -> Option<(Box<dyn Downloader>, PathBuf)> {
        if let Some((make, root)) = &self.injected {
            return Some((make(), root.clone()));
        }
        if self.simulate {
            return Some((Box::new(MemoryDownloader::default()), default_root()));
        }
        let limit: i64 = settings
            .get(UPLOAD)
            .and_then(|v| v.parse().ok())
            .unwrap_or(UPLOAD_DEFAULT);
        match tako_downloads::TorrentEngine::new(tako_downloads::EngineOptions {
            upload_limit: (limit > 0).then(|| (limit * 1024).min(u32::MAX as i64) as u32),
        }) {
            Ok(engine) => Some((Box::new(engine), default_root())),
            Err(e) => {
                tracing::error!("motor de downloads: {e}");
                self.engine_failed = true;
                None
            }
        }
    }

    fn save(&self) {
        if let (Some(store), Some(manager)) = (&self.store, &self.manager)
            && let Err(e) = store.save_downloads(manager.list())
        {
            tracing::warn!("downloads: {e}");
        }
    }

    fn refresh_ticker(&self) {
        if let (Some(ticker), Some(manager)) = (&self.ticker, &self.manager) {
            ticker
                .active
                .store(manager.summary().downloading > 0, Ordering::Relaxed);
        }
    }

    /// Lê as fontes numa thread (o cache pode ter alguns MB por fonte) na primeira vez que alguém precisa.
    fn start_loading(&mut self) {
        if self.sources_loaded || self.loading.is_some() {
            return;
        }
        let Some(store) = self.store.clone() else {
            self.sources_loaded = true;
            return;
        };
        let dir = self.sources_dir.clone();
        let wake = Arc::clone(&self.wake);
        let (sender, receiver) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("download-sources".into())
            .spawn(move || {
                let rows = store.sources().unwrap_or_else(|e| {
                    tracing::warn!("fontes de download: {e}");
                    Vec::new()
                });
                let loaded: Loaded = rows
                    .into_iter()
                    .map(|row| {
                        let source = source::read_cache(&dir, &row.url).unwrap_or(Source {
                            name: row.name.clone(),
                            items: Vec::new(),
                            dropped: 0,
                        });
                        (row, source)
                    })
                    .collect();
                if sender.send(loaded).is_ok() {
                    notify(&wake);
                }
            });
        match spawned {
            Ok(_) => self.loading = Some(receiver),
            Err(e) => {
                tracing::warn!("leitura das fontes de download: {e}");
                self.sources_loaded = true;
            }
        }
    }

    /// Recolhe o que o motor e o leitor de fontes entregaram, sem esperar. `true` = a interface deve consultar de novo.
    pub(crate) fn poll(&mut self, settings: &HashMap<String, String>) -> bool {
        let mut changed = false;
        if let Some(receiver) = &self.loading {
            match receiver.try_recv() {
                Ok(loaded) => {
                    self.sources = loaded;
                    self.sources_loaded = true;
                    self.loading = None;
                    changed = true;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    tracing::error!("leitor das fontes de download encerrou sem resultado");
                    self.sources_loaded = true;
                    self.loading = None;
                    changed = true;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if !enabled(settings) {
            if self.manager.is_some() {
                // Desligar para o motor: solta-o depois de guardar o estado (o que baixava volta pausado).
                self.save();
                self.manager = None;
                self.ticker = None;
                changed = true;
            }
            return changed;
        }
        let Some(manager) = self.ensure(settings) else {
            return changed;
        };
        let states = |m: &Downloads| m.list().iter().map(|d| d.state.clone()).collect::<Vec<_>>();
        let before = states(manager);
        let moved = manager.poll();
        let state_changed = before != states(manager);
        if state_changed {
            self.save();
        }
        self.refresh_ticker();
        changed || moved || state_changed
    }
}

fn disabled() -> BridgeError {
    BridgeError::new(ErrorCode::Conflict, "downloads desligados")
}

fn bad(message: &str) -> BridgeError {
    crate::organize::bad(message)
}

fn download_error(error: DownloadError) -> BridgeError {
    let code = match &error {
        DownloadError::NotFound => ErrorCode::NotFound,
        DownloadError::InvalidUri => ErrorCode::BadRequest,
        DownloadError::Engine(_) => ErrorCode::Internal,
        DownloadError::UnsafePaths
        | DownloadError::NoSpace { .. }
        | DownloadError::GameBusy
        | DownloadError::Duplicate => ErrorCode::Conflict,
    };
    BridgeError::new(code, error.to_string())
}

fn valid_id(value: &str) -> Result<(), BridgeError> {
    if value.trim().is_empty() || value.chars().count() > MAX_ID_CHARS || value.contains('\0') {
        Err(bad("id inválido"))
    } else {
        Ok(())
    }
}

fn entry(d: &Download) -> DownloadEntry {
    let (state, error) = match &d.state {
        DownloadState::Queued => (DownloadStatus::Queued, None),
        DownloadState::Downloading => (DownloadStatus::Downloading, None),
        DownloadState::Paused => (DownloadStatus::Paused, None),
        DownloadState::Completed => (DownloadStatus::Completed, None),
        DownloadState::Failed(error) => (DownloadStatus::Failed, Some(error.clone())),
    };
    let eta_seconds = match d.total {
        Some(total) if state == DownloadStatus::Downloading && d.speed > 0 && total > d.done => {
            Some((total - d.done).div_ceil(d.speed))
        }
        _ => None,
    };
    DownloadEntry {
        id: d.id,
        title: d.title.clone(),
        game_id: d.game_id.clone(),
        source: d.source.clone(),
        state,
        error,
        done: d.done,
        total: d.total,
        progress: d.fraction(),
        speed: d.speed,
        eta_seconds,
    }
}

/// Id estável da fonte para a interface: o nome do cache em disco (hash da URL). O endereço não sai do núcleo.
fn source_id(url: &str) -> String {
    source::cache_file(Path::new(""), url)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_owned()
}

/// Só o domínio: o caminho do endereço pode levar um segredo.
fn host(url: &str) -> String {
    url.split("://")
        .nth(1)
        .and_then(|rest| rest.split(['/', '?', '#']).next())
        .unwrap_or_default()
        .to_owned()
}

fn source_name<'a>(row: &'a SourceRow, source: &'a Source) -> &'a str {
    if source.name.is_empty() {
        &row.name
    } else {
        &source.name
    }
}

impl AppService {
    /// Chamadas `downloads.*`. Emite `downloads.changed` quando a chamada mudou algo.
    pub(crate) fn handle_downloads(&mut self, call: Call) -> Result<Reply, BridgeError> {
        let core = self
            .core
            .as_mut()
            .ok_or_else(|| BridgeError::new(ErrorCode::Internal, "núcleo não aberto"))?;
        let (reply, changed) = run(core, call)?;
        if changed {
            self.emit(Event::DownloadsChanged);
        }
        Ok(reply)
    }

    /// Parte do `AppService::poll`: progresso do motor e fontes lidas.
    pub(crate) fn poll_downloads(&mut self) {
        let Some(core) = self.core.as_mut() else {
            return;
        };
        if core.downloads.poll(&core.settings) {
            self.emit(Event::DownloadsChanged);
        }
    }
}

fn run(core: &mut Core, call: Call) -> Result<(Reply, bool), BridgeError> {
    if !enabled(&core.settings) {
        return Err(disabled());
    }
    match call {
        Call::DownloadsSources(request) => sources(core, request).map(|r| (r, false)),
        Call::DownloadsAdd(request) => add(core, request),
        call => {
            let manager = manager(core)?;
            let reply = match call {
                Call::DownloadsList => {
                    return Ok((
                        Reply::DownloadsList(manager.list().iter().map(entry).collect()),
                        false,
                    ));
                }
                Call::DownloadsSummary => {
                    let s = manager.summary();
                    return Ok((
                        Reply::DownloadsSummary(DownloadsSummary {
                            downloading: s.downloading,
                            queued: s.queued,
                            speed: s.speed,
                            done: s.done,
                            total: s.total,
                        }),
                        false,
                    ));
                }
                Call::DownloadsPause(request) => {
                    manager.pause(request.download_id).map_err(download_error)?;
                    Reply::DownloadsPause
                }
                Call::DownloadsResume(request) => {
                    manager
                        .resume(request.download_id)
                        .map_err(download_error)?;
                    Reply::DownloadsResume
                }
                Call::DownloadsCancel(DownloadCancel {
                    download_id,
                    delete_files,
                }) => {
                    manager
                        .cancel(download_id, delete_files)
                        .map_err(download_error)?;
                    Reply::DownloadsCancel
                }
                _ => return Err(bad("chamada fora dos downloads")),
            };
            core.downloads.save();
            core.downloads.refresh_ticker();
            Ok((reply, true))
        }
    }
}

fn manager(core: &mut Core) -> Result<&mut Downloads, BridgeError> {
    core.downloads
        .ensure(&core.settings)
        .ok_or_else(|| BridgeError::new(ErrorCode::Internal, "motor de downloads indisponível"))
}

fn sources(core: &mut Core, request: DownloadSourcesRequest) -> Result<Reply, BridgeError> {
    if request.search.chars().count() > MAX_TEXT_CHARS {
        return Err(bad("busca grande demais"));
    }
    for id in [&request.source_id, &request.game_id].into_iter().flatten() {
        valid_id(id)?;
    }
    if request.game_id.is_some() && !request.search.trim().is_empty() {
        return Err(bad("sugestões e busca não combinam"));
    }
    core.downloads.start_loading();
    let hub = &core.downloads;
    let infos: Vec<DownloadSource> = hub
        .sources
        .iter()
        .map(|(row, source)| DownloadSource {
            id: source_id(&row.url),
            name: source_name(row, source).to_owned(),
            host: host(&row.url),
            item_count: source.items.len(),
            dropped: source.dropped,
            updated_at: row.updated_at.to_rfc3339_opts(SecondsFormat::Millis, true),
        })
        .collect();
    if hub.sources_loaded
        && request
            .source_id
            .as_ref()
            .is_some_and(|id| !infos.iter().any(|s| &s.id == id))
    {
        return Err(BridgeError::new(ErrorCode::NotFound, "fonte"));
    }
    let wanted: Vec<(&String, &Source)> = infos
        .iter()
        .zip(&hub.sources)
        .filter(|(info, _)| request.source_id.as_ref().is_none_or(|id| &info.id == id))
        .map(|(info, (_, source))| (&info.id, source))
        .collect();
    let limit = request.limit.min(MAX_PAGE_LIMIT) as usize;
    let offset = request.offset as usize;
    let mut total = 0;
    let mut items = Vec::new();
    let item = |source_id: &str, index: usize, i: &source::SourceItem| DownloadSourceItem {
        source_id: source_id.to_owned(),
        index,
        title: i.title.clone(),
        file_size: i.file_size.clone(),
        upload_date: i.upload_date.clone(),
    };
    if let Some(game_id) = &request.game_id {
        let game = core
            .library
            .find(game_id)
            .ok_or_else(|| BridgeError::new(ErrorCode::NotFound, format!("jogo {game_id}")))?;
        // Só sugestão (a regra é do núcleo): o usuário escolhe o item.
        let found = suggest(
            &game.title,
            wanted.iter().map(|(id, source)| (id.as_str(), *source)),
            usize::MAX,
        );
        total = found.len();
        for (source_id, found) in found.into_iter().skip(offset).take(limit) {
            if let Some((_, source)) = wanted.iter().find(|(id, _)| id.as_str() == source_id)
                && let Some(index) = source.items.iter().position(|i| std::ptr::eq(i, found))
            {
                items.push(item(source_id, index, found));
            }
        }
    } else {
        // Cada palavra da busca é prefixo de alguma palavra do título (já sem acento nem caixa). Busca só com
        // palavras de enchimento ("the", "edition") não combina com nada.
        let search = request.search.trim();
        let terms = (!search.is_empty()).then(|| title_matcher::tokens(search));
        for (id, source) in &wanted {
            for (index, i) in source.items.iter().enumerate() {
                let matches = terms.as_ref().is_none_or(|terms| {
                    !terms.is_empty()
                        && terms
                            .iter()
                            .all(|term| i.tokens.iter().any(|t| t.starts_with(term.as_str())))
                });
                if !matches {
                    continue;
                }
                if total >= offset && items.len() < limit {
                    items.push(item(id, index, i));
                }
                total += 1;
            }
        }
    }
    Ok(Reply::DownloadsSources(DownloadSources {
        loading: !hub.sources_loaded,
        sources: infos,
        items,
        total,
    }))
}

fn add(core: &mut Core, request: DownloadAdd) -> Result<(Reply, bool), BridgeError> {
    valid_id(&request.source_id)?;
    if request.title.chars().count() > MAX_TEXT_CHARS {
        return Err(bad("título grande demais"));
    }
    if let Some(game_id) = &request.game_id {
        valid_id(game_id)?;
        if core.library.find(game_id).is_none() {
            return Err(BridgeError::new(
                ErrorCode::NotFound,
                format!("jogo {game_id}"),
            ));
        }
    }
    core.downloads.start_loading();
    if !core.downloads.sources_loaded {
        return Err(BridgeError::new(
            ErrorCode::Conflict,
            "fontes ainda sendo lidas",
        ));
    }
    let (row, source) = core
        .downloads
        .sources
        .iter()
        .find(|(row, _)| source_id(&row.url) == request.source_id)
        .ok_or_else(|| BridgeError::new(ErrorCode::NotFound, "fonte"))?;
    let chosen = source
        .items
        .get(request.index)
        .ok_or_else(|| BridgeError::new(ErrorCode::NotFound, "item da fonte"))?;
    if chosen.title != request.title {
        return Err(BridgeError::new(
            ErrorCode::Conflict,
            "a fonte mudou desde a listagem",
        ));
    }
    let new = NewDownload {
        title: chosen.title.clone(),
        game_id: request.game_id,
        source: source_name(row, source).to_owned(),
        // `uris` nunca vem vazio: o leitor da fonte descarta o item sem endereço válido.
        uri: chosen.uris.first().cloned().unwrap_or_default(),
    };
    let manager = manager(core)?;
    if manager.list().len() >= MAX_DOWNLOADS {
        return Err(BridgeError::new(
            ErrorCode::Conflict,
            "limite de downloads na lista",
        ));
    }
    let id = manager.add(new).map_err(download_error)?;
    let created = manager.get(id).map(entry);
    core.downloads.save();
    core.downloads.refresh_ticker();
    let created =
        created.ok_or_else(|| BridgeError::new(ErrorCode::Internal, "download perdido"))?;
    Ok((Reply::DownloadsAdd(created), true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tako_core::paths::DataPaths;
    use tako_platform::Platform;

    fn on() -> HashMap<String, String> {
        [(ENABLED, "true"), (ACCEPTED, "true")]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect()
    }

    #[test]
    fn both_settings_are_needed_and_default_is_off() {
        assert!(!enabled(&HashMap::new()));
        let mut only_enabled = on();
        only_enabled.remove(ACCEPTED);
        assert!(!enabled(&only_enabled));
        assert!(enabled(&on()));
        assert!(enabled(
            &[
                (ENABLED.to_owned(), " TRUE ".to_owned()),
                (ACCEPTED.to_owned(), "true".to_owned())
            ]
            .into_iter()
            .collect()
        ));
    }

    #[test]
    fn demonstration_uses_the_fake_engine_and_turning_off_drops_it() {
        let dir = tempfile::tempdir().unwrap();
        let mut core = Core::open(DataPaths::at(dir.path()), Platform::simulated(), true).unwrap();
        assert!(core.downloads.manager.is_none());
        core.settings.extend(on());
        let (reply, changed) = run(&mut core, Call::DownloadsList).unwrap();
        assert_eq!((reply, changed), (Reply::DownloadsList(Vec::new()), false));
        assert!(core.downloads.manager.is_some());
        // Demonstração: nenhuma thread de ritmo e nada gravado em disco.
        assert!(core.downloads.ticker.is_none());
        assert!(!dir.path().join("tako.db").exists());

        core.settings.insert(ENABLED.into(), "false".into());
        assert!(core.downloads.poll(&core.settings));
        assert!(core.downloads.manager.is_none());
        assert_eq!(
            run(&mut core, Call::DownloadsList).unwrap_err().code,
            ErrorCode::Conflict
        );
    }

    #[test]
    fn source_ids_do_not_expose_the_address() {
        let id = source_id("https://exemplo.invalid/segredo/fonte.json");
        assert_eq!(id.len(), 64);
        assert!(!id.contains("segredo"));
        assert_eq!(
            host("https://exemplo.invalid/segredo/fonte.json?t=1"),
            "exemplo.invalid"
        );
    }

    #[test]
    fn eta_only_while_downloading_with_speed() {
        let mut d = Download::stored(
            1,
            "A".into(),
            None,
            "f".into(),
            "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567".into(),
            "/d/A".into(),
            DownloadState::Paused,
            50,
            Some(150),
        );
        assert_eq!(entry(&d).eta_seconds, None);
        assert_eq!(entry(&d).progress, Some(1. / 3.));
        assert_eq!(entry(&d).state, DownloadStatus::Paused);
        d.state = DownloadState::Failed("sem espaço".into());
        let failed = entry(&d);
        assert_eq!(failed.state, DownloadStatus::Failed);
        assert_eq!(failed.error.as_deref(), Some("sem espaço"));
    }
}
