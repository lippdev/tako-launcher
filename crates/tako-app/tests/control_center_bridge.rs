//! Central rápida durante o jogo pela ponte (WEB-22).
//!
//! Exercita `control_center.state`, `control_center.set`, `control_center.open`, o atalho
//! e as validações de volume, mudo, saída de áudio e modo de tela.

#![allow(clippy::unwrap_used)]

use serde_json::json;
use std::sync::mpsc;
use tako_app::protocol::{
    Call, ControlCenterAction, ErrorCode, Event, Reply, Setting, parse_request,
};
use tako_app::{AppService, Core};
use tako_core::input::shortcut::{KeyModifiers, KeyShortcut};
use tako_core::paths::DataPaths;
use tako_platform::Platform;

fn setup_service() -> (AppService, tempfile::TempDir, mpsc::Receiver<Event>) {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path());
    let core = Core::open(paths, Platform::simulated(), true).unwrap();
    let mut app = AppService::with_core("test", core);
    let (tx, rx) = mpsc::channel();
    app.set_event_sink(tx);
    (app, dir, rx)
}

#[test]
fn state_without_game_reports_closed_and_rejects_mutations() {
    let (mut app, _dir, _events) = setup_service();
    assert!(app.central_shortcut().is_none());
    assert!(!app.central_open());

    let Reply::ControlCenterState(state) = app.handle(Call::ControlCenterState).unwrap() else {
        panic!("esperado ControlCenterState");
    };
    assert!(!state.open);
    assert!(!state.display_managed);
    assert!(state.audio.is_some());
    assert!(!state.displays.is_empty());

    let err = app
        .handle(Call::ControlCenterSet(ControlCenterAction::Volume {
            value: 0.5,
        }))
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::Conflict);

    let err_mute = app
        .handle(Call::ControlCenterSet(ControlCenterAction::Mute {
            muted: true,
        }))
        .unwrap_err();
    assert_eq!(err_mute.code, ErrorCode::Conflict);
}

#[test]
fn shortcut_and_open_follow_session_lifecycle() {
    let (mut app, _dir, events) = setup_service();

    // Sem jogo, set_central_open(true) não surte efeito nem emite evento.
    app.set_central_open(true);
    assert!(!app.central_open());
    assert!(events.try_recv().is_err());

    // Inicia um jogo simulado.
    app.launch_game("steam:demo0");
    assert_eq!(app.central_shortcut(), Some(KeyShortcut::default()));
    assert!(!app.central_open());

    // Descarta eventos de abertura do jogo (ex.: LibraryChanged).
    while events.try_recv().is_ok() {}

    // Abre a Central: estado muda e evento é emitido.
    app.set_central_open(true);
    assert!(app.central_open());
    assert_eq!(events.try_recv().unwrap(), Event::ControlCenterOpen(true));

    // Repetir a mesma abertura não emite evento redundante.
    app.set_central_open(true);
    assert!(events.try_recv().is_err());

    // Fecha a Central: estado muda e evento é emitido.
    app.set_central_open(false);
    assert!(!app.central_open());
    assert_eq!(events.try_recv().unwrap(), Event::ControlCenterOpen(false));
}

#[test]
fn custom_shortcut_from_settings_is_honored() {
    let (mut app, _dir, _events) = setup_service();
    let custom = KeyShortcut::new(KeyModifiers::ALT, "F10");
    app.handle(Call::SettingsSet(Setting {
        key: "ControlCenterKey".into(),
        value: custom.serialize(),
    }))
    .unwrap();
    app.launch_game("steam:demo0");

    assert_eq!(app.central_shortcut(), Some(custom));
}

