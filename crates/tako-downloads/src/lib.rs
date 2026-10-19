//! Downloads por torrent do Tako (DL-001, ADR-0028): o motor `librqbit` numa thread própria com `tokio`, atrás do
//! trait [`tako_core::downloads::Downloader`], e a leitura das fontes `.json` do usuário por `https`.
//!
//! Padrões conservadores (ADR-0028 §2 e §7): nenhuma porta aberta (sem escuta, sem UPnP), sem LSD, sem
//! persistência do motor, limite de envio, e o torrent sai do motor ao terminar (sem semear). Antes de gravar, os
//! caminhos de dentro do torrent são validados e o espaço livre é conferido. Nada do conteúdo baixado é executado.

use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, ManagedTorrent, Session, SessionOptions,
    api::TorrentIdOrHash, limits::LimitsConfig,
};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tako_core::downloads::{DownloadError, Downloader, EngineId, EngineStatus, paths};
use tokio::sync::mpsc;

type ManagedTorrentHandle = Arc<ManagedTorrent>;

/// Arquivo `.torrent` baixado por `https`.
const MAX_TORRENT_BYTES: u64 = 16 * 1024 * 1024;
const HTTP_TIMEOUT: Duration = Duration::from_secs(30);

/// Erro ao ler uma fonte ou um `.torrent` por `https`.
#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("só endereços https:// são aceitos")]
    NotHttps,
    #[error("o arquivo passa do limite")]
    TooLarge,
    #[error("rede: {0}")]
    Http(String),
}

/// GET por `https` sem cookies nem identificação do usuário, com tempo-limite, até 3 redirecionamentos (também só
/// `https`) e no máximo `limit` bytes.
pub fn fetch_https(url: &str, limit: u64) -> Result<Vec<u8>, FetchError> {
    if !url.trim().to_ascii_lowercase().starts_with("https://") {
        return Err(FetchError::NotHttps);
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .https_only(true)
        .max_redirects(3)
        .timeout_global(Some(HTTP_TIMEOUT))
        .user_agent("Tako")
        .build()
        .into();
    let mut response = agent.get(url.trim()).call().map_err(|e| match e {
        ureq::Error::RequireHttpsOnly(_) => FetchError::NotHttps,
        e => FetchError::Http(e.to_string()),
    })?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(limit + 1)
        .read_to_vec()
        .map_err(|e| FetchError::Http(e.to_string()))?;
    if bytes.len() as u64 > limit {
        return Err(FetchError::TooLarge);
    }
    Ok(bytes)
}

/// Opções do motor (Configurações › Downloads).
#[derive(Debug, Clone, Copy, Default)]
pub struct EngineOptions {
    /// Limite de envio em bytes/s; `None` = sem limite.
    pub upload_limit: Option<u32>,
}

enum Cmd {
    Start(EngineId, String, PathBuf),
    Pause(EngineId),
    Resume(EngineId),
    Remove(EngineId, bool),
}

#[derive(Default)]
struct Shared {
    status: HashMap<EngineId, EngineStatus>,
    handles: HashMap<EngineId, ManagedTorrentHandle>,
    /// Pedidos que chegaram antes de o torrent entrar no motor (metadados do magnet ainda vindo).
    paused: HashMap<EngineId, bool>,
    removed: HashMap<EngineId, bool>,
}

type State = Arc<Mutex<Shared>>;

fn lock(s: &State) -> std::sync::MutexGuard<'_, Shared> {
    s.lock().unwrap_or_else(|e| e.into_inner())
}

/// Motor `librqbit`. A sessão (e o DHT) só começa no primeiro download: ligar a função não fala com ninguém.
pub struct TorrentEngine {
    tx: mpsc::UnboundedSender<Cmd>,
    state: State,
    next: EngineId,
}

impl TorrentEngine {
    pub fn new(options: EngineOptions) -> Result<TorrentEngine, DownloadError> {
        let (tx, rx) = mpsc::unbounded_channel();
        let state = State::default();
        let shared = state.clone();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("tako-downloads")
            .enable_all()
            .build()
            .map_err(|e| DownloadError::Engine(e.to_string()))?;
        std::thread::Builder::new()
            .name("tako-downloads".into())
            .spawn(move || runtime.block_on(run(rx, shared, options)))
            .map_err(|e| DownloadError::Engine(e.to_string()))?;
        Ok(TorrentEngine { tx, state, next: 0 })
    }

