//! Atualização do app pela ponte (WEB-33, PRD D-15): procurar, avisar, baixar com progresso, conferir e aplicar, com
//! a mesma regra e os mesmos textos do app Slint (`tako-ui/src/app_updates.rs`). A regra vive em
//! `tako_core::updates`; aqui ficam o estado, os eventos e a ligação com o hospedeiro.
//!
//! - **Procurar.** Canal e "procurar ao abrir" vêm de `updates.json` (as mesmas preferências do Slint). Ao abrir o
//!   núcleo ([`AppService::check_updates_on_open`]) só procura se `AutoUpdate` estiver ligado e não for uma
//!   compilação de desenvolvimento; o pedido do usuário (`updates.check`) sempre procura.
//! - **Baixar.** Achou e o pacote se atualiza sozinho (arquivo com SHA-256 publicado): o download começa sozinho,
//!   como no Slint. Sem arquivo conferível (`.deb`, release sem hash) fica `available` e a interface só leva à página.
//!   O arquivo só vira `ready` depois de o SHA-256 conferir ([`tako_core::updates::download`]); o que não confere é
//!   apagado e nada é executado.
//! - **Aplicar.** `updates.apply` confere o arquivo de novo (ele fica na pasta temporária até o usuário decidir),
//!   roda o instalador ou o script de troca e pede `HostRequest::Exit`: o script espera este processo sair. Com um
//!   jogo aberto não reinicia (`conflict`). "Depois" instala ao sair do Tako ([`AppService::install_update_on_exit`]).
//! - **Rede.** O download usa sempre a rede de verdade (`ureq`); só a consulta às releases (e o instalador) são
//!   trocáveis ([`UpdateSetup`]), para os testes usarem rede e instalador falsos. Na demonstração (`simulate`) nada
//!   sai da máquina: uma versão de exemplo, com download e progresso simulados, e aplicar não faz nada.
//! - Nenhum segredo passa pela ponte: só versão, notas, tamanho e o endereço da página da release (sempre https).

use crate::protocol::*;
use crate::{AppService, HostRequest};
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};
use tako_core::paths::DataPaths;
use tako_core::updates::{
    self, Arch, Asset, Channel, Package, SemVer, UpdateError, UpdateInfo, UpdatePreferences,
};

/// A página das releases, quando a do GitHub não vem em https.
const RELEASES_PAGE: &str = "https://github.com/lippdev/tako-launcher/releases";
/// Tempo mínimo da procura, para o "Procurando…" aparecer (como no Slint).
const MIN_CHECK: Duration = Duration::from_millis(600);
/// Mais rápido que isto o progresso não vira evento (a interface redesenha a cada um).
const PROGRESS_EVERY: Duration = Duration::from_millis(120);

/// Quem consulta as releases. O de verdade é o GitHub; os testes e a demonstração trocam.
pub trait UpdateFeed: Send + Sync {
    fn check(
        &self,
        current: &SemVer,
        channel: Channel,
        package: Package,
        arch: Arch,
    ) -> Result<Option<UpdateInfo>, UpdateError>;
}

/// Quem troca o executável e reinicia (instalador ou script). O de verdade é `tako_core::updates::install`.
pub trait UpdateInstaller: Send + Sync {
    /// `Ok(false)`: esta instalação não se atualiza sozinha.
    fn install(&self, paths: &DataPaths, downloaded: &Path, relaunch: bool) -> io::Result<bool>;
}

struct GithubFeed;

impl UpdateFeed for GithubFeed {
    fn check(
        &self,
        current: &SemVer,
        channel: Channel,
        package: Package,
        arch: Arch,
    ) -> Result<Option<UpdateInfo>, UpdateError> {
        updates::check(&updates::agent(), current, channel, package, arch)
    }
}

struct SystemInstaller;

impl UpdateInstaller for SystemInstaller {
    fn install(&self, paths: &DataPaths, downloaded: &Path, relaunch: bool) -> io::Result<bool> {
        updates::install(paths, downloaded, relaunch)
    }
}

/// Uma versão de exemplo para a demonstração: nenhuma rede.
struct SampleFeed;

impl UpdateFeed for SampleFeed {
    fn check(
        &self,
        _: &SemVer,
        _: Channel,
        _: Package,
        _: Arch,
    ) -> Result<Option<UpdateInfo>, UpdateError> {
        Ok(Some(UpdateInfo {
            version: "9.9.9".into(),
            name: "Tako 9.9.9".into(),
            notes: "- Versão de exemplo da demonstração\n- Nada foi baixado da rede\n- Reiniciar não faz nada aqui".into(),
            page_url: RELEASES_PAGE.into(),
            asset: Some(Asset {
                url: String::new(),
                name: "Tako-Demo.exe".into(),
                digest: String::new(),
                size: 28 * 1024 * 1024,
            }),
            is_prerelease: false,
        }))
    }
}

