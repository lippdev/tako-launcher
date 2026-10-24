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
            | "settings.get"
            | "settings.set"
            | "collections.list"
            | "collections.create"
            | "collections.rename"
            | "collections.delete"
            | "collections.add_games"
            | "collections.remove_games"
            | "collections.smart.set"
            | "tags.list"
            | "game.tags.set"
            | "game.status.set"
            | "game.favorite.set"
            | "system.displays"
            | "system.audio_outputs"
            | "i18n.catalog"
            | "downloads.list"
            | "downloads.summary"
            | "downloads.pause"
            | "downloads.resume"
            | "downloads.cancel"
            | "downloads.sources"
            | "downloads.add"
            | "game.settings"
            | "game.settings.set"
            | "profile.summary"
            | "profile.sessions"
            | "profile.collections"
            | "profiles.list"
            | "profiles.create"
            | "profiles.rename"
            | "profiles.delete"
            | "profiles.ask.set"
            | "profiles.switch"
            | "game.running"
            | "game.resume"
            | "game.end"
    ) {
        return Err((
            header.id,
            BridgeError::new(
                ErrorCode::UnknownCall,
                format!("chamada desconhecida: {}", header.name),
            ),
        ));
    }
    if matches!(
        header.name.as_str(),
        "library.scan"
            | "library.sources"
            | "system.displays"
            | "system.audio_outputs"
            | "i18n.catalog"
            | "downloads.list"
            | "downloads.summary"
            | "profile.summary"
            | "profile.collections"
            | "profiles.list"
            | "game.running"
            | "game.resume"
    ) && value.get("payload").is_some()
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
    #[serde(rename = "settings.get")]
    SettingsGet(
        #[serde(with = "object_payload")]
        #[ts(as = "SettingsGetRequest")]
        SettingsGetRequest,
    ),
    #[serde(rename = "settings.set")]
    SettingsSet(
        #[serde(with = "object_payload")]
        #[ts(as = "Setting")]
        Setting,
    ),
    #[serde(rename = "collections.list")]
    CollectionsList,
    #[serde(rename = "collections.create")]
    CollectionsCreate(
        #[serde(with = "object_payload")]
        #[ts(as = "CollectionCreate")]
        CollectionCreate,
    ),
    #[serde(rename = "collections.rename")]
    CollectionsRename(
        #[serde(with = "object_payload")]
        #[ts(as = "CollectionRename")]
        CollectionRename,
    ),
    #[serde(rename = "collections.delete")]
    CollectionsDelete(
        #[serde(with = "object_payload")]
        #[ts(as = "CollectionRequest")]
        CollectionRequest,
    ),
    #[serde(rename = "collections.add_games")]
    CollectionsAddGames(
        #[serde(with = "object_payload")]
        #[ts(as = "CollectionGames")]
        CollectionGames,
    ),
    #[serde(rename = "collections.remove_games")]
    CollectionsRemoveGames(
        #[serde(with = "object_payload")]
        #[ts(as = "CollectionGames")]
        CollectionGames,
    ),
    #[serde(rename = "collections.smart.set")]
    CollectionsSmartSet(
        #[serde(with = "object_payload")]
        #[ts(as = "SmartCollectionSet")]
        SmartCollectionSet,
    ),
    #[serde(rename = "tags.list")]
    TagsList,
    #[serde(rename = "game.tags.set")]
    GameTagsSet(
        #[serde(with = "object_payload")]
        #[ts(as = "GameTagsSet")]
        GameTagsSet,
    ),
    #[serde(rename = "game.status.set")]
    GameStatusSet(
        #[serde(with = "object_payload")]
        #[ts(as = "GameStatusSet")]
        GameStatusSet,
    ),
    #[serde(rename = "game.favorite.set")]
    GameFavoriteSet(
        #[serde(with = "object_payload")]
        #[ts(as = "GameFavoriteSet")]
        GameFavoriteSet,
    ),
    /// As telas ligadas ao computador (Configurações › Tela).
    #[serde(rename = "system.displays")]
    SystemDisplays,
    /// As saídas de áudio do computador (Configurações › Tela e áudio do jogo).
    #[serde(rename = "system.audio_outputs")]
    SystemAudioOutputs,
    /// Idioma atual e o catálogo inteiro dele; a interface só escreve texto que vem daqui (`t()`).
    #[serde(rename = "i18n.catalog")]
    I18nCatalog,
    /// Downloads com estado, progresso e jogo vinculado (DL-001). Desligados por padrão: `conflict` até o usuário
    /// ligar e aceitar o aviso legal (configurações `DownloadsEnabled` e `DownloadsLegalAccepted`).
    #[serde(rename = "downloads.list")]
    DownloadsList,
    #[serde(rename = "downloads.summary")]
    DownloadsSummary,
    #[serde(rename = "downloads.pause")]
    DownloadsPause(
        #[serde(with = "object_payload")]
        #[ts(as = "DownloadRequest")]
        DownloadRequest,
    ),
    /// Retoma um pausado ou com erro, ou "baixar agora" um item da fila.
    #[serde(rename = "downloads.resume")]
    DownloadsResume(
        #[serde(with = "object_payload")]
        #[ts(as = "DownloadRequest")]
        DownloadRequest,
    ),
    /// Cancela (ou tira da lista um concluído); `deleteFiles` apaga também o que já foi baixado.
    #[serde(rename = "downloads.cancel")]
    DownloadsCancel(
        #[serde(with = "object_payload")]
        #[ts(as = "DownloadCancel")]
        DownloadCancel,
    ),
    /// Fontes que o usuário adicionou e os itens delas (paginados, por busca ou sugeridos para um jogo).
    #[serde(rename = "downloads.sources")]
    DownloadsSources(
        #[serde(with = "object_payload")]
        #[ts(as = "DownloadSourcesRequest")]
        DownloadSourcesRequest,
    ),
    /// Começa um item de uma fonte do usuário, opcionalmente vinculado a um jogo da biblioteca.
    #[serde(rename = "downloads.add")]
    DownloadsAdd(
        #[serde(with = "object_payload")]
        #[ts(as = "DownloadAdd")]
        DownloadAdd,
    ),
    /// Configurações deste jogo (UX-007): o que o jogo mudou, o que vale de fato e o padrão de cada linha.
    #[serde(rename = "game.settings")]
    GameSettings(
        #[serde(with = "object_payload")]
        #[ts(as = "GameRequest")]
        GameRequest,
    ),
    /// Grava uma configuração do jogo (ou volta ao padrão) e devolve as configurações já atualizadas.
    #[serde(rename = "game.settings.set")]
    GameSettingsSet(
        #[serde(with = "object_payload")]
        #[ts(as = "GameSettingsSet")]
        GameSettingsSet,
    ),
    /// Resumo do perfil em uso: tempo de jogo, a semana, mais jogados, últimos adicionados e sessões recentes.
    #[serde(rename = "profile.summary")]
    ProfileSummary,
    /// Histórico de sessões do perfil em um período, agrupado por dia (as 300 mais recentes).
    #[serde(rename = "profile.sessions")]
    ProfileSessions(
        #[serde(with = "object_payload")]
        #[ts(as = "ProfileSessionsRequest")]
        ProfileSessionsRequest,
    ),
    /// Coleções do perfil (do usuário e automáticas) com contagens e as primeiras capas.
    #[serde(rename = "profile.collections")]
    ProfileCollections,
    /// Perfis locais (UX-004) e se perguntar quem está jogando ao abrir.
    #[serde(rename = "profiles.list")]
    ProfilesList,
    #[serde(rename = "profiles.create")]
    ProfilesCreate(
        #[serde(with = "object_payload")]
        #[ts(as = "ProfileCreate")]
        ProfileCreate,
    ),
    #[serde(rename = "profiles.rename")]
    ProfilesRename(
        #[serde(with = "object_payload")]
        #[ts(as = "ProfileRename")]
        ProfileRename,
    ),
    /// Apaga um perfil e os dados dele; o principal, o em uso e o que a próxima abertura vai usar não se apagam.
    #[serde(rename = "profiles.delete")]
    ProfilesDelete(
        #[serde(with = "object_payload")]
        #[ts(as = "ProfileRequest")]
        ProfileRequest,
    ),
    #[serde(rename = "profiles.ask.set")]
    ProfilesAskSet(
        #[serde(with = "object_payload")]
        #[ts(as = "ProfileAskSet")]
        ProfileAskSet,
    ),
    /// Grava o perfil que o Tako usa na próxima abertura (os dados do perfil em uso só trocam ao reabrir).
    #[serde(rename = "profiles.switch")]
    ProfilesSwitch(
        #[serde(with = "object_payload")]
        #[ts(as = "ProfileRequest")]
        ProfileRequest,
    ),
    /// O jogo que o Tako abriu e ainda acompanha, ou `null` (WEB-20).
    #[serde(rename = "game.running")]
    GameRunning,
    /// Traz o jogo para a frente: a interface fecha e reabre quando ele terminar (D-10). `conflict` sem jogo.
    #[serde(rename = "game.resume")]
    GameResume,
    /// Pede para o jogo fechar; `force` encerra os processos dele à força. `conflict` sem jogo ou quando o pedido não
    /// deu agora (a janela do jogo ainda não existe). O fim chega por `game.ended`.
    #[serde(rename = "game.end")]
    GameEnd(
        #[serde(with = "object_payload")]
        #[ts(as = "GameEndRequest")]
        GameEndRequest,
    ),
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
    #[serde(rename = "settings.get")]
    SettingsGet(SettingsValues),
    #[serde(rename = "settings.set")]
    SettingsSet,
    #[serde(rename = "collections.list")]
    CollectionsList(Vec<Collection>),
    #[serde(rename = "collections.create")]
    CollectionsCreate(Collection),
    #[serde(rename = "collections.rename")]
    CollectionsRename,
    #[serde(rename = "collections.delete")]
    CollectionsDelete,
    #[serde(rename = "collections.add_games")]
    CollectionsAddGames,
    #[serde(rename = "collections.remove_games")]
    CollectionsRemoveGames,
    #[serde(rename = "collections.smart.set")]
    CollectionsSmartSet(Collection),
    #[serde(rename = "tags.list")]
    TagsList(Vec<Tag>),
    #[serde(rename = "game.tags.set")]
    GameTagsSet,
    #[serde(rename = "game.status.set")]
    GameStatusSet,
    #[serde(rename = "game.favorite.set")]
    GameFavoriteSet,
    #[serde(rename = "system.displays")]
    SystemDisplays(DisplayList),
    #[serde(rename = "system.audio_outputs")]
    SystemAudioOutputs(AudioOutputList),
    #[serde(rename = "i18n.catalog")]
    I18nCatalog(I18nCatalog),
    #[serde(rename = "downloads.list")]
    DownloadsList(Vec<DownloadEntry>),
    #[serde(rename = "downloads.summary")]
    DownloadsSummary(DownloadsSummary),
    #[serde(rename = "downloads.pause")]
    DownloadsPause,
    #[serde(rename = "downloads.resume")]
    DownloadsResume,
    #[serde(rename = "downloads.cancel")]
    DownloadsCancel,
    #[serde(rename = "downloads.sources")]
    DownloadsSources(DownloadSources),
    #[serde(rename = "downloads.add")]
    DownloadsAdd(DownloadEntry),
    #[serde(rename = "game.settings")]
    GameSettings(Box<GameSettingsView>),
    #[serde(rename = "game.settings.set")]
    GameSettingsSet(Box<GameSettingsView>),
    #[serde(rename = "profile.summary")]
    ProfileSummary(Box<ProfileSummary>),
    #[serde(rename = "profile.sessions")]
    ProfileSessions(ProfileSessions),
    #[serde(rename = "profile.collections")]
    ProfileCollections(ProfileCollections),
    #[serde(rename = "profiles.list")]
    ProfilesList(ProfilesList),
    #[serde(rename = "profiles.create")]
    ProfilesCreate(ProfilesList),
    #[serde(rename = "profiles.rename")]
    ProfilesRename(ProfilesList),
    #[serde(rename = "profiles.delete")]
    ProfilesDelete(ProfilesList),
    #[serde(rename = "profiles.ask.set")]
    ProfilesAskSet(ProfilesList),
    #[serde(rename = "profiles.switch")]
    ProfilesSwitch(ProfilesList),
    #[serde(rename = "game.running")]
    GameRunning(Option<RunningGame>),
    #[serde(rename = "game.resume")]
    GameResume,
    #[serde(rename = "game.end")]
    GameEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum WindowAction {
    Minimize,
    /// Maximiza ou restaura, conforme o estado atual.
    ToggleMaximize,
    Close,
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
    /// O idioma mudou (`settings.set` com `Language`): a interface pede `i18n.catalog` de novo.
    #[serde(rename = "i18n.changed")]
    I18nChanged(I18nLang),
    /// Algo mudou nos downloads (progresso, estado, lista ou fontes lidas): a interface consulta de novo.
    #[serde(rename = "downloads.changed")]
    DownloadsChanged,
    /// O jogo pedido por `game.play` abriu. Com `tracked`, `game.running` passa a mostrá-lo e a interface fecha
    /// quando a janela dele aparecer.
    #[serde(rename = "game.started")]
    GameStarted(GameStarted),
    /// A sessão terminou (tela, áudio e tempo jogado já tratados) ou o jogo não abriu (`launch_failed`). Se a interface
    /// estava fechada, o aviso sai logo depois da resposta do próximo `game.running`.
    #[serde(rename = "game.ended")]
    GameEnded(GameEnded),
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
    // Id da coleção manual ou automática, resolvido no núcleo.
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

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct SettingsGetRequest {
    #[serde(default)]
    #[ts(optional)]
    pub keys: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct SettingsValues {
    pub values: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct CollectionRequest {
    pub collection_id: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct CollectionCreate {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct CollectionRename {
    pub collection_id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct CollectionGames {
    pub collection_id: String,
    pub game_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
/// Aplica ou remove cada tag de todos os jogos, preservando as outras tags (LIB-002).
pub struct GameTagsSet {
    pub game_ids: Vec<String>,
    pub tags: Vec<String>,
    pub on: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct GameStatusSet {
    pub game_ids: Vec<String>,
    pub status: Option<GameStatus>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct GameFavoriteSet {
    pub game_ids: Vec<String>,
    pub favorite: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct Tag {
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
/// `criteria: null` identifica uma coleção manual; automáticas resolvem os jogos na consulta.
pub struct Collection {
    pub id: String,
    pub name: String,
    pub game_ids: Vec<String>,
    pub criteria: Option<SmartCriteria>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
/// Sem id cria uma coleção; com id substitui nome e critérios da coleção automática.
pub struct SmartCollectionSet {
    pub collection_id: Option<String>,
    pub name: String,
    #[serde(with = "object_payload")]
    #[ts(as = "SmartCriteria")]
    pub criteria: SmartCriteria,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum PlayTime {
    #[default]
    Any,
    Unplayed,
    UnderHour,
    OneToTenHours,
    OverTenHours,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct SmartCriteria {
    #[serde(default)]
    #[serde(with = "object_payload")]
    #[ts(as = "Option<LibraryListRequest>", optional)]
    pub library: LibraryListRequest,
    #[serde(default)]
    #[ts(as = "Option<Vec<String>>", optional)]
    pub genres: Vec<String>,
    #[serde(default)]
    #[ts(as = "Option<PlayTime>", optional)]
    pub play_time: PlayTime,
    #[serde(default)]
    #[ts(as = "Option<bool>", optional)]
    pub played_only: bool,
    #[serde(default)]
    #[ts(as = "Option<bool>", optional)]
    pub broken_only: bool,
    #[serde(default)]
    #[ts(as = "Option<bool>", optional)]
    pub favorites_only: bool,
    #[serde(default)]
    #[ts(as = "Option<Vec<String>>", optional)]
    pub collections: Vec<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub filter_store: Option<Store>,
}

/// Estado de HDR ou VRR de uma tela, como o sistema informa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum DisplayFeature {
    /// O sistema não diz se a tela tem o recurso.
    Unknown,
    Unsupported,
    Off,
    On,
}

/// Uma tela ligada ao computador.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DisplayInfo {
    /// Id estável entre execuções; é o que as configurações guardam em `GameScreenId`.
    pub id: String,
    pub name: String,
    pub primary: bool,
    pub active: bool,
    pub hdr: DisplayFeature,
    pub vrr: DisplayFeature,
    /// Resoluções e taxas da tela, do maior para o menor, no formato guardado em `GameScreenMode` (`3840x2160@120`).
    pub modes: Vec<String>,
}

/// Resposta de `system.displays`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DisplayList {
    pub displays: Vec<DisplayInfo>,
    /// Sessão que cuida das telas sozinha (gamescope): o Tako só informa.
    pub managed_by_session: bool,
    /// Por que o Tako não consegue ligar o VRR neste sistema; `null` quando consegue.
    pub vrr_limitation: Option<String>,
}

/// Uma saída de áudio do computador.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AudioOutput {
    /// Id estável; é o que as configurações guardam em `GameAudioDeviceId`.
    pub id: String,
    pub name: String,
    /// A saída padrão do sistema agora.
    pub default: bool,
}

/// Resposta de `system.audio_outputs`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AudioOutputList {
    pub outputs: Vec<AudioOutput>,
    /// Falso quando o sistema não deixa o Tako trocar a saída de áudio.
    pub can_switch: bool,
}

/// Código do idioma (`pt-BR`, `en-US`), como o núcleo usa em `Language`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct I18nLang {
    pub lang: String,
}

/// Catálogo do idioma atual: `chave completa → texto`. Os parâmetros são `{nome}` e o plural vem em duas chaves,
/// `chave.one` e `chave.other`, escolhidas pelo `n` (`n == 1` é `one`), o mesmo formato de `tako_core::i18n`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct I18nCatalog {
    pub lang: String,
    pub messages: std::collections::BTreeMap<String, String>,
}

/// Estado de um download. O motor só mostra "baixando" depois de aceitar o item; `queued` espera a vez.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum DownloadStatus {
    Queued,
    Downloading,
    Paused,
    Completed,
    Failed,
}

/// Um download do usuário. Só dados para desenhar: a pasta, o endereço do item e o que veio da fonte ficam no núcleo.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DownloadEntry {
    #[ts(type = "number")]
    pub id: u64,
    /// Título do item na fonte, texto simples.
    pub title: String,
    /// Jogo da biblioteca vinculado pelo usuário; null = sem vínculo.
    pub game_id: Option<String>,
    /// Nome da fonte de onde veio (procedência).
    pub source: String,
    pub state: DownloadStatus,
    /// Só em `failed`: o motivo, já no idioma do usuário (texto do núcleo, não é um código).
    pub error: Option<String>,
    #[ts(type = "number")]
    pub done: u64,
    /// null até os metadados do torrent chegarem.
    #[ts(type = "number | null")]
    pub total: Option<u64>,
    /// 0 a 1; null enquanto o tamanho é desconhecido.
    pub progress: Option<f32>,
    /// Bytes por segundo; 0 fora de `downloading`.
    #[ts(type = "number")]
    pub speed: u64,
    /// null quando não há velocidade ou tamanho para estimar.
    #[ts(type = "number | null")]
    pub eta_seconds: Option<u64>,
}

/// Resumo para a pílula da barra e o topo do painel; só conta os downloads em aberto (não concluídos nem com erro).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DownloadsSummary {
    pub downloading: usize,
    pub queued: usize,
    #[ts(type = "number")]
    pub speed: u64,
    #[ts(type = "number")]
    pub done: u64,
    #[ts(type = "number")]
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct DownloadRequest {
    #[ts(type = "number")]
    pub download_id: u64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct DownloadCancel {
    #[ts(type = "number")]
    pub download_id: u64,
    #[serde(default)]
    #[ts(as = "Option<bool>", optional)]
    pub delete_files: bool,
}

/// Itens das fontes do usuário. `gameId` pede as sugestões para o título do jogo (mais parecidos primeiro) e não
/// combina com `search`; sem nenhum dos dois, lista tudo na ordem das fontes.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
#[ts(export)]
pub struct DownloadSourcesRequest {
    /// Restringe os itens a uma fonte (`DownloadSource.id`).
    #[ts(optional = nullable)]
    pub source_id: Option<String>,
    #[ts(as = "Option<String>", optional)]
    pub search: String,
    #[ts(optional = nullable)]
    pub game_id: Option<String>,
    #[ts(as = "Option<u32>", optional)]
    pub offset: u32,
    /// Acima de MAX_PAGE_LIMIT é reduzido ao teto; zero devolve só as fontes e o total.
    #[ts(as = "Option<u32>", optional)]
    pub limit: u32,
}

impl Default for DownloadSourcesRequest {
    fn default() -> Self {
        Self {
            source_id: None,
            search: String::new(),
            game_id: None,
            offset: 0,
            limit: DEFAULT_PAGE_LIMIT,
        }
    }
}

/// Fontes lidas do disco em segundo plano: com `loading` verdadeiro a lista ainda está incompleta e o evento
/// `downloads.changed` avisa quando termina.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DownloadSources {
    pub loading: bool,
    pub sources: Vec<DownloadSource>,
    pub items: Vec<DownloadSourceItem>,
    /// Itens que combinam com o pedido, antes de `offset` e `limit`.
    pub total: usize,
}

/// Uma fonte `.json` que o próprio usuário adicionou. O endereço não sai do núcleo: só o domínio, para o usuário
/// reconhecer a fonte.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DownloadSource {
    pub id: String,
    pub name: String,
    pub host: String,
    pub item_count: usize,
    /// Itens da fonte descartados por estarem quebrados ou além do limite.
    pub dropped: usize,
    /// ISO-8601 em UTC.
    pub updated_at: String,
}

/// Texto da fonte, mostrado só como texto simples.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DownloadSourceItem {
    pub source_id: String,
    /// Posição na fonte; vale junto com `title` em `downloads.add`.
    pub index: usize,
    pub title: String,
    pub file_size: String,
    pub upload_date: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct DownloadAdd {
    pub source_id: String,
    pub index: usize,
    /// Título do item como a interface o mostrou; se a fonte mudou desde então, a chamada dá `conflict`.
    pub title: String,
    /// Jogo da biblioteca a vincular; um jogo tem no máximo um download em aberto.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub game_id: Option<String>,
}

/// Um campo do jogo que tem "padrão do Tako", "manter como está" e um valor próprio (resolução, saída de áudio).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum GameChoice {
    /// O jogo não escolhe: vale o padrão do Tako.
    Default,
    /// O jogo pede para não mexer (manter a resolução atual, não trocar a saída de áudio).
    Keep,
    /// O jogo escolhe: a chave da resolução (`1920x1080@60`) ou o id da saída de áudio.
    Pick { id: String },
}

/// Uma configuração de liga/desliga: o que o jogo guardou (`null` = segue o padrão), o padrão e o que vale.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameFlagSetting {
    pub saved: Option<bool>,
    pub default: bool,
    pub effective: bool,
}

/// VRR: como [`GameFlagSetting`], mais o aviso de que a tela não deixa ligar.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameVrrSetting {
    pub saved: Option<bool>,
    pub default: bool,
    pub effective: bool,
    /// A tela do jogo informa que não tem VRR; a linha fica à vista, mas não grava.
    pub unsupported: bool,
    /// Por que o Tako não consegue ligar o VRR neste sistema (o mesmo de `system.displays`).
    pub limitation: Option<String>,
}

/// Um texto livre do jogo (argumentos, pasta, variáveis, comandos). Não existe padrão do Tako: o padrão é "nenhum".
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameTextSetting {
    /// Como o jogo guardou, sem espaços nas pontas; `null` = vazio.
    pub saved: Option<String>,
    /// O que a sessão usa (variáveis já lidas e sem as inválidas); `null` = nada.
    pub effective: Option<String>,
}

/// Uma tela ou saída de áudio pelo id estável e o nome para mostrar.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameDevice {
    pub id: String,
    pub name: String,
}

