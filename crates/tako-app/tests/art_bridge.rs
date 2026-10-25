//! Arte pela ponte: novos jogos, chegada em segundo plano e demonstração sem rede/cofre.
#![allow(clippy::unwrap_used)]

use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};
use tako_app::protocol::{Call, Event, GameRequest, Reply};
use tako_app::{AppService, Core, HostRequest};
use tako_core::library::{ArtKind, Game};
use tako_core::metadata::agent::MetadataAgent;
use tako_core::metadata::art_cache::ArtCache;
use tako_core::metadata::game_art::GameArt;
use tako_core::metadata::http::{Http, HttpRequest, HttpResponse, SystemClock};
use tako_core::metadata::steam_art::SteamArtResolver;
use tako_core::paths::DataPaths;
use tako_platform::Platform;
use tako_platform::secrets::Secrets;

struct SlowHttp {
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}
impl Http for SlowHttp {
    fn send(&self, _: &HttpRequest) -> Result<HttpResponse, String> {
        self.entered.send(()).unwrap();
        self.release
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        Ok(HttpResponse {
            status: 200,
            body: b"\x89PNG\r\n\x1a\n".to_vec(),
            content_type: Some("image/png".into()),
            retry_after: None,
        })
    }
}

#[test]
fn new_store_game_downloads_without_blocking_bridge_and_event_follows_publication() {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path());
    let (entered, started) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let http = Arc::new(SlowHttp {
        entered,
        release: Mutex::new(released),
    });
    let cache = Arc::new(ArtCache::new(
        paths.art_cache(),
        http.clone(),
        SteamArtResolver::new(http, None),
        None,
    ));
    let core = Core::open(paths.clone(), Platform::simulated(), true)
        .unwrap()
        .with_art_pipeline(
            &paths,
            cache,
            MetadataAgent::new(Vec::new(), Arc::new(SystemClock)),
        );
    let mut app = AppService::with_core("test", core);
    let (host, wake) = mpsc::channel();
    let (events, received) = mpsc::channel();
    app.set_host_sink(host);
    app.set_event_sink(events);
    app.reload_library(vec![Game {
        id: "epic:new".into(),
        title: "New".into(),
        cover_uri: Some("https://cdn.epicgames.com/new.png".into()),
        ..Default::default()
    }])
    .unwrap();
    app.poll();
    started.recv_timeout(Duration::from_secs(5)).unwrap();
    let art = GameArt::new(&paths);
    let before = Instant::now();
    for _ in 0..100 {
        assert!(matches!(
            app.handle(Call::AppInfo).unwrap(),
            Reply::AppInfo(_)
        ));
        assert!(art.resolve("epic:new", ArtKind::Cover).is_none());
    }
    assert!(before.elapsed() < Duration::from_secs(1));
    assert!(
        received
            .try_iter()
            .all(|event| !matches!(event, Event::ArtChanged(_)))
    );
    release.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        assert!(Instant::now() < deadline, "faltou art.changed");
        assert_eq!(
            wake.recv_timeout(Duration::from_secs(5)).unwrap(),
            HostRequest::PollBackground
        );
        app.poll();
        if received.try_iter().any(|event| {
            event
                == Event::ArtChanged(GameRequest {
                    game_id: "epic:new".into(),
                })
        }) {
            assert!(art.resolve("epic:new", ArtKind::Cover).is_some());
            break;
        }
    }
}

struct ForbiddenVault;
impl Secrets for ForbiddenVault {
    fn get(&self, _: &str) -> Option<String> {
        panic!("simulate leu o cofre")
    }
    fn set(&self, _: &str, _: &str) -> Result<(), String> {
        panic!("simulate gravou o cofre")
    }
    fn delete(&self, _: &str) -> Result<(), String> {
        panic!("simulate apagou o cofre")
    }
}

#[test]
fn simulate_does_not_start_art_jobs_or_access_vault_or_disk() {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path().join("not-created"));
    let platform = Platform {
        secrets: Arc::new(ForbiddenVault),
        ..Platform::simulated()
    };
    let core = Core::open(paths.clone(), platform, true).unwrap();
    let mut app = AppService::with_core("test", core);
    let (host, _wake) = mpsc::channel();
    let (events, received) = mpsc::channel();
    app.set_host_sink(host);
    app.set_event_sink(events);
    for _ in 0..10 {
        app.poll();
    }
    assert!(
        !received
            .try_iter()
            .any(|e| matches!(e, Event::ArtChanged(_)))
    );
    assert!(!paths.root.exists());
}

