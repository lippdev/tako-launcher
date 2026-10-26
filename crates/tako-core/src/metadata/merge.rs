//! Port de `MetadataMerge.cs`: ordem das fontes por tipo de jogo e combinação campo a campo.

use super::{GameMetadata, sources, steam_art};
use crate::i18n::Lang;
use crate::library::{Game, GameStore};
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;

/// Tipo de jogo para decidir a ordem das fontes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataKind {
    /// Jogo da loja Steam (tem appid próprio).
    Steam,
    /// Jogo de PC de outra loja ou adicionado pelo usuário.
    Pc,
    /// ROM de console.
    Emulated,
}

/// Campos que a combinação decide um a um.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataField {
    Description,
    Genres,
    Themes,
    GameModes,
    Franchise,
    ReleaseDate,
    Developer,
    Publisher,
    Score,
    Art,
}

pub fn kind_of(game: &Game) -> MetadataKind {
    match game.platform.as_deref() {
        Some(p) if !p.is_empty() && !p.eq_ignore_ascii_case("pc") => MetadataKind::Emulated,
        _ if game.store == Some(GameStore::Steam) => MetadataKind::Steam,
        _ => MetadataKind::Pc,
    }
}

/// O ScreenScraper só atende ROMs (gastaria a cota diária do usuário sem ganho, ADR-0022).
pub fn applies(provider: &str, kind: MetadataKind) -> bool {
    provider != sources::SCREENSCRAPER || kind == MetadataKind::Emulated
}

/// Ordem geral das fontes para o tipo de jogo.
pub fn order(kind: MetadataKind) -> [&'static str; 4] {
    use sources::*;
    match kind {
        MetadataKind::Steam => [STEAM, IGDB, SCREENSCRAPER, STEAMGRIDDB],
        MetadataKind::Emulated => [SCREENSCRAPER, IGDB, STEAM, STEAMGRIDDB],
        MetadataKind::Pc => [IGDB, STEAM, SCREENSCRAPER, STEAMGRIDDB],
    }
}

/// Ordem para um campo: em jogos de PC a arte da Steam vem primeiro (capa 600×900, fundo 3840×1240).
pub fn order_for(kind: MetadataKind, field: MetadataField) -> [&'static str; 4] {
    use sources::*;
    match (kind, field) {
        (MetadataKind::Pc, MetadataField::Art) => [STEAM, IGDB, SCREENSCRAPER, STEAMGRIDDB],
        _ => order(kind),
    }
}

/// Ordena pela prioridade; desconhecidos vão para o fim, sem perder a ordem original (sort estável).
pub fn sort_by_priority<T>(items: &mut [T], name: impl Fn(&T) -> &str, order: &[&str]) {
    items.sort_by_key(|i| {
        order
            .iter()
            .position(|o| *o == name(i))
            .unwrap_or(order.len())
    });
}