/// Uma tela ou saída que o jogo pode escolher; a escolhida e desconectada agora continua na lista.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameDeviceOption {
    pub id: String,
    pub name: String,
    pub connected: bool,
}

/// "Tela deste jogo" (PERF-002): outra tela só para o jogo, no lugar da tela de jogo do Tako.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameScreenSetting {
    /// Id da tela do jogo; `null` = usa a tela de jogo do Tako.
    pub saved: Option<String>,
    /// A tela de jogo do Tako (Configurações › Tela); `null` = nenhuma escolhida.
    pub default: Option<GameDevice>,
    /// Id da tela que o jogo vai usar; `null` = o jogo não usa tela própria.
    pub effective: Option<String>,
    pub options: Vec<GameDeviceOption>,
    /// O jogo pediu a tela de jogo e não há nenhuma escolhida.
    pub no_screen_chosen: bool,
}

/// Resolução e taxa na tela do jogo.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameModeSetting {
    pub saved: GameChoice,
    /// A resolução padrão nesta tela (a do Tako, ou o preset do monitor); `null` = manter a atual.
    pub default: Option<String>,
    /// O que vale: `null` = manter a atual.
    pub effective: Option<String>,
    /// As resoluções da tela do jogo, no formato `1920x1080@60`, e a guardada que a tela não tem mais.
    pub options: Vec<String>,
}

