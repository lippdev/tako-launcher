//! Lado Rust da ponte: texto JSON da interface → `AppService` → texto JSON de volta. Independe do motor
//! (CEF ou WebView2): cada transporte só entrega e devolve strings.

use tako_app::AppService;
use tako_app::protocol::{BRIDGE_VERSION, Event, EventEnvelope, Response};

/// Maior mensagem aceita da interface. Pedidos são pequenos; acima disso é erro de quem mandou.
pub use tako_app::protocol::MAX_MESSAGE_BYTES;

/// Uma única thread drena os eventos depois de handle/poll; a resposta sempre precede seus eventos.
#[cfg(any(all(windows, feature = "cef"), test))]
pub(crate) fn drain_messages(
    response: Option<String>,
    events: &std::sync::mpsc::Receiver<Event>,
) -> Vec<String> {
    response
        .into_iter()
        .chain(events.try_iter().map(event_message))
        .collect()
}

/// Atende uma mensagem da interface e devolve a resposta serializada. Nunca entra em pânico com entrada ruim.
pub fn handle_message(app: &mut AppService, text: &str) -> String {
    serialize(&respond(app, text))
}

fn respond(app: &mut AppService, text: &str) -> Response {
    let req = match tako_app::protocol::parse_request(text) {
        Ok(request) => request,
        Err((id, error)) => return Response::err(id, error),
    };
    match app.handle(req.call) {
        Ok(reply) => Response::ok(req.id, reply),
        Err(error) => {
            tracing::warn!("ponte: {error}");
            Response::err(req.id, error)
        }
    }
}

/// Evento pronto para mandar à interface.
pub fn event_message(event: Event) -> String {
    serde_json::to_string(&EventEnvelope::from(event)).unwrap_or_default()
}

fn serialize(response: &Response) -> String {
    serde_json::to_string(response).unwrap_or_else(|e| {
        tracing::error!("ponte: resposta não serializou: {e}");
        format!(
            r#"{{"v":{BRIDGE_VERSION},"id":{},"ok":false,"error":{{"code":"internal","message":"serialização"}}}}"#,
            response.id
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_precedes_caused_events_and_poll_preserves_event_order() {
        let dir = tempfile::tempdir().unwrap();
        let core = tako_app::Core::open_with_sources(
            tako_core::paths::DataPaths::at(dir.path()),
            tako_platform::Platform::simulated(),
            false,
            Vec::new(),
        )
        .unwrap();
        let mut app = AppService::with_core("1", core);
        let (tx, events) = std::sync::mpsc::channel();
        app.set_event_sink(tx);
        // Esperar a leitura inicial vazia antes de provocar outra por library.scan.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let response = handle_message(&mut app, r#"{"v":1,"id":1,"name":"library.sources"}"#);
            let response: serde_json::Value = serde_json::from_str(&response).unwrap();
            if response["reply"]["data"]["scanning"] == false {
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        drain_messages(None, &events);
        let response = handle_message(&mut app, r#"{"v":1,"id":7,"name":"library.scan"}"#);
        let messages = drain_messages(Some(response), &events);
        let response: serde_json::Value = serde_json::from_str(&messages[0]).unwrap();
        assert_eq!(response["id"], 7);
        assert_eq!(response["ok"], true);
        let event: serde_json::Value = serde_json::from_str(&messages[1]).unwrap();
        assert_eq!(event["event"]["name"], "library.scan.progress");
        // Ao terminar, poll emite library.changed antes do progresso final.
        let mut polled = Vec::new();
        loop {
            app.poll();
            polled.extend(drain_messages(None, &events));
            if polled.len() == 4 {
                let names = polled
                    .iter()
                    .map(|text| {
                        serde_json::from_str::<serde_json::Value>(text).unwrap()["event"]["name"]
                            .as_str()
                            .unwrap()
                            .to_owned()
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    names,
                    [
                        "library.scan.progress",
                        "library.scan.progress",
                        "library.changed",
                        "library.scan.progress"
                    ]
                );
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    fn call(text: &str) -> serde_json::Value {
        let mut app = AppService::new("9.9.9");
        serde_json::from_str(&handle_message(&mut app, text)).unwrap()
    }

    #[test]
    fn answers_a_valid_call() {
        let r = call(r#"{"v":1,"id":3,"name":"app.info"}"#);
        assert_eq!(r["ok"], true);
        assert_eq!(r["id"], 3);
        assert_eq!(r["reply"]["data"]["version"], "9.9.9");
    }

    #[test]
    fn unknown_call_keeps_the_id() {
        let r = call(r#"{"v":1,"id":4,"name":"nao.existe"}"#);
        assert_eq!(r["ok"], false);
        assert_eq!(r["id"], 4);
        assert_eq!(r["error"]["code"], "unknown_call");
    }

    #[test]
    fn rejects_other_versions() {
        let r = call(r#"{"v":99,"id":5,"name":"app.info"}"#);
        assert_eq!(r["error"]["code"], "version_mismatch");
    }

    #[test]
    fn broken_input_is_a_bad_request() {
        for text in [
            "",
            "{",
            "null",
            "[]",
            r#"{"v":1}"#,
            r#"{"v":"1","id":1,"name":"app.info"}"#,
        ] {
            let r = call(text);
            assert_eq!(r["ok"], false, "{text}");
            assert_eq!(r["error"]["code"], "bad_request", "{text}");
        }
    }

    #[test]
    fn oversized_message_is_rejected_without_parsing() {
        let big = format!(
            r#"{{"v":1,"id":1,"name":"app.info","x":"{}"}}"#,
            "a".repeat(MAX_MESSAGE_BYTES)
        );
        assert_eq!(call(&big)["error"]["code"], "bad_request");
    }

    #[test]
    fn event_wire_format() {
        assert_eq!(
            event_message(Event::LibraryChanged),
            r#"{"v":1,"event":{"name":"library.changed"}}"#
        );
    }
}
