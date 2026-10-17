//! Modelo de jogo e biblioteca (port de `Launcher.Core/Library`).
//!
//! Os tipos deste arquivo são o contrato comum entre fontes (`tako-sources`), metadados, sessão e UI.

pub mod collections;
pub mod criteria;
pub mod duplicates;
pub mod game;
pub mod game_library;
pub mod manual;
pub mod query;
pub mod search_text;
pub mod storage;
pub mod tags;
pub mod title_matcher;

#[cfg(test)]
pub(crate) mod sample;

pub use collections::{
    CollectionError, CollectionInfo, CollectionLibrary, CollectionStore, NameProblem,
};
pub use criteria::{InstallFilter, LibraryCriteria, LibraryFacts};
pub use game::{ArtKind, Game, GameStore, GameTrackingStrategy, PlayStatus, media};
pub use game_library::{
    GameLibrary, LibraryData, LibraryError, LibraryStore, MAX_SESSIONS, MemoryLibraryStore,
};
pub use manual::{GameStateRecord, ManualGameRecord, PlaySession};
pub use query::{FacetValue, LibraryFilter, LibrarySort};

use chrono::{DateTime, Utc};

/// Relógio injetado (o `TimeProvider` do C#): testes passam um relógio fixo.
pub type Clock = Box<dyn Fn() -> DateTime<Utc> + Send + Sync>;

/// Relógio do sistema.
pub fn system_clock() -> Clock {
    Box::new(Utc::now)
}
