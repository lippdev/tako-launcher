//! Escolha da atualização na lista de releases do GitHub (port de `ReleaseFeed.cs`, puro e testável).

use std::path::Path;

use serde::Deserialize;

use super::SemVer;
use crate::i18n::Lang;
use crate::paths::{InstallKind, Os};

pub const REPOSITORY: &str = "lippdev/tako-launcher";
pub const RELEASES_URL: &str =
    "https://api.github.com/repos/lippdev/tako-launcher/releases?per_page=15";
/// Os alphas em C# procuram por prefixo + `.exe`; os nomes do Windows não podem mudar (MIG-040).
pub const SETUP_ASSET_PREFIX: &str = "Tako-Setup";
pub const PORTABLE_ASSET_PREFIX: &str = "Tako-Portable";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X64,
    Arm64,
}

impl Arch {
    /// Arquitetura deste build; outras não têm pacote publicado.
    pub const fn current() -> Option<Arch> {
        if cfg!(target_arch = "x86_64") {
            Some(Arch::X64)
        } else if cfg!(target_arch = "aarch64") {
            Some(Arch::Arm64)
        } else {
            None
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Arch::X64 => "x64",
            Arch::Arm64 => "arm64",
        }
    }
}

/// Como o Tako foi instalado: decide o arquivo da release e o jeito de aplicar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Package {
    WindowsSetup,
    WindowsPortable,
    LinuxAppImage,
    LinuxTarGz,
    /// Atualiza pelo gerenciador de pacotes (apt, pacman, Flatpak), nunca sozinho.
    LinuxDeb,
    /// `.app` atualizado pelo `.zip`; o `.dmg` é só para instalar à mão.
    MacApp,
}

impl Package {
    /// Regra pura: `appimage` é a variável `APPIMAGE` (definida pelo runtime do AppImage); `flatpak` diz que o
    /// processo roda numa sandbox Flatpak (`FLATPAK_ID` definido ou `/.flatpak-info` presente).
    pub fn detect(
        os: Os,
        kind: InstallKind,
        exe: &Path,
        appimage: Option<&Path>,
        flatpak: bool,
    ) -> Package {
        match os {
            Os::Windows if kind == InstallKind::Installed => Package::WindowsSetup,
            Os::Windows => Package::WindowsPortable,
            Os::MacOs => Package::MacApp,
            Os::Linux if appimage.is_some() => Package::LinuxAppImage,
            // Flatpak (somente leitura; quem atualiza é o Flatpak), /usr: deb e AUR.
            Os::Linux if flatpak || exe.starts_with("/usr") => Package::LinuxDeb,
            Os::Linux => Package::LinuxTarGz,
        }
    }

    pub fn self_updates(self) -> bool {
        self != Package::LinuxDeb
    }

    /// Nome do arquivo na release (o pipeline de release já usa estes; sem versão no nome).
    /// `None` para `.deb`, que não se atualiza sozinho.
    pub fn asset_name(self, arch: Arch) -> Option<String> {
        let arch = arch.name();
        Some(match self {
            Package::WindowsSetup => format!("{SETUP_ASSET_PREFIX}-{arch}.exe"),
            Package::WindowsPortable => format!("{PORTABLE_ASSET_PREFIX}-{arch}.exe"),
            Package::LinuxAppImage => format!("Tako-Linux-{arch}.AppImage"),
            Package::LinuxTarGz => format!("Tako-Linux-{arch}.tar.gz"),
            Package::LinuxDeb => return None,
            Package::MacApp => format!("Tako-macOS-{arch}.zip"),
        })
    }

    /// O nome do arquivo da release serve para este pacote e arquitetura?
    pub fn matches_asset(self, arch: Arch, name: &str) -> bool {
        self.asset_name(arch)
            .is_some_and(|n| n.eq_ignore_ascii_case(name))
    }
}

/// Arquivo da release com o SHA-256 publicado pelo GitHub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub url: String,
    pub name: String,
    pub digest: String,
    pub size: u64,
}

/// Atualização encontrada. Sem arquivo confiável (hash), só a página é oferecida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateInfo {
    pub version: String,
    pub name: String,
    pub notes: String,
    pub page_url: String,
    pub asset: Option<Asset>,
    pub is_prerelease: bool,
}

