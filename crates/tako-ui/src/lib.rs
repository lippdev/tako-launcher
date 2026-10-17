//! Interface do Tako (MIG-029): tema Slint carregado em tempo de execução (PRD §7), estado e foco em Rust.

pub mod app;
pub mod art;
pub mod bridge;
pub mod demo;
pub mod discord;
pub mod effects;
pub mod gamepad;
pub mod images;
pub mod metadata;
pub mod model;
pub mod playnite;
pub mod screen_test;
pub mod session;
pub mod shell;
pub mod sources;
pub mod steam_owned;
pub mod tasks;
pub mod theme;
pub mod values;

#[cfg(test)]
mod ui_tests;

/// Versão do app: a da release (CI define `TAKO_VERSION`) ou a do Cargo.
pub fn version() -> &'static str {
    option_env!("TAKO_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}

/// O idioma é global: testes que dependem dele (ou o trocam) rodam um por vez.
#[cfg(test)]
pub(crate) static LANG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
