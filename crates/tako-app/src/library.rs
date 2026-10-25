//! Adapta os critérios e os dados do core ao contrato da ponte; não replica busca nem ordenação.

use crate::Core;
use crate::protocol::{
    GameDetails, GameStatus, LibraryGame, LibraryListRequest, LibraryPage, LibraryTab,
    MAX_PAGE_LIMIT, RecentSession, SortOrder, Store,
};
use chrono::SecondsFormat;
use tako_core::library::{
    Game, GameStore, InstallFilter, LibraryCriteria, LibraryFilter, PlayStatus, query,
};

macro_rules! core_enum {
    ($wire:ident, $core:ident, $($variant:ident),+ $(,)?) => {
        impl From<$wire> for $core {
            fn from(value: $wire) -> Self {
                match value { $($wire::$variant => Self::$variant),+ }
            }
        }
        impl From<$core> for $wire {
            fn from(value: $core) -> Self {
                match value { $($core::$variant => Self::$variant),+ }
            }
        }
    };
}

core_enum!(
    Store, GameStore, Steam, Epic, Gog, Xbox, Ubisoft, Ea, BattleNet, Amazon, Heroic, Lutris,
    Bottles, Itch, Emulators, Manual
);
core_enum!(
    GameStatus, PlayStatus, Playing, Beaten, Paused, Abandoned, Wishlist
);

fn sort(value: SortOrder) -> query::LibrarySort {
    match value {
        SortOrder::RecentlyPlayed => query::LibrarySort::RecentlyPlayed,
        SortOrder::Name => query::LibrarySort::Name,
        SortOrder::PlayTime => query::LibrarySort::PlayTime,
        SortOrder::RecentlyAdded => query::LibrarySort::RecentlyAdded,
        SortOrder::Rating => query::LibrarySort::Rating,
        SortOrder::Size => query::LibrarySort::Size,
    }
}

pub(crate) fn criteria(request: LibraryListRequest) -> (LibraryCriteria, LibraryFilter) {
    let filter = match request.filter {
        LibraryTab::All => LibraryFilter::All,
        LibraryTab::Favorites => LibraryFilter::Favorites,
        LibraryTab::Installed => LibraryFilter::Installed,
    };
    let criteria = LibraryCriteria {
        search: request.search,
        sort: sort(request.sort),
        reversed: request.reversed,
        collections: request.collection.into_iter().collect(),
        tags: request.tags.into_iter().collect(),
        statuses: request.statuses.into_iter().map(Into::into).collect(),
        no_status: request.no_status,
        genres: request.genres.into_iter().collect(),
        install: match request.installed {
            None => InstallFilter::Any,
            Some(true) => InstallFilter::Installed,
            Some(false) => InstallFilter::NotInstalled,
        },
        ..Default::default()
    };
    (criteria, filter)
}

pub(crate) fn list(core: &Core, mut request: LibraryListRequest) -> LibraryPage {
    let smart = crate::organize::smart_collections(core)
        .into_iter()
        .find(|c| Some(c.id.as_str()) == request.collection.as_deref());
    if let Some(saved) = &smart {
        request.collection = None;
        if request.sort == SortOrder::RecentlyPlayed && !request.reversed {
            request.sort = wire_sort(saved.criteria.sort);
            request.reversed = saved.criteria.reversed;
        }
    }
    let (criteria, filter) = criteria(request.clone());
    let allowed = smart.as_ref().map(|c| {
        let criteria = c.criteria_in(core.language());
        query::apply_by(
            core.library()
                .games()
                .iter()
                .filter(|g| c.filter.matches(g)),
            |g| {
                let mut facts = tako_core::library::LibraryFacts::of(g);
                facts.source =
                    tako_core::presentation::game_text::store_name(core.language(), g.store());
                facts
            },
            &criteria,
        )
        .into_iter()
        .map(|g| g.id.as_str())
        .collect::<std::collections::HashSet<_>>()
    });
    // A API usa identificadores de loja, sem depender do idioma global usado por LibraryFacts::source.
    let games = core.library().games();
    let stores = request.stores;
    let matches = query::apply_by(
        games.iter().filter(|g| {
            filter.matches(g)
                && allowed
                    .as_ref()
                    .is_none_or(|ids| ids.contains(g.id.as_str()))
                && (stores.is_empty() || stores.iter().any(|s| GameStore::from(*s) == g.store()))
        }),
        |g| tako_core::library::LibraryFacts::of(g),
        &criteria,
    );
    LibraryPage {
        genres: matches
            .iter()
            .flat_map(|g| g.genres.iter().cloned())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect(),
        total: matches.len(),
        rows: matches
            .into_iter()
            .skip(request.offset as usize)
            .take(request.limit.min(MAX_PAGE_LIMIT) as usize)
            .map(row)
            .collect(),
    }
}

