//! Port de `Presentation/VirtualKeyboard.cs` (UX-001): teclado virtual da busca, grade de 10 colunas que o
//! direcional percorre. A digitação física passa pelas mesmas regras ([`type_text`]): o texto nunca começa com
//! espaço, não repete espaços nem passa de [`MAX_LENGTH`] caracteres.

use super::tr;
use crate::i18n::Lang;

pub const COLUMNS: u8 = 10;
pub const MAX_LENGTH: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VirtualKeyKind {
    Character,
    Space,
    Backspace,
    Clear,
    Done,
}

/// Tecla do teclado virtual. `width` é a largura em colunas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualKey {
    pub kind: VirtualKeyKind,
    pub text: String,
    pub label: String,
    pub width: u8,
}

impl VirtualKey {
    fn character(c: char) -> VirtualKey {
        VirtualKey {
            kind: VirtualKeyKind::Character,
            text: c.to_string(),
            label: c.to_uppercase().collect(),
            width: 1,
        }
    }

    fn action(kind: VirtualKeyKind, text: &str, label: String, width: u8) -> VirtualKey {
        VirtualKey {
            kind,
            text: text.into(),
            label,
            width,
        }
    }
}

fn letters(s: &str) -> Vec<VirtualKey> {
    s.chars().map(VirtualKey::character).collect()
}

/// As linhas do teclado, com os rótulos no idioma.
pub fn rows(lang: Lang) -> Vec<Vec<VirtualKey>> {
    let label = |k: &str| tr(lang, &format!("presentation.keyboard.{k}"), &[]);
    let mut fourth = letters("zxcvbnm");
    fourth.push(VirtualKey::action(
        VirtualKeyKind::Backspace,
        "",
        label("backspace"),
        3,
    ));
    vec![
        letters("1234567890"),
        letters("qwertyuiop"),
        letters("asdfghjkl-"),
        fourth,
        vec![
            VirtualKey::action(VirtualKeyKind::Space, " ", label("space"), 4),
            VirtualKey::action(VirtualKeyKind::Clear, "", label("clear"), 3),
            VirtualKey::action(VirtualKeyKind::Done, "", label("done"), 3),
        ],
    ]
}

/// Texto depois de apertar a tecla. Pronto não muda o texto (quem chama fecha o teclado).
pub fn apply(text: &str, key: &VirtualKey) -> String {
    match key.kind {
        VirtualKeyKind::Character | VirtualKeyKind::Space => type_text(text, &key.text),
        VirtualKeyKind::Backspace => backspace(text),
        VirtualKeyKind::Clear => String::new(),
        VirtualKeyKind::Done => text.to_owned(),
    }
}

/// Acrescenta o que foi digitado (teclado físico ou virtual), ignorando caracteres de controle.
pub fn type_text(text: &str, typed: &str) -> String {
    let mut out = text.to_owned();
    let mut len = out.chars().count();
    for c in typed.chars() {
        if len >= MAX_LENGTH {
            break;
        }
        if c.is_control() {
            continue; // Enter, Tab, Backspace...
        }
        if !c.is_whitespace() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with(' ') {
            out.push(' ');
        } else {
            continue;
        }
        len += 1;
    }
    out
}

/// Apaga o último caractere (um emoji sai inteiro).
pub fn backspace(text: &str) -> String {
    let mut out = text.to_owned();
    out.pop();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_deletes_and_clears() {
        let keys: Vec<VirtualKey> = rows(Lang::PtBr).into_iter().flatten().collect();
        let key = |label: &str| keys.iter().find(|k| k.label == label).unwrap();

        let mut text = String::new();
        for label in ["Espaço", "G", "O", "W", "Espaço", "Espaço", "3"] {
            text = apply(&text, key(label));
        }
        assert_eq!(text, "gow 3"); // sem espaço no início nem espaço repetido
        text = apply(&text, key("Apagar"));
        assert_eq!(text, "gow ");
        assert_eq!(apply(&text, key("Pronto")), text);
        assert_eq!(apply(&text, key("Limpar")), "");
        assert_eq!(backspace(""), "");
    }

    #[test]
    fn rows_fill_the_ten_column_grid() {
        for lang in [Lang::PtBr, Lang::EnUs] {
            let rows = rows(lang);
            assert!(
                rows.iter()
                    .all(|r| r.iter().map(|k| k.width).sum::<u8>() == COLUMNS)
            );
            let mut labels: Vec<_> = rows.iter().flatten().map(|k| k.label.clone()).collect();
            let total = labels.len();
            labels.sort();
            labels.dedup();
            assert_eq!(labels.len(), total);
        }
        assert!(
            rows(Lang::EnUs)
                .iter()
                .flatten()
                .any(|k| k.label == "Space")
        );
    }

    #[test]
    fn physical_typing_ignores_control_characters_and_respects_the_limit() {
        assert_eq!(type_text("pok", "é\r\nmon\u{8}"), "pokémon");
        assert_eq!(type_text("a", "\t b"), "a b");
        assert_eq!(
            type_text(&"x".repeat(59), "yyyy").chars().count(),
            MAX_LENGTH
        );
        assert_eq!(backspace("a😀"), "a");
    }
}
