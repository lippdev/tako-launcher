//! Tags do usuário (LIB-002): rótulos livres por jogo, aplicados a vários jogos de uma vez e usados como filtro.
//! Diferente das coleções, não têm id nem cadastro: uma tag existe enquanto algum jogo a tiver.

use super::Game;
use super::collections::{MAX_NAME_LENGTH, NameProblem, clean_name};
use super::query::{FacetValue, title_cmp, title_eq};

/// Problema do nome de uma tag nova, ou `None` quando serve (repetir uma existente é aplicá-la).
pub fn name_problem(name: &str) -> Option<NameProblem> {
    let clean = clean_name(name);
    if clean.is_empty() {
        Some(NameProblem::Empty)
    } else if clean.chars().count() > MAX_NAME_LENGTH {
        Some(NameProblem::TooLong)
    } else {
        None
    }
}

/// O nome como fica gravado: limpo e, se já existe uma tag igual sem caixa nem acento, a grafia dela.
pub fn canonical(name: &str, games: &[Game]) -> String {
    let clean = clean_name(name);
    games
        .iter()
        .flat_map(|g| g.tags.iter())
        .find(|t| title_eq(t, &clean))
        .cloned()
        .unwrap_or(clean)
}

/// Todas as tags em uso, em ordem de nome, com quantos jogos têm cada uma.
pub fn all<'a>(games: impl IntoIterator<Item = &'a Game>) -> Vec<FacetValue> {
    let mut out: Vec<FacetValue> = Vec::new();
    for tag in games.into_iter().flat_map(|g| g.tags.iter()) {
        match out.iter_mut().find(|f| title_eq(&f.name, tag)) {
            Some(f) => f.count += 1,
            None => out.push(FacetValue {
                name: tag.clone(),
                count: 1,
            }),
        }
    }
    out.sort_by(|a, b| title_cmp(&a.name, &b.name));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(tags: &[&str]) -> Game {
        Game {
            tags: tags.iter().map(|t| t.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn names_are_cleaned_and_reuse_the_existing_spelling() {
        let games = [game(&["Sofá"]), game(&["Coop", "Sofá"])];
        assert_eq!(canonical("  sofa ", &games), "Sofá");
        assert_eq!(canonical("Para  zerar", &games), "Para zerar");
        assert_eq!(name_problem("   "), Some(NameProblem::Empty));
        assert_eq!(name_problem(&"x".repeat(41)), Some(NameProblem::TooLong));
        assert_eq!(name_problem("Coop"), None);
        let all = all(&games);
        assert_eq!(
            all.iter()
                .map(|f| (f.name.as_str(), f.count))
                .collect::<Vec<_>>(),
            [("Coop", 1), ("Sofá", 2)]
        );
    }
}
