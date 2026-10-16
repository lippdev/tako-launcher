//! App Xbox / Microsoft Store (port de `Library/Xbox`): pastas de jogos do app Xbox (`.GamingRoot` de
//! cada unidade, `<pasta>\<jogo>\Content\MicrosoftGame.config`) e pacotes do usuário que são jogos.
//! Só lê arquivos; o jogo abre pelo AUMID (`shell:AppsFolder\Família!App`), nunca pelo executável.

pub mod manifests;
pub mod package;
pub mod rules;
#[cfg(windows)]
pub mod windows;

use crate::{LibraryScan, LibrarySource};
use chrono::{DateTime, Utc};
use manifests::{APPX_MANIFEST_FILE, AppxManifest, GAME_CONFIG_FILE, MicrosoftGameConfig};
use rules::XboxAvailability;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use tako_core::library::{Game, GameStore};

/// Pacote registrado para o usuário: nome completo e pasta de instalação.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledPackage {
    pub full_name: String,
    pub folder: PathBuf,
}

/// O que a fonte Xbox precisa do Windows; os testes usam uma máquina falsa com pastas de exemplo.
pub trait XboxSystem: Send + Sync {
    /// Raízes das unidades fixas (ex.: `C:\`, `D:\`).
    fn drive_roots(&self) -> Vec<PathBuf>;
    /// Pacotes MSIX/AppX do usuário atual; `None` quando não foi possível ler.
    fn user_packages(&self) -> Option<Vec<InstalledPackage>>;
    /// Pacotes que os Serviços de Jogos registram como jogos (nomes completos ou famílias).
    fn gaming_services_packages(&self) -> Vec<String>;
    fn gaming_services_installed(&self) -> bool;
    /// Resolve um texto indireto (`@{Pacote?ms-resource://…}`); `None` se falhar.
    fn load_indirect_string(&self, indirect: &str) -> Option<String>;
}

/// Jogo do app Xbox / Microsoft Store encontrado no disco.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XboxInstalledGame {
    pub family_name: String,
    /// Id do app no AUMID (`Família!AppId`).
    pub app_id: String,
    pub title: String,
    /// Pasta do pacote (ex.: `C:\XboxGames\Jogo\Content` ou `WindowsApps\…`).
    pub folder: String,
    pub cover_path: Option<PathBuf>,
    pub hero_path: Option<PathBuf>,
    pub accent: String,
    pub store_id: Option<String>,
    pub installed_at: Option<DateTime<Utc>>,
}

