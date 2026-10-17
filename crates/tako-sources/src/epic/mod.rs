//! Epic Games (port de `Library/Epic`): manifests `*.item` em
//! `%ProgramData%\Epic\EpicGamesLauncher\Data\Manifests` e a lista `LauncherInstalled.dat`, lidos sem
//! a Epic aberta. Abre sempre pelo protocolo oficial (login e DRM ficam com a Epic).

pub mod owned;

use crate::fs::StoreFs;
use crate::registry::RegistryReader;
use crate::{LibraryScan, LibrarySource};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tako_core::i18n::t;
use tako_core::library::{Game, GameStore};

/// Onde a Epic guarda os dados (inclusive a pasta de manifests), quando instalada.
pub const LAUNCHER_KEY: &str =
    r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Epic Games\EpicGamesLauncher";
/// Protocolo `com.epicgames.launcher://`: sem ele, nada abre os jogos.
pub const PROTOCOL_KEY: &str = r"HKEY_CLASSES_ROOT\com.epicgames.launcher\shell\open\command";
const ACCENT: &str = "#2A2A2A";

/// Jogo instalado pela Epic Games Launcher (um arquivo `.item`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpicInstall {
    pub app_name: String,
    pub display_name: String,
    pub install_location: String,
    pub launch_executable: Option<String>,
    pub catalog_namespace: String,
    pub catalog_item_id: String,
    pub install_size: u64,
}

/// Um manifest `.item`; `Ok(None)` quando não é jogo jogável (instalação incompleta, DLC, plugin ou
/// engine, campos faltando). JSON inválido é `Err`.
pub fn parse_manifest(json: &str) -> Result<Option<EpicInstall>, serde_json::Error> {
    let root: Value = serde_json::from_str(json)?;
    if !root.is_object() || root.get("bIsIncompleteInstall") == Some(&Value::Bool(true)) {
        return Ok(None);
    }
    let (Some(app), Some(display), Some(location), Some(ns), Some(item)) = (
        text(&root, "AppName"),
        text(&root, "DisplayName"),
        text(&root, "InstallLocation"),
        text(&root, "CatalogNamespace"),
        text(&root, "CatalogItemId"),
    ) else {
        return Ok(None);
    };
    // DLC: aponta para o jogo principal, que tem o próprio manifest.
    if text(&root, "MainGameAppName").is_some_and(|m| !m.eq_ignore_ascii_case(app)) {
        return Ok(None);
    }
    // Plugins do Unreal Engine e o próprio engine aparecem sem a categoria "games".
    if let Some(Value::Array(cats)) = root.get("AppCategories")
        && !cats
            .iter()
            .any(|c| c.as_str().is_some_and(|c| c.eq_ignore_ascii_case("games")))
    {
        return Ok(None);
    }
    Ok(Some(EpicInstall {
        app_name: app.to_owned(),
        display_name: display.trim().to_owned(),
        install_location: location.to_owned(),
        launch_executable: text(&root, "LaunchExecutable").map(str::to_owned),
        catalog_namespace: ns.to_owned(),
        catalog_item_id: item.to_owned(),
        install_size: root.get("InstallSize").and_then(Value::as_u64).unwrap_or(0),
    }))
}

/// `LauncherInstalled.dat`: pasta de instalação por AppName (a Epic atualiza ao mover um jogo).
/// Chaves em minúsculas (o C# compara sem diferenciar maiúsculas).
pub fn parse_installed_list(json: &str) -> Result<HashMap<String, String>, serde_json::Error> {
    let root: Value = serde_json::from_str(json)?;
    let mut out = HashMap::new();
    if let Some(Value::Array(list)) = root.get("InstallationList") {
        for e in list {
            if let (Some(app), Some(loc)) = (text(e, "AppName"), text(e, "InstallLocation")) {
                out.insert(app.to_lowercase(), loc.to_owned());
            }
        }
    }
    Ok(out)
}

/// `com.epicgames.launcher://apps/<Namespace>%3A<CatalogItemId>%3A<AppName>?action=launch&silent=true`.
pub fn launch_uri(i: &EpicInstall) -> String {
    format!(
        "com.epicgames.launcher://apps/{}%3A{}%3A{}?action=launch&silent=true",
        escape_data(&i.catalog_namespace),
        escape_data(&i.catalog_item_id),
        escape_data(&i.app_name)
    )
}

/// Como o `Uri.EscapeDataString` do .NET: só letras, números e `-_.~` ficam.
pub(crate) fn escape_data(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn text<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key)?.as_str().filter(|s| !s.trim().is_empty())
}

pub fn id_for(i: &EpicInstall) -> String {
    format!("epic:{}", i.app_name)
}

pub fn to_game(i: &EpicInstall, added_at: Option<DateTime<Utc>>) -> Game {
    Game {
        id: id_for(i),
        title: i.display_name.clone(),
        store: Some(GameStore::Epic),
        size_label: crate::steam::size_label(i.install_size),
        size_bytes: (i.install_size > 0).then_some(i.install_size),
        accent: ACCENT.to_owned(),
        is_installed: true,
        added_at,
        launch_target: Some(launch_uri(i)),
        install_path: Some(i.install_location.clone()),
        store_product_id: Some(i.catalog_item_id.clone()),
        ..Default::default()
    }
}