#[test]
fn control_center_actions_modify_devices_with_validation() {
    let (mut app, _dir, _events) = setup_service();
    app.launch_game("steam:demo0");

    // Volume inválido (fora do intervalo 0.0..=1.0 e não finito).
    for bad_vol in [-0.1, 1.01, f32::NAN, f32::INFINITY] {
        let err = app
            .handle(Call::ControlCenterSet(ControlCenterAction::Volume {
                value: bad_vol,
            }))
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::BadRequest);
    }

    // Volume válido aplica alteração e devolve estado atualizado.
    let Reply::ControlCenterSet(state) = app
        .handle(Call::ControlCenterSet(ControlCenterAction::Volume {
            value: 0.75,
        }))
        .unwrap()
    else {
        panic!("esperado ControlCenterSet");
    };
    assert_eq!(state.audio.as_ref().and_then(|a| a.volume), Some(0.75));

    // Mudo altera estado.
    let Reply::ControlCenterSet(state) = app
        .handle(Call::ControlCenterSet(ControlCenterAction::Mute {
            muted: true,
        }))
        .unwrap()
    else {
        panic!("esperado ControlCenterSet");
    };
    assert!(state.audio.as_ref().is_some_and(|a| a.muted));

    // Troca de saída de áudio para dispositivo existente.
    let Reply::ControlCenterSet(state) = app
        .handle(Call::ControlCenterSet(ControlCenterAction::AudioOutput {
            id: "tv".into(),
        }))
        .unwrap()
    else {
        panic!("esperado ControlCenterSet");
    };
    assert_eq!(
        state.audio.as_ref().and_then(|a| a.default_id.as_deref()),
        Some("tv")
    );

    // Saída de áudio desconhecida falha com NotFound.
    let err = app
        .handle(Call::ControlCenterSet(ControlCenterAction::AudioOutput {
            id: "nao_existe".into(),
        }))
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::NotFound);

    // Modo de tela inválido / desconhecido falha com BadRequest.
    let err = app
        .handle(Call::ControlCenterSet(ControlCenterAction::DisplayMode {
            id: "DISPLAY1".into(),
            mode: "modo_invalido".into(),
        }))
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::BadRequest);

    // Modo válido oferecido pela tela aplica com sucesso.
    let Reply::ControlCenterSet(state) = app
        .handle(Call::ControlCenterSet(ControlCenterAction::DisplayMode {
            id: "DISPLAY1".into(),
            mode: "1920x1080@60".into(),
        }))
        .unwrap()
    else {
        panic!("esperado ControlCenterSet");
    };
    let display = state
        .displays
        .iter()
        .find(|d| d.id == "DISPLAY1")
        .expect("DISPLAY1");
    assert_eq!(display.current_mode.as_deref(), Some("1920x1080@60"));
}

#[test]
fn bridge_wire_format_parses_and_serializes_correctly() {
    // Validação de envelope para control_center.state sem argumentos.
    let req = parse_request(r#"{"v":1,"id":10,"name":"control_center.state"}"#).unwrap();
    assert_eq!(req.call, Call::ControlCenterState);

    // Com argumento deve ser rejeitado como BadRequest.
    let err = parse_request(r#"{"v":1,"id":11,"name":"control_center.state","payload":{}}"#);
    assert_eq!(err.unwrap_err().1.code, ErrorCode::BadRequest);

    // control_center.set com payload bem formado.
    let req = parse_request(
        r#"{"v":1,"id":12,"name":"control_center.set","payload":{"action":"volume","value":0.4}}"#,
    )
    .unwrap();
    assert_eq!(
        req.call,
        Call::ControlCenterSet(ControlCenterAction::Volume { value: 0.4 })
    );

    // control_center.set com payload malformado (tipo errado ou campos estranhos).
    for bad in [
        r#"{"v":1,"id":13,"name":"control_center.set","payload":{"action":"volume","value":"alta"}}"#,
        r#"{"v":1,"id":14,"name":"control_center.set","payload":{"action":"mute"}}"#,
        r#"{"v":1,"id":15,"name":"control_center.set","payload":{"action":"inexistente"}}"#,
    ] {
        let err = parse_request(bad);
        assert_eq!(err.unwrap_err().1.code, ErrorCode::BadRequest);
    }

    // Serialização de evento.
    assert_eq!(
        serde_json::to_value(Event::ControlCenterOpen(true)).unwrap(),
        json!({"name": "control_center.open", "payload": true})
    );
    assert_eq!(
        serde_json::to_value(Event::ControlCenterOpen(false)).unwrap(),
        json!({"name": "control_center.open", "payload": false})
    );
}
