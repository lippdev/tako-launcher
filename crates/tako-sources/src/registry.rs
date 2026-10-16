//! Acesso ao registro do Windows atrás de um trait (port de `IRegistryReader`), para os parsers
//! rodarem nos testes em qualquer SO com um registro em memória.

use std::collections::BTreeMap;

pub trait RegistryReader: Send + Sync {
    /// Nomes das subchaves de `key` (ex.: `HKLM\SOFTWARE\WOW6432Node\GOG.com\Games`).
    fn sub_keys(&self, key: &str) -> Vec<String>;
    fn get_string(&self, key: &str, value: &str) -> Option<String>;
}

/// Registro em memória para testes: `(chave, valor) -> texto`; subchaves derivadas das chaves.
#[derive(Debug, Default, Clone)]
pub struct MemoryRegistry {
    pub values: BTreeMap<(String, String), String>,
}

impl MemoryRegistry {
    pub fn set(&mut self, key: &str, value: &str, data: &str) -> &mut Self {
        self.values.insert(
            (key.to_ascii_lowercase(), value.to_ascii_lowercase()),
            data.to_owned(),
        );
        self
    }
}

impl RegistryReader for MemoryRegistry {
    fn sub_keys(&self, key: &str) -> Vec<String> {
        let prefix = format!("{}\\", key.to_ascii_lowercase());
        let mut out: Vec<String> = Vec::new();
        for (k, _) in self.values.keys() {
            if let Some(rest) = k.strip_prefix(&prefix) {
                let name = rest.split('\\').next().unwrap_or(rest).to_owned();
                if !out.contains(&name) {
                    out.push(name);
                }
            }
        }
        out
    }

    fn get_string(&self, key: &str, value: &str) -> Option<String> {
        self.values
            .get(&(key.to_ascii_lowercase(), value.to_ascii_lowercase()))
            .cloned()
    }
}
