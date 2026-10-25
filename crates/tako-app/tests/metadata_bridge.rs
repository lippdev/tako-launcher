//! Rede falsa: fila, escolhas em Fontes e IDs, cache, arte e ausência de credenciais na ponte.
#![allow(clippy::unwrap_used)]

use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};
use tako_app::protocol::{
    Call, Event, GameRequest, GameSourceIdSet, LibraryListRequest, Reply, SourceIdChoice,
};
use tako_app::{AppService, Core};
use tako_core::library::{ArtKind, Game};
use tako_core::metadata::agent::MetadataAgent;
use tako_core::metadata::art_cache::ArtCache;
use tako_core::metadata::cache::MetadataCache;
use tako_core::metadata::game_art::GameArt;
use tako_core::metadata::http::{Http, HttpRequest, HttpResponse, SystemClock};
use tako_core::metadata::steam_art::SteamArtResolver;
use tako_core::metadata::{GameMetadata, MetadataLookup, MetadataMatch, MetadataProvider};
use tako_core::paths::DataPaths;
use tako_platform::Platform;

#[derive(Default)]
struct Source {
    fetched: Mutex<Vec<String>>,
    block: Option<(mpsc::Sender<()>, Mutex<mpsc::Receiver<()>>)>,
}

impl MetadataProvider for Source {
    fn name(&self) -> &str {
        "IGDB"
    }
    fn is_enabled(&self) -> bool {
        true
    }
    fn find_exact(&self, _: &MetadataLookup) -> Option<String> {
        Some("1".into())
    }
    fn search(&self, _: &MetadataLookup) -> Vec<MetadataMatch> {
        panic!("ID conhecido buscou por nome")
    }
    fn fetch(&self, id: &str) -> Option<GameMetadata> {
        self.fetched.lock().unwrap().push(id.into());
        if id == "1"
            && let Some((started, release)) = &self.block
        {
            started.send(()).unwrap();
            release
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(10))
                .unwrap();
        }
        Some(GameMetadata {
            source_ids: [("IGDB".into(), id.into())].into(),
            description: Some(format!("Descrição {id}")),
            genres: vec![format!("Gênero {id}")],
            developer: Some(format!("Estúdio {id}")),
            publisher: (id != "3").then(|| format!("Publicadora {id}")),
            release_date: chrono::NaiveDate::from_ymd_opt(2000 + id.parse::<i32>().unwrap(), 1, 1),
            cover_url: Some(format!("https://images.igdb.com/{id}.png")),
            ..GameMetadata::new("IGDB")
        })
    }
}

struct Images;
impl Http for Images {
    fn send(&self, _: &HttpRequest) -> Result<HttpResponse, String> {
        Ok(HttpResponse {
            status: 200,
            body: b"\x89PNG\r\n\x1a\n".to_vec(),
            content_type: Some("image/png".into()),
            retry_after: None,
        })
    }
}

fn app(paths: &DataPaths, source: Arc<Source>) -> (AppService, mpsc::Receiver<Event>) {
    let http = Arc::new(Images);
    let cache = Arc::new(ArtCache::new(
        paths.art_cache(),
        http.clone(),
        SteamArtResolver::new(http, None),
        None,
    ));
    let core = Core::open(paths.clone(), Platform::simulated(), true)
        .unwrap()
        .with_art_pipeline(
            paths,
            cache,
            MetadataAgent::new(vec![source], Arc::new(SystemClock)),
        );
    let mut app = AppService::with_core("test", core);
    let (tx, events) = mpsc::channel();
    app.set_event_sink(tx);
    app.reload_library(vec![Game {
        id: "gog:32".into(),
        title: "Jogo".into(),
        is_favorite: true,
        ..Game::default()
    }])
    .unwrap();
    (app, events)
}

fn details(app: &mut AppService) -> tako_app::protocol::GameDetails {
    let Reply::GameDetails(details) = app
        .handle(Call::GameDetails(GameRequest {
            game_id: "gog:32".into(),
        }))
        .unwrap()
    else {
        panic!()
    };
    *details
}

fn choose(app: &mut AppService, choice: SourceIdChoice) {
    app.handle(Call::GameSourceIdsSet(GameSourceIdSet {
        game_id: "gog:32".into(),
        source: "IGDB".into(),
        choice,
    }))
    .unwrap();
}

