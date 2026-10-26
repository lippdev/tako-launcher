//! Código comum do Linux e do macOS. Os parsers e a lógica pura não dependem de SO: compilam e
//! são testados em qualquer plataforma; só [`run`] (processos, sinais, arquivos) é `cfg(unix)`.

pub mod kscreen;
pub mod mutter;
pub mod outputs;
pub mod pulse;
pub mod session;
pub mod text;
pub mod wlr;

#[cfg(unix)]
pub mod run;