    fn send(&self, cmd: Cmd) {
        // A thread do motor só termina quando este lado é solto.
        let _ = self.tx.send(cmd);
    }
}

impl Downloader for TorrentEngine {
    fn start(&mut self, uri: &str, folder: &Path) -> Result<EngineId, DownloadError> {
        let uri = tako_core::downloads::source::valid_uri(uri).ok_or(DownloadError::InvalidUri)?;
        self.next += 1;
        lock(&self.state)
            .status
            .insert(self.next, EngineStatus::default());
        self.send(Cmd::Start(self.next, uri, folder.to_owned()));
        Ok(self.next)
    }
    fn pause(&mut self, id: EngineId) {
        self.send(Cmd::Pause(id));
    }
    fn resume(&mut self, id: EngineId) {
        self.send(Cmd::Resume(id));
    }
    fn remove(&mut self, id: EngineId, delete_files: bool) {
        lock(&self.state).status.remove(&id);
        self.send(Cmd::Remove(id, delete_files));
    }
    fn status(&self, id: EngineId) -> Option<EngineStatus> {
        lock(&self.state).status.get(&id).cloned()
    }
}

async fn session(folder: &Path, options: EngineOptions) -> Result<Arc<Session>, String> {
    let opts = SessionOptions {
        // Sem escuta (nenhuma porta aberta, nem por UPnP), sem LSD e sem arquivos de estado do motor.
        listen: None,
        persistence: None,
        fastresume: false,
        disable_local_service_discovery: true,
        ratelimits: LimitsConfig {
            upload_bps: options.upload_limit.and_then(NonZeroU32::new),
            download_bps: None,
        },
        peer_limit: Some(80),
        ..Default::default()
    };
    Session::new_with_opts(folder.to_owned(), opts)
        .await
        .map_err(|e| e.to_string())
}

async fn run(mut rx: mpsc::UnboundedReceiver<Cmd>, state: State, options: EngineOptions) {
    let mut current: Option<Arc<Session>> = None;
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    loop {
        tokio::select! {
            cmd = rx.recv() => {
                let Some(cmd) = cmd else { break };
                match cmd {
                    Cmd::Start(id, uri, folder) => {
                        if current.is_none() {
                            match session(&folder, options).await {
                                Ok(s) => current = Some(s),
                                Err(e) => {
                                    fail(&state, id, DownloadError::Engine(e));
                                    continue;
                                }
                            }
                        }
                        if let Some(s) = current.clone() {
                            tokio::spawn(start(s, state.clone(), id, uri, folder));
                        }
                    }
                    Cmd::Pause(id) => {
                        let h = { let mut g = lock(&state); g.paused.insert(id, true); g.handles.get(&id).cloned() };
                        if let (Some(s), Some(h)) = (&current, h) { let _ = s.pause(&h).await; }
                    }
                    Cmd::Resume(id) => {
                        let h = { let mut g = lock(&state); g.paused.insert(id, false); g.handles.get(&id).cloned() };
                        if let (Some(s), Some(h)) = (&current, h) { let _ = s.unpause(&h).await; }
                    }
                    Cmd::Remove(id, delete) => {
                        let h = { let mut g = lock(&state); g.removed.insert(id, delete); g.handles.remove(&id) };
                        if let (Some(s), Some(h)) = (&current, h) {
                            let _ = s.delete(TorrentIdOrHash::Id(h.id()), delete).await;
                        }
                    }
                }
            }
            _ = tick.tick() => refresh(&state),
        }
    }
    if let Some(s) = current {
        s.stop().await;
    }
}

fn fail(state: &State, id: EngineId, error: DownloadError) {
    tracing::warn!(id, kind = %error, "download falhou");
    if let Some(s) = lock(state).status.get_mut(&id) {
        s.error = Some(error);
    }
}

fn refresh(state: &State) {
    let mut g = lock(state);
    let stats: Vec<_> = g.handles.iter().map(|(id, h)| (*id, h.stats())).collect();
    for (id, st) in stats {
        let Some(s) = g.status.get_mut(&id) else {
            continue;
        };
        s.done = st.progress_bytes;
        s.total = Some(st.total_bytes);
        s.down_bps = st.live.as_ref().map_or(0, |l| l.download_speed.as_bytes());
        s.finished = st.finished;
        if st.error.is_some() && s.error.is_none() {
            // O texto do motor pode ter caminhos e hashes: fica fora do log e da tela.
            s.error = Some(DownloadError::Engine("falha no torrent".into()));
        }
    }
}

