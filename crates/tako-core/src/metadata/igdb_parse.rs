//! Port de `Igdb/IgdbQueries.cs`, `IgdbParser.cs`, `IgdbPlatforms.cs` e `IgdbText.cs`: consultas
//! Apicalypse, respostas → metadados, plataformas e tradução de gêneros/temas/modos (puros, testados).

use super::art::ArtCandidate;
use super::{GameMetadata, MetadataMatch, external_ids, sources};
use crate::i18n::t;
use crate::library::ArtKind;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::BTreeMap;

pub const MAX_LIMIT: usize = 500;
pub const COVER_SIZE: &str = "t_cover_big";
pub const ARTWORK_SIZE: &str = "t_1080p";
pub const SCREENSHOT_SIZE: &str = "t_screenshot_huge";
pub const PING: &str = "fields name; limit 1;";

/// Campos de um jogo, já expandidos. `external_game_source` substitui o antigo `category`.
pub const GAME_FIELDS: &str = "name, summary, first_release_date, aggregated_rating, total_rating, platforms, \
genres.name, themes.name, game_modes.name, \
involved_companies.company.name, involved_companies.developer, involved_companies.publisher, \
cover.image_id, artworks.image_id, screenshots.image_id, \
franchise.name, franchises.name, collections.name, \
external_games.uid, external_games.external_game_source, \
game_localizations.name, game_localizations.region.identifier, game_localizations.region.name, game_localizations.cover.image_id";

/// Lojas → id em `external_game_sources`.
pub const EXTERNAL_SOURCES: [(&str, i64); 4] = [
    (external_ids::STEAM, 1),
    (external_ids::GOG, 5),
    (external_ids::MICROSOFT, 11),
    (external_ids::EPIC, 26),
];

pub fn external_source_id(store: &str) -> Option<i64> {
    EXTERNAL_SOURCES
        .iter()
        .find(|(s, _)| *s == store)
        .map(|(_, id)| *id)
}

fn join<T: ToString>(v: &[T]) -> String {
    v.iter().map(T::to_string).collect::<Vec<_>>().join(",")
}

/// Busca por nome, só jogos principais, filtrada pela plataforma quando conhecida.
pub fn search_query(title: &str, platforms: &[i64], limit: usize) -> String {
    let mut wh = vec!["version_parent = null".to_owned()];
    if !platforms.is_empty() {
        wh.push(format!("platforms = ({})", join(platforms)));
    }
    format!(
        "search \"{}\"; fields {GAME_FIELDS}; where {}; limit {limit};",
        escape(title),
        wh.join(" & ")
    )
}

/// Vários jogos pelo id (lote); ordem fixa para a mesma consulta acertar o cache.
pub fn games_by_id(ids: &[i64]) -> String {
    let mut list = ids.to_vec();
    list.sort_unstable();
    list.dedup();
    format!(
        "fields {GAME_FIELDS}; where id = ({}); limit {};",
        join(&list),
        list.len().clamp(1, MAX_LIMIT)
    )
}

/// Ids de jogos da IGDB a partir dos ids numa loja (lote, uma fonte por consulta).
pub fn external_games(source: i64, uids: &[String]) -> String {
    let mut list: Vec<&String> = Vec::new();
    for u in uids {
        if !list.contains(&u) {
            list.push(u);
        }
    }
    let quoted = list
        .iter()
        .map(|u| format!("\"{}\"", escape(u)))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "fields game, uid, external_game_source; where external_game_source = {source} & uid = ({quoted}); limit {};",
        (list.len() * 2).clamp(1, MAX_LIMIT)
    )
}

/// Id do jogo pelo nome na URL.
pub fn by_slug(slug: &str) -> String {
    format!(
        "fields id, name; where slug = \"{}\"; limit 1;",
        escape(&slug.trim().to_lowercase())
    )
}

pub fn image(image_id: &str, size: &str) -> String {
    format!(
        "https://images.igdb.com/igdb/image/upload/{size}/{}.jpg",
        super::http::escape(image_id)
    )
}

