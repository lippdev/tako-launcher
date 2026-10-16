//! Biblioteca fictícia dos testes (port de `tests/Shared/SampleLibrary.cs`): só URLs das artes, nenhuma imagem.

use super::{Game, GameStore};
use chrono::{DateTime, NaiveDate, TimeDelta, Utc};
use serde::Deserialize;
use std::time::Duration;

/// O "agora" dos testes do C#.
pub(crate) fn now() -> DateTime<Utc> {
    "2026-10-14T20:00:00Z".parse().unwrap()
}

#[derive(Deserialize)]
struct Doc {
    games: Vec<Entry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    id: String,
    title: String,
    store: GameStore,
    size_label: Option<String>,
    accent: String,
    cover_url: Option<String>,
    hero_url: Option<String>,
    hero_alignment: Option<String>,
    genres: Option<Vec<String>>,
    description: Option<String>,
    note: Option<String>,
    release_date: Option<NaiveDate>,
    upcoming_release: Option<NaiveDate>,
    last_played_days_ago: Option<i64>,
    play_time_hours: f64,
    is_installed: bool,
    is_favorite: bool,
    added_days_ago: i64,
}

pub(crate) fn games() -> Vec<Game> {
    let doc: Doc = serde_json::from_str(include_str!("testdata/sample_library.json")).unwrap();
    let now = now();
    doc.games
        .into_iter()
        .map(|e| Game {
            id: e.id,
            title: e.title,
            store: Some(e.store),
            size_label: e.size_label,
            accent: e.accent,
            cover_uri: e.cover_url,
            hero_uri: e.hero_url,
            hero_alignment: e.hero_alignment,
            genres: e.genres.unwrap_or_default(),
            description: e.description.unwrap_or_default(),
            note: e.note,
            release_date: e.release_date,
            upcoming_release: e.upcoming_release,
            is_installed: e.is_installed,
            is_favorite: e.is_favorite,
            added_at: Some(now - TimeDelta::days(e.added_days_ago)),
            last_played_at: e
                .last_played_days_ago
                .map(|d| now - TimeDelta::days(d) - TimeDelta::hours(1)),
            play_time: Duration::from_secs_f64(e.play_time_hours * 3600.0),
            ..Game::default()
        })
        .collect()
}

#[test]
fn sample_library_loads_every_game_with_art_urls_only() {
    let games = games();
    assert_eq!(games.len(), 19);
    assert!(games.iter().all(|g| {
        g.cover_uri
            .as_deref()
            .is_none_or(|u| u.starts_with("https://"))
    }));
    assert!(
        games
            .iter()
            .any(|g| g.id == "phantom" && g.is_upcoming() && !g.can_play())
    );
    assert!(
        games
            .iter()
            .any(|g| g.store() == GameStore::Manual && g.note.is_some())
    );
}