fn wait(app: &mut AppService, condition: impl Fn(&mut AppService) -> bool) {
    let start = Instant::now();
    loop {
        app.poll();
        if condition(app) {
            return;
        }
        assert!(
            start.elapsed() < Duration::from_secs(12),
            "fila não concluiu"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn queue_caches_and_pinned_id_replaces_fields_and_art_then_none_stops_source() {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path());
    let source = Arc::new(Source::default());
    let (mut app, events) = app(&paths, source.clone());
    let start = Instant::now();
    wait(&mut app, |app| {
        details(app).description.as_deref() == Some("Descrição 1")
    });
    let first = details(&mut app);
    assert_eq!(first.genres, Some(vec!["Gênero 1".into()]));
    assert_eq!(first.publisher.as_deref(), Some("Publicadora 1"));
    assert_eq!(first.developer.as_deref(), Some("Estúdio 1"));
    assert_eq!(first.release_date.as_deref(), Some("2001-01-01"));
    assert!(first.game.favorite);
    assert!(
        events
            .try_iter()
            .any(|event| event == Event::LibraryChanged)
    );
    let art = GameArt::new(&paths);
    wait(&mut app, |_| {
        art.resolve("gog:32", ArtKind::Cover).is_some()
    });
    let old_art = art.resolve("gog:32", ArtKind::Cover).unwrap();
    assert_eq!(
        MetadataCache::new(paths.metadata_cache())
            .read("gog:32")
            .unwrap()
            .description,
        first.description
    );
    let fetched = source.fetched.lock().unwrap().len();
    app.reload_library(vec![Game {
        id: "gog:32".into(),
        title: "Jogo".into(),
        ..Game::default()
    }])
    .unwrap();
    for _ in 0..10 {
        app.poll();
    }
    assert_eq!(source.fetched.lock().unwrap().len(), fetched);

    choose(&mut app, SourceIdChoice::Id("2".into()));
    wait(&mut app, |app| {
        details(app).description.as_deref() == Some("Descrição 2")
    });
    wait(&mut app, |_| {
        art.resolve("gog:32", ArtKind::Cover)
            .is_some_and(|path| path != old_art)
    });
    assert_eq!(
        details(&mut app).publisher.as_deref(),
        Some("Publicadora 2")
    );
    choose(&mut app, SourceIdChoice::Id("3".into()));
    wait(&mut app, |app| {
        details(app).description.as_deref() == Some("Descrição 3")
    });
    assert_eq!(details(&mut app).publisher, None);
    choose(&mut app, SourceIdChoice::None);
    wait(&mut app, |app| details(app).description.is_none());
    assert!(details(&mut app).genres.is_none());
    assert_eq!(*source.fetched.lock().unwrap(), ["1", "2", "3"]);
    choose(&mut app, SourceIdChoice::Auto);
    wait(&mut app, |app| {
        details(app).description.as_deref() == Some("Descrição 1")
    });

    let Reply::LibraryList(page) = app
        .handle(Call::LibraryList(LibraryListRequest {
            genres: vec!["Gênero 1".into()],
            ..Default::default()
        }))
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(page.total, 1);
    assert_eq!(page.genres, ["Gênero 1"]);
    assert_eq!(page.rows[0].publisher.as_deref(), Some("Publicadora 1"));
    assert_eq!(page.rows[0].genres, Some(vec!["Gênero 1".into()]));
    let wire = serde_json::to_string(&page).unwrap();
    assert!(!wire.contains("https://"));
    assert!(!wire.contains("token"));
    assert!(!wire.contains("client_secret"));
    println!(
        "rede falsa: 1 jogo enriquecido, IDs 1 → 2 → 3 → Nenhum → automático em {:?}",
        start.elapsed()
    );
}

#[test]
fn bridge_remains_responsive_and_discards_result_from_previous_choice() {
    let dir = tempfile::tempdir().unwrap();
    let (entered, started) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let source = Arc::new(Source {
        block: Some((entered, Mutex::new(released))),
        ..Default::default()
    });
    let (mut app, events) = app(&DataPaths::at(dir.path()), source);
    app.poll();
    started.recv_timeout(Duration::from_secs(5)).unwrap();
    let start = Instant::now();
    for _ in 0..100 {
        app.handle(Call::AppInfo).unwrap();
    }
    assert!(start.elapsed() < Duration::from_secs(1));
    choose(&mut app, SourceIdChoice::None);
    events.try_iter().for_each(drop);
    release.send(()).unwrap();
    let start = Instant::now();
    wait(&mut app, |_| start.elapsed() > Duration::from_secs(2));
    assert_eq!(details(&mut app).description, None);
    assert!(
        !events
            .try_iter()
            .any(|event| matches!(event, Event::ArtChanged(_)))
    );
}
