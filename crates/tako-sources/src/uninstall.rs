//! Programas instalados do Windows (chaves `Uninstall`), port de `UninstallEntries`. Ubisoft Connect,
//! Battle.net e Amazon Games registram ali os jogos e o próprio cliente.

use crate::registry::RegistryReader;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UninstallEntry {
    pub key_path: String,
    pub key_name: String,
    pub display_name: Option<String>,
    pub publisher: Option<String>,
    pub install_location: Option<String>,
    pub uninstall_string: Option<String>,
    pub display_icon: Option<String>,
}

pub const ROOTS: [&str; 3] = [
    r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
    r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    r"HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
];

pub fn read(registry: &dyn RegistryReader) -> Vec<UninstallEntry> {
    ROOTS
        .iter()
        .flat_map(|root| {
            registry
                .sub_keys(root)
                .into_iter()
                .map(move |name| entry(registry, &format!(r"{root}\{name}"), &name))
        })
        .collect()
}

/// Uma chave pelo nome (ex.: `Uplay Install 635`), na primeira raiz em que existir.
pub fn find(registry: &dyn RegistryReader, key_name: &str) -> Option<UninstallEntry> {
    ROOTS
        .iter()
        .map(|root| entry(registry, &format!(r"{root}\{key_name}"), key_name))
        .find(|e| e.display_name.is_some() || e.install_location.is_some())
}

fn entry(registry: &dyn RegistryReader, key: &str, name: &str) -> UninstallEntry {
    let get = |v: &str| clean(registry.get_string(key, v));
    UninstallEntry {
        key_path: key.to_owned(),
        key_name: name.to_owned(),
        display_name: get("DisplayName"),
        publisher: get("Publisher"),
        install_location: get("InstallLocation").map(|s| s.trim_matches('"').to_owned()),
        uninstall_string: get("UninstallString"),
        display_icon: get("DisplayIcon"),
    }
}

/// Texto vazio vira `None`; o resto sem espaços nas pontas.
pub(crate) fn clean(value: Option<String>) -> Option<String> {
    value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
}
