//! Configurações › Aparência no desktop: tema, cor de destaque e opções do Liquid Glass (UX-007).

use super::{Ctx, toggle};
use crate::desk_theme::{self as dt, ACCENTS, GLASS, THEMES};
use crate::model::{ChoiceCard, PanelRow};
use crate::shell::keyboard::KeyboardLayer;
use crate::shell::layer::{Flow, MenuItem};
use tako_core::presentation::Rgb;

const T: &str = "settings.appearance.";
const TT: &str = "settings.theme.";

/// Configurações › Tema (TelaTema): os cinco temas em cartões, a cor de destaque (fora do TAKO), as opções do tema
/// escolhido e a prévia.
pub(super) fn rows(ctx: &Ctx) -> Vec<MenuItem> {
    let theme = dt::theme(&ctx.settings);
    let header = |key: &str| MenuItem::new(PanelRow::header(ctx.t(&format!("{TT}{key}"))));
    let mut rows = vec![header("themes_header"), themes_row(ctx, theme)];
    // No TAKO o destaque vem da arte (ou é a pedra); a cor escolhida só vale nos outros temas.
    if theme != 0 {
        rows.extend([header("accent_header"), accents_row(ctx)]);
    }
    if theme == 0 {
        rows.extend([
            MenuItem::new(PanelRow::header(ctx.t(&format!("{T}desk_theme_tako")))),
            toggle(ctx, "settings.appearance.art_colors", dt::ART_COLORS, true),
        ]);
    } else if theme == GLASS {
        rows.extend([
            MenuItem::new(PanelRow::header(ctx.t(&format!("{T}desk_theme_glass")))),
            toggle(
                ctx,
                "settings.appearance.glass_ambient",
                dt::GLASS_AMBIENT,
                true,
            ),
            slider_row(ctx, "glass-opacity", "glass_opacity", dt::GLASS_OPACITY, 28),
            toggle(ctx, "settings.appearance.glass_pill", dt::GLASS_PILL, true),
        ]);
    }
    rows.extend([
        header("preview_header"),
        MenuItem::new(
            PanelRow::new(
                "desk-preview",
                "preview",
                ctx.t(&format!("{TT}preview_status")),
            )
            .value(ctx.t("game.play")),
        ),
    ]);
    rows
}

/// Os cinco cartões; ←/→ andam entre eles sem dar a volta.
fn themes_row(ctx: &Ctx, current: usize) -> MenuItem {
    let cards = THEMES
        .iter()
        .enumerate()
        .map(|(i, (_, id))| ChoiceCard {
            title: ctx.t(&format!("{T}desk_theme_{id}")),
            detail: ctx.t(&format!("{TT}desc_{id}")),
            on: i == current,
            ..Default::default()
        })
        .collect();
    let set = |ctx: &mut Ctx, i: usize| {
        ctx.settings.set(dt::THEME, THEMES[i].0);
        Flow::Stay
    };
    MenuItem::new(PanelRow::new(
        "desk-theme",
        "themes",
        ctx.t(&format!("{T}desk_theme")),
    ))
    .cards(cards, set)
    .on_adjust(move |ctx, direction| {
        let next = (dt::theme(&ctx.settings) as i32 + direction.signum())
            .clamp(0, THEMES.len() as i32 - 1);
        set(ctx, next as usize)
    })
}

/// As nove cores, a cor própria (o código num campo) e o código da atual; ←/→ andam pelas nove.
fn accents_row(ctx: &Ctx) -> MenuItem {
    let now = dt::accent(&ctx.settings);
    let preset = ACCENTS.iter().position(|(c, _)| *c == now);
    let cards = ACCENTS
        .iter()
        .enumerate()
        .map(|(i, (c, _))| ChoiceCard {
            title: accent_name(ctx, i),
            color: slint::Color::from_rgb_u8(c.r, c.g, c.b),
            on: preset == Some(i),
            ..Default::default()
        })
        .collect();
    let set = |ctx: &mut Ctx, i: usize| {
        ctx.settings.set(dt::ACCENT, &ACCENTS[i].0.to_string());
        Flow::Stay
    };
    let label = preset.map_or_else(|| ctx.t(&format!("{TT}custom")), |i| accent_name(ctx, i));
    MenuItem::new(PanelRow {
        // Cor própria em uso: o anel vai para a bolinha dela.
        on: preset.is_none(),
        ..PanelRow::new("desk-accent", "accents", ctx.t(&format!("{T}desk_accent")))
            .value(now.to_string())
            .desc(label)
    })
    .on(custom_accent)
    .cards(cards, set)
    .on_adjust(move |ctx, direction| {
        let at = ACCENTS
            .iter()
            .position(|(c, _)| *c == dt::accent(&ctx.settings));
        let next = at.map_or(0, |i| {
            (i as i32 + direction.signum()).clamp(0, ACCENTS.len() as i32 - 1) as usize
        });
        set(ctx, next)
    })
}

fn accent_name(ctx: &Ctx, i: usize) -> String {
    ctx.t(&format!("{T}desk_accent_{}", ACCENTS[i].1))
}

/// "Cor própria": o código da cor num campo de texto; só aceita #RRGGBB.
fn custom_accent(ctx: &mut Ctx) -> Flow {
    let now = dt::accent(&ctx.settings).to_string();
    Flow::Push(Box::new(
        KeyboardLayer::new(
            ctx,
            ctx.t(&format!("{T}desk_accent_custom")),
            &now,
            |ctx, text| {
                if let Some(color) = Rgb::parse(text.trim()) {
                    ctx.settings.set(dt::ACCENT, &color.to_string());
                }
                Flow::Close
            },
        )
        .hint(ctx.t(&format!("{T}desk_accent_custom_hint")))
        .check(|ctx, text| {
            Rgb::parse(text.trim())
                .is_none()
                .then(|| ctx.t(&format!("{T}desk_accent_invalid")))
        }),
    ))
}

/// Deslizante de 0 a 100 que o `desk_theme` converte para a faixa do design (opacidade 2–20%).
fn slider_row(ctx: &Ctx, id: &str, text: &str, key: &'static str, default: i32) -> MenuItem {
    let value = dt::slider(&ctx.settings, key, default);
    MenuItem::new(
        PanelRow {
            kind: "slider".into(),
            ..PanelRow::choice(id, ctx.t(&format!("{T}{text}")), value.to_string())
        }
        .desc(ctx.t(&format!("{T}{text}_desc"))),
    )
    .on_slide(move |ctx, v, _done| {
        ctx.settings.set(key, &v.to_string());
        ctx.dirty = true;
        Flow::Stay
    })
    // ← → andam 5; Shift ← → continuam andando 10 pelo fluxo comum.
    .on_adjust(move |ctx, direction| {
        let next = (dt::slider(&ctx.settings, key, default) + direction.signum() * 5).clamp(0, 100);
        ctx.settings.set(key, &next.to_string());
        ctx.dirty = true;
        Flow::Stay
    })
}