/// O que a atualização precisa do ambiente. [`UpdateSetup::system`] é o de verdade; os testes montam o deles.
pub struct UpdateSetup {
    pub feed: Arc<dyn UpdateFeed>,
    pub installer: Arc<dyn UpdateInstaller>,
    pub package: Package,
    /// `None` = arquitetura sem arquivo na release: a procura falha.
    pub arch: Option<Arch>,
    pub current: SemVer,
    pub download_dir: PathBuf,
    /// Compilação de desenvolvimento: a procura ao abrir fica desligada (a do usuário funciona).
    pub debug_build: bool,
    pub min_check: Duration,
}

impl UpdateSetup {
    /// O ambiente real: GitHub, o instalador do sistema e o tipo de instalação desta execução.
    pub fn system(paths: &DataPaths) -> Self {
        Self {
            feed: Arc::new(GithubFeed),
            installer: Arc::new(SystemInstaller),
            package: updates::detect_package(paths),
            arch: Arch::current(),
            current: SemVer::current(),
            download_dir: updates::download_dir(),
            debug_build: cfg!(debug_assertions),
            min_check: MIN_CHECK,
        }
    }

    /// A demonstração: versão de exemplo e um instalador que não faz nada.
    fn demo(paths: &DataPaths) -> Self {
        struct Nothing;
        impl UpdateInstaller for Nothing {
            fn install(&self, _: &DataPaths, _: &Path, _: bool) -> io::Result<bool> {
                Ok(true)
            }
        }
        Self {
            feed: Arc::new(SampleFeed),
            installer: Arc::new(Nothing),
            package: Package::WindowsPortable,
            arch: Some(Arch::X64),
            download_dir: paths.cache().join("update-demo"),
            ..Self::system(paths)
        }
    }
}

/// O que as threads de rede entregam ao núcleo.
enum Msg {
    Checked(u64, Result<Option<UpdateInfo>, UpdateError>),
    Progress(f64),
    Downloaded(Result<PathBuf, UpdateError>),
}

type Wake = Arc<Mutex<Option<mpsc::Sender<HostRequest>>>>;

fn notify(wake: &Wake) {
    if let Some(host) = &*wake.lock().unwrap_or_else(|e| e.into_inner())
        && host.send(HostRequest::PollBackground).is_err()
    {
        tracing::debug!("atualização sem hospedeiro para despertar");
    }
}

/// Estado da atualização dentro do `Core`.
pub(crate) struct UpdateHub {
    setup: UpdateSetup,
    paths: DataPaths,
    simulate: bool,
    state: UpdateState,
    info: Option<UpdateInfo>,
    ready: Option<PathBuf>,
    progress: f64,
    started: Option<Instant>,
    generation: u64,
    tx: mpsc::Sender<Msg>,
    rx: mpsc::Receiver<Msg>,
    wake: Wake,
    /// O último estado avisado à interface (só muda → só avisa).
    announced: Option<UpdateStatus>,
}

