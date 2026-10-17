//! ART-001: depois do casamento automático da biblioteca, o que vale o usuário conferir. Jogo sem algum tipo de
//! imagem e jogo que uma fonte achou só pelo nome, quando o nome achado não é igual ao da biblioteca.

use super::GameMetadata;
use super::art::pins;
use super::title_matcher;
use crate::library::{ArtKind, Game};

/// Uma fonte que achou o jogo só pelo nome, com outro nome.
#[derive(Debug, Clone, PartialEq)]
pub struct Doubt {
    pub source: String,
    /// Id e nome do jogo na fonte.
    pub id: String,
    pub found: String,
    /// `title_matcher::score` entre os dois nomes (menor = mais duvidoso).
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Review {
    /// Tipos sem imagem nenhuma (nem automática nem escolhida).
    pub missing: Vec<ArtKind>,
    pub doubt: Option<Doubt>,
}

impl Review {
    pub fn needs_attention(&self) -> bool {
        !self.missing.is_empty() || self.doubt.is_some()
    }

    /// Ordem da lista: sem capa primeiro, depois a correspondência mais duvidosa, depois o resto.
    pub fn rank(&self) -> (u8, i64) {
        let score = self
            .doubt
            .as_ref()
            .map_or(1000, |d| (d.score * 1000.) as i64);
        let tier = if self.missing.contains(&ArtKind::Cover) {
            0
        } else if self.doubt.is_some() {
            1
        } else {
            2
        };
        (tier, score)
    }
}

pub fn review(game: &Game, cached: Option<&GameMetadata>) -> Review {
    let missing = ArtKind::ALL
        .into_iter()
        .filter(|k| pins::shown(game, *k).is_none())
        .collect();
    let doubt = cached
        .into_iter()
        .flat_map(|c| &c.parts)
        .filter(|p| p.matched_by_title)
        .filter_map(|p| {
            let found = p.title.clone()?;
            let id = p.id_in(&p.source)?;
            let score = title_matcher::score(&game.title, &found);
            (score < 1.).then(|| Doubt {
                source: p.source.clone(),
                id,
                found,
                score,
            })
        })
        .min_by(|a, b| a.score.total_cmp(&b.score));
    Review { missing, doubt }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::media;

    fn part(source: &str, title: &str, by_title: bool) -> GameMetadata {
        GameMetadata {
            source_ids: [(source.to_owned(), "7".to_owned())].into(),
            title: Some(title.into()),
            matched_by_title: by_title,
            ..GameMetadata::new(source)
        }
    }

    #[test]
    fn steam_games_get_all_four_kinds_from_the_app_id() {
        let mut g = Game {
            title: "Lanterna".into(),
            ..Default::default()
        };
        GameMetadata {
            steam_app_id: Some(570),
            ..GameMetadata::new("Steam")
        }
        .apply_to(&mut g);
        assert_eq!(g.shown_logo(), Some("steam-art://570/logo"));
        assert_eq!(g.shown_icon(), Some("steam-art://570/icon"));
        assert_eq!(review(&g, None).missing, Vec::<ArtKind>::new());
        // Logo de outra fonte vale mais que o da Steam.
        let mut g2 = Game::default();
        GameMetadata {
            steam_app_id: Some(570),
            media: [(media::LOGO.to_owned(), "https://x/l.png".to_owned())].into(),
            ..GameMetadata::new("Steam")
        }
        .apply_to(&mut g2);
        assert_eq!(g2.shown_logo(), Some("https://x/l.png"));
    }

    #[test]
    fn title_only_matches_with_another_name_are_doubts_and_missing_art_ranks_first() {
        let g = Game {
            title: "Lanterna Distante".into(),
            cover_uri: Some("https://x/c.png".into()),
            ..Default::default()
        };
        let exact_name = GameMetadata {
            parts: vec![part("IGDB", "Lanterna Distante", true)],
            ..Default::default()
        };
        let r = review(&g, Some(&exact_name));
        assert_eq!(r.doubt, None);
        assert_eq!(r.missing, [ArtKind::Hero, ArtKind::Logo, ArtKind::Icon]);

        let other = GameMetadata {
            parts: vec![
                part("Steam", "Outro Jogo", false),
                part("IGDB", "Lanterna Distante Remaster", true),
                part("SteamGridDB", "Lanterna", true),
            ],
            ..Default::default()
        };
        let d = review(&g, Some(&other)).doubt.unwrap();
        assert_eq!((d.source.as_str(), d.id.as_str()), ("SteamGridDB", "7"));
        assert!(d.score < 0.8);

        let no_cover = review(&Game::default(), None);
        assert!(no_cover.needs_attention());
        assert!(no_cover.rank() < review(&g, Some(&other)).rank());
        assert!(review(&g, Some(&other)).rank() < review(&g, Some(&exact_name)).rank());
    }
}