/// Saída de áudio do jogo.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameAudioSetting {
    pub saved: GameChoice,
    /// A saída do Tako (Configurações › Tela e áudio do jogo); `null` = não alterar.
    pub default: Option<GameDevice>,
    /// A saída que vale; `null` = não alterar.
    pub effective: Option<GameDevice>,
    pub options: Vec<GameDeviceOption>,
    /// Falso quando o sistema não deixa o Tako trocar a saída de áudio.
    pub can_switch: bool,
}

/// Resposta de `game.settings` e `game.settings.set`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameSettingsView {
    pub game_id: String,
    /// Quantas coisas o jogo muda em relação ao Tako.
    pub changes: usize,
    pub use_screen: GameFlagSetting,
    pub screen: GameScreenSetting,
    pub mode: GameModeSetting,
    pub hdr: GameFlagSetting,
    pub vrr: GameVrrSetting,
    pub audio: GameAudioSetting,
    pub arguments: GameTextSetting,
    pub working_directory: GameTextSetting,
    pub environment: GameTextSetting,
    /// Só leitura na interface: gravar comandos depende de o Tako pedir a permissão ao abrir o jogo (SESSION-004).
    pub before_command: GameTextSetting,
    pub after_command: GameTextSetting,
    /// Os comandos, os argumentos e as variáveis do jogo já foram permitidos como estão (ou não há o que permitir).
    pub commands_approved: bool,
}

