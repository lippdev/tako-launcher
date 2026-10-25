//! Links `tako://` no núcleo da versão web (WEB-34): o que cada link faz com o estado de tela e com a sessão de
//! jogo. A validação do texto (`tako_core::links::parse`) tem os próprios testes; aqui entram só links já válidos.
//! Só funções de plataforma em memória: nenhum jogo abre.

#![allow(clippy::unwrap_used)]

use serde_json::Value;
use std::sync::{Mutex, mpsc};
use std::time::{Duration, Instant};
use tako_app::protocol::{Call, Reply, UiState};
use tako_app::{AppService, Core, HostRequest, LinkOutcome};
use tako_core::library::{Game, GameStore};
use tako_core::links::{Link, parse};
use tako_core::paths::DataPaths;
use tako_core::persistence::{GameSettingsStore, TakoDatabase};
use tako_core::session::settings::GameSettingsOverride;
use tako_platform::Platform;
use tako_sources::{LibraryScan, LibrarySource};

struct Stub {
    games: Vec<Game>,
    /// Segura a leitura até chegar um sinal (para o link que espera a biblioteca).
    gate: Option<Mutex<mpsc::Receiver<()>>>,
}

impl LibrarySource for Stub {
    fn store(&self) -> GameStore {
        GameStore::Steam
    }
    fn scan(&self) -> LibraryScan {
        if let Some(gate) = &self.gate {
            let _ = gate.lock().unwrap().recv();
        }
        LibraryScan {
            games: self.games.clone(),
            client_installed: true,
            notice: None,
        }
    }
}

fn game(id: &str, installed: bool) -> Game {
    Game {
        id: id.into(),
        title: format!("Jogo {id}"),
        store: Some(GameStore::Steam),
        is_installed: installed,
        launch_target: Some("/jogos/jogo.exe".into()),
        ..Default::default()
    }
}

struct Rig {
    app: AppService,
    host: mpsc::Receiver<HostRequest>,
    paths: DataPaths,
    _dir: tempfile::TempDir,
}

fn open(gate: Option<mpsc::Receiver<()>>) -> Rig {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path());
    let database = TakoDatabase::new(paths.database());
    database.open().unwrap();
    let source = Stub {
        games: vec![
            game("steam:10", true),
            game("steam:11", true),
            game("steam:12", false),
        ],
        gate: gate.map(Mutex::new),
    };
    let core = Core::open_with_sources(
        paths.clone(),
        Platform::simulated(),
        false,
        vec![Box::new(source)],
    )
    .unwrap();
    let mut app = AppService::with_core("test", core);
    let (host_tx, host) = mpsc::channel();
    app.set_host_sink(host_tx);
    Rig {
        app,
        host,
        paths,
        _dir: dir,
    }
}

fn open_ready() -> Rig {
    let mut rig = open(None);
    let deadline = Instant::now() + Duration::from_secs(5);
    while rig.app.open_link(Link::Game {
        id: "steam:10".into(),
        play: false,
    }) != LinkOutcome::Show
    {
        assert!(Instant::now() < deadline, "leitura não terminou");
        rig.app.poll();
        std::thread::sleep(Duration::from_millis(2));
    }
    // O estado do link de teste não conta.
    rig.app
        .handle(Call::UiStateSet(UiState { json: None }))
        .unwrap();
    rig
}

impl Rig {
    fn link(&mut self, url: &str) -> LinkOutcome {
        self.app.open_link(parse(url).expect("link válido"))
    }

    fn screen(&mut self) -> Value {
        let Reply::UiState(UiState { json: Some(json) }) = self.app.handle(Call::UiState).unwrap()
        else {
            panic!("sem estado de tela");
        };
        serde_json::from_str(&json).unwrap()
    }

    fn launches(&self) -> Vec<String> {
        self.host
            .try_iter()
            .filter_map(|r| match r {
                HostRequest::Launch { game_id } => Some(game_id),
                _ => None,
            })
            .collect()
    }
}

#[test]
fn a_game_link_opens_its_page_without_launching() {
    let mut rig = open_ready();
    assert_eq!(rig.link("tako://game/steam:10"), LinkOutcome::Show);
    let screen = rig.screen();
    assert_eq!(screen["view"], "biblioteca");
    assert_eq!(screen["activeGame"], "steam:10");
    assert!(rig.launches().is_empty(), "abrir a página nunca joga");
}

