//! Heroic Games Launcher (Linux e macOS, MIG-026/027): jogos da Epic (legendary), GOG (gogdl),
//! Amazon (nile) e apps adicionados à mão, lidos dos arquivos do Heroic sem ele aberto.
//!
//! Os jogos abrem pelo protocolo do Heroic (`heroic://launch/<runner>/<appName>`, aceito pelas
//! versões antigas e novas), nunca pelo executável: o Heroic cuida de Wine/Proton, login e DRM.

use crate::{LibraryScan, LibrarySource, MAX_STORE_FILE, file_uri, read_limited};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tako_core::i18n::t;
use tako_core::library::{Game, GameStore};

#[cfg(test)]
mod tests;

/// Backend do Heroic que instalou o jogo (nome igual ao do Heroic, usado na URI e no id).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Runner {
    Legendary,
    Gog,
    Nile,
    Sideload,
}

impl Runner {
    pub fn key(self) -> &'static str {
        match self {
            Runner::Legendary => "legendary",
            Runner::Gog => "gog",
            Runner::Nile => "nile",
            Runner::Sideload => "sideload",
        }
    }

    /// Cor da loja de origem (ADR-0019: store-epic, store-gog, store-amazon).
    fn accent(self) -> &'static str {
        match self {
            Runner::Legendary => "#2A2A2A",
            Runner::Gog => "#7A3FD1",
            Runner::Nile => "#B35C00",
            Runner::Sideload => "#3A3F4B",
        }
    }
}

/// Um jogo instalado segundo os arquivos de um backend do Heroic.
#[derive(Debug, Clone, PartialEq)]
pub struct HeroicInstall {
    pub runner: Runner,
    pub app_name: String,
    pub title: Option<String>,
    pub install_path: Option<String>,
    /// Arte do próprio registro (só os apps adicionados à mão têm).
    pub art_square: Option<String>,
    pub art_cover: Option<String>,
}

/// Título e arte de um jogo no cache de biblioteca do Heroic (`store_cache/*_library.json`).
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct CachedInfo {
    pub app_name: String,
    pub title: Option<String>,
    pub art_square: Option<String>,
    pub art_cover: Option<String>,
    pub is_installed: bool,
    pub folder_name: Option<String>,
    pub install: Option<CachedInstall>,
}

/// `install` de um item do cache: só o que diz se é DLC.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct CachedInstall {
    pub is_dlc: bool,
}

/// `legendaryConfig/legendary/installed.json`: objeto `appName -> instalação`.
pub fn parse_legendary_installed(bytes: &[u8]) -> Vec<HeroicInstall> {
    #[derive(Deserialize)]
    struct Entry {
        #[serde(default)]
        app_name: String,
        title: Option<String>,
        install_path: Option<String>,
        #[serde(default)]
        is_dlc: bool,
    }
    let Ok(map) = serde_json::from_slice::<HashMap<String, Entry>>(bytes) else {
        tracing::warn!("installed.json do legendary ilegível");
        return Vec::new();
    };
    map.into_iter()
        .filter(|(_, e)| !e.is_dlc)
        .map(|(key, e)| HeroicInstall {
            runner: Runner::Legendary,
            app_name: if e.app_name.is_empty() {
                key
            } else {
                e.app_name
            },
            title: e.title,
            install_path: e.install_path,
            art_square: None,
            art_cover: None,
        })
        .collect()
}

