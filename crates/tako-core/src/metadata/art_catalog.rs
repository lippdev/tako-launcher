//! Port da lógica de `ArtCatalog.cs` e de `MetadataService.KnownIds`: as fontes do seletor de imagens
//! para o jogo e as imagens de cada uma, pelos ids casados ou fixados em "Fontes e IDs".

use super::agent::{MetadataAgent, PinnedIds};
use super::art::{ArtCandidate, FILE_SOURCE, local_images, steam_candidates};
use super::igdb::IgdbProvider;
use super::screenscraper::ScreenScraperProvider;
use super::steamgriddb::{SteamGridDbFilter, SteamGridDbProvider};
use super::title_matcher;
use super::{GameMetadata, MetadataLookup, MetadataMatch, MetadataProvider, external_ids, sources};
use crate::library::{ArtKind, Game};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Ids do jogo em cada fonte: os da loja, os do cache e os fixados (vazio = "nenhum" tira a fonte).
pub fn known_ids(
    game: &Game,
    cached: Option<&GameMetadata>,
    pinned: &PinnedIds,
) -> BTreeMap<String, String> {
    let mut ids = external_ids::for_game(game);
    if let Some(app) = game.steam_app_id {
        ids.entry(external_ids::STEAM.into())
            .or_insert_with(|| app.to_string());
    }
    if let Some(c) = MetadataAgent::normalize(cached.cloned()) {
        ids.extend(c.source_ids);
    }
    for (s, id) in pinned {
        if id.is_empty() {
            ids.remove(s);
        } else {
            ids.insert(s.clone(), id.clone());
        }
    }
    ids
}

/// O que o seletor pede: as imagens de uma fonte, de um tipo.
#[derive(Debug, Clone)]
pub struct ArtQuery<'a> {
    pub game: &'a Game,
    pub source: &'a str,
    pub kind: ArtKind,
    /// Filtro do SteamGridDB; `None` = o padrão do tipo.
    pub filter: Option<SteamGridDbFilter>,
    /// Outro jogo do SteamGridDB (busca livre); `None` = o casado.
    pub steamgriddb_game: Option<String>,
    pub page: u32,
}

/// Fontes ligadas. Downloads só quando a miniatura aparece (cache de artes).
pub struct ArtCatalog {
    pub igdb: Option<Arc<IgdbProvider>>,
    pub screenscraper: Option<Arc<ScreenScraperProvider>>,
    pub steamgriddb: Option<Arc<SteamGridDbProvider>>,
    /// Imagens, Downloads e Área de Trabalho do usuário (a plataforma informa).
    pub picture_folders: Vec<PathBuf>,
}

fn on<P: MetadataProvider>(p: &Option<Arc<P>>) -> Option<&Arc<P>> {
    p.as_ref().filter(|p| p.is_enabled())
}