impl XboxInstalledGame {
    pub fn launch_target(&self) -> String {
        package::launch_target(&self.family_name, &self.app_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XboxScanResult {
    pub games: Vec<XboxInstalledGame>,
    pub availability: XboxAvailability,
}

/// `file:///…` de um caminho local (barras normais; espaço e `%#?` escapados).
pub fn file_uri(path: &Path) -> String {
    let p = path.to_string_lossy().replace('\\', "/");
    let mut out = String::from("file://");
    if !p.starts_with('/') {
        out.push('/');
    }
    for c in p.chars() {
        match c {
            ' ' => out.push_str("%20"),
            '%' => out.push_str("%25"),
            '#' => out.push_str("%23"),
            '?' => out.push_str("%3F"),
            c => out.push(c),
        }
    }
    out
}

pub fn to_game(g: &XboxInstalledGame) -> Game {
    let cover = g.cover_path.as_deref().map(file_uri);
    Game {
        id: format!("xbox:{}", g.family_name),
        title: g.title.clone(),
        store: Some(GameStore::Xbox),
        accent: g.accent.clone(),
        // Logos do pacote são só o plano B: a arte casada pelos metadados tem prioridade.
        hero_uri: g
            .hero_path
            .as_deref()
            .map(file_uri)
            .or_else(|| cover.clone()),
        has_fallback_art: cover.is_some(),
        cover_uri: cover,
        is_installed: true,
        added_at: g.installed_at,
        // Sempre pelo AUMID: pacotes protegidos não abrem pelo executável.
        launch_target: Some(g.launch_target()),
        // O AUMID não devolve processo: o acompanhamento usa os processos dentro da pasta do pacote.
        install_path: Some(g.folder.clone()),
        store_product_id: g.store_id.clone(),
        ..Default::default()
    }
}

struct Context<'a> {
    system: &'a dyn XboxSystem,
    /// Família em minúsculas → pacote; `None` quando a lista não pôde ser lida.
    registered: Option<HashMap<String, InstalledPackage>>,
    gaming_services: BTreeSet<String>,
}

/// Procura os jogos sem contornar o pacote.
pub fn scan(system: &dyn XboxSystem) -> XboxScanResult {
    let packages = system.user_packages();
    let mut registered: Vec<(String, InstalledPackage)> = Vec::new();
    for p in packages.iter().flatten() {
        if let Some(family) = package::family_from_full_name(&p.full_name)
            && !registered
                .iter()
                .any(|(f, _)| f.eq_ignore_ascii_case(&family))
        {
            registered.push((family, p.clone()));
        }
    }
    let services = system.gaming_services_packages();
    let ctx = Context {
        system,
        registered: packages.is_some().then(|| {
            registered
                .iter()
                .map(|(f, p)| (f.to_lowercase(), p.clone()))
                .collect()
        }),
        gaming_services: rules::families(services.iter().map(String::as_str)),
    };
    let mut games: Vec<XboxInstalledGame> = Vec::new();
    let mut add = |g: XboxInstalledGame| {
        if !games
            .iter()
            .any(|x| x.family_name.eq_ignore_ascii_case(&g.family_name))
        {
            games.push(g);
        }
    };

    // 1. Pastas de jogos do app Xbox em cada unidade.
    for drive in system.drive_roots() {
        // Sem acesso ao arquivo: a pasta padrão.
        let data = crate::read_limited(&drive.join(package::GAMING_ROOT_FILE), 64 * 1024)
            .unwrap_or_default();
        for games_folder in package::gaming_folders(&drive, &data) {
            for entry in subfolders(&games_folder) {
                let content = entry.join("Content");
                let folder = if content.is_dir() { content } else { entry };
                if folder.join(GAME_CONFIG_FILE).is_file()
                    && let Some(g) = read(&ctx, &folder, None)
                {
                    add(g);
                }
            }
        }
    }

    // 2. Pacotes do usuário (jogos em WindowsApps ou em pastas fora do .GamingRoot).
    for (family, p) in &registered {
        if !rules::is_system_package(family)
            && p.folder.is_dir()
            && let Some(g) = read(&ctx, &p.folder, Some(&p.full_name))
        {
            add(g);
        }
    }

    let availability = rules::availability(
        packages
            .is_some()
            .then(|| registered.iter().map(|(f, _)| f.as_str())),
        system.gaming_services_installed(),
    );
    games.sort_by_cached_key(|g| crate::text::fold(&g.title));
    XboxScanResult {
        games,
        availability,
    }
}

/// Um pacote; `None` quando não é jogo, não está registrado para o usuário ou não dá para abrir.
fn read(ctx: &Context, folder: &Path, full_name: Option<&str>) -> Option<XboxInstalledGame> {
    let config = read_text(folder, GAME_CONFIG_FILE).and_then(|x| MicrosoftGameConfig::parse(&x));
    let manifest = read_text(folder, APPX_MANIFEST_FILE).and_then(|x| AppxManifest::parse(&x));
    let family = manifest
        .as_ref()
        .map(AppxManifest::family_name)
        .or_else(|| config.as_ref().map(MicrosoftGameConfig::family_name))
        .or_else(|| package::family_from_full_name(full_name?))?;

    // Pasta sem pacote registrado (outro usuário, instalação incompleta ou removida): o Windows não abriria.
    let package = match &ctx.registered {
        Some(r) => Some(r.get(&family.to_lowercase())?),
        None => None,
    };
    let full_name = full_name.or(package.map(|p| p.full_name.as_str()));
    let has_xbox_services = folder.join(rules::XBOX_SERVICES_CONFIG).is_file();
    if !rules::is_game(
        &family,
        config.is_some(),
        has_xbox_services,
        ctx.gaming_services.contains(&family.to_lowercase()),
        manifest.as_ref(),
    ) {
        return None;
    }

    let app = manifest.as_ref().and_then(AppxManifest::main_application);
    let app_id = app
        .map(|a| a.id.clone())
        .or_else(|| config.as_ref()?.executables.first().map(|e| e.id.clone()))?;
    let package_name = package::name_of(&family).to_owned();
    let title = [
        config.as_ref().and_then(|c| c.display_name.as_deref()),
        manifest.as_ref().and_then(|m| m.display_name.as_deref()),
        app.and_then(|a| a.display_name.as_deref()),
    ]
    .into_iter()
    .find_map(|v| text(ctx.system, full_name, &package_name, v));
    let game_folder_name = folder
        .file_name()
        .filter(|n| n.to_string_lossy().eq_ignore_ascii_case("Content"))
        .and_then(|_| folder.parent()?.file_name())
        .map(|n| n.to_string_lossy().into_owned());
    let c = config.as_ref();
    let first_asset = |declared: &[Option<&String>]| {
        declared
            .iter()
            .find_map(|d| package::resolve_asset(folder, d.map(String::as_str)))
    };
    Some(XboxInstalledGame {
        title: title.unwrap_or_else(|| rules::fallback_title(&family, game_folder_name.as_deref())),
        app_id,
        folder: folder.to_string_lossy().into_owned(),
        cover_path: first_asset(&[
            c.and_then(|c| c.square480_logo.as_ref()),
            app.and_then(|a| a.square310_logo.as_ref()),
            c.and_then(|c| c.square150_logo.as_ref()),
            app.and_then(|a| a.square150_logo.as_ref()),
            c.and_then(|c| c.store_logo.as_ref()),
            manifest.as_ref().and_then(|m| m.logo.as_ref()),
        ]),
        hero_path: first_asset(&[
            c.and_then(|c| c.splash_screen_image.as_ref()),
            app.and_then(|a| a.splash_screen.as_ref()),
            app.and_then(|a| a.wide310_logo.as_ref()),
        ]),
        accent: rules::accent(&[
            c.and_then(|c| c.background_color.as_deref()),
            app.and_then(|a| a.background_color.as_deref()),
        ]),
        store_id: c.and_then(|c| c.store_id.clone()),
        installed_at: crate::install_dates::of(folder),
        family_name: family,
    })
}

/// Texto do manifesto; `ms-resource:` é resolvido pelo Windows (sem nome completo, fica de fora).
fn text(
    system: &dyn XboxSystem,
    full_name: Option<&str>,
    package_name: &str,
    value: Option<&str>,
) -> Option<String> {
    let value = value.filter(|v| !v.trim().is_empty())?;
    let Some(indirect) =
        package::indirect_string(full_name.unwrap_or(""), package_name, Some(value))
    else {
        return Some(value.trim().to_owned());
    };
    full_name?;
    system
        .load_indirect_string(&indirect)
        .map(|r| r.trim().to_owned())
        .filter(|r| !r.is_empty() && !r.to_ascii_lowercase().starts_with("ms-resource:"))
}

fn subfolders(folder: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(folder)
        .map(|d| {
            d.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect()
        })
        .unwrap_or_default()
}

fn read_text(folder: &Path, name: &str) -> Option<String> {
    crate::read_limited(&folder.join(name), crate::MAX_STORE_FILE)
        .ok()
        .map(|b| crate::text_of(&b))
}

/// Fonte do app Xbox: "cliente instalado" = app Xbox e Serviços de Jogos presentes.
pub struct XboxLibrarySource {
    system: Box<dyn XboxSystem>,
}

impl XboxLibrarySource {
    pub fn new(system: Box<dyn XboxSystem>) -> Self {
        XboxLibrarySource { system }
    }
}

impl LibrarySource for XboxLibrarySource {
    fn store(&self) -> GameStore {
        GameStore::Xbox
    }

    fn scan(&self) -> LibraryScan {
        let r = scan(self.system.as_ref());
        LibraryScan {
            games: r.games.iter().map(to_game).collect(),
            client_installed: r.availability == XboxAvailability::Ready,
            notice: rules::notice(r.availability),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::manifests::tests::fixture;
    use super::package::{family_name, publisher_id};
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    const CONTOSO: &str = "CN=Contoso Games, O=Contoso, L=Seattle, S=Washington, C=US";
    const FABRIKAM: &str = "CN=Fabrikam, O=Fabrikam, C=US";

    fn contoso_family() -> String {
        family_name("Contoso.ContosoRacing", CONTOSO)
    }

    fn contoso_full_name() -> String {
        format!(
            "Contoso.ContosoRacing_1.4.2.0_x64__{}",
            publisher_id(CONTOSO)
        )
    }

    #[derive(Default)]
    struct FakeSystem {
        drives: Vec<PathBuf>,
        packages: Mutex<Option<Vec<InstalledPackage>>>,
        gaming_services: Vec<String>,
        resources: Mutex<HashMap<String, String>>,
        gaming_services_installed: bool,
    }

    impl XboxSystem for FakeSystem {
        fn drive_roots(&self) -> Vec<PathBuf> {
            self.drives.clone()
        }
        fn user_packages(&self) -> Option<Vec<InstalledPackage>> {
            self.packages.lock().unwrap().clone()
        }
        fn gaming_services_packages(&self) -> Vec<String> {
            self.gaming_services.clone()
        }
        fn gaming_services_installed(&self) -> bool {
            self.gaming_services_installed
        }
        fn load_indirect_string(&self, indirect: &str) -> Option<String> {
            self.resources.lock().unwrap().get(indirect).cloned()
        }
    }

    fn touch(folder: &Path, rel: &str) {
        let p = folder.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, b"").unwrap();
    }

    fn copy(fixture_name: &str, folder: &Path, name: &str) {
        fs::create_dir_all(folder).unwrap();
        fs::write(folder.join(name), fixture(fixture_name)).unwrap();
    }

    /// Unidade D com `.GamingRoot` e um jogo do GDK, um jogo UWP antigo e um app comum em
    /// WindowsApps, o app Xbox e uma pasta de jogo sem pacote registrado.
    fn build_machine(root: &Path) -> FakeSystem {
        let drive = root.join("D");
        fs::create_dir_all(&drive).unwrap();
        fs::write(
            drive.join(package::GAMING_ROOT_FILE),
            package::tests::gaming_root_bytes(&["XboxGames"]),
        )
        .unwrap();

        let racing = drive.join("XboxGames/Contoso Racing/Content");
        copy("MicrosoftGame.config", &racing, GAME_CONFIG_FILE);
        copy("AppxManifest.xml", &racing, APPX_MANIFEST_FILE);
        for f in [
            "Square480.scale-100.png",
            "Square480.scale-200.png",
            "SplashScreen.scale-100.png",
            "gamelaunchhelper.exe",
        ] {
            touch(&racing, f);
        }

        let orphan = drive.join("XboxGames/Orphan/Content");
        fs::create_dir_all(&orphan).unwrap();
        fs::write(
            orphan.join(GAME_CONFIG_FILE),
            fixture("MicrosoftGame.config").replace("Contoso.ContosoRacing", "Contoso.Orphan"),
        )
        .unwrap();

        let apps = root.join("WindowsApps");
        let puzzle = apps.join("Fabrikam.PuzzleQuest_3.1.0.0_x64__x");
        copy("AppxManifest-Uwp.xml", &puzzle, APPX_MANIFEST_FILE);
        touch(&puzzle, rules::XBOX_SERVICES_CONFIG);
        touch(&puzzle, "Assets/Square310x310Logo.scale-100.png");
        let notes = apps.join("Fabrikam.NotesApp_2.0.0.0_x64__x");
        copy("AppxManifest-App.xml", &notes, APPX_MANIFEST_FILE);
        let xbox_app = apps.join("Microsoft.GamingApp");
        touch(&xbox_app, rules::XBOX_SERVICES_CONFIG);

        let fab = publisher_id(FABRIKAM);
        let pkg = |full: String, folder: PathBuf| InstalledPackage {
            full_name: full,
            folder,
        };
        FakeSystem {
            drives: vec![drive],
            packages: Mutex::new(Some(vec![
                pkg(contoso_full_name(), racing),
                pkg(format!("Fabrikam.PuzzleQuest_3.1.0.0_x64__{fab}"), puzzle),
                pkg(format!("Fabrikam.NotesApp_2.0.0.0_x64__{fab}"), notes),
                pkg(
                    "Microsoft.GamingApp_2509.1001.27.0_x64__8wekyb3d8bbwe".into(),
                    xbox_app,
                ),
                pkg(
                    format!("Fabrikam.Removed_1.0.0.0_x64__{fab}"),
                    apps.join("nao-existe"),
                ),
            ])),
            gaming_services: vec![contoso_full_name()],
            gaming_services_installed: true,
            ..Default::default()
        }
    }

    #[test]
    fn scanner_finds_gdk_and_uwp_games_and_skips_apps() {
        let dir = tempfile::tempdir().unwrap();
        let r = scan(&build_machine(dir.path()));

        assert_eq!(r.availability, XboxAvailability::Ready);
        let titles: Vec<_> = r.games.iter().map(|g| g.title.as_str()).collect();
        assert_eq!(titles, ["Contoso Racing", "Fabrikam Puzzle Quest"]);

        let racing = &r.games[0];
        assert_eq!(racing.family_name, contoso_family());
        assert_eq!(racing.app_id, "Game");
        assert_eq!(
            racing.launch_target(),
            format!(r"shell:AppsFolder\{}!Game", contoso_family())
        );
        assert_eq!(
            PathBuf::from(&racing.folder),
            dir.path().join("D/XboxGames/Contoso Racing/Content")
        );
        assert!(
            racing
                .cover_path
                .as_ref()
                .unwrap()
                .ends_with("Square480.scale-200.png")
        );
        assert!(
            racing
                .hero_path
                .as_ref()
                .unwrap()
                .ends_with("SplashScreen.scale-100.png")
        );
        assert_eq!(racing.accent, "#1E3A5F");
        assert_eq!(racing.store_id.as_deref(), Some("9NCONTOSO001"));

        let puzzle = &r.games[1];
        assert_eq!(puzzle.app_id, "App");
        assert!(
            puzzle
                .cover_path
                .as_ref()
                .unwrap()
                .ends_with("Square310x310Logo.scale-100.png")
        );
        assert_eq!(puzzle.hero_path, None);
        assert_eq!(puzzle.accent, rules::BRAND_COLOR);
    }

    #[test]
    fn scanner_resolves_resource_titles_and_skips_unregistered_folders() {
        let dir = tempfile::tempdir().unwrap();
        let system = build_machine(dir.path());
        let config = dir
            .path()
            .join("D/XboxGames/Contoso Racing/Content")
            .join(GAME_CONFIG_FILE);
        let text = fs::read_to_string(&config).unwrap().replace(
            "DefaultDisplayName=\"Contoso Racing\"",
            "DefaultDisplayName=\"ms-resource:GameTitle\"",
        );
        fs::write(&config, text).unwrap();
        system.resources.lock().unwrap().insert(
            format!(
                "@{{{}?ms-resource://Contoso.ContosoRacing/resources/GameTitle}}",
                contoso_full_name()
            ),
            "Corrida Contoso".into(),
        );
        let titles = |s: &FakeSystem| {
            scan(s)
                .games
                .into_iter()
                .map(|g| g.title)
                .collect::<Vec<_>>()
        };
        assert!(titles(&system).contains(&"Corrida Contoso".to_owned()));

        // Sem o recurso: o próximo nome do manifesto também é ms-resource; fica o nome da pasta.
        system.resources.lock().unwrap().clear();
        assert!(titles(&system).contains(&"Contoso Racing".to_owned()));

        // Pacote desinstalado para este usuário: a pasta existe, mas o Windows não abriria o jogo.
        if let Some(p) = system.packages.lock().unwrap().as_mut() {
            p.retain(|p| p.full_name != contoso_full_name());
        }
        assert!(
            !scan(&system)
                .games
                .iter()
                .any(|g| g.family_name == contoso_family())
        );
    }

    #[test]
    fn scanner_without_package_list_still_reads_game_folders() {
        let dir = tempfile::tempdir().unwrap();
        let system = build_machine(dir.path());
        *system.packages.lock().unwrap() = None;

        let r = scan(&system);

        assert_eq!(r.availability, XboxAvailability::Unknown);
        let mut families: Vec<_> = r.games.into_iter().map(|g| g.family_name).collect();
        families.sort();
        assert_eq!(
            families,
            [contoso_family(), family_name("Contoso.Orphan", CONTOSO)]
        );
    }

    #[test]
    fn scanner_survives_a_failing_system() {
        let r = scan(&FakeSystem::default());
        assert!(r.games.is_empty());
        let none = FakeSystem {
            packages: Mutex::new(None),
            ..Default::default()
        };
        assert_eq!(scan(&none).availability, XboxAvailability::Unknown);
    }

    #[test]
    fn games_open_by_aumid_and_keep_the_package_folder() {
        let game = to_game(&XboxInstalledGame {
            family_name: contoso_family(),
            app_id: "Game".into(),
            title: "Contoso Racing".into(),
            folder: r"C:\XboxGames\Contoso Racing\Content".into(),
            cover_path: Some(PathBuf::from(
                r"C:\XboxGames\Contoso Racing\Content\cover.png",
            )),
            hero_path: None,
            accent: "#1E3A5F".into(),
            store_id: None,
            installed_at: None,
        });
        assert_eq!(game.id, format!("xbox:{}", contoso_family()));
        assert_eq!(game.store, Some(GameStore::Xbox));
        assert_eq!(
            game.launch_target,
            Some(format!(r"shell:AppsFolder\{}!Game", contoso_family()))
        );
        assert!(game.can_play());
        assert!(game.has_fallback_art);
        assert_eq!(game.cover_uri, game.hero_uri);
        assert_eq!(
            game.cover_uri.as_deref(),
            Some("file:///C:/XboxGames/Contoso%20Racing/Content/cover.png")
        );
        assert_eq!(
            game.install_path.as_deref(),
            Some(r"C:\XboxGames\Contoso Racing\Content")
        );
    }

    #[test]
    fn source_reports_missing_xbox_app_or_gaming_services() {
        let cases = [
            (false, false, false, Some("App Xbox não encontrado")),
            (
                true,
                false,
                false,
                Some("Serviços de Jogos não encontrados"),
            ),
            (true, true, true, None),
        ];
        for (xbox_app, services, client, notice) in cases {
            let packages = if xbox_app {
                vec![InstalledPackage {
                    full_name: "Microsoft.GamingApp_2510.1001.1.0_x64__8wekyb3d8bbwe".into(),
                    folder: "/nao-existe".into(),
                }]
            } else {
                vec![]
            };
            let system = FakeSystem {
                packages: Mutex::new(Some(packages)),
                gaming_services_installed: services,
                ..Default::default()
            };
            let scan = XboxLibrarySource::new(Box::new(system)).scan();
            assert!(scan.games.is_empty());
            assert_eq!(scan.client_installed, client);
            match notice {
                None => assert_eq!(scan.notice, None),
                Some(n) => assert!(scan.notice.unwrap().starts_with(n)),
            }
        }
    }
}