/// Texto entre aspas: escapa barra invertida e aspas, troca quebras de linha por espaço.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' | '"' => {
                out.push('\\');
                out.push(c);
            }
            '\r' | '\n' | '\t' => out.push(' '),
            _ => out.push(c),
        }
    }
    out.trim().to_owned()
}

// ---------------------------------------------------------------- Plataformas

pub const PC_WINDOWS: i64 = 6;

/// Chave do Tako → (nome curto, ids da IGDB).
pub const PLATFORMS: [(&str, &str, &[i64]); 31] = [
    ("pc", "PC", &[PC_WINDOWS]),
    ("nes", "NES", &[18, 99]),
    ("snes", "SNES", &[19, 58]),
    ("n64", "Nintendo 64", &[4]),
    ("gamecube", "GameCube", &[21]),
    ("wii", "Wii", &[5]),
    ("wiiu", "Wii U", &[41]),
    ("switch", "Nintendo Switch", &[130]),
    ("gb", "Game Boy", &[33]),
    ("gbc", "Game Boy Color", &[22]),
    ("gba", "Game Boy Advance", &[24]),
    ("nds", "Nintendo DS", &[20]),
    ("3ds", "Nintendo 3DS", &[37]),
    ("ps1", "PlayStation", &[7]),
    ("ps2", "PlayStation 2", &[8]),
    ("ps3", "PlayStation 3", &[9]),
    ("psp", "PSP", &[38]),
    ("psvita", "PS Vita", &[46]),
    ("xbox", "Xbox", &[11]),
    ("xbox360", "Xbox 360", &[12]),
    ("mastersystem", "Master System", &[64]),
    ("megadrive", "Mega Drive", &[29]),
    ("segacd", "Sega CD", &[78]),
    ("32x", "Sega 32X", &[30]),
    ("saturn", "Saturn", &[32]),
    ("dreamcast", "Dreamcast", &[23]),
    ("gamegear", "Game Gear", &[35]),
    ("atari2600", "Atari 2600", &[59]),
    ("pcengine", "PC Engine", &[86]),
    ("neogeo", "Neo Geo", &[80]),
    ("arcade", "Arcade", &[52]),
];

/// Ids da IGDB para a plataforma; vazia = PC; desconhecida = sem filtro.
pub fn platform_ids(platform: Option<&str>) -> Vec<i64> {
    match platform.map(str::trim).filter(|p| !p.is_empty()) {
        None => vec![PC_WINDOWS],
        Some(p) => PLATFORMS
            .iter()
            .find(|(k, _, _)| k.eq_ignore_ascii_case(p))
            .map(|(_, _, ids)| ids.to_vec())
            .unwrap_or_default(),
    }
}

pub fn platform_name(igdb_id: i64) -> Option<&'static str> {
    PLATFORMS
        .iter()
        .find(|(_, _, ids)| ids.contains(&igdb_id))
        .map(|(_, n, _)| *n)
}

// ---------------------------------------------------------------- Termos

/// Nome em inglês da IGDB → chave de tradução (`metadata.term.*`).
const TERMS: [(&str, &str); 51] = [
    ("Adventure", "adventure"),
    ("Arcade", "arcade"),
    ("Card & Board Game", "card_board"),
    ("Fighting", "fighting"),
    ("Hack and slash/Beat 'em up", "hack_slash"),
    ("Indie", "indie"),
    ("MOBA", "moba"),
    ("Music", "music"),
    ("Pinball", "pinball"),
    ("Platform", "platform"),
    ("Point-and-click", "point_click"),
    ("Puzzle", "puzzle"),
    ("Quiz/Trivia", "quiz"),
    ("Racing", "racing"),
    ("Real Time Strategy (RTS)", "rts"),
    ("Role-playing (RPG)", "rpg"),
    ("Shooter", "shooter"),
    ("Simulator", "simulator"),
    ("Sport", "sport"),
    ("Strategy", "strategy"),
    ("Tactical", "tactical"),
    ("Turn-based strategy (TBS)", "tbs"),
    ("Visual Novel", "visual_novel"),
    ("Action", "action"),
    ("Business", "business"),
    ("Comedy", "comedy"),
    ("Drama", "drama"),
    ("Educational", "educational"),
    ("Erotic", "erotic"),
    ("Fantasy", "fantasy"),
    ("Historical", "historical"),
    ("Horror", "horror"),
    ("Kids", "kids"),
    ("Mystery", "mystery"),
    ("Non-fiction", "non_fiction"),
    ("Open world", "open_world"),
    ("Party", "party"),
    ("Romance", "romance"),
    ("Sandbox", "sandbox"),
    ("Science fiction", "science_fiction"),
    ("Stealth", "stealth"),
    ("Survival", "survival"),
    ("Thriller", "thriller"),
    ("Warfare", "warfare"),
    ("4X (explore, expand, exploit, and exterminate)", "four_x"),
    ("Single player", "single_player"),
    ("Multiplayer", "multiplayer"),
    ("Co-operative", "cooperative"),
    ("Split screen", "split_screen"),
    ("Massively Multiplayer Online (MMO)", "mmo"),
    ("Battle Royale", "battle_royale"),
];

