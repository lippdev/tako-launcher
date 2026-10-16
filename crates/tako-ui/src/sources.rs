//! Lojas de cada SO lidas para a biblioteca (PRD §6: as do Windows, Steam nos três, Heroic e Lutris no Linux).

use tako_core::library::{Game, GameStore};
use tako_sources::LibrarySource;
use tako_sources::report::{LibrarySourceReport, LibrarySources};

pub fn all_sources() -> Vec<Box<dyn LibrarySource>> {
    let mut v: Vec<Box<dyn LibrarySource>> = vec![tako_sources::steam_source()];
    #[cfg(windows)]
    v.extend(tako_sources::windows_sources(std::sync::Arc::new(
        tako_sources::registry_windows::WindowsRegistry,
    )));
    #[cfg(unix)]
    v.extend(tako_sources::unix_sources());
    v
}

/// `DisabledStores` = nomes separados por vírgula (como nas alphas).
pub fn is_disabled(disabled: &str, store: GameStore) -> bool {
    disabled
        .split(',')
        .any(|s| s.trim().eq_ignore_ascii_case(store.key()))
}

/// Lê as lojas ligadas (bloqueante: rodar fora da thread da UI).
pub fn scan(disabled: &str) -> (Vec<Game>, Vec<LibrarySourceReport>) {
    let mut sources = LibrarySources::new(all_sources());
    let games = sources.discover(|s| !is_disabled(disabled, s));
    let reports = sources.reports().values().cloned().collect();
    (games, reports)
}

/// Lojas que este SO lê (Configurações › Lojas).
pub fn stores() -> Vec<GameStore> {
    LibrarySources::new(all_sources()).stores()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_list() {
        assert!(is_disabled("Epic, Gog", GameStore::Gog));
        assert!(!is_disabled("Epic", GameStore::Steam));
        assert!(!is_disabled("", GameStore::Steam));
    }
}
