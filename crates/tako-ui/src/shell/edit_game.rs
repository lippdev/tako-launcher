//! Editar jogo adicionado (nome, argumentos, pasta, fim do jogo).

use super::Ctx;
use super::layer::{Flow, Layer, MenuItem, MenuLayer};
use crate::model::PanelRow;

pub fn panel(ctx: &Ctx, _id: &str) -> Box<dyn Layer> {
    Box::new(MenuLayer::dialog(ctx.t("common.soon"), |ctx| {
        vec![MenuItem::new(PanelRow::button("ok", ctx.t("common.ok"))).on(|_| Flow::Close)]
    }))
}
