//! Mensagens da ponte entre a interface web e o núcleo (contrato em `docs/web/ponte.md`).
//!
//! Cada chamada (`Call`) tem uma resposta (`Reply`) com o **mesmo nome**; o TypeScript gerado usa isso para tipar a
//! resposta pelo nome da chamada. Para acrescentar uma chamada: uma variante em `Call`, outra com o mesmo
//! `rename` em `Reply`, o tratamento em `AppService::handle` e um teste; `cargo test -p tako-app` regenera os tipos.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Versão do contrato. Sobe quando uma mudança quebra a interface já publicada (campo removido ou renomeado).
pub const BRIDGE_VERSION: u32 = 1;

/// Maior mensagem aceita da interface; confere antes de alocar o JSON.
pub const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

/// Valida o envelope antes do payload, para distinguir chamada desconhecida de enum inválido.
pub fn parse_request(text: &str) -> Result<Request, (u32, BridgeError)> {
    let bad_request = |message| BridgeError::new(ErrorCode::BadRequest, message);
    if text.len() > MAX_MESSAGE_BYTES {
        return Err((0, bad_request("mensagem grande demais".into())));
    }
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| (0, bad_request(e.to_string())))?;
    let id = value
        .get("id")
        .and_then(serde_json::Value::as_u64)
        .and_then(|id| u32::try_from(id).ok())
        .unwrap_or(0);
    #[derive(Deserialize)]
    struct Header {
        v: u32,
        id: u32,
        name: String,
    }
    let header = Header::deserialize(&value).map_err(|e| (id, bad_request(e.to_string())))?;
    if header.v != BRIDGE_VERSION {
        return Err((
            header.id,
            BridgeError::new(
                ErrorCode::VersionMismatch,
                format!("interface v{}, núcleo v{BRIDGE_VERSION}", header.v),
            ),
        ));
    }
    if !matches!(
        header.name.as_str(),
        "app.info"
            | "window.action"
            | "ui.state"
            | "ui.state.set"
            | "library.list"
            | "game.details"
            | "game.play"
            | "library.scan"
            | "library.sources"
    ) {
        return Err((
            header.id,
            BridgeError::new(
                ErrorCode::UnknownCall,
                format!("chamada desconhecida: {}", header.name),
            ),
        ));
    }
    if matches!(header.name.as_str(), "library.scan" | "library.sources")
        && value.get("payload").is_some()
    {
        return Err((id, bad_request("chamada não aceita argumentos".into())));
    }
    let request: Request =
        serde_json::from_value(value).map_err(|e| (id, bad_request(e.to_string())))?;
    if let Call::LibraryList(criteria) = &request.call {
        criteria.validate().map_err(|error| (id, error))?;
    }
    Ok(request)
}

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
    /// Botões da barra da janela (BarraDaJanela) e tela cheia; quem executa é o hospedeiro.
    #[serde(rename = "window.action")]
    WindowAction(WindowAction),
    /// Onde a interface estava antes de fechar (D-10); `json: null` na primeira abertura.
    #[serde(rename = "ui.state")]
    UiState,
    /// Guarda onde a interface está (tela, jogo, filtros, rolagem), em JSON da própria interface, até 64 KiB.
    #[serde(rename = "ui.state.set")]
    UiStateSet(UiState),
    #[serde(rename = "library.list")]
    LibraryList(
        #[serde(with = "object_payload")]
        #[ts(as = "LibraryListRequest")]
        LibraryListRequest,
    ),
    #[serde(rename = "game.details")]
    GameDetails(
        #[serde(with = "object_payload")]
        #[ts(as = "GameRequest")]
        GameRequest,
    ),
    #[serde(rename = "game.play")]
    GamePlay(
        #[serde(with = "object_payload")]
        #[ts(as = "GameRequest")]
        GameRequest,
    ),
    #[serde(rename = "library.scan")]
    LibraryScan,
    #[serde(rename = "library.sources")]
    LibrarySources,
}

