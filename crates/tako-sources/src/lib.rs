//! Fontes de biblioteca (port de `Launcher.Core/Library/{Sources,Steam,Epic,...}`).
//!
//! Cada fonte implementa [`LibrarySource`]. Os parsers de arquivo (VDF, manifestos, bancos) são
//! funções puras, testadas com fixtures fictícias em todos os SOs; só a descoberta de pastas e o
//! acesso ao registro dependem do SO. Arquivos das lojas são dados não confiáveis (PRD §8.3):
//! nada de `unwrap`, limites de tamanho e falha de uma fonte não derruba as outras.

use tako_core::library::{Game, GameStore};

pub mod registry;

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