/// Combina o resultado de vários provedores pela prioridade do tipo de jogo (regra pura).
/// A descrição prefere o idioma da interface (`lang`); sem ela, a primeira na ordem.
pub fn combine(
    parts: &[GameMetadata],
    kind: MetadataKind,
    now: DateTime<Utc>,
    consulted: Option<&[String]>,
    lang: Lang,
) -> GameMetadata {
    let found: Vec<&GameMetadata> = parts.iter().filter(|p| !p.is_empty()).collect();
    let by = |field: MetadataField| {
        let mut v = found.clone();
        sort_by_priority(&mut v, |p| &p.source, &order_for(kind, field));
        v
    };
    let first = |field, pick: fn(&GameMetadata) -> &Option<String>| {
        by(field).into_iter().find_map(|p| pick(p).clone())
    };
    let first_list = |field, pick: fn(&GameMetadata) -> &Vec<String>| {
        by(field)
            .into_iter()
            .map(pick)
            .find(|l| !l.is_empty())
            .cloned()
            .unwrap_or_default()
    };
    let first_score = |pick: fn(&GameMetadata) -> Option<i32>| {
        by(MetadataField::Score).into_iter().find_map(pick)
    };

    let prefix = match lang {
        Lang::PtBr => "pt",
        Lang::EnUs => "en",
    };
    let with_text: Vec<&GameMetadata> = by(MetadataField::Description)
        .into_iter()
        .filter(|p| p.description.as_ref().is_some_and(|d| !d.trim().is_empty()))
        .collect();
    let description = with_text
        .iter()
        .find(|p| {
            p.description_language
                .as_ref()
                .is_some_and(|l| l.to_ascii_lowercase().starts_with(prefix))
        })
        .or(with_text.first());

    // Capa e fundo da mesma fonte, para combinarem.
    let art = by(MetadataField::Art)
        .into_iter()
        .find(|p| p.cover_url.is_some() || p.steam_app_id.is_some());

    let general = by(MetadataField::Description); // mesma ordem geral
    let mut ids = BTreeMap::new();
    for part in general.iter().rev() {
        for (s, id) in &part.source_ids {
            ids.insert(s.clone(), id.clone()); // maior prioridade escreve por último e vence
        }
    }
    let steam_app_id = found
        .iter()
        .find_map(|p| p.steam_app_id)
        .or_else(|| ids.get(sources::STEAM).and_then(|s| s.parse::<u32>().ok()));

    let mut media = BTreeMap::new();
    for part in by(MetadataField::Art) {
        for (k, v) in &part.media {
            media.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }

    let source = if found.is_empty() {
        consulted.unwrap_or_default().join(" + ")
    } else {
        general
            .iter()
            .map(|p| p.source.as_str())
            .collect::<Vec<_>>()
            .join(" + ")
    };
    let consulted = consulted.map(<[String]>::to_vec).unwrap_or_else(|| {
        let mut names: Vec<String> = Vec::new();
        for p in parts {
            if !names.contains(&p.source) {
                names.push(p.source.clone());
            }
        }
        names
    });

    GameMetadata {
        source,
        fetched_at: Some(now),
        is_pinned: false,
        steam_app_id,
        source_ids: ids,
        consulted,
        parts: parts
            .iter()
            .map(|p| GameMetadata {
                parts: Vec::new(),
                ..p.clone()
            })
            .collect(),
        title: general.iter().find_map(|p| p.title.clone()),
        description: description.and_then(|p| p.description.clone()),
        description_language: description.and_then(|p| p.description_language.clone()),
        genres: first_list(MetadataField::Genres, |p| &p.genres),
        themes: first_list(MetadataField::Themes, |p| &p.themes),
        game_modes: first_list(MetadataField::GameModes, |p| &p.game_modes),
        franchise: first(MetadataField::Franchise, |p| &p.franchise),
        release_date: by(MetadataField::ReleaseDate)
            .into_iter()
            .find_map(|p| p.release_date),
        developer: first(MetadataField::Developer, |p| &p.developer),
        publisher: first(MetadataField::Publisher, |p| &p.publisher),
        metacritic_score: first_score(|p| p.metacritic_score),
        critic_score: first_score(|p| p.critic_score),
        cover_url: art.and_then(|a| {
            a.cover_url
                .clone()
                .or_else(|| a.steam_app_id.map(|id| steam_art(id, "cover")))
        }),
        background_url: art.and_then(|a| {
            a.background_url
                .clone()
                .or_else(|| a.steam_app_id.map(|id| steam_art(id, "hero")))
        }),
        players: general.iter().find_map(|p| p.players.clone()),
        media,
        // Só as partes dizem como cada fonte achou o jogo.
        matched_by_title: false,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::metadata::test_support::now;
    use chrono::NaiveDate;

    fn ids(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    pub(crate) fn steam_part() -> GameMetadata {
        GameMetadata {
            steam_app_id: Some(4000001),
            source_ids: ids(&[("Steam", "4000001")]),
            title: Some("Lanterna Distante".into()),
            description: Some("Um faroleiro explora um arquipélago.".into()),
            description_language: Some("pt-BR".into()),
            genres: vec!["Aventura".into()],
            developer: Some("Maré Baixa (Steam)".into()),
            metacritic_score: Some(88),
            ..GameMetadata::new("Steam")
        }
    }

    pub(crate) fn igdb_part() -> GameMetadata {
        GameMetadata {
            steam_app_id: Some(4000001),
            source_ids: ids(&[
                ("IGDB", "900001"),
                ("Steam", "4000001"),
                ("GOG", "1400000001"),
            ]),
            title: Some("Lanterna Distante".into()),
            description: Some("A lighthouse keeper explores an archipelago.".into()),
            description_language: Some("en".into()),
            genres: vec!["Aventura".into(), "Quebra-cabeça".into()],
            themes: vec!["Fantasia".into()],
            game_modes: vec!["Um jogador".into()],
            franchise: Some("Lanterna".into()),
            release_date: NaiveDate::from_ymd_opt(2024, 6, 1),
            developer: Some("Maré Baixa (IGDB)".into()),
            publisher: Some("Estúdio Farol".into()),
            critic_score: Some(86),
            cover_url: Some("https://images.igdb.com/igdb/image/upload/t_cover_big/c.jpg".into()),
            background_url: Some("https://images.igdb.com/igdb/image/upload/t_1080p/a.jpg".into()),
            ..GameMetadata::new("IGDB")
        }
    }

    pub(crate) fn igdb_only() -> GameMetadata {
        GameMetadata {
            steam_app_id: None,
            source_ids: ids(&[("IGDB", "900001")]),
            ..igdb_part()
        }
    }

    fn game(store: GameStore) -> Game {
        Game {
            id: format!("{}:1", store.id_prefix()),
            title: "Lanterna Distante".into(),
            store: Some(store),
            ..Default::default()
        }
    }

    #[test]
    fn steam_games_keep_steam_data_and_fill_gaps_with_igdb() {
        let m = combine(
            &[igdb_part(), steam_part()],
            MetadataKind::Steam,
            now(),
            None,
            Lang::PtBr,
        );
        assert_eq!(m.source, "Steam + IGDB");
        assert_eq!(
            m.description.as_deref(),
            Some("Um faroleiro explora um arquipélago.")
        );
        assert_eq!(m.genres, ["Aventura"]);
        assert_eq!(m.developer.as_deref(), Some("Maré Baixa (Steam)"));
        assert_eq!(m.publisher.as_deref(), Some("Estúdio Farol"));
        assert_eq!(m.themes, ["Fantasia"]);
        assert_eq!(m.franchise.as_deref(), Some("Lanterna"));
        assert_eq!(m.metacritic_score, Some(88));
        assert_eq!(m.critic_score, Some(86));
        assert_eq!(m.cover_url.as_deref(), Some("steam-art://4000001/cover"));
        assert_eq!(
            m.background_url.as_deref(),
            Some("steam-art://4000001/hero")
        );
        assert_eq!(m.id_in("IGDB").as_deref(), Some("900001"));
        assert_eq!(m.parts.len(), 2);
    }

    #[test]
    fn other_pc_games_use_igdb_first_but_keep_ui_language_text_and_bigger_steam_art() {
        let m = combine(
            &[steam_part(), igdb_part()],
            MetadataKind::Pc,
            now(),
            None,
            Lang::PtBr,
        );
        assert_eq!(m.source, "IGDB + Steam");
        assert_eq!(m.genres, ["Aventura", "Quebra-cabeça"]);
        assert_eq!(m.developer.as_deref(), Some("Maré Baixa (IGDB)"));
        assert_eq!(m.description_language.as_deref(), Some("pt-BR")); // português vence
        assert_eq!(m.cover_url.as_deref(), Some("steam-art://4000001/cover"));

        // Interface em inglês: o resumo em inglês vence.
        let en = combine(
            &[steam_part(), igdb_part()],
            MetadataKind::Steam,
            now(),
            None,
            Lang::EnUs,
        );
        assert_eq!(en.description_language.as_deref(), Some("en"));
    }

    #[test]
    fn without_steam_the_igdb_art_and_english_summary_are_used() {
        let m = combine(&[igdb_only()], MetadataKind::Pc, now(), None, Lang::PtBr);
        assert_eq!(m.cover_url, igdb_part().cover_url);
        assert_eq!(m.background_url, igdb_part().background_url);
        assert_eq!(m.description_language.as_deref(), Some("en"));
        assert_eq!(m.steam_app_id, None);
    }

    #[test]
    fn emulated_games_prefer_the_rom_provider_and_unknown_providers_go_last() {
        let rom = GameMetadata {
            source_ids: ids(&[("ScreenScraper", "77")]),
            genres: vec!["Plataforma".into()],
            cover_url: Some("https://example.invalid/box.png".into()),
            ..GameMetadata::new("ScreenScraper")
        };
        let other = GameMetadata {
            source_ids: ids(&[("Outro", "1")]),
            genres: vec!["X".into()],
            publisher: Some("Só no outro".into()),
            ..GameMetadata::new("Outro")
        };
        let m = combine(
            &[other, igdb_part(), rom.clone()],
            MetadataKind::Emulated,
            now(),
            None,
            Lang::PtBr,
        );
        assert_eq!(m.source, "ScreenScraper + IGDB + Outro");
        assert_eq!(m.genres, ["Plataforma"]);
        assert_eq!(m.cover_url, rom.cover_url);
        assert_eq!(m.publisher.as_deref(), Some("Estúdio Farol"));
        assert_eq!(
            order(MetadataKind::Steam),
            ["Steam", "IGDB", "ScreenScraper", "SteamGridDB"]
        );
        assert_eq!(
            order(MetadataKind::Pc),
            ["IGDB", "Steam", "ScreenScraper", "SteamGridDB"]
        );
    }

    #[test]
    fn kind_comes_from_store_and_platform() {
        assert_eq!(kind_of(&game(GameStore::Steam)), MetadataKind::Steam);
        assert_eq!(kind_of(&game(GameStore::Epic)), MetadataKind::Pc);
        let mut rom = game(GameStore::Manual);
        rom.platform = Some("snes".into());
        assert_eq!(kind_of(&rom), MetadataKind::Emulated);
        rom.platform = Some("PC".into());
        assert_eq!(kind_of(&rom), MetadataKind::Pc);
        assert!(!applies("ScreenScraper", MetadataKind::Pc));
        assert!(applies("ScreenScraper", MetadataKind::Emulated));
    }

    #[test]
    fn applying_igdb_art_fills_games_without_their_own_cover() {
        let mut g = game(GameStore::Epic);
        combine(&[igdb_only()], MetadataKind::Pc, now(), None, Lang::PtBr).apply_to(&mut g);
        assert_eq!(g.cover_uri, igdb_part().cover_url);
        assert_eq!(g.hero_uri, igdb_part().background_url);
        assert_eq!(g.themes, ["Fantasia"]);
        assert_eq!(g.game_modes, ["Um jogador"]);
        assert_eq!(g.franchise.as_deref(), Some("Lanterna"));
        assert_eq!(g.critic_score, Some(86));
    }
}
