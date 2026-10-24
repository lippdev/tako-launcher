//! Serviços do Tako sem interface (PRD D-09, WEB-01): o que a interface web pede e o que o núcleo avisa.
//!
//! A interface só desenha; dados e regras ficam aqui e nos crates do núcleo. Nada neste crate conhece CEF,
//! janela ou React: o hospedeiro (`tako-web`) recebe o JSON, chama `AppService::handle` e devolve a resposta.

pub mod protocol;

use protocol::{AppInfo, BRIDGE_VERSION, BridgeError, Call, Event, Reply};
use std::sync::mpsc::Sender;

/// Atende as chamadas da interface. Uma instância por processo, na thread do núcleo.
pub struct AppService {
    version: String,
    events: Option<Sender<Event>>,
}

impl AppService {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
            events: None,
        }
    }

    /// Para onde vão os eventos do núcleo; o hospedeiro repassa à interface aberta (ou descarta, com ela fechada).
    pub fn set_event_sink(&mut self, sink: Sender<Event>) {
        self.events = Some(sink);
    }

    pub fn handle(&mut self, call: Call) -> Result<Reply, BridgeError> {
        match call {
            Call::AppInfo => Ok(Reply::AppInfo(AppInfo {
                version: self.version.clone(),
                bridge_version: BRIDGE_VERSION,
            })),
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
