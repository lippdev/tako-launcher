//! API local de integração (EXT-002, ADR-0033): permite que ferramentas locais (Stream Deck, automações,
//! scripts e o Console Mode) consultem a biblioteca, o estado do jogo e acionem ações básicas.
//!
//! Regras de segurança (PRD §0, §7):
//! - Escuta exclusivamente em `127.0.0.1` (loopback).
//! - Exige autenticação por token (`Authorization: Bearer <token>`).
//! - Opt-in: desligada por padrão.
//!
//! Ponto de ligação com EXT-001:
//! Assina eventos de sessão (`GameStarting`, `GameStarted`, `GameExited`) para manter o estado
//! sincronizado sem necessidade de polling ativo.

pub mod runtime;
pub mod server;

pub use server::{
    API_ENABLED, API_PORT, API_TOKEN, ApiActionError, ApiConfig, ApiGame, ApiGameState, ApiHandler,
    ApiServerError, BACKUP_EXCLUDED_SETTINGS, DEFAULT_PORT, IMPORT_EXCLUDED_SETTINGS,
    MAX_CONCURRENT_CONNECTIONS, MAX_HEADER_LINE_BYTES, MAX_HEADERS_TOTAL_BYTES,
    MAX_REQUEST_LINE_BYTES, READ_TIMEOUT, REQUEST_TIMEOUT, RunningGameInfo, ServerHandle,
    WRITE_TIMEOUT, generate_token, is_backup_excluded, is_import_excluded, is_valid_token,
    start_server, start_server_with_timeout,
};
