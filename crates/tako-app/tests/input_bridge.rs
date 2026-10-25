//! Controle pela ponte (WEB-05b): leituras simuladas do SDL3 → `input.nav`, `input.controller`, `input.notice` e o
//! atalho da Central, com as regras da interface Slint (zona morta, repetição, perfis, avisos). Nenhum controle de
//! verdade: as leituras são montadas à mão.

#![allow(clippy::unwrap_used)]

use serde_json::{Value, json};
use std::time::Duration;
use tako_app::AppService;
use tako_app::input::{BatteryReading, InputBridge, InputOutput, InputScene, PadChange, PadFrame};
use tako_app::protocol::{
    ControllerInfo, ControllerStyle, Event, InputAction, InputNav, InputNotice, InputNoticeKind,
};
use tako_core::input::GamepadButtons as B;
use tako_core::input::pad::PadFamily;
use tako_core::input::profiles::{DEFAULT_ID, Profiles};

const PAD: &str = "030000005e0400000b0b000000000000";
const PAD2: &str = "050000004c050000e60c000000000000";

fn ms(v: u64) -> Duration {
    Duration::from_millis(v)
}

const OPEN: InputScene = InputScene {
    listening: true,
    playing: false,
    over_game: false,
};

const PLAYING: InputScene = InputScene {
    listening: false,
    playing: true,
    over_game: false,
};

/// Um Xbox conectado (sem botões), interface aberta.
fn bridge() -> InputBridge {
    let mut b = InputBridge::new(Profiles::default());
    b.set_scene(OPEN);
    // Primeira leitura depois de abrir: o que estivesse apertado ficaria para depois de solto.
    b.update(frame(B::NONE), ms(0));
    b
}

fn frame(buttons: B) -> PadFrame {
    PadFrame {
        pads: vec![(PAD.into(), buttons)],
        count: 1,
        family: PadFamily::Xbox,
        name: "Xbox Controller".into(),
        ..Default::default()
    }
}

fn stick(x: f32, y: f32) -> PadFrame {
    PadFrame {
        left_x: x,
        left_y: y,
        ..frame(B::NONE)
    }
}

fn connect(name: &str, family: PadFamily, count: usize) -> PadFrame {
    PadFrame {
        changes: vec![PadChange::Connected {
            name: name.into(),
            family,
            guid: PAD.into(),
        }],
        count,
        family,
        name: name.into(),
        pads: vec![(PAD.into(), B::NONE)],
        ..Default::default()
    }
}

fn disconnect(remaining: usize) -> PadFrame {
    PadFrame {
        changes: vec![PadChange::Disconnected { remaining }],
        count: remaining,
        name: if remaining > 0 {
            "Xbox Controller".into()
        } else {
            String::new()
        },
        pads: if remaining > 0 {
            vec![(PAD2.into(), B::NONE)]
        } else {
            Vec::new()
        },
        ..Default::default()
    }
}

/// Só os `input.nav` de uma volta.
fn navs(out: &[InputOutput]) -> Vec<InputNav> {
    out.iter()
        .filter_map(|o| match o {
            InputOutput::Event(Event::InputNav(nav)) => Some(*nav),
            _ => None,
        })
        .collect()
}

fn notices(out: &[InputOutput]) -> Vec<InputNotice> {
    out.iter()
        .filter_map(|o| match o {
            InputOutput::Event(Event::InputNotice(n)) => Some(n.clone()),
            _ => None,
        })
        .collect()
}

fn controller(out: &[InputOutput]) -> Option<ControllerInfo> {
    out.iter().find_map(|o| match o {
        InputOutput::Event(Event::InputController(info)) => Some(info.clone()),
        _ => None,
    })
}

fn nav(action: InputAction, repeat: bool) -> InputNav {
    InputNav { action, repeat }
}

#[test]
fn the_stick_has_the_slint_dead_zone_and_y_down_is_down() {
    let mut b = bridge();
    // SDL: Y positivo para baixo. 0,5 ainda é zona morta.
    assert!(navs(&b.update(stick(0.0, 0.5), ms(0))).is_empty());
    assert!(navs(&b.update(stick(-0.45, 0.0), ms(16))).is_empty());
    assert_eq!(
        navs(&b.update(stick(0.0, 0.6), ms(32))),
        [nav(InputAction::Down, false)]
    );
    assert!(navs(&b.update(stick(0.0, 0.0), ms(48))).is_empty());
    assert_eq!(
        navs(&b.update(stick(0.0, -0.9), ms(64))),
        [nav(InputAction::Up, false)]
    );
    assert!(navs(&b.update(stick(0.0, 0.0), ms(80))).is_empty());
    assert_eq!(
        navs(&b.update(stick(-0.8, 0.0), ms(96))),
        [nav(InputAction::Left, false)]
    );
}

