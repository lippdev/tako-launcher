//! Downloads pela ponte com o motor falso: nada sai da máquina e nenhuma fonte vem com o Tako.

#![allow(clippy::unwrap_used)]

use chrono::DateTime;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};
use tako_app::protocol::{Event, EventEnvelope, Response};
use tako_app::{AppService, Core, HostRequest};
use tako_core::downloads::fake::MemoryDownloader;
use tako_core::downloads::{Downloader, paths::folder_name, source};
use tako_core::library::{Game, GameStore};
use tako_core::paths::DataPaths;
use tako_core::persistence::{DownloadStore, SettingsStore, SourceRow, TakoDatabase};
use tako_platform::Platform;
use tako_sources::{LibraryScan, LibrarySource};

// Só magnets fictícios (ADR-0028 §8).
const A: &str = "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567";
const B: &str = "magnet:?xt=urn:btih:89abcdef0123456789abcdef0123456789abcdef";
const C: &str = "magnet:?xt=urn:btih:fedcba9876543210fedcba9876543210fedcba98";
const SOURCE_URL: &str = "https://exemplo.invalid/segredo/fonte.json";

struct Stub(Vec<Game>);

// Downloads não exercitam fontes de arte; mantém a fila nova isolada da rede e das credenciais reais.
struct OfflineArt;
impl tako_core::metadata::http::Http for OfflineArt {
    fn send(
        &self,
        _: &tako_core::metadata::http::HttpRequest,
    ) -> Result<tako_core::metadata::http::HttpResponse, String> {
        Err("rede desativada neste teste".into())
    }
}

impl LibrarySource for Stub {
    fn store(&self) -> GameStore {
        GameStore::Steam
    }
    fn scan(&self) -> LibraryScan {
        LibraryScan {
            games: self.0.clone(),
            client_installed: true,
            notice: None,
        }
    }
}

fn game(id: &str, title: &str) -> Game {
    Game {
        id: id.into(),
        title: title.into(),
        store: Some(GameStore::Steam),
        ..Default::default()
    }
}

struct Env {
    app: AppService,
    engine: MemoryDownloader,
    /// Quantas vezes o motor foi criado.
    engines: Arc<AtomicUsize>,
    events: mpsc::Receiver<Event>,
    wakes: mpsc::Receiver<HostRequest>,
    root: PathBuf,
}

/// Grava a fonte do usuário (banco + cache em disco) antes de abrir o núcleo.
fn add_source(paths: &DataPaths) {
    let json = format!(
        r#"{{"name":"Minha fonte","downloads":[
            {{"title":"Jogo Exemplo Deluxe Edition","uris":["{A}"],"fileSize":"12.4 GB","uploadDate":"2026-01-01"}},
            {{"title":"Outra Coisa","uris":["{B}"]}},
            {{"title":"Hollow Knight","uris":["{C}"]}}
        ]}}"#
    );
    let dir = paths.root.join("download-sources");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(source::cache_file(&dir, SOURCE_URL), json).unwrap();
    DownloadStore::new(TakoDatabase::new(paths.database()))
        .save_source(&SourceRow {
            url: SOURCE_URL.into(),
            name: "Minha fonte".into(),
            items: 3,
            updated_at: DateTime::UNIX_EPOCH,
        })
        .unwrap();
}

