//! Port de `TitleMatcher.cs`: compara o título local (nome do executável, por exemplo) com os
//! resultados de busca. Cópia de `library::title_matcher` (outra frente); deduplicar no merge.

use super::MetadataMatch;
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
    let mut clean = String::with_capacity(title.len());
    for c in title.chars().flat_map(char::to_lowercase) {
        if ('\u{300}'..='\u{36f}').contains(&c) {
            continue; // acento já decomposto
        }
        let c = fold(c);
        clean.push(if c.is_alphanumeric() { c } else { ' ' });
    }
    clean
        .split(' ')
        .filter(|t| !t.is_empty())
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

// shortcut: tira acentos do latim (Latin-1 e Extended-A) por tabela, sem normalização Unicode
// completa; trocar por `unicode-normalization` se aparecerem títulos em outros alfabetos com acento.
fn fold(c: char) -> char {
    const GROUPS: [(&str, char); 19] = [
        ("àáâãäåāăą", 'a'),
        ("çćĉċč", 'c'),
        ("ďđ", 'd'),
        ("èéêëēĕėęě", 'e'),
        ("ĝğġģ", 'g'),
        ("ĥħ", 'h'),
        ("ìíîïĩīĭįı", 'i'),
        ("ĵ", 'j'),
        ("ķ", 'k'),
        ("ĺļľŀł", 'l'),
        ("ñńņňŉ", 'n'),
        ("òóôõöøōŏő", 'o'),
        ("ŕŗř", 'r'),
        ("śŝşš", 's'),
        ("ţťŧ", 't'),
        ("ùúûüũūŭůűų", 'u'),
        ("ŵ", 'w'),
        ("ýÿŷ", 'y'),
        ("źżž", 'z'),
    ];
    if c.is_ascii() {
        return c;
    }
    GROUPS
        .iter()
        .find(|(set, _)| set.contains(c))
        .map_or(c, |(_, base)| *base)
}

/// 1 = mesmo título; 0 = nada em comum (Dice sobre as palavras, com bônus para prefixo).
pub fn score(local: &str, candidate: &str) -> f64 {
    let a = tokens(local);
    let b = tokens(candidate);
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
    let set_b: HashSet<&String> = b.iter().collect();
    let common = a
        .iter()
        .collect::<HashSet<_>>()
        .iter()
        .filter(|t| set_b.contains(*t))
        .count();
    let dice = 2.0 * common as f64 / (a.len() + b.len()) as f64;
    // Prefixo só conta com 2+ palavras: "Minecraft" sozinho não é "Minecraft Dungeons".
    let prefix = a.len().min(b.len()) >= 2 && (b.starts_with(&a) || a.starts_with(&b));
    let s = if prefix { dice.max(0.8) } else { dice };
    s.min(0.99)
}

/// Termos de busca: o título como está e, se diferente, a versão limpa.
pub fn search_terms(title: &str) -> Vec<String> {
    let clean = tokens(title).join(" ");
    if clean.is_empty() || clean.to_lowercase() == title.trim().to_lowercase() {
        vec![title.to_owned()]
    } else {
        vec![title.to_owned(), clean]
    }
}

/// O resultado mais parecido, se passar de [`THRESHOLD`] (empate: o primeiro).
pub fn best<'a>(title: &str, candidates: &'a [MetadataMatch]) -> Option<&'a MetadataMatch> {
    let mut top: Option<(&MetadataMatch, f64)> = None;
    for c in candidates {
        let s = score(title, &c.title);
        if top.is_none_or(|(_, t)| s > t) {
            top = Some((c, s));
        }
    }
    top.filter(|(_, s)| *s >= THRESHOLD).map(|(c, _)| c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equivalent_titles_match_exactly() {
        for (a, b, expected) in [
            ("Hollow Knight: Silksong", "Hollow Knight Silksong", 1.0),
            ("God of War III", "God of War 3", 1.0),
            ("Pokémon", "Pokemon", 1.0),
            (
                "The Witcher 3: Wild Hunt GOTY",
                "The Witcher® 3: Wild Hunt",
                1.0,
            ),
            ("eldenring", "ELDEN RING", 0.95),
        ] {
            assert!((score(a, b) - expected).abs() < 0.005, "{a} × {b}");
        }
    }

    #[test]
    fn search_terms_add_a_clean_title_when_it_differs() {
        assert_eq!(
            search_terms("The Witcher 3 Wild Hunt GOTY"),
            ["The Witcher 3 Wild Hunt GOTY", "witcher 3 wild hunt"]
        );
        assert_eq!(search_terms("hades 2"), ["hades 2"]);
    }

    #[test]
    fn picks_the_closest_result_and_rejects_weak_ones() {
        let results = [
            MetadataMatch::steam("Hollow Knight", 367520),
            MetadataMatch::steam("Hollow Knight: Silksong", 1030300),
            MetadataMatch::steam("Hollow Knight Soundtrack", 1),
        ];
        assert_eq!(
            best("silksong hollow knight", &results).and_then(|m| m.steam_app_id()),
            Some(1030300)
        );
        assert_eq!(
            best("Hollow Knight", &results).and_then(|m| m.steam_app_id()),
            Some(367520)
        );
        assert!(best("Forza Horizon 5", &results).is_none());
        let minecraft = [
            MetadataMatch::steam("Minecraft Dungeons II", 1912410),
            MetadataMatch::steam("Minecraft Legends", 1928870),
        ];
        assert!(best("Minecraft Launcher", &minecraft).is_none());
    }
}
