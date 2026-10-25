//! API local de integração no núcleo da versão web (WEB-34, EXT-002, ADR-0033): ligada só pela chave do Slint
//! (`LocalApiEnabled`), com o contrato, o token e os limites de `tako_core::api`. Porta 0 (efêmera) e `127.0.0.1`.

#![allow(clippy::unwrap_used)]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tako_app::protocol::{Call, Reply};
use tako_app::{AppService, Core};
use tako_core::api::runtime::Dispatch;
use tako_core::api::{ApiActionError, is_valid_token};
use tako_core::library::{Game, GameStore};
use tako_core::links::Link;
use tako_core::paths::DataPaths;
use tako_core::persistence::{SettingsStore, TakoDatabase};
use tako_platform::Platform;
use tako_sources::{LibraryScan, LibrarySource};

struct Stub;

impl LibrarySource for Stub {
    fn store(&self) -> GameStore {
        GameStore::Steam
    }
    fn scan(&self) -> LibraryScan {
        LibraryScan {
            games: vec![Game {
                id: "steam:570".into(),
                title: "Dota 2".into(),
                store: Some(GameStore::Steam),
                is_installed: true,
                launch_target: Some("/jogos/dota.exe".into()),
                ..Default::default()
            }],
            client_installed: true,
            notice: None,
        }
    }
}

struct Rig {
    app: AppService,
    settings: SettingsStore,
    _dir: tempfile::TempDir,
}

/// O núcleo de verdade (banco numa pasta temporária) com as chaves da API como o Slint as grava.
fn open(settings: &[(&str, &str)]) -> Rig {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path());
    let database = TakoDatabase::new(paths.database());
    database.open().unwrap();
    let store = SettingsStore::new(database);
    for (key, value) in settings {
        store.set(key, value).unwrap();
    }
    let core =
        Core::open_with_sources(paths, Platform::simulated(), false, vec![Box::new(Stub)]).unwrap();
    let mut app = AppService::with_core("test", core);
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let Reply::LibrarySources(state) = app.handle(Call::LibrarySources).unwrap() else {
            panic!("relatório esperado");
        };
        if !state.scanning {
            break;
        }
        assert!(Instant::now() < deadline, "leitura não terminou");
        std::thread::sleep(Duration::from_millis(2));
    }
    Rig {
        app,
        settings: store,
        _dir: dir,
    }
}

fn call(method: &str, port: u16, path: &str, token: Option<&str>) -> (u16, String) {
    let url = format!("http://127.0.0.1:{port}{path}");
    let mut request = ureq::http::Request::builder().method(method).uri(url);
    if let Some(token) = token {
        request = request.header("Authorization", format!("Bearer {token}"));
    }
    let request = request.body(()).unwrap();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into();
    let mut response = agent.run(request).unwrap();
    (
        response.status().as_u16(),
        response.body_mut().read_to_string().unwrap(),
    )
}

fn recording() -> (Dispatch, Arc<Mutex<Vec<Link>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = seen.clone();
    let dispatch: Dispatch = Arc::new(move |link| {
        sink.lock().unwrap().push(link);
        Ok(())
    });
    (dispatch, seen)
}

#[test]
fn it_stays_off_when_the_key_is_off_or_missing() {
    for settings in [
        &[][..],
        &[("LocalApiEnabled", "false"), ("LocalApiPort", "0")][..],
    ] {
        let mut rig = open(settings);
        rig.app.sync_local_api();
        assert_eq!(rig.app.local_api_port(), None);
        assert_eq!(
            rig.settings.get("LocalApiToken").unwrap(),
            None,
            "desligada, nem token se gera"
        );
    }
}