/// `gog_store/installed.json`: `{ "installed": [{ appName, install_path, is_dlc, … }] }`.
pub fn parse_gog_installed(bytes: &[u8]) -> Vec<HeroicInstall> {
    #[derive(Deserialize)]
    struct File {
        #[serde(default)]
        installed: Vec<Entry>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Entry {
        app_name: Option<String>,
        #[serde(rename = "install_path")]
        install_path: Option<String>,
        #[serde(default, rename = "is_dlc")]
        is_dlc: bool,
    }
    let Ok(file) = serde_json::from_slice::<File>(bytes) else {
        tracing::warn!("installed.json do GOG (Heroic) ilegível");
        return Vec::new();
    };
    file.installed
        .into_iter()
        .filter(|e| !e.is_dlc)
        .filter_map(|e| {
            Some(HeroicInstall {
                runner: Runner::Gog,
                app_name: e.app_name?,
                title: None,
                install_path: e.install_path,
                art_square: None,
                art_cover: None,
            })
        })
        .collect()
}

/// `nile_config/nile/installed.json`: `[{ id, path, size, version }]`.
pub fn parse_nile_installed(bytes: &[u8]) -> Vec<HeroicInstall> {
    #[derive(Deserialize)]
    struct Entry {
        id: String,
        path: Option<String>,
    }
    let Ok(list) = serde_json::from_slice::<Vec<Entry>>(bytes) else {
        tracing::warn!("installed.json do nile ilegível");
        return Vec::new();
    };
    list.into_iter()
        .map(|e| HeroicInstall {
            runner: Runner::Nile,
            app_name: e.id,
            title: None,
            install_path: e.path,
            art_square: None,
            art_cover: None,
        })
        .collect()
}

/// Cache de biblioteca (`{ "library": [...] }` na Epic e Amazon, `{ "games": [...] }` no GOG e nos
/// apps adicionados à mão): lista de `GameInfo` do Heroic.
pub fn parse_library_cache(bytes: &[u8]) -> Vec<CachedInfo> {
    let Ok(serde_json::Value::Object(root)) = serde_json::from_slice(bytes) else {
        tracing::warn!("cache de biblioteca do Heroic ilegível");
        return Vec::new();
    };
    ["library", "games"]
        .iter()
        .filter_map(|k| root.get(*k)?.as_array())
        .flatten()
        // Um item quebrado não derruba os outros.
        .filter_map(|v| CachedInfo::deserialize(v).ok())
        .filter(|c| !c.app_name.is_empty())
        .collect()
}

/// `sideload_apps/library.json`: apps e jogos adicionados à mão no Heroic.
pub fn parse_sideload_library(bytes: &[u8]) -> Vec<HeroicInstall> {
    parse_library_cache(bytes)
        .into_iter()
        .filter(|c| c.is_installed)
        .map(|c| HeroicInstall {
            runner: Runner::Sideload,
            app_name: c.app_name,
            title: c.title,
            install_path: c.folder_name,
            art_square: c.art_square,
            art_cover: c.art_cover,
        })
        .collect()
}

/// URI de abertura pelo Heroic.
pub fn launch_uri(runner: Runner, app_name: &str) -> String {
    format!("heroic://launch/{}/{app_name}", runner.key())
}

/// O appName vai na URI e no id: só aceitamos os caracteres que as lojas usam.
fn valid_app_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 200
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}

/// Arte que o Heroic já baixou: `images-cache/<sha256 da URL>` (veja `images_cache.ts`), ou um
/// arquivo local (apps adicionados à mão). `suffix` é o que a tela do Heroic acrescenta à URL.
fn cached_art(root: &Path, art: Option<&str>, suffix: &str) -> Option<String> {
    let art = art?.trim();
    if let Some(local) = art
        .strip_prefix("file://")
        .or(Path::new(art).is_absolute().then_some(art))
    {
        let path = Path::new(local);
        return path.is_file().then(|| file_uri(path));
    }
    if !art.starts_with("http") {
        return None;
    }
    [format!("{art}{suffix}"), art.to_owned()]
        .iter()
        .map(|url| {
            root.join("images-cache")
                .join(format!("{:x}", Sha256::digest(url.as_bytes())))
        })
        .find(|p| p.is_file())
        .map(|p| file_uri(&p))
}

fn read_file(path: &Path) -> Option<Vec<u8>> {
    match read_limited(path, MAX_STORE_FILE) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            tracing::warn!("não deu para ler {}: {e}", path.display());
            None
        }
    }
}

/// Jogos instalados de uma pasta de configuração do Heroic (nativa, Flatpak ou macOS).
pub fn scan_root(root: &Path) -> Vec<Game> {
    let cache = |name: &str| -> HashMap<String, CachedInfo> {
        read_file(&root.join("store_cache").join(name))
            .map(|b| parse_library_cache(&b))
            .unwrap_or_default()
            .into_iter()
            .map(|c| (c.app_name.clone(), c))
            .collect()
    };
    let installed = |rel: &[&str], parse: fn(&[u8]) -> Vec<HeroicInstall>| {
        let path = rel.iter().fold(root.to_path_buf(), |p, part| p.join(part));
        read_file(&path).map(|b| parse(&b)).unwrap_or_default()
    };

    let groups = [
        (
            installed(
                &["legendaryConfig", "legendary", "installed.json"],
                parse_legendary_installed,
            ),
            cache("legendary_library.json"),
        ),
        (
            installed(&["gog_store", "installed.json"], parse_gog_installed),
            cache("gog_library.json"),
        ),
        (
            installed(
                &["nile_config", "nile", "installed.json"],
                parse_nile_installed,
            ),
            cache("nile_library.json"),
        ),
        (
            installed(&["sideload_apps", "library.json"], parse_sideload_library),
            HashMap::new(),
        ),
    ];

    let mut games = Vec::new();
    let runners = [
        Runner::Legendary,
        Runner::Gog,
        Runner::Nile,
        Runner::Sideload,
    ];
    for (runner, (installs, cache)) in runners.into_iter().zip(groups) {
        let before = games.len();
        for install in installs {
            if !valid_app_name(&install.app_name) {
                continue;
            }
            // Jogo apagado por fora do Heroic: a pasta sumiu (apps à mão podem nem ter pasta).
            let missing = install
                .install_path
                .as_deref()
                .is_some_and(|p| !Path::new(p).is_dir());
            if missing && install.runner != Runner::Sideload {
                continue;
            }
            let cached = cache.get(&install.app_name);
            games.push(to_game(root, install, cached));
        }
        // Os da conta que não estão instalados (SRC-005): o resto do cache, fora as DLCs (o GOG põe ali também o
        // `gog-redist`, marcado como DLC). O protocolo do Heroic pergunta se instala quando o jogo não está
        // instalado (`protocol.ts`, `handleLaunch`).
        let installed: HashSet<String> = games[before..].iter().map(|g| g.id.clone()).collect();
        let mut owned: Vec<&CachedInfo> = cache
            .values()
            .filter(|c| {
                !c.install.as_ref().is_some_and(|i| i.is_dlc) && valid_app_name(&c.app_name)
            })
            .collect();
        owned.sort_by(|a, b| a.app_name.cmp(&b.app_name));
        for cached in owned {
            let install = HeroicInstall {
                runner,
                app_name: cached.app_name.clone(),
                title: None,
                install_path: None,
                art_square: None,
                art_cover: None,
            };
            let mut game = to_game(root, install, Some(cached));
            if installed.contains(&game.id) {
                continue;
            }
            game.is_installed = false;
            game.install_uri = game.launch_target.take();
            games.push(game);
        }
    }
    games
}