/// Uma configuração do jogo; `null` (ou `{ kind: "default" }`) volta ao padrão do Tako.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(tag = "field", content = "value", rename_all = "snake_case")]
#[ts(export)]
pub enum GameSettingChange {
    /// Usar a tela de jogo do Tako neste jogo.
    UseScreen(Option<bool>),
    /// Id da tela só deste jogo. Trocar de tela esquece a resolução escolhida para a anterior.
    Screen(Option<String>),
    Mode(GameChoice),
    Hdr(Option<bool>),
    Vrr(Option<bool>),
    Audio(GameChoice),
    /// Argumentos extras, em uma linha (até 2048 caracteres).
    Arguments(Option<String>),
    /// Pasta de trabalho; precisa existir (até 1024 caracteres).
    WorkingDirectory(Option<String>),
    /// `NOME=valor` separados por `;` ou por linha (até 32 variáveis, 4096 caracteres).
    Environment(Option<String>),
    /// "Usar tudo do Tako": apaga todas as escolhas do jogo.
    ResetAll,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct GameSettingsSet {
    pub game_id: String,
    pub change: GameSettingChange,
}

/// Um perfil local (UX-004). `name` vem vazio no principal enquanto ninguém lhe deu um nome: a interface mostra o
/// texto `profiles.main`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProfileEntry {
    pub id: String,
    pub name: String,
    /// O perfil que já existia antes dos perfis: não se apaga.
    pub main: bool,
    /// O perfil cujos dados esta abertura do Tako está usando.
    pub in_use: bool,
    /// O perfil que a próxima abertura vai usar (difere de `inUse` depois de `profiles.switch`).
    pub next: bool,
}

