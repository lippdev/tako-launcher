//! `MicrosoftGame.config` (jogos do GDK) e `AppxManifest.xml` (port de `PackageManifests.cs`).
//! Leitura tolerante: ignora namespaces (uap, uap10…) e recusa DTD.

use super::package::family_name;
use roxmltree::{Document, Node};

pub const GAME_CONFIG_FILE: &str = "MicrosoftGame.config";
pub const APPX_MANIFEST_FILE: &str = "AppxManifest.xml";

/// Executável declarado no `MicrosoftGame.config`; o `id` vira o id do app no AUMID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameExecutable {
    pub name: String,
    pub id: String,
}

/// `MicrosoftGame.config`: arquivo presente = o pacote é um jogo.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MicrosoftGameConfig {
    pub identity_name: String,
    pub publisher: String,
    pub version: Option<String>,
    pub executables: Vec<GameExecutable>,
    /// Nome exibido (pode ser `ms-resource:`).
    pub display_name: Option<String>,
    pub publisher_display_name: Option<String>,
    pub square480_logo: Option<String>,
    pub square150_logo: Option<String>,
    pub store_logo: Option<String>,
    pub splash_screen_image: Option<String>,
    pub background_color: Option<String>,
    pub store_id: Option<String>,
    pub title_id: Option<String>,
}

impl MicrosoftGameConfig {
    pub fn family_name(&self) -> String {
        family_name(&self.identity_name, &self.publisher)
    }

