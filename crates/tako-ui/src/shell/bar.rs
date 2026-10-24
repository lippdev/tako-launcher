//! Itens da barra de cima na ordem do teclado (←/→ e Tab na zona Bar). O Slint conta na mesma ordem (`focus-index`).

use super::{Ctx, Effect, Nav, activities, desktop};

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

/// Abas da barra (só a Biblioteca). O Slint soma `items.length` (as abas de `desk-nav`) ao índice do foco
/// (`first-right: 2 + items.length`), então `render_desktop_bar` publica exatamente esta quantidade; o teste da
/// barra falha se `ITEMS`, `TABS` e o Slint divergirem.
pub(super) const TABS: usize = 1;

pub(super) const ITEMS: [Item; 8] = [
    Item::Brand,
    Item::Library,
    Item::Downloads,
    Item::Profile,
    Item::BigPicture,
    Item::Minimize,
    Item::Maximize,
    Item::Close,
];

pub(super) fn item(index: usize) -> Option<Item> {
    ITEMS.get(index).copied()
}

/// Tab / Shift+Tab dentro da barra: o item vizinho; `None` no extremo (a página passa para a zona seguinte).
pub(super) fn step(index: usize, forward: bool) -> Option<usize> {
    let next = if forward {
        index + 1
    } else {
        index.checked_sub(1)?
    };
    (next < ITEMS.len()).then_some(next)
}

/// Enter no item. A Biblioteca é de cada página (a biblioteca vai para a lista, o perfil volta a ela).
pub(super) fn activate(item: Item, ctx: &mut Ctx) -> Nav {
    match item {
        Item::Brand => return Nav::Layer(Box::new(desktop::brand_menu())),
        Item::Profile => return Nav::Layer(Box::new(desktop::profile_menu())),
        Item::Downloads => activities::open_list(ctx),
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
    use std::time::Instant;

    #[test]
    fn the_slint_tabs_and_first_right_match_the_keyboard_items() {
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
        let library = ITEMS.iter().filter(|i| **i == Item::Library).count();
        assert_eq!((tabs.row_count(), library), (TABS, TABS));
        // `first-right` (perfil) é 2 (marca e downloads) + as abas.
        let profile = ITEMS.iter().position(|i| *i == Item::Profile);
        assert_eq!(profile, Some(2 + tabs.row_count()));
    }
}
