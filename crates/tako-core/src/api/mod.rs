//! API local de integração (EXT-002, ADR-0030): permite que ferramentas locais (Stream Deck, automações,
//! scripts e o Console Mode) consultem a biblioteca, o estado do jogo e acionem ações básicas.
//!
//! Regras de segurança (PRD §0, §7):
//! - Escuta exclusivamente em `127.0.0.1` (loopback).
//! - Exige autenticação por token (`Authorization: Bearer <token>`).
//! - Opt-in: desligada por padrão.
//!
//! Ponto de ligação com EXT-001:
//! Quando o PR #132 (`session::events`) for integrado, a API local poderá assinar eventos de sessão
//! (`GameStarting`, `GameStarted`, `GameExited`) para transmitir atualizações reativas ou manter o estado
//! sincronizado sem necessidade de polling ativo.

pub mod server;

pub use server::{
    API_ENABLED, API_PORT, API_TOKEN, ApiActionError, ApiConfig, ApiGame, ApiGameState, ApiHandler,
    ApiServerError, DEFAULT_PORT, RunningGameInfo, ServerHandle, generate_token, start_server,
};