/// Respostas, uma por chamada e com o mesmo nome.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(tag = "name", content = "data")]
#[ts(export)]
pub enum Reply {
    #[serde(rename = "app.info")]
    AppInfo(AppInfo),
    #[serde(rename = "window.action")]
    WindowAction,
    #[serde(rename = "ui.state")]
    UiState(UiState),
    #[serde(rename = "ui.state.set")]
    UiStateSet,
    #[serde(rename = "library.list")]
    LibraryList(LibraryPage),
    #[serde(rename = "game.details")]
    GameDetails(Box<GameDetails>),
    #[serde(rename = "game.play")]
    GamePlay,
    #[serde(rename = "library.scan")]
    LibraryScan,
    #[serde(rename = "library.sources")]
    LibrarySources(LibrarySources),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum WindowAction {
    Minimize,
    /// Maximiza ou restaura, conforme o estado atual.
    ToggleMaximize,
    Close,
    /// Começa a arrastar a janela (barra própria sem moldura do sistema).
    StartDrag,
    /// Troca para a tela cheia (Big Picture).
    BigPicture,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct UiState {
    pub json: Option<String>,
}

/// Eventos do núcleo para a interface.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(tag = "name", content = "payload")]
#[ts(export)]
pub enum Event {
    /// A biblioteca mudou (fonte lida, jogo editado, metadados chegaram): a interface consulta de novo o que mostra.
    #[serde(rename = "library.changed")]
    LibraryChanged,
    #[serde(rename = "library.scan.progress")]
    LibraryScanProgress(LibrarySources),
}

/// Estado consultável também depois de reabrir a interface. Durante a medição, a leitura segue em andamento.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LibrarySources {
    pub scanning: bool,
    pub completed: usize,
    pub total: usize,
    pub measuring_sizes: bool,
    pub sources: Vec<LibrarySourceReport>,
    // Falha do trabalhador; os avisos de cada loja ficam no relatório dela.
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LibrarySourceReport {
    pub store: Store,
    pub name: String,
    pub enabled: bool,
    pub found: usize,
    pub client_installed: bool,
    // A tako-sources entrega erros e avisos (inclusive cliente ausente) pelo mesmo campo notice.
    pub error: Option<String>,
    // ISO-8601 em UTC; null para fonte desligada, que não foi lida.
    pub last_scanned_at: Option<String>,
    #[ts(type = "number | null")]
    pub elapsed_ms: Option<u64>,
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

/// Tamanho padrão e teto de uma página; o total nunca depende deste recorte.
pub const DEFAULT_PAGE_LIMIT: u32 = 50;
pub const MAX_PAGE_LIMIT: u32 = 200;

// Serde também aceita structs como sequências. Na ponte o contrato exige objetos, inclusive para
// consultas cujos campos têm padrão (caso contrário [] seria uma consulta válida).
mod object_payload {
    use serde::{Deserialize, Serialize};

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: serde::Deserializer<'de>,
        T: serde::de::DeserializeOwned,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        if !value.is_object() {
            return Err(serde::de::Error::custom("payload deve ser um objeto"));
        }
        serde_json::from_value(value).map_err(serde::de::Error::custom)
    }

    pub fn serialize<S: serde::Serializer, T: Serialize>(
        value: &T,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value.serialize(serializer)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Store {
    Steam,
    Epic,
    Gog,
    Xbox,
    Ubisoft,
    Ea,
    BattleNet,
    Amazon,
    Heroic,
    Lutris,
    Bottles,
    Itch,
    Emulators,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum GameStatus {
    Playing,
    Beaten,
    Paused,
    Abandoned,
    Wishlist,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SortOrder {
    RecentlyPlayed,
    Name,
    PlayTime,
    RecentlyAdded,
    Rating,
    Size,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum LibraryTab {
    All,
    Favorites,
    Installed,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
#[ts(export)]
pub struct LibraryListRequest {
    #[ts(as = "Option<LibraryTab>", optional)]
    pub filter: LibraryTab,
    #[ts(as = "Option<String>", optional)]
    pub search: String,
    #[ts(as = "Option<SortOrder>", optional)]
    pub sort: SortOrder,
    #[ts(as = "Option<bool>", optional)]
    pub reversed: bool,
    // Id da coleção; a resolução das coleções automáticas entra na WEB-01b.
    #[ts(optional = nullable)]
    pub collection: Option<String>,
    // Dentro de cada grupo vale qualquer um; entre grupos, todos precisam valer (regra do core).
    #[ts(as = "Option<Vec<String>>", optional)]
    pub tags: Vec<String>,
    #[ts(as = "Option<Vec<GameStatus>>", optional)]
    pub statuses: Vec<GameStatus>,
    #[ts(as = "Option<bool>", optional)]
    pub no_status: bool,
    #[ts(as = "Option<Vec<Store>>", optional)]
    pub stores: Vec<Store>,
    // null = todos, true = instalados, false = não instalados.
    #[ts(optional = nullable)]
    pub installed: Option<bool>,
    #[ts(as = "Option<u32>", optional)]
    pub offset: u32,
    // Valores acima de MAX_PAGE_LIMIT são reduzidos ao teto; zero retorna só o total.
    #[ts(as = "Option<u32>", optional)]
    pub limit: u32,
}

impl Default for LibraryListRequest {
    fn default() -> Self {
        Self {
            filter: LibraryTab::All,
            search: String::new(),
            sort: SortOrder::RecentlyPlayed,
            reversed: false,
            collection: None,
            tags: Vec::new(),
            statuses: Vec::new(),
            no_status: false,
            stores: Vec::new(),
            installed: None,
            offset: 0,
            limit: DEFAULT_PAGE_LIMIT,
        }
    }
}

impl LibraryListRequest {
    pub fn validate(&self) -> Result<(), BridgeError> {
        if self.search.chars().count() > 256
            || self.tags.len() > 64
            || self.statuses.len() > 64
            || self.stores.len() > 64
            || self.tags.iter().any(|tag| tag.chars().count() > 128)
            || self
                .collection
                .as_ref()
                .is_some_and(|id| id.chars().count() > 128)
        {
            return Err(BridgeError::new(
                ErrorCode::BadRequest,
                "critérios da biblioteca excedem os limites",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct GameRequest {
    pub game_id: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LibraryPage {
    pub rows: Vec<LibraryGame>,
    pub total: usize,
}

/// Só dados para desenhar; imagens são pedidas por id em tako-img://, nunca por URI da fonte.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LibraryGame {
    pub id: String,
    pub title: String,
    pub store: Store,
    pub installed: bool,
    pub favorite: bool,
    pub status: Option<GameStatus>,
    // ISO-8601 em UTC; null = nunca jogado.
    pub last_played_at: Option<String>,
    #[ts(type = "number")]
    pub play_time_seconds: u64,
    pub accent: Option<String>,
    pub has_cover: bool,
    pub has_icon: bool,
    pub has_hero: bool,
    pub has_logo: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameDetails {
    pub game: LibraryGame,
    pub can_play: bool,
    pub upcoming: bool,
    pub platform: Option<String>,
    pub description: Option<String>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    // Data ISO-8601 (sem horário).
    pub release_date: Option<String>,
    pub genres: Option<Vec<String>>,
    pub metacritic_score: Option<i32>,
    pub critic_score: Option<i32>,
    // Mais recentes primeiro, até dez; null = sem histórico.
    pub recent_sessions: Option<Vec<RecentSession>>,
    pub tags: Option<Vec<String>>,
    #[ts(type = "number | null")]
    pub size_bytes: Option<u64>,
    pub size_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RecentSession {
    pub started_at: String,
    #[ts(type = "number")]
    pub duration_seconds: u64,
}