/// Gêneros, temas e modos da IGDB (só em inglês) no idioma da interface; fora da lista, como vieram.
pub fn translate(term: &str) -> String {
    let term = term.trim();
    TERMS
        .iter()
        .find(|(en, _)| en.eq_ignore_ascii_case(term))
        .map_or_else(
            || term.to_owned(),
            |(_, k)| t(&format!("metadata.term.{k}")),
        )
}

// ---------------------------------------------------------------- Respostas

fn text<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str)
}

fn array<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    v.get(k)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn names(game: &Value, k: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for n in array(game, k).iter().filter_map(|i| text(i, "name")) {
        let tr = translate(n);
        if !out.contains(&tr) {
            out.push(tr);
        }
    }
    out
}

fn is_brazil(loc: &Value) -> bool {
    let Some(region) = loc.get("region").filter(|r| r.is_object()) else {
        return false;
    };
    let id = text(region, "identifier").map(str::to_lowercase);
    text(region, "name").is_some_and(|n| n.eq_ignore_ascii_case("Brazil"))
        || id.is_some_and(|i| i == "br" || i.ends_with("_br") || i.ends_with("-br"))
}

fn brazil(game: &Value) -> Option<&Value> {
    array(game, "game_localizations")
        .iter()
        .find(|l| is_brazil(l))
}

fn cover_id(v: &Value) -> Option<&str> {
    v.get("cover").and_then(|c| text(c, "image_id"))
}

fn year_of(seconds: i64) -> Option<i32> {
    use chrono::Datelike;
    DateTime::from_timestamp(seconds, 0).map(|d| d.year())
}

fn store_of(external: &Value) -> Option<&'static str> {
    let s = external
        .get("external_game_source")
        .or_else(|| external.get("category"))
        .and_then(Value::as_i64)?;
    EXTERNAL_SOURCES
        .iter()
        .find(|(_, id)| *id == s)
        .map(|(n, _)| *n)
}