pub struct EpicLibrarySource {
    registry: Arc<dyn RegistryReader>,
    program_data: PathBuf,
    /// Só para as pastas dos jogos; os manifests são lidos do disco de verdade.
    pub fs: StoreFs,
}

impl EpicLibrarySource {
    pub fn new(registry: Arc<dyn RegistryReader>, program_data: impl AsRef<Path>) -> Self {
        EpicLibrarySource {
            registry,
            program_data: program_data.as_ref().to_path_buf(),
            fs: StoreFs::real(),
        }
    }

    /// `Data` da Epic: manifests dos instalados e o cache do catálogo.
    fn data_folder(&self) -> PathBuf {
        match self.registry.get_string(LAUNCHER_KEY, "AppDataPath") {
            Some(d) if !d.trim().is_empty() => PathBuf::from(d),
            _ => self
                .program_data
                .join("Epic")
                .join("EpicGamesLauncher")
                .join("Data"),
        }
    }

    fn installed_list(&self) -> HashMap<String, String> {
        let file = self
            .program_data
            .join("Epic")
            .join("UnrealEngineLauncher")
            .join("LauncherInstalled.dat");
        crate::read_limited(&file, crate::MAX_STORE_FILE)
            .ok()
            .and_then(|b| parse_installed_list(&crate::text_of(&b)).ok())
            .unwrap_or_default()
    }
}

impl LibrarySource for EpicLibrarySource {
    fn store(&self) -> GameStore {
        GameStore::Epic
    }