#[test]
fn it_answers_like_the_slint_when_the_key_is_on() {
    let mut rig = open(&[("LocalApiEnabled", "true"), ("LocalApiPort", "0")]);
    let (dispatch, seen) = recording();
    rig.app.set_link_dispatch(dispatch);
    rig.app.sync_local_api();
    let port = rig.app.local_api_port().expect("ligada pela chave");

    // O token nasce na primeira vez e fica nas configurações, como no Slint.
    let token = rig.settings.get("LocalApiToken").unwrap().unwrap();
    assert!(is_valid_token(&token));

    let (status, body) = call("GET", port, "/api/v1/games", Some(&token));
    assert_eq!(status, 200);
    let games: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        games,
        serde_json::json!([{
            "id": "steam:570", "title": "Dota 2", "store": "Steam", "installed": true
        }])
    );
    let (status, body) = call("GET", port, "/api/v1/state", Some(&token));
    assert_eq!(status, 200);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&body).unwrap(),
        serde_json::json!({ "is_game_running": false, "running_game": null })
    );

    // Autenticação, `Host` e métodos como no Slint.
    assert_eq!(call("GET", port, "/api/v1/games", None).0, 401);
    assert_eq!(
        call("GET", port, "/api/v1/games", Some(&"0".repeat(32))).0,
        401
    );
    assert_eq!(call("OPTIONS", port, "/api/v1/games", Some(&token)).0, 405);

    // Abrir e jogar chegam como o mesmo link `tako://`; jogo desconhecido nem chega.
    assert_eq!(
        call("POST", port, "/api/v1/games/steam:570/open", Some(&token)).0,
        200
    );
    assert_eq!(
        call("POST", port, "/api/v1/games/steam:570/play", Some(&token)).0,
        200
    );
    assert_eq!(
        call("POST", port, "/api/v1/games/steam:1/play", Some(&token)).0,
        404
    );
    assert_eq!(
        *seen.lock().unwrap(),
        vec![
            Link::Game {
                id: "steam:570".into(),
                play: false
            },
            Link::Game {
                id: "steam:570".into(),
                play: true
            },
        ]
    );
}

#[test]
fn actions_are_unavailable_without_a_host_to_deliver_them() {
    let mut rig = open(&[("LocalApiEnabled", "true"), ("LocalApiPort", "0")]);
    rig.app.sync_local_api();
    let port = rig.app.local_api_port().unwrap();
    let token = rig.settings.get("LocalApiToken").unwrap().unwrap();
    assert_eq!(
        call("POST", port, "/api/v1/games/steam:570/open", Some(&token)).0,
        503
    );
    let closed: Dispatch = Arc::new(|_| Err(ApiActionError::Unavailable));
    rig.app.set_link_dispatch(closed);
    rig.app.sync_local_api();
    let port = rig.app.local_api_port().unwrap();
    assert_eq!(
        call("POST", port, "/api/v1/games/steam:570/play", Some(&token)).0,
        503
    );
}

#[test]
fn an_invalid_stored_token_is_replaced_and_the_old_one_never_works() {
    let mut rig = open(&[
        ("LocalApiEnabled", "true"),
        ("LocalApiPort", "0"),
        ("LocalApiToken", "curto"),
    ]);
    rig.app.sync_local_api();
    let port = rig.app.local_api_port().unwrap();
    let token = rig.settings.get("LocalApiToken").unwrap().unwrap();
    assert!(is_valid_token(&token));
    assert_eq!(call("GET", port, "/api/v1/games", Some("curto")).0, 401);
    assert_eq!(call("GET", port, "/api/v1/games", Some(&token)).0, 200);
}

#[test]
fn the_token_never_reaches_the_bridge() {
    let mut rig = open(&[("LocalApiEnabled", "true"), ("LocalApiPort", "0")]);
    rig.app.sync_local_api();
    let token = rig.settings.get("LocalApiToken").unwrap().unwrap();
    // Nenhuma chave da API está na lista de configurações da interface (ela não lê nem grava o segredo).
    for key in ["LocalApiToken", "LocalApiEnabled", "LocalApiPort"] {
        let request: tako_app::protocol::SettingsGetRequest =
            serde_json::from_value(serde_json::json!({ "keys": [key] })).unwrap();
        assert!(rig.app.handle(Call::SettingsGet(request)).is_err(), "{key}");
    }
    let Reply::SettingsGet(all) = rig
        .app
        .handle(Call::SettingsGet(serde_json::from_str("{}").unwrap()))
        .unwrap()
    else {
        panic!("configurações");
    };
    assert!(!serde_json::to_string(&all).unwrap().contains(&token));
}
