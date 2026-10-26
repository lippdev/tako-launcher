//! ES-DE (EmulationStation Desktop Edition) e o EmulationStation antigo: a pasta de ROMs (`ROMDirectory` do
//! `es_settings.xml`, padrão `~/ROMs`, uma subpasta por sistema) e, dos `gamelist.xml`, o nome de cada jogo,
//! os escondidos e as capas baixadas.

use super::Fs;
use super::emulator::Env;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Pastas do ES-DE (3.x: `~/ES-DE`) e do EmulationStation antigo (`~/.emulationstation`).
pub fn roots(env: &Env) -> Vec<PathBuf> {
    vec![env.home.join("ES-DE"), env.home.join(".emulationstation")]
}

/// `ROMDirectory` do `es_settings.xml` (que não tem elemento raiz). `None` = o padrão.
pub fn rom_directory(settings: &str, home: &Path) -> Option<PathBuf> {
    let body = settings
        .trim_start()
        .strip_prefix("<?xml")
        .and_then(|s| s.split_once("?>").map(|(_, rest)| rest))
        .unwrap_or(settings);
    let wrapped = format!("<root>{body}</root>");
    let doc = roxmltree::Document::parse(&wrapped).ok()?;
    let value = doc
        .descendants()
        .find(|n| n.has_tag_name("string") && n.attribute("name") == Some("ROMDirectory"))?
        .attribute("value")?
        .trim();
    if value.is_empty() || value.contains("%ESPATH%") {
        return None;
    }
    Some(match value.strip_prefix('~') {
        Some(rest) => home.join(rest.trim_start_matches(['/', '\\'])),
        None => PathBuf::from(value),
    })
}

/// Um jogo do `gamelist.xml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: PathBuf,
    pub name: Option<String>,
    pub hidden: bool,
}

/// Jogos de um `gamelist.xml`; `./` é relativo à pasta do sistema.
pub fn parse_gamelist(xml: &str, system_dir: &Path) -> Vec<Entry> {
    let Ok(doc) = roxmltree::Document::parse(xml) else {
        return Vec::new();
    };
    let text = |node: roxmltree::Node, tag: &str| {
        node.children()
            .find(|c| c.has_tag_name(tag))
            .and_then(|c| c.text())
            .map(|t| t.trim().to_owned())
            .filter(|t| !t.is_empty())
    };
    doc.descendants()
        .filter(|n| n.has_tag_name("game"))
        .filter_map(|g| {
            let raw = text(g, "path")?;
            let path = match raw.strip_prefix("./") {
                Some(rel) => system_dir.join(rel),
                None if Path::new(&raw).is_absolute() => PathBuf::from(&raw),
                None => system_dir.join(&raw),
            };
            Some(Entry {
                path,
                name: text(g, "name"),
                hidden: text(g, "hidden").is_some_and(|h| h.eq_ignore_ascii_case("true")),
            })
        })
        .collect()
}

/// O que o ES-DE sabe: a pasta de ROMs e, por ROM, nome, escondido e capa.
#[derive(Debug, Default)]
pub struct Library {
    pub rom_dir: Option<PathBuf>,
    pub entries: HashMap<String, (Entry, Option<PathBuf>)>,
}

/// Chave de comparação de caminhos (o ES-DE grava `/` mesmo no Windows).
pub fn key_of(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) { s.to_lowercase() } else { s }
}

pub fn read(env: &Env, fs: &Fs) -> Library {
    let mut lib = Library::default();
    let Some(root) = roots(env).into_iter().find(|r| (fs.is_dir)(r)) else {
        return lib;
    };
    let settings = [
        root.join("settings").join("es_settings.xml"),
        root.join("es_settings.xml"),
    ]
    .into_iter()
    .find_map(|p| fs.read_text(&p));
    let rom_dir = settings
        .and_then(|s| rom_directory(&s, &env.home))
        .unwrap_or_else(|| env.home.join("ROMs"));
    for system_list in (fs.list_dir)(&root.join("gamelists")) {
        let Some(system) = system_list
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
        else {
            continue;
        };
        let Some(xml) = fs.read_text(&system_list.join("gamelist.xml")) else {
            continue;
        };
        let media = root.join("downloaded_media").join(&system).join("covers");
        for e in parse_gamelist(&xml, &rom_dir.join(&system)) {
            let cover = e.path.file_stem().and_then(|stem| {
                ["png", "jpg"]
                    .iter()
                    .map(|ext| media.join(format!("{}.{ext}", stem.to_string_lossy())))
                    .find(|p| (fs.exists)(p))
            });
            lib.entries.insert(key_of(&e.path), (e, cover));
        }
    }
    lib.rom_dir = Some(rom_dir).filter(|d| (fs.is_dir)(d));
    lib
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_without_root_element() {
        let s = "<?xml version=\"1.0\"?>\n<bool name=\"X\" value=\"true\" />\n<string name=\"ROMDirectory\" value=\"~/Jogos/ROMs\" />\n";
        assert_eq!(
            rom_directory(s, Path::new("/home/u")),
            Some(Path::new("/home/u").join("Jogos/ROMs"))
        );
        assert_eq!(
            rom_directory(
                "<string name=\"ROMDirectory\" value=\"\" />",
                Path::new("/h")
            ),
            None
        );
        assert_eq!(rom_directory("<lixo", Path::new("/h")), None);
    }

    #[test]
    fn gamelist_paths_names_and_hidden() {
        let xml = r#"<?xml version="1.0"?><gameList>
            <folder><path>./Pasta</path></folder>
            <game><path>./Super Mario World (USA).sfc</path><name>Super Mario World</name></game>
            <game><path>/abs/x.sfc</path><hidden>true</hidden></game>
            <game><name>sem caminho</name></game>
        </gameList>"#;
        let v = parse_gamelist(xml, Path::new("/roms/snes"));
        assert_eq!(v.len(), 2);
        assert_eq!(
            v[0].path,
            Path::new("/roms/snes").join("Super Mario World (USA).sfc")
        );
        assert_eq!(v[0].name.as_deref(), Some("Super Mario World"));
        assert!(v[1].hidden);
        assert!(parse_gamelist("<gameList>", Path::new("/r")).is_empty());
    }
}
