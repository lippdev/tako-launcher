//! Sessão de jogo: estados, configurações por jogo, abrir e acompanhar (MIG-024, MIG-024, WEB-20).
//!
//! Tudo aqui é regra pura sobre os traits de [`crate::devices`]; testes rodam com [`crate::devices::fake`].
//! Abrir e acompanhar ([`launch`], [`running`]) servem às duas interfaces: a Slint e o núcleo da versão web.

pub mod audio;
pub mod discord;
pub mod events;
pub mod launch;
pub mod restore;
pub mod running;
pub mod screen;
pub mod settings;
pub mod state;
pub mod tracking;

pub use launch::{SessionDevices, Started, TrackRequest};
pub use running::GameSession;
pub use state::{SessionState, SessionStateMachine, SessionTransition};
pub use tracking::{GameEnd, GameRunStatus, GameRunTracker, TrackingPlan};
