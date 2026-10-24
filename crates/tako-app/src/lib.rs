//! Serviços do Tako sem interface (PRD D-09, WEB-01): o que a interface web pede e o que o núcleo avisa.
//!
//! A interface só desenha; dados e regras ficam aqui e nos crates do núcleo. Nada neste crate conhece CEF,
//! janela ou React: o hospedeiro (`tako-web`) recebe o JSON, chama `AppService::handle` e devolve a resposta.

pub mod protocol;

use protocol::{
    AppInfo, BRIDGE_VERSION, BridgeError, Call, ErrorCode, Event, Reply, UiState, WindowAction,
};
use std::sync::mpsc::Sender;

/// Maior estado de tela guardado (`ui.state.set`); é só onde a interface estava, não dados.
const MAX_UI_STATE_BYTES: usize = 64 * 1024;

/// O que só o hospedeiro sabe fazer (janela, sessão de jogo). A chamada responde na hora que o pedido foi aceito;
/// o resultado chega depois por evento.
#[derive(Debug, Clone, PartialEq)]
pub enum HostRequest {
    Window(WindowAction),
}

/// Atende as chamadas da interface. Uma instância por processo, na thread do núcleo.
pub struct AppService {
    version: String,
    events: Option<Sender<Event>>,
    host: Option<Sender<HostRequest>>,
    ui_state: Option<String>,
}

impl AppService {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
            events: None,
            host: None,
            ui_state: None,
        }
    }

    /// Para onde vão os eventos do núcleo; o hospedeiro repassa à interface aberta (ou descarta, com ela fechada).
    pub fn set_event_sink(&mut self, sink: Sender<Event>) {
        self.events = Some(sink);
    }

    /// Para onde vão os pedidos ao hospedeiro (`HostRequest`).
    pub fn set_host_sink(&mut self, sink: Sender<HostRequest>) {
        self.host = Some(sink);
    }

    pub fn handle(&mut self, call: Call) -> Result<Reply, BridgeError> {
        match call {
            Call::AppInfo => Ok(Reply::AppInfo(AppInfo {
                version: self.version.clone(),
                bridge_version: BRIDGE_VERSION,
            })),
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
        }
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
