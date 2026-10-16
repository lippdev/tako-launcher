//! Steam nos três SOs (port de `Library/Steam`): jogos instalados lidos dos arquivos da Steam, sem
//! API nem Steam aberta. Bibliotecas em `steamapps/libraryfolders.vdf`, jogos em `appmanifest_*.acf`
//! e tempo jogado / última vez em `userdata/<conta>/config/localconfig.vdf` (da conta usada por último).

pub mod vdf;

use crate::{LibraryScan, LibrarySource};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tako_core::i18n::{Lang, lang, t};
use tako_core::library::{Game, GameStore};

/// Esquema das artes resolvidas pelo launcher (API da Steam, CDN e cache local da Steam).
pub const ART_SCHEME: &str = "steam-art";

const FULLY_INSTALLED: u32 = 4;

/// Ferramentas que a Steam instala como "apps" e que não são jogos (Proton, runtimes do Linux, redistribuíveis).
const TOOL_APP_IDS: [u32; 7] = [228980, 1070560, 1391110, 1628350, 1493710, 2180100, 250820];
const TOOL_PREFIXES: [&str; 4] = [
    "Steamworks Common",
    "Proton",
    "Steam Linux Runtime",
    "SteamVR",
];

/// Cores de destaque enquanto não há cor extraída da arte; escolhidas pelo appid para variar entre jogos.
const ACCENTS: [&str; 8] = [
    "#2B3A55", "#4A2B55", "#552B3A", "#2B5548", "#55432B", "#2B4655", "#3F2B55", "#55302B",
];

/// Jogo instalado numa biblioteca da Steam.
#[derive(Debug, Clone, PartialEq)]
pub struct SteamApp {
    pub app_id: u32,
    pub name: String,
    pub install_path: PathBuf,
    pub size_on_disk: u64,
    pub installed_at: Option<DateTime<Utc>>,
    pub last_played: Option<DateTime<Utc>>,
    pub play_time: Duration,
}

/// Jogos instalados em todas as bibliotecas de `steam_root`, por nome.
pub fn scan(steam_root: &Path) -> Vec<SteamApp> {
    let usage = read_usage(steam_root);
    let mut apps: Vec<SteamApp> = Vec::new();
    for library in library_folders(steam_root) {
        let steamapps = library.join("steamapps");
        let Ok(entries) = std::fs::read_dir(&steamapps) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !(name.starts_with("appmanifest_") && name.ends_with(".acf")) {
                continue;
            }
            if let Some(app) = read_manifest(&entry.path(), &steamapps, &usage)
                && !apps.iter().any(|a| a.app_id == app.app_id)
            {
                apps.push(app);
            }
        }
    }
    apps.sort_by_cached_key(|a| crate::text::fold(&a.name));
    apps
}

/// Pastas de biblioteca: a da própria Steam e as de `libraryfolders.vdf` (formatos novo e antigo).
pub fn library_folders(steam_root: &Path) -> Vec<PathBuf> {
    let mut folders = vec![steam_root.to_path_buf()];
    let Ok(root) = vdf::load(&steam_root.join("steamapps").join("libraryfolders.vdf")) else {
        return folders;
    };
    let Some(list) = root
        .node("libraryfolders")
        .or_else(|| root.node("LibraryFolders"))
    else {
        return folders;
    };
    for (key, value) in &list.children {
        if !is_digits(key) {
            continue; // ex.: "TimeNextStatsReport", "contentstatsid"
        }
        let path = match value {
            vdf::VdfValue::Node(n) => n.value("path"),
            vdf::VdfValue::Text(s) => Some(s.as_str()),
        };
        if let Some(p) = path.map(str::trim).filter(|p| !p.is_empty()) {
            let p = PathBuf::from(p);
            if !folders.iter().any(|f| same_path(f, &p)) {
                folders.push(p);
            }
        }
    }
    folders
}

