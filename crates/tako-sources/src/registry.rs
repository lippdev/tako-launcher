//! Acesso ao registro do Windows atrás de um trait (port de `IRegistryReader`), para os parsers
//! rodarem nos testes em qualquer SO com um registro em memória.

pub trait RegistryReader: Send + Sync {
    /// Nomes das subchaves de `key` (ex.: `HKLM\SOFTWARE\WOW6432Node\GOG.com\Games`).
    fn sub_keys(&self, key: &str) -> Vec<String>;
    fn get_string(&self, key: &str, value: &str) -> Option<String>;
}

/// Registro em memória para testes: `(chave, valor) -> texto`, sem diferenciar maiúsculas; subchaves
/// derivadas das chaves, com a grafia original (o nome da subchave vira título em algumas lojas).
#[derive(Debug, Default, Clone)]
pub struct MemoryRegistry {
    pub values: Vec<((String, String), String)>,
}

impl MemoryRegistry {
    pub fn set(&mut self, key: &str, value: &str, data: &str) -> &mut Self {
        self.values
            .retain(|((k, v), _)| !(k.eq_ignore_ascii_case(key) && v.eq_ignore_ascii_case(value)));
        self.values
            .push(((key.to_owned(), value.to_owned()), data.to_owned()));
        self
    }

    /// Versão encadeável por valor (`MemoryRegistry::default().with(..).with(..)`).
    pub fn with(mut self, key: &str, value: &str, data: &str) -> Self {
        self.set(key, value, data);
        self
    }
}

impl RegistryReader for MemoryRegistry {
    fn sub_keys(&self, key: &str) -> Vec<String> {
        let prefix = format!("{key}\\");
        let mut out: Vec<String> = Vec::new();
        for ((k, _), _) in &self.values {
            let Some(head) = k.get(..prefix.len()) else {
                continue;
            };
            if head.eq_ignore_ascii_case(&prefix) {
                let rest = &k[prefix.len()..];
                let name = rest.split('\\').next().unwrap_or(rest).to_owned();
                if !out.iter().any(|o| o.eq_ignore_ascii_case(&name)) {
                    out.push(name);
                }
            }
        }
        out
    }

    fn get_string(&self, key: &str, value: &str) -> Option<String> {
        self.values
            .iter()
            .find(|((k, v), _)| k.eq_ignore_ascii_case(key) && v.eq_ignore_ascii_case(value))
            .map(|(_, d)| d.clone())
    }
}
