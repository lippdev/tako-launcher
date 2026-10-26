//! EA app (port de `Library/Ea`): pastas dos jogos pelo registro dos instaladores da EA e pelas
//! chaves de desinstalação de "Electronic Arts"; cada pasta precisa do `__Installer\installerdata.xml`
//! (ids de conteúdo e nome). Abre sempre pelo EA app (`origin2://`).

use crate::fs::StoreFs;
use crate::registry::RegistryReader;
use crate::windows_path;
use crate::{LibraryScan, LibrarySource};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tako_core::i18n::t;
use tako_core::library::{Game, GameStore};

/// Chaves com uma subchave por jogo, gravadas pelos instaladores da EA.
pub const GAME_KEYS: [&str; 4] = [
    r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\EA Games",
    r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Electronic Arts",
    r"HKEY_LOCAL_MACHINE\SOFTWARE\EA Games",
    r"HKEY_LOCAL_MACHINE\SOFTWARE\Electronic Arts",
];
pub const ORIGIN2_PROTOCOL_KEY: &str = r"HKEY_CLASSES_ROOT\origin2\shell\open\command";
pub const LINK2EA_PROTOCOL_KEY: &str = r"HKEY_CLASSES_ROOT\link2ea\shell\open\command";
const ACCENT: &str = "#D9302B";
const INSTALL_DIR_VALUES: [&str; 3] = ["Install Dir", "InstallDir", "Install Location"];
const PREFERRED_LOCALES: [&str; 2] = ["pt_BR", "en_US"];

/// O que o `installerdata.xml` diz: ids de conteúdo e o nome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EaInstallerData {
    pub content_ids: Vec<String>,
    pub title: Option<String>,
}

/// Quem atende o protocolo da EA neste PC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EaClientKind {
    /// Nenhum: os jogos aparecem, mas não abrem.
    Missing,
    /// Só o Origin, desligado pela EA em 2025: o protocolo existe, mas não abre mais os jogos.
    LegacyOrigin,
    EaApp,
}

/// `installerdata.xml` (formatos do Origin e do EA app): ids em `contentIDs/contentID` em qualquer
/// nível; nome em `gameTitles/gameTitle` ou `localeInfo/title`, preferindo pt_BR e depois en_US.
pub fn parse(xml: &str) -> Result<EaInstallerData, roxmltree::Error> {
    let doc = roxmltree::Document::parse(xml)?;
    let mut content_ids: Vec<String> = Vec::new();
    let mut titles: Vec<(Option<String>, String)> = Vec::new();
    for e in doc.descendants().filter(|n| n.is_element()) {
        let name = e.tag_name().name();
        if name.eq_ignore_ascii_case("contentID") {
            let id = inner_text(e).trim().to_owned();
            if !id.is_empty() && !content_ids.iter().any(|c| c.eq_ignore_ascii_case(&id)) {
                content_ids.push(id);
            }
        } else if name.eq_ignore_ascii_case("gameTitle") {
            titles.push((
                e.attribute("locale").map(Into::into),
                inner_text(e).trim().to_owned(),
            ));
        } else if name.eq_ignore_ascii_case("title")
            && let Some(parent) = e.parent_element()
            && parent.tag_name().name().eq_ignore_ascii_case("localeInfo")
        {
            titles.push((
                parent.attribute("locale").map(Into::into),
                inner_text(e).trim().to_owned(),
            ));
        }
    }
    titles.retain(|(_, t)| !t.is_empty());
    let title = PREFERRED_LOCALES
        .iter()
        .find_map(|l| {
            titles
                .iter()
                .find(|(loc, _)| loc.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(l)))
        })
        .or(titles.first())
        .map(|(_, t)| t.clone());
    Ok(EaInstallerData { content_ids, title })
}

/// Texto de um elemento e dos filhos (como `XElement.Value`).
pub(crate) fn inner_text(node: roxmltree::Node) -> String {
    node.descendants()
        .filter(|n| n.is_text())
        .filter_map(|n| n.text())
        .collect()
}

/// `origin2://game/launch?offerIds=<ids>&autoDownload=1`: o EA app herdou o protocolo do Origin e
/// aceita os ids de conteúdo (o `link2ea://` exige o id de oferta da loja, que não fica no disco).
pub fn launch_uri(content_ids: &[String]) -> String {
    let ids: Vec<String> = content_ids
        .iter()
        .map(|i| crate::epic::escape_data(i))
        .collect();
    format!(
        "origin2://game/launch?offerIds={}&autoDownload=1",
        ids.join(",")
    )
}

