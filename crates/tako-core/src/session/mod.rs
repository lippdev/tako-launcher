//! Sessão de jogo: estados, configurações por jogo, abrir e acompanhar (MIG-016, MIG-024).
//!
//! Tudo aqui é regra pura sobre os traits de [`crate::devices`]; testes rodam com [`crate::devices::fake`].

pub mod audio;
pub mod restore;
pub mod screen;
pub mod settings;
pub mod state;
pub mod tracking;

pub use state::{SessionState, SessionStateMachine, SessionTransition};
pub use tracking::{GameRunStatus, GameRunTracker, TrackingPlan};
