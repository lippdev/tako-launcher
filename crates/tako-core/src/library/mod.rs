//! Modelo de jogo e biblioteca (port de `Launcher.Core/Library`).
//!
//! Os tipos deste arquivo são o contrato comum entre fontes (`tako-sources`), metadados, sessão e UI.

pub mod game;

pub use game::{ArtKind, Game, GameStore, GameTrackingStrategy, media};