    fn scan(&self) -> LibraryScan {
        let client = self.registry.get_string(PROTOCOL_KEY, "").is_some();
        let data = self.data_folder();
        let owned = owned::load(&data.join("Catalog").join("catcache.bin"));
        let dir = std::fs::read_dir(data.join("Manifests"));
        if dir.is_err() && owned.is_empty() {
            return if client {
                LibraryScan {
                    client_installed: true,
                    ..Default::default()
                }
            } else {
                LibraryScan::missing(t("sources.epic.missing"))
            };
        }
        let mut files: Vec<PathBuf> = dir
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("item"))
            })
            .collect();
        files.sort_by_key(|p| p.to_string_lossy().to_lowercase());
        let locations = self.installed_list();
        let mut games: Vec<Game> = Vec::new();
        for file in files {
            let Ok(bytes) = crate::read_limited(&file, crate::MAX_STORE_FILE) else {
                continue;
            };
            // Manifest corrompido ou sendo gravado: os outros seguem.
            let Ok(Some(mut install)) = parse_manifest(&crate::text_of(&bytes)) else {
                continue;
            };
            if !(self.fs.exists)(&install.install_location)
                && let Some(moved) = locations.get(&install.app_name.to_lowercase())
            {
                install.install_location = moved.clone();
            }
            if (self.fs.exists)(&install.install_location)
                && !games.iter().any(|g| g.id == id_for(&install))
            {
                games.push(to_game(&install, crate::install_dates::of(&file)));
            }
        }
        // Os da conta que faltam instalar (SRC-005).
        let missing = owned::not_installed(&owned, &games);
        games.extend(missing);
        crate::text::sort_by_title(&mut games);
        LibraryScan {
            games,
            client_installed: client,
            notice: (!client).then(|| t("sources.epic.no_client")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::fake;
    use crate::registry::MemoryRegistry;

    const FAROL: &str = r"C:\Program Files\Epic Games\Farol";
    const MOVED_CORRIDA: &str = r"E:\Jogos\CorridaNoturna";

    fn program_data() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/Stores/ProgramData")
    }

    fn manifest(name: &str) -> String {
        std::fs::read_to_string(
            program_data()
                .join("Epic/EpicGamesLauncher/Data/Manifests")
                .join(name),
        )
        .unwrap()
    }

    fn source(
        registry: MemoryRegistry,
        program_data: PathBuf,
        folders: &[&str],
    ) -> EpicLibrarySource {
        let mut s = EpicLibrarySource::new(Arc::new(registry), program_data);
        s.fs = fake::folders(folders);
        s
    }

    #[test]
    fn manifest_is_read_with_launch_identity() {
        let i = parse_manifest(&manifest("0A1B2C3D.item")).unwrap().unwrap();
        assert_eq!(i.app_name, "Farol");
        assert_eq!(i.display_name, "Farol do Fim do Mundo");
        assert_eq!(i.install_location, FAROL);
        assert_eq!(
            i.launch_executable.as_deref(),
            Some("Binaries/Win64/Farol.exe")
        );
        assert_eq!(i.catalog_namespace, "farolns");
        assert_eq!(i.catalog_item_id, "a1b2c3d4e5f60718293a4b5c6d7e8f90");
        assert_eq!(i.install_size, 16106127360);
    }

    #[test]
    fn manifests_that_are_not_playable_games_are_skipped() {
        for f in ["1B2C3D4E.item", "2C3D4E5F.item", "3D4E5F60.item"] {
            assert_eq!(parse_manifest(&manifest(f)).unwrap(), None, "{f}");
        }
    }

    #[test]
    fn invalid_manifest_is_a_json_error() {
        assert!(parse_manifest(&manifest("5F607182.item")).is_err());
    }

    #[test]
    fn installed_list_maps_app_to_current_folder() {
        let text = std::fs::read_to_string(
            program_data().join("Epic/UnrealEngineLauncher/LauncherInstalled.dat"),
        )
        .unwrap();
        let list = parse_installed_list(&text).unwrap();
        assert_eq!(list["farol"], FAROL);
        assert_eq!(list["corrida noturna"], MOVED_CORRIDA);
    }

    #[test]
    fn games_open_through_the_official_protocol_never_the_exe() {
        let i = EpicInstall {
            app_name: "Corrida Noturna".into(),
            display_name: "Corrida".into(),
            install_location: r"C:\X".into(),
            launch_executable: Some("Corrida.exe".into()),
            catalog_namespace: "corrida".into(),
            catalog_item_id: "c0ffee".into(),
            install_size: 0,
        };
        assert_eq!(
            launch_uri(&i),
            "com.epicgames.launcher://apps/corrida%3Ac0ffee%3ACorrida%20Noturna?action=launch&silent=true"
        );
    }

    #[test]
    fn source_lists_installed_games_and_follows_moved_folders() {
        let reg = MemoryRegistry::default().with(
            PROTOCOL_KEY,
            "",
            "\"C:\\Epic\\EpicGamesLauncher.exe\" %1",
        );
        let scan = source(reg, program_data(), &[FAROL, MOVED_CORRIDA]).scan();

        assert!(scan.client_installed);
        assert_eq!(scan.notice, None);
        let titles: Vec<_> = scan.games.iter().map(|g| g.title.as_str()).collect();
        assert_eq!(titles, ["Corrida Noturna™", "Farol do Fim do Mundo"]);
        let corrida = &scan.games[0];
        assert_eq!(corrida.id, "epic:Corrida Noturna");
        assert_eq!(corrida.store, Some(GameStore::Epic));
        assert_eq!(corrida.install_path.as_deref(), Some(MOVED_CORRIDA));
        assert!(
            corrida
                .launch_target
                .as_deref()
                .unwrap()
                .starts_with("com.epicgames.launcher://apps/corrida%3Ac0ffee%3A")
        );
        assert_eq!(corrida.size_label.as_deref(), Some("700 MB"));
        assert!(corrida.can_play());
        assert_eq!(corrida.cover_uri, None);
    }

    #[test]
    fn source_warns_when_the_launcher_is_missing() {
        let with_games = source(MemoryRegistry::default(), program_data(), &[FAROL]).scan();
        assert!(!with_games.client_installed);
        assert_eq!(with_games.games.len(), 1);
        assert!(
            with_games
                .notice
                .unwrap()
                .contains("Epic Games Launcher não encontrado")
        );

        let nothing = source(
            MemoryRegistry::default(),
            program_data().join("nao-existe"),
            &[],
        )
        .scan();
        assert!(nothing.games.is_empty());
        assert_eq!(
            nothing.notice.as_deref(),
            Some("Epic Games Launcher não encontrado neste PC")
        );
    }

    #[test]
    fn source_adds_owned_games_that_are_not_installed() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("Epic/EpicGamesLauncher/Data");
        std::fs::create_dir_all(data.join("Manifests")).unwrap();
        std::fs::create_dir_all(data.join("Catalog")).unwrap();
        std::fs::write(
            data.join("Manifests/0A1B2C3D.item"),
            manifest("0A1B2C3D.item"),
        )
        .unwrap();
        std::fs::write(
            data.join("Catalog/catcache.bin"),
            owned::tests::catcache(owned::tests::CATALOG),
        )
        .unwrap();
        let reg = MemoryRegistry::default().with(PROTOCOL_KEY, "", "epic");
        let scan = source(reg.clone(), dir.path().to_path_buf(), &[FAROL]).scan();
        let games: Vec<_> = scan
            .games
            .iter()
            .map(|g| (g.id.as_str(), g.is_installed))
            .collect();
        assert_eq!(games, [("epic:Farol", true), ("epic:MareAlta", false)]);
        assert!(scan.games[1].install_uri.is_some());

        // Sem nenhum instalado (nem a pasta de manifests), a conta aparece do mesmo jeito.
        std::fs::remove_dir_all(data.join("Manifests")).unwrap();
        let scan = source(reg.clone(), dir.path().to_path_buf(), &[]).scan();
        assert_eq!(scan.games.len(), 2);
        assert!(scan.games.iter().all(|g| !g.is_installed));

        // Cache quebrado: só fica o que está instalado.
        std::fs::write(data.join("Catalog/catcache.bin"), "{ quebrado").unwrap();
        assert!(
            source(reg, dir.path().to_path_buf(), &[])
                .scan()
                .games
                .is_empty()
        );
    }
}
