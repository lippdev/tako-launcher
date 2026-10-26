//! Atualização do app pela ponte (WEB-33): `updates.*` e o evento `updates.changed`. A consulta às releases é falsa e o
//! download vem de um servidor HTTP na própria máquina, então a conferência do SHA-256 é a de verdade; o instalador é
//! falso e anota o que recebeu. Nada sai da máquina e nenhum instalador roda.

#![allow(clippy::unwrap_used)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};
use tako_app::protocol::{
    Call, ErrorCode, Event, GameRequest, Reply, UpdatePrefs, UpdateState, UpdateStatus,
    parse_request,
};
use tako_app::{AppService, Core, HostRequest, UpdateFeed, UpdateInstaller, UpdateSetup};
use tako_core::library::{Game, GameStore};
use tako_core::paths::DataPaths;
use tako_core::updates::{
    Arch, Asset, Channel, Package, SemVer, UpdateError, UpdateInfo, UpdatePreferences,
};
use tako_platform::Platform;
use tako_sources::{LibraryScan, LibrarySource};

/// SHA-256 de 240 000 bytes `a`.
const DIGEST: &str = "sha256:fa72c362ac1eacb3ec4130131f0842cff4a3a61871b36de2a96244940c283ed1";
const SIZE: usize = 240_000;
const ASSET: &str = "Tako-Portable-x64.exe";

fn body() -> Vec<u8> {
    vec![b'a'; SIZE]
}

fn tampered() -> Vec<u8> {
    let mut bytes = body();
    bytes[1234] = b'b';
    bytes
}

/// Serve um corpo por conexão, em três pedaços com pausa (para o progresso aparecer). Devolve o endereço.
fn serve(bodies: Vec<Vec<u8>>, pause: Duration) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/{ASSET}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for bytes in bodies {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                if stream.read_exact(&mut byte).is_err() {
                    break;
                }
                request.push(byte[0]);
            }
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            );
            if stream.write_all(head.as_bytes()).is_err() {
                continue;
            }
            for part in bytes.chunks(bytes.len() / 3 + 1) {
                if stream.write_all(part).is_err() {
                    break;
                }
                let _ = stream.flush();
                std::thread::sleep(pause);
            }
        }
    });
    url
}

fn info(url: &str, digest: &str) -> UpdateInfo {
    UpdateInfo {
        version: "0.3.0".into(),
        name: "Tako 0.3.0".into(),
        notes: "# Novidades\n- Primeira\n- Segunda\n- Terceira\n- Quarta".into(),
        page_url: "https://github.com/lippdev/tako-launcher/releases/tag/v0.3.0".into(),
        asset: Some(Asset {
            url: url.into(),
            name: ASSET.into(),
            digest: digest.into(),
            size: SIZE as u64,
        }),
        is_prerelease: false,
    }
}

type Answer = Result<Option<UpdateInfo>, UpdateError>;

/// A consulta às releases: responde o que o teste mandou e anota o canal pedido.
#[derive(Clone, Default)]
struct Feed {
    answers: Arc<Mutex<Vec<Answer>>>,
    channels: Arc<Mutex<Vec<Channel>>>,
}

impl Feed {
    fn answer(&self, answer: Answer) {
        self.answers.lock().unwrap().push(answer);
    }
    fn asked(&self) -> Vec<Channel> {
        self.channels.lock().unwrap().clone()
    }
}

impl UpdateFeed for Feed {
    fn check(&self, _: &SemVer, channel: Channel, _: Package, _: Arch) -> Answer {
        self.channels.lock().unwrap().push(channel);
        let mut answers = self.answers.lock().unwrap();
        if answers.is_empty() {
            Ok(None)
        } else {
            answers.remove(0)
        }
    }
}

/// O instalador falso: anota (arquivo, reabrir) e não executa nada.
#[derive(Clone, Default)]
struct Installer {
    calls: Arc<Mutex<Vec<(PathBuf, bool)>>>,
}

impl Installer {
    fn calls(&self) -> Vec<(PathBuf, bool)> {
        self.calls.lock().unwrap().clone()
    }
}

