//! Capas e textos de jogo para o tema (o `GameItemViewModel` do WPF).

use super::Ctx;
use crate::images;
use crate::model::Tile;
use crate::values::parse_color;
use tako_core::library::Game;
use tako_core::presentation::game_text;

/// Tamanho em que as capas são decodificadas (2:3, nítido até 1440p; o tema escala).
pub const COVER: (u32, u32) = (300, 450);
pub const HERO: (u32, u32) = (1920, 1080);
pub const LOGO: (u32, u32) = (900, 360);

pub fn image(uri: Option<&str>, size: (u32, u32)) -> slint::Image {
    uri.and_then(|u| images::get(u, size.0, size.1))
        .unwrap_or_default()
}

pub fn meta(ctx: &Ctx, g: &Game) -> String {
    let lang = ctx.lang();
    let mut parts = vec![game_text::store_name(lang, g.store())];
    if let Some(size) = &g.size_label {
        parts.push(size.clone());
    } else if !g.is_installed {
        parts.push(ctx.t("game.not_installed"));
    }
    parts.join(" · ")
}

pub fn tile(ctx: &Ctx, g: &Game) -> Tile {
    Tile {
        id: g.id.clone(),
        title: g.title.clone(),
        meta: meta(ctx, g),
        cover: image(g.shown_cover(), COVER),
        accent: parse_color(&g.accent),
        installed: g.is_installed,
        favorite: g.is_favorite,
        running: ctx.running.as_ref().is_some_and(|r| r.id == g.id),
    }
}

/// "Jogar", "Voltar ao jogo", "Instalar via loja"... (botão principal do destaque e dos detalhes).
pub fn play_label(ctx: &Ctx, g: &Game) -> String {
    if ctx.running.as_ref().is_some_and(|r| r.id == g.id) {
        ctx.t("game.resume")
    } else if g.is_upcoming() {
        ctx.t("game.upcoming")
    } else if g.is_rom() {
        ctx.t("game.rom_soon")
    } else if !g.is_installed {
        if g.store() == tako_core::library::GameStore::Manual {
            ctx.t("game.missing_file")
        } else {
            ctx.t("game.install_via_store")
        }
    } else {
        ctx.t("game.play")
    }
}
