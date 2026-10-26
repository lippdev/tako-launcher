//! Ubisoft Connect (port de `UbisoftLibrarySource`): jogos lidos do registro (`InstallDir` por id),
//! nome pela chave de desinstalação `Uplay Install <id>` ou pela pasta. Abre sempre pelo protocolo
//! `uplay://` (login, DRM e atualização ficam com a Ubisoft Connect).

use crate::fs::StoreFs;
use crate::registry::RegistryReader;
use crate::windows_path;
use crate::{LibraryScan, LibrarySource};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tako_core::i18n::t;
use tako_core::library::{Game, GameStore};

/// Jogos instalados (o cliente é 32 bits; a chave sem WOW6432Node fica como plano B).
pub const INSTALLS_KEYS: [&str; 2] = [
    r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Ubisoft\Launcher\Installs",
    r"HKEY_LOCAL_MACHINE\SOFTWARE\Ubisoft\Launcher\Installs",
];
pub const LAUNCHER_KEY: &str = r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Ubisoft\Launcher";
pub const PROTOCOL_KEY: &str = r"HKEY_CLASSES_ROOT\uplay\shell\open\command";
const ACCENT: &str = "#2E3192";

pub fn id_for(install_id: &str) -> String {
    format!("ubisoft:{install_id}")
}

/// `uplay://launch/<id>/0` (0 = a primeira opção de jogar).
pub fn launch_uri(install_id: &str) -> String {
    format!("uplay://launch/{install_id}/0")
}

pub fn to_game(
    install_id: &str,
    title: &str,
    folder: &str,
    added_at: Option<DateTime<Utc>>,
) -> Game {
    Game {
        id: id_for(install_id),
        title: title.trim().to_owned(),
        store: Some(GameStore::Ubisoft),
        accent: ACCENT.to_owned(),
        is_installed: true,
        added_at,
        launch_target: Some(launch_uri(install_id)),
        install_path: Some(folder.to_owned()),
        store_product_id: Some(install_id.to_owned()),
        ..Default::default()
    }
}

pub struct UbisoftLibrarySource {
    registry: Arc<dyn RegistryReader>,
    pub fs: StoreFs,
}

impl UbisoftLibrarySource {
    pub fn new(registry: Arc<dyn RegistryReader>) -> Self {
        UbisoftLibrarySource {
            registry,
            fs: StoreFs::real(),
        }
    }

    /// Protocolo registrado ou executável na pasta do cliente.
    pub fn client_installed(&self) -> bool {
        if self.registry.get_string(PROTOCOL_KEY, "").is_some() {
            return true;
        }
        self.registry
            .get_string(LAUNCHER_KEY, "InstallDir")
            .filter(|f| !f.trim().is_empty())
            .is_some_and(|f| {
                ["UbisoftConnect.exe", "upc.exe"]
                    .iter()
                    .any(|exe| (self.fs.exists)(&windows_path::combine(&f, &[exe])))
            })
    }

    /// Nome do programa instalado ("Uplay Install 635" → "Far Cry 5"); sem ele, o nome da pasta.
    pub fn title_for(&self, id: &str, folder: &str) -> String {
        crate::uninstall::find(self.registry.as_ref(), &format!("Uplay Install {id}"))
            .and_then(|e| e.display_name)
            .unwrap_or_else(|| windows_path::last_name(folder))
    }
}

impl LibrarySource for UbisoftLibrarySource {
    fn store(&self) -> GameStore {
        GameStore::Ubisoft
    }

