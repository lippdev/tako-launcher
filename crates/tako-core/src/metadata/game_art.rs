//! Arte por jogo e tipo: publicação pelo núcleo e resolução local, sem rede, no processo da interface.

use super::art_cache::{ArtCache, MAX_IMAGE_BYTES};
use super::cache::write_cache_file;
use super::http::hex;
use crate::library::{ArtKind, Game};
use crate::paths::DataPaths;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};

/// O índice pertence ao perfil; as imagens continuam no cache da máquina, com o limite do ArtCache.
#[derive(Clone)]
pub struct GameArt {
    root: PathBuf,
    index: PathBuf,
}

impl GameArt {
    /// Índices temporários separados dos jogos da biblioteca.
    pub fn candidates(&self) -> Self {
        Self {
            root: self.root.clone(),
            index: self.index.join("candidates"),
        }
    }
    pub fn new(paths: &DataPaths) -> Self {
        Self {
            root: paths.art_cache(),
            index: paths.root.join("cache/art-index"),
        }
    }

    fn index_path(&self, game_id: &str, kind: ArtKind) -> PathBuf {
        let hash = hex(&Sha256::digest(game_id));
        self.index.join(format!("{hash}-{}.json", kind.key()))
    }

    /// Remove só a publicação deste jogo; o arquivo compartilhado fica no cache até sua limpeza normal.
    pub fn clear(&self, game_id: &str, kind: ArtKind) -> std::io::Result<bool> {
        match std::fs::remove_file(self.index_path(game_id, kind)) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }

    /// Só lê um índice pequeno e um arquivo já no cache. Nenhum id ou conteúdo do índice vira caminho livre.
    pub fn resolve(&self, game_id: &str, kind: ArtKind) -> Option<PathBuf> {
        if let Some(token) = game_id.strip_prefix("\0candidate:") {
            return self.candidates().resolve_index(token, kind);
        }
        self.resolve_index(game_id, kind)
    }

    /// Como [`GameArt::resolve`], sem renovar a data do arquivo no cache: para quem só lê a pasta de dados e não
    /// pode alterá-la (`tako dev-snapshot`).
    pub fn peek(&self, game_id: &str, kind: ArtKind) -> Option<PathBuf> {
        self.locate(game_id, kind)
    }

    fn resolve_index(&self, game_id: &str, kind: ArtKind) -> Option<PathBuf> {
        let file = self.locate(game_id, kind)?;
        super::art_cache::touch(&file);
        Some(file)
    }

    fn locate(&self, game_id: &str, kind: ArtKind) -> Option<PathBuf> {
        let index = confined_file(&self.index, &self.index_path(game_id, kind))?;
        let bytes = read_limited(&index, 1024)?;
        let name: String = serde_json::from_slice(&bytes).ok()?;
        let mut parts = Path::new(&name).components();
        if name.contains(['/', '\\', ':'])
            || !matches!(parts.next(), Some(Component::Normal(_)))
            || parts.next().is_some()
        {
            return None;
        }
        confined_file(&self.root, &self.root.join(name))
    }

    /// Publica só depois de a imagem estar inteira no cache. Caminhos locais/Steam são importados pelo worker.
    pub fn publish(&self, game_id: &str, kind: ArtKind, file: &Path) -> std::io::Result<bool> {
        let file = confined_file(&self.root, file)
            .ok_or_else(|| std::io::Error::other("arte fora do cache"))?;
        if self.resolve(game_id, kind).as_ref() == Some(&file) {
            return Ok(false);
        }
        let name = file
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| std::io::Error::other("nome de arte inválido"))?;
        let bytes = serde_json::to_vec(name).map_err(std::io::Error::other)?;
        write_cache_file(&self.index_path(game_id, kind), &bytes)?;
        Ok(true)
    }
}

fn confined_file(root: &Path, file: &Path) -> Option<PathBuf> {
    let root = root.canonicalize().ok()?;
    let file = file.canonicalize().ok()?;
    (file.starts_with(root) && file.is_file()).then_some(file)
}

/// Lê no máximo `limit + 1` bytes e rejeita o excedente antes de importar uma imagem local.
pub fn read_limited(path: &Path, limit: u64) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() as u64 <= limit).then_some(bytes)
}

/// Mesma escolha usada pelo Slint, inclusive imagens fixadas e mídias de outras lojas.
pub fn uri(game: &Game, kind: ArtKind) -> Option<&str> {
    match kind {
        ArtKind::Cover => game.shown_cover(),
        ArtKind::Hero => game.shown_hero(),
        ArtKind::Logo => game.shown_logo(),
        ArtKind::Icon => game.shown_icon(),
    }
}