impl UpdateHub {
    pub(crate) fn new(paths: DataPaths, simulate: bool) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            setup: if simulate {
                UpdateSetup::demo(&paths)
            } else {
                UpdateSetup::system(&paths)
            },
            paths,
            simulate,
            state: UpdateState::Idle,
            info: None,
            ready: None,
            progress: 0.0,
            started: None,
            generation: 0,
            tx,
            rx,
            wake: Arc::default(),
            announced: None,
        }
    }

    pub(crate) fn set_setup(&mut self, setup: UpdateSetup) {
        self.setup = setup;
    }

    pub(crate) fn set_host_sink(&mut self, sink: mpsc::Sender<HostRequest>) {
        *self.wake.lock().unwrap_or_else(|e| e.into_inner()) = Some(sink);
    }

    fn installable(&self, info: &UpdateInfo) -> bool {
        info.can_install() && self.setup.package.self_updates()
    }

    fn status(&self, game_running: bool) -> UpdateStatus {
        let offer = self.info.as_ref().map(|info| UpdateOffer {
            version: info.version.clone(),
            notes: updates::release_notes(&info.notes),
            page_url: if info.page_url.starts_with("https://") {
                info.page_url.clone()
            } else {
                RELEASES_PAGE.into()
            },
            installable: self.installable(info),
            size: info.asset.as_ref().map_or(0, |a| a.size),
        });
        let size = offer.as_ref().map_or(0, |o| o.size);
        let downloading = self.state == UpdateState::Downloading;
        let remaining = self
            .started
            .filter(|_| downloading && self.progress > 0.0)
            .map(|t| (t.elapsed().as_secs_f64() * (1.0 - self.progress) / self.progress).ceil())
            .map(|s| s as u64)
            .filter(|s| *s > 0);
        UpdateStatus {
            state: self.state,
            current: self.setup.current.to_string(),
            offer,
            progress: self.progress,
            downloaded_bytes: (size as f64 * self.progress) as u64,
            remaining_seconds: remaining,
            game_running,
        }
    }

    /// Começa a procura, a menos que já haja uma em curso, um download, uma atualização pronta ou o Tako esteja saindo.
    fn check(&mut self) {
        if matches!(
            self.state,
            UpdateState::Searching | UpdateState::Downloading | UpdateState::Installing
        ) || self.ready.is_some()
        {
            return;
        }
        let Some(arch) = self.setup.arch else {
            self.state = UpdateState::CheckFailed;
            return;
        };
        self.generation += 1;
        self.state = UpdateState::Searching;
        self.info = None;
        self.progress = 0.0;
        let generation = self.generation;
        let channel = UpdatePreferences::load(&self.paths.updates()).channel();
        let feed = Arc::clone(&self.setup.feed);
        let (current, package, min) = (
            self.setup.current.clone(),
            self.setup.package,
            self.setup.min_check,
        );
        let (tx, wake) = (self.tx.clone(), Arc::clone(&self.wake));
        let spawned = std::thread::Builder::new()
            .name("update-check".into())
            .spawn(move || {
                let start = Instant::now();
                // Um pânico na consulta não pode deixar o "Procurando…" para sempre.
                let result = catch_unwind(AssertUnwindSafe(|| {
                    feed.check(&current, channel, package, arch)
                }))
                .unwrap_or(Err(UpdateError::Stalled));
                std::thread::sleep(min.saturating_sub(start.elapsed()));
                if tx.send(Msg::Checked(generation, result)).is_ok() {
                    notify(&wake);
                }
            });
        if let Err(e) = spawned {
            tracing::warn!("procura de atualização: {e}");
            self.state = UpdateState::CheckFailed;
        }
    }

    fn found(&mut self, info: UpdateInfo) {
        let installable = self.installable(&info);
        self.info = Some(info);
        if installable {
            self.start_download();
        } else {
            self.state = UpdateState::Available;
        }
    }

    /// Baixa a versão achada. `false` = não há o que baixar (sem versão, sem arquivo conferível ou já em curso).
    fn start_download(&mut self) -> bool {
        let Some(info) = self.info.clone().filter(|i| self.installable(i)) else {
            return false;
        };
        if matches!(
            self.state,
            UpdateState::Downloading | UpdateState::Installing
        ) || self.ready.is_some()
        {
            return false;
        }
        self.state = UpdateState::Downloading;
        self.progress = 0.0;
        self.started = Some(Instant::now());
        let (tx, wake) = (self.tx.clone(), Arc::clone(&self.wake));
        let dir = self.setup.download_dir.clone();
        let simulate = self.simulate;
        let spawned = std::thread::Builder::new()
            .name("update-download".into())
            .spawn(move || {
                let send = |msg| {
                    if tx.send(msg).is_ok() {
                        notify(&wake);
                    }
                };
                if simulate {
                    // Sem rede: 20 passos de meio segundo até o fim.
                    for step in 1..=20 {
                        std::thread::sleep(Duration::from_millis(250));
                        send(Msg::Progress(f64::from(step) / 20.0));
                    }
                    send(Msg::Downloaded(Ok(dir.join(info_name(&info)))));
                    return;
                }
                let mut last_sent = Instant::now();
                let result = updates::download(&updates::agent(), &info, &dir, &mut |fraction| {
                    if fraction >= 1.0 || last_sent.elapsed() >= PROGRESS_EVERY {
                        last_sent = Instant::now();
                        send(Msg::Progress(fraction));
                    }
                });
                send(Msg::Downloaded(result));
            });
        if let Err(e) = spawned {
            tracing::warn!("download da atualização: {e}");
            self.state = UpdateState::DownloadFailed;
        }
        true
    }

    /// Recolhe o que as threads entregaram, sem esperar.
    fn poll(&mut self) {
        while let Ok(message) = self.rx.try_recv() {
            match message {
                Msg::Checked(generation, result)
                    if generation == self.generation && self.state == UpdateState::Searching =>
                {
                    match result {
                        Ok(None) => self.state = UpdateState::Current,
                        Ok(Some(info)) => self.found(info),
                        Err(e) => {
                            tracing::warn!("atualizações: {e}");
                            self.state = UpdateState::CheckFailed;
                        }
                    }
                }
                Msg::Checked(..) => {}
                Msg::Progress(fraction) if self.state == UpdateState::Downloading => {
                    // Em milésimos, como o Slint: o mesmo valor não vira outro evento.
                    self.progress = (fraction.clamp(0.0, 1.0) * 1000.0).floor() / 1000.0;
                }
                Msg::Progress(_) => {}
                Msg::Downloaded(result) if self.state == UpdateState::Downloading => match result {
                    Ok(path) => {
                        self.progress = 1.0;
                        self.ready = Some(path);
                        self.state = UpdateState::Ready;
                    }
                    Err(e) => {
                        tracing::warn!("download da atualização: {e}");
                        self.state = UpdateState::DownloadFailed;
                    }
                },
                Msg::Downloaded(_) => {}
            }
        }
    }
}

