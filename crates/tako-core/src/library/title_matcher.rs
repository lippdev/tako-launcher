//! Port de `Library/Metadata/TitleMatcher.cs`: compara o título local (nome do executável, por exemplo) com os
//! resultados de busca.

use super::search_text::without_marks;
use std::collections::HashSet;

/// Abaixo disso a correspondência não é aplicada automaticamente (o usuário pode corrigir).
pub const THRESHOLD: f64 = 0.7;

const NOISE: [&str; 15] = [
    "the",
    "edition",
    "goty",
    "definitive",
    "remastered",
    "deluxe",
    "complete",
    "ultimate",
    "game",
    "of",
    "year",
    "launcher",
    "x64",
    "win64",
    "shipping",
];

/// Minúsculas, sem acentos, ™/®, pontuação e palavras de edição; algarismos romanos simples viram números.
pub fn tokens(title: &str) -> Vec<String> {
    let lower = title.to_lowercase();
    let cleaned: String = without_marks(&lower)
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    cleaned
        .split_whitespace()
        .map(|t| match t {
            "ii" => "2",
            "iii" => "3",
            "iv" => "4",
            "v" => "5",
            "vi" => "6",
            _ => t,
        })
        .filter(|t| !NOISE.contains(t))
        .map(str::to_owned)
        .collect()
}

/// 1 = mesmo título; 0 = nada em comum (Dice sobre as palavras, com bônus para prefixo).
pub fn score(local: &str, candidate: &str) -> f64 {
    let a = tokens(local);
    let b = tokens(candidate);
    score_tokens(&a, &b)
}

/// Compara fatias de tokens já extraídas (evita re-tokenização repetida).
pub fn score_tokens(a: &[String], b: &[String]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    if a == b {
        return 1.0;
    }
    // Nomes de executável costumam vir colados ("eldenring", "HollowKnight").
    if a.concat() == b.concat() {
        return 0.95;
    }
    let in_b: HashSet<&String> = b.iter().collect();
    let common = a
        .iter()
        .filter(|t| in_b.contains(t))
        .collect::<HashSet<_>>()
        .len();
    let dice = 2.0 * common as f64 / (a.len() + b.len()) as f64;
    // Prefixo ("Hollow Knight" → "Hollow Knight: Silksong") só conta com 2+ palavras: "Minecraft" sozinho não é
    // "Minecraft Dungeons".
    let prefix = a.len().min(b.len()) >= 2 && (b.starts_with(a) || a.starts_with(b));
    (if prefix { dice.max(0.8) } else { dice }).min(0.99)
}

/// Termos de busca: o título como está e, se diferente, a versão limpa (sem "GOTY", ™, edição...).
pub fn search_terms(title: &str) -> Vec<String> {
    let clean = tokens(title).join(" ");
    if clean.is_empty() || clean.to_lowercase() == title.trim().to_lowercase() {
        vec![title.to_owned()]
    } else {
        vec![title.to_owned(), clean]
    }
}

/// Melhor candidato com nota a partir de [`THRESHOLD`]; no empate, o primeiro.
pub fn best<T>(
    title: &str,
    candidates: impl IntoIterator<Item = T>,
    title_of: impl Fn(&T) -> &str,
) -> Option<T> {
    let a = tokens(title);
    if a.is_empty() {
        return None;
    }
    let mut best: Option<(T, f64)> = None;
    for c in candidates {
        let b = tokens(title_of(&c));
        let s = score_tokens(&a, &b);
        if best.as_ref().is_none_or(|(_, b)| s > *b) {
            best = Some((c, s));
        }
    }
    best.filter(|(_, s)| *s >= THRESHOLD).map(|(c, _)| c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_drop_editions_marks_and_romans() {
        assert_eq!(
            tokens("The Witcher® III: Wild Hunt — GOTY Édition"),
            ["witcher", "3", "wild", "hunt"]
        );
        assert!(tokens("™ ®").is_empty());
    }

    #[test]
    fn scores_same_glued_and_prefix_titles() {
        assert_eq!(score("Hollow Knight", "Hollow Knight™"), 1.0);
        assert_eq!(score("HollowKnight", "Hollow Knight"), 0.95);
        assert_eq!(score("Hollow Knight", "Hollow Knight: Silksong"), 0.8);
        assert!(score("Minecraft", "Minecraft Dungeons") < THRESHOLD);
        assert_eq!(score("", "x"), 0.0);
        assert_eq!(
            best("Elden Ring", ["Elden Ring Nightreign", "ELDEN RING"], |s| s),
            Some("ELDEN RING")
        );
        assert_eq!(best("Elden Ring", ["Zelda"], |s| s), None);
        assert_eq!(search_terms("Hades"), ["Hades"]);
        assert_eq!(search_terms("Hades™ GOTY"), ["Hades™ GOTY", "hades"]);
    }
}