impl UpdateInstaller for Installer {
    fn install(&self, _: &DataPaths, downloaded: &Path, relaunch: bool) -> std::io::Result<bool> {
        self.calls
            .lock()
            .unwrap()
            .push((downloaded.to_path_buf(), relaunch));
        Ok(true)
    }
}

struct Stub(Vec<Game>);

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

struct Rig {
    app: AppService,
    feed: Feed,
    installer: Installer,
    host: mpsc::Receiver<HostRequest>,
    events: mpsc::Receiver<Event>,
    paths: DataPaths,
    downloads: PathBuf,
    _dir: tempfile::TempDir,
}

fn rig() -> Rig {
    rig_with(|_| {}, vec![])
}

fn rig_with(change: impl FnOnce(&mut UpdateSetup), games: Vec<Game>) -> Rig {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path());
    let platform = Platform {
        processes: Arc::new(tako_core::devices::fake::FakeProcesses::default()),
        ..Platform::simulated()
    };
    let core = Core::open_with_sources(paths.clone(), platform, false, vec![Box::new(Stub(games))])
        .unwrap();
    let mut app = AppService::with_core("0.2.0", core);
    let (host_tx, host) = mpsc::channel();
    let (event_tx, events) = mpsc::channel();
    app.set_host_sink(host_tx);
    app.set_event_sink(event_tx);
    let (feed, installer) = (Feed::default(), Installer::default());
    let downloads = dir.path().join("TakoUpdate");
    let mut setup = UpdateSetup {
        feed: Arc::new(feed.clone()),
        installer: Arc::new(installer.clone()),
        package: Package::WindowsPortable,
        arch: Some(Arch::X64),
        current: SemVer::parse("0.2.0"),
        download_dir: downloads.clone(),
        debug_build: false,
        min_check: Duration::ZERO,
    };
    change(&mut setup);
    app.set_update_setup(setup);
    Rig {
        app,
        feed,
        installer,
        host,
        events,
        paths,
        downloads,
        _dir: dir,
    }
}

impl Rig {
    fn call(&mut self, call: Call) -> Result<Reply, tako_app::protocol::BridgeError> {
        self.app.handle(call)
    }

    fn status(&mut self) -> UpdateStatus {
        match self.call(Call::UpdatesStatus).unwrap() {
            Reply::UpdatesStatus(status) => status,
            other => panic!("updates.status: {other:?}"),
        }
    }

    fn check(&mut self) -> UpdateStatus {
        match self.call(Call::UpdatesCheck).unwrap() {
            Reply::UpdatesCheck(status) => status,
            other => panic!("updates.check: {other:?}"),
        }
    }