/// Um jogo do endpoint `games` (com os campos de [`GAME_FIELDS`]).
pub fn game(game: &Value, now: DateTime<Utc>) -> Option<GameMetadata> {
    let id = game.get("id")?.as_i64()?;
    let mut ids = BTreeMap::from([(sources::IGDB.to_owned(), id.to_string())]);
    for ext in array(game, "external_games") {
        if let (Some(uid), Some(store)) = (text(ext, "uid"), store_of(ext)) {
            ids.entry(store.to_owned())
                .or_insert_with(|| uid.to_owned());
        }
    }
    let br = brazil(game);
    let cover = br.and_then(cover_id).or_else(|| cover_id(game));
    // Fundo: arte oficial em 1080p; sem ela, a primeira captura de tela.
    let first_image = |k| array(game, k).iter().find_map(|a| text(a, "image_id"));
    let background = first_image("artworks")
        .map(|a| image(a, ARTWORK_SIZE))
        .or_else(|| first_image("screenshots").map(|s| image(s, SCREENSHOT_SIZE)));
    let summary = text(game, "summary")
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let company = |role: &str| {
        array(game, "involved_companies")
            .iter()
            .filter(|i| i.get(role).and_then(Value::as_bool) == Some(true))
            .find_map(|i| i.get("company").and_then(|c| text(c, "name")))
            .map(str::to_owned)
    };
    let score = |k: &str| {
        game.get(k)
            .and_then(Value::as_f64)
            .filter(|s| *s > 0.0)
            .map(|s| s.clamp(0.0, 100.0).round() as i32)
    };
    let steam_app_id = ids.get(external_ids::STEAM).and_then(|s| s.parse().ok());
    Some(GameMetadata {
        fetched_at: Some(now),
        steam_app_id,
        source_ids: ids,
        title: br
            .and_then(|b| text(b, "name"))
            .or_else(|| text(game, "name"))
            .map(str::to_owned),
        // A IGDB não traduz resumos: sempre em inglês.
        description: summary.map(str::to_owned),
        description_language: summary.map(|_| "en".to_owned()),
        genres: names(game, "genres").into_iter().take(3).collect(),
        themes: names(game, "themes").into_iter().take(3).collect(),
        game_modes: names(game, "game_modes"),
        franchise: game
            .get("franchise")
            .and_then(|f| text(f, "name"))
            .map(str::to_owned)
            .or_else(|| names(game, "franchises").into_iter().next())
            .or_else(|| names(game, "collections").into_iter().next()),
        release_date: game
            .get("first_release_date")
            .and_then(Value::as_i64)
            .and_then(|s| DateTime::from_timestamp(s, 0))
            .map(|d| d.date_naive()),
        developer: company("developer"),
        publisher: company("publisher"),
        critic_score: score("aggregated_rating").or_else(|| score("total_rating")),
        cover_url: cover.map(|c| image(c, COVER_SIZE)),
        background_url: background,
        ..GameMetadata::new(sources::IGDB)
    })
}

/// Resultado de busca para "Corrigir correspondência" (nome, ano e plataformas).
pub fn match_of(game: &Value) -> Option<MetadataMatch> {
    let id = game.get("id")?.as_i64()?;
    let name = text(game, "name")?;
    let mut platforms: Vec<&str> = Vec::new();
    for p in array(game, "platforms")
        .iter()
        .filter_map(Value::as_i64)
        .filter_map(platform_name)
    {
        if !platforms.contains(&p) {
            platforms.push(p);
        }
    }
    let mut m = MetadataMatch::new(sources::IGDB, id.to_string(), name);
    m.year = game
        .get("first_release_date")
        .and_then(Value::as_i64)
        .and_then(year_of);
    m.platforms = (!platforms.is_empty()).then(|| platforms.join(", "));
    m.cover_url = cover_id(game).map(|c| image(c, "t_cover_small"));
    Some(m)
}

/// Imagens do jogo para o seletor (ADR-0023).
pub fn images(game: &Value) -> Vec<ArtCandidate> {
    let mut out: Vec<ArtCandidate> = Vec::new();
    let mut add = |id: Option<&str>, kind, size: &str, thumb: &str, label: &str| {
        let Some(id) = id else { return };
        let url = image(id, size);
        if !out.iter().any(|c| c.url == url) {
            out.push(ArtCandidate {
                thumbnail: Some(image(id, thumb)),
                label: Some(t(label)),
                ..ArtCandidate::new(sources::IGDB, kind, url)
            });
        }
    };
    add(
        cover_id(game),
        ArtKind::Cover,
        "t_cover_big_2x",
        "t_cover_big",
        "metadata.art.igdb_cover",
    );
    add(
        brazil(game).and_then(cover_id),
        ArtKind::Cover,
        "t_cover_big_2x",
        "t_cover_big",
        "metadata.art.igdb_cover_brazil",
    );
    for a in array(game, "artworks") {
        add(
            text(a, "image_id"),
            ArtKind::Hero,
            ARTWORK_SIZE,
            "t_screenshot_med",
            "metadata.art.igdb_artwork",
        );
    }
    for s in array(game, "screenshots") {
        add(
            text(s, "image_id"),
            ArtKind::Hero,
            SCREENSHOT_SIZE,
            "t_screenshot_med",
            "metadata.art.igdb_screenshot",
        );
    }
    out
}

