//! Testes da ponte do seletor de arquivos e pastas do sistema operacional (WEB-12e).

use std::sync::mpsc;
use tako_app::protocol::{
    Call, ErrorCode, Event, PickFilter, PickKind, PickResponse, Request, Response,
    SystemPickRequest, SystemPicked,
};
use tako_app::{AppService, HostRequest};

#[test]
fn system_pick_dispatches_host_request_and_replies_with_id() {
    let mut app = AppService::new("1.0.0");
    let (host_tx, host_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();
    app.set_host_sink(host_tx);
    app.set_event_sink(event_tx);

    let req = SystemPickRequest {
        kind: PickKind::File,
        purpose: "add_game".into(),
        filters: Some(vec![PickFilter {
            name: "Jogos".into(),
            extensions: vec!["exe".into(), "lnk".into()],
        }]),
    };

    let reply = app.handle(Call::SystemPick(req.clone())).unwrap();
    let id = match reply {
        tako_app::protocol::Reply::SystemPick(PickResponse { id }) => id,
        other => panic!("resposta inesperada: {other:?}"),
    };
    assert_eq!(id, 1);

    // Confere se o hospedeiro recebeu o pedido correto
    let host_req = host_rx.try_recv().expect("HostRequest esperado");
    assert_eq!(
        host_req,
        HostRequest::Pick {
            id: 1,
            kind: PickKind::File,
            purpose: "add_game".into(),
            filters: req.filters,
        }
    );

    // Entrega o caminho escolhido
    let accepted = app.deliver_picked(1, Some("C:\\Jogos\\game.exe".into()));
    assert!(accepted);

    let event = event_rx.try_recv().expect("evento system.picked esperado");
    assert_eq!(
        event,
        Event::SystemPicked(SystemPicked {
            id: 1,
            path: Some("C:\\Jogos\\game.exe".into()),
        })
    );
}

#[test]
fn system_pick_cancelled_delivers_null_path() {
    let mut app = AppService::new("1.0.0");
    let (host_tx, _host_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();
    app.set_host_sink(host_tx);
    app.set_event_sink(event_tx);

    let reply = app
        .handle(Call::SystemPick(SystemPickRequest {
            kind: PickKind::Folder,
            purpose: "emulators".into(),
            filters: None,
        }))
        .unwrap();

    let id = match reply {
        tako_app::protocol::Reply::SystemPick(PickResponse { id }) => id,
        other => panic!("resposta inesperada: {other:?}"),
    };

    // Usuário cancelou o diálogo -> None
    let accepted = app.deliver_picked(id, None);
    assert!(accepted);

    let event = event_rx.try_recv().expect("evento system.picked esperado");
    assert_eq!(event, Event::SystemPicked(SystemPicked { id, path: None }));

    // Não aceita segunda entrega para o mesmo id
    assert!(!app.deliver_picked(id, Some("outra".into())));
    assert!(event_rx.try_recv().is_err());
}

#[test]
fn deliver_picked_rejects_unknown_id() {
    let mut app = AppService::new("1.0.0");
    let (event_tx, event_rx) = mpsc::channel();
    app.set_event_sink(event_tx);

    let accepted = app.deliver_picked(999, Some("C:\\invadido.exe".into()));
    assert!(!accepted, "id desconhecido não pode ser aceito");
    assert!(event_rx.try_recv().is_err());
}

#[test]
fn system_pick_fails_without_host() {
    let mut app = AppService::new("1.0.0");
    let err = app
        .handle(Call::SystemPick(SystemPickRequest {
            kind: PickKind::File,
            purpose: "test".into(),
            filters: None,
        }))
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::Internal);
}

#[test]
fn system_pick_wire_format_round_trip() {
    let json = r#"{"v":1,"id":42,"name":"system.pick","payload":{"kind":"file","purpose":"add_game","filters":[{"name":"Executáveis","extensions":["exe"]}]}}"#;
    let req: Request = serde_json::from_str(json).unwrap();
    assert_eq!(req.id, 42);
    match req.call {
        Call::SystemPick(p) => {
            assert_eq!(p.kind, PickKind::File);
            assert_eq!(p.purpose, "add_game");
            assert_eq!(
                p.filters,
                Some(vec![PickFilter {
                    name: "Executáveis".into(),
                    extensions: vec!["exe".into()],
                }])
            );
        }
        other => panic!("chamada inesperada: {other:?}"),
    }

    let ok = Response::ok(
        42,
        tako_app::protocol::Reply::SystemPick(PickResponse { id: 7 }),
    );
    let res_json = serde_json::to_string(&ok).unwrap();
    assert_eq!(
        res_json,
        r#"{"v":1,"id":42,"ok":true,"reply":{"name":"system.pick","data":{"id":7}}}"#
    );

    let event = Event::SystemPicked(SystemPicked {
        id: 7,
        path: Some("C:\\test.exe".into()),
    });
    let ev_json = serde_json::to_string(&tako_app::protocol::EventEnvelope::from(event)).unwrap();
    assert_eq!(
        ev_json,
        r#"{"v":1,"event":{"name":"system.picked","payload":{"id":7,"path":"C:\\test.exe"}}}"#
    );

    let event_cancelled = Event::SystemPicked(SystemPicked { id: 7, path: None });
    let ev_cancel_json =
        serde_json::to_string(&tako_app::protocol::EventEnvelope::from(event_cancelled)).unwrap();
    assert_eq!(
        ev_cancel_json,
        r#"{"v":1,"event":{"name":"system.picked","payload":{"id":7,"path":null}}}"#
    );
}
