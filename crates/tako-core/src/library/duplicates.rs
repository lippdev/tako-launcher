//! Port de `Library/Sources/StoreDuplicates.cs` (STORE-010, ADR-0019): jogo instalado em mais de uma loja. Cada
//! cópia continua um item próprio (loja, jeito de abrir e tempo de jogo dela); as cópias só ficam ligadas. A cópia
//! sem appid da Steam herda o appid e as artes da cópia da Steam (capa certa sem esperar a busca por nome).
//! O mesmo vale para o jogo da conta ainda não instalado em mais de uma loja: cada cópia recebe as ofertas de
//! instalação de todas (SRC-005).

use super::title_matcher::tokens;
use super::{Game, GameStore, InstallOffer};
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
        let stores_of = |games: &[Game], keep: &dyn Fn(&Game) -> bool| {
            let mut stores: Vec<GameStore> = copies
                .iter()
                .map(|&i| &games[i])
                .filter(|g| keep(g))
                .map(Game::store)
                .collect();
            stores.sort();
            stores.dedup();
            stores
        };
        let linked = !k.is_empty() && stores_of(games, &|_| true).len() > 1;
        // "Instalado em": só as cópias que de fato estão instaladas.
        let stores = stores_of(games, &|g| g.is_installed);
        let offers = if linked {
            offers(games, &copies)
        } else {
            Vec::new()
        };
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
            game.install_offers = if game.is_installed {
                Vec::new()
            } else {
                offers.clone()
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

/// Lojas por onde o jogo da conta pode ser instalado (cópias não instaladas, com endereço de instalação e sem
/// instalação em andamento); só vale com duas ou mais, senão o `install_uri` do próprio jogo basta.
/// Ordem: a loja da compra mais antiga primeiro seria o ideal, mas nenhuma loja entrega a data de compra offline
/// (só o banco do GOG Galaxy a tem, e não dá para comparar com lojas que não têm). Sem data, a ordem é a fixa do
/// enum `GameStore`: estável entre leituras e igual em todas as cópias.
fn offers(games: &[Game], copies: &[usize]) -> Vec<InstallOffer> {
    let mut offers: Vec<InstallOffer> = copies
        .iter()
        .map(|&i| &games[i])
        .filter(|g| !g.is_installed && g.installing.is_none())
        .filter_map(|g| {
            Some(InstallOffer {
                store: g.store(),
                install_uri: g.install_uri.clone()?,
                size: g.install_size,
            })
        })
        .collect();
    offers.sort_by_key(|o| o.store);
    offers.dedup_by_key(|o| o.store);
    if offers.len() < 2 {
        offers.clear();
    }
    offers
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

    fn owned(id: &str, title: &str, store: GameStore, uri: &str) -> Game {
        Game {
            is_installed: false,
            install_uri: Some(uri.into()),
            ..store_game(id, title, store, None)
        }
    }

    #[test]
    fn same_game_owned_in_two_stores_offers_both_in_a_stable_order() {
        let build = |reverse: bool| {
            let mut games = vec![
                owned(
                    "gog:9",
                    "Farol",
                    GameStore::Gog,
                    "goggalaxy://openGameView/9",
                ),
                Game {
                    install_size: Some(5 << 30),
                    ..owned("steam:7", "Farol", GameStore::Steam, "steam://install/7")
                },
                owned("epic:F", "Farol™", GameStore::Epic, "epic://install"),
                owned(
                    "gog:1",
                    "Outro",
                    GameStore::Gog,
                    "goggalaxy://openGameView/1",
                ),
            ];
            if reverse {
                games.reverse();
            }
            link(&mut games);
            games
        };
        for games in [build(false), build(true)] {
            let stores = |id: &str| -> Vec<GameStore> {
                let g = games.iter().find(|g| g.id == id).unwrap();
                g.install_offers.iter().map(|o| o.store).collect()
            };
            // A ordem não depende da ordem de leitura; todas as cópias mostram as mesmas ofertas.
            for id in ["steam:7", "epic:F", "gog:9"] {
                assert_eq!(
                    stores(id),
                    [GameStore::Steam, GameStore::Epic, GameStore::Gog],
                    "{id}"
                );
            }
            let steam = games.iter().find(|g| g.id == "steam:7").unwrap();
            assert_eq!(steam.install_offers[0].install_uri, "steam://install/7");
            assert_eq!(steam.install_offers[0].size, Some(5 << 30));
            assert_eq!(steam.install_offers[1].size, None);
            // Sozinho numa loja: sem ofertas (vale o install_uri do jogo).
            assert!(stores("gog:1").is_empty());
            // Nada instalado: ninguém "também instalado em".
            assert!(games.iter().all(|g| g.also_installed_in.is_empty()));
        }
    }

    #[test]
    fn installed_copy_is_not_an_offer_and_not_installed_copy_is_not_installed_in() {
        let mut games = [
            store_game("steam:7", "Farol", GameStore::Steam, Some(7)),
            owned("epic:F", "Farol", GameStore::Epic, "epic://install"),
            Game {
                installing: Some(Default::default()),
                ..owned("gog:9", "Farol", GameStore::Gog, "goggalaxy://x")
            },
        ];
        link(&mut games);
        let [steam, epic, gog] = &games;
        assert_eq!(epic.also_installed_in, [GameStore::Steam]);
        assert!(steam.also_installed_in.is_empty()); // a Epic e a GOG não têm o jogo instalado
        // Uma só loja disponível para instalar (a GOG já está instalando): sem ofertas.
        assert!(games.iter().all(|g| g.install_offers.is_empty()));
        assert!(gog.installing.is_some());
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