/// Quem atende o protocolo: o EA app (`EADesktop.exe`, `EALauncher.exe`…), só o `Origin.exe` ou nada.
pub fn client_kind(origin2: Option<&str>, link2ea: Option<&str>) -> EaClientKind {
    let is_ea_app = |c: &str| {
        let c = c.to_ascii_lowercase();
        ["ea desktop", "eadesktop", "ealaunch"]
            .iter()
            .any(|k| c.contains(k))
    };
    if link2ea.is_some_and(|c| !c.trim().is_empty()) || origin2.is_some_and(is_ea_app) {
        EaClientKind::EaApp
    } else if origin2.is_some_and(|c| !c.trim().is_empty()) {
        EaClientKind::LegacyOrigin
    } else {
        EaClientKind::Missing
    }
}

pub fn id_for(content_id: &str) -> String {
    format!("ea:{content_id}")
}

/// `None` sem id de conteúdo.
pub fn to_game(
    data: &EaInstallerData,
    title: &str,
    folder: &str,
    added_at: Option<DateTime<Utc>>,
) -> Option<Game> {
    let first = data.content_ids.first()?;
    Some(Game {
        id: id_for(first),
        title: title.trim().to_owned(),
        store: Some(GameStore::Ea),
        accent: ACCENT.to_owned(),
        is_installed: true,
        added_at,
        launch_target: Some(launch_uri(&data.content_ids)),
        install_path: Some(folder.to_owned()),
        store_product_id: Some(first.clone()),
        ..Default::default()
    })
}

pub struct EaLibrarySource {
    registry: Arc<dyn RegistryReader>,
    pub fs: StoreFs,
}

impl EaLibrarySource {
    pub fn new(registry: Arc<dyn RegistryReader>) -> Self {
        EaLibrarySource {
            registry,
            fs: StoreFs::real(),
        }
    }

    pub fn client(&self) -> EaClientKind {
        client_kind(
            self.registry
                .get_string(ORIGIN2_PROTOCOL_KEY, "")
                .as_deref(),
            self.registry
                .get_string(LINK2EA_PROTOCOL_KEY, "")
                .as_deref(),
        )
    }

    /// Pastas candidatas: o registro dos jogos da EA e os programas de "Electronic Arts" instalados.
    fn candidate_folders(&self) -> Vec<(String, Option<String>)> {
        let mut out = Vec::new();
        for root in GAME_KEYS {
            for name in self.registry.sub_keys(root) {
                let key = format!(r"{root}\{name}");
                if let Some(folder) = INSTALL_DIR_VALUES
                    .iter()
                    .filter_map(|v| self.registry.get_string(&key, v))
                    .find(|v| !v.trim().is_empty())
                {
                    out.push((folder, Some(name)));
                }
            }
        }
        for e in crate::uninstall::read(self.registry.as_ref()) {
            if let Some(folder) = e.install_location
                && e.publisher
                    .is_some_and(|p| p.to_lowercase().contains("electronic arts"))
            {
                out.push((folder, e.display_name));
            }
        }
        out
    }
}

impl LibrarySource for EaLibrarySource {
    fn store(&self) -> GameStore {
        GameStore::Ea
    }

