//! Teste de controle (Configurações › Diagnóstico) e a seção do relatório do sistema.
//! Port de `Input/ControllerProbe.cs` sem o relatório HID cru: o SDL3 já entrega o estado decodificado.

use super::{GamepadButtons as B, GamepadSnapshot};
use crate::i18n::{t, tf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Connection {
    Usb,
    Bluetooth,
}

/// Um controle visto agora, sem caminho de dispositivo nem número de série (o caminho Bluetooth traz o
/// endereço do controle): só ids USB, ligação e estado.
#[derive(Debug, Clone, PartialEq)]
pub struct ControllerProbe {
    /// Fonte da leitura (ex.: "SDL").
    pub source: String,
    /// Modelo (ex.: "DualSense Edge").
    pub model: String,
    pub connection: Option<Connection>,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    /// Estado agora ([`describe`]).
    pub state: String,
}

const BUTTONS: [(B, &str); 9] = [
    (B::A, "A/✕"),
    (B::B, "B/○"),
    (B::X, "X/□"),
    (B::Y, "Y/△"),
    (B::LEFT_SHOULDER, "LB/L1"),
    (B::RIGHT_SHOULDER, "RB/R1"),
    (B::VIEW, "View/Create"),
    (B::START, "Menu/Options"),
    (B::GUIDE, ""),
];

/// Direcional, botões e analógico esquerdo (o que a navegação usa).
pub fn describe(pad: GamepadSnapshot) -> String {
    let dpad: Vec<String> = [
        (B::DPAD_UP, "up"),
        (B::DPAD_DOWN, "down"),
        (B::DPAD_LEFT, "left"),
        (B::DPAD_RIGHT, "right"),
    ]
    .into_iter()
    .filter(|(b, _)| pad.buttons.contains(*b))
    .map(|(_, k)| t(&format!("input.test.{k}")))
    .collect();
    let buttons: Vec<String> = BUTTONS
        .into_iter()
        .filter(|(b, _)| pad.buttons.contains(*b))
        .map(|(b, l)| {
            if b == B::GUIDE {
                t("input.test.guide")
            } else {
                l.to_owned()
            }
        })
        .collect();
    let dpad = if dpad.is_empty() {
        t("input.test.released")
    } else {
        dpad.join("+")
    };
    let buttons = if buttons.is_empty() {
        t("input.test.no_buttons")
    } else {
        buttons.join(" ")
    };
    let stick = axes(pad.left_x, pad.left_y);
    tf(
        "input.test.state",
        &[("dpad", &dpad), ("buttons", &buttons), ("stick", &stick)],
    )
}

/// Uma linha de título por controle e o estado (seção do diagnóstico e da tela de teste).
pub fn lines(probes: &[ControllerProbe], steam_running: Option<bool>) -> Vec<String> {
    let steam = match steam_running {
        Some(true) => t("common.yes").to_lowercase(),
        Some(false) => t("common.no").to_lowercase(),
        None => "?".to_owned(),
    };
    let mut out = vec![tf("input.test.steam_running", &[("value", &steam)])];
    if probes.is_empty() {
        out.push(t("input.test.none"));
    }
    for p in probes {
        let mut header = format!("{} · {}", p.source, p.model);
        match p.connection {
            Some(Connection::Usb) => header.push_str(" · USB"),
            Some(Connection::Bluetooth) => header.push_str(" · Bluetooth"),
            None => {}
        }
        if let (Some(v), Some(pid)) = (p.vendor_id, p.product_id) {
            header.push_str(&format!(" · {v:04X}:{pid:04X}"));
        }
        out.push(header);
        out.push(format!("  {}", p.state));
    }
    out
}

fn axes(x: f64, y: f64) -> String {
    // "-0.00" vira "+0.00", como no C#.
    let f = |v: f64| if (v * 100.0).round() == 0.0 { 0.0 } else { v };
    format!("x {:+.2} y {:+.2}", f(x), f(y))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn describes_dpad_buttons_and_stick() {
        let text = describe(GamepadSnapshot::new(B::DPAD_UP | B::A, 0.0, 0.95));
        assert!(text.contains("direcional: cima"), "{text}");
        assert!(text.contains("A/✕"));
        assert!(text.contains("x +0.00 y +0.95"), "{text}");
        assert!(describe(GamepadSnapshot::default()).contains("direcional: solto"));
        assert!(describe(GamepadSnapshot::new(B::NONE, -0.001, 0.0)).contains("x +0.00"));
    }

    #[test]
    fn lines_show_source_model_connection_and_ids_without_device_path() {
        let probe = ControllerProbe {
            source: "SDL".into(),
            model: "DualSense Edge".into(),
            connection: Some(Connection::Bluetooth),
            vendor_id: Some(0x054C),
            product_id: Some(0x0DF2),
            state: "direcional: solto".into(),
        };
        let l = lines(&[probe], Some(false));
        assert_eq!(l[0], "Steam em execução: não");
        assert_eq!(l[1], "SDL · DualSense Edge · Bluetooth · 054C:0DF2");
        assert_eq!(l[2], "  direcional: solto");
        assert_eq!(
            lines(&[], None),
            ["Steam em execução: ?", "nenhum controle lido agora"]
        );
    }
}