#[test]
fn a_held_direction_repeats_after_380_ms_and_then_every_110_ms() {
    let mut b = bridge();
    let down = || frame(B::DPAD_DOWN);
    assert_eq!(
        navs(&b.update(down(), ms(0))),
        [nav(InputAction::Down, false)]
    );
    let mut t = 16;
    while t < 380 {
        assert!(
            navs(&b.update(down(), ms(t))).is_empty(),
            "repetiu em {t} ms"
        );
        t += 16;
    }
    assert_eq!(
        navs(&b.update(down(), ms(380))),
        [nav(InputAction::Down, true)]
    );
    assert!(navs(&b.update(down(), ms(480))).is_empty());
    assert_eq!(
        navs(&b.update(down(), ms(490))),
        [nav(InputAction::Down, true)]
    );
    // Soltou e apertou de novo: aperto novo, não repetição.
    assert!(navs(&b.update(frame(B::NONE), ms(500))).is_empty());
    assert_eq!(
        navs(&b.update(down(), ms(516))),
        [nav(InputAction::Down, false)]
    );
}

#[test]
fn buttons_are_actions_once_per_press_and_the_guide_alone_is_nothing() {
    let mut b = bridge();
    let cases = [
        (B::A, InputAction::Accept),
        (B::B, InputAction::Back),
        (B::X, InputAction::Favorite),
        (B::Y, InputAction::Search),
        (B::LEFT_SHOULDER, InputAction::PreviousSection),
        (B::RIGHT_SHOULDER, InputAction::NextSection),
        (B::START, InputAction::Menu),
    ];
    let mut t = 0;
    for (button, action) in cases {
        assert_eq!(navs(&b.update(frame(button), ms(t))), [nav(action, false)]);
        // Segurado, botão não repete (só direção).
        assert!(navs(&b.update(frame(button), ms(t + 1000))).is_empty());
        assert!(navs(&b.update(frame(B::NONE), ms(t + 1016))).is_empty());
        t += 2000;
    }
    assert!(navs(&b.update(frame(B::GUIDE), ms(t))).is_empty());
    assert!(navs(&b.update(frame(B::VIEW), ms(t + 16))).is_empty());
}

#[test]
fn each_pad_uses_its_profile() {
    let mut profiles = Profiles::default();
    let swapped = profiles.create("Trocado", DEFAULT_ID).unwrap();
    profiles
        .get_mut(swapped)
        .unwrap()
        .buttons
        .assign(tako_core::input::AppCommand::Accept, B::B);
    profiles.assign(PAD, "Xbox Controller", swapped);
    let mut b = InputBridge::new(Profiles::default());
    b.set_scene(OPEN);
    b.set_profiles(profiles);
    b.update(frame(B::NONE), ms(0));
    // Neste controle o B aceita (e o A, que ficou com o "voltar", volta).
    assert_eq!(
        navs(&b.update(frame(B::B), ms(0))),
        [nav(InputAction::Accept, false)]
    );
    assert!(navs(&b.update(frame(B::NONE), ms(16))).is_empty());
    // Outro controle segue o padrão.
    let other = PadFrame {
        pads: vec![(PAD2.into(), B::B)],
        ..frame(B::NONE)
    };
    assert_eq!(
        navs(&b.update(other, ms(32))),
        [nav(InputAction::Back, false)]
    );
}

#[test]
fn nothing_reaches_a_closed_interface_or_one_behind_the_game() {
    let mut b = InputBridge::new(Profiles::default());
    // Interface fechada.
    assert!(navs(&b.update(frame(B::A), ms(0))).is_empty());
    assert!(navs(&b.update(frame(B::NONE), ms(16))).is_empty());
    // Jogo abrindo com a interface ainda aberta ("Abrindo…"): o controle já é do jogo.
    b.set_scene(InputScene {
        listening: true,
        playing: true,
        over_game: false,
    });
    assert!(navs(&b.update(frame(B::A), ms(32))).is_empty());
    assert!(navs(&b.update(frame(B::DPAD_DOWN), ms(48))).is_empty());
    assert_eq!(b.timeout(), InputBridge::FRAME);
    b.set_scene(PLAYING);
    assert!(navs(&b.update(frame(B::A), ms(64))).is_empty());
    assert_eq!(b.timeout(), InputBridge::IDLE);
}