    fn scan(&self) -> LibraryScan {
        let client = self.client();
        let mut games: Vec<Game> = Vec::new();
        for (folder, fallback_title) in self.candidate_folders() {
            let path = windows_path::normalize(&folder);
            if !(self.fs.exists)(&path)
                || games.iter().any(|g| {
                    g.install_path
                        .as_deref()
                        .is_some_and(|p| p.eq_ignore_ascii_case(&path))
                })
            {
                continue;
            }
            // Sem o arquivo do instalador não há como pedir ao EA app para abrir.
            let xml = self.fs.read_text(&windows_path::combine(
                &path,
                &["__Installer", "installerdata.xml"],
            ));
            let Some(Ok(data)) = xml.map(|x| parse(&x)) else {
                continue;
            };
            let Some(first) = data.content_ids.first() else {
                continue;
            };
            if games.iter().any(|g| g.id == id_for(first)) {
                continue;
            }
            let title = data
                .title
                .clone()
                .or(fallback_title)
                .unwrap_or_else(|| windows_path::last_name(&path));
            games.extend(to_game(
                &data,
                &title,
                &path,
                crate::install_dates::of(&path),
            ));
        }
        if games.is_empty() && client == EaClientKind::Missing {
            return LibraryScan::missing(t("sources.ea.missing"));
        }
        crate::text::sort_by_title(&mut games);
        LibraryScan {
            games,
            client_installed: client == EaClientKind::EaApp,
            notice: match client {
                EaClientKind::LegacyOrigin => Some(t("sources.ea.outdated")),
                EaClientKind::Missing => Some(t("sources.ea.no_client")),
                EaClientKind::EaApp => None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::fake;
    use crate::registry::MemoryRegistry;
    use std::collections::HashMap;
    use std::path::Path;

    const UNINSTALL: &str =
        r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/Stores/Ea")
                .join(name),
        )
        .unwrap()
    }

    #[test]
    fn installer_data_gives_content_ids_and_localized_title() {
        let data = parse(&fixture("installerdata-eaapp.xml")).unwrap();
        assert_eq!(data.content_ids, ["1077777", "1077778"]);
        assert_eq!(data.title.as_deref(), Some("Arena Estelar"));
        let origin = parse(&fixture("installerdata-origin.xml")).unwrap();
        assert_eq!(origin.content_ids, ["OFB-EAST:55555"]);
        assert_eq!(origin.title.as_deref(), Some("Deep Sea Station"));
        assert!(parse("<DiPManifest>").is_err());
        assert!(parse("<!DOCTYPE x [<!ENTITY a \"b\">]><x>&a;</x>").is_err());
    }

    #[test]
    fn launch_uri_uses_the_ea_app_protocol_with_every_content_id() {
        assert_eq!(
            launch_uri(&["1077777".into(), "OFB-EAST:55555".into()]),
            "origin2://game/launch?offerIds=1077777,OFB-EAST%3A55555&autoDownload=1"
        );
    }

    #[test]
    fn client_kind_tells_ea_app_from_old_origin() {
        assert_eq!(client_kind(None, None), EaClientKind::Missing);
        assert_eq!(
            client_kind(
                Some(r#""C:\Program Files (x86)\Origin\Origin.exe" "%1""#),
                None
            ),
            EaClientKind::LegacyOrigin
        );
        assert_eq!(
            client_kind(
                Some(
                    r#""C:\Program Files\Electronic Arts\EA Desktop\EA Desktop\EALauncher.exe" "%1""#
                ),
                None
            ),
            EaClientKind::EaApp
        );
        assert_eq!(
            client_kind(
                None,
                Some(
                    r#""C:\Program Files\Electronic Arts\EA Desktop\EA Desktop\EALaunchHelper.exe" "%1""#
                )
            ),
            EaClientKind::EaApp
        );
    }

    #[test]
    fn source_reads_registry_and_uninstall_folders_with_installer_data() {
        let arena = r"C:\Program Files\EA Games\Arena Estelar";
        let deep = r"D:\Origin Games\Deep Sea Station";
        let ea_desktop = r"C:\Program Files\Electronic Arts\EA Desktop";
        let reg = MemoryRegistry::default()
            .with(
                &format!(r"{}\Arena Estelar", GAME_KEYS[0]),
                "Install Dir",
                &format!("{arena}\\"),
            )
            .with(
                &format!(r"{}\EA Desktop", GAME_KEYS[1]),
                "InstallLocation",
                ea_desktop,
            )
            .with(
                &format!(r"{UNINSTALL}\{{DEEP-SEA}}"),
                "DisplayName",
                "Deep Sea Station",
            )
            .with(
                &format!(r"{UNINSTALL}\{{DEEP-SEA}}"),
                "Publisher",
                "Electronic Arts",
            )
            .with(
                &format!(r"{UNINSTALL}\{{DEEP-SEA}}"),
                "InstallLocation",
                deep,
            )
            .with(
                &format!(r"{UNINSTALL}\{{ARENA}}"),
                "Publisher",
                "Electronic Arts, Inc.",
            )
            .with(&format!(r"{UNINSTALL}\{{ARENA}}"), "InstallLocation", arena)
            .with(LINK2EA_PROTOCOL_KEY, "", "EALaunchHelper.exe \"%1\"");
        let files = HashMap::from([
            (
                format!(r"{arena}\__Installer\installerdata.xml"),
                fixture("installerdata-eaapp.xml").into_bytes(),
            ),
            (
                format!(r"{deep}\__Installer\installerdata.xml"),
                fixture("installerdata-origin.xml").into_bytes(),
            ),
        ]);
        let mut source = EaLibrarySource::new(Arc::new(reg));
        source.fs = fake::with(&[arena, deep, ea_desktop], files);

        let scan = source.scan();

        assert!(scan.client_installed);
        assert_eq!(scan.notice, None);
        let ids: Vec<_> = scan.games.iter().map(|g| g.id.as_str()).collect();
        assert_eq!(ids, ["ea:1077777", "ea:OFB-EAST:55555"]);
        let g = &scan.games[0];
        assert_eq!(g.title, "Arena Estelar");
        assert_eq!(g.install_path.as_deref(), Some(arena));
        assert!(
            g.launch_target
                .as_deref()
                .unwrap()
                .starts_with("origin2://game/launch?offerIds=1077777")
        );
    }

    #[test]
    fn only_old_origin_is_outdated() {
        let reg = MemoryRegistry::default().with(
            ORIGIN2_PROTOCOL_KEY,
            "",
            "\"C:\\Origin\\Origin.exe\" \"%1\"",
        );
        let mut source = EaLibrarySource::new(Arc::new(reg));
        source.fs = fake::folders(&[]);
        let scan = source.scan();
        assert!(!scan.client_installed);
        assert_eq!(
            scan.notice.as_deref(),
            Some("Só o Origin está instalado e ele não abre mais os jogos: instale o EA app")
        );
    }
}