fn read_manifest(
    file: &Path,
    steamapps: &Path,
    usage: &HashMap<u32, (Option<DateTime<Utc>>, Duration)>,
) -> Option<SteamApp> {
    let root = vdf::load(file).ok()?;
    let state = root.node("AppState")?;
    let app_id = u32::try_from(number(state.value("appid"))?).ok()?;
    let name = state.value("name").filter(|s| !s.is_empty())?;
    let install_dir = state.value("installdir").filter(|s| !s.is_empty())?;
    let flags = number(state.value("StateFlags")).unwrap_or(0);
    if flags & u64::from(FULLY_INSTALLED) == 0
        || TOOL_APP_IDS.contains(&app_id)
        || TOOL_PREFIXES.iter().any(|p| {
            name.get(..p.len())
                .is_some_and(|h| h.eq_ignore_ascii_case(p))
        })
    {
        return None;
    }
    let (last_played, play_time) = usage.get(&app_id).cloned().unwrap_or_default();
    Some(SteamApp {
        app_id,
        name: name.to_owned(),
        install_path: steamapps.join("common").join(install_dir),
        size_on_disk: number(state.value("SizeOnDisk")).unwrap_or(0),
        installed_at: crate::install_dates::of(file),
        last_played: last_played.or_else(|| unix_time(state.value("LastPlayed"))),
        play_time,
    })
}

/// Tempo jogado e última vez por jogo, da conta cujo `localconfig.vdf` mudou por último.
fn read_usage(steam_root: &Path) -> HashMap<u32, (Option<DateTime<Utc>>, Duration)> {
    let mut usage = HashMap::new();
    let Ok(users) = std::fs::read_dir(steam_root.join("userdata")) else {
        return usage;
    };
    let config = users
        .flatten()
        .map(|d| d.path().join("config").join("localconfig.vdf"))
        .filter_map(|p| {
            let modified = std::fs::metadata(&p).and_then(|m| m.modified()).ok()?;
            Some((modified, p))
        })
        .max_by_key(|(m, _): &(SystemTime, PathBuf)| *m);
    let Some((_, config)) = config else {
        return usage;
    };
    let Ok(root) = vdf::load(&config) else {
        return usage;
    };
    let Some(apps) = root.path(&["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"])
    else {
        return usage;
    };
    for (key, value) in &apps.children {
        let app_id = number(Some(key)).and_then(|n| u32::try_from(n).ok());
        if let (vdf::VdfValue::Node(node), Some(app_id)) = (value, app_id) {
            let minutes = number(node.value("Playtime")).unwrap_or(0);
            usage.insert(
                app_id,
                (
                    unix_time(node.value("LastPlayed")),
                    Duration::from_secs(minutes.saturating_mul(60)),
                ),
            );
        }
    }
    usage
}

fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// Número sem sinal como o `NumberStyles.None` do C#; appid além de 32 bits é inválido.
fn number(s: Option<&str>) -> Option<u64> {
    s.filter(|s| is_digits(s))?.parse().ok()
}

fn unix_time(s: Option<&str>) -> Option<DateTime<Utc>> {
    let secs = i64::try_from(number(s)?).ok().filter(|&s| s > 0)?;
    DateTime::from_timestamp(secs, 0)
}

fn same_path(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| {
        let p = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        p.to_string_lossy()
            .trim_end_matches(['\\', '/'])
            .to_lowercase()
    };
    norm(a) == norm(b)
}

/// Converte um jogo da Steam em [`Game`] (id `steam:<appid>`, abre por `steam://rungameid/<appid>`).
pub fn to_game(app: &SteamApp) -> Game {
    let id = app.app_id;
    Game {
        id: format!("steam:{id}"),
        title: app.name.clone(),
        store: Some(GameStore::Steam),
        size_label: size_label(app.size_on_disk),
        accent: ACCENTS[id as usize % ACCENTS.len()].to_owned(),
        cover_uri: Some(format!("{ART_SCHEME}://{id}/cover")),
        hero_uri: Some(format!("{ART_SCHEME}://{id}/hero")),
        is_installed: true,
        added_at: app.installed_at,
        last_played_at: app.last_played,
        play_time: app.play_time,
        launch_target: Some(format!("steam://rungameid/{id}")),
        install_path: Some(app.install_path.to_string_lossy().into_owned()),
        steam_app_id: Some(id),
        ..Default::default()
    }
}

/// "500 MB", "2,5 GB", "120 GB" (uma casa abaixo de 10 GB); `None` sem tamanho.
pub fn size_label(bytes: u64) -> Option<String> {
    const GB: u64 = 1 << 30;
    match bytes {
        0 => None,
        b if b < GB => Some(format!("{} MB", (b >> 20).max(1))),
        b => {
            let gb = b as f64 / GB as f64;
            let rounded = if b < 10 * GB {
                (gb * 10.0).round_ties_even() / 10.0
            } else {
                gb.round_ties_even()
            };
            let text = format!("{rounded}");
            let text = if lang() == Lang::EnUs {
                text
            } else {
                text.replace('.', ",")
            };
            Some(format!("{text} GB"))
        }
    }
}

