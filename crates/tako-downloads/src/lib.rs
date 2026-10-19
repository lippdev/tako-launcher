//! Downloads por torrent do Tako (DL-001, ADR-0028): o motor `librqbit` numa thread própria com `tokio`, atrás do
//! trait [`tako_core::downloads::Downloader`], e a leitura das fontes `.json` do usuário por `https`.
//!
//! Padrões conservadores (ADR-0028 §2 e §7): sem porta TCP de entrada para pares e sem UPnP (o DHT, necessário
//! para magnet, usa uma porta UDP), sem LSD, sem arquivos de estado do motor nem do DHT, limite de envio, e o
//! torrent sai do motor ao terminar (sem semear). Antes de gravar, os caminhos de dentro do torrent são validados
//! (inclusive contra links simbólicos) e o espaço livre é conferido. Nada do conteúdo baixado é executado.

use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, DhtSessionConfig, ManagedTorrent, Session,
    SessionOptions, api::TorrentIdOrHash, limits::LimitsConfig,
};
use std::collections::HashMap;
use std::io::{Read, Write};
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
const OWNER_FILE: &str = ".tako-download-owner";

/// Erro ao ler uma fonte ou um `.torrent` por `https`.
#[derive(Debug)]
pub enum FetchError {
    NotHttps,
    TooLarge,
    Http(String),
}

impl std::error::Error for FetchError {}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::NotHttps => {
                write!(f, "{}", tako_core::i18n::t("downloads.error.not_https"))
            }
            FetchError::TooLarge => write!(
                f,
                "{}",
                tako_core::i18n::t("downloads.error.source_too_large")
            ),
            FetchError::Http(err) => write!(
                f,
                "{}",
                tako_core::i18n::tf("downloads.error.http", &[("error", err)])
            ),
        }
    }
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
    Start(EngineId, String, PathBuf, bool),
    Pause(EngineId),
    Resume(EngineId),
    Remove(EngineId, bool),
    Finish(EngineId),
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
    fn start(&mut self, uri: &str, folder: &Path, resume: bool) -> Result<EngineId, DownloadError> {
        let uri = tako_core::downloads::source::valid_uri(uri).ok_or(DownloadError::InvalidUri)?;
        self.next += 1;
        lock(&self.state)
            .status
            .insert(self.next, EngineStatus::default());
        self.send(Cmd::Start(self.next, uri, folder.to_owned(), resume));
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
    fn finish(&mut self, id: EngineId) {
        lock(&self.state).status.remove(&id);
        self.send(Cmd::Finish(id));
    }
    fn status(&self, id: EngineId) -> Option<EngineStatus> {
        lock(&self.state).status.get(&id).cloned()
    }
}

