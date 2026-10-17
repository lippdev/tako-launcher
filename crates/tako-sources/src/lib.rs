//! Fontes de biblioteca (port de `Launcher.Core/Library/{Sources,Steam,Epic,...}`).
//!
//! Cada fonte implementa [`LibrarySource`]. Os parsers de arquivo (VDF, manifestos, bancos) são
//! funções puras, testadas com fixtures fictícias em todos os SOs; só a descoberta de pastas e o
//! acesso ao registro dependem do SO. Arquivos das lojas são dados não confiáveis (PRD §8.3):
//! nada de `unwrap`, limites de tamanho e falha de uma fonte não derruba as outras.

use tako_core::library::{Game, GameStore};

pub mod amazon;
pub mod battlenet;
pub mod ea;
pub mod emulators;
pub mod epic;
pub mod fs;
pub mod gog;
pub mod install_dates;
pub mod playnite;
pub mod registry;
#[cfg(windows)]
pub mod registry_windows;
pub mod report;
pub mod steam;
pub mod text;
pub mod ubisoft;
pub mod uninstall;
pub mod windows_path;
pub mod xbox;

/// Steam: em todos os SOs (raiz achada por [`steam::find_root`]).
/// `owned_cache`: cache dos jogos da conta não instalados (SRC-005), quando o usuário deu a chave da Web API.
pub fn steam_source(owned_cache: Option<std::path::PathBuf>) -> Box<dyn LibrarySource> {
    Box::new(steam::SteamLibrarySource::new(steam::find_root()).with_owned_cache(owned_cache))
}

/// Lojas só do Windows, na ordem da barra lateral (a Steam vem de [`steam_source`]).
#[cfg(windows)]
pub fn windows_sources(
    registry: std::sync::Arc<dyn registry::RegistryReader>,
) -> Vec<Box<dyn LibrarySource>> {
    let env = |k: &str| std::env::var(k).unwrap_or_default();
    let (program_data, local_app_data) = (env("ProgramData"), env("LOCALAPPDATA"));
    let r = || registry.clone();
    vec![
        Box::new(epic::EpicLibrarySource::new(r(), &program_data)),
        Box::new(gog::GogLibrarySource::new(r())),
        Box::new(xbox::XboxLibrarySource::new(Box::new(
            xbox::windows::WindowsXboxSystem::new(r()),
        ))),
        Box::new(ubisoft::UbisoftLibrarySource::new(r())),
        Box::new(ea::EaLibrarySource::new(r())),
        Box::new(battlenet::BattleNetLibrarySource::new(
            r(),
            &program_data,
            Some(env("ProgramFiles(x86)")),
        )),
        Box::new(amazon::AmazonLibrarySource::new(r(), &local_app_data)),
    ]
}

// Linux e macOS (MIG-026/027): parsers puros, só a descoberta de pastas tem cfg.
pub mod desktop_entries;
pub mod heroic;
pub mod lutris;
pub mod mac_apps;

/// Resultado da varredura de uma fonte.
#[derive(Debug, Clone, Default)]
pub struct LibraryScan {
    pub games: Vec<Game>,
    /// O cliente da loja está instalado (mesmo sem jogos).
    pub client_installed: bool,
    /// Aviso para a UI (ex.: cliente ausente), já traduzido.
    pub notice: Option<String>,
}

impl LibraryScan {
    pub fn missing(notice: impl Into<String>) -> Self {
        LibraryScan {
            games: Vec::new(),
            client_installed: false,
            notice: Some(notice.into()),
        }
    }
}

pub trait LibrarySource: Send + Sync {
    fn store(&self) -> GameStore;
    fn scan(&self) -> LibraryScan;
}

/// Lê no máximo `limit` bytes de um arquivo (arquivos de loja são dados não confiáveis).
pub fn read_limited(path: &std::path::Path, limit: u64) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut buf = Vec::new();
    std::fs::File::open(path)?
        .take(limit)
        .read_to_end(&mut buf)?;
    Ok(buf)
}

/// Limite padrão para manifestos e arquivos de configuração das lojas.
pub const MAX_STORE_FILE: u64 = 16 * 1024 * 1024;

/// Fontes de Linux e macOS que não são a Steam: Heroic nos dois, Lutris só no Linux.
#[cfg(unix)]
pub fn unix_sources() -> Vec<Box<dyn LibrarySource>> {
    let mut out: Vec<Box<dyn LibrarySource>> = vec![Box::new(heroic::HeroicSource::detect())];
    if cfg!(target_os = "linux") {
        out.push(Box::new(lutris::LutrisSource::detect()));
    }
    out
}

/// `file:///…` de um caminho absoluto, com escape do que quebraria a URI (espaço, `#`, `?`, `%`).
pub fn file_uri(path: &std::path::Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let mut out = String::from(if text.starts_with('/') {
        "file://"
    } else {
        "file:///"
    });
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || b"/-._~:".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Texto de um arquivo de loja: UTF-8 sem o BOM (como o `File.ReadAllText` do C#), bytes inválidos trocados.
pub fn text_of(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    String::from_utf8_lossy(bytes).into_owned()
}