#[test]
fn reopening_ignores_what_is_still_held_until_released() {
    let mut b = InputBridge::new(Profiles::default());
    b.set_scene(PLAYING);
    b.update(frame(B::A), ms(0));
    b.set_scene(InputScene {
        listening: true,
        playing: true,
        over_game: true,
    });
    // O A que ficou apertado não aceita nada na interface que acabou de abrir.
    assert!(navs(&b.update(frame(B::A), ms(16))).is_empty());
    assert!(navs(&b.update(frame(B::A), ms(500))).is_empty());
    assert!(navs(&b.update(frame(B::NONE), ms(516))).is_empty());
    assert_eq!(
        navs(&b.update(frame(B::A), ms(532))),
        [nav(InputAction::Accept, false)]
    );
}

#[test]
fn the_central_combination_during_the_game_reopens_the_interface() {
    let mut b = InputBridge::new(Profiles::default());
    b.set_scene(PLAYING);
    let chord = || frame(B::VIEW | B::Y);
    assert!(!b.update(chord(), ms(0)).contains(&InputOutput::Central));
    // Segurando: acorda a cada 50 ms para não depender de evento novo do SDL.
    assert_eq!(b.timeout(), InputBridge::HOLDING);
    assert!(!b.update(chord(), ms(400)).contains(&InputOutput::Central));
    let out = b.update(chord(), ms(500));
    assert_eq!(out, [InputOutput::Central]);
    // Uma vez por aperto.
    assert!(b.update(chord(), ms(1200)).is_empty());
    b.update(frame(B::NONE), ms(1300));

    // Com a interface por cima do jogo, a mesma combinação volta ao jogo; o Y dela não abre a busca.
    b.set_scene(InputScene {
        listening: true,
        playing: true,
        over_game: true,
    });
    assert!(navs(&b.update(chord(), ms(2000))).is_empty());
    assert_eq!(b.update(chord(), ms(2500)), [InputOutput::Central]);
}

#[test]
fn the_combination_without_a_game_is_not_the_central() {
    let mut b = bridge();
    b.update(frame(B::VIEW | B::Y), ms(0));
    let out = b.update(frame(B::VIEW | B::Y), ms(600));
    assert!(!out.contains(&InputOutput::Central));
    // O Y segurado com o View é da combinação: não vira busca.
    assert!(navs(&out).is_empty());
}

#[test]
fn disconnecting_and_reconnecting_show_the_slint_notices() {
    let mut b = InputBridge::new(Profiles::default());
    b.set_scene(OPEN);
    let out = b.update(
        connect("DualSense Wireless Controller", PadFamily::PlayStation, 1),
        ms(0),
    );
    // Conectar o primeiro não é aviso; o estado muda.
    assert!(notices(&out).is_empty());
    assert_eq!(
        controller(&out),
        Some(ControllerInfo {
            connected: true,
            count: 1,
            style: ControllerStyle::Playstation,
            name: "DualSense Wireless Controller".into(),
            battery: None,
        })
    );
    let out = b.update(disconnect(0), ms(1000));
    assert_eq!(
        notices(&out),
        [InputNotice {
            kind: InputNoticeKind::Disconnected,
            pad: None,
            percent: None
        }]
    );
    let info = controller(&out).unwrap();
    assert!(!info.connected);
    assert_eq!(info.name, "");
    let out = b.update(connect("Xbox Controller", PadFamily::Xbox, 1), ms(5000));
    assert_eq!(notices(&out)[0].kind, InputNoticeKind::Connected);
    assert_eq!(controller(&out).unwrap().style, ControllerStyle::Xbox);
    // Navega de novo logo depois de voltar.
    assert_eq!(
        navs(&b.update(frame(B::A), ms(5016))),
        [nav(InputAction::Accept, false)]
    );
}

#[test]
fn losing_one_of_two_pads_warns_without_a_welcome_back_later() {
    let mut b = bridge();
    b.update(connect("Xbox Controller", PadFamily::Xbox, 2), ms(0));
    let out = b.update(disconnect(1), ms(100));
    assert_eq!(notices(&out)[0].kind, InputNoticeKind::Disconnected);
    let out = b.update(connect("Xbox Controller", PadFamily::Xbox, 2), ms(200));
    assert!(notices(&out).is_empty());
}

#[test]
fn no_notice_while_playing() {
    let mut b = InputBridge::new(Profiles::default());
    b.set_scene(OPEN);
    b.update(connect("Xbox Controller", PadFamily::Xbox, 1), ms(0));
    b.set_scene(PLAYING);
    assert!(notices(&b.update(disconnect(0), ms(100))).is_empty());
    b.set_scene(OPEN);
    assert!(notices(&b.update(connect("Xbox Controller", PadFamily::Xbox, 1), ms(200))).is_empty());
}

