//! Interface do Tako (MIG-029): tema Slint carregado em tempo de execução (PRD §7), estado e foco em Rust.

pub mod app;
pub mod art;
pub mod bridge;
pub mod demo;
pub mod effects;
pub mod gamepad;
pub mod images;
pub mod model;
pub mod session;
pub mod shell;
pub mod sources;
pub mod tasks;
pub mod theme;
pub mod values;

/// Versão do app: a da release (CI define `TAKO_VERSION`) ou a do Cargo.
pub fn version() -> &'static str {
    option_env!("TAKO_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}
