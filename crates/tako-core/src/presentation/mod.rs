//! Regras puras de apresentação (port de `Launcher.Core/Presentation`): grade virtual, cache, dicas, cores e textos.
//!
//! Os textos recebem o idioma explicitamente (`Lang`): a UI passa `i18n::lang()` e os testes podem conferir os dois
//! idiomas sem mexer no idioma global.

pub mod color;
pub mod game_text;
pub mod hint_fit;
pub mod library_text;
pub mod lru;
pub mod plain_text;
pub mod virtual_grid;
pub mod virtual_keyboard;

pub use color::{AccentPalette, Rgb};
pub use hint_fit::{Hint, HintLayout};
pub use lru::SizedLruCache;
pub use virtual_grid::{ItemRange, VirtualGrid};
pub use virtual_keyboard::{VirtualKey, VirtualKeyKind};

use crate::i18n::{Lang, t_in};

/// `tf` com idioma explícito.
pub(crate) fn tr(lang: Lang, key: &str, args: &[(&str, &str)]) -> String {
    args.iter().fold(t_in(lang, key), |s, (k, v)| {
        s.replace(&format!("{{{k}}}"), v)
    })
}

/// `tn` com idioma explícito.
pub(crate) fn trn(lang: Lang, key: &str, n: i64, args: &[(&str, &str)]) -> String {
    let form = if n == 1 { "one" } else { "other" };
    let n_text = n.to_string();
    let mut all = vec![("n", n_text.as_str())];
    all.extend_from_slice(args);
    tr(lang, &format!("{key}.{form}"), &all)
}