fn text(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.to_owned())
}

fn optional_text(value: Option<&str>) -> Option<String> {
    value.and_then(text)
}

fn nonempty<T>(values: Vec<T>) -> Option<Vec<T>> {
    (!values.is_empty()).then_some(values)
}

pub(crate) fn row(game: &Game) -> LibraryGame {
    LibraryGame {
        id: game.id.clone(),
        title: game.title.clone(),
        store: game.store().into(),
        installed: game.is_installed,
        favorite: game.is_favorite,
        status: game.play_status.map(Into::into),
        last_played_at: game
            .last_played_at
            .map(|d| d.to_rfc3339_opts(SecondsFormat::Millis, true)),
        play_time_seconds: game.play_time.as_secs(),
        accent: (game.accent.len() == 7
            && game.accent.starts_with('#')
            && game.accent.as_bytes()[1..]
                .iter()
                .all(u8::is_ascii_hexdigit))
        .then(|| game.accent.clone()),
        has_cover: game.shown_cover().is_some_and(|s| !s.trim().is_empty()),
        has_icon: game.shown_icon().is_some_and(|s| !s.trim().is_empty()),
        has_hero: game.shown_hero().is_some_and(|s| !s.trim().is_empty()),
        has_logo: game.shown_logo().is_some_and(|s| !s.trim().is_empty()),
        genres: nonempty(game.genres.clone()),
        publisher: optional_text(game.publisher.as_deref()),
    }
}

pub(crate) fn details(core: &Core, game: &Game) -> GameDetails {
    let mut sessions: Vec<_> = core.library().sessions(&game.id).iter().collect();
    sessions.sort_by_key(|s| std::cmp::Reverse(s.started_at));
    GameDetails {
        game: row(game),
        can_play: game.can_play(),
        upcoming: game.is_upcoming(),
        platform: optional_text(game.platform.as_deref()),
        description: text(&game.description),
        developer: optional_text(game.developer.as_deref()),
        publisher: optional_text(game.publisher.as_deref()),
        release_date: game.release_date.map(|d| d.to_string()),
        genres: nonempty(game.genres.clone()),
        metacritic_score: game.metacritic_score,
        critic_score: game.critic_score,
        recent_sessions: nonempty(
            sessions
                .into_iter()
                .take(10)
                .map(|s| RecentSession {
                    started_at: s.started_at.to_rfc3339_opts(SecondsFormat::Millis, true),
                    duration_seconds: s.duration.as_secs(),
                })
                .collect(),
        ),
        tags: nonempty(game.tags.iter().cloned().collect()),
        size_bytes: game.size_bytes,
        size_label: optional_text(game.size_label.as_deref()),
    }
}

pub(crate) fn wire_sort(value: query::LibrarySort) -> SortOrder {
    match value {
        query::LibrarySort::RecentlyPlayed => SortOrder::RecentlyPlayed,
        query::LibrarySort::Name => SortOrder::Name,
        query::LibrarySort::PlayTime => SortOrder::PlayTime,
        query::LibrarySort::RecentlyAdded => SortOrder::RecentlyAdded,
        query::LibrarySort::Rating => SortOrder::Rating,
        query::LibrarySort::Size => SortOrder::Size,
    }
}