fn open(
    dir: &Path,
    engine: &MemoryDownloader,
    settings: &[(&str, &str)],
    with_source: bool,
) -> Env {
    let paths = DataPaths::at(dir);
    let store = SettingsStore::new(TakoDatabase::new(paths.database()));
    for (key, value) in settings {
        store.set(key, value).unwrap();
    }
    if with_source {
        add_source(&paths);
    }
    let engines = Arc::new(AtomicUsize::new(0));
    let (count, shared) = (Arc::clone(&engines), engine.clone());
    let root = dir.join("Downloads");
    let core = Core::open_with_downloads(
        paths.clone(),
        Platform::simulated(),
        false,
        vec![Box::new(Stub(vec![
            game("steam:1", "Jogo Exemplo"),
            game("steam:2", "Hollow Knight"),
        ]))],
        move || {
            count.fetch_add(1, Ordering::SeqCst);
            Box::new(shared.clone()) as Box<dyn Downloader>
        },
        root.clone(),
    )
    .unwrap();
    let http = Arc::new(OfflineArt);
    let cache = Arc::new(tako_core::metadata::art_cache::ArtCache::new(
        paths.art_cache(),
        http.clone(),
        tako_core::metadata::steam_art::SteamArtResolver::new(http, None),
        None,
    ));
    let core = core.with_art_pipeline(
        &paths,
        cache,
        tako_core::metadata::agent::MetadataAgent::new(
            Vec::new(),
            Arc::new(tako_core::metadata::http::SystemClock),
        ),
    );
    let mut app = AppService::with_core("test", core);
    let (event_sink, events) = mpsc::channel();
    let (host_sink, wakes) = mpsc::channel();
    app.set_event_sink(event_sink);
    app.set_host_sink(host_sink);
    let deadline = Instant::now() + Duration::from_secs(10);
    while wire(&mut app, json!({"v":1,"id":1,"name":"library.sources"}))["reply"]["data"]["scanning"]
        == true
    {
        assert!(Instant::now() < deadline, "leitura não terminou");
        std::thread::sleep(Duration::from_millis(2));
    }
    Env {
        app,
        engine: engine.clone(),
        engines,
        events,
        wakes,
        root,
    }
}

const ON: [(&str, &str); 2] = [
    ("DownloadsEnabled", "true"),
    ("DownloadsLegalAccepted", "true"),
];

fn wire(app: &mut AppService, request: Value) -> Value {
    let response = match tako_app::protocol::parse_request(&request.to_string()) {
        Ok(request) => match app.handle(request.call) {
            Ok(reply) => Response::ok(request.id, reply),
            Err(error) => Response::err(request.id, error),
        },
        Err((id, error)) => Response::err(id, error),
    };
    let text = serde_json::to_string(&response).unwrap();
    assert_eq!(serde_json::from_str::<Response>(&text).unwrap(), response);
    serde_json::from_str(&text).unwrap()
}

fn call(app: &mut AppService, name: &str, payload: Value) -> Value {
    wire(app, json!({"v":1,"id":5,"name":name,"payload":payload}))
}

fn query(app: &mut AppService, name: &str) -> Value {
    wire(app, json!({"v":1,"id":5,"name":name}))
}

fn data(response: &Value) -> &Value {
    assert_eq!(response["ok"], true, "{response}");
    &response["reply"]["data"]
}

fn code(response: &Value) -> &str {
    assert_eq!(response["ok"], false, "{response}");
    response["error"]["code"].as_str().unwrap()
}

fn changed(env: &Env) -> bool {
    env.events
        .try_iter()
        .any(|event| event == Event::DownloadsChanged)
}

/// Espera as fontes lidas pela thread: o aviso ao hospedeiro e o poll são o caminho real.
fn wait_sources(env: &mut Env) {
    let first = call(&mut env.app, "downloads.sources", json!({}));
    assert_eq!(data(&first)["loading"], true);
    assert_eq!(data(&first)["items"], json!([]));
    // Avisos da leitura das lojas também chegam aqui: repete o que o hospedeiro faria até o aviso das fontes.
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(wake) = env.wakes.recv_timeout(Duration::from_millis(50)) {
            assert_eq!(wake, HostRequest::PollBackground);
        }
        env.app.poll();
        if changed(env) {
            break;
        }
        assert!(Instant::now() < deadline, "fontes não chegaram");
    }
}

