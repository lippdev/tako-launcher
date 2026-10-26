//! Acesso ao disco das fontes do Windows atrás de funções, para os testes usarem caminhos do
//! Windows (`D:\Jogos`) em qualquer SO, como os `Func<string, bool>` do C#.

use std::sync::Arc;

pub type ExistsFn = Arc<dyn Fn(&str) -> bool + Send + Sync>;
pub type ReadFn = Arc<dyn Fn(&str) -> Option<Vec<u8>> + Send + Sync>;

#[derive(Clone)]
pub struct StoreFs {
    /// A pasta ou o arquivo existe.
    pub exists: ExistsFn,
    /// Bytes do arquivo (no máximo [`crate::MAX_STORE_FILE`]); `None` se não existe ou não abre.
    pub read: ReadFn,
}

impl StoreFs {
    /// O disco de verdade.
    pub fn real() -> Self {
        StoreFs {
            exists: Arc::new(|p| std::path::Path::new(p).exists()),
            read: Arc::new(|p| {
                crate::read_limited(std::path::Path::new(p), crate::MAX_STORE_FILE).ok()
            }),
        }
    }

    pub fn read_text(&self, path: &str) -> Option<String> {
        (self.read)(path).map(|b| crate::text_of(&b))
    }
}

impl std::fmt::Debug for StoreFs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StoreFs")
    }
}

#[cfg(test)]
pub(crate) mod fake {
    use super::*;
    use std::collections::HashMap;

    /// Só estas pastas/arquivos existem (sem diferenciar maiúsculas) e estes arquivos têm conteúdo.
    pub fn with(existing: &[&str], files: HashMap<String, Vec<u8>>) -> StoreFs {
        let existing: Vec<String> = existing.iter().map(|s| s.to_ascii_lowercase()).collect();
        StoreFs {
            exists: Arc::new(move |p| existing.contains(&p.to_ascii_lowercase())),
            read: Arc::new(move |p| files.get(p).cloned()),
        }
    }

    pub fn folders(existing: &[&str]) -> StoreFs {
        with(existing, HashMap::new())
    }
}
