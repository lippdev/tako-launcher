//! RetroArch: pastas do `retroarch.cfg` (playlists, núcleos, capas) e as playlists `.lpl` (JSON desde a 1.7.6;
//! antes, 6 linhas por jogo). Cada item vira uma ROM com o núcleo escolhido nela (ou o padrão da playlist).

use super::emulator::Env;
use super::ini::Ini;
use super::{Found, Origin, Via, systems};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tako_core::paths::Os;

/// Pastas do RetroArch lidas do `retroarch.cfg`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    pub playlists: PathBuf,
    pub cores: PathBuf,
    pub thumbnails: PathBuf,
}

/// Onde o `retroarch.cfg` pode estar. `exe_dir`: o RetroArch "portátil" do Windows (e o da Steam) guarda ao lado.
pub fn config_dirs(env: &Env, exe_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = exe_dir.map(Path::to_path_buf).into_iter().collect();
    match env.os() {
        Os::Windows => {
            if let Some(a) = env.var("APPDATA") {
                v.push(a.join("RetroArch"));
            }
        }
        Os::Linux => {
            v.push(env.config_home().join("retroarch"));
            v.push(
                env.flatpak_home("org.libretro.RetroArch")
                    .join("config")
                    .join("retroarch"),
            );
        }
        Os::MacOs => v.push(env.mac_support().join("RetroArch")),
    }
    if let Some(steam) = &env.steam_root {
        v.push(steam.join("steamapps").join("common").join("RetroArch"));
    }
    v
}

/// `:` no começo = pasta do executável; `~` = pasta do usuário; vazio ou `default` = o padrão.
fn resolve(
    value: Option<&str>,
    config_dir: &Path,
    exe_dir: Option<&Path>,
    home: &Path,
) -> Option<PathBuf> {
    let v = value?.trim();
    if v.is_empty() || v.eq_ignore_ascii_case("default") {
        return None;
    }
    if let Some(rest) = v.strip_prefix(':') {
        let base = exe_dir.unwrap_or(config_dir);
        return Some(base.join(rest.trim_start_matches(['/', '\\'])));
    }
    if let Some(rest) = v.strip_prefix('~') {
        return Some(home.join(rest.trim_start_matches(['/', '\\'])));
    }
    Some(PathBuf::from(v))
}