fn source_id(env: &mut Env) -> String {
    let response = call(&mut env.app, "downloads.sources", json!({"limit": 0}));
    data(&response)["sources"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn off_by_default_and_nothing_starts_the_engine() {
    for settings in [
        &[][..],
        &[("DownloadsEnabled", "true")][..],
        &[("DownloadsLegalAccepted", "true")][..],
        &[
            ("DownloadsEnabled", "false"),
            ("DownloadsLegalAccepted", "true"),
        ][..],
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut env = open(dir.path(), &MemoryDownloader::default(), settings, true);
        for (name, payload) in [
            ("downloads.list", None),
            ("downloads.summary", None),
            ("downloads.pause", Some(json!({"downloadId":1}))),
            ("downloads.resume", Some(json!({"downloadId":1}))),
            ("downloads.cancel", Some(json!({"downloadId":1}))),
            ("downloads.sources", Some(json!({}))),
            (
                "downloads.add",
                Some(json!({"sourceId":"x","index":0,"title":"t"})),
            ),
        ] {
            let request = match payload {
                Some(payload) => json!({"v":1,"id":2,"name":name,"payload":payload}),
                None => json!({"v":1,"id":2,"name":name}),
            };
            let response = wire(&mut env.app, request);
            assert_eq!(code(&response), "conflict", "{name}");
            assert!(
                response["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("desligados")
            );
        }
        env.app.poll();
        assert_eq!(env.engines.load(Ordering::SeqCst), 0, "{settings:?}");
        assert!(env.engine.started().is_empty());
        assert!(!changed(&env));
        // Ligar depois (a mesma configuração do app Slint) liberta as chamadas e só então cria o motor.
    }
}

#[test]
fn list_add_pause_resume_cancel_and_progress_with_the_fake_engine() {
    let dir = tempfile::tempdir().unwrap();
    let mut env = open(dir.path(), &MemoryDownloader::default(), &ON, true);
    assert_eq!(env.engines.load(Ordering::SeqCst), 1);
    assert_eq!(data(&query(&mut env.app, "downloads.list")), &json!([]));
    wait_sources(&mut env);

    // Fontes: só o domínio aparece, nunca o caminho do endereço.
    let listing = call(&mut env.app, "downloads.sources", json!({}));
    let listing = data(&listing);
    assert_eq!(listing["loading"], false);
    assert_eq!(listing["total"], 3);
    assert_eq!(listing["sources"][0]["name"], "Minha fonte");
    assert_eq!(listing["sources"][0]["host"], "exemplo.invalid");
    assert_eq!(listing["sources"][0]["itemCount"], 3);
    assert!(!listing.to_string().contains("segredo"));
    let sid = listing["sources"][0]["id"].as_str().unwrap().to_owned();
    assert_eq!(listing["items"][0]["title"], "Jogo Exemplo Deluxe Edition");
    assert_eq!(listing["items"][0]["fileSize"], "12.4 GB");
    assert_eq!(listing["items"][0]["sourceId"], sid.as_str());

    let search = |env: &mut Env, payload: Value| call(&mut env.app, "downloads.sources", payload);
    let hits = search(&mut env, json!({"search": "hol KNI"}));
    assert_eq!(data(&hits)["total"], 1);
    assert_eq!(data(&hits)["items"][0]["index"], 2);
    assert_eq!(
        data(&search(&mut env, json!({"search": "zzz"})))["total"],
        0
    );
    let suggested = search(&mut env, json!({"gameId": "steam:1"}));
    assert_eq!(data(&suggested)["total"], 1);
    assert_eq!(
        data(&suggested)["items"][0]["title"],
        "Jogo Exemplo Deluxe Edition"
    );
    let page = search(&mut env, json!({"limit": 2, "offset": 2}));
    assert_eq!(
        (
            data(&page)["total"].clone(),
            data(&page)["items"].as_array().unwrap().len()
        ),
        (json!(3), 1)
    );
    let counted = search(&mut env, json!({"limit": 0, "sourceId": sid}));
    assert_eq!(
        (
            data(&counted)["total"].clone(),
            data(&counted)["items"].clone()
        ),
        (json!(3), json!([]))
    );
    assert_eq!(
        code(&search(&mut env, json!({"sourceId": "nope"}))),
        "not_found"
    );
    assert_eq!(
        code(&search(&mut env, json!({"gameId": "steam:404"}))),
        "not_found"
    );
    assert_eq!(
        code(&search(
            &mut env,
            json!({"gameId": "steam:1", "search": "x"})
        )),
        "bad_request"
    );

    // Começar: o primeiro baixa, o segundo espera a vez; ambos vinculados a jogos.
    env.events.try_iter().for_each(drop);
    let add = |env: &mut Env, index: usize, title: &str, game: Value| {
        call(
            &mut env.app,
            "downloads.add",
            json!({"sourceId": sid, "index": index, "title": title, "gameId": game}),
        )
    };
    let first = add(&mut env, 0, "Jogo Exemplo Deluxe Edition", json!("steam:1"));
    assert_eq!(data(&first)["state"], "downloading");
    assert_eq!(data(&first)["gameId"], "steam:1");
    assert_eq!(data(&first)["source"], "Minha fonte");
    assert!(changed(&env));
    assert_eq!(
        env.engine.started(),
        vec![(
            A.to_owned(),
            env.root.join(folder_name("Jogo Exemplo Deluxe Edition"))
        )]
    );
    let second = add(&mut env, 2, "Hollow Knight", json!("steam:2"));
    assert_eq!(data(&second)["state"], "queued");
    assert_eq!(data(&second)["id"], 2);

    // Recusas: o mesmo item, o mesmo jogo, a fonte mudou, item/fonte/jogo que não existem.
    assert_eq!(
        code(&add(
            &mut env,
            0,
            "Jogo Exemplo Deluxe Edition",
            json!(null)
        )),
        "conflict"
    );
    assert_eq!(
        code(&add(&mut env, 1, "Outra Coisa", json!("steam:1"))),
        "conflict"
    );
    assert_eq!(
        code(&add(&mut env, 1, "Outro título", json!(null))),
        "conflict"
    );
    assert_eq!(
        code(&add(&mut env, 9, "Outra Coisa", json!(null))),
        "not_found"
    );
    assert_eq!(
        code(&add(&mut env, 1, "Outra Coisa", json!("steam:404"))),
        "not_found"
    );
    let unknown_source = call(
        &mut env.app,
        "downloads.add",
        json!({"sourceId": "0".repeat(64), "index": 0, "title": "x"}),
    );
    assert_eq!(code(&unknown_source), "not_found");
    assert_eq!(env.engine.started().len(), 1);

    // Progresso: só chega pelo poll, e a interface é avisada.
    env.events.try_iter().for_each(drop);
    env.engine.set(1, 50, Some(100), false);
    env.app.poll();
    assert!(changed(&env));
    let list = query(&mut env.app, "downloads.list");
    println!(
        "downloads.list = {}",
        serde_json::to_string_pretty(&list).unwrap()
    );
    assert_eq!(data(&list)[0]["progress"], 0.5);
    assert_eq!(data(&list)[0]["speed"], 1000);
    assert_eq!(data(&list)[0]["etaSeconds"], 1);
    assert_eq!(data(&list)[1]["state"], "queued");
    assert_eq!(data(&list)[1]["total"], Value::Null);
    let summary = query(&mut env.app, "downloads.summary");
    assert_eq!(
        data(&summary),
        &json!({"downloading":1,"queued":1,"speed":1000,"done":50,"total":100})
    );

    // Pausar e retomar.
    env.events.try_iter().for_each(drop);
    assert_eq!(
        data(&call(
            &mut env.app,
            "downloads.pause",
            json!({"downloadId":1})
        )),
        &Value::Null
    );
    assert!(env.engine.paused(1));
    assert!(changed(&env));
    let list = query(&mut env.app, "downloads.list");
    assert_eq!(data(&list)[0]["state"], "paused");
    assert_eq!(data(&list)[0]["speed"], 0);
    call(&mut env.app, "downloads.resume", json!({"downloadId":1}));
    assert!(!env.engine.paused(1));
    for name in ["downloads.pause", "downloads.resume", "downloads.cancel"] {
        let missing = call(&mut env.app, name, json!({"downloadId": 99}));
        assert_eq!(code(&missing), "not_found", "{name}");
    }

    // Cancelar apagando os arquivos: o motor é avisado e a fila anda.
    call(
        &mut env.app,
        "downloads.cancel",
        json!({"downloadId": 1, "deleteFiles": true}),
    );
    assert!(env.engine.removed().contains(&(1, true)));
    let list = query(&mut env.app, "downloads.list");
    assert_eq!(data(&list).as_array().unwrap().len(), 1);
    assert_eq!(data(&list)[0]["id"], 2);
    assert_eq!(data(&list)[0]["state"], "downloading");

    // Concluir: sai do motor sem semear, e o jogo fica livre para outro download.
    env.engine.set(2, 100, Some(100), true);
    env.app.poll();
    let list = query(&mut env.app, "downloads.list");
    assert_eq!(data(&list)[0]["state"], "completed");
    assert_eq!(data(&list)[0]["progress"], 1.0);
    assert_eq!(data(&list)[0]["etaSeconds"], Value::Null);
    assert!(env.engine.removed().contains(&(2, false)));
    call(&mut env.app, "downloads.cancel", json!({"downloadId": 2}));
    assert_eq!(data(&query(&mut env.app, "downloads.list")), &json!([]));

    let event = serde_json::to_value(EventEnvelope::from(Event::DownloadsChanged)).unwrap();
    assert_eq!(event, json!({"v":1,"event":{"name":"downloads.changed"}}));
}

#[test]
fn strange_payloads_are_bad_requests() {
    let dir = tempfile::tempdir().unwrap();
    let mut env = open(dir.path(), &MemoryDownloader::default(), &ON, true);
    wait_sources(&mut env);
    let sid = source_id(&mut env);
    for name in ["downloads.list", "downloads.summary"] {
        for payload in [
            json!({}),
            json!([]),
            json!(null),
            json!("x"),
            json!({"extra":1}),
        ] {
            let response = call(&mut env.app, name, payload);
            assert_eq!(code(&response), "bad_request", "{name}");
        }
    }
    let id_payloads = [
        json!(null),
        json!([]),
        json!({}),
        json!({"downloadId": "1"}),
        json!({"downloadId": -1}),
        json!({"downloadId": 1.5}),
        json!({"downloadId": 1, "extra": true}),
    ];
    for name in ["downloads.pause", "downloads.resume", "downloads.cancel"] {
        for payload in &id_payloads {
            let response = call(&mut env.app, name, payload.clone());
            assert_eq!(code(&response), "bad_request", "{name} {payload}");
        }
    }
    let cancel = call(
        &mut env.app,
        "downloads.cancel",
        json!({"downloadId":1,"deleteFiles":"sim"}),
    );
    assert_eq!(code(&cancel), "bad_request");
    let long = "x".repeat(257);
    for payload in [
        json!(null),
        json!([]),
        json!({"limit": -1}),
        json!({"offset": "0"}),
        json!({"search": 5}),
        json!({"search": long}),
        json!({"sourceId": ""}),
        json!({"sourceId": "x".repeat(129)}),
        json!({"gameId": " "}),
        json!({"unknown": 1}),
    ] {
        let response = call(&mut env.app, "downloads.sources", payload.clone());
        assert_eq!(code(&response), "bad_request", "{payload}");
    }
    for payload in [
        json!(null),
        json!([]),
        json!({}),
        json!({"sourceId": sid}),
        json!({"sourceId": sid, "index": -1, "title": "x"}),
        json!({"sourceId": sid, "index": 0, "title": 3}),
        json!({"sourceId": sid, "index": 0, "title": "x", "uri": "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567"}),
        json!({"sourceId": "", "index": 0, "title": "x"}),
        json!({"sourceId": sid, "index": 0, "title": "x".repeat(257)}),
        json!({"sourceId": sid, "index": 0, "title": "x", "gameId": ""}),
    ] {
        let response = call(&mut env.app, "downloads.add", payload.clone());
        assert_eq!(code(&response), "bad_request", "{payload}");
    }
    // O hospedeiro nunca passa o endereço do item: nada chegou ao motor.
    assert!(env.engine.started().is_empty());
    assert_eq!(
        code(&wire(
            &mut env.app,
            json!({"v":1,"id":3,"name":"downloads.nope"})
        )),
        "unknown_call"
    );
    // Sem núcleo aberto, a ponte responde erro interno em vez de falhar.
    let response = wire(
        &mut AppService::new("t"),
        json!({"v":1,"id":4,"name":"downloads.list"}),
    );
    assert_eq!(code(&response), "internal");
}

#[test]
fn downloads_survive_reopening_and_resume_from_the_saved_state() {
    let dir = tempfile::tempdir().unwrap();
    let engine = MemoryDownloader::default();
    {
        let mut env = open(dir.path(), &engine, &ON, true);
        wait_sources(&mut env);
        let sid = source_id(&mut env);
        let added = call(
            &mut env.app,
            "downloads.add",
            json!({"sourceId": sid, "index": 0, "title": "Jogo Exemplo Deluxe Edition", "gameId": "steam:1"}),
        );
        assert_eq!(data(&added)["state"], "downloading");
        engine.set(1, 10, Some(100), false);
        env.app.poll();
    }
    let engine = MemoryDownloader::default();
    let mut env = open(dir.path(), &engine, &[], false);
    // Restaurado na abertura, sem nenhuma chamada: o que baixava volta pausado e o motor não recomeça sozinho.
    assert_eq!(env.engines.load(Ordering::SeqCst), 1);
    assert!(engine.started().is_empty());
    let list = query(&mut env.app, "downloads.list");
    assert_eq!(data(&list)[0]["state"], "paused");
    // Como no app Slint, o banco guarda o estado a cada mudança de estado, não o byte a byte (o motor retoma pela pasta).
    assert_eq!(data(&list)[0]["done"], 0);
    assert_eq!(data(&list)[0]["gameId"], "steam:1");
    call(&mut env.app, "downloads.resume", json!({"downloadId": 1}));
    assert_eq!(engine.started().len(), 1);
    // Já era do Tako: pode escrever sobre a pasta que ele mesmo criou.
    assert_eq!(engine.resumes(), vec![true]);
}

#[test]
fn the_host_is_asked_to_poll_only_while_something_downloads() {
    let dir = tempfile::tempdir().unwrap();
    let mut env = open(dir.path(), &MemoryDownloader::default(), &ON, true);
    wait_sources(&mut env);
    let sid = source_id(&mut env);
    let added = call(
        &mut env.app,
        "downloads.add",
        json!({"sourceId": sid, "index": 1, "title": "Outra Coisa"}),
    );
    assert_eq!(data(&added)["gameId"], Value::Null);
    env.events.try_iter().for_each(drop);
    env.engine.set(1, 30, Some(100), false);
    // Sem nenhuma chamada da interface: o ritmo de 1 s chega ao hospedeiro, que chama poll.
    let started = Instant::now();
    assert_eq!(
        env.wakes.recv_timeout(Duration::from_secs(5)).unwrap(),
        HostRequest::PollBackground
    );
    assert!(started.elapsed() < Duration::from_secs(3));
    env.app.poll();
    assert!(changed(&env));
    let list = query(&mut env.app, "downloads.list");
    assert_eq!(data(&list)[0]["done"], 30);

    // Pausado: ninguém mais acorda o hospedeiro.
    call(&mut env.app, "downloads.pause", json!({"downloadId": 1}));
    std::thread::sleep(Duration::from_millis(100));
    env.wakes.try_iter().for_each(drop);
    assert!(env.wakes.recv_timeout(Duration::from_millis(2500)).is_err());
}

#[test]
fn a_user_without_sources_gets_an_empty_answer() {
    let dir = tempfile::tempdir().unwrap();
    let mut env = open(dir.path(), &MemoryDownloader::default(), &ON, false);
    wait_sources(&mut env);
    let done = call(&mut env.app, "downloads.sources", json!({}));
    assert_eq!(
        data(&done),
        &json!({"loading": false, "sources": [], "items": [], "total": 0})
    );
}