    fn scan(&self) -> LibraryScan {
        let client = self.client_installed();
        let mut games: Vec<Game> = Vec::new();
        for root in INSTALLS_KEYS {
            for id in self.registry.sub_keys(root) {
                let Some(folder) = self
                    .registry
                    .get_string(&format!(r"{root}\{id}"), "InstallDir")
                    .filter(|f| !f.trim().is_empty())
                else {
                    continue;
                };
                if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
                    continue;
                }
                let path = windows_path::normalize(&folder);
                // Desinstalado sem limpar o registro, ou já lido na outra visão.
                if !(self.fs.exists)(&path) || games.iter().any(|g| g.id == id_for(&id)) {
                    continue;
                }
                games.push(to_game(
                    &id,
                    &self.title_for(&id, &path),
                    &path,
                    crate::install_dates::of(&path),
                ));
            }
        }
        if games.is_empty() && !client {
            return LibraryScan::missing(t("sources.ubisoft.missing"));
        }
        crate::text::sort_by_title(&mut games);
        LibraryScan {
            games,
            client_installed: client,
            notice: (!client).then(|| t("sources.ubisoft.no_client")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::fake;
    use crate::registry::MemoryRegistry;

    const UNINSTALL: &str =
        r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";

    fn source(reg: MemoryRegistry, fs: StoreFs) -> UbisoftLibrarySource {
        let mut s = UbisoftLibrarySource::new(Arc::new(reg));
        s.fs = fs;
        s
    }

    #[test]
    fn reads_installs_with_uninstall_name_or_folder_and_launches_by_protocol() {
        let k = |id: &str| format!(r"{}\{id}", INSTALLS_KEYS[0]);
        let reg = MemoryRegistry::default()
            .with(&k("635"), "InstallDir", "D:/Ubisoft/Far Horizon 5/")
            .with(&k("4311"), "InstallDir", r"D:\Ubisoft\Sem Nome")
            .with(&k("999"), "InstallDir", r"D:\Ubisoft\Sumiu")
            .with(&k("launcher"), "InstallDir", r"D:\Ubisoft\Nao e jogo")
            .with(
                &format!(r"{UNINSTALL}\Uplay Install 635"),
                "DisplayName",
                "Far Horizon 5",
            )
            .with(PROTOCOL_KEY, "", "\"C:\\Ubisoft\\upc.exe\" \"%1\"");
        let fs = fake::folders(&[
            r"D:\Ubisoft\Far Horizon 5",
            r"D:\Ubisoft\Sem Nome",
            r"D:\Ubisoft\Nao e jogo",
        ]);

        let scan = source(reg, fs).scan();

        assert!(scan.client_installed);
        assert_eq!(scan.notice, None);
        let titles: Vec<_> = scan.games.iter().map(|g| g.title.as_str()).collect();
        assert_eq!(titles, ["Far Horizon 5", "Sem Nome"]);
        let g = &scan.games[0];
        assert_eq!(g.id, "ubisoft:635");
        assert_eq!(g.launch_target.as_deref(), Some("uplay://launch/635/0"));
        assert_eq!(g.install_path.as_deref(), Some(r"D:\Ubisoft\Far Horizon 5"));
        assert_eq!(g.store_product_id.as_deref(), Some("635"));
    }

    #[test]
    fn without_client_keeps_games_and_warns() {
        let reg = MemoryRegistry::default().with(
            &format!(r"{}\5", INSTALLS_KEYS[1]),
            "InstallDir",
            r"E:\Jogo",
        );
        let scan = source(reg, fake::folders(&[r"E:\Jogo"])).scan();
        assert_eq!(scan.games.len(), 1);
        assert!(!scan.client_installed);
        assert_eq!(
            scan.notice.as_deref(),
            Some("Ubisoft Connect não encontrada: instale-a para abrir estes jogos")
        );
        let empty = source(MemoryRegistry::default(), fake::folders(&[])).scan();
        assert_eq!(
            empty.notice.as_deref(),
            Some("Ubisoft Connect não encontrada neste PC")
        );
    }

    #[test]
    fn client_found_by_executable_in_launcher_folder() {
        let reg = MemoryRegistry::default().with(
            LAUNCHER_KEY,
            "InstallDir",
            r"C:\Program Files (x86)\Ubisoft\Ubisoft Game Launcher\",
        );
        let s = source(
            reg,
            fake::folders(&[
                r"C:\Program Files (x86)\Ubisoft\Ubisoft Game Launcher\UbisoftConnect.exe",
            ]),
        );
        assert!(s.client_installed());
        assert!(s.scan().client_installed);
    }
}