/// Resposta de `external_games` → (uid na loja → id do jogo na IGDB).
pub fn external_game_map(array: &Value) -> BTreeMap<String, i64> {
    let mut map = BTreeMap::new();
    for item in array.as_array().map(Vec::as_slice).unwrap_or_default() {
        if let (Some(uid), Some(game)) =
            (text(item, "uid"), item.get("game").and_then(Value::as_i64))
        {
            map.entry(uid.to_owned()).or_insert(game);
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::test_support::{fixture, now};
    use chrono::NaiveDate;

    fn games() -> Value {
        serde_json::from_str(&fixture("games-lanterna.json")).unwrap()
    }

    #[test]
    fn search_escapes_the_title_and_filters_by_platform() {
        let q = search_query("Jogo \"citado\" \\ teste\n", &platform_ids(None), 10);
        assert!(q.starts_with(r#"search "Jogo \"citado\" \\ teste"; fields name, summary"#));
        assert!(q.contains("where version_parent = null & platforms = (6); limit 10;"));
        assert!(q.contains("involved_companies.company.name"));
        assert!(q.contains("external_games.external_game_source"));
        assert!(
            search_query("Mario", &platform_ids(Some("snes")), 10).contains("platforms = (19,58)")
        );
        assert!(
            !search_query("Mario", &platform_ids(Some("desconhecida")), 10).contains("platforms =")
        );
    }

    #[test]
    fn batch_queries_list_ids_and_store_uids() {
        assert!(games_by_id(&[3, 1, 2, 2]).ends_with("where id = (1,2,3); limit 3;"));
        assert_eq!(
            external_games(1, &["570".into(), "730".into(), "570".into()]),
            r#"fields game, uid, external_game_source; where external_game_source = 1 & uid = ("570","730"); limit 4;"#
        );
        assert_eq!(
            image("abc", COVER_SIZE),
            "https://images.igdb.com/igdb/image/upload/t_cover_big/abc.jpg"
        );
    }

    #[test]
    fn platforms_map_console_keys_to_igdb_ids() {
        assert_eq!(platform_ids(Some("")), [PC_WINDOWS]);
        assert_eq!(platform_ids(Some("PS2")), [8]);
        assert_eq!(platform_ids(Some("megadrive")), [29]);
        assert_eq!(platform_name(58), Some("SNES"));
        assert_eq!(platform_name(-1), None);
    }

    #[test]
    fn parses_a_game_with_portuguese_localization_companies_and_art() {
        let g = game(&games()[0], now()).unwrap();
        assert_eq!(g.source, "IGDB");
        assert_eq!(g.title.as_deref(), Some("Lanterna Distante: Edição Brasil"));
        assert!(
            g.description
                .as_deref()
                .unwrap()
                .starts_with("A lighthouse keeper")
        );
        assert_eq!(g.description_language.as_deref(), Some("en"));
        assert_eq!(
            g.genres,
            [
                translate("Adventure"),
                translate("Puzzle"),
                translate("Indie")
            ]
        );
        assert_eq!(g.themes, [translate("Fantasy"), translate("Mystery")]);
        assert_eq!(g.game_modes, [translate("Single player")]);
        assert_eq!(g.developer.as_deref(), Some("Maré Baixa Games"));
        assert_eq!(g.publisher.as_deref(), Some("Estúdio Farol"));
        assert_eq!(g.critic_score, Some(86));
        assert_eq!(g.franchise.as_deref(), Some("Lanterna"));
        assert_eq!(g.release_date, NaiveDate::from_ymd_opt(2024, 6, 1));
        assert_eq!(
            g.cover_url.as_deref(),
            Some("https://images.igdb.com/igdb/image/upload/t_cover_big/cobrfake.jpg")
        );
        assert_eq!(
            g.background_url.as_deref(),
            Some("https://images.igdb.com/igdb/image/upload/t_1080p/arfake01.jpg")
        );
        assert_eq!(g.steam_app_id, Some(4000001));
        assert_eq!(g.id_in("IGDB").as_deref(), Some("900001"));
        assert_eq!(g.id_in("GOG").as_deref(), Some("1400000001"));
        assert_eq!(g.source_ids.len(), 3); // fonte desconhecida (10) fica de fora
    }

    #[test]
    fn parses_sparse_games_with_screenshot_and_legacy_category() {
        let g = game(&games()[1], now()).unwrap();
        assert_eq!(g.description, None);
        assert_eq!(g.cover_url, None);
        assert_eq!(
            g.background_url.as_deref(),
            Some("https://images.igdb.com/igdb/image/upload/t_screenshot_huge/scfake02.jpg")
        );
        assert_eq!(g.franchise.as_deref(), Some("Série Lanterna"));
        assert_eq!(g.steam_app_id, Some(4000002));
        assert_eq!(g.critic_score, None);
        assert!(game(&serde_json::json!({"name": "sem id"}), now()).is_none());
        assert!(game(&serde_json::json!("lixo"), now()).is_none());
    }

    #[test]
    fn matches_show_year_and_platforms() {
        let first = match_of(&games()[0]).unwrap();
        let second = match_of(&games()[1]).unwrap();
        let mut expected = MetadataMatch::new("IGDB", "900001", "Lanterna Distante");
        expected.year = Some(2024);
        expected.platforms = Some("PC, Nintendo Switch".into());
        expected.cover_url =
            Some("https://images.igdb.com/igdb/image/upload/t_cover_small/cofake01.jpg".into());
        assert_eq!(first, expected);
        assert_eq!(second.label(), "Lanterna Distante 2 (2026) · SNES · IGDB");
    }

    #[test]
    fn external_games_map_store_ids_to_igdb_games() {
        let map =
            external_game_map(&serde_json::from_str(&fixture("external-games.json")).unwrap());
        assert_eq!(map["4000001"], 900001);
        assert_eq!(map["4000002"], 900002);
        assert!(external_game_map(&serde_json::json!({})).is_empty());
    }

    #[test]
    fn translates_known_terms_in_both_languages() {
        use crate::i18n::{Lang, t_in};
        for (en, pt, us) in [
            ("Role-playing (RPG)", "RPG", "RPG"),
            ("Shooter", "Tiro", "Shooter"),
            ("Co-operative", "Cooperativo", "Co-op"),
        ] {
            let key = TERMS.iter().find(|(e, _)| *e == en).unwrap().1;
            assert_eq!(t_in(Lang::PtBr, &format!("metadata.term.{key}")), pt);
            assert_eq!(t_in(Lang::EnUs, &format!("metadata.term.{key}")), us);
        }
        assert_eq!(translate("Gênero novo"), "Gênero novo");
        for (_, key) in TERMS {
            let k = format!("metadata.term.{key}");
            assert_ne!(t_in(Lang::PtBr, &k), k, "falta {k}");
        }
    }

    #[test]
    fn images_for_the_picker() {
        let v = serde_json::json!({"id":1,"cover":{"image_id":"capa"},"artworks":[{"image_id":"arte"}],"screenshots":[{"image_id":"tela"}],
             "game_localizations":[{"region":{"identifier":"br"},"cover":{"image_id":"capabr"}}]});
        let imgs = images(&v);
        let labels: Vec<_> = imgs.iter().map(|i| i.label.clone().unwrap()).collect();
        assert_eq!(
            labels,
            [
                t("metadata.art.igdb_cover"),
                t("metadata.art.igdb_cover_brazil"),
                t("metadata.art.igdb_artwork"),
                t("metadata.art.igdb_screenshot")
            ]
        );
        assert!(imgs[0].url.contains("t_cover_big_2x/capa.jpg"));
        assert!(imgs[0].preview().contains("t_cover_big/capa.jpg"));
        assert_eq!(imgs[2].kind, Some(ArtKind::Hero));
    }
}
