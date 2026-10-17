//! Configurações › Biblioteca e lojas: adicionar e procurar jogos, lojas ligadas e coleções.

use super::{Ctx, Effect, Flow, MenuItem, MenuLayer, action, effect};
use crate::model::PanelRow;
use crate::shell::ctx::keys;
use crate::shell::dialogs;
use crate::shell::keyboard::KeyboardLayer;
use std::sync::OnceLock;
use tako_core::i18n::tn;
use tako_core::library::{GameStore, collections};
use tako_core::presentation::game_text;
use tako_sources::report::LibrarySourceReport;

/// Lojas que este SO lê (montar as fontes a cada desenho seria caro e o resultado não muda).
fn stores() -> &'static [GameStore] {
    static S: OnceLock<Vec<GameStore>> = OnceLock::new();
    S.get_or_init(crate::sources::stores)
}

/// `DisabledStores` com a loja ligada ou desligada.
pub(crate) fn toggle_store(disabled: &str, store: GameStore) -> String {
    let off = crate::sources::is_disabled(disabled, store);
    let mut list: Vec<&str> = disabled
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case(store.key()))
        .collect();
    if !off {
        list.push(store.key());
    }
    list.join(",")
}

fn store_icon(store: GameStore) -> String {
    match store {
        GameStore::Heroic | GameStore::Lutris => String::new(),
        s => format!("store-{}", s.key().to_ascii_lowercase()),
    }
}

fn store_row(ctx: &Ctx, store: GameStore) -> MenuItem {
    let disabled = ctx.settings.string(keys::DISABLED_STORES);
    let on = !crate::sources::is_disabled(&disabled, store);
    let desc = if !on {
        LibrarySourceReport {
            store,
            enabled: false,
            game_count: 0,
            client_installed: false,
            notice: None,
        }
        .summary()
    } else {
        ctx.sources
            .iter()
            .find(|r| r.store == store && r.enabled)
            .map(LibrarySourceReport::summary)
            .unwrap_or_else(|| ctx.t("settings.library.store_unchecked"))
    };
    MenuItem::new(
        PanelRow::toggle(
            format!("store:{}", store.key()),
            game_text::store_name(ctx.lang(), store),
            on,
        )
        .desc(desc)
        .icon(&store_icon(store)),
    )
    .on(move |ctx| {
        let next = toggle_store(&ctx.settings.string(keys::DISABLED_STORES), store);
        ctx.settings.set(keys::DISABLED_STORES, &next);
        ctx.effect(Effect::ReloadLibrary);
        Flow::Stay
    })
}

/// Uma linha liga/desliga por loja deste SO (também no assistente da primeira execução).
pub(crate) fn store_rows(ctx: &Ctx) -> Vec<MenuItem> {
    stores().iter().map(|s| store_row(ctx, *s)).collect()
}

pub fn rows(ctx: &Ctx) -> Vec<MenuItem> {
    let mut rescan = action(ctx, "settings.library.rescan");
    if ctx.scanning {
        rescan.value = ctx.t("settings.library.rescan_busy");
    }
    let mut v = vec![
        MenuItem::new(action(ctx, "settings.library.add")).on(effect(Effect::AddGame)),
        MenuItem::new(rescan).on(effect(Effect::Rescan)),
        MenuItem::new(PanelRow::header(ctx.t("settings.library.stores_header"))),
    ];
    v.extend(store_rows(ctx));
    if cfg!(windows) {
        v.push(
            MenuItem::new(action(ctx, "settings.library.playnite"))
                .on(effect(Effect::ImportPlaynite)),
        );
    }
    v.push(MenuItem::new(PanelRow::header(
        ctx.t("settings.library.organize_header"),
    )));
    let n = ctx.collections.collections().len();
    let summary = if n == 0 {
        ctx.t("settings.library.collections_create")
    } else {
        tn("settings.library.collections_count", n as i64, &[])
    };
    v.push(
        MenuItem::new(
            PanelRow::action("collections", ctx.t("settings.library.collections"))
                .desc(ctx.t("settings.library.collections_desc"))
                .value(summary),
        )
        .on(|ctx| Flow::Push(Box::new(collections_panel(ctx)))),
    );
    v
}

pub(crate) fn collections_panel(ctx: &Ctx) -> MenuLayer {
    MenuLayer::panel(ctx.t("settings.library.collections"), |ctx| {
        let mut v: Vec<MenuItem> = ctx
            .collections
            .collections()
            .iter()
            .map(|c| {
                let id = c.id.clone();
                MenuItem::new(PanelRow::action(c.id.clone(), c.name.clone()).value(tn(
                    "common.games",
                    c.game_ids.len() as i64,
                    &[],
                )))
                .on(move |ctx| Flow::Push(Box::new(collection_actions(ctx, &id))))
            })
            .collect();
        v.push(
            MenuItem::new(PanelRow::action("new", ctx.t("collections.new")).icon("more"))
                .on(|ctx| Flow::Push(Box::new(dialogs::new_collection(ctx, None)))),
        );
        v
    })
}

fn collection_actions(ctx: &Ctx, id: &str) -> MenuLayer {
    let name = ctx
        .collections
        .find(id)
        .map(|c| c.name.clone())
        .unwrap_or_default();
    let id = id.to_owned();
    MenuLayer::dialog(name, move |ctx| {
        let (a, b) = (id.clone(), id.clone());
        vec![
            MenuItem::new(PanelRow::button("rename", ctx.t("settings.library.rename")))
                .on(move |ctx| Flow::Replace(Box::new(rename(ctx, &a)))),
            MenuItem::new(PanelRow::button("delete", ctx.t("settings.library.delete")).danger())
                .on(move |ctx| Flow::Replace(Box::new(confirm_delete(ctx, &b)))),
            MenuItem::new(PanelRow::button("back", ctx.t("hint.back"))).on(|_| Flow::Close),
        ]
    })
}

fn rename(ctx: &Ctx, id: &str) -> KeyboardLayer {
    let current = ctx
        .collections
        .find(id)
        .map(|c| c.name.clone())
        .unwrap_or_default();
    let (id, check_id) = (id.to_owned(), id.to_owned());
    KeyboardLayer::new(
        ctx,
        ctx.t("settings.library.rename"),
        &current,
        move |ctx, name| match ctx.collections.rename(&id, name) {
            Ok(()) => {
                ctx.sync_collections();
                Flow::Close
            }
            Err(e) => {
                ctx.toast(e.to_string());
                Flow::Stay
            }
        },
    )
    .hint(ctx.t("collections.name_rule"))
    .check(move |ctx, name| {
        collections::name_problem(name, ctx.collections.collections(), Some(&check_id))
            .map(|p| p.message(ctx.lang()))
    })
}

fn confirm_delete(ctx: &Ctx, id: &str) -> MenuLayer {
    let name = ctx
        .collections
        .find(id)
        .map(|c| c.name.clone())
        .unwrap_or_default();
    let id = id.to_owned();
    MenuLayer::dialog(
        ctx.tf("settings.library.delete_title", &[("name", &name)]),
        move |ctx| {
            let id = id.clone();
            vec![
                MenuItem::new(
                    PanelRow::button("delete", ctx.t("settings.library.delete")).danger(),
                )
                .on(move |ctx| {
                    if let Err(e) = ctx.collections.delete(&id) {
                        ctx.toast(e.to_string());
                    }
                    ctx.sync_collections();
                    Flow::Close
                }),
                MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel")))
                    .on(|_| Flow::Close),
            ]
        },
    )
    .text(ctx.t("settings.library.delete_text"))
}
