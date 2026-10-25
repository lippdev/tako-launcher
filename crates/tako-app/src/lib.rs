//! Serviços do Tako sem interface (PRD D-09, WEB-01): o que a interface web pede e o que o núcleo avisa.
//!
//! A interface só desenha; dados e regras ficam aqui e nos crates do núcleo. Nada neste crate conhece CEF,
//! janela ou React: o hospedeiro (`tako-web`) recebe o JSON, chama `AppService::handle` e devolve a resposta.

mod art;
mod control_center;
mod core;
mod demo;
mod downloads;
mod edit;
mod emulators;
mod game_settings;
mod i18n;
pub mod input;
mod library;
mod links;
mod local_api;
mod metadata;
mod organize;
mod picker;
mod profile;
pub mod protocol;
mod session;
mod settings;
mod sources;
mod system;
mod updates;

pub use core::{Core, CoreError};
pub use links::LinkOutcome;
pub use session::SessionUpdate;
pub use updates::{UpdateFeed, UpdateInstaller, UpdateSetup};

use protocol::{
    AppInfo, BRIDGE_VERSION, BridgeError, Call, ErrorCode, Event, I18nLang, Reply, UiState,
    WindowAction,
};
use std::sync::mpsc::Sender;

/// Maior estado de tela guardado (`ui.state.set`); é só onde a interface estava, não dados.
const MAX_UI_STATE_BYTES: usize = 64 * 1024;

/// O que só o hospedeiro sabe fazer (janela, sessão de jogo). A chamada responde na hora que o pedido foi aceito;
/// o resultado chega depois por evento.
#[derive(Debug, Clone, PartialEq)]
pub enum HostRequest {
    Window(WindowAction),
    Launch {
        game_id: String,
    },
    /// Há resultados de trabalho em segundo plano. O hospedeiro deve chamar AppService::poll na thread do
    /// núcleo, mesmo sem interface aberta; não encaminha este aviso à janela.
    PollBackground,
    /// `game.resume`: trazer o jogo para a frente. Na versão web a interface fecha (D-10) e reabre no fim do jogo.
    Resume,
    /// A atualização foi aplicada (o instalador ou o script de troca já foi iniciado e espera este processo): o
    /// hospedeiro fecha a interface e encerra o núcleo (WEB-33).
    Exit,
    /// Abre o seletor de arquivos ou de pastas do sistema operacional numa thread separada (WEB-12e).
    Pick {
        id: u32,
        kind: protocol::PickKind,
        purpose: String,
        filters: Option<Vec<protocol::PickFilter>>,
    },
}

/// Atende as chamadas da interface. Uma instância por processo, na thread do núcleo.
pub struct AppService {
    version: String,
    events: Option<Sender<Event>>,
    host: Option<Sender<HostRequest>>,
    ui_state: Option<String>,
    core: Option<Core>,
    /// Controle em uso (WEB-05b), atualizado pelo hospedeiro que lê o SDL3.
    controller: protocol::ControllerInfo,
    /// Link `tako://` de um jogo que a leitura das lojas ainda não trouxe (WEB-34).
    pending_link: Option<tako_core::links::Link>,
    /// API local de integração (WEB-34): só existe com a chave ligada.
    local_api: local_api::WebApi,
    /// A última entrada vista foi do controle (WEB-05b).
    using_pad: bool,
    central_open: bool,
    picker: picker::PickerState,
}

