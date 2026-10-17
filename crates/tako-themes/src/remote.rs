//! Tema pelo endereço do repositório (THEME-004): o Tako lê a última release do repositório no GitHub, baixa o
//! primeiro `.takotheme` dela e instala pelo mesmo caminho do arquivo local (validação e limites iguais). A origem
//! fica em `themes/.sources.json`, para "Atualizar temas" comparar com a release nova.
//!
//! Só a instalação usa a rede; o tema continua sendo dados (PRD §7).

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tako_core::i18n::{t, tf};

use crate::{EXTENSION, InstalledTheme, Limits, Problem, install_with, is_safe_id};

const SOURCES_FILE: &str = ".sources.json";
const MAX_SOURCES_BYTES: u64 = 256 * 1024;
const MAX_RELEASE_BYTES: u64 = 2 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(30);

/// Repositório do GitHub (`dono/nome`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    pub owner: String,
    pub name: String,
}

impl Repo {
    /// Aceita `https://github.com/dono/nome` (com `.git`, barra ou caminho a mais), `github.com/dono/nome` e
    /// `dono/nome`. Outros sites não: a release e o arquivo vêm da API do GitHub.
    pub fn parse(text: &str) -> Option<Repo> {
        let text = text.trim();
        let rest = text
            .strip_prefix("https://")
            .or_else(|| text.strip_prefix("http://"))
            .unwrap_or(text);
        let rest = rest.strip_prefix("www.").unwrap_or(rest);
        let path = match rest.split_once('/') {
            Some((host, path)) if host.eq_ignore_ascii_case("github.com") => path,
            _ if text.contains("://") => return None,
            _ => rest,
        };
        let mut parts = path.split('/').filter(|p| !p.is_empty());
        let owner = parts.next()?;
        let name = parts.next()?;
        let name = name.strip_suffix(".git").unwrap_or(name);
        let ok = |s: &str| {
            !s.is_empty()
                && s.len() <= 100
                && s != "."
                && s != ".."
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        };
        (ok(owner) && ok(name)).then(|| Repo {
            owner: owner.to_owned(),
            name: name.to_owned(),
        })
    }

    pub fn url(&self) -> String {
        format!("https://github.com/{}/{}", self.owner, self.name)
    }

    fn latest_release_api(&self) -> String {
        format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            self.owner, self.name
        )
    }
}

/// Release com um tema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub tag: String,
    pub asset_url: String,
    pub size: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum RemoteError {
    #[error("{}", t("themes.remote.bad_url"))]
    BadUrl,
    #[error("{}", t("themes.remote.no_release"))]
    NoRelease,
    #[error("{}", tf("themes.remote.no_asset", &[("extension", EXTENSION)]))]
    NoAsset,
    #[error("{}", t("themes.remote.too_large"))]
    TooLarge,
    #[error("{}", tf("themes.remote.network", &[("error", .0)]))]
    Network(String),
    #[error("{}", .0.iter().map(|p| p.message.clone()).collect::<Vec<_>>().join(" "))]
    Install(Vec<Problem>),
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl From<ureq::Error> for RemoteError {
    fn from(e: ureq::Error) -> Self {
        match e {
            ureq::Error::StatusCode(404) => RemoteError::NoRelease,
            ureq::Error::Io(io) => RemoteError::Io(io),
            other => RemoteError::Network(other.to_string()),
        }
    }
}

#[derive(Deserialize)]
struct ApiRelease {
    #[serde(default)]
    tag_name: Option<String>,
    #[serde(default)]
    assets: Vec<ApiAsset>,
}

#[derive(Deserialize)]
struct ApiAsset {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    browser_download_url: Option<String>,
    #[serde(default)]
    size: u64,
}

/// O primeiro `.takotheme` da resposta de `releases/latest`. O arquivo precisa vir do próprio GitHub.
pub fn pick_release(json: &str) -> Result<Release, RemoteError> {
    let release: ApiRelease = serde_json::from_str(json).map_err(|_| RemoteError::NoRelease)?;
    let tag = release
        .tag_name
        .filter(|t| !t.trim().is_empty())
        .ok_or(RemoteError::NoRelease)?;
    let asset = release
        .assets
        .into_iter()
        .find(|a| {
            a.name
                .as_deref()
                .is_some_and(|n| n.to_ascii_lowercase().ends_with(&format!(".{EXTENSION}")))
                && a.browser_download_url
                    .as_deref()
                    .is_some_and(|u| u.starts_with("https://github.com/"))
        })
        .ok_or(RemoteError::NoAsset)?;
    Ok(Release {
        tag: tag.trim().to_owned(),
        asset_url: asset.browser_download_url.unwrap_or_default(),
        size: asset.size,
    })
}

/// De onde veio cada tema instalado por endereço (id → repositório e versão).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub repo: String,
    pub tag: String,
}

