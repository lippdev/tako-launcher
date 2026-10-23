//! Tema do desktop (UX-007): TAKO, Escuro, Meia-noite, Preto OLED e Liquid Glass, a cor de destaque e as opções do
//! Liquid Glass. Tudo fica nas configurações; o Slint (`Ui.desk-theme` e cia.) escolhe as cores a partir daqui.

use crate::shell::ctx::Settings;
use slint_interpreter::{ComponentInstance, Value};
use tako_core::presentation::Rgb;

pub const THEME: &str = "DesktopTheme";
pub const ACCENT: &str = "DesktopAccent";
/// "Cores da arte do jogo" (só no TAKO, ligado por padrão); a "Cor dinâmica" da tela cheia é outro ajuste.
pub const ART_COLORS: &str = crate::shell::ctx::keys::DESKTOP_ART_COLORS;
pub const GLASS_AMBIENT: &str = "DesktopGlassAmbient";
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

/// Opacidade das superfícies do vidro: 2–20%.
fn opacity(position: i32) -> f64 {
    f64::from(2 + position * 18 / 100) / 100.0
}

/// Texto sobre o destaque: escuro (#14110f) quando ele é claro (luma > 0,6), branco nos outros. Luma perceptual
/// (0,299 R + 0,587 G + 0,114 B) como nas prévias: pela luminância WCAG a pedra daria texto branco a 1,7:1.
fn on_accent(accent: Rgb) -> Rgb {
    let luma =
        (0.299 * f64::from(accent.r) + 0.587 * f64::from(accent.g) + 0.114 * f64::from(accent.b))
            / 255.0;
    if luma > 0.6 {
        Rgb::new(0x14, 0x11, 0x0f)
    } else {
        Rgb::WHITE
    }
}

/// O destaque que vale na tela. No TAKO é a cor da arte do jogo (com "Cores da arte do jogo" ligado e arte com cor)
/// ou a pedra do logo; nos outros temas, a escolhida.
pub fn effective_accent(settings: &Settings, art: Option<Rgb>) -> Rgb {
    if theme(settings) != 0 {
        return accent(settings);
    }
    art.filter(|_| settings.bool(ART_COLORS, true))
        .unwrap_or(STONE)
}

/// Entrega ao Slint o que o tema escolhe; o `Ui` calcula as cores. `art` é a cor da arte do jogo na tela.
pub fn apply(ui: &ComponentInstance, settings: &Settings, art: Option<Rgb>) {
    let set = |name, value| crate::bridge::set_global(ui, "Ui", name, value);
    set("desk-theme", Value::Number(theme(settings) as f64));
    let c = effective_accent(settings, art);
    set(
        "desk-accent",
        slint::Color::from_rgb_u8(c.r, c.g, c.b).into(),
    );
    let o = on_accent(c);
    set(
        "desk-on-accent",
        slint::Color::from_rgb_u8(o.r, o.g, o.b).into(),
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
    fn art_colors_only_apply_to_tako_when_on_and_fall_back_to_the_chosen_accent() {
        let blue = Rgb::new(0x51, 0x8b, 0xd6);
        let art = Some(blue);
        assert_eq!(effective_accent(&settings(&[]), art), blue);
        assert_eq!(effective_accent(&settings(&[]), None), STONE);
        assert_eq!(
            effective_accent(&settings(&[(ART_COLORS, "false")]), art),
            STONE
        );
        // No TAKO a cor escolhida não vale: é a da arte ou a pedra.
        let orange = (ACCENT, "#ff8a3d");
        assert_eq!(effective_accent(&settings(&[orange]), art), blue);
        assert_eq!(effective_accent(&settings(&[orange]), None), STONE);
        assert_eq!(
            effective_accent(&settings(&[orange, (ART_COLORS, "false")]), art),
            STONE
        );
        // A tela cheia desligar a "Cor dinâmica" não mexe nas cores da arte do desktop.
        assert_eq!(
            effective_accent(
                &settings(&[(crate::shell::ctx::keys::DYNAMIC_COLOR, "false")]),
                art
            ),
            blue
        );
        assert_eq!(
            effective_accent(&settings(&[(THEME, "dark")]), art),
            ACCENTS[5].0
        );
        assert_eq!(
            effective_accent(&settings(&[(THEME, "glass"), orange]), art),
            ACCENTS[1].0
        );
    }

    #[test]
    fn sliders_map_to_the_design_ranges() {
        assert_eq!((opacity(0), opacity(100)), (0.02, 0.2));
        assert_eq!(
            slider(&settings(&[(GLASS_OPACITY, "250")]), GLASS_OPACITY, 28),
            28
        );
    }

    #[test]
    fn light_accents_get_dark_text() {
        let dark = Rgb::new(0x14, 0x11, 0x0f);
        assert_eq!(on_accent(STONE), dark); // luma 0,80
        assert_eq!(on_accent(ACCENTS[2].0), dark); // ouro
        assert_eq!(on_accent(ACCENTS[8].0), dark); // branco
        assert_eq!(on_accent(ACCENTS[1].0), dark); // laranja, luma 0,64
        assert_eq!(on_accent(ACCENTS[5].0), Rgb::WHITE); // azul
    }
}