impl UpdateInfo {
    pub fn can_install(&self) -> bool {
        self.asset.is_some()
    }
}

#[derive(Deserialize)]
struct Release {
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    tag_name: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    html_url: Option<String>,
    #[serde(default)]
    assets: Vec<ReleaseAsset>,
}

#[derive(Deserialize)]
struct ReleaseAsset {
    #[serde(default)]
    size: u64,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    browser_download_url: Option<String>,
    #[serde(default)]
    digest: Option<String>,
}

/// Que versões o usuário quer receber.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Stable,
    /// Alpha e beta.
    Testing,
    /// As builds noturnas da develop, além das de teste (PLAT-008).
    Nightly,
}

/// Versões de teste entram quando o usuário optou ou quando já roda uma; desligar nunca rebaixa.
pub fn include_prereleases(current_is_prerelease: bool, opted_in: bool) -> bool {
    current_is_prerelease || opted_in
}

/// A release entra neste canal? A noturna só para quem escolheu: quem roda alpha ou beta não a recebe sem pedir.
fn allowed(version: &SemVer, prerelease: bool, current: &SemVer, channel: Channel) -> bool {
    if version.is_nightly() {
        return channel == Channel::Nightly;
    }
    !prerelease || include_prereleases(current.is_prerelease(), channel != Channel::Stable)
}

/// Release publicada mais nova acima da atual, com o arquivo do pacote, ou `None`.
/// JSON quebrado também dá `None`: a checagem roda em segundo plano e não pode derrubar nada.
pub fn pick(
    releases_json: &str,
    current: &SemVer,
    channel: Channel,
    package: Package,
    arch: Arch,
) -> Option<UpdateInfo> {
    let releases: Vec<Release> = serde_json::from_str(releases_json).ok()?;
    let mut best: Option<(SemVer, UpdateInfo)> = None;
    for r in releases {
        if r.draft {
            continue;
        }
        let tag = r.tag_name.unwrap_or_default();
        let name = r.name.unwrap_or_default();
        let Some(version) = SemVer::find(&tag).or_else(|| SemVer::find(&name)) else {
            continue;
        };
        if !allowed(&version, r.prerelease, current, channel)
            || version <= *current
            || best.as_ref().is_some_and(|(b, _)| version <= *b)
        {
            continue;
        }
        let asset = r
            .assets
            .iter()
            .find(|a| {
                a.name
                    .as_deref()
                    .is_some_and(|n| package.matches_asset(arch, n))
            })
            .and_then(|a| {
                let digest = a.digest.clone().filter(|d| is_valid_sha256_digest(d))?;
                Some(Asset {
                    url: a.browser_download_url.clone()?,
                    name: a.name.clone()?,
                    digest,
                    size: a.size,
                })
            })
            .filter(|_| package.self_updates());
        let info = UpdateInfo {
            version: version.to_string(),
            name: if name.trim().is_empty() { tag } else { name },
            notes: r.body.unwrap_or_default(),
            page_url: r
                .html_url
                .unwrap_or_else(|| format!("https://github.com/{REPOSITORY}/releases")),
            asset,
            is_prerelease: r.prerelease,
        };
        best = Some((version, info));
    }
    best.map(|(_, info)| info)
}

/// `sha256:` + 64 hex.
pub fn is_valid_sha256_digest(digest: &str) -> bool {
    let b = digest.as_bytes();
    b.len() == 71
        && b[..7].eq_ignore_ascii_case(b"sha256:")
        && b[7..].iter().all(u8::is_ascii_hexdigit)
}

/// Changelog da tag no idioma da UI (o inglês fica em `CHANGELOG.en-US.md`).
pub fn changelog_url(version: &str, lang: Lang) -> String {
    let file = match lang {
        Lang::PtBr => "CHANGELOG.md",
        Lang::EnUs => "CHANGELOG.en-US.md",
    };
    format!("https://raw.githubusercontent.com/{REPOSITORY}/v{version}/{file}")
}