    /// `None` se não for um `MicrosoftGame.config` com identidade.
    pub fn parse(xml: &str) -> Option<Self> {
        let doc = Document::parse(xml).ok()?;
        let root = doc.root_element();
        if root.tag_name().name() != "Game" {
            return None;
        }
        let identity = child(Some(root), "Identity")?;
        let visuals = child(Some(root), "ShellVisuals");
        let executables = child(Some(root), "ExecutableList")
            .map(|list| {
                list.children()
                    .filter(|e| e.is_element() && e.tag_name().name() == "Executable")
                    .filter_map(|e| {
                        Some(GameExecutable {
                            name: attr(Some(e), "Name")?,
                            id: attr(Some(e), "Id")?,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Some(MicrosoftGameConfig {
            identity_name: attr(Some(identity), "Name")?,
            publisher: attr(Some(identity), "Publisher")?,
            version: attr(Some(identity), "Version"),
            executables,
            display_name: attr(visuals, "DefaultDisplayName"),
            publisher_display_name: attr(visuals, "PublisherDisplayName"),
            square480_logo: attr(visuals, "Square480x480Logo"),
            square150_logo: attr(visuals, "Square150x150Logo"),
            store_logo: attr(visuals, "StoreLogo"),
            splash_screen_image: attr(visuals, "SplashScreenImage"),
            background_color: attr(visuals, "BackgroundColor"),
            store_id: text(child(Some(root), "StoreId")),
            title_id: text(child(Some(root), "TitleId")),
        })
    }
}

/// App declarado no `AppxManifest.xml`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PackageApplication {
    pub id: String,
    pub executable: Option<String>,
    pub display_name: Option<String>,
    pub square310_logo: Option<String>,
    pub square150_logo: Option<String>,
    pub wide310_logo: Option<String>,
    pub splash_screen: Option<String>,
    pub background_color: Option<String>,
    /// `AppListEntry="none"`: o app não aparece no menu Iniciar (serviço, ajudante).
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppxManifest {
    pub identity_name: String,
    pub publisher: String,
    pub display_name: Option<String>,
    pub logo: Option<String>,
    /// Pacote de framework (ex.: VCLibs) ou de recursos: nunca é um jogo.
    pub is_framework_or_resource: bool,
    pub applications: Vec<PackageApplication>,
}

impl AppxManifest {
    pub fn family_name(&self) -> String {
        family_name(&self.identity_name, &self.publisher)
    }

    /// App que o menu Iniciar abriria: o primeiro visível.
    pub fn main_application(&self) -> Option<&PackageApplication> {
        self.applications.iter().find(|a| !a.hidden)
    }

    pub fn parse(xml: &str) -> Option<Self> {
        let doc = Document::parse(xml).ok()?;
        let root = doc.root_element();
        if root.tag_name().name() != "Package" {
            return None;
        }
        let identity = child(Some(root), "Identity")?;
        let props = child(Some(root), "Properties");
        let flag = |name| text(child(props, name)).is_some_and(|v| v.eq_ignore_ascii_case("true"));
        let applications = child(Some(root), "Applications")
            .map(|apps| {
                apps.children()
                    .filter(|e| e.is_element() && e.tag_name().name() == "Application")
                    .filter_map(application)
                    .collect()
            })
            .unwrap_or_default();
        Some(AppxManifest {
            identity_name: attr(Some(identity), "Name")?,
            publisher: attr(Some(identity), "Publisher")?,
            display_name: text(child(props, "DisplayName")),
            logo: text(child(props, "Logo")),
            is_framework_or_resource: flag("Framework") || flag("ResourcePackage"),
            applications,
        })
    }
}

fn application(app: Node) -> Option<PackageApplication> {
    let visual = child(Some(app), "VisualElements");
    let tile = child(visual, "DefaultTile");
    Some(PackageApplication {
        id: attr(Some(app), "Id")?,
        executable: attr(Some(app), "Executable"),
        display_name: attr(visual, "DisplayName"),
        square150_logo: attr(visual, "Square150x150Logo"),
        square310_logo: attr(tile, "Square310x310Logo"),
        wide310_logo: attr(tile, "Wide310x150Logo"),
        splash_screen: attr(child(visual, "SplashScreen"), "Image"),
        background_color: attr(visual, "BackgroundColor"),
        hidden: attr(visual, "AppListEntry").is_some_and(|v| v.eq_ignore_ascii_case("none")),
    })
}

fn child<'a, 'i>(parent: Option<Node<'a, 'i>>, name: &str) -> Option<Node<'a, 'i>> {
    parent?
        .children()
        .find(|e| e.is_element() && e.tag_name().name() == name)
}

fn attr(e: Option<Node>, name: &str) -> Option<String> {
    e?.attributes()
        .find(|a| a.name() == name)
        .map(|a| a.value().trim().to_owned())
        .filter(|v| !v.is_empty())
}

fn text(e: Option<Node>) -> Option<String> {
    Some(crate::ea::inner_text(e?).trim().to_owned()).filter(|v| !v.is_empty())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::xbox::package::publisher_id;

    const CONTOSO: &str = "CN=Contoso Games, O=Contoso, L=Seattle, S=Washington, C=US";

    pub fn fixture(name: &str) -> String {
        std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/Xbox")
                .join(name),
        )
        .unwrap()
    }

    #[test]
    fn game_config_is_read_from_the_fixture() {
        let c = MicrosoftGameConfig::parse(&fixture("MicrosoftGame.config")).unwrap();
        assert_eq!(c.identity_name, "Contoso.ContosoRacing");
        assert_eq!(c.publisher, CONTOSO);
        assert_eq!(
            c.family_name(),
            format!("Contoso.ContosoRacing_{}", publisher_id(CONTOSO))
        );
        let exe = |n: &str, i: &str| GameExecutable {
            name: n.into(),
            id: i.into(),
        };
        assert_eq!(
            c.executables,
            [
                exe("gamelaunchhelper.exe", "Game"),
                exe("ContosoRacing_DX11.exe", "GameDX11")
            ]
        );
        assert_eq!(c.display_name.as_deref(), Some("Contoso Racing"));
        assert_eq!(c.square480_logo.as_deref(), Some("Square480.png"));
        assert_eq!(c.splash_screen_image.as_deref(), Some("SplashScreen.png"));
        assert_eq!(c.background_color.as_deref(), Some("#1e3a5f"));
        assert_eq!(c.store_id.as_deref(), Some("9NCONTOSO001"));
        assert_eq!(c.title_id.as_deref(), Some("1A2B3C4D"));
    }

    #[test]
    fn invalid_game_config_is_ignored() {
        for xml in [
            "",
            "<Game><ExecutableList /></Game>",
            "<Package><Identity Name=\"A\" Publisher=\"CN=A\" /></Package>",
            "<!DOCTYPE Game [<!ENTITY x \"y\">]><Game><Identity Name=\"&x;\" Publisher=\"CN=A\" /></Game>",
            "<Game><Identity Name=\"A\"",
        ] {
            assert_eq!(MicrosoftGameConfig::parse(xml), None, "{xml}");
        }
    }

    #[test]
    fn appx_main_app_is_the_first_visible_one() {
        let m = AppxManifest::parse(&fixture("AppxManifest.xml")).unwrap();
        assert_eq!(m.identity_name, "Contoso.ContosoRacing");
        assert_eq!(
            m.display_name.as_deref(),
            Some("ms-resource:AppDisplayName")
        );
        assert!(!m.is_framework_or_resource);
        let ids: Vec<_> = m.applications.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["Game", "GameDX11"]);
        assert!(m.applications[1].hidden);
        let main = m.main_application().unwrap();
        assert_eq!(main.id, "Game");
        assert_eq!(main.splash_screen.as_deref(), Some("SplashScreen.png"));

        let uwp = AppxManifest::parse(&fixture("AppxManifest-Uwp.xml")).unwrap();
        let main = uwp.main_application().unwrap();
        assert_eq!(
            main.square310_logo.as_deref(),
            Some(r"Assets\Square310x310Logo.png")
        );
        assert_eq!(
            main.wide310_logo.as_deref(),
            Some(r"Assets\Wide310x150Logo.png")
        );
    }

    #[test]
    fn framework_packages_are_flagged() {
        let m = AppxManifest::parse(
            r#"<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10">
              <Identity Name="Microsoft.VCLibs.140.00" Publisher="CN=Microsoft Corporation, O=Microsoft Corporation, L=Redmond, S=Washington, C=US" />
              <Properties><Framework>true</Framework><DisplayName>VCLibs</DisplayName></Properties>
            </Package>"#,
        )
        .unwrap();
        assert!(m.is_framework_or_resource);
        assert_eq!(m.main_application(), None);
        assert_eq!(m.family_name(), "Microsoft.VCLibs.140.00_8wekyb3d8bbwe");
    }
}