/// Metadados primeiro (sem gravar nada), depois a checagem dos caminhos e do espaço, e só então o download.
async fn start(session: Arc<Session>, state: State, id: EngineId, uri: String, folder: PathBuf) {
    if let Err(e) = add(&session, &state, id, &uri, &folder).await {
        fail(&state, id, e);
    }
}

async fn add(
    session: &Arc<Session>,
    state: &State,
    id: EngineId,
    uri: &str,
    folder: &Path,
) -> Result<(), DownloadError> {
    let engine = |e: String| DownloadError::Engine(e);
    let source = if uri.to_ascii_lowercase().starts_with("magnet:") {
        AddTorrent::from_url(uri.to_owned())
    } else {
        let url = uri.to_owned();
        let bytes = tokio::task::spawn_blocking(move || fetch_https(&url, MAX_TORRENT_BYTES))
            .await
            .map_err(|e| engine(e.to_string()))?
            .map_err(|e| engine(e.to_string()))?;
        AddTorrent::from_bytes(bytes)
    };
    let listed = session
        .add_torrent(
            source,
            Some(AddTorrentOptions {
                list_only: true,
                output_folder: Some(folder.to_string_lossy().into_owned()),
                ..Default::default()
            }),
        )
        .await
        .map_err(|e| engine(e.to_string()))?;
    let AddTorrentResponse::ListOnly(listed) = listed else {
        return Err(engine("resposta inesperada do motor".into()));
    };
    let mut total = 0u64;
    for f in listed.info.iter_file_details() {
        let parts = f.filename.to_vec();
        let ok = !f.attrs().symlink
            && f.symlink_path.is_none()
            && paths::safe_relative(&parts).is_some_and(|p| paths::inside(folder, &p));
        if !ok {
            return Err(DownloadError::UnsafePaths);
        }
        total = total.saturating_add(f.len);
    }
    std::fs::create_dir_all(folder).map_err(|e| engine(e.to_string()))?;
    if let Some(free) = tako_platform::space::free_space(folder)
        && free.bytes < total
    {
        return Err(DownloadError::NoSpace {
            missing: total - free.bytes,
        });
    }
    if lock(state).removed.contains_key(&id) {
        return Ok(());
    }
    let paused = lock(state).paused.get(&id).copied().unwrap_or(false);
    let added = session
        .add_torrent(
            AddTorrent::from_bytes(listed.torrent_bytes),
            Some(AddTorrentOptions {
                paused,
                // Retomar depois de reabrir o Tako precisa escrever sobre o que já baixou (na pasta dele).
                overwrite: true,
                output_folder: Some(folder.to_string_lossy().into_owned()),
                ..Default::default()
            }),
        )
        .await
        .map_err(|e| engine(e.to_string()))?;
    let Some(handle) = added.into_handle() else {
        return Err(engine("o motor não aceitou o torrent".into()));
    };
    let removed = {
        let mut g = lock(state);
        if let Some(s) = g.status.get_mut(&id) {
            s.total = Some(total);
        }
        let removed = g.removed.get(&id).copied();
        if removed.is_none() {
            g.handles.insert(id, handle.clone());
        }
        removed
    };
    // Cancelado enquanto os metadados chegavam.
    if let Some(delete) = removed {
        let _ = session
            .delete(TorrentIdOrHash::Id(handle.id()), delete)
            .await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_http_is_refused_before_touching_the_network() {
        assert!(matches!(
            fetch_https("http://exemplo.invalid/fonte.json", 10),
            Err(FetchError::NotHttps)
        ));
        assert!(matches!(
            fetch_https("file:///etc/passwd", 10),
            Err(FetchError::NotHttps)
        ));
    }

    #[test]
    fn invalid_uris_never_reach_the_engine() {
        let mut e = TorrentEngine::new(EngineOptions::default()).unwrap();
        assert_eq!(
            e.start("javascript:alert(1)", Path::new("/tmp/x")),
            Err(DownloadError::InvalidUri)
        );
    }
}
