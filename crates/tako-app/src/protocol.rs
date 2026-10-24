//! Mensagens da ponte entre a interface web e o núcleo (contrato em `docs/web/ponte.md`).
//!
//! Cada chamada (`Call`) tem uma resposta (`Reply`) com o **mesmo nome**; o TypeScript gerado usa isso para tipar a
//! resposta pelo nome da chamada. Para acrescentar uma chamada: uma variante em `Call`, outra com o mesmo
//! `rename` em `Reply`, o tratamento em `AppService::handle` e um teste; `cargo test -p tako-app` regenera os tipos.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Versão do contrato. Sobe quando uma mudança quebra a interface já publicada (campo removido ou renomeado).
pub const BRIDGE_VERSION: u32 = 1;

/// Pedido da interface: `{ "v": 1, "id": 7, "name": "app.info", "payload": {…} }`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Request {
    pub v: u32,
    /// Escolhido pela interface; volta igual na resposta. Contador simples, pode dar a volta.
    pub id: u32,
    #[serde(flatten)]
    pub call: Call,
}

/// Resposta a um `Request`: `{ "v": 1, "id": 7, "ok": true, "reply": {…} }` ou `ok: false` com `error`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Response {
    pub v: u32,
    pub id: u32,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply: Option<Reply>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<BridgeError>,
}

impl Response {
    pub fn ok(id: u32, reply: Reply) -> Self {
        Self {
            v: BRIDGE_VERSION,
            id,
            ok: true,
            reply: Some(reply),
            error: None,
        }
    }

    pub fn err(id: u32, error: BridgeError) -> Self {
        Self {
            v: BRIDGE_VERSION,
            id,
            ok: false,
            reply: None,
            error: Some(error),
        }
    }
}

/// Aviso do núcleo sem pedido: `{ "v": 1, "event": { "name": "library.changed", "payload": … } }`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct EventEnvelope {
    pub v: u32,
    pub event: Event,
}

impl From<Event> for EventEnvelope {
    fn from(event: Event) -> Self {
        Self {
            v: BRIDGE_VERSION,
            event,
        }
    }
}

/// Chamadas. Nomes em `dominio.acao`; consultas com substantivo (`library.list`), comandos com verbo
/// (`game.play`).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(tag = "name", content = "payload")]
#[ts(export)]
pub enum Call {
    /// Versão do app e do contrato; a interface confere a versão ao abrir.
    #[serde(rename = "app.info")]
    AppInfo,
}

/// Respostas, uma por chamada e com o mesmo nome.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(tag = "name", content = "data")]
#[ts(export)]
pub enum Reply {
    #[serde(rename = "app.info")]
    AppInfo(AppInfo),
}

/// Eventos do núcleo para a interface.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(tag = "name", content = "payload")]
#[ts(export)]
pub enum Event {
    /// A biblioteca mudou (fonte lida, jogo editado, metadados chegaram): a interface consulta de novo o que mostra.
    #[serde(rename = "library.changed")]
    LibraryChanged,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppInfo {
    pub version: String,
    pub bridge_version: u32,
}

/// Erro de uma chamada. `message` é para log; a interface mostra texto dos catálogos pelo `code`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS, thiserror::Error)]
#[error("{code:?}: {message}")]
#[ts(export)]
pub struct BridgeError {
    pub code: ErrorCode,
    pub message: String,
}

impl BridgeError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ErrorCode {
    /// Mensagem que não é JSON válido, grande demais ou fora do formato.
    BadRequest,
    /// Nome de chamada que este núcleo não conhece (interface mais nova que o núcleo).
    UnknownCall,
    /// Versão do contrato diferente da do núcleo.
    VersionMismatch,
    /// O que a chamada pede não existe (jogo, coleção…).
    NotFound,
    /// A chamada é válida mas não pode ser feita agora (jogo já aberto, download em curso…).
    Conflict,
    /// Falha do núcleo; detalhes no log.
    Internal,
}