#[test]
fn a_play_link_launches_only_what_the_slint_would_launch_without_asking() {
    let mut rig = open_ready();
    assert_eq!(rig.link("tako://play/steam:10"), LinkOutcome::Handled);
    assert_eq!(rig.launches(), vec!["steam:10".to_owned()]);
    assert_eq!(
        rig.screen()["activeGame"],
        "steam:10",
        "a interface volta no jogo ao terminar"
    );

    // O jogo abre de verdade (simulado) e então: o mesmo jogo volta para a frente; outro jogo pede confirmação.
    rig.app.launch_game("steam:10");
    assert_eq!(rig.link("tako://play/steam:10"), LinkOutcome::Handled);
    assert!(
        rig.host
            .try_iter()
            .any(|r| matches!(r, HostRequest::Resume))
    );
    assert_eq!(rig.link("tako://play/steam:11"), LinkOutcome::Show);
    assert!(rig.launches().is_empty());
    assert_eq!(rig.screen()["activeGame"], "steam:11");
}

#[test]
fn a_play_link_asks_when_commands_are_not_approved_or_the_game_is_not_installed() {
    let mut rig = open_ready();
    let store = GameSettingsStore::new({
        let db = TakoDatabase::new(rig.paths.database());
        db.open().unwrap();
        db
    });
    store
        .save(
            "steam:11",
            &GameSettingsOverride {
                before_command: Some("montar.bat".into()),
                approved_commands: None,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        rig.link("tako://play/steam:11"),
        LinkOutcome::Show,
        "comando sem aprovação: quem decide é o usuário, na página do jogo"
    );
    assert_eq!(
        rig.link("tako://play/steam:12"),
        LinkOutcome::Show,
        "jogo não instalado"
    );
    assert!(rig.launches().is_empty());
    assert_eq!(rig.screen()["activeGame"], "steam:12");
}

#[test]
fn screens_set_the_view_and_the_library_tab() {
    let mut rig = open_ready();
    assert_eq!(rig.link("tako://settings"), LinkOutcome::Show);
    assert_eq!(rig.screen()["view"], "configuracoes");
    assert_eq!(rig.link("tako://library/installed"), LinkOutcome::Show);
    let screen = rig.screen();
    assert_eq!(screen["view"], "biblioteca");
    assert_eq!(screen["library"]["filter"], "installed");
    assert_eq!(screen["activeGame"], Value::Null);
    assert_eq!(rig.link("tako://home"), LinkOutcome::Show);
    assert_eq!(rig.screen()["activeGame"], Value::Null);
}

#[test]
fn an_unknown_game_does_nothing_once_the_library_is_read() {
    let mut rig = open_ready();
    assert_eq!(rig.link("tako://game/steam:999"), LinkOutcome::Missing);
    assert_eq!(rig.link("tako://play/steam:999"), LinkOutcome::Missing);
    assert!(rig.launches().is_empty());
    assert_eq!(
        rig.app.handle(Call::UiState).unwrap(),
        Reply::UiState(UiState { json: None }),
        "nada mudou na tela"
    );
}

#[test]
fn a_link_waits_for_the_first_read_of_the_stores() {
    let (release, gate) = mpsc::channel();
    let mut rig = open(Some(gate));
    assert_eq!(rig.link("tako://game/steam:10"), LinkOutcome::Waiting);
    assert_eq!(rig.app.resume_pending_link(), None, "ainda lendo");
    release.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let outcome = loop {
        rig.app.poll();
        if let Some(outcome) = rig.app.resume_pending_link() {
            break outcome;
        }
        assert!(Instant::now() < deadline, "leitura não terminou");
        std::thread::sleep(Duration::from_millis(2));
    };
    assert_eq!(outcome, LinkOutcome::Show);
    assert_eq!(rig.screen()["activeGame"], "steam:10");
    assert_eq!(rig.app.resume_pending_link(), None, "só uma vez");
}

#[test]
fn only_the_last_waiting_link_counts() {
    let (release, gate) = mpsc::channel();
    let mut rig = open(Some(gate));
    assert_eq!(rig.link("tako://game/steam:10"), LinkOutcome::Waiting);
    assert_eq!(rig.link("tako://game/steam:11"), LinkOutcome::Waiting);
    release.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while rig.app.resume_pending_link().is_none() {
        rig.app.poll();
        assert!(Instant::now() < deadline, "leitura não terminou");
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(rig.screen()["activeGame"], "steam:11");
}
