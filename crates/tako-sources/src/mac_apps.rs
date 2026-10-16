//! Apps `.app` do macOS (MIG-027): título e ícone de um jogo manual pelo `Info.plist` do pacote,
//! e a lista de jogos instalados em `/Applications` pela categoria declarada no pacote.

use serde::Deserialize;
use std::path::{Path, PathBuf};

/// O que o Tako usa de um pacote `.app`.
#[derive(Debug, Clone, PartialEq)]
pub struct MacApp {
    pub bundle: PathBuf,
    pub name: String,
    pub bundle_id: Option<String>,
    /// `Contents/Resources/<CFBundleIconFile>.icns`, quando existe.
    pub icon: Option<PathBuf>,
    pub category: Option<String>,
}

impl MacApp {
    pub fn is_game(&self) -> bool {
        self.category.as_deref().is_some_and(is_game_category)
    }
}

/// Campos do `Info.plist` (XML ou binário).
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct InfoPlist {
    #[serde(rename = "CFBundleName")]
    pub name: Option<String>,
    #[serde(rename = "CFBundleDisplayName")]
    pub display_name: Option<String>,
    #[serde(rename = "CFBundleIdentifier")]
    pub identifier: Option<String>,
    #[serde(rename = "CFBundleIconFile")]
    pub icon_file: Option<String>,
    #[serde(rename = "LSApplicationCategoryType")]
    pub category: Option<String>,
}

pub fn parse_info_plist(bytes: &[u8]) -> Option<InfoPlist> {
    plist::from_bytes(bytes).ok()
}

/// `public.app-category.games` ou uma das subcategorias (`…action-games`, `…puzzle-games`…).
pub fn is_game_category(category: &str) -> bool {
    category
        .strip_prefix("public.app-category.")
        .is_some_and(|c| c == "games" || c.ends_with("-games"))
}

/// Lê um pacote `.app`. Sem `Info.plist` legível, usa o nome da pasta.
pub fn read_app(bundle: &Path) -> Option<MacApp> {
    if !bundle.is_dir() {
        return None;
    }
    let info = crate::read_limited(&bundle.join("Contents/Info.plist"), 1024 * 1024)
        .ok()
        .and_then(|b| parse_info_plist(&b))
        .unwrap_or_default();
    let clean = |s: Option<String>| s.filter(|s| !s.trim().is_empty());
    let name = clean(info.display_name)
        .or(clean(info.name))
        .or_else(|| Some(bundle.file_stem()?.to_string_lossy().into_owned()))?;
    let icon = clean(info.icon_file)
        // O nome não pode sair de `Resources`.
        .filter(|f| !f.contains('/') && !f.starts_with('.'))
        .map(|f| {
            let file = if Path::new(&f).extension().is_some() {
                f
            } else {
                format!("{f}.icns")
            };
            bundle.join("Contents/Resources").join(file)
        })
        .filter(|p| p.is_file());
    Some(MacApp {
        bundle: bundle.to_path_buf(),
        name,
        bundle_id: clean(info.identifier),
        icon,
        category: clean(info.category),
    })
}

/// Jogos (pela categoria) nas pastas dadas, sem descer em subpastas que não são `.app`.
pub fn list_game_apps_in(dirs: &[PathBuf]) -> Vec<MacApp> {
    let mut out: Vec<MacApp> = dirs
        .iter()
        .filter_map(|d| std::fs::read_dir(d).ok())
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "app"))
        .filter_map(|p| read_app(&p))
        .filter(MacApp::is_game)
        .collect();
    out.sort_by_cached_key(|a| a.name.to_lowercase());
    out
}

/// Jogos em `/Applications` e `~/Applications`, para o "Adicionar jogo".
#[cfg(target_os = "macos")]
pub fn list_game_apps() -> Vec<MacApp> {
    let mut dirs = vec![PathBuf::from("/Applications")];
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join("Applications"));
    }
    list_game_apps_in(&dirs)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::fs;

    const PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>JogoMac</string>
  <key>CFBundleDisplayName</key><string>Jogo Mac Fictício</string>
  <key>CFBundleIdentifier</key><string>com.exemplo.jogomac</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.puzzle-games</string>
  <key>CFBundleVersion</key><string>1.0</string>
</dict>
</plist>"#;

    fn make_app(dir: &Path, name: &str, plist: &[u8]) -> PathBuf {
        let bundle = dir.join(format!("{name}.app"));
        fs::create_dir_all(bundle.join("Contents/Resources")).unwrap();
        fs::write(bundle.join("Contents/Info.plist"), plist).unwrap();
        bundle
    }

    #[test]
    fn categories() {
        assert!(is_game_category("public.app-category.games"));
        assert!(is_game_category("public.app-category.action-games"));
        assert!(!is_game_category("public.app-category.utilities"));
        assert!(!is_game_category("games"));
    }

    #[test]
    fn reads_bundle_title_icon_and_category() {
        let tmp = tempfile::tempdir().unwrap();
        let bundle = make_app(tmp.path(), "JogoMac", PLIST.as_bytes());
        fs::write(bundle.join("Contents/Resources/AppIcon.icns"), b"icns").unwrap();
        let app = read_app(&bundle).unwrap();
        assert_eq!(app.name, "Jogo Mac Fictício");
        assert_eq!(app.bundle_id.as_deref(), Some("com.exemplo.jogomac"));
        assert_eq!(
            app.icon,
            Some(bundle.join("Contents/Resources/AppIcon.icns"))
        );
        assert!(app.is_game());
    }

    #[test]
    fn binary_plist_and_broken_plist() {
        let tmp = tempfile::tempdir().unwrap();
        let info = parse_info_plist(PLIST.as_bytes()).unwrap();
        let value: plist::Value = plist::from_bytes(PLIST.as_bytes()).unwrap();
        let mut binary = Vec::new();
        value.to_writer_binary(&mut binary).unwrap();
        assert_eq!(parse_info_plist(&binary), Some(info));

        assert_eq!(parse_info_plist(b"<plist><dict><key>"), None);
        let broken = make_app(tmp.path(), "Quebrado", b"\0\0lixo");
        let app = read_app(&broken).unwrap();
        assert_eq!(app.name, "Quebrado");
        assert_eq!(app.icon, None);
        assert!(!app.is_game());
        assert_eq!(read_app(&tmp.path().join("nada.app")), None);
    }

    #[test]
    fn lists_only_games() {
        let tmp = tempfile::tempdir().unwrap();
        make_app(tmp.path(), "JogoMac", PLIST.as_bytes());
        let tool = PLIST.replace("puzzle-games", "utilities");
        make_app(tmp.path(), "Ferramenta", tool.as_bytes());
        let list = list_game_apps_in(&[tmp.path().to_path_buf(), tmp.path().join("nada")]);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Jogo Mac Fictício");
    }
}