/// Resposta de `profiles.list` e de todas as chamadas `profiles.*` que mudam a lista.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProfilesList {
    pub profiles: Vec<ProfileEntry>,
    /// Perguntar "Quem está jogando?" ao abrir o Tako.
    pub ask_on_start: bool,
    #[ts(type = "number")]
    pub max_profiles: usize,
    #[ts(type = "number")]
    pub max_name_chars: usize,
    /// O perfil gravado para a próxima abertura não é o que está em uso: a troca só vale ao reabrir o Tako.
    pub restart_required: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct ProfileCreate {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct ProfileRename {
    pub profile_id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct ProfileRequest {
    pub profile_id: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct ProfileAskSet {
    pub on: bool,
}

/// Um dia da semana de `profile.summary`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProfileWeekDay {
    /// Data local, `AAAA-MM-DD`.
    pub day: String,
    /// Tempo jogado no dia (sessões de menos de 1 minuto não contam).
    #[ts(type = "number")]
    pub seconds: u64,
}

/// Uma sessão de jogo do perfil.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProfileSession {
    pub game: LibraryGame,
    /// ISO-8601 em UTC.
    pub started_at: String,
    #[ts(type = "number")]
    pub duration_seconds: u64,
    /// Menos de 1 minuto: "abriu e fechou", fora dos totais.
    pub quick: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProfileSummary {
    /// O perfil cujos dados estão em uso.
    pub profile: ProfileEntry,
    /// Última sessão ou abertura de qualquer jogo; null = nada jogado ainda.
    pub last_played_at: Option<String>,
    /// Soma do tempo de jogo de todos os jogos.
    #[ts(type = "number")]
    pub total_play_seconds: u64,
    #[ts(type = "number")]
    pub game_count: usize,
    /// Data local de hoje, `AAAA-MM-DD`.
    pub today: String,
    /// A semana atual, de segunda a domingo (sempre sete dias).
    pub week: Vec<ProfileWeekDay>,
    /// Até três jogos com mais tempo de jogo, o primeiro com mais.
    pub top_games: Vec<LibraryGame>,
    /// Até três jogos adicionados por último, o mais recente primeiro.
    pub latest_games: Vec<LibraryGame>,
    /// As cinco sessões mais recentes, a mais recente primeiro.
    pub recent_sessions: Vec<ProfileSession>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SessionPeriod {
    /// Desde a segunda-feira desta semana.
    Week,
    /// Desde o dia 1 deste mês.
    Month,
    All,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct ProfileSessionsRequest {
    pub period: SessionPeriod,
}

/// Sessões de um dia local, da mais recente para a mais antiga.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProfileDay {
    /// Data local, `AAAA-MM-DD`.
    pub day: String,
    /// Tempo do dia, sem as sessões rápidas.
    #[ts(type = "number")]
    pub total_seconds: u64,
    pub sessions: Vec<ProfileSession>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProfileSessions {
    /// Data local de hoje, `AAAA-MM-DD`.
    pub today: String,
    /// Sessões do período, sem o recorte das mostradas (as rápidas contam como sessão, mas não no tempo).
    #[ts(type = "number")]
    pub count: usize,
    #[ts(type = "number")]
    pub total_seconds: u64,
    /// Até 300 sessões, as mais recentes, em grupos por dia.
    pub days: Vec<ProfileDay>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProfileCollection {
    /// O mesmo id que `library.list` aceita em `collection`.
    pub id: String,
    pub name: String,
    pub automatic: bool,
    #[ts(type = "number")]
    pub game_count: usize,
    #[ts(type = "number")]
    pub installed_count: usize,
    /// Até quatro jogos, para o mosaico.
    pub covers: Vec<LibraryGame>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProfileCollections {
    pub collections: Vec<ProfileCollection>,
    /// Jogos distintos nas coleções do usuário (as automáticas não entram na conta).
    #[ts(type = "number")]
    pub game_count: usize,
}

/// Em que ponto está o jogo aberto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum RunningState {
    /// Aberto, esperando a janela dele aparecer ("Abrindo…").
    Launching,
    Playing,
    /// `game.end` pedido; esperando os processos sumirem ("Fechando o jogo…").
    Closing,
}

/// Resposta de `game.running`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RunningGame {
    pub game_id: String,
    pub title: String,
    /// ISO-8601 em UTC.
    pub started_at: String,
    pub state: RunningState,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct GameEndRequest {
    #[serde(default)]
    #[ts(as = "Option<bool>", optional)]
    pub force: bool,
}

/// Evento `game.started`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameStarted {
    pub game_id: String,
    pub title: String,
    /// ISO-8601 em UTC.
    pub started_at: String,
    /// Falso quando o Tako só abriu (URI sem processo nem pasta): não há como saber quando termina, nem sessão.
    pub tracked: bool,
    /// Avisos da abertura (ferramenta que não abriu, comando antes do jogo que falhou), já no idioma do usuário.
    pub notices: Vec<String>,
}

/// Como a sessão terminou (SESSION-003).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum GameEndReason {
    Normal,
    /// Fechou sozinho em menos de 10 s: caiu ou desistiu ao abrir.
    Quick,
    /// O jogo não apareceu dentro do prazo.
    NeverStarted,
    /// Fechou sozinho com um código de erro.
    Failed,
    /// Não chegou a abrir; `message` diz por quê.
    LaunchFailed,
}

/// Evento `game.ended`: para o "bem-vindo de volta" e o aviso de erro do jogo.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameEnded {
    pub game_id: String,
    pub title: String,
    #[ts(type = "number")]
    pub played_seconds: u64,
    pub end: GameEndReason,
    /// Código de saída, quando o Tako abriu o executável direto e o jogo terminou com erro.
    #[ts(type = "number | null")]
    pub exit_code: Option<u32>,
    /// O código explicado (hexadecimal, decimal e o que significa), já no idioma do usuário.
    pub exit_code_text: Option<String>,
    /// Só em `launch_failed`: o motivo, já no idioma do usuário.
    pub message: Option<String>,
}