#[test]
fn art_event_contains_only_the_game_id() {
    let event = Event::ArtChanged(GameRequest {
        game_id: "steam:570".into(),
    });
    assert_eq!(
        serde_json::to_value(event).unwrap(),
        serde_json::json!({"name":"art.changed", "payload":{"gameId":"steam:570"}})
    );
}

struct FoundArt;
impl tako_core::metadata::MetadataProvider for FoundArt {
    fn name(&self) -> &str {
        "IGDB"
    }
    fn is_enabled(&self) -> bool {
        true
    }
    fn search(
        &self,
        _: &tako_core::metadata::MetadataLookup,
    ) -> Vec<tako_core::metadata::MetadataMatch> {
        Vec::new()
    }
    fn find_exact(&self, _: &tako_core::metadata::MetadataLookup) -> Option<String> {
        Some("123".into())
    }
    fn fetch(&self, _: &str) -> Option<tako_core::metadata::GameMetadata> {
        Some(tako_core::metadata::GameMetadata {
            source_ids: [("IGDB".into(), "123".into())].into(),
            cover_url: Some("https://images.igdb.com/cover.png".into()),
            ..tako_core::metadata::GameMetadata::new("IGDB")
        })
    }
}

#[test]
fn missing_art_is_discovered_by_metadata_worker_then_downloaded_and_persisted() {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path());
    let (entered, started) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let http = Arc::new(SlowHttp {
        entered,
        release: Mutex::new(released),
    });
    let cache = Arc::new(ArtCache::new(
        paths.art_cache(),
        http.clone(),
        SteamArtResolver::new(http, None),
        None,
    ));
    let agent = MetadataAgent::new(vec![Arc::new(FoundArt)], Arc::new(SystemClock));
    let core = Core::open(paths.clone(), Platform::simulated(), true)
        .unwrap()
        .with_art_pipeline(&paths, cache, agent);
    let mut app = AppService::with_core("test", core);
    let (host, wake) = mpsc::channel();
    let (events, received) = mpsc::channel();
    app.set_host_sink(host);
    app.set_event_sink(events);
    app.reload_library(vec![Game {
        id: "gog:123".into(),
        title: "Found".into(),
        ..Default::default()
    }])
    .unwrap();
    app.poll();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        assert!(Instant::now() < deadline, "faltou a URL dos metadados");
        assert_eq!(
            wake.recv_timeout(Duration::from_secs(5)).unwrap(),
            HostRequest::PollBackground
        );
        app.poll();
        if started.try_recv().is_ok() {
            break;
        }
        // O aviso de metadados já foi processado; o worker de arte pode estar entrando no transporte.
        if tako_core::metadata::cache::MetadataCache::new(paths.metadata_cache())
            .read("gog:123")
            .is_some()
        {
            started.recv_timeout(Duration::from_secs(5)).unwrap();
            break;
        }
    }
    release.send(()).unwrap();
    loop {
        assert!(Instant::now() < deadline, "faltou art.changed");
        assert_eq!(
            wake.recv_timeout(Duration::from_secs(5)).unwrap(),
            HostRequest::PollBackground
        );
        app.poll();
        if received
            .try_iter()
            .any(|e| matches!(e, Event::ArtChanged(ref g) if g.game_id == "gog:123"))
        {
            break;
        }
    }
    assert!(
        GameArt::new(&paths)
            .resolve("gog:123", ArtKind::Cover)
            .is_some()
    );
    let metadata = tako_core::metadata::cache::MetadataCache::new(paths.metadata_cache())
        .read("gog:123")
        .unwrap();
    assert_eq!(
        metadata.cover_url.as_deref(),
        Some("https://images.igdb.com/cover.png")
    );
    // O boot real reutiliza os dados e as imagens fixadas antes da primeira resposta da ponte.
    let db = tako_core::persistence::TakoDatabase::new(paths.database());
    db.open().unwrap();
    let chosen = dir.path().join("chosen.png");
    tako_core::persistence::GameChoicesStore::new(db)
        .set_art(
            "gog:123",
            ArtKind::Cover,
            Some(&format!(
                "file:///{}",
                chosen.to_string_lossy().replace('\\', "/")
            )),
        )
        .unwrap();
    std::fs::write(
        paths.cache().join("found-games.json"),
        serde_json::to_vec(&vec![Game {
            id: "gog:123".into(),
            ..Default::default()
        }])
        .unwrap(),
    )
    .unwrap();
    let core = Core::open_with_sources(paths, Platform::simulated(), false, Vec::new()).unwrap();
    assert_eq!(
        core.library().find("gog:123").unwrap().cover_uri.as_deref(),
        Some("https://images.igdb.com/cover.png")
    );
    assert!(
        core.library()
            .find("gog:123")
            .unwrap()
            .shown_cover()
            .unwrap()
            .starts_with("file://")
    );
}
