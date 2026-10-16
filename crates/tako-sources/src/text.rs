//! Comparação de títulos sem maiúsculas nem acentos (como `CurrentCultureIgnoreCase` e o
//! `TitleComparer` do C#) e a chave de "mesmo jogo" de `StoreDuplicates.Key`.

use tako_core::library::Game;

/// Minúsculas sem acentos latinos.
// shortcut: só acentos do Latin-1/Latin Extended-A comuns, sem normalização Unicode completa; trocar por `unicode-normalization` se aparecer outro alfabeto.
pub fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(strip_accent)
        .collect()
}

fn strip_accent(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => 'a',
        'ç' | 'ć' | 'č' => 'c',
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ė' | 'ę' | 'ě' => 'e',
        'ì' | 'í' | 'î' | 'ï' | 'ī' | 'į' => 'i',
        'ñ' | 'ń' | 'ň' => 'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ō' | 'ő' => 'o',
        'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' => 'u',
        'ý' | 'ÿ' => 'y',
        'š' | 'ś' => 's',
        'ž' | 'ź' | 'ż' => 'z',
        other => other,
    }
}

/// Ordena por título como a biblioteca (sem maiúsculas nem acentos).
pub fn sort_by_title(games: &mut [Game]) {
    games.sort_by_cached_key(|g| fold(&g.title));
}

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

/// Chave de "mesmo jogo" (port de `TitleMatcher.Tokens` + `StoreDuplicates.Key`): sem edição, ™/®,
/// acentos e pontuação; algarismos romanos simples viram números.
pub fn title_key(title: &str) -> String {
    let spaced: String = fold(title)
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    spaced
        .split_whitespace()
        .map(|t| match t {
            "ii" => "2",
            "iii" => "3",
            "iv" => "4",
            "v" => "5",
            "vi" => "6",
            t => t,
        })
        .filter(|t| !NOISE.contains(t))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_fold_accents_and_edition_noise() {
        assert_eq!(fold("Él Ação"), "el acao");
        assert_eq!(title_key("Lanterna Distante™"), "lanterna distante");
        assert_eq!(title_key("Farol II: GOTY Edition"), "farol 2");
        assert_eq!(title_key("™"), "");
    }
}
