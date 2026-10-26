//! Port de `Library/SearchText.cs` (UX-001): busca sem diferenciar maiúsculas nem acentos ("pokemon" acha
//! "Pokémon"), ignorando pontuação e com as palavras digitadas em qualquer ordem.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// Texto decomposto (NFD) sem os acentos.
pub(crate) fn without_marks(text: &str) -> impl Iterator<Item = char> + '_ {
    text.nfd().filter(|c| !is_combining_mark(*c))
}

/// Minúsculas, sem acentos, pontuação vira espaço e espaços repetidos viram um.
pub fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut space = true; // sem espaço no início
    for c in without_marks(text) {
        if c.is_alphanumeric() {
            out.extend(fold(c).to_lowercase());
            space = false;
        } else if matches!(c, '\'' | '’' | '`' | '´') {
            // Apóstrofo some ("Assassin's" = "assassins"); o resto da pontuação separa palavras.
        } else if !space {
            out.push(' ');
            space = true;
        }
    }
    out.truncate(out.trim_end().len());
    out
}

/// Letras que não se decompõem em letra + acento (ø, ł, đ, ı).
fn fold(c: char) -> char {
    match c {
        'ø' | 'Ø' => 'o',
        'ł' | 'Ł' => 'l',
        'đ' | 'Đ' => 'd',
        'ı' => 'i',
        _ => c,
    }
}

/// Busca já normalizada (normalize uma vez por lista).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    pub normalized: String,
    pub words: Vec<String>,
    /// A busca sem espaços ("residentevil" acha "Resident Evil").
    pub compact: String,
}

impl Query {
    pub fn parse(text: &str) -> Query {
        let normalized = normalize(text);
        Query {
            words: normalized
                .split(' ')
                .filter(|w| !w.is_empty())
                .map(str::to_owned)
                .collect(),
            compact: normalized.replace(' ', ""),
            normalized,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.normalized.is_empty()
    }

    /// Cada palavra aparece no texto (já normalizado), em qualquer ordem. Busca vazia combina com tudo.
    pub fn matches(&self, normalized_text: &str) -> bool {
        self.is_empty()
            || self
                .words
                .iter()
                .all(|w| normalized_text.contains(w.as_str()))
            || (self.compact.chars().count() > 1
                && normalized_text
                    .replace(' ', "")
                    .contains(self.compact.as_str()))
    }
}

/// Atalho para um texto só.
pub fn matches(text: &str, query: &str) -> bool {
    Query::parse(query).matches(&normalize(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_drops_case_accents_and_punctuation() {
        for (text, expected) in [
            ("Pokémon Édition", "pokemon edition"),
            ("  God of War:   Ragnarök ", "god of war ragnarok"),
            ("Assassin's Creed® Shadows™", "assassins creed shadows"),
            (
                "Indiana Jones e o Grande Círculo",
                "indiana jones e o grande circulo",
            ),
            ("ÆON Ølstad Łódź", "æon olstad lodz"),
            ("", ""),
        ] {
            assert_eq!(normalize(text), expected, "{text}");
        }
    }

    #[test]
    fn matches_every_word_without_case_or_accents() {
        for (title, query, expected) in [
            ("God of War Ragnarök", "ragnarok", true),
            ("God of War Ragnarök", "RAGNARÖK", true),
            ("God of War Ragnarök", "war god", true), // qualquer ordem
            ("God of War Ragnarök", "gow", false),
            ("Resident Evil Requiem", "residentevil", true), // sem espaços
            ("Indiana Jones e o Grande Círculo", "circulo", true),
            ("Hollow Knight: Silksong", "knight silk", true),
            ("Hollow Knight: Silksong", "knight silkk", false),
            ("Qualquer jogo", "", true),
            ("Qualquer jogo", "   ", true),
        ] {
            assert_eq!(matches(title, query), expected, "{title} / {query}");
        }
    }
}