/// Fonte da Steam: os arquivos em `root` (`None` = não instalada).
#[derive(Debug, Clone)]
pub struct SteamLibrarySource {
    root: Option<PathBuf>,
}

impl SteamLibrarySource {
    pub fn new(root: Option<PathBuf>) -> Self {
        SteamLibrarySource { root }
    }
}

impl LibrarySource for SteamLibrarySource {
    fn store(&self) -> GameStore {
        GameStore::Steam
    }

    fn scan(&self) -> LibraryScan {
        match &self.root {
            None => LibraryScan::missing(t("sources.steam.missing")),
            Some(root) => LibraryScan {
                games: scan(root).iter().map(to_game).collect(),
                client_installed: true,
                notice: None,
            },
        }
    }
}

/// Pastas onde a Steam costuma ficar no Linux e no macOS, na ordem de preferência.
pub fn unix_root_candidates(home: &Path, macos: bool) -> Vec<PathBuf> {
    if macos {
        return vec![home.join("Library/Application Support/Steam")];
    }
    vec![
        home.join(".steam/steam"),
        home.join(".local/share/Steam"),
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
        home.join("snap/steam/common/.local/share/Steam"),
    ]
}

/// Pasta da Steam neste PC; `None` se não está instalada.
pub fn find_root() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        use crate::registry::RegistryReader;
        let reg = crate::registry_windows::WindowsRegistry;
        [
            reg.get_string(r"HKEY_CURRENT_USER\Software\Valve\Steam", "SteamPath"),
            reg.get_string(
                r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Valve\Steam",
                "InstallPath",
            ),
            reg.get_string(r"HKEY_LOCAL_MACHINE\SOFTWARE\Valve\Steam", "InstallPath"),
            Some(r"C:\Program Files (x86)\Steam".to_owned()),
        ]
        .into_iter()
        .flatten()
        .map(|p| PathBuf::from(p.trim().replace('/', "\\")))
        .find(|p| p.is_dir())
    }
    #[cfg(not(windows))]
    {
        let home = PathBuf::from(std::env::var_os("HOME")?);
        // `~/.steam/steam` costuma ser link para `~/.local/share/Steam`: a primeira com `steamapps` vale.
        unix_root_candidates(&home, cfg!(target_os = "macos"))
            .into_iter()
            .find(|p| p.join("steamapps").is_dir())
    }
}

/// `RunningAppID` do `registry.vdf` da Steam (Linux/macOS); 0 = nenhum jogo rodando ou ilegível.
pub fn running_app_id(registry_vdf: &str) -> u32 {
    vdf::parse(registry_vdf)
        .path(&["Registry", "HKCU", "Software", "Valve", "Steam"])
        .and_then(|n| number(n.value("RunningAppID")))
        .and_then(|n| u32::try_from(n).ok())
        .unwrap_or(0)
}