impl ArtCatalog {
    /// Fontes para o jogo, na ordem das abas, com "Arquivo do PC" no fim.
    pub fn sources(&self, game: &Game, ids: &BTreeMap<String, String>) -> Vec<&'static str> {
        let mut list = Vec::new();
        if ids.contains_key(sources::STEAM) || game.steam_app_id.is_some() {
            list.push(sources::STEAM);
        }
        if on(&self.igdb).is_some() {
            list.push(sources::IGDB);
        }
        if on(&self.screenscraper).is_some() && game.is_rom() {
            list.push(sources::SCREENSCRAPER);
        }
        if on(&self.steamgriddb).is_some() {
            list.push(sources::STEAMGRIDDB);
        }
        list.push(FILE_SOURCE);
        list
    }

    /// Bloqueante (rede e disco): chamar fora da thread da UI.
    pub fn list(&self, q: &ArtQuery, ids: &BTreeMap<String, String>) -> Vec<ArtCandidate> {
        let of_kind = |v: Vec<ArtCandidate>| -> Vec<ArtCandidate> {
            v.into_iter().filter(|c| c.kind == Some(q.kind)).collect()
        };
        match q.source {
            sources::STEAM => ids
                .get(sources::STEAM)
                .and_then(|s| s.parse().ok())
                .or(q.game.steam_app_id)
                .map(|app| of_kind(steam_candidates(app)))
                .unwrap_or_default(),
            sources::IGDB => match (on(&self.igdb), ids.get(sources::IGDB)) {
                (Some(p), Some(id)) => of_kind(p.images(id)),
                _ => Vec::new(),
            },
            sources::SCREENSCRAPER => {
                match (on(&self.screenscraper), ids.get(sources::SCREENSCRAPER)) {
                    (Some(p), Some(id)) => of_kind(p.media(id)),
                    _ => Vec::new(),
                }
            }
            sources::STEAMGRIDDB => {
                let Some(p) = on(&self.steamgriddb) else {
                    return Vec::new();
                };
                let game_id = q
                    .steamgriddb_game
                    .clone()
                    .or_else(|| self.steamgriddb_game(q.game, ids).map(|m| m.id));
                let filter = q
                    .filter
                    .clone()
                    .unwrap_or_else(|| SteamGridDbFilter::new(q.kind));
                game_id
                    .map(|id| p.images(&id, &filter, q.page))
                    .unwrap_or_default()
            }
            FILE_SOURCE => local_images::find(&self.picture_folders, 60)
                .into_iter()
                .map(|f| ArtCandidate {
                    label: f.file_name().map(|n| n.to_string_lossy().into_owned()),
                    ..ArtCandidate::new(FILE_SOURCE, q.kind, f.to_string_lossy())
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Jogo do SteamGridDB em uso para este jogo (casado ou fixado), para o seletor mostrar.
    pub fn steamgriddb_game(
        &self,
        game: &Game,
        ids: &BTreeMap<String, String>,
    ) -> Option<MetadataMatch> {
        let p = on(&self.steamgriddb)?;
        let lookup = MetadataLookup {
            external_ids: ids.clone(),
            ..MetadataLookup::for_game(game)
        };
        if let Some(pinned) = ids.get(sources::STEAMGRIDDB) {
            return Some(
                p.search(&lookup)
                    .into_iter()
                    .find(|m| &m.id == pinned)
                    .unwrap_or_else(|| {
                        MetadataMatch::new(sources::STEAMGRIDDB, pinned, &game.title)
                    }),
            );
        }
        let results = p.search(&lookup);
        results
            .iter()
            .find(|r| r.is_exact)
            .or_else(|| title_matcher::best(&game.title, &results))
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::GameStore;

    #[test]
    fn known_ids_merge_store_cache_and_pins() {
        let g = Game {
            id: "steam:570".into(),
            store: Some(GameStore::Steam),
            steam_app_id: Some(570),
            ..Default::default()
        };
        let cached = GameMetadata {
            source_ids: [
                ("IGDB".to_owned(), "1".to_owned()),
                ("SteamGridDB".to_owned(), "9".to_owned()),
            ]
            .into(),
            ..GameMetadata::new("IGDB")
        };
        let pinned = PinnedIds::from([
            ("IGDB".to_owned(), "2".to_owned()),
            ("SteamGridDB".to_owned(), String::new()),
        ]);
        let ids = known_ids(&g, Some(&cached), &pinned);
        assert_eq!(ids["Steam"], "570");
        assert_eq!(ids["IGDB"], "2");
        assert!(!ids.contains_key("SteamGridDB"));

        let catalog = ArtCatalog {
            igdb: None,
            screenscraper: None,
            steamgriddb: None,
            picture_folders: Vec::new(),
        };
        assert_eq!(catalog.sources(&g, &ids), [sources::STEAM, FILE_SOURCE]);
        let q = ArtQuery {
            game: &g,
            source: sources::STEAM,
            kind: ArtKind::Hero,
            filter: None,
            steamgriddb_game: None,
            page: 0,
        };
        let hero = catalog.list(&q, &ids);
        assert_eq!(hero.len(), 1);
        assert_eq!(hero[0].url, "steam-art://570/hero");
        assert!(
            catalog
                .list(
                    &ArtQuery {
                        source: sources::IGDB,
                        ..q
                    },
                    &ids
                )
                .is_empty()
        );
    }
}
