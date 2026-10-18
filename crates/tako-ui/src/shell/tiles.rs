//! Capas e textos de jogo para o tema (o `GameItemViewModel` do WPF).

use super::Ctx;
use crate::images;
use crate::model::Tile;
use crate::values::parse_color;
use tako_core::library::Game;
use tako_core::presentation::game_text;

/// Tamanho em que as capas são decodificadas (2:3, nítido até 1440p; o tema escala).
pub const COVER: (u32, u32) = (300, 450);
/// Arte de fundo: cobre a altura da janela, então decodifica na altura dela (ver `art_height`), nunca mais larga que
/// 3840. Um banner largo (3840 × 1240) cortado para 1920 de largura ficaria ampliado e borrado.
pub fn hero(ctx: &Ctx) -> (u32, u32) {
    (3840, ctx.art_height)
}

/// Altura em que a arte de fundo é decodificada para uma janela de `w` × `h` pixels físicos: a altura dela ou, numa
/// janela mais larga que 16:9, a altura de uma arte 16:9 que cobre a largura. Em degraus, para redimensionar a
/// janela não decodificar a arte de novo a cada pixel; 2160 (4K) no máximo, como antes.
pub fn art_height(w: u32, h: u32) -> u32 {
    let need = h.max(w * 9 / 16);
    [1080, 1440, 2160]
        .into_iter()
        .find(|b| *b >= need)
        .unwrap_or(2160)
}
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

/// `cover: false` não pede a capa (tile fora da área que o tema desenha).
pub fn tile(ctx: &Ctx, g: &Game, cover: bool) -> Tile {
    Tile {
        id: g.id.clone(),
        title: g.title.clone(),
        meta: meta(ctx, g),
        cover: image(g.shown_cover().filter(|_| cover), COVER),
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

#[cfg(test)]
mod tests {
    #[test]
    fn hero_is_decoded_at_the_window_height_in_steps() {
        use super::art_height;
        // 1080p e menores: 1080 (um banner 3840 × 1240 vira 3345 × 1080, sem ampliar).
        assert_eq!(art_height(1600, 900), 1080);
        assert_eq!(art_height(1920, 1080), 1080);
        assert_eq!(art_height(2560, 1440), 1440);
        // Ultrawide: a arte 16:9 precisa cobrir a largura.
        assert_eq!(art_height(3440, 1440), 2160);
        assert_eq!(art_height(2560, 1080), 1440);
        // Retina / 4K: o teto de antes.
        assert_eq!(art_height(3200, 1800), 2160);
        assert_eq!(art_height(7680, 4320), 2160);
    }
}
