//! Port de `Presentation/AccentPalette.cs` e `Presentation/ContrastRules.cs`: accent do jogo em foco e contraste
//! mínimo WCAG 2.x AA (UX-009).

use std::fmt;

/// Cor RGB de 8 bits por canal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const WHITE: Rgb = Rgb::new(255, 255, 255);

    pub const fn new(r: u8, g: u8, b: u8) -> Rgb {
        Rgb { r, g, b }
    }

    /// "#RRGGBB" ou "RRGGBB"; `None` se inválida (o accent vem de metadados).
    pub fn parse(hex: &str) -> Option<Rgb> {
        let v = hex.trim_start_matches('#');
        if v.len() != 6 || !v.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let n = u32::from_str_radix(v, 16).ok()?;
        Some(Rgb::new((n >> 16) as u8, (n >> 8) as u8, n as u8))
    }

    /// Luminância relativa (WCAG).
    pub fn luminance(self) -> f64 {
        fn channel(c: u8) -> f64 {
            let v = f64::from(c) / 255.0;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * channel(self.r) + 0.7152 * channel(self.g) + 0.0722 * channel(self.b)
    }

    pub fn scale(self, factor: f64) -> Rgb {
        let c = |v: u8| round(f64::from(v) * factor);
        Rgb::new(c(self.r), c(self.g), c(self.b))
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

/// `Math.Round` do C# (meio para o par) com limite de canal.
fn round(v: f64) -> u8 {
    v.clamp(0.0, 255.0).round_ties_even() as u8
}

/// Razão de contraste WCAG entre duas cores.
pub fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (a.luminance() + 0.05, b.luminance() + 0.05);
    la.max(lb) / la.min(lb)
}

/// Accent do jogo em foco e seu tom profundo (DESIGN.md › Colors).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccentPalette {
    pub accent: Rgb,
    pub deep: Rgb,
}

impl AccentPalette {
    pub const FALLBACK: Rgb = Rgb::new(0x2B, 0x3A, 0x55);

    /// Escurece o accent até garantir contraste 4.5:1 com texto branco; o tom profundo é 55% dele.
    pub fn for_accent(accent_hex: Option<&str>, dynamic_color: bool) -> AccentPalette {
        Self::for_accent_with_fallback(accent_hex, dynamic_color, None)
    }

    /// O mesmo que [`for_accent`], mas aceita uma cor de fallback (ex.: cor do tema ou sobreposição).
    pub fn for_accent_with_fallback(
        accent_hex: Option<&str>,
        dynamic_color: bool,
        fallback: Option<Rgb>,
    ) -> AccentPalette {
        let mut accent = accent_hex
            .filter(|_| dynamic_color)
            .and_then(Rgb::parse)
            .or(fallback)
            .unwrap_or(Self::FALLBACK);
        while accent.luminance() > 0.18 {
            accent = accent.scale(0.92);
        }
        AccentPalette {
            accent,
            deep: accent.scale(0.55),
        }
    }
}

/// Mínimos WCAG AA: texto 4.5:1, texto grande 3:1 e componentes (chave, marcador, ícone, contorno) 3:1.
pub mod contrast_rules {
    use super::{Rgb, round};

    pub const TEXT: f64 = 4.5;
    pub const LARGE_TEXT: f64 = 3.0;
    pub const COMPONENT: f64 = 3.0;

    /// Mínimo para um texto: grande é 24px ou mais, ou 18.66px em negrito (peso 700+), no palco de 1080.
    pub fn for_text(font_size: f64, font_weight: u16) -> f64 {
        if font_size >= 24.0 || (font_size >= 18.66 && font_weight >= 700) {
            LARGE_TEXT
        } else {
            TEXT
        }
    }

    pub fn ratio(a: Rgb, b: Rgb) -> f64 {
        super::contrast(a, b)
    }

    /// Cor com transparência (`alpha` de 0 a 1) desenhada sobre um fundo opaco.
    pub fn over(color: Rgb, alpha: f64, background: Rgb) -> Rgb {
        let a = alpha.clamp(0.0, 1.0);
        let mix = |top: u8, bottom: u8| round(a * f64::from(top) + (1.0 - a) * f64::from(bottom));
        Rgb::new(
            mix(color.r, background.r),
            mix(color.g, background.g),
            mix(color.b, background.b),
        )
    }

