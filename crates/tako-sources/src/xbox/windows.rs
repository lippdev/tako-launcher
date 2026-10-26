//! [`XboxSystem`] de verdade (port de `Services/WindowsXboxSystem.cs`), sem administrador e sem
//! `unsafe`: pacotes do usuário pelo `PackageManager` do WinRT, jogos dos Serviços de Jogos pelo
//! registro (HKLM, só leitura).

use super::{InstalledPackage, XboxSystem};
use crate::registry::RegistryReader;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use windows::Management::Deployment::PackageManager;
use windows::core::HSTRING;

const GAMING_SERVICES_REPOSITORY: &str =
    r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\GamingServices\PackageRepository";
const GAMING_SERVICES_SERVICE: &str =
    r"HKEY_LOCAL_MACHINE\SYSTEM\CurrentControlSet\Services\GamingServices";

pub struct WindowsXboxSystem {
    registry: Arc<dyn RegistryReader>,
    /// Nome exibido de cada pacote (nome completo em minúsculas), já traduzido pelo Windows.
    display_names: Mutex<HashMap<String, String>>,
}

impl WindowsXboxSystem {
    pub fn new(registry: Arc<dyn RegistryReader>) -> Self {
        WindowsXboxSystem {
            registry,
            display_names: Mutex::new(HashMap::new()),
        }
    }
}

impl XboxSystem for WindowsXboxSystem {
    // shortcut: toda letra com raiz acessível, inclusive removível e de rede (`GetDriveTypeW` exige `unsafe`); mover para o tako-platform se uma unidade de rede lenta atrasar a leitura.
    fn drive_roots(&self) -> Vec<PathBuf> {
        ('C'..='Z')
            .map(|c| PathBuf::from(format!("{c}:\\")))
            .filter(|p| p.is_dir())
            .collect()
    }

    fn user_packages(&self) -> Option<Vec<InstalledPackage>> {
        // Usuário vazio = o usuário atual (não precisa de administrador).
        let packages = PackageManager::new()
            .ok()?
            .FindPackagesByUserSecurityId(&HSTRING::new())
            .ok()?;
        let mut names = self.display_names.lock().unwrap_or_else(|e| e.into_inner());
        let mut out = Vec::new();
        for p in packages {
            let Ok(full_name) = p.Id().and_then(|id| id.FullName()) else {
                continue;
            };
            let Ok(folder) = p.InstalledPath() else {
                continue; // pacote sendo instalado ou removido
            };
            let full_name = full_name.to_string();
            if let Ok(name) = p.DisplayName() {
                names.insert(full_name.to_lowercase(), name.to_string());
            }
            out.push(InstalledPackage {
                full_name,
                folder: PathBuf::from(folder.to_string()),
            });
        }
        Some(out)
    }

    fn gaming_services_packages(&self) -> Vec<String> {
        // PackageRepository\Package\<nome completo> e PackageRepository\Root\<guid>\<nome completo>.
        let mut names = self
            .registry
            .sub_keys(&format!(r"{GAMING_SERVICES_REPOSITORY}\Package"));
        let roots = format!(r"{GAMING_SERVICES_REPOSITORY}\Root");
        for root in self.registry.sub_keys(&roots) {
            names.extend(self.registry.sub_keys(&format!(r"{roots}\{root}")));
        }
        names
    }

    fn gaming_services_installed(&self) -> bool {
        // Todo serviço registrado tem o valor `Type`.
        self.registry
            .get_string(GAMING_SERVICES_SERVICE, "Type")
            .is_some()
    }

    // shortcut: devolve o `Package.DisplayName` do pacote para qualquer `ms-resource:` dele (o nome do jogo é o caso real); resolver a chave exata exige `SHLoadIndirectString` (`unsafe`, tako-platform).
    fn load_indirect_string(&self, indirect: &str) -> Option<String> {
        let full_name = indirect.strip_prefix("@{")?.split('?').next()?;
        self.display_names
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&full_name.to_lowercase())
            .cloned()
    }
}
