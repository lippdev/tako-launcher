//! Lado Rust da ponte: texto JSON da interface → `AppService` → texto JSON de volta. Independe do motor
//! (CEF ou WebView2): cada transporte só entrega e devolve strings.

use tako_app::AppService;
use tako_app::protocol::{BRIDGE_VERSION, Event, EventEnvelope, Response};

/// Maior mensagem aceita da interface. Pedidos são pequenos; acima disso é erro de quem mandou.
pub const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

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