    /// Lê "#AARRGGBB" ou "#RRGGBB" (alpha 1).
    pub fn parse_argb(hex: &str) -> Option<(Rgb, f64)> {
        let v = hex.trim_start_matches('#');
        match v.len() {
            6 => Some((Rgb::parse(v)?, 1.0)),
            8 if v.is_char_boundary(2) => {
                let alpha = u8::from_str_radix(&v[..2], 16)
                    .ok()
                    .filter(|_| v[..2].bytes().all(|b| b.is_ascii_hexdigit()))?;
                Some((Rgb::parse(&v[2..])?, f64::from(alpha) / 255.0))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::contrast_rules::*;
    use super::*;
    use crate::library::sample;

    #[test]
    fn every_sample_accent_keeps_white_text_readable() {
        for game in sample::games() {
            let palette = AccentPalette::for_accent(Some(&game.accent), true);
            assert!(contrast(palette.accent, Rgb::WHITE) >= 4.5, "{}", game.id);
        }
    }

    #[test]
    fn disabling_dynamic_color_uses_the_slate_fallback() {
        assert_eq!(
            AccentPalette::for_accent(Some("#C8102E"), false).accent,
            AccentPalette::FALLBACK
        );
        assert_eq!(
            AccentPalette::for_accent(Some("lixo"), true).accent,
            AccentPalette::FALLBACK
        );
    }

    #[test]
    fn rgb_parses_hex() {
        assert_eq!(Rgb::parse("#C8102E"), Some(Rgb::new(200, 16, 46)));
        assert_eq!(Rgb::parse("2B3A55"), Some(Rgb::new(43, 58, 85)));
        assert_eq!(Rgb::parse("+12345"), None);
        assert_eq!(Rgb::new(43, 58, 85).to_string(), "#2B3A55");
    }

    // ---- FocusContrastTests: tokens do tema padrão do C# (Themes/Default/Tokens.xaml).

    fn token(key: &str) -> (Rgb, f64) {
        let hex = match key {
            "CanvasColor" => "#FF0E1116",
            "OnAccentBrush" => "#FFFFFFFF",
            "OnAccentMutedBrush" => "#B8FFFFFF",
            "SurfaceSolidBrush" => "#FFFFFFFF",
            "OnSurfaceSolidBrush" => "#FF11141A",
            "PanelStrongBrush" => "#FA12151C",
            "SuccessBrush" => "#FF3DDC84",
            "ControlKnobBrush" => "#FFFFFFFF",
            "ControlTrackOffBrush" => "#40FFFFFF",
            "ControlOffBrush" => "#8CFFFFFF",
            "OnSurfaceSolidMutedBrush" => "#FF565C68",
            "ControlOnOnSolidBrush" => "#FF0B7A3B",
            "ControlOffOnSolidBrush" => "#FF5B6170",
            _ => panic!("token {key}"),
        };
        parse_argb(hex).unwrap()
    }

    fn on(key: &str, background: Rgb) -> Rgb {
        let (c, a) = token(key);
        over(c, a, background)
    }

    fn solid() -> Rgb {
        token("SurfaceSolidBrush").0
    }

    fn panel() -> Rgb {
        let (c, a) = token("PanelStrongBrush");
        over(c, a, token("CanvasColor").0)
    }

    #[test]
    fn controls_on_the_focused_row_contrast_with_the_white_surface() {
        for (key, min) in [
            ("OnSurfaceSolidBrush", TEXT),
            ("OnSurfaceSolidMutedBrush", TEXT),
            ("ControlOnOnSolidBrush", TEXT),
            ("ControlOffOnSolidBrush", COMPONENT),
        ] {
            assert!(ratio(on(key, solid()), solid()) >= min, "{key}");
            assert_ne!(on(key, solid()), solid(), "{key}"); // nada some na linha branca
        }
        let track = on("ControlOnOnSolidBrush", solid());
        assert!(ratio(on("ControlKnobBrush", track), track) >= COMPONENT);
    }

    #[test]
    fn controls_on_an_unfocused_row_contrast_with_the_panel() {
        for (key, min) in [
            ("OnAccentBrush", TEXT),
            ("OnAccentMutedBrush", TEXT),
            ("SuccessBrush", TEXT),
            ("ControlOffBrush", COMPONENT),
        ] {
            assert!(ratio(on(key, panel()), panel()) >= min, "{key}");
        }
        let track = on("ControlTrackOffBrush", panel());
        assert!(ratio(on("ControlKnobBrush", track), track) >= COMPONENT);
    }

    #[test]
    fn large_text_follows_the_wcag_size_rule() {
        assert_eq!(for_text(18.0, 400), TEXT);
        assert_eq!(for_text(20.0, 600), TEXT);
        assert_eq!(for_text(20.0, 700), LARGE_TEXT);
        assert_eq!(for_text(24.0, 400), LARGE_TEXT);
    }

    #[test]
    fn translucent_colors_are_blended_over_the_background() {
        let (color, alpha) = parse_argb("#8011141A").unwrap();
        assert_eq!(color, Rgb::new(0x11, 0x14, 0x1A));
        assert_eq!(over(color, alpha, Rgb::WHITE), Rgb::new(0x88, 0x89, 0x8C));
        assert_eq!(parse_argb("#FFFFFF"), Some((Rgb::WHITE, 1.0)));
        assert_eq!(parse_argb("#12"), None);
        assert_eq!(parse_argb("#ççFFFFFF"), None);
    }
}
