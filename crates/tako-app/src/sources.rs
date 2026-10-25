//! Leitura bloqueante das lojas isolada do núcleo. Só mensagens atravessam a fronteira da thread.

use crate::HostRequest;
use crate::protocol::{Event, LibrarySourceReport, LibrarySources};
use chrono::{SecondsFormat, Utc};
use std::collections::{HashMap, HashSet};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Instant;
use tako_core::i18n::Lang;
use tako_core::library::{Game, storage};
use tako_sources::LibrarySource;
use tako_sources::emulators::{Config, EmulatorSource, keys};

pub(crate) type Factory = Box<dyn Fn() -> Arc<Vec<Box<dyn LibrarySource>>> + Send>;
type Wake = Arc<Mutex<Option<mpsc::Sender<HostRequest>>>>;

pub(crate) enum Update {
    Progress(LibrarySources),
    Finished(Vec<Game>),
    Failed(String),
}

/// Nenhum JoinHandle: fechar o núcleo não espera uma loja ou pasta lenta. Ao perder o receptor, o trabalhador
/// abandona a leitura entre fontes e encerra. Ele nunca recebe referência ao núcleo nem ao snapshot.
pub(crate) struct Scanner {
    /// Cada pedido leva as lojas desligadas da hora (`DisabledStores`), não as da abertura.
    commands: Option<mpsc::Sender<String>>,
    updates: mpsc::Receiver<Update>,
    wake: Wake,
    pub state: LibrarySources,
}

impl Scanner {
    pub fn open(
        factory: Factory,
        disabled: String,
        lang: Lang,
        simulate: bool,
    ) -> std::io::Result<Self> {
        let (sender, updates) = mpsc::channel();
        let wake: Wake = Arc::default();
        let mut scanner = Self {
            commands: None,
            updates,
            wake: Arc::clone(&wake),
            state: LibrarySources::default(),
        };
        if simulate {
            // Nem a construção das fontes pode consultar registro, ambiente ou disco na demonstração.
            return Ok(scanner);
        }
        let (commands, requests) = mpsc::channel::<String>();
        std::thread::Builder::new()
            .name("library-sources".into())
            .spawn(move || {
                let send = |update| {
                    let delivered = sender.send(update).is_ok();
                    if delivered
                        && let Some(host) = &*wake.lock().unwrap_or_else(|e| e.into_inner())
                        && host.send(HostRequest::PollBackground).is_err()
                    {
                        tracing::debug!("leitura sem hospedeiro para despertar");
                    }
                    delivered
                };
                let result = catch_unwind(AssertUnwindSafe(|| {
                    for disabled in requests {
                        let sources = factory();
                        if !scan(&sources, &disabled, lang, &send) {
                            break;
                        }
                    }
                }));
                if result.is_err() {
                    tracing::error!("trabalhador de leitura das lojas terminou com pânico");
                    send(Update::Failed("Falha na leitura das lojas".into()));
                }
            })?;
        scanner.commands = Some(commands);
        scanner.start(&disabled)?;
        Ok(scanner)
    }

    /// Um pedido durante a leitura é ignorado, inclusive se o resultado ainda aguarda o núcleo.
    pub fn start(&mut self, disabled: &str) -> std::io::Result<bool> {
        if self.state.scanning || self.commands.is_none() {
            return Ok(false);
        }
        self.commands
            .as_ref()
            .ok_or_else(|| std::io::Error::other("leitor indisponível"))?
            .send(disabled.to_owned())
            .map_err(|_| std::io::Error::other("leitor das lojas encerrado"))?;
        self.state.scanning = true;
        self.state.completed = 0;
        self.state.measuring_sizes = false;
        self.state.error = None;
        Ok(true)
    }

    pub fn set_host_sink(&mut self, sink: mpsc::Sender<HostRequest>) {
        *self.wake.lock().unwrap_or_else(|e| e.into_inner()) = Some(sink.clone());
        // O resultado pode ter chegado entre Core::open e a instalação do hospedeiro.
        if self.state.scanning && sink.send(HostRequest::PollBackground).is_err() {
            tracing::debug!("hospedeiro encerrado antes da leitura");
        }
    }

    pub fn next(&mut self) -> Option<Update> {
        match self.updates.try_recv() {
            Ok(update) => Some(update),
            Err(mpsc::TryRecvError::Disconnected) if self.state.scanning => {
                Some(Update::Failed("Leitor das lojas encerrado".into()))
            }
            Err(_) => None,
        }
    }
}

/// As pastas de ROMs e os executáveis gravados nas configurações.
pub(crate) fn emulator_config(settings: &HashMap<String, String>) -> Config {
    Config::parse(
        settings.get(keys::ROM_FOLDERS).map(String::as_str),
        settings.get(keys::PATHS).map(String::as_str),
        settings.get(keys::BY_SYSTEM).map(String::as_str),
    )
}

