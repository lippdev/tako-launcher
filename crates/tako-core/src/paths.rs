//! Pasta de dados do Tako (port de `Services/AppPaths.cs`, estendido para Linux e macOS).
//!
//! - Windows: com `Tako.installed` ao lado do executável → instalado (`%LOCALAPPDATA%\Tako`);
//!   sem ele → portable (`<pasta do exe>\Tako_Data`). Igual ao C#, para a migração (MIG-030).
//! - Linux e macOS: portable só se existir `Tako_Data` (ou o marcador `tako.portable`) ao lado do
//!   executável; senão, a pasta do usuário (`$XDG_DATA_HOME/tako` ou `~/Library/Application Support/Tako`).

use std::path::{Path, PathBuf};

pub const INSTALLED_MARKER: &str = "Tako.installed";
pub const PORTABLE_MARKER: &str = "tako.portable";
pub const PORTABLE_DIR: &str = "Tako_Data";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallKind {
    Installed,
    Portable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Windows,
    Linux,
    MacOs,
}

impl Os {
    pub const fn current() -> Os {
        if cfg!(target_os = "windows") {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else {
            Os::Linux
        }
    }
}

/// Onde ficam os dados e se a instalação é portable.
///
/// `machine` é a pasta da instalação; `root`, a do perfil em uso (UX-004). O perfil principal usa a própria
/// pasta da instalação, então quem tinha dados antes dos perfis continua com eles no mesmo lugar. Do perfil:
/// banco (biblioteca, favoritos, tempo de jogo, configurações), imagens escolhidas e temas. Da máquina: logs,
/// caches, sessão de jogo e atualizações.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataPaths {
    pub kind: InstallKind,
    pub root: PathBuf,
    pub machine: PathBuf,
}

/// Pasta dos perfis além do principal.
pub const PROFILES_DIR: &str = "profiles";

impl DataPaths {
    fn new(kind: InstallKind, root: PathBuf) -> DataPaths {
        DataPaths {
            kind,
            machine: root.clone(),
            root,
        }
    }

    /// Resolve para o executável atual e o ambiente do processo.
    pub fn detect() -> DataPaths {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."));
        resolve(
            Os::current(),
            &exe_dir,
            |k| std::env::var_os(k).map(PathBuf::from),
            |p| p.exists(),
        )
    }

    /// Para testes e para `--data-dir`.
    pub fn at(root: impl Into<PathBuf>) -> DataPaths {
        DataPaths::new(InstallKind::Portable, root.into())
    }

    /// As mesmas pastas da máquina com os dados do perfil `dir` (`None` = o principal). `dir` vem de
    /// `profiles::Profile::dir`, já validado (só letras, números e `-`).
    pub fn for_profile(&self, dir: Option<&str>) -> DataPaths {
        DataPaths {
            kind: self.kind,
            root: match dir {
                Some(d) => self.machine.join(PROFILES_DIR).join(d),
                None => self.machine.clone(),
            },
            machine: self.machine.clone(),
        }
    }

    pub fn database(&self) -> PathBuf {
        self.root.join("tako.db")
    }
    pub fn art_choices(&self) -> PathBuf {
        self.root.join("art-choices")
    }
    pub fn themes(&self) -> PathBuf {
        self.root.join("themes")
    }
    pub fn logs(&self) -> PathBuf {
        self.machine.join("logs")
    }
    pub fn cache(&self) -> PathBuf {
        self.machine.join("cache")
    }
    pub fn art_cache(&self) -> PathBuf {
        self.cache().join("art")
    }
    pub fn metadata_cache(&self) -> PathBuf {
        self.cache().join("metadata")
    }
    pub fn session_journal(&self) -> PathBuf {
        self.machine.join("session.json")
    }
    pub fn session_environment(&self) -> PathBuf {
        self.machine.join("session-environment.json")
    }
    pub fn cursor_marker(&self) -> PathBuf {
        self.machine.join("cursor-hidden")
    }
    pub fn updates(&self) -> PathBuf {
        self.machine.join("updates.json")
    }
    /// Pasta padrão do prefixo do Wine/Proton para este jogo (SESSION-005).
    pub fn wine_prefix(&self, game_id: &str) -> PathBuf {
        let safe = game_id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>();
        self.root.join("prefixes").join(safe)
    }
}