async fn session(folder: &Path, options: EngineOptions) -> Result<Arc<Session>, String> {
    let opts = SessionOptions {
        // Sem escuta TCP para pares e sem UPnP; o DHT escuta UDP (magnet precisa dele), sem gravar `dht.json`.
        // Sem LSD e sem arquivos de estado do motor.
        listen: None,
        dht: Some(DhtSessionConfig {
            persistence: None,
            ..Default::default()
        }),
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
                    Cmd::Start(id, uri, folder, resume) => {
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
                            tokio::spawn(start(s, state.clone(), id, uri, folder, resume));
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
                        if let (Some(s), Some(h)) = (&current, h)
                            && s.delete(TorrentIdOrHash::Id(h.id()), delete).await.is_ok() && delete
                        {
                            clear_owner(h.output_folder());
                        }
                    }
                    Cmd::Finish(id) => {
                        let h = { let mut g = lock(&state); g.handles.remove(&id) };
                        if let (Some(s), Some(h)) = (&current, h)
                            && s.delete(TorrentIdOrHash::Id(h.id()), false).await.is_ok()
                        {
                            clear_owner(h.output_folder());
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
async fn start(
    session: Arc<Session>,
    state: State,
    id: EngineId,
    uri: String,
    folder: PathBuf,
    resume: bool,
) {
    if let Err(e) = add(&session, &state, id, &uri, &folder, resume).await {
        fail(&state, id, e);
    }
}

async fn add(
    session: &Arc<Session>,
    state: &State,
    id: EngineId,
    uri: &str,
    folder: &Path,
    resume: bool,
) -> Result<(), DownloadError> {
    // Mensagens fixas: o texto dos erros de rede e do motor pode ter a URL, o hash ou caminhos (fora do log).
    let engine = |e: &str| DownloadError::Engine(e.to_owned());
    let source = if uri.to_ascii_lowercase().starts_with("magnet:") {
        AddTorrent::from_url(uri.to_owned())
    } else {
        let url = uri.to_owned();
        let bytes = tokio::task::spawn_blocking(move || fetch_https(&url, MAX_TORRENT_BYTES))
            .await
            .map_err(|_| engine(&tako_core::i18n::t("downloads.error.fetch_torrent_failed")))?
            .map_err(|_| engine(&tako_core::i18n::t("downloads.error.fetch_torrent_failed")))?;
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
        .map_err(|_| engine("não deu para ler o torrent"))?;
    let AddTorrentResponse::ListOnly(listed) = listed else {
        return Err(engine("resposta inesperada do motor"));
    };
    let mut total = 0u64;
    let mut files = Vec::new();
    for f in listed.info.iter_file_details() {
        let parts = f.filename.to_vec();
        let rel = paths::safe_relative(&parts).filter(|p| paths::inside(folder, p));
        match rel {
            Some(rel) if !f.attrs().symlink && f.symlink_path.is_none() => files.push(rel),
            _ => return Err(DownloadError::UnsafePaths),
        }
        total = total.saturating_add(f.len);
    }
    check_folder(folder, &files, resume, uri)?;
    prepare_folder(folder, uri)?;
    // De novo depois de criar: nada trocou a pasta por um link no meio do caminho.
    check_folder(folder, &files, true, uri)?;
    if let Some(free) = tako_platform::space::free_space(folder)
        && free.bytes < total
    {
        return Err(DownloadError::NoSpace {
            missing: total - free.bytes,
        });
    }
    let removed = lock(state).removed.get(&id).copied();
    if let Some(delete) = removed {
        if delete {
            clear_owner(folder);
        }
        return Ok(());
    }
    let paused = lock(state).paused.get(&id).copied().unwrap_or(false);
    let added = session
        .add_torrent(
            AddTorrent::from_bytes(listed.torrent_bytes),
            Some(AddTorrentOptions {
                paused,
                // Só ao retomar um download do Tako: escreve sobre o que ele mesmo já baixou na pasta dele.
                overwrite: resume,
                output_folder: Some(folder.to_string_lossy().into_owned()),
                ..Default::default()
            }),
        )
        .await
        .map_err(|_| engine("o motor não aceitou o torrent"))?;
    let Some(handle) = added.into_handle() else {
        return Err(engine("o motor não aceitou o torrent"));
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
    if let Some(delete) = removed
        && session
            .delete(TorrentIdOrHash::Id(handle.id()), delete)
            .await
            .is_ok()
        && delete
    {
        clear_owner(folder);
    }
    Ok(())
}

/// A pasta do download e tudo abaixo dela que já existe: nenhum link simbólico (nem a própria pasta, nem
/// `<Downloads>/Tako`). Download novo nunca grava numa pasta que já existe.
fn check_folder(
    folder: &Path,
    files: &[PathBuf],
    resume: bool,
    uri: &str,
) -> Result<(), DownloadError> {
    let (Some(root), Some(name)) = (folder.parent(), folder.file_name()) else {
        return Err(DownloadError::UnsafePaths);
    };
    if paths::has_link(root, Path::new(name))
        || paths::has_link(folder, Path::new(OWNER_FILE))
        || files.iter().any(|f| {
            paths::has_link(folder, f)
                || f.components()
                    .next()
                    .is_some_and(|c| c.as_os_str().eq_ignore_ascii_case(OWNER_FILE))
        })
    {
        return Err(DownloadError::UnsafePaths);
    }
    if !resume && std::fs::symlink_metadata(folder).is_ok() {
        return Err(DownloadError::Engine(tako_core::i18n::t(
            "downloads.error.folder_exists",
        )));
    }
    if resume && folder.exists() {
        let mut owner = Vec::new();
        let read = std::fs::File::open(folder.join(OWNER_FILE))
            .and_then(|f| f.take(65).read_to_end(&mut owner));
        if read.is_err() || owner != owner_key(uri).as_bytes() {
            return Err(DownloadError::UnsafePaths);
        }
    }
    Ok(())
}

fn owner_key(uri: &str) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(uri.as_bytes()))
}

/// A criação exclusiva impede que uma pasta surgida depois da validação seja adotada como nossa.
fn prepare_folder(folder: &Path, uri: &str) -> Result<(), DownloadError> {
    let create = || -> std::io::Result<()> {
        if let Some(parent) = folder.parent() {
            std::fs::create_dir_all(parent)?;
        }
        match std::fs::create_dir(folder) {
            Ok(()) => std::fs::File::create_new(folder.join(OWNER_FILE))?
                .write_all(owner_key(uri).as_bytes()),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                check_folder(folder, &[], true, uri).map_err(std::io::Error::other)
            }
            Err(e) => Err(e),
        }
    };
    create().map_err(|_| {
        DownloadError::Engine(tako_core::i18n::t("downloads.error.reserve_folder_failed"))
    })
}

fn clear_owner(folder: &Path) {
    if paths::has_link(folder, Path::new(OWNER_FILE)) {
        return;
    }
    if std::fs::remove_file(folder.join(OWNER_FILE)).is_err() {
        tracing::warn!("não deu para remover a marca da pasta do download");
        return;
    }
    // Sem remoção recursiva: arquivos que o usuário acrescentou ficam onde estão.
    if let Err(e) = std::fs::remove_dir(folder)
        && e.kind() != std::io::ErrorKind::DirectoryNotEmpty
    {
        tracing::warn!("não deu para remover a pasta vazia do download");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const URI: &str = "https://exemplo.invalid/jogo.torrent";

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
    fn new_downloads_never_write_into_an_existing_folder_or_through_a_link() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().canonicalize().unwrap().join("Tako").join("Jogo");
        let files = [PathBuf::from("data/a.bin")];
        assert!(check_folder(&folder, &files, false, URI).is_ok());
        std::fs::create_dir_all(folder.join("data")).unwrap();
        assert!(
            check_folder(&folder, &files, false, URI).is_err(),
            "pasta já existe"
        );
        std::fs::write(folder.join(OWNER_FILE), owner_key(URI)).unwrap();
        assert!(check_folder(&folder, &files, true, URI).is_ok(), "retomada");
        assert!(check_folder(&folder, &files, true, "outro torrent").is_err());
        assert!(check_folder(&folder, &[OWNER_FILE.into()], true, URI).is_err());
        #[cfg(unix)]
        {
            let outside = dir.path().join("fora");
            std::fs::create_dir_all(&outside).unwrap();
            std::os::unix::fs::symlink(&outside, folder.join("data/a.bin")).unwrap();
            assert_eq!(
                check_folder(&folder, &files, true, URI),
                Err(DownloadError::UnsafePaths)
            );
            let linked = dir.path().join("Tako").join("Link");
            std::os::unix::fs::symlink(&outside, &linked).unwrap();
            assert_eq!(
                check_folder(&linked, &[], true, URI),
                Err(DownloadError::UnsafePaths)
            );
        }
    }

    #[test]
    fn a_retry_cannot_claim_an_existing_user_folder() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().canonicalize().unwrap().join("Tako/Jogo");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("a.bin"), b"arquivo do usuario").unwrap();
        assert!(check_folder(&folder, &[PathBuf::from("a.bin")], true, URI).is_err());
        assert!(prepare_folder(&folder, URI).is_err());
        assert_eq!(
            std::fs::read(folder.join("a.bin")).unwrap(),
            b"arquivo do usuario"
        );
        let own = folder.with_file_name("Proprio");
        prepare_folder(&own, URI).unwrap();
        assert!(check_folder(&own, &["a.bin".into()], true, URI).is_ok());
        assert!(prepare_folder(&own, URI).is_ok());
        std::fs::write(own.join("do-usuario.txt"), b"preservar").unwrap();
        clear_owner(&own);
        assert_eq!(
            std::fs::read(own.join("do-usuario.txt")).unwrap(),
            b"preservar"
        );
        assert!(!own.join(OWNER_FILE).exists());
        let empty = folder.with_file_name("Vazio");
        prepare_folder(&empty, URI).unwrap();
        clear_owner(&empty);
        assert!(!empty.exists());
    }

    #[test]
    fn invalid_uris_never_reach_the_engine() {
        let mut e = TorrentEngine::new(EngineOptions::default()).unwrap();
        assert_eq!(
            e.start("javascript:alert(1)", Path::new("/tmp/x"), false),
            Err(DownloadError::InvalidUri)
        );
    }
}