/// Arquivo ausente, grande demais ou quebrado conta como vazio (os temas continuam instalados).
pub fn sources(themes_root: &Path) -> BTreeMap<String, Source> {
    let mut text = String::new();
    let read = fs::File::open(themes_root.join(SOURCES_FILE))
        .and_then(|f| f.take(MAX_SOURCES_BYTES).read_to_string(&mut text));
    if read.is_err() {
        return BTreeMap::new();
    }
    let mut map: BTreeMap<String, Source> = serde_json::from_str(&text).unwrap_or_default();
    map.retain(|id, s| is_safe_id(id) && Repo::parse(&s.repo).is_some());
    map
}

fn save_sources(themes_root: &Path, map: &BTreeMap<String, Source>) -> io::Result<()> {
    let text = serde_json::to_string_pretty(map).map_err(io::Error::other)?;
    fs::create_dir_all(themes_root)?;
    fs::write(themes_root.join(SOURCES_FILE), text)
}

/// Instala o pacote já baixado e lembra a origem.
pub fn install_downloaded(
    package: &Path,
    themes_root: &Path,
    repo: &Repo,
    tag: &str,
    limits: &Limits,
) -> Result<InstalledTheme, RemoteError> {
    let theme = install_with(package, themes_root, limits).map_err(RemoteError::Install)?;
    let mut map = sources(themes_root);
    map.insert(
        theme.id(),
        Source {
            repo: repo.url(),
            tag: tag.to_owned(),
        },
    );
    if let Err(e) = save_sources(themes_root, &map) {
        // O tema funciona; só não vai aparecer em "Atualizar temas".
        tracing::warn!("origem do tema {} não gravada: {e}", theme.id());
    }
    Ok(theme)
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .user_agent(format!(
            "Tako/{} (+https://github.com/lippdev/tako-launcher)",
            env!("CARGO_PKG_VERSION")
        ))
        .timeout_global(Some(TIMEOUT))
        .build()
        .into()
}

/// Última release do repositório (rede).
pub fn latest(repo: &Repo) -> Result<Release, RemoteError> {
    let json = agent()
        .get(repo.latest_release_api())
        .header("Accept", "application/vnd.github+json")
        .call()?
        .body_mut()
        .with_config()
        .limit(MAX_RELEASE_BYTES)
        .read_to_string()?;
    pick_release(&json)
}

/// Baixa e instala a release (rede). O tamanho para no limite de pacote, mesmo que a API diga outro.
pub fn install_release(
    repo: &Repo,
    release: &Release,
    themes_root: &Path,
) -> Result<InstalledTheme, RemoteError> {
    let limits = Limits::default();
    if release.size > limits.max_package_bytes {
        return Err(RemoteError::TooLarge);
    }
    let mut response = agent().get(&release.asset_url).call()?;
    let mut reader = response
        .body_mut()
        .with_config()
        .limit(limits.max_package_bytes + 1)
        .reader();
    let mut file = tempfile::Builder::new()
        .suffix(&format!(".{EXTENSION}"))
        .tempfile()?;
    let copied = io::copy(&mut reader, file.as_file_mut())?;
    if copied > limits.max_package_bytes {
        return Err(RemoteError::TooLarge);
    }
    file.as_file_mut().flush()?;
    install_downloaded(file.path(), themes_root, repo, &release.tag, &limits)
}

/// Endereço digitado → tema instalado (rede).
pub fn install_from(text: &str, themes_root: &Path) -> Result<InstalledTheme, RemoteError> {
    let repo = Repo::parse(text).ok_or(RemoteError::BadUrl)?;
    let release = latest(&repo)?;
    install_release(&repo, &release, themes_root)
}

/// Resultado de "Atualizar temas" para um tema.
#[derive(Debug)]
pub enum UpdateOutcome {
    UpToDate,
    Updated(String),
    Failed(RemoteError),
}

