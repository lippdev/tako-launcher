//! Núcleo do Tako: modelo de jogo, biblioteca, consulta, metadados, navegação, sessão e persistência.
//!
//! Regra (PRD §5.1): nada aqui depende de UI nem de SO. Código específico de sistema fica em
//! `tako-platform` e `tako-sources`.

pub mod activities;
pub mod api;
pub mod console_mode;
pub mod devices;
pub mod diagnostics;
pub mod downloads;
pub mod i18n;
pub mod input;
pub mod library;
pub mod links;
pub mod metadata;
pub mod navigation;
pub mod paths;
pub mod persistence;
pub mod presentation;
pub mod profiles;
pub mod session;
pub mod supervision;
pub mod updates;
