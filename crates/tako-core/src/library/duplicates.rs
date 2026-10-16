//! Port de `Library/Sources/StoreDuplicates.cs` (STORE-010, ADR-0019): jogo instalado em mais de uma loja. Cada
//! cópia continua um item próprio (loja, jeito de abrir e tempo de jogo dela); as cópias só ficam ligadas. A cópia
//! sem appid da Steam herda o appid e as artes da cópia da Steam (capa certa sem esperar a busca por nome).

use super::title_matcher::tokens;
use super::{Game, GameStore};
use std::collections::HashMap;

/// Esquema das artes da Steam (`steam-art://570/cover`), o mesmo do C#.
pub const STEAM_ART_SCHEME: &str = "steam-art";

/// Chave de comparação: o título sem edição, ™/®, acentos e pontuação.
pub fn key(title: &str) -> String {
    tokens(title).join(" ")
}

pub fn link(games: &mut [Game]) {
    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, g) in games.iter().enumerate() {
        groups.entry(key(&g.title)).or_default().push(i);
    }
    for (k, copies) in groups {
        let mut stores: Vec<GameStore> = copies.iter().map(|&i| games[i].store()).collect();
        stores.sort();
        stores.dedup();
        let linked = !k.is_empty() && stores.len() > 1;
        let steam_app_id = if linked {
            copies
                .iter()
                .find(|&&i| games[i].store() == GameStore::Steam)
                .and_then(|&i| games[i].steam_app_id)
        } else {
            None
        };
        for &i in &copies {
            let game = &mut games[i];
            let own = game.store();
            game.also_installed_in = if linked {
                stores.iter().copied().filter(|s| *s != own).collect()
            } else {
                Vec::new()
            };
            if let Some(app_id) = steam_app_id.filter(|_| game.steam_app_id.is_none()) {
                game.steam_app_id = Some(app_id);
                game.cover_uri
                    .get_or_insert_with(|| format!("{STEAM_ART_SCHEME}://{app_id}/cover"));
                game.hero_uri
                    .get_or_insert_with(|| format!("{STEAM_ART_SCHEME}://{app_id}/hero"));
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn store_game(
    id: &str,
    title: &str,
    store: GameStore,
    steam_app_id: Option<u32>,
) -> Game {
    Game {
        id: id.into(),
        title: title.into(),
        store: Some(store),
        accent: "#000000".into(),
        is_installed: true,
        steam_app_id,
        ..Game::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_game_in_two_stores_keeps_both_items_linked() {
        let mut games = [
            store_game(
                "steam:4242",
                "Corrida Noturna",
                GameStore::Steam,
                Some(4242),
            ),
            store_game(
                "epic:Corrida Noturna",
                "Corrida Noturna™",
                GameStore::Epic,
                None,
            ),
            store_game("gog:9", "Cidade Submersa", GameStore::Gog, None),
        ];
        link(&mut games);
        let [steam, epic, other] = &games;
        assert_eq!(steam.also_installed_in, [GameStore::Epic]);
        assert_eq!(epic.also_installed_in, [GameStore::Steam]);
        assert!(other.also_installed_in.is_empty());
        assert_eq!(epic.steam_app_id, Some(4242)); // capa e dados da cópia da Steam, sem esperar a busca por nome
        assert_eq!(epic.cover_uri.as_deref(), Some("steam-art://4242/cover"));
        assert_eq!(other.steam_app_id, None);
    }

    #[test]
    fn same_title_in_one_store_is_not_linked() {
        let mut games = [
            store_game("epic:A", "Farol", GameStore::Epic, None),
            store_game("epic:B", "Farol", GameStore::Epic, None),
        ];
        link(&mut games);
        assert!(games.iter().all(|g| g.also_installed_in.is_empty()));
    }
}
