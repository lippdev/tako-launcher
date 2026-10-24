//! Configurações › Aparência no desktop: tema, cor de destaque e opções do Liquid Glass (UX-007).

use super::{Ctx, toggle};
use crate::desk_theme::{self as dt, ACCENTS, GLASS, THEMES};
use crate::model::PanelRow;
use crate::shell::keyboard::KeyboardLayer;
use crate::shell::layer::{Flow, MenuItem, Pick};
use tako_core::presentation::Rgb;

const T: &str = "settings.appearance.";

pub(super) fn rows(ctx: &Ctx) -> Vec<MenuItem> {
    let theme = dt::theme(&ctx.settings);
    let mut rows = vec![
        MenuItem::new(PanelRow::header(ctx.t(&format!("{T}desk_header")))),
        theme_row(ctx, theme),
    ];
    // No TAKO o destaque vem da arte (ou é a pedra); a cor escolhida só vale nos outros temas.
    if theme != 0 {
        rows.push(accent_row(ctx));
        rows.push(
            MenuItem::new(
                PanelRow::action(
                    "desk-accent-custom",
                    ctx.t(&format!("{T}desk_accent_custom")),
                )
                .value(dt::accent(&ctx.settings).to_string()),
            )
            .on(custom_accent),
        );
    } else {
        rows.push(toggle(
            ctx,
            "settings.appearance.art_colors",
            dt::ART_COLORS,
            true,
        ));
    }
    if theme == GLASS {
        rows.extend([
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
    rows
}

fn theme_row(ctx: &Ctx, current: usize) -> MenuItem {
    let name = |ctx: &Ctx, i: usize| ctx.t(&format!("{T}desk_theme_{}", THEMES[i].1));
    MenuItem::new(
        PanelRow::choice(
            "desk-theme",
            ctx.t(&format!("{T}desk_theme")),
            name(ctx, current),
        )
        .desc(ctx.t(&format!("{T}desk_theme_desc"))),
    )
    .options(move |ctx| {
        let picks = (0..THEMES.len())
            .map(|i| {
                Pick::new(name(ctx, i)).set(move |ctx| {
                    ctx.settings.set(dt::THEME, THEMES[i].0);
                    Flow::Stay
                })
            })
            .collect();
        (picks, dt::theme(&ctx.settings))
    })
}

fn accent_row(ctx: &Ctx) -> MenuItem {
    let now = dt::accent(&ctx.settings);
    let preset = ACCENTS.iter().position(|(c, _)| *c == now);
    let label = preset.map_or_else(|| now.to_string(), |i| accent_name(ctx, i));
    MenuItem::new(
        PanelRow::choice("desk-accent", ctx.t(&format!("{T}desk_accent")), label)
            .desc(ctx.t(&format!("{T}desk_accent_desc"))),
    )
    .options(move |ctx| {
        let mut picks: Vec<Pick> = ACCENTS
            .iter()
            .enumerate()
            .map(|(i, (color, _))| {
                let hex = color.to_string();
                Pick::new(accent_name(ctx, i)).set(move |ctx| {
                    ctx.settings.set(dt::ACCENT, &hex);
                    Flow::Stay
                })
            })
            .collect();
        // Uma cor própria já escolhida aparece como a última opção, marcada.
        if preset.is_none() {
            picks.push(Pick::new(now.to_string()));
        }
        (picks, preset.unwrap_or(ACCENTS.len()))
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
        let next =
            (dt::slider(&ctx.settings, key, default) + direction.signum() * 5).clamp(0, 100);
        ctx.settings.set(key, &next.to_string());
        ctx.dirty = true;
        Flow::Stay
    })
}
