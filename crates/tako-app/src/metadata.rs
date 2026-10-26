//! Fila compartilhada com o Slint: rede e cache fora da ponte; escolhas atuais acompanham cada resultado.

use std::path::PathBuf;
use std::sync::Arc;
use tako_core::library::Game;
use tako_core::metadata::GameMetadata;
use tako_core::metadata::agent::{MetadataAgent, PinnedIds};
use tako_core::metadata::cache::MetadataCache;
use tako_core::metadata::worker::{MetadataUpdated, MetadataWorker, SPACING};

pub(crate) struct MetadataQueue {
    pub worker: MetadataWorker,
}

impl MetadataQueue {
    pub fn new(
        agent: Arc<MetadataAgent>,
        cache: PathBuf,
        notify: impl Fn(MetadataUpdated, PinnedIds) + Send + 'static,
    ) -> Self {
        Self {
            worker: MetadataWorker::spawn_observed(
                agent,
                MetadataCache::new(cache),
                SPACING,
                notify,
            ),
        }
    }
}

impl crate::Core {
    pub(crate) fn refresh_metadata(&mut self, game_id: &str) {
        if let Some(game) = self
            .metadata_base_game(game_id)
            .or_else(|| self.library.find(game_id).cloned())
        {
            self.art.refresh_metadata(game, &self.pinned_ids);
        }
    }
}

/// Reaplica sobre a loja: campos ausentes no ID novo não conservam a correspondência antiga.
/// Preferências, tempo de jogo, instalação e imagens fixadas continuam no jogo vivo.
pub(crate) fn apply(game: &mut Game, base: Option<&Game>, metadata: &GameMetadata) {
    let mut fresh = base.cloned().unwrap_or_else(|| Game {
        id: game.id.clone(),
        title: game.title.clone(),
        store: game.store,
        ..Game::default()
    });
    metadata.apply_to(&mut fresh);
    game.description = fresh.description;
    game.genres = fresh.genres;
    game.themes = fresh.themes;
    game.game_modes = fresh.game_modes;
    game.franchise = fresh.franchise;
    game.release_date = fresh.release_date;
    game.developer = fresh.developer;
    game.publisher = fresh.publisher;
    game.metacritic_score = fresh.metacritic_score;
    game.critic_score = fresh.critic_score;
    game.players = fresh.players;
    game.cover_uri = fresh.cover_uri;
    game.hero_uri = fresh.hero_uri;
    game.media = fresh.media;
    game.icon_origin = fresh.icon_origin;
    game.steam_app_id = fresh.steam_app_id;
}