/// Só o último componente do nome do arquivo, como o `download` faz.
fn info_name(info: &UpdateInfo) -> String {
    info.asset
        .as_ref()
        .and_then(|a| Path::new(&a.name).file_name())
        .map_or_else(
            || "tako-update".into(),
            |n| n.to_string_lossy().into_owned(),
        )
}

fn conflict(message: &str) -> BridgeError {
    BridgeError::new(ErrorCode::Conflict, message)
}

fn prefs_of(prefs: UpdatePreferences) -> UpdatePrefs {
    UpdatePrefs {
        auto_update: prefs.auto_update,
        prereleases: prefs.prereleases,
        nightly: prefs.nightly,
    }
}

impl AppService {
    /// Troca o ambiente da atualização (testes e hospedeiros de teste).
    pub fn set_update_setup(&mut self, setup: UpdateSetup) {
        if let Some(core) = &mut self.core {
            core.updates.set_setup(setup);
        }
    }

    /// Ao abrir o núcleo: avisa se o script de troca deixou `failed` (a versão antiga foi restaurada) e procura
    /// atualização se `AutoUpdate` estiver ligado. Compilação de desenvolvimento e demonstração não procuram.
    pub fn check_updates_on_open(&mut self) {
        let Some(core) = self.core.as_mut() else {
            return;
        };
        let hub = &mut core.updates;
        if updates::take_failed(&hub.setup.download_dir) {
            tracing::warn!("a troca do executável falhou; a versão antiga foi restaurada");
            hub.state = UpdateState::DownloadFailed;
        } else if !hub.simulate
            && !hub.setup.debug_build
            && UpdatePreferences::load(&hub.paths.updates()).auto_update
        {
            hub.check();
        }
        self.announce_updates();
    }

    /// "Depois" = instala ao sair do Tako, sem reabrir (o Slint faz o mesmo em `ExitToDesktop`). O hospedeiro chama
    /// isto quando o app vai terminar. Nada com um jogo aberto, sem atualização pronta ou na demonstração.
    pub fn install_update_on_exit(&mut self) {
        if self.game_active() {
            return;
        }
        let Some(core) = self.core.as_mut() else {
            return;
        };
        let hub = &mut core.updates;
        let Some(file) = hub.ready.clone().filter(|_| !hub.simulate) else {
            return;
        };
        if hub.state == UpdateState::Installing || !verified(hub, &file) {
            return;
        }
        match hub.setup.installer.install(&hub.paths, &file, false) {
            Ok(true) => hub.state = UpdateState::Installing,
            Ok(false) => {}
            Err(e) => tracing::warn!("atualização ao sair: {e}"),
        }
    }