/// Confere cada tema instalado por endereço e instala a release nova quando a versão mudou (rede).
pub fn update_all(themes_root: &Path) -> Vec<(String, UpdateOutcome)> {
    sources(themes_root)
        .into_iter()
        .map(|(id, source)| {
            let outcome = match Repo::parse(&source.repo).ok_or(RemoteError::BadUrl) {
                Err(e) => UpdateOutcome::Failed(e),
                Ok(repo) => match latest(&repo) {
                    Err(e) => UpdateOutcome::Failed(e),
                    Ok(r) if r.tag == source.tag => UpdateOutcome::UpToDate,
                    Ok(r) => match install_release(&repo, &r, themes_root) {
                        Ok(_) => UpdateOutcome::Updated(r.tag),
                        Err(e) => UpdateOutcome::Failed(e),
                    },
                },
            };
            if let UpdateOutcome::Failed(e) = &outcome {
                tracing::warn!("atualizar o tema {id}: {e}");
            }
            (id, outcome)
        })
        .collect()
}

/// Esquece a origem de um tema removido.
pub fn forget(themes_root: &Path, id: &str) {
    let mut map = sources(themes_root);
    if map.remove(id).is_some()
        && let Err(e) = save_sources(themes_root, &map)
    {
        tracing::warn!("origem do tema {id} não apagada: {e}");
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn repository_addresses_are_parsed_and_others_refused() {
        let repo = Repo {
            owner: "autor".into(),
            name: "tema-tako".into(),
        };
        for ok in [
            "https://github.com/autor/tema-tako",
            "https://github.com/autor/tema-tako/",
            "https://github.com/autor/tema-tako.git",
            "https://www.github.com/autor/tema-tako/releases/latest",
            "github.com/autor/tema-tako",
            " autor/tema-tako ",
        ] {
            assert_eq!(Repo::parse(ok), Some(repo.clone()), "{ok}");
        }
        for bad in [
            "",
            "autor",
            "https://gitlab.com/autor/tema",
            "https://github.com/autor",
            "https://github.com/../tema",
            "autor/te ma",
            "file:///c:/temas/x",
        ] {
            assert_eq!(Repo::parse(bad), None, "{bad}");
        }
        assert_eq!(repo.url(), "https://github.com/autor/tema-tako");
    }

    #[test]
    fn the_first_theme_asset_of_the_release_is_picked() {
        let json = r#"{"tag_name":"v1.2","assets":[
            {"name":"leiame.txt","browser_download_url":"https://github.com/a/b/releases/download/v1.2/leiame.txt","size":10},
            {"name":"Neon.TAKOTHEME","browser_download_url":"https://github.com/a/b/releases/download/v1.2/Neon.TAKOTHEME","size":2048}
        ]}"#;
        assert_eq!(
            pick_release(json).unwrap(),
            Release {
                tag: "v1.2".into(),
                asset_url: "https://github.com/a/b/releases/download/v1.2/Neon.TAKOTHEME".into(),
                size: 2048,
            }
        );
        let elsewhere = r#"{"tag_name":"v1","assets":[{"name":"x.takotheme","browser_download_url":"https://exemplo.com/x.takotheme"}]}"#;
        assert!(matches!(pick_release(elsewhere), Err(RemoteError::NoAsset)));
        assert!(matches!(
            pick_release(r#"{"tag_name":"v1","assets":[]}"#),
            Err(RemoteError::NoAsset)
        ));
        for broken in ["", "{", "[]", r#"{"assets":[]}"#, r#"{"tag_name":" "}"#] {
            assert!(
                matches!(pick_release(broken), Err(RemoteError::NoRelease)),
                "{broken}"
            );
        }
    }

    #[test]
    fn sources_survive_broken_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        assert!(sources(root).is_empty());
        fs::write(root.join(SOURCES_FILE), "lixo").unwrap();
        assert!(sources(root).is_empty());
        fs::write(
            root.join(SOURCES_FILE),
            r#"{"../fora":{"repo":"a/b","tag":"v1"},"ok":{"repo":"https://github.com/a/b","tag":"v1"},"ruim":{"repo":"https://exemplo.com/a/b","tag":"v1"}}"#,
        )
        .unwrap();
        assert_eq!(sources(root).keys().collect::<Vec<_>>(), ["ok"]);
        forget(root, "ok");
        assert!(sources(root).is_empty());
    }
}