#[derive(Default)]
struct Queue {
    games: BTreeMap<String, Game>,
    stop: bool,
}

/// Uma única busca de arte por vez. Pedidos repetidos ainda pendentes são substituídos pela versão mais nova.
pub struct ArtWorker {
    queue: Arc<(Mutex<Queue>, Condvar)>,
}

impl ArtWorker {
    pub fn spawn(
        cache: Arc<ArtCache>,
        index: GameArt,
        changed: impl Fn(String) + Send + 'static,
    ) -> Self {
        let queue = Arc::new((Mutex::new(Queue::default()), Condvar::new()));
        let shared = queue.clone();
        let result = std::thread::Builder::new()
            .name("tako-art".into())
            .spawn(move || {
                loop {
                    let (lock, ready) = &*shared;
                    let mut pending = lock.lock().unwrap_or_else(|e| e.into_inner());
                    while pending.games.is_empty() && !pending.stop {
                        pending = ready.wait(pending).unwrap_or_else(|e| e.into_inner());
                    }
                    if pending.stop {
                        return;
                    }
                    let game = pending.games.pop_first().map(|(_, game)| game);
                    drop(pending);
                    let Some(game) = game else { continue };
                    for kind in ArtKind::ALL {
                        if lock.lock().unwrap_or_else(|e| e.into_inner()).stop {
                            return;
                        }
                        let Some(uri) = uri(&game, kind) else {
                            match index.clear(&game.id, kind) {
                                Ok(true) => changed(game.id.clone()),
                                Ok(false) => {}
                                Err(_) => {
                                    tracing::warn!("não foi possível remover o índice de arte")
                                }
                            }
                            continue;
                        };
                        let missing = index.resolve(&game.id, kind).is_none();
                        let Some(file) = cache.locate(uri) else {
                            continue;
                        };
                        // file:// e o cache da Steam nunca são servidos diretamente ao Chromium.
                        let file = if confined_file(&index.root, &file).is_some() {
                            Some(file)
                        } else {
                            read_limited(&file, MAX_IMAGE_BYTES).and_then(|b| cache.import(uri, &b))
                        };
                        if let Some(file) = file {
                            match index.publish(&game.id, kind, &file) {
                                Ok(published) if published || missing => changed(game.id.clone()),
                                Ok(_) => {}
                                Err(_) => {
                                    tracing::warn!("não foi possível publicar o índice de arte")
                                }
                            }
                        }
                    }
                }
            });
        if result.is_err() {
            tracing::error!("não foi possível iniciar a fila de arte");
        }
        Self { queue }
    }

    pub fn refresh(&self, games: impl IntoIterator<Item = Game>) {
        let (lock, ready) = &*self.queue;
        let mut queue = lock.lock().unwrap_or_else(|e| e.into_inner());
        for game in games {
            queue.games.insert(game.id.clone(), game);
        }
        ready.notify_one();
    }
}

impl Drop for ArtWorker {
    fn drop(&mut self) {
        let (lock, ready) = &*self.queue;
        lock.lock().unwrap_or_else(|e| e.into_inner()).stop = true;
        ready.notify_one();
        // Como MetadataWorker: download em curso tem timeout, mas não prende o encerramento.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::http::{HttpRequest, HttpResponse};
    use crate::metadata::steam_art::SteamArtResolver;
    use crate::metadata::test_support::FakeHttp;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";

    #[test]
    fn candidate_namespace_cannot_overwrite_a_real_game_index() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::at(dir.path());
        let art = GameArt::new(&paths);
        let cache = cache(&paths, Arc::new(FakeHttp::new()));
        let original = cache.import("original", PNG).unwrap();
        let preview = cache.import("preview", b"\x89PNG\r\n\x1a\nother").unwrap();
        art.publish("g-cand-0", ArtKind::Cover, &original).unwrap();
        art.candidates()
            .publish("g-cand-0", ArtKind::Cover, &preview)
            .unwrap();
        assert_eq!(
            art.resolve("g-cand-0", ArtKind::Cover),
            Some(original.canonicalize().unwrap())
        );
        assert_eq!(
            art.candidates().resolve("g-cand-0", ArtKind::Cover),
            Some(preview.canonicalize().unwrap())
        );
    }

    fn cache(paths: &DataPaths, http: Arc<FakeHttp>) -> Arc<ArtCache> {
        Arc::new(ArtCache::new(
            paths.art_cache(),
            http.clone(),
            SteamArtResolver::new(http, None),
            None,
        ))
    }