/// Seção `## [versão]` de um changelog (o mesmo recorte do workflow de release), ou `None`.
pub fn changelog_section(changelog: &str, version: &str) -> Option<String> {
    let header = format!("## [{version}]");
    let mut lines = changelog.split('\n');
    lines.find(|l| l.strip_suffix('\r').unwrap_or(l) == header)?;
    let notes: Vec<&str> = lines.take_while(|l| !l.starts_with("## ")).collect();
    Some(notes.join("\n").trim().to_owned())
}

/// Primeiro item das notas ("- …"), sem `**` e sem o "(#NN)" do fim, para o aviso de atualização.
pub fn first_note(notes: &str) -> Option<String> {
    let line = notes
        .split('\n')
        .map(str::trim)
        .find(|l| l.starts_with("- "))?;
    let line = line[2..].replace("**", "");
    Some(match line.rfind(" (#") {
        Some(pr) if pr > 0 => line[..pr].to_owned(),
        _ => line,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST: &str = "sha256:1861bbb28004cc7811dc37015e74acee00f5cf59242dcb8e64d995754ba2dea7";
    const ALL_ASSETS: &[&str] = &[
        "Tako-Setup-x64.exe",
        "Tako-Portable-x64.exe",
        "Tako-Setup-arm64.exe",
        "Tako-Portable-arm64.exe",
        "Tako-Linux-x64.AppImage",
        "Tako-Linux-x64.tar.gz",
        "Tako-Linux-x64.deb",
        "Tako-Linux-arm64.AppImage",
        "Tako-Linux-arm64.tar.gz",
        "Tako-macOS-arm64.zip",
        "Tako-macOS-arm64.dmg",
        "Tako-macOS-x64.zip",
    ];

    /// (tag, prerelease, draft, com digest)
    fn releases(list: &[(&str, bool, bool, bool)], assets: &[&str]) -> String {
        let v: Vec<_> = list
            .iter()
            .map(|(tag, pre, draft, with_digest)| {
                let assets: Vec<_> = assets
                    .iter()
                    .map(|a| {
                        serde_json::json!({
                            "name": a,
                            "browser_download_url": format!("https://example/{tag}/{a}"),
                            "digest": if *with_digest { Some(DIGEST) } else { None },
                        })
                    })
                    .collect();
                serde_json::json!({
                    "tag_name": tag,
                    "name": format!("Tako {}", tag.trim_start_matches('v')),
                    "draft": draft,
                    "prerelease": pre,
                    "html_url": format!("https://github.com/lippdev/tako-launcher/releases/tag/{tag}"),
                    "body": "### What's new\n- **Your real library**: Steam games show up by themselves. (#13)\n- Other.",
                    "assets": assets,
                })
            })
            .collect();
        serde_json::to_string(&v).unwrap()
    }

    fn feed(list: &[(&str, bool, bool, bool)]) -> String {
        releases(list, &["Tako-Setup-x64.exe", "Tako-Portable-x64.exe"])
    }

    fn pick_win(json: &str, current: &str, opted: bool, installed: bool) -> Option<UpdateInfo> {
        let p = if installed {
            Package::WindowsSetup
        } else {
            Package::WindowsPortable
        };
        let channel = if opted {
            Channel::Testing
        } else {
            Channel::Stable
        };
        pick(json, &SemVer::parse(current), channel, p, Arch::X64)
    }

    #[test]
    fn nightlies_only_reach_the_nightly_channel() {
        let f = feed(&[
            ("v0.2.1-nightly.20261017", true, false, true),
            ("v0.2.1-nightly.20261016", true, false, true),
            ("v0.2.0-beta.1", true, false, true),
            ("v0.1.0", false, false, true),
        ]);
        let pick_on = |current: &str, channel| {
            pick(
                &f,
                &SemVer::parse(current),
                channel,
                Package::WindowsSetup,
                Arch::X64,
            )
            .map(|u| u.version)
        };
        assert_eq!(pick_on("0.1.0", Channel::Stable), None);
        assert_eq!(
            pick_on("0.1.0", Channel::Testing).as_deref(),
            Some("0.2.0-beta.1")
        );
        // Quem roda uma beta continua sem noturnas até escolher.
        assert_eq!(
            pick_on("0.2.0-alpha.1", Channel::Stable).as_deref(),
            Some("0.2.0-beta.1")
        );
        assert_eq!(
            pick_on("0.1.0", Channel::Nightly).as_deref(),
            Some("0.2.1-nightly.20261017")
        );
        assert_eq!(
            pick_on("0.2.1-nightly.20261016", Channel::Nightly).as_deref(),
            Some("0.2.1-nightly.20261017")
        );
        // Saiu do canal noturno: nada mais velho é oferecido.
        assert_eq!(pick_on("0.2.1-nightly.20261016", Channel::Testing), None);
    }

    #[test]
    fn picks_the_newest_newer_release_with_the_right_file() {
        let f = feed(&[
            ("v0.1.0-alpha.7", true, false, true),
            ("v0.1.0-alpha.6", true, false, true),
            ("v0.1.0-alpha.5", true, false, true),
        ]);
        let portable = pick_win(&f, "0.1.0-alpha.5", false, false).unwrap();
        let installed = pick_win(&f, "0.1.0-alpha.5", false, true).unwrap();
        assert_eq!(portable.version, "0.1.0-alpha.7");
        assert_eq!(
            portable.asset.as_ref().unwrap().name,
            "Tako-Portable-x64.exe"
        );
        assert_eq!(installed.asset.as_ref().unwrap().name, "Tako-Setup-x64.exe");
        assert!(portable.can_install());
        assert_eq!(
            first_note(&portable.notes).as_deref(),
            Some("Your real library: Steam games show up by themselves.")
        );
    }

    #[test]
    fn stable_builds_skip_test_releases_unless_opted_in() {
        let f = feed(&[
            ("v0.2.0-beta.1", true, false, true),
            ("v0.1.1", false, false, true),
        ]);
        assert_eq!(pick_win(&f, "0.1.0", false, true).unwrap().version, "0.1.1");
        assert_eq!(
            pick_win(&f, "0.1.0", true, true).unwrap().version,
            "0.2.0-beta.1"
        );
        // Já numa versão de teste: continua recebendo testes mesmo sem optar.
        assert_eq!(
            pick_win(&f, "0.1.0-alpha.1", false, true).unwrap().version,
            "0.2.0-beta.1"
        );
    }

    #[test]
    fn up_to_date_drafts_and_older_releases_give_nothing() {
        let f = feed(&[
            ("v0.2.0", false, true, true),
            ("v0.1.0", false, false, true),
        ]);
        assert!(pick_win(&f, "0.1.0", true, true).is_none());
    }

    #[test]
    fn without_github_digest_the_update_is_only_offered_as_a_page() {
        let u = pick_win(
            &feed(&[("v0.2.0", false, false, false)]),
            "0.1.0",
            false,
            true,
        )
        .unwrap();
        assert!(!u.can_install());
        assert_eq!(
            u.page_url,
            "https://github.com/lippdev/tako-launcher/releases/tag/v0.2.0"
        );
    }

    #[test]
    fn broken_json_gives_nothing() {
        for bad in ["", "{", "{}", "[1, 2]", "[{\"draft\": \"sim\"}]", "null"] {
            assert!(pick_win(bad, "0.1.0", true, true).is_none(), "{bad}");
        }
        // Campos ausentes não derrubam: release sem assets vira só página.
        let u = pick_win(r#"[{"tag_name":"v9.0.0"}]"#, "0.1.0", false, true).unwrap();
        assert!(!u.can_install());
        assert_eq!(u.name, "v9.0.0");
    }

    #[test]
    fn chooses_the_asset_per_os_arch_and_package() {
        let f = releases(&[("v0.2.0", false, false, true)], ALL_ASSETS);
        let name = |p: Package, a: Arch| {
            pick(&f, &SemVer::parse("0.1.0"), Channel::Stable, p, a)
                .and_then(|u| u.asset)
                .map(|a| a.name)
        };
        use Arch::*;
        use Package::*;
        assert_eq!(
            name(WindowsSetup, X64).as_deref(),
            Some("Tako-Setup-x64.exe")
        );
        assert_eq!(
            name(WindowsSetup, Arm64).as_deref(),
            Some("Tako-Setup-arm64.exe")
        );
        assert_eq!(
            name(WindowsPortable, Arm64).as_deref(),
            Some("Tako-Portable-arm64.exe")
        );
        assert_eq!(
            name(LinuxAppImage, X64).as_deref(),
            Some("Tako-Linux-x64.AppImage")
        );
        assert_eq!(
            name(LinuxTarGz, Arm64).as_deref(),
            Some("Tako-Linux-arm64.tar.gz")
        );
        assert_eq!(name(LinuxDeb, X64), None);
        assert_eq!(name(MacApp, Arm64).as_deref(), Some("Tako-macOS-arm64.zip"));
        assert_eq!(name(MacApp, X64).as_deref(), Some("Tako-macOS-x64.zip"));
        // .deb: a release aparece, mas só como página (gerenciador de pacotes).
        let deb = pick(&f, &SemVer::parse("0.1.0"), Channel::Stable, LinuxDeb, X64).unwrap();
        assert!(!deb.can_install());
    }

    #[test]
    fn detects_the_package() {
        let d = |os, kind, exe: &str, app: Option<&str>| {
            Package::detect(os, kind, Path::new(exe), app.map(Path::new), false)
        };
        use InstallKind::*;
        assert_eq!(
            d(Os::Windows, Installed, "C:/P/Tako.exe", None),
            Package::WindowsSetup
        );
        assert_eq!(
            d(Os::Windows, Portable, "D:/Tako.exe", None),
            Package::WindowsPortable
        );
        assert_eq!(
            d(
                Os::Linux,
                Installed,
                "/tmp/.mount_x/tako",
                Some("/home/a/Tako.AppImage")
            ),
            Package::LinuxAppImage
        );
        assert_eq!(
            d(Os::Linux, Installed, "/usr/bin/tako", None),
            Package::LinuxDeb
        );
        // Flatpak: pela sandbox, não pelo caminho (um tar.gz extraído em /app/... continua sendo tar.gz).
        let flatpak = |exe: &str| Package::detect(Os::Linux, Installed, Path::new(exe), None, true);
        assert_eq!(flatpak("/app/bin/tako"), Package::LinuxDeb);
        assert_eq!(
            d(Os::Linux, Portable, "/app/tako", None),
            Package::LinuxTarGz
        );
        assert_eq!(
            d(Os::Linux, Portable, "/opt/tako/tako", None),
            Package::LinuxTarGz
        );
        assert_eq!(
            d(
                Os::MacOs,
                Installed,
                "/Applications/Tako.app/Contents/MacOS/tako",
                None
            ),
            Package::MacApp
        );
    }

    #[test]
    fn validates_digests() {
        assert!(is_valid_sha256_digest(DIGEST));
        assert!(is_valid_sha256_digest(&DIGEST.to_uppercase()));
        assert!(!is_valid_sha256_digest("md5:abc"));
        assert!(!is_valid_sha256_digest(&DIGEST.replace('1', "g")));
        assert!(!is_valid_sha256_digest(&DIGEST[..70]));
        assert!(!is_valid_sha256_digest(&format!(
            "sha256é{}",
            "a".repeat(63)
        )));
    }

    #[test]
    fn reads_the_version_section_of_a_changelog() {
        let changelog = "# Histórico de alterações\r\n\r\n## [Unreleased]\r\n\r\n## [0.1.0-alpha.6]\r\n### Novidades\r\n- **Atualização automática**: o Tako procura e baixa versões novas sozinho. (#15)\r\n\r\n## [0.1.0-alpha.5]\r\n### Novidades\r\n- Outra coisa.\r\n";
        let section = changelog_section(changelog, "0.1.0-alpha.6").unwrap();
        assert_eq!(
            first_note(&section).as_deref(),
            Some("Atualização automática: o Tako procura e baixa versões novas sozinho.")
        );
        assert!(!section.contains("Outra coisa"));
        assert!(changelog_section(changelog, "9.9.9").is_none());
        assert!(first_note("sem itens").is_none());
        assert!(changelog_url("0.2.0", Lang::EnUs).ends_with("/v0.2.0/CHANGELOG.en-US.md"));
        assert!(changelog_url("0.2.0", Lang::PtBr).ends_with("/v0.2.0/CHANGELOG.md"));
    }
}
