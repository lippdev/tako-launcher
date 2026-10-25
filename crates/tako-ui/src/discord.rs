//! Discord Rich Presence (EXT-004): a regra e o IPC vivem em `tako_core::session::discord`, compartilhados com o
//! núcleo da versão web (WEB-34).

pub use tako_core::session::discord::{DISCORD_CLIENT_ID, Presence, available};
