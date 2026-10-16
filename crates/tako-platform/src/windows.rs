//! Implementações para windows (PRD §5.3). Até cada função ficar pronta, usa a simulada do core.

use crate::Platform;

pub fn platform(_data_root: &std::path::Path) -> Platform {
    let mut p = Platform::simulated();
    p.secrets = std::sync::Arc::new(crate::secrets::OsSecrets);
    p
}