fn to_game(root: &Path, install: HeroicInstall, cached: Option<&CachedInfo>) -> Game {
    let pick = |own: &Option<String>, from_cache: Option<&Option<String>>| {
        own.clone()
            .or_else(|| from_cache.cloned().flatten())
            .filter(|s| !s.trim().is_empty())
    };
    let title = pick(&install.title, cached.map(|c| &c.title))
        .or_else(|| {
            let path = install.install_path.as_deref()?;
            Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| install.app_name.clone());
    let square = pick(&install.art_square, cached.map(|c| &c.art_square));
    let wide = pick(&install.art_cover, cached.map(|c| &c.art_cover));
    // A grade do Heroic pede a capa da Epic redimensionada (`getImageFormatting`).
    let suffix = if install.runner == Runner::Legendary {
        "?h=400&resize=1&w=300"
    } else {
        ""
    };
    let cover_uri = cached_art(root, square.as_deref(), suffix);
    let hero_uri = cached_art(root, wide.as_deref(), "");
    Game {
        id: format!("heroic:{}:{}", install.runner.key(), install.app_name),
        title,
        store: Some(GameStore::Heroic),
        accent: install.runner.accent().to_owned(),
        has_fallback_art: cover_uri.is_some(),
        cover_uri,
        hero_uri,
        is_installed: true,
        launch_target: Some(launch_uri(install.runner, &install.app_name)),
        install_path: install.install_path,
        store_product_id: Some(install.app_name),
        ..Default::default()
    }
}

/// Fonte do Heroic: junta as pastas de configuração encontradas (nativa e Flatpak).
pub struct HeroicSource {
    roots: Vec<PathBuf>,
}

impl HeroicSource {
    pub fn new(roots: Vec<PathBuf>) -> Self {
        HeroicSource { roots }
    }

    /// Pastas padrão do Heroic neste SO.
    #[cfg(unix)]
    pub fn detect() -> Self {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default();
        #[cfg(not(target_os = "macos"))]
        let roots = {
            let config = std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .unwrap_or_else(|| home.join(".config"));
            let mut roots = vec![
                config.join("heroic"),
                home.join(".var/app/com.heroicgameslauncher.hgl/config/heroic"),
            ];
            // Num Flatpak o XDG_CONFIG_HOME é o da sandbox; o Heroic nativo fica na pasta padrão do usuário.
            let host = home.join(".config/heroic");
            if !roots.contains(&host) {
                roots.push(host);
            }
            roots
        };
        #[cfg(target_os = "macos")]
        let roots = vec![home.join("Library/Application Support/heroic")];
        HeroicSource::new(roots)
    }
}

impl LibrarySource for HeroicSource {
    fn store(&self) -> GameStore {
        GameStore::Heroic
    }

    fn scan(&self) -> LibraryScan {
        let roots: Vec<&PathBuf> = self.roots.iter().filter(|r| r.is_dir()).collect();
        if roots.is_empty() {
            return LibraryScan::missing(t("sources_unix.heroic_missing"));
        }
        let mut seen = HashSet::new();
        let mut games: Vec<Game> = roots
            .into_iter()
            .flat_map(|r| scan_root(r))
            .filter(|g| seen.insert(g.id.clone()))
            .collect();
        games.sort_by_cached_key(|g| g.title.to_lowercase());
        LibraryScan {
            games,
            client_installed: true,
            notice: None,
        }
    }
}