    /// Chama `poll` (o que o hospedeiro faz ao receber `PollBackground`) até o estado ser `want`.
    fn until(&mut self, want: UpdateState) -> UpdateStatus {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            self.app.poll();
            let status = self.status();
            if status.state == want {
                return status;
            }
            assert!(
                Instant::now() < deadline,
                "esperava {want:?}, está em {:?}",
                status.state
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn changes(&self) -> Vec<UpdateStatus> {
        self.events
            .try_iter()
            .filter_map(|e| match e {
                Event::UpdatesChanged(status) => Some(status),
                _ => None,
            })
            .collect()
    }

    fn hosted(&self) -> Vec<HostRequest> {
        self.host
            .try_iter()
            .filter(|r| !matches!(r, HostRequest::PollBackground))
            .collect()
    }

    /// Procura, baixa do servidor e deixa pronta.
    fn ready(&mut self, url: &str) -> UpdateStatus {
        self.feed.answer(Ok(Some(info(url, DIGEST))));
        self.check();
        self.until(UpdateState::Ready)
    }
}

fn game() -> Game {
    Game {
        id: "steam:10".into(),
        title: "Árvore".into(),
        store: Some(GameStore::Steam),
        is_installed: true,
        launch_target: Some("/jogos/arvore.exe".into()),
        ..Default::default()
    }
}

#[test]
fn the_calls_take_no_payload_and_the_preferences_take_an_object() {
    for name in ["status", "check", "download", "apply", "preferences"] {
        let ok = format!(r#"{{"v":1,"id":1,"name":"updates.{name}"}}"#);
        assert!(parse_request(&ok).is_ok(), "{name}");
        let extra = format!(r#"{{"v":1,"id":1,"name":"updates.{name}","payload":{{}}}}"#);
        assert!(parse_request(&extra).is_err(), "{name}");
    }
    let set = r#"{"v":1,"id":2,"name":"updates.preferences.set","payload":{"autoUpdate":true,"prereleases":false,"nightly":false}}"#;
    assert!(parse_request(set).is_ok());
    let unknown = r#"{"v":1,"id":2,"name":"updates.preferences.set","payload":{"autoUpdate":true,"prereleases":false,"nightly":false,"x":1}}"#;
    assert!(parse_request(unknown).is_err());
    assert!(
        parse_request(r#"{"v":1,"id":2,"name":"updates.preferences.set","payload":1}"#).is_err()
    );
}

#[test]
fn up_to_date_is_reported_with_the_channel_from_the_preferences() {
    let mut rig = rig();
    assert_eq!(rig.status().state, UpdateState::Idle);
    assert_eq!(rig.status().current, "0.2.0");
    let searching = rig.check();
    assert_eq!(searching.state, UpdateState::Searching);
    let done = rig.until(UpdateState::Current);
    assert!(done.offer.is_none());
    assert_eq!(rig.feed.asked(), [Channel::Stable]);
    let states: Vec<_> = rig.changes().iter().map(|s| s.state).collect();
    assert_eq!(states, [UpdateState::Searching, UpdateState::Current]);
    // Versões de teste e noturnas mudam o canal, como no Slint.
    for (prefs, channel) in [
        (
            UpdatePrefs {
                auto_update: true,
                prereleases: true,
                nightly: false,
            },
            Channel::Testing,
        ),
        (
            UpdatePrefs {
                auto_update: true,
                prereleases: false,
                nightly: true,
            },
            Channel::Nightly,
        ),
    ] {
        assert_eq!(
            rig.call(Call::UpdatesPreferencesSet(prefs)).unwrap(),
            Reply::UpdatesPreferencesSet(prefs)
        );
        rig.check();
        rig.until(UpdateState::Current);
        assert_eq!(rig.feed.asked().last(), Some(&channel));
    }
}

#[test]
fn a_failed_check_is_reported_and_can_be_tried_again() {
    let mut rig = rig();
    rig.feed.answer(Err(UpdateError::Http("sem rede".into())));
    rig.check();
    let failed = rig.until(UpdateState::CheckFailed);
    assert!(failed.offer.is_none());
    rig.check();
    rig.until(UpdateState::Current);
    // Arquitetura sem arquivo na release: a procura falha sem consultar nada.
    let mut other = rig_with(|setup| setup.arch = None, vec![]);
    assert_eq!(other.check().state, UpdateState::CheckFailed);
    assert!(other.feed.asked().is_empty());
}

#[test]
fn a_new_version_downloads_with_progress_and_becomes_ready() {
    let url = serve(vec![body()], Duration::from_millis(200));
    let mut rig = rig();
    let ready = rig.ready(&url);
    assert_eq!(ready.progress, 1.0);
    assert_eq!(ready.downloaded_bytes, SIZE as u64);
    let offer = ready.offer.unwrap();
    assert_eq!(offer.version, "0.3.0");
    assert_eq!(offer.notes, ["Primeira", "Segunda", "Terceira"]);
    assert!(offer.installable);
    assert_eq!(offer.size, SIZE as u64);
    // O arquivo está lá, inteiro e conferido, e nada sobrou do download.
    let files: Vec<_> = std::fs::read_dir(&rig.downloads)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(files, [ASSET]);
    assert_eq!(std::fs::read(rig.downloads.join(ASSET)).unwrap(), body());
    let changes = rig.changes();
    let states: Vec<_> = changes.iter().map(|s| s.state).collect();
    assert_eq!(
        &states[..2],
        [UpdateState::Searching, UpdateState::Downloading]
    );
    assert_eq!(states.last(), Some(&UpdateState::Ready));
    // O progresso anda aos poucos, nunca volta e termina em 1.
    let steps: Vec<f64> = changes
        .iter()
        .filter(|s| s.state == UpdateState::Downloading)
        .map(|s| s.progress)
        .collect();
    assert!(steps.iter().any(|p| *p > 0.0 && *p < 1.0), "{steps:?}");
    assert!(steps.windows(2).all(|w| w[0] <= w[1]), "{steps:?}");
    // Uma busca com tudo pronto não refaz nada.
    assert_eq!(rig.check().state, UpdateState::Ready);
}

#[test]
fn a_tampered_download_is_refused_and_nothing_is_kept_or_run() {
    let url = serve(vec![tampered(), body()], Duration::ZERO);
    let mut rig = rig();
    rig.feed.answer(Ok(Some(info(&url, DIGEST))));
    rig.check();
    let failed = rig.until(UpdateState::DownloadFailed);
    // A oferta continua (para "Tentar de novo"), mas não há arquivo nenhum.
    assert_eq!(failed.offer.unwrap().version, "0.3.0");
    assert_eq!(std::fs::read_dir(&rig.downloads).unwrap().count(), 0);
    let refused = rig.call(Call::UpdatesApply).unwrap_err();
    assert_eq!(refused.code, ErrorCode::Conflict);
    assert!(rig.installer.calls().is_empty());
    assert!(rig.hosted().is_empty());
    // Tentar de novo baixa o arquivo certo.
    let again = rig.call(Call::UpdatesDownload).unwrap();
    assert!(matches!(again, Reply::UpdatesDownload(s) if s.state == UpdateState::Downloading));
    rig.until(UpdateState::Ready);
}

#[test]
fn a_release_without_a_published_hash_is_only_offered_as_a_page() {
    let mut rig = rig();
    let mut release = info("http://127.0.0.1:9/x", "");
    release.asset = None;
    release.page_url = "http://exemplo.invalid/release".into();
    rig.feed.answer(Ok(Some(release)));
    rig.check();
    let status = rig.until(UpdateState::Available);
    let offer = status.offer.unwrap();
    assert!(!offer.installable);
    // O endereço da página só passa em https.
    assert_eq!(
        offer.page_url,
        "https://github.com/lippdev/tako-launcher/releases"
    );
    assert_eq!(
        rig.call(Call::UpdatesDownload).unwrap_err().code,
        ErrorCode::Conflict
    );
    assert_eq!(
        rig.call(Call::UpdatesApply).unwrap_err().code,
        ErrorCode::Conflict
    );
    // Instalação que não se atualiza sozinha (.deb) também só mostra a página, mesmo com arquivo.
    let mut deb = rig_with(|setup| setup.package = Package::LinuxDeb, vec![]);
    deb.feed
        .answer(Ok(Some(info("http://127.0.0.1:9/x", DIGEST))));
    deb.check();
    assert!(!deb.until(UpdateState::Available).offer.unwrap().installable);
}

#[test]
fn apply_runs_the_installer_once_with_relaunch_and_asks_the_host_to_exit() {
    let url = serve(vec![body()], Duration::ZERO);
    let mut rig = rig();
    rig.ready(&url);
    rig.changes();
    assert_eq!(rig.call(Call::UpdatesApply).unwrap(), Reply::UpdatesApply);
    assert_eq!(rig.installer.calls(), [(rig.downloads.join(ASSET), true)]);
    assert_eq!(rig.hosted(), [HostRequest::Exit]);
    assert_eq!(rig.status().state, UpdateState::Installing);
    assert_eq!(
        rig.changes().last().map(|s| s.state),
        Some(UpdateState::Installing)
    );
    // Uma segunda chamada não roda o instalador de novo.
    assert_eq!(
        rig.call(Call::UpdatesApply).unwrap_err().code,
        ErrorCode::Conflict
    );
    assert_eq!(rig.installer.calls().len(), 1);
    // Ao sair depois de aplicada, não instala outra vez.
    rig.app.install_update_on_exit();
    assert_eq!(rig.installer.calls().len(), 1);
}

#[test]
fn apply_without_a_ready_update_does_nothing() {
    let mut rig = rig();
    let refused = rig.call(Call::UpdatesApply).unwrap_err();
    assert_eq!(refused.code, ErrorCode::Conflict);
    assert!(rig.installer.calls().is_empty() && rig.hosted().is_empty());
}

#[test]
fn a_file_changed_after_the_download_is_discarded_instead_of_run() {
    let url = serve(vec![body()], Duration::ZERO);
    let mut rig = rig();
    rig.ready(&url);
    let file = rig.downloads.join(ASSET);
    std::fs::write(&file, tampered()).unwrap();
    let refused = rig.call(Call::UpdatesApply).unwrap_err();
    assert_eq!(refused.code, ErrorCode::Conflict);
    assert!(rig.installer.calls().is_empty() && rig.hosted().is_empty());
    assert!(!file.exists(), "o arquivo adulterado foi apagado");
    assert_eq!(rig.status().state, UpdateState::DownloadFailed);
    // E "Depois" também confere antes de instalar ao sair.
    let url = serve(vec![body()], Duration::ZERO);
    let mut rig = rig_with(|_| {}, vec![]);
    rig.ready(&url);
    std::fs::write(rig.downloads.join(ASSET), tampered()).unwrap();
    rig.app.install_update_on_exit();
    assert!(rig.installer.calls().is_empty());
}

#[test]
fn with_a_game_open_the_restart_waits() {
    let mut rig = rig_with(|_| {}, vec![game()]);
    // A biblioteca precisa estar lida para o jogo abrir.
    let deadline = Instant::now() + Duration::from_secs(5);
    while rig
        .app
        .handle(Call::GameDetails(GameRequest {
            game_id: "steam:10".into(),
        }))
        .is_err()
    {
        assert!(Instant::now() < deadline, "leitura não terminou");
        std::thread::sleep(Duration::from_millis(2));
    }
    let url = serve(vec![body()], Duration::ZERO);
    rig.ready(&url);
    rig.call(Call::GamePlay(GameRequest {
        game_id: "steam:10".into(),
    }))
    .unwrap();
    let launch = rig
        .host
        .try_iter()
        .find_map(|r| match r {
            HostRequest::Launch { game_id } => Some(game_id),
            _ => None,
        })
        .unwrap();
    rig.app.launch_game(&launch);
    assert!(rig.app.game_active());
    assert!(rig.status().game_running);
    let refused = rig.call(Call::UpdatesApply).unwrap_err();
    assert_eq!(refused.code, ErrorCode::Conflict);
    assert!(refused.message.contains("jogo"), "{}", refused.message);
    assert!(rig.installer.calls().is_empty() && rig.hosted().is_empty());
    // Ao sair com o jogo aberto também não instala.
    rig.app.install_update_on_exit();
    assert!(rig.installer.calls().is_empty());
    // A atualização continua pronta, esperando o fim do jogo.
    assert_eq!(rig.status().state, UpdateState::Ready);
}

#[test]
fn leaving_the_app_installs_a_ready_update_without_relaunching() {
    let url = serve(vec![body()], Duration::ZERO);
    let mut rig = rig();
    rig.app.install_update_on_exit();
    assert!(
        rig.installer.calls().is_empty(),
        "nada pronto, nada a instalar"
    );
    rig.ready(&url);
    rig.app.install_update_on_exit();
    assert_eq!(rig.installer.calls(), [(rig.downloads.join(ASSET), false)]);
}

#[test]
fn checking_on_open_follows_auto_update_and_skips_development_builds() {
    let path = |rig: &Rig| rig.paths.updates();
    // Padrão: procura ao abrir.
    let mut rig = rig();
    rig.feed.answer(Ok(None));
    rig.app.check_updates_on_open();
    rig.until(UpdateState::Current);
    assert_eq!(rig.feed.asked().len(), 1);
    // AutoUpdate desligado: não consulta.
    let mut rig = self::rig();
    UpdatePreferences {
        auto_update: false,
        ..Default::default()
    }
    .save(&path(&rig))
    .unwrap();
    rig.app.check_updates_on_open();
    assert!(rig.feed.asked().is_empty());
    assert_eq!(rig.status().state, UpdateState::Idle);
    // Compilação de desenvolvimento: não consulta ao abrir, mas o pedido do usuário funciona.
    let mut rig = rig_with(|setup| setup.debug_build = true, vec![]);
    rig.app.check_updates_on_open();
    assert!(rig.feed.asked().is_empty());
    rig.check();
    rig.until(UpdateState::Current);
    assert_eq!(rig.feed.asked().len(), 1);
}

#[test]
fn the_failed_swap_marker_is_reported_once_on_open() {
    let mut rig = rig_with(|setup| setup.debug_build = true, vec![]);
    std::fs::create_dir_all(&rig.downloads).unwrap();
    std::fs::write(rig.downloads.join("failed"), "").unwrap();
    rig.app.check_updates_on_open();
    assert_eq!(rig.status().state, UpdateState::DownloadFailed);
    assert!(!rig.downloads.join("failed").exists());
    assert_eq!(
        rig.changes().last().map(|s| s.state),
        Some(UpdateState::DownloadFailed)
    );
}

#[test]
fn preferences_default_to_automatic_and_are_saved_in_updates_json() {
    let mut rig = rig();
    let Reply::UpdatesPreferences(prefs) = rig.call(Call::UpdatesPreferences).unwrap() else {
        panic!("updates.preferences");
    };
    assert_eq!(
        prefs,
        UpdatePrefs {
            auto_update: true,
            prereleases: false,
            nightly: false
        }
    );
    let wanted = UpdatePrefs {
        auto_update: false,
        prereleases: true,
        nightly: false,
    };
    rig.call(Call::UpdatesPreferencesSet(wanted)).unwrap();
    let saved = UpdatePreferences::load(&rig.paths.updates());
    assert_eq!(
        (saved.auto_update, saved.prereleases, saved.nightly),
        (false, true, false)
    );
    let text = std::fs::read_to_string(rig.paths.updates()).unwrap();
    assert!(text.contains("\"AutoUpdate\": false"), "{text}");
}

#[test]
fn the_demo_offers_a_sample_without_touching_the_network() {
    let dir = tempfile::tempdir().unwrap();
    let core = Core::open(DataPaths::at(dir.path()), Platform::simulated(), true).unwrap();
    let mut app = AppService::with_core("0.2.0", core);
    let (host_tx, host) = mpsc::channel();
    app.set_host_sink(host_tx);
    // Na demonstração nem a procura ao abrir roda sozinha.
    app.check_updates_on_open();
    let Reply::UpdatesStatus(idle) = app.handle(Call::UpdatesStatus).unwrap() else {
        panic!("updates.status");
    };
    assert_eq!(idle.state, UpdateState::Idle);
    app.handle(Call::UpdatesCheck).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let status = loop {
        app.poll();
        let Reply::UpdatesStatus(status) = app.handle(Call::UpdatesStatus).unwrap() else {
            panic!("updates.status");
        };
        if status.state == UpdateState::Ready {
            break status;
        }
        assert!(Instant::now() < deadline, "{:?}", status.state);
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(status.offer.unwrap().version, "9.9.9");
    // Reiniciar na demonstração não faz nada: a janela não fecha.
    assert_eq!(app.handle(Call::UpdatesApply).unwrap(), Reply::UpdatesApply);
    assert!(host.try_iter().all(|r| !matches!(r, HostRequest::Exit)));
    let Reply::UpdatesStatus(after) = app.handle(Call::UpdatesStatus).unwrap() else {
        panic!("updates.status");
    };
    assert_eq!(after.state, UpdateState::Ready);
}