/// Regra pura, testável: `env` lê variáveis de ambiente, `exists` testa caminhos.
pub fn resolve(
    os: Os,
    exe_dir: &Path,
    env: impl Fn(&str) -> Option<PathBuf>,
    exists: impl Fn(&Path) -> bool,
) -> DataPaths {
    let portable = DataPaths::new(InstallKind::Portable, exe_dir.join(PORTABLE_DIR));
    match os {
        Os::Windows => {
            if !exists(&exe_dir.join(INSTALLED_MARKER)) {
                return portable;
            }
            let base = env("LOCALAPPDATA")
                .or_else(|| env("USERPROFILE").map(|h| h.join("AppData").join("Local")))
                .unwrap_or_else(|| exe_dir.to_path_buf());
            DataPaths::new(InstallKind::Installed, base.join("Tako"))
        }
        Os::Linux | Os::MacOs => {
            if exists(&exe_dir.join(PORTABLE_DIR)) || exists(&exe_dir.join(PORTABLE_MARKER)) {
                return portable;
            }
            let home = env("HOME").unwrap_or_else(|| exe_dir.to_path_buf());
            let root = if os == Os::MacOs {
                home.join("Library")
                    .join("Application Support")
                    .join("Tako")
            } else {
                env("XDG_DATA_HOME")
                    .filter(|p| p.has_root())
                    .unwrap_or_else(|| home.join(".local").join("share"))
                    .join("tako")
            };
            DataPaths::new(InstallKind::Installed, root)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<PathBuf> + 'a {
        move |k| {
            vars.iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| PathBuf::from(v))
        }
    }

    #[test]
    fn windows_without_marker_is_portable() {
        let p = resolve(Os::Windows, Path::new("C:/Games/Tako"), env(&[]), |_| false);
        assert_eq!(p.kind, InstallKind::Portable);
        assert_eq!(p.root, Path::new("C:/Games/Tako").join("Tako_Data"));
    }

    #[test]
    fn windows_with_marker_uses_local_app_data() {
        let p = resolve(
            Os::Windows,
            Path::new("C:/P/Tako"),
            env(&[("LOCALAPPDATA", "C:/U/AppData/Local")]),
            |p| p.ends_with(INSTALLED_MARKER),
        );
        assert_eq!(p.kind, InstallKind::Installed);
        assert_eq!(p.root, Path::new("C:/U/AppData/Local").join("Tako"));
    }

    #[test]
    fn linux_defaults_to_xdg() {
        let p = resolve(
            Os::Linux,
            Path::new("/usr/bin"),
            env(&[("HOME", "/home/a")]),
            |_| false,
        );
        assert_eq!(p.root, Path::new("/home/a/.local/share/tako"));
        let p = resolve(
            Os::Linux,
            Path::new("/usr/bin"),
            env(&[("HOME", "/home/a"), ("XDG_DATA_HOME", "/x")]),
            |_| false,
        );
        assert_eq!(p.root, Path::new("/x/tako"));
    }

    #[test]
    fn linux_portable_when_data_dir_exists() {
        let p = resolve(
            Os::Linux,
            Path::new("/opt/tako"),
            env(&[("HOME", "/h")]),
            |p| p.ends_with(PORTABLE_DIR),
        );
        assert_eq!(p.kind, InstallKind::Portable);
    }

    #[test]
    fn macos_uses_application_support() {
        let p = resolve(
            Os::MacOs,
            Path::new("/Applications/Tako.app/Contents/MacOS"),
            env(&[("HOME", "/Users/a")]),
            |_| false,
        );
        assert_eq!(
            p.root,
            Path::new("/Users/a/Library/Application Support/Tako")
        );
    }

    #[test]
    fn wine_prefix_sanitizes_game_id() {
        let p = DataPaths::at("/home/user/.local/share/tako");
        assert_eq!(
            p.wine_prefix("manual:game-1/v2"),
            PathBuf::from("/home/user/.local/share/tako/prefixes/manual_game-1_v2")
        );
    }
}