/// O jogo da Steam rodando agora no Linux/macOS (0 = nenhum), lido de `registry.vdf`.
pub fn steam_running_app_id_unix() -> u32 {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return 0;
    };
    [
        home.join(".steam/registry.vdf"),
        home.join(".var/app/com.valvesoftware.Steam/.steam/registry.vdf"),
        home.join("Library/Application Support/Steam/registry.vdf"),
    ]
    .iter()
    .filter_map(|p| crate::read_limited(p, crate::MAX_STORE_FILE).ok())
    .map(|b| running_app_id(&crate::text_of(&b)))
    .find(|&id| id != 0)
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(root: &Path, rel: &str, content: &str) -> PathBuf {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, content).unwrap();
        p
    }

    fn escape(p: &Path) -> String {
        p.to_string_lossy().replace('\\', "\\\\")
    }

    fn manifest(library: &Path, app_id: u32, name: &str, flags: u32, size: u64) {
        write(
            library,
            &format!("steamapps/appmanifest_{app_id}.acf"),
            &format!(
                "\"AppState\"\n{{\n \"appid\" \"{app_id}\"\n \"name\" \"{name}\"\n \"StateFlags\" \"{flags}\"\n \"installdir\" \"{name}\"\n \"SizeOnDisk\" \"{size}\"\n}}"
            ),
        );
    }

    #[test]
    fn scanner_finds_installed_games_in_every_library_with_usage() {
        let dir = tempfile::tempdir().unwrap();
        let steam = dir.path().join("Steam");
        let extra = dir.path().join("SteamLibrary");
        write(
            &steam,
            "steamapps/libraryfolders.vdf",
            &format!(
                "\"libraryfolders\" {{ \"0\" {{ \"path\" \"{}\" }} \"1\" {{ \"path\" \"{}\" }} }}",
                escape(&steam),
                escape(&extra)
            ),
        );
        manifest(&steam, 570, "Dota 2", 4, 40 << 30);
        manifest(&extra, 1030300, "Hollow Knight: Silksong", 4, 8 << 30);
        manifest(&extra, 228980, "Steamworks Common Redistributables", 4, 1);
        manifest(&extra, 1493710, "Proton Experimental", 4, 1);
        manifest(&extra, 1628350, "Steam Linux Runtime 3.0 (sniper)", 4, 1);
        manifest(&extra, 999, "Baixando", 1026, 1);
        write(
            &steam,
            "userdata/123/config/localconfig.vdf",
            r#""UserLocalConfigStore" { "Software" { "Valve" { "Steam" { "apps" {
                "570" { "LastPlayed" "1760000000" "Playtime" "125" }
            } } } } }"#,
        );

        let apps = scan(&steam);

        let names: Vec<_> = apps.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["Dota 2", "Hollow Knight: Silksong"]);
        let dota = &apps[0];
        assert_eq!(dota.play_time, Duration::from_secs(125 * 60));
        assert_eq!(dota.last_played, DateTime::from_timestamp(1_760_000_000, 0));
        assert_eq!(
            dota.install_path,
            steam.join("steamapps").join("common").join("Dota 2")
        );

        let game = to_game(dota);
        assert_eq!(game.id, "steam:570");
        assert_eq!(game.launch_target.as_deref(), Some("steam://rungameid/570"));
        assert_eq!(game.size_label.as_deref(), Some("40 GB"));
        assert_eq!(game.cover_uri.as_deref(), Some("steam-art://570/cover"));
        assert_eq!(game.steam_app_id, Some(570));
    }

    #[test]
    fn scanner_accepts_old_libraryfolders_format_and_missing_files() {
        let dir = tempfile::tempdir().unwrap();
        let steam = dir.path().join("Steam");
        let extra = dir.path().join("Old");
        write(
            &steam,
            "steamapps/libraryfolders.vdf",
            &format!(
                "\"LibraryFolders\" {{ \"TimeNextStatsReport\" \"1\" \"1\" \"{}\" }}",
                escape(&extra)
            ),
        );
        assert_eq!(library_folders(&steam), [steam.clone(), extra]);
        assert!(scan(&dir.path().join("nao-existe")).is_empty());
    }

    #[test]
    fn broken_manifest_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "steamapps/appmanifest_1.acf",
            "\"AppState\" { \"appid\" \"-1\" ",
        );
        write(dir.path(), "steamapps/appmanifest_2.acf", "\u{0}\u{1}lixo");
        assert!(scan(dir.path()).is_empty());
    }

    #[test]
    fn size_label() {
        assert_eq!(super::size_label(0), None);
        assert_eq!(super::size_label(500 << 20).as_deref(), Some("500 MB"));
        assert_eq!(
            super::size_label((2.5 * (1u64 << 30) as f64) as u64).as_deref(),
            Some("2,5 GB")
        );
        assert_eq!(super::size_label(120 << 30).as_deref(), Some("120 GB"));
    }

    #[test]
    fn source_reports_missing_client() {
        let scan = SteamLibrarySource::new(None).scan();
        assert!(scan.games.is_empty());
        assert_eq!(
            scan.notice.as_deref(),
            Some("Steam não encontrada neste PC")
        );
    }

    #[test]
    fn root_candidates_cover_native_flatpak_and_snap() {
        let c = unix_root_candidates(Path::new("/home/u"), false);
        assert!(c.contains(&PathBuf::from(
            "/home/u/.var/app/com.valvesoftware.Steam/.local/share/Steam"
        )));
        assert!(c.contains(&PathBuf::from(
            "/home/u/snap/steam/common/.local/share/Steam"
        )));
        assert_eq!(
            unix_root_candidates(Path::new("/Users/u"), true),
            [PathBuf::from("/Users/u/Library/Application Support/Steam")]
        );
    }

    #[test]
    fn running_app_id_reads_registry_vdf() {
        let text = r#""Registry" { "HKCU" { "Software" { "Valve" { "Steam" {
            "RunningAppID" "1030300" "language" "brazilian" } } } } }"#;
        assert_eq!(running_app_id(text), 1030300);
        assert_eq!(running_app_id(&text.replace("1030300", "0")), 0);
        assert_eq!(running_app_id("lixo"), 0);
    }
}