/// As fontes de cada leitura. A configuração dos emuladores é lida a cada leitura (e não só ao abrir), para uma
/// pasta de ROMs nova entrar na leitura seguinte sem reabrir o Tako.
pub(crate) fn factory(emulators: Arc<Mutex<Config>>) -> Factory {
    Box::new(move || {
        let config = emulators.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let mut sources = vec![tako_sources::steam_source(None)];
        #[cfg(windows)]
        sources.extend(tako_sources::windows_sources(
            Arc::new(tako_sources::registry_windows::WindowsRegistry),
            tako_platform::windows::dpapi::unprotect,
        ));
        #[cfg(unix)]
        sources.extend(tako_sources::unix_sources());
        sources.push(Box::new(EmulatorSource::new(config)));
        Arc::new(sources)
    })
}

fn scan(
    sources: &[Box<dyn LibrarySource>],
    disabled: &str,
    lang: Lang,
    send: &impl Fn(Update) -> bool,
) -> bool {
    let mut state = LibrarySources {
        scanning: true,
        total: sources.len(),
        ..Default::default()
    };
    let mut games = Vec::new();
    let mut ids = HashSet::new();
    if !send(Update::Progress(state.clone())) {
        return false;
    }
    for source in sources {
        let store = source.store();
        let enabled = !disabled
            .split(',')
            .any(|s| s.trim().eq_ignore_ascii_case(store.key()));
        let started = Instant::now();
        let result = enabled.then(|| catch_unwind(AssertUnwindSafe(|| source.scan())));
        let before = games.len();
        let (client_installed, error) = match result {
            Some(Ok(scan)) => {
                games.extend(scan.games.into_iter().filter(|g| ids.insert(g.id.clone())));
                (scan.client_installed, scan.notice)
            }
            Some(Err(_)) => {
                tracing::error!(
                    fonte = store.key(),
                    "falha ao ler loja; continuando as outras"
                );
                (false, Some("Falha ao ler a fonte".into()))
            }
            None => (false, None),
        };
        if let Some(error) = &error {
            tracing::warn!(fonte = store.key(), aviso = error, "relatório da loja");
        }
        state.sources.push(LibrarySourceReport {
            store: store.into(),
            name: tako_core::presentation::game_text::store_name(lang, store),
            enabled,
            found: games.len() - before,
            client_installed,
            error,
            last_scanned_at: enabled
                .then(|| Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)),
            elapsed_ms: enabled
                .then(|| u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)),
        });
        state.completed += 1;
        if !send(Update::Progress(state.clone())) {
            return false;
        }
    }
    state.measuring_sizes = true;
    if !send(Update::Progress(state)) {
        return false;
    }
    for game in games.iter_mut().filter(|g| storage::needs_measure(g)) {
        if let Some(bytes) = game
            .install_path
            .as_deref()
            .and_then(|path| storage::folder_size(std::path::Path::new(path), storage::MAX_ENTRIES))
            .filter(|bytes| *bytes > 0)
        {
            game.size_bytes = Some(bytes);
            game.size_label = tako_sources::steam::size_label(bytes);
        }
    }
    send(Update::Finished(games))
}

impl crate::AppService {
    /// Recolhe só mensagens disponíveis; nunca espera pela descoberta nem pela medição de pastas. O loop do
    /// hospedeiro chama isto ao receber PollBackground. Também roda antes de cada chamada da ponte.
    pub fn poll(&mut self) {
        while let Some(update) = self.core.as_mut().and_then(|core| core.sources.next()) {
            match update {
                Update::Progress(mut state) => {
                    if let Some(core) = &mut self.core {
                        // Uma leitura nova não apaga os relatórios das lojas ainda não relidas.
                        for previous in &core.sources.state.sources {
                            if !state.sources.iter().any(|s| s.store == previous.store) {
                                state.sources.push(previous.clone());
                            }
                        }
                        core.sources.state = state.clone();
                    }
                    self.emit(Event::LibraryScanProgress(state));
                }
                Update::Finished(games) => {
                    if let Some(core) = &mut self.core {
                        core.reload_library(games);
                        core.sources.state.scanning = false;
                        core.sources.state.measuring_sizes = false;
                    }
                    self.emit(Event::LibraryChanged);
                    self.emit_scan_progress();
                }
                Update::Failed(error) => {
                    tracing::error!("leitura das lojas: {error}");
                    if let Some(core) = &mut self.core {
                        core.sources.state.scanning = false;
                        core.sources.state.measuring_sizes = false;
                        core.sources.state.error = Some(error);
                    }
                    self.emit_scan_progress();
                }
            }
        }
        self.poll_downloads();
        self.poll_edit();
        self.poll_art();
    }

    pub(super) fn emit_scan_progress(&self) {
        if let Some(core) = &self.core {
            self.emit(Event::LibraryScanProgress(core.sources.state.clone()));
        }
    }
}
