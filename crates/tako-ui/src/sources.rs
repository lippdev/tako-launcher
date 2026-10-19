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

/// Lê as lojas ligadas (bloqueante: rodar fora da thread da UI). `exe_icons`: onde ficam os ícones extraídos dos
/// executáveis (UX-005).
pub fn scan(
    disabled: &str,
    steam_owned: Option<std::path::PathBuf>,
    emulators: Config,
    exe_icons: &std::path::Path,
    progress: impl FnMut(usize, usize),
) -> (Vec<Game>, Vec<LibrarySourceReport>) {
    let mut sources = LibrarySources::new(all_sources(steam_owned, emulators));
    let mut games = sources.discover_with(|s| !is_disabled(disabled, s), progress);
    games
        .iter_mut()
        .for_each(|g| fill_exe_icon(g, exe_icons, tako_platform::exe_icon::icon_file));
    let reports = sources.reports().values().cloned().collect();
    (games, reports)
}

/// Ícone do `.exe` do jogo, o último recurso (UX-005): só para jogo instalado que abre por um executável e que
/// nenhuma loja deu ícone (a extração só existe no Windows; nos outros SOs não acha nada). ROMs ficam de fora: o executável é o do emulador, igual para todas.
pub fn fill_exe_icon(
    game: &mut Game,
    dir: &std::path::Path,
    extract: impl Fn(&std::path::Path, &std::path::Path) -> Option<std::path::PathBuf>,
) {
    use tako_core::library::media::ICON;
    let exe = game
        .launch_target
        .as_deref()
        .filter(|t| t.to_ascii_lowercase().ends_with(".exe"));
    let Some(exe) = exe.filter(|_| {
        game.is_installed && game.store() != GameStore::Emulators && !game.media.contains_key(ICON)
    }) else {
        return;
    };
    if let Some(file) = extract(std::path::Path::new(exe), dir) {
        game.media
            .insert(ICON.to_owned(), tako_sources::file_uri(&file));
        game.icon_origin = Some(tako_core::library::IconOrigin::Exe);
    }
}

/// Lojas que este SO lê (Configurações › Lojas).
pub fn stores() -> Vec<GameStore> {
    LibrarySources::new(all_sources(None, Config::default())).stores()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exe_icon_only_fills_installed_games_without_a_store_icon() {
        use std::path::PathBuf;
        let dir = std::path::Path::new("cache");
        let fake = |_: &std::path::Path, _: &std::path::Path| Some(PathBuf::from("/c/icone.png"));
        let game = |store, target: &str| Game {
            store: Some(store),
            is_installed: true,
            launch_target: Some(target.to_owned()),
            ..Default::default()
        };
        let icon = |g: &Game| g.media.get("icon").cloned();
        let mut manual = game(GameStore::Manual, r"C:\Jogos\Farol.EXE");
        fill_exe_icon(&mut manual, dir, fake);
        assert_eq!(icon(&manual).as_deref(), Some("file:///c/icone.png"));
        assert_eq!(
            manual.icon_origin,
            Some(tako_core::library::IconOrigin::Exe)
        );
        let mut store_icon = game(GameStore::Gog, r"C:\Jogos\x.exe");
        store_icon
            .media
            .insert("icon".into(), "https://loja/i.png".into());
        let mut rom = game(GameStore::Emulators, r"C:\Emu\emu.exe");
        let mut uri = game(GameStore::Steam, "steam://rungameid/570");
        let mut not_installed = game(GameStore::Manual, r"C:\Jogos\y.exe");
        not_installed.is_installed = false;
        for g in [&mut store_icon, &mut rom, &mut uri, &mut not_installed] {
            fill_exe_icon(g, dir, fake);
        }
        assert_eq!(icon(&store_icon).as_deref(), Some("https://loja/i.png"));
        assert_eq!(
            (icon(&rom), icon(&uri), icon(&not_installed)),
            (None, None, None)
        );
    }

    #[test]
    fn disabled_list() {
        assert!(is_disabled("Epic, Gog", GameStore::Gog));
        assert!(!is_disabled("Epic", GameStore::Steam));
        assert!(!is_disabled("", GameStore::Steam));
    }
}