    pub(crate) fn handle_updates(&mut self, call: Call) -> Result<Reply, BridgeError> {
        let reply = match call {
            // Só consultas: o estado que elas leem não mudou, então não há o que avisar.
            Call::UpdatesStatus => return Ok(Reply::UpdatesStatus(self.update_status()?)),
            Call::UpdatesCheck => {
                self.require_core_mut()?.updates.check();
                Reply::UpdatesCheck(self.update_status()?)
            }
            Call::UpdatesDownload => {
                let hub = &mut self.require_core_mut()?.updates;
                if !hub.start_download() {
                    return Err(conflict("nenhuma atualização para baixar"));
                }
                Reply::UpdatesDownload(self.update_status()?)
            }
            Call::UpdatesApply => {
                self.apply_update()?;
                Reply::UpdatesApply
            }
            Call::UpdatesPreferences => {
                let paths = &self.require_core_mut()?.updates.paths;
                return Ok(Reply::UpdatesPreferences(prefs_of(
                    UpdatePreferences::load(&paths.updates()),
                )));
            }
            Call::UpdatesPreferencesSet(prefs) => {
                let path = self.require_core_mut()?.updates.paths.updates();
                let saved = UpdatePreferences {
                    auto_update: prefs.auto_update,
                    prereleases: prefs.prereleases,
                    nightly: prefs.nightly,
                };
                saved.save(&path).map_err(|e| {
                    BridgeError::new(ErrorCode::Internal, format!("preferências: {e}"))
                })?;
                Reply::UpdatesPreferencesSet(prefs_of(UpdatePreferences::load(&path)))
            }
            other => unreachable!("não é de atualização: {other:?}"),
        };
        self.announce_updates();
        Ok(reply)
    }

    /// Recolhe o que as threads de rede entregaram e avisa a interface se o estado mudou.
    pub(crate) fn poll_updates(&mut self) {
        let Some(core) = self.core.as_mut() else {
            return;
        };
        core.updates.poll();
        self.announce_updates();
    }

    fn require_core_mut(&mut self) -> Result<&mut crate::Core, BridgeError> {
        self.core
            .as_mut()
            .ok_or_else(|| BridgeError::new(ErrorCode::Internal, "núcleo não aberto"))
    }

    fn update_status(&self) -> Result<UpdateStatus, BridgeError> {
        Ok(self.require_core()?.updates.status(self.game_active()))
    }

    /// `updates.changed` só quando o estado mudou desde o último aviso.
    fn announce_updates(&mut self) {
        let game = self.game_active();
        let Some(core) = self.core.as_mut() else {
            return;
        };
        let status = core.updates.status(game);
        // Sem aviso nenhum ainda, "parado" é o que a interface já supõe.
        let unchanged = core
            .updates
            .announced
            .as_ref()
            .map_or(status.state == UpdateState::Idle, |last| *last == status);
        if unchanged {
            return;
        }
        core.updates.announced = Some(status.clone());
        self.emit(Event::UpdatesChanged(status));
    }

    fn apply_update(&mut self) -> Result<(), BridgeError> {
        // O jogo continua sendo contado: reiniciar o Tako não o fecha, então não se reinicia (Slint: "Encerre o jogo").
        if self.game_active() {
            return Err(conflict("encerre o jogo antes de atualizar"));
        }
        let has_host = self.host.is_some();
        let hub = &mut self.require_core_mut()?.updates;
        if hub.state == UpdateState::Installing {
            return Err(conflict("o Tako já está reiniciando para atualizar"));
        }
        let Some(file) = hub
            .ready
            .clone()
            .filter(|_| hub.state == UpdateState::Ready)
        else {
            return Err(conflict("nenhuma atualização pronta"));
        };
        if !hub.simulate && !has_host {
            return Err(BridgeError::new(ErrorCode::Internal, "sem hospedeiro"));
        }
        // O arquivo ficou na pasta temporária desde o download: confere o SHA-256 de novo antes de executar.
        if !hub.simulate && !verified(hub, &file) {
            tracing::error!(file = %file.display(), "a atualização baixada não confere mais; descartada");
            let _ = std::fs::remove_file(&file);
            hub.ready = None;
            hub.state = UpdateState::DownloadFailed;
            self.announce_updates();
            return Err(conflict("o arquivo da atualização não confere"));
        }
        match hub.setup.installer.install(&hub.paths, &file, true) {
            Ok(true) => {}
            Ok(false) => return Err(conflict("esta instalação não se atualiza sozinha")),
            Err(e) => {
                tracing::error!("aplicar a atualização: {e}");
                return Err(BridgeError::new(ErrorCode::Internal, e.to_string()));
            }
        }
        if hub.simulate {
            return Ok(());
        }
        hub.state = UpdateState::Installing;
        // O instalador ou o script já foi iniciado e espera este processo sair.
        self.request(HostRequest::Exit)
    }
}

/// O arquivo pronto ainda confere com o SHA-256 publicado?
fn verified(hub: &UpdateHub, file: &Path) -> bool {
    hub.info
        .as_ref()
        .and_then(|i| i.asset.as_ref())
        .is_some_and(|a| updates::verify_file(file, &a.digest).unwrap_or(false))
}
