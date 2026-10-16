//! Atualização automática pelas releases do GitHub (MIG-032): procura, escolhe o arquivo do pacote
//! (SO, arquitetura e tipo de instalação), baixa, confere o SHA-256 e monta o plano de aplicação.

mod apply;
mod download;
pub mod feed;
mod semver;

use std::path::Path;

use serde::{Deserialize, Serialize};

pub use apply::{UpdateApplyPlan, installer_args, mac_bundle_of, portable_swap_script};
pub use download::{
    STALL_TIMEOUT, UpdateError, agent, check, download, download_dir, fetch_highlight, verify_file,
};
pub use feed::{Arch, Asset, Package, UpdateInfo};
pub use semver::SemVer;

/// Preferências salvas em `updates.json` (mesmos nomes do C#, para a migração).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct UpdatePreferences {
    /// Procurar e baixar ao abrir.
    pub auto_update: bool,
    /// Receber versões de teste (alpha/beta) numa versão estável.
    pub prereleases: bool,
}

impl Default for UpdatePreferences {
    fn default() -> Self {
        UpdatePreferences {
            auto_update: true,
            prereleases: false,
        }
    }
}

impl UpdatePreferences {
    /// Arquivo ausente ou inválido vira o padrão.
    pub fn load(path: &Path) -> UpdatePreferences {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_round_trip_with_csharp_names() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("updates.json");
        assert_eq!(UpdatePreferences::load(&path), UpdatePreferences::default());
        std::fs::write(&path, r#"{"AutoUpdate":false,"Prereleases":true}"#).unwrap();
        let p = UpdatePreferences::load(&path);
        assert!(!p.auto_update && p.prereleases);
        p.save(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"AutoUpdate\": false") && text.contains("\"Prereleases\": true"));
        std::fs::write(&path, "{quebrado").unwrap();
        assert_eq!(UpdatePreferences::load(&path), UpdatePreferences::default());
        std::fs::write(&path, r#"{"Prereleases":true}"#).unwrap();
        assert!(UpdatePreferences::load(&path).auto_update);
    }
}
