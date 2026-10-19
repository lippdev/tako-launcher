//! Lojas de cada SO lidas para a biblioteca (PRD §6: as do Windows, Steam nos três, Heroic e Lutris no Linux,
//! emuladores nos três).

use tako_core::library::{Game, GameStore};
use tako_sources::LibrarySource;
use tako_sources::emulators::{Config, EmulatorSource};
use tako_sources::report::{LibrarySourceReport, LibrarySources};

pub fn all_sources(
    steam_owned: Option<std::path::PathBuf>,
    emulators: Config,
) -> Vec<Box<dyn LibrarySource>> {
    let mut v: Vec<Box<dyn LibrarySource>> = vec![tako_sources::steam_source(steam_owned)];
    #[cfg(windows)]
    v.extend(tako_sources::windows_sources(
        std::sync::Arc::new(tako_sources::registry_windows::WindowsRegistry),
        tako_platform::windows::dpapi::unprotect,
    ));
    #[cfg(unix)]
    v.extend(tako_sources::unix_sources());
    // Emuladores e ROMs (SRC-001), nos três SOs, antes dos adicionados à mão.
    v.push(Box::new(EmulatorSource::new(emulators)));
    v
}

/// Configuração dos emuladores gravada em Configurações › Biblioteca › Emuladores e ROMs.
pub fn emulator_config(settings: &crate::shell::Settings) -> Config {
    use tako_sources::emulators::keys;
    Config::parse(
        settings.get(keys::ROM_FOLDERS),
        settings.get(keys::PATHS),
        settings.get(keys::BY_SYSTEM),
    )
}

/// `DisabledStores` = nomes separados por vírgula (como nas alphas).
pub fn is_disabled(disabled: &str, store: GameStore) -> bool {
    disabled
        .split(',')
        .any(|s| s.trim().eq_ignore_ascii_case(store.key()))
}

/// Lê as lojas ligadas (bloqueante: rodar fora da thread da UI).
pub fn scan(
    disabled: &str,
    steam_owned: Option<std::path::PathBuf>,
    emulators: Config,
    progress: impl FnMut(usize, usize),
) -> (Vec<Game>, Vec<LibrarySourceReport>) {
    let mut sources = LibrarySources::new(all_sources(steam_owned, emulators));
    let games = sources.discover_with(|s| !is_disabled(disabled, s), progress);
    let reports = sources.reports().values().cloned().collect();
    (games, reports)
}

/// Lojas que este SO lê (Configurações › Lojas).
pub fn stores() -> Vec<GameStore> {
    LibrarySources::new(all_sources(None, Config::default())).stores()
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