#[test]
fn battery_is_read_every_30_s_with_the_interface_open_and_warns_once() {
    let mut b = InputBridge::new(Profiles::default());
    // Sem controle ou sem interface, não lê.
    assert!(!b.wants_battery(ms(0)));
    b.update(connect("Xbox Controller", PadFamily::Xbox, 1), ms(0));
    assert!(!b.wants_battery(ms(0)));
    b.set_scene(OPEN);
    assert!(b.wants_battery(ms(0)));
    let out = b.update(
        PadFrame {
            battery: Some(BatteryReading {
                active: Some(12),
                low: vec![("Xbox Controller".into(), 12)],
            }),
            ..frame(B::NONE)
        },
        ms(16),
    );
    assert_eq!(
        notices(&out),
        [InputNotice {
            kind: InputNoticeKind::BatteryLow,
            pad: Some("Xbox Controller".into()),
            percent: Some(12)
        }]
    );
    assert_eq!(controller(&out).unwrap().battery, Some(12));
    assert!(!b.wants_battery(ms(29_000)));
    assert!(b.wants_battery(ms(30_016)));
    // Um controle que entra ou sai: lê de novo já.
    b.update(disconnect(1), ms(31_000));
    assert!(b.wants_battery(ms(31_016)));
    // Sem mudança, nada de `input.controller` repetido.
    let out = b.update(frame(B::NONE), ms(31_032));
    assert_eq!(controller(&out), None);
}

#[test]
fn the_controller_reaches_the_interface_by_call_and_event() {
    let mut app = AppService::new("1");
    let (tx, rx) = std::sync::mpsc::channel();
    app.set_event_sink(tx);
    let ask = |app: &mut AppService| -> Value {
        let reply = app
            .handle(tako_app::protocol::Call::InputController)
            .unwrap();
        serde_json::to_value(reply).unwrap()
    };
    assert_eq!(
        ask(&mut app),
        json!({"name": "input.controller", "data": {"connected": false, "count": 0, "style": "xbox", "name": "", "battery": null}})
    );
    let info = ControllerInfo {
        connected: true,
        count: 1,
        style: ControllerStyle::Nintendo,
        name: "Pro Controller".into(),
        battery: Some(80),
    };
    app.set_controller(info.clone());
    assert_eq!(rx.try_recv().unwrap(), Event::InputController(info.clone()));
    // Igual: sem evento repetido.
    app.set_controller(info);
    assert!(rx.try_recv().is_err());
    assert_eq!(ask(&mut app)["data"]["style"], "nintendo");
    assert_eq!(ask(&mut app)["data"]["battery"], 80);
}

#[test]
fn wire_format_of_the_input_events() {
    let wire = |event: Event| {
        serde_json::to_value(tako_app::protocol::EventEnvelope::from(event)).unwrap()
    };
    assert_eq!(
        wire(Event::InputNav(nav(InputAction::NextSection, false))),
        json!({"v": 1, "event": {"name": "input.nav", "payload": {"action": "next_section", "repeat": false}}})
    );
    assert_eq!(
        wire(Event::InputNotice(InputNotice {
            kind: InputNoticeKind::BatteryLow,
            pad: Some("DualSense".into()),
            percent: Some(10),
        })),
        json!({"v": 1, "event": {"name": "input.notice", "payload": {"kind": "battery_low", "pad": "DualSense", "percent": 10}}})
    );
    let request: tako_app::protocol::Request =
        serde_json::from_str(r#"{"v":1,"id":9,"name":"input.controller"}"#).unwrap();
    assert_eq!(request.call, tako_app::protocol::Call::InputController);
}

#[test]
fn profiles_come_from_the_settings_of_the_profile_in_use() {
    // Sem núcleo, o padrão.
    assert_eq!(AppService::new("1").pad_profiles(), Profiles::default());
    let dir = tempfile::tempdir().unwrap();
    let core = tako_app::Core::open(
        tako_core::paths::DataPaths::at(dir.path()),
        tako_platform::Platform::simulated(),
        true,
    )
    .unwrap();
    let mut app = AppService::with_core("1", core);
    // Guia + A segurados abrem a Central (a mesma chave que a interface Slint grava).
    let chord = (B::GUIDE | B::A).0.to_string();
    let request: tako_app::protocol::Request = serde_json::from_value(json!({
        "v": 1, "id": 1, "name": "settings.set",
        "payload": {"key": "ControlCenterChord", "value": chord}
    }))
    .unwrap();
    app.handle(request.call).unwrap();
    let profiles = app.pad_profiles();
    assert_eq!(profiles.of(PAD).chord, B::GUIDE | B::A);
    let mut b = InputBridge::new(Profiles::default());
    b.set_profiles(profiles);
    b.set_scene(PLAYING);
    b.update(frame(B::GUIDE | B::A), ms(0));
    assert_eq!(
        b.update(frame(B::GUIDE | B::A), ms(500)),
        [InputOutput::Central]
    );
}
