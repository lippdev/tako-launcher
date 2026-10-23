//! Tema do desktop (UX-007): TAKO, Escuro, Meia-noite, Preto OLED e Liquid Glass, a cor de destaque e as opções do
//! Liquid Glass. Tudo fica nas configurações; o Slint (`Ui.desk-theme` e cia.) escolhe as cores a partir daqui.

use crate::shell::ctx::Settings;
use slint_interpreter::{ComponentInstance, Value};
use tako_core::presentation::Rgb;

pub const THEME: &str = "DesktopTheme";
pub const ACCENT: &str = "DesktopAccent";
pub const GLASS_AMBIENT: &str = "DesktopGlassAmbient";
pub const GLASS_BLUR: &str = "DesktopGlassBlur";
pub const GLASS_OPACITY: &str = "DesktopGlassOpacity";
pub const GLASS_PILL: &str = "DesktopGlassPill";

/// Valor guardado e nome no catálogo; a posição é o `Ui.desk-theme`. Sem valor guardado, vale o TAKO.
pub const THEMES: [(&str, &str); 5] = [
    ("tako", "tako"),
    ("dark", "dark"),
    ("midnight", "midnight"),
    ("oled", "oled"),
    ("glass", "glass"),
];
pub const GLASS: usize = 4;

/// Cor e nome no catálogo (`desk_accent_<nome>`).
pub const ACCENTS: [(Rgb, &str); 9] = [
    (STONE, "stone"),
    (Rgb::new(0xff, 0x8a, 0x3d), "orange"),
    (Rgb::new(0xff, 0xc9, 0x4d), "gold"),
    (Rgb::new(0x3d, 0xdc, 0x84), "green"),
    (Rgb::new(0x3d, 0xd6, 0xe0), "cyan"),
    (Rgb::new(0x4d, 0x9b, 0xff), "blue"),
    (Rgb::new(0x9b, 0x7b, 0xff), "violet"),
    (Rgb::new(0xff, 0x5d, 0x9e), "pink"),
    (Rgb::new(0xe8, 0xe8, 0xe8), "white"),
];
/// A pedra do logo: destaque do TAKO sem arte (ou com "Cores da arte do jogo" desligado).
pub const STONE: Rgb = Rgb::new(0xd8, 0xc8, 0xbc);

pub fn theme(settings: &Settings) -> usize {
    let saved = settings.get(THEME).unwrap_or_default();
    THEMES.iter().position(|(v, _)| *v == saved).unwrap_or(0)
}

/// A escolhida, ou o padrão do tema: pedra no TAKO e azul nos outros.
pub fn accent(settings: &Settings) -> Rgb {
    settings
        .get(ACCENT)
        .and_then(Rgb::parse)
        .unwrap_or(if theme(settings) == 0 {
            STONE
        } else {
            ACCENTS[5].0
        })
}

/// Posição (0–100) de um deslizante do Liquid Glass; fora da faixa ou ilegível vale `default`.
pub fn slider(settings: &Settings, key: &str, default: i32) -> i32 {
    settings
        .get(key)
        .and_then(|v| v.parse().ok())
        .filter(|v| (0..=100).contains(v))
        .unwrap_or(default)
}

/// Desfoque do vidro: 12–60 px.
fn blur(position: i32) -> f64 {
    f64::from(12 + position * 48 / 100)
}

/// Opacidade das superfícies do vidro: 2–20%.
fn opacity(position: i32) -> f64 {
    f64::from(2 + position * 18 / 100) / 100.0
}

/// Entrega ao Slint o que o tema escolhe; o `Ui` calcula as cores.
pub fn apply(ui: &ComponentInstance, settings: &Settings) {
    let set = |name, value| crate::bridge::set_global(ui, "Ui", name, value);
    set("desk-theme", Value::Number(theme(settings) as f64));
    let c = accent(settings);
    set(
        "desk-accent",
        slint::Color::from_rgb_u8(c.r, c.g, c.b).into(),
    );
    set(
        "glass-blur",
        Value::Number(blur(slider(settings, GLASS_BLUR, 34))),
    );
    set(
        "glass-opacity",
        Value::Number(opacity(slider(settings, GLASS_OPACITY, 28))),
    );
    set("glass-pill", Value::Bool(settings.bool(GLASS_PILL, true)));
    set(
        "glass-ambient",
        Value::Bool(settings.bool(GLASS_AMBIENT, true)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(pairs: &[(&str, &str)]) -> Settings {
        let mut s = Settings::new(None);
        for (k, v) in pairs {
            s.set(k, v);
        }
        s
    }

    #[test]
    fn tako_is_the_default_and_unknown_values_fall_back() {
        assert_eq!(theme(&settings(&[])), 0);
        assert_eq!(theme(&settings(&[(THEME, "glass")])), GLASS);
        assert_eq!(theme(&settings(&[(THEME, "lixo")])), 0);
    }

    #[test]
    fn accent_defaults_follow_the_theme_and_ignore_broken_values() {
        assert_eq!(accent(&settings(&[])), STONE);
        assert_eq!(accent(&settings(&[(THEME, "oled")])), ACCENTS[5].0);
        assert_eq!(accent(&settings(&[(ACCENT, "#FF8A3D")])), ACCENTS[1].0);
        assert_eq!(accent(&settings(&[(ACCENT, "vermelho")])), STONE);
    }

    #[test]
    fn sliders_map_to_the_design_ranges() {
        assert_eq!((blur(0), blur(34), blur(100)), (12.0, 28.0, 60.0));
        assert_eq!((opacity(0), opacity(100)), (0.02, 0.2));
        assert_eq!(
            slider(&settings(&[(GLASS_BLUR, "250")]), GLASS_BLUR, 34),
            34
        );
    }
}