pub fn read_dirs(cfg: &str, config_dir: &Path, exe_dir: Option<&Path>, home: &Path) -> Dirs {
    let ini = Ini::parse(cfg);
    let get = |key: &str, default: &str| {
        resolve(ini.get("", key), config_dir, exe_dir, home)
            .unwrap_or_else(|| config_dir.join(default))
    };
    Dirs {
        playlists: get("playlist_directory", "playlists"),
        cores: get("libretro_directory", "cores"),
        thumbnails: get("thumbnails_directory", "thumbnails"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub path: String,
    pub label: String,
    /// Núcleo escolhido no item; `None` = "DETECT" (o RetroArch pergunta).
    pub core: Option<String>,
}

#[derive(Deserialize)]
struct JsonPlaylist {
    #[serde(default)]
    default_core_path: String,
    #[serde(default)]
    items: Vec<JsonItem>,
}

#[derive(Deserialize)]
struct JsonItem {
    #[serde(default)]
    path: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    core_path: String,
}

fn core_of(value: &str) -> Option<String> {
    let v = value.trim();
    (!v.is_empty() && !v.eq_ignore_ascii_case("DETECT")).then(|| v.to_owned())
}

/// Itens de uma playlist; texto quebrado devolve o que der para ler.
pub fn parse_playlist(text: &str) -> Vec<Item> {
    let trimmed = text.trim_start();
    if trimmed.starts_with('{') {
        let Ok(p) = serde_json::from_str::<JsonPlaylist>(trimmed) else {
            return Vec::new();
        };
        let default = core_of(&p.default_core_path);
        return p
            .items
            .into_iter()
            .filter(|i| !i.path.trim().is_empty())
            .map(|i| Item {
                core: core_of(&i.core_path).or_else(|| default.clone()),
                path: i.path,
                label: i.label,
            })
            .collect();
    }
    // Formato antigo: caminho, nome, núcleo, nome do núcleo, crc, banco.
    let lines: Vec<&str> = text.lines().collect();
    lines
        .chunks(6)
        .filter(|c| c.len() >= 3 && !c[0].trim().is_empty())
        .map(|c| Item {
            path: c[0].trim().to_owned(),
            label: c[1].trim().to_owned(),
            core: core_of(c[2]),
        })
        .collect()
}

/// Nome de arquivo das capas do RetroArch: `&*/:`<>?\|"` viram `_`.
pub fn thumbnail_name(label: &str) -> String {
    label
        .chars()
        .map(|c| if "&*/:`<>?\\|\"".contains(c) { '_' } else { c })
        .collect()
}

/// ROMs das playlists, com o sistema pelo nome da playlist e a capa baixada pelo RetroArch, se houver.
pub fn read(
    dirs: &Dirs,
    exists: &dyn Fn(&Path) -> bool,
    read_text: &dyn Fn(&Path) -> Option<String>,
    list_dir: &dyn Fn(&Path) -> Vec<PathBuf>,
) -> Vec<Found> {
    let mut out = Vec::new();
    for playlist in list_dir(&dirs.playlists) {
        let is_lpl = playlist
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("lpl"));
        let Some(name) = playlist
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
        else {
            continue;
        };
        // Histórico e favoritos repetem jogos de outras playlists.
        if !is_lpl
            || [
                "content_history",
                "content_favorites",
                "content_music_history",
                "content_video_history",
                "content_image_history",
            ]
            .contains(&name.as_str())
        {
            continue;
        }
        let Some(text) = read_text(&playlist) else {
            continue;
        };
        let system = systems::by_name(&name);
        for item in parse_playlist(&text) {
            // `jogo.zip#rom.sfc`: o arquivo que existe é o zip.
            let file = item.path.split('#').next().unwrap_or(&item.path);
            if !exists(Path::new(file)) {
                continue;
            }
            let cover = (!item.label.is_empty())
                .then(|| {
                    dirs.thumbnails
                        .join(&name)
                        .join("Named_Boxarts")
                        .join(format!("{}.png", thumbnail_name(&item.label)))
                })
                .filter(|p| exists(p));
            out.push(Found {
                path: PathBuf::from(&item.path),
                title: (!item.label.is_empty()).then(|| item.label.clone()),
                system,
                via: match item.core {
                    Some(c) => Via::Core(PathBuf::from(c)),
                    None => Via::Auto,
                },
                cover,
                origin: Origin::RetroArch,
            });
        }
    }
    out
}

/// Arquivo do núcleo na pasta de núcleos (`snes9x_libretro.dll`).
pub fn core_file(cores: &Path, core: &str, os: Os) -> PathBuf {
    let ext = match os {
        Os::Windows => "dll",
        Os::Linux => "so",
        Os::MacOs => "dylib",
    };
    cores.join(format!("{core}_libretro.{ext}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cfg_dirs_follow_portable_home_and_defaults() {
        let cfg = "playlist_directory = \":\\playlists\"\nlibretro_directory = \"~/cores\"\nthumbnails_directory = \"default\"\n";
        let d = read_dirs(
            cfg,
            Path::new("/cfg"),
            Some(Path::new("/exe")),
            Path::new("/home/u"),
        );
        assert_eq!(d.playlists, Path::new("/exe").join("playlists"));
        assert_eq!(d.cores, Path::new("/home/u").join("cores"));
        assert_eq!(d.thumbnails, Path::new("/cfg").join("thumbnails"));
        let d = read_dirs("", Path::new("/cfg"), None, Path::new("/h"));
        assert_eq!(d.playlists, Path::new("/cfg").join("playlists"));
    }

    #[test]
    fn json_and_legacy_playlists() {
        let json = r#"{"version":"1.5","default_core_path":"/c/snes9x_libretro.so","items":[
            {"path":"/r/a.sfc","label":"A","core_path":"DETECT"},
            {"path":"/r/b.sfc","label":"B","core_path":"/c/bsnes_libretro.so"},
            {"path":"","label":"vazio"}]}"#;
        let items = parse_playlist(json);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].core.as_deref(), Some("/c/snes9x_libretro.so"));
        assert_eq!(items[1].core.as_deref(), Some("/c/bsnes_libretro.so"));
        let legacy = "/r/c.nes\nC\nDETECT\nDETECT\n0|crc\nNintendo - NES.lpl\n/r/d.nes\nD\n/c/mesen_libretro.so\nMesen\n0|crc\nx.lpl\n";
        let items = parse_playlist(legacy);
        assert_eq!(items[0].core, None);
        assert_eq!(items[1].core.as_deref(), Some("/c/mesen_libretro.so"));
        assert!(parse_playlist("{lixo").is_empty());
        assert_eq!(thumbnail_name("A: B/C?"), "A_ B_C_");
    }
}
