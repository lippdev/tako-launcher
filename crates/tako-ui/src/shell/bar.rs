//! Itens da barra de cima na ordem do teclado (←/→ na zona Bar). O Slint conta na mesma ordem (`focus-index`).

use super::{Ctx, Effect, Nav, activities, desktop};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Item {
    Brand,
    Library,
    Downloads,
    Profile,
    BigPicture,
    Minimize,
    Maximize,
    Close,
}

/// Abas da barra (só a Biblioteca). O Slint soma `items.length` (as abas de `desk-nav`) ao índice do foco, então
/// `render_desktop_bar` publica exatamente esta quantidade; o teste da barra falha se divergirem.
pub(super) const TABS: usize = 1;

/// Os itens agora: a pílula de atividades só entra com algo na fila.
pub(super) fn items(ctx: &Ctx) -> Vec<Item> {
    let mut v = vec![Item::Brand];
    v.extend([Item::Library; TABS]);
    if activities::visible(ctx, Instant::now()) {
        v.push(Item::Downloads);
    }
    v.extend([
        Item::Profile,
        Item::BigPicture,
        Item::Minimize,
        Item::Maximize,
        Item::Close,
    ]);
    v
}

pub(super) fn item(ctx: &Ctx, index: usize) -> Option<Item> {
    items(ctx).get(index).copied()
}

/// Tab / Shift+Tab dentro da barra: o item vizinho; `None` no extremo (a página passa para a zona seguinte).
pub(super) fn step(ctx: &Ctx, index: usize, forward: bool) -> Option<usize> {
    let next = if forward {
        index + 1
    } else {
        index.checked_sub(1)?
    };
    (next < items(ctx).len()).then_some(next)
}

/// Enter no item. A Biblioteca é de cada página (a biblioteca vai para a lista, o perfil volta a ela).
pub(super) fn activate(item: Item, ctx: &mut Ctx) -> Nav {
    match item {
        Item::Brand => return Nav::Layer(Box::new(desktop::brand_menu())),
        Item::Profile => return Nav::Layer(Box::new(desktop::profile_menu())),
        Item::Downloads => activities::accept(ctx, Instant::now()),
        Item::BigPicture => ctx.effect(Effect::SwitchMode(crate::mode::UiMode::BigPicture)),
        Item::Minimize => ctx.effect(Effect::Window("min")),
        Item::Maximize => ctx.effect(Effect::Window("max")),
        Item::Close => ctx.effect(Effect::Window("close")),
        Item::Library => {}
    }
    Nav::Stay
}

#[cfg(test)]
mod tests {
    use super::*;
    use slint::Model;

    #[test]
    fn the_slint_tabs_match_the_keyboard_items() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let ctx = crate::app::build_ctx(
            tako_core::paths::DataPaths::at(dir.path()),
            tako_platform::Platform::simulated(),
            true,
        );
        let mut shell = super::super::Shell::new(ctx);
        shell.enter_desktop();
        let out = shell.render(Instant::now());
        let slint_interpreter::Value::Model(tabs) =
            &out.props.iter().find(|(n, _)| *n == "desk-nav").unwrap().1
        else {
            panic!("desk-nav não é uma lista");
        };
        let keyboard = items(&shell.ctx)
            .iter()
            .filter(|i| **i == Item::Library)
            .count();
        assert_eq!((tabs.row_count(), keyboard), (TABS, TABS));
    }
}
