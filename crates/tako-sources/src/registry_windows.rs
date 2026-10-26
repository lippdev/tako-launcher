//! Registro do Windows de verdade (port de `WindowsRegistryReader`): visão de 64 bits, só leitura;
//! chave ausente ou sem permissão = vazio.

use crate::registry::RegistryReader;
use windows_registry::{CLASSES_ROOT, CURRENT_USER, Key, LOCAL_MACHINE};

/// `KEY_WOW64_64KEY`: a mesma visão que o C# pedia (`RegistryView.Registry64`).
const WOW64_64KEY: u32 = 0x0100;

#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsRegistry;

impl WindowsRegistry {
    fn open(path: &str) -> Option<Key> {
        let (hive, sub) = path.split_once('\\').unwrap_or((path, ""));
        let root = match hive.to_ascii_uppercase().as_str() {
            "HKEY_LOCAL_MACHINE" | "HKLM" => LOCAL_MACHINE,
            "HKEY_CURRENT_USER" | "HKCU" => CURRENT_USER,
            "HKEY_CLASSES_ROOT" | "HKCR" => CLASSES_ROOT,
            _ => return None,
        };
        root.options().read().access(WOW64_64KEY).open(sub).ok()
    }
}

impl RegistryReader for WindowsRegistry {
    fn sub_keys(&self, key: &str) -> Vec<String> {
        Self::open(key)
            .and_then(|k| k.keys().ok().map(|it| it.collect()))
            .unwrap_or_default()
    }

    fn get_string(&self, key: &str, value: &str) -> Option<String> {
        let k = Self::open(key)?;
        // Como o `Convert.ToString` do C#: números (DWORD/QWORD) também viram texto.
        k.get_string(value)
            .ok()
            .or_else(|| k.get_u32(value).ok().map(|n| n.to_string()))
            .or_else(|| k.get_u64(value).ok().map(|n| n.to_string()))
    }
}