impl AppService {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
            events: None,
            host: None,
            ui_state: None,
            core: None,
            controller: protocol::ControllerInfo::default(),
            pending_link: None,
            local_api: local_api::WebApi::default(),
            using_pad: false,
            central_open: false,
            picker: picker::PickerState::default(),
        }
    }

    /// Serviço com o núcleo já aberto, independente da vida da janela.
    pub fn with_core(version: impl Into<String>, core: Core) -> Self {
        Self {
            core: Some(core),
            ..Self::new(version)
        }
    }

    /// Recebe uma leitura concluída das fontes e avisa a interface para consultar de novo.
    pub fn reload_library(
        &mut self,
        games: Vec<tako_core::library::Game>,
    ) -> Result<(), BridgeError> {
        self.core
            .as_mut()
            .ok_or_else(|| BridgeError::new(ErrorCode::Internal, "núcleo não aberto"))?
            .reload_library(games);
        self.emit(Event::LibraryChanged);
        Ok(())
    }

    /// Para onde vão os eventos do núcleo; o hospedeiro repassa à interface aberta (ou descarta, com ela fechada).
    pub fn set_event_sink(&mut self, sink: Sender<Event>) {
        self.events = Some(sink);
    }

    /// Para onde vão os pedidos ao hospedeiro (`HostRequest`).
    pub fn set_host_sink(&mut self, sink: Sender<HostRequest>) {
        if let Some(core) = &mut self.core {
            core.sources.set_host_sink(sink.clone());
            core.downloads.set_host_sink(sink.clone());
            core.jobs.set_host_sink(sink.clone());
            core.art
                .set_host_sink(sink.clone(), !core.library.games().is_empty());
            core.updates.set_host_sink(sink.clone());
        }
        self.host = Some(sink);
    }

    pub fn handle(&mut self, call: Call) -> Result<Reply, BridgeError> {
        self.poll();
        match call {
            call @ (Call::ControlCenterState | Call::ControlCenterSet(_)) => {
                self.handle_control_center(call)
            }
            Call::AppInfo => Ok(Reply::AppInfo(AppInfo {
                version: self.version.clone(),
                bridge_version: BRIDGE_VERSION,
            })),
            Call::SystemPick(request) => {
                if self.picker.is_busy() {
                    return Err(BridgeError::new(
                        ErrorCode::Conflict,
                        "já há um seletor aberto",
                    ));
                }
                let id = self.picker.next_id();
                self.request(HostRequest::Pick {
                    id,
                    kind: request.kind,
                    purpose: request.purpose,
                    filters: request.filters,
                })?;
                self.picker.register(id);
                Ok(Reply::SystemPick(protocol::PickResponse { id }))
            }
            Call::WindowAction(action) => {
                self.request(HostRequest::Window(action))?;
                Ok(Reply::WindowAction)
            }
            Call::UiState => Ok(Reply::UiState(UiState {
                json: self.ui_state.clone(),
            })),
            Call::UiStateSet(state) => {
                if state
                    .json
                    .as_ref()
                    .is_some_and(|s| s.len() > MAX_UI_STATE_BYTES)
                {
                    return Err(BridgeError::new(
                        ErrorCode::BadRequest,
                        "estado de tela grande demais",
                    ));
                }
                self.ui_state = state.json;
                Ok(Reply::UiStateSet)
            }
            Call::LibraryList(request) => {
                request.validate()?;
                Ok(Reply::LibraryList(library::list(
                    self.require_core()?,
                    request,
                )))
            }
            Call::GameDetails(request) => {
                let core = self.require_core()?;
                let game = self.find_game(&request.game_id)?;
                Ok(Reply::GameDetails(Box::new(library::details(core, game))))
            }
            Call::GamePlay(request) => self.play(request),
            Call::GameRunning => self.running(),
            Call::GameResume => self.resume(),
            Call::GameEnd(request) => self.end_game(request),
            Call::InputController => Ok(Reply::InputController(self.controller.clone())),
            Call::InputSource(request) => {
                self.set_using_pad(request.pad);
                Ok(Reply::InputSource)
            }
            Call::LibraryScan => {
                let core = self
                    .core
                    .as_mut()
                    .ok_or_else(|| BridgeError::new(ErrorCode::Internal, "núcleo não aberto"))?;
                let disabled = core
                    .settings
                    .get("DisabledStores")
                    .cloned()
                    .unwrap_or_default();
                if core
                    .sources
                    .start(&disabled)
                    .map_err(|e| BridgeError::new(ErrorCode::Internal, e.to_string()))?
                {
                    self.emit_scan_progress();
                }
                Ok(Reply::LibraryScan)
            }
            Call::LibrarySources => Ok(Reply::LibrarySources(
                self.require_core()?.sources.state.clone(),
            )),
            Call::SystemDisplays => Ok(Reply::SystemDisplays(system::displays(
                &self.require_core()?.devices,
            )?)),
            Call::SystemAudioOutputs => Ok(Reply::SystemAudioOutputs(system::audio_outputs(
                &self.require_core()?.devices,
            )?)),
            Call::I18nCatalog => Ok(Reply::I18nCatalog(i18n::catalog(self.require_core()?))),
            Call::GameSettings(request) => Ok(Reply::GameSettings(Box::new(game_settings::view(
                self.require_core()?,
                &request.game_id,
            )?))),
            Call::GameSettingsSet(request) => {
                let core = self
                    .core
                    .as_mut()
                    .ok_or_else(|| BridgeError::new(ErrorCode::Internal, "núcleo não aberto"))?;
                Ok(Reply::GameSettingsSet(Box::new(game_settings::set(
                    core, request,
                )?)))
            }
            call @ (Call::DownloadsList
            | Call::DownloadsSummary
            | Call::DownloadsPause(_)
            | Call::DownloadsResume(_)
            | Call::DownloadsCancel(_)
            | Call::DownloadsSources(_)
            | Call::DownloadsAdd(_)) => self.handle_downloads(call),
            call @ (Call::ProfileSummary
            | Call::ProfileSessions(_)
            | Call::ProfileCollections
            | Call::ProfilesList
            | Call::ProfilesCreate(_)
            | Call::ProfilesRename(_)
            | Call::ProfilesDelete(_)
            | Call::ProfilesAskSet(_)
            | Call::ProfilesSwitch(_)) => {
                let core = self
                    .core
                    .as_mut()
                    .ok_or_else(|| BridgeError::new(ErrorCode::Internal, "núcleo não aberto"))?;
                profile::handle(core, call)
            }
            call @ (Call::UpdatesStatus
            | Call::UpdatesCheck
            | Call::UpdatesDownload
            | Call::UpdatesApply
            | Call::UpdatesPreferences
            | Call::UpdatesPreferencesSet(_)) => self.handle_updates(call),
            call @ (Call::GameEdit(_)
            | Call::GameEditSet(_)
            | Call::GameRemove(_)
            | Call::LibraryAddGame(_)
            | Call::GameSourceIds(_)
            | Call::GameSourceIdsSet(_)
            | Call::GameSourceIdsSearch(_)
            | Call::GameSourceIdsCheck(_)) => self.handle_edit(call),
            call @ (Call::EmulatorsList
            | Call::EmulatorsFolderAdd(_)
            | Call::EmulatorsFolderRemove(_)
            | Call::EmulatorsFolderSystemSet(_)) => {
                let core = self
                    .core
                    .as_mut()
                    .ok_or_else(|| BridgeError::new(ErrorCode::Internal, "núcleo não aberto"))?;
                let (reply, changed) = emulators::handle(core, call)?;
                // A configuração já está gravada; se a leitura não puder começar, o botão "Ler de novo" cobre.
                if changed && emulators::rescan(core)? {
                    self.emit_scan_progress();
                }
                Ok(reply)
            }
            Call::ArtCandidates(request) => {
                let core = self.require_core()?;
                Ok(Reply::ArtCandidates(core.art_candidates(&request)?))
            }
            Call::ArtChoose(request) => {
                let core = self
                    .core
                    .as_mut()
                    .ok_or_else(|| BridgeError::new(ErrorCode::Internal, "núcleo não aberto"))?;
                let game_id = request.game_id.clone();
                let has_jobs = core.art.jobs.is_some();
                core.art_choose(&request)?;
                if !has_jobs {
                    self.emit(Event::ArtChanged(protocol::GameRequest { game_id }));
                }
                self.emit(Event::LibraryChanged);
                Ok(Reply::ArtChoose)
            }
            Call::ArtReset(request) => {
                let core = self
                    .core
                    .as_mut()
                    .ok_or_else(|| BridgeError::new(ErrorCode::Internal, "núcleo não aberto"))?;
                let game_id = request.game_id.clone();
                let has_jobs = core.art.jobs.is_some();
                core.art_reset(&request)?;
                if !has_jobs {
                    self.emit(Event::ArtChanged(protocol::GameRequest { game_id }));
                }
                self.emit(Event::LibraryChanged);
                Ok(Reply::ArtReset)
            }
            Call::ArtReview => {
                let core = self.require_core()?;
                Ok(Reply::ArtReview(core.art_review()?))
            }
            Call::ArtReviewAccept(request) => {
                let core = self
                    .core
                    .as_mut()
                    .ok_or_else(|| BridgeError::new(ErrorCode::Internal, "núcleo não aberto"))?;
                core.art_review_accept(&request)?;
                Ok(Reply::ArtReviewAccept)
            }
            call => {
                let core = self
                    .core
                    .as_mut()
                    .ok_or_else(|| BridgeError::new(ErrorCode::Internal, "núcleo não aberto"))?;
                let before = (core.library.version(), core.collections.version());
                let lang_before = core.lang;
                let result = organize::handle(core, call);
                // O núcleo pode mudar a memória antes de uma falha de disco; a tela precisa consultar de novo.
                core.collections.apply_to(core.library.games_mut());
                let changed = result.as_ref().is_ok_and(|(_, changed)| *changed)
                    || before != (core.library.version(), core.collections.version());
                let lang = core.lang;
                if changed {
                    self.emit(Event::LibraryChanged);
                }
                if lang != lang_before {
                    self.emit(Event::I18nChanged(I18nLang {
                        lang: lang.code().to_owned(),
                    }));
                }
                result.map(|(reply, _)| reply)
            }
        }
    }

    fn require_core(&self) -> Result<&Core, BridgeError> {
        self.core
            .as_ref()
            .ok_or_else(|| BridgeError::new(ErrorCode::Internal, "núcleo não aberto"))
    }

    fn find_game(&self, id: &str) -> Result<&tako_core::library::Game, BridgeError> {
        if id.trim().is_empty() {
            return Err(BridgeError::new(ErrorCode::BadRequest, "id de jogo vazio"));
        }
        self.require_core()?
            .library()
            .find(id)
            .ok_or_else(|| BridgeError::new(ErrorCode::NotFound, format!("jogo {id}")))
    }

    fn request(&self, req: HostRequest) -> Result<(), BridgeError> {
        let sent = self.host.as_ref().is_some_and(|h| h.send(req).is_ok());
        if sent {
            Ok(())
        } else {
            Err(BridgeError::new(ErrorCode::Internal, "sem hospedeiro"))
        }
    }

    /// Avisa a interface. Sem interface aberta o evento se perde de propósito: ao reabrir, ela consulta tudo (D-10).
    pub fn emit(&self, event: Event) {
        if let Some(sink) = &self.events
            && sink.send(event).is_err()
        {
            tracing::debug!("evento sem interface aberta");
        }
    }

    /// Entrega o caminho escolhido no diálogo nativo ou `None` se o usuário cancelou (WEB-12e).
    /// Só emite o evento `system.picked` se o id corresponder a um pedido aberto.
    pub fn deliver_picked(&mut self, id: u32, path: Option<String>) -> bool {
        if self.picker.complete(id) {
            self.emit(Event::SystemPicked(protocol::SystemPicked { id, path }));
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::{Request, Response};

    #[test]
    fn app_info_reports_versions() {
        let mut app = AppService::new("1.2.3");
        let reply = app.handle(Call::AppInfo).unwrap();
        assert_eq!(
            reply,
            Reply::AppInfo(AppInfo {
                version: "1.2.3".into(),
                bridge_version: BRIDGE_VERSION
            })
        );
    }

    #[test]
    fn request_without_payload_parses() {
        let req: Request = serde_json::from_str(r#"{"v":1,"id":7,"name":"app.info"}"#).unwrap();
        assert_eq!(
            req,
            Request {
                v: 1,
                id: 7,
                call: Call::AppInfo
            }
        );
    }

    #[test]
    fn response_wire_format() {
        let ok = Response::ok(
            7,
            Reply::AppInfo(AppInfo {
                version: "1".into(),
                bridge_version: 1,
            }),
        );
        assert_eq!(
            serde_json::to_string(&ok).unwrap(),
            r#"{"v":1,"id":7,"ok":true,"reply":{"name":"app.info","data":{"version":"1","bridgeVersion":1}}}"#
        );
        let err = Response::err(8, BridgeError::new(protocol::ErrorCode::NotFound, "jogo x"));
        assert_eq!(
            serde_json::to_string(&err).unwrap(),
            r#"{"v":1,"id":8,"ok":false,"error":{"code":"not_found","message":"jogo x"}}"#
        );
    }

    #[test]
    fn window_actions_go_to_the_host() {
        let mut app = AppService::new("1");
        let no_host = app.handle(Call::WindowAction(WindowAction::Close));
        assert_eq!(no_host.unwrap_err().code, ErrorCode::Internal);
        let (tx, rx) = std::sync::mpsc::channel();
        app.set_host_sink(tx);
        let req: Request = serde_json::from_str(
            r#"{"v":1,"id":1,"name":"window.action","payload":"toggle_maximize"}"#,
        )
        .unwrap();
        assert_eq!(app.handle(req.call).unwrap(), Reply::WindowAction);
        assert_eq!(
            rx.try_recv().unwrap(),
            HostRequest::Window(WindowAction::ToggleMaximize)
        );
    }

    #[test]
    fn ui_state_survives_until_replaced_and_has_a_limit() {
        let mut app = AppService::new("1");
        let empty = Reply::UiState(UiState { json: None });
        assert_eq!(app.handle(Call::UiState).unwrap(), empty);
        let state = UiState {
            json: Some(r#"{"screen":"library"}"#.into()),
        };
        assert_eq!(
            app.handle(Call::UiStateSet(state.clone())).unwrap(),
            Reply::UiStateSet
        );
        assert_eq!(app.handle(Call::UiState).unwrap(), Reply::UiState(state));
        let big = UiState {
            json: Some("x".repeat(MAX_UI_STATE_BYTES + 1)),
        };
        assert_eq!(
            app.handle(Call::UiStateSet(big)).unwrap_err().code,
            ErrorCode::BadRequest
        );
    }

    #[test]
    fn events_reach_the_sink_and_are_dropped_without_one() {
        let mut app = AppService::new("1");
        app.emit(Event::LibraryChanged);
        let (tx, rx) = std::sync::mpsc::channel();
        app.set_event_sink(tx);
        app.emit(Event::LibraryChanged);
        assert_eq!(rx.try_recv().unwrap(), Event::LibraryChanged);
        drop(rx);
        app.emit(Event::LibraryChanged);
    }
}

impl AppService {
    /// Idioma ativo no núcleo (WEB-36).
    pub fn language(&self) -> tako_core::i18n::Lang {
        self.core
            .as_ref()
            .map(|c| c.lang)
            .unwrap_or(tako_core::i18n::Lang::PtBr)
    }

    /// Jogos jogados mais recentemente (WEB-36), até o limite pedido.
    pub fn recent_games(&self, count: usize) -> Vec<(String, String)> {
        let Some(core) = self.core.as_ref() else {
            return Vec::new();
        };
        tako_core::library::query::recently_played(core.library().games())
            .into_iter()
            .filter(|game| game.can_play())
            .take(count)
            .map(|g| (g.id.clone(), g.title.clone()))
            .collect()
    }

    /// Se a opção "fechar vai para a bandeja" está ligada (WEB-36, PRD D-10).
    pub fn close_to_tray(&self) -> bool {
        // Atalho de teste só em debug: em release vale apenas a configuração.
        if cfg!(debug_assertions)
            && std::env::var("TAKO_CLOSE_TO_TRAY")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false)
        {
            return true;
        }
        self.core
            .as_ref()
            .and_then(|c| c.settings.get("CloseToTray"))
            .is_some_and(|v| v == "true")
    }
}

#[cfg(test)]
mod tray_tests {
    use super::*;

    #[test]
    fn app_tray_helpers_without_core() {
        let app = AppService::new("1.0");
        assert_eq!(app.language(), tako_core::i18n::Lang::PtBr);
        assert!(app.recent_games(5).is_empty());
        assert!(!app.close_to_tray());
    }

    #[test]
    fn tray_recent_games_filter_unplayable_before_limit() {
        let dir = tempfile::tempdir().unwrap();
        let core = Core::open(
            tako_core::paths::DataPaths::at(dir.path()),
            tako_platform::Platform::simulated(),
            true,
        )
        .unwrap();
        let mut app = AppService::with_core("test", core);
        let now = chrono::Utc::now();
        app.reload_library(vec![
            tako_core::library::Game {
                id: "removed".into(),
                last_played_at: Some(now),
                ..Default::default()
            },
            tako_core::library::Game {
                id: "upcoming".into(),
                is_installed: true,
                upcoming_release: chrono::NaiveDate::from_ymd_opt(2030, 1, 1),
                last_played_at: Some(now),
                ..Default::default()
            },
            tako_core::library::Game {
                id: "installed".into(),
                is_installed: true,
                last_played_at: Some(now - chrono::TimeDelta::days(1)),
                ..Default::default()
            },
        ])
        .unwrap();
        assert_eq!(
            app.recent_games(1)
                .iter()
                .map(|g| g.0.as_str())
                .collect::<Vec<_>>(),
            vec!["installed"]
        );
    }
}