    fn image(_: &HttpRequest) -> Option<HttpResponse> {
        Some(HttpResponse {
            status: 200,
            body: PNG.to_vec(),
            content_type: Some("image/png".into()),
            retry_after: None,
        })
    }

    #[test]
    fn index_is_live_per_profile_and_never_uses_the_id_as_a_path() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::at(dir.path());
        let reader = GameArt::new(&paths);
        let writer = GameArt::new(&paths);
        assert!(reader.resolve("epic:new", ArtKind::Cover).is_none());
        let cache = cache(&paths, Arc::new(FakeHttp::new()));
        let file = cache
            .import("https://cdn.epicgames.com/cover.png", PNG)
            .unwrap();
        assert!(writer.publish("epic:new", ArtKind::Cover, &file).unwrap());
        assert_eq!(
            reader.resolve("epic:new", ArtKind::Cover),
            Some(file.canonicalize().unwrap())
        );
        assert!(!writer.publish("epic:new", ArtKind::Cover, &file).unwrap());
        assert!(reader.resolve("../epic:new", ArtKind::Cover).is_none());
        assert!(
            GameArt::new(&paths.for_profile(Some("other")))
                .resolve("epic:new", ArtKind::Cover)
                .is_none()
        );
        std::fs::remove_file(file).unwrap();
        assert!(reader.resolve("epic:new", ArtKind::Cover).is_none());
    }

    #[test]
    fn rejects_tampered_paths_and_oversized_or_broken_indexes() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::at(dir.path());
        let art = GameArt::new(&paths);
        std::fs::create_dir_all(&art.index).unwrap();
        std::fs::create_dir_all(&art.root).unwrap();
        let outside = dir.path().join("secret.png");
        std::fs::write(&outside, PNG).unwrap();
        assert!(art.publish("g", ArtKind::Cover, &outside).is_err());
        for value in [
            "../secret.png",
            "..\\secret.png",
            "/secret.png",
            "C:/secret.png",
            "file:///secret.png",
            "x/y.png",
            "..",
            ".",
        ] {
            std::fs::write(
                art.index_path("g", ArtKind::Cover),
                serde_json::to_vec(value).unwrap(),
            )
            .unwrap();
            assert!(art.resolve("g", ArtKind::Cover).is_none(), "{value}");
        }
        for bytes in [b"broken".to_vec(), vec![b' '; 1025]] {
            std::fs::write(art.index_path("g", ArtKind::Cover), bytes).unwrap();
            assert!(art.resolve("g", ArtKind::Cover).is_none());
        }
    }

    #[test]
    fn refuses_links_outside_cache() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::at(dir.path());
        let art = GameArt::new(&paths);
        std::fs::create_dir_all(&art.root).unwrap();
        let outside = dir.path().join("secret.png");
        std::fs::write(&outside, PNG).unwrap();
        let link = art.root.join("link.png");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        #[cfg(windows)]
        {
            // Sem privilégio de symlink, uma junction exercita o mesmo bloqueio de canonicalize.
            let junction = art.root.join("escape");
            assert!(
                std::process::Command::new("cmd")
                    .args(["/c", "mklink", "/J"])
                    .arg(&junction)
                    .arg(dir.path())
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
            assert!(confined_file(&art.root, &junction.join("secret.png")).is_none());
            std::fs::remove_dir(&junction).unwrap();
            if std::os::windows::fs::symlink_file(&outside, &link).is_err() {
                return;
            }
        }
        std::fs::create_dir_all(&art.index).unwrap();
        std::fs::write(art.index_path("g", ArtKind::Cover), b"\"link.png\"").unwrap();
        assert!(art.resolve("g", ArtKind::Cover).is_none());
        assert!(art.publish("g", ArtKind::Cover, &link).is_err());
    }

    #[test]
    fn worker_coalesces_pending_games_and_cache_prevents_duplicate_downloads() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::at(dir.path());
        let (entered, start) = mpsc::channel();
        let (release, released) = mpsc::channel();
        let released = Mutex::new(released);
        let mut http = FakeHttp::new();
        http.rule = Some(Box::new(move |req| {
            if req.url.ends_with("block.png") {
                entered.send(()).unwrap();
                released
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(5))
                    .unwrap();
            }
            image(req)
        }));
        let http = Arc::new(http);
        let cache = cache(&paths, http.clone());
        let index = GameArt::new(&paths);
        let reader = index.clone();
        let (changed, events) = mpsc::channel();
        let worker = ArtWorker::spawn(cache.clone(), index, move |id| {
            changed.send(id).unwrap();
        });
        let game = |id: &str, uri: &str| Game {
            id: id.into(),
            cover_uri: Some(uri.into()),
            ..Default::default()
        };
        worker.refresh([game("steam:block", "https://cdn.steamstatic.com/block.png")]);
        start.recv_timeout(Duration::from_secs(5)).unwrap();
        // Uma consulta ao índice e dez mil chamadas repetidas não esperam a rede bloqueada.
        let before = Instant::now();
        assert!(reader.resolve("epic:new", ArtKind::Cover).is_none());
        for _ in 0..10000 {
            worker.refresh([game("epic:new", "https://cdn.epicgames.com/old.png")]);
        }
        worker.refresh([game("epic:new", "https://cdn.epicgames.com/new.png")]);
        assert!(before.elapsed() < Duration::from_secs(2));
        release.send(()).unwrap();
        assert_eq!(
            events.recv_timeout(Duration::from_secs(5)).unwrap(),
            "steam:block"
        );
        assert_eq!(
            events.recv_timeout(Duration::from_secs(5)).unwrap(),
            "epic:new"
        );
        assert_eq!(
            reader.resolve("epic:new", ArtKind::Cover),
            Some(
                cache
                    .cache_path("https://cdn.epicgames.com/new.png")
                    .canonicalize()
                    .unwrap()
            )
        );
        worker.refresh([game("epic:new", "https://cdn.epicgames.com/new.png")]);
        assert!(events.recv_timeout(Duration::from_millis(150)).is_err());
        assert_eq!(http.sent().len(), 2);
        assert!(http.sent().iter().all(|r| !r.url.ends_with("old.png")));
        // O índice pode sobreviver à limpeza/evicção: baixar o mesmo arquivo de novo precisa de outro aviso.
        cache.clear();
        worker.refresh([game("epic:new", "https://cdn.epicgames.com/new.png")]);
        assert_eq!(
            events.recv_timeout(Duration::from_secs(5)).unwrap(),
            "epic:new"
        );
        assert_eq!(http.sent().len(), 3);
    }

    #[test]
    fn worker_imports_pinned_local_images_and_cache_limit_still_applies() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::at(dir.path());
        let http = Arc::new(FakeHttp::new());
        let cache = cache(&paths, http.clone());
        let outside = dir.path().join("chosen.png");
        std::fs::write(&outside, PNG).unwrap();
        let (tx, rx) = mpsc::channel();
        let art = GameArt::new(&paths);
        let worker = ArtWorker::spawn(cache.clone(), art.clone(), move |id| {
            tx.send(id).unwrap();
        });
        worker.refresh([Game {
            id: "gog:1".into(),
            cover_uri: Some("https://unused/auto.png".into()),
            pinned_art: [(ArtKind::Cover, outside.to_string_lossy().into_owned())].into(),
            ..Default::default()
        }]);
        assert_eq!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), "gog:1");
        assert!(
            art.resolve("gog:1", ArtKind::Cover)
                .unwrap()
                .starts_with(paths.art_cache().canonicalize().unwrap())
        );
        assert!(http.sent().is_empty());
        let mut oversized = vec![0; MAX_IMAGE_BYTES as usize + 1];
        oversized[..PNG.len()].copy_from_slice(PNG);
        assert!(cache.import("oversize", &oversized).is_none());
        cache.set_limit(0);
        assert_eq!(cache.disk_bytes(), 0);
        assert!(art.resolve("gog:1", ArtKind::Cover).is_none());
    }

    #[test]
    fn local_steam_ico_is_imported_without_network() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::at(dir.path());
        let http = Arc::new(FakeHttp::new());
        let cache = cache(&paths, http.clone());
        let ico = dir.path().join("steam.ico");
        std::fs::write(&ico, [0, 0, 1, 0, 1, 0]).unwrap();
        let art = GameArt::new(&paths);
        let (tx, rx) = mpsc::channel();
        let worker = ArtWorker::spawn(cache, art.clone(), move |id| {
            tx.send(id).unwrap();
        });
        worker.refresh([Game {
            id: "steam:1".into(),
            media: [("icon".into(), ico.to_string_lossy().into_owned())].into(),
            ..Default::default()
        }]);
        assert_eq!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), "steam:1");
        assert!(art.resolve("steam:1", ArtKind::Icon).is_some());
        assert!(http.sent().is_empty());
    }
}
