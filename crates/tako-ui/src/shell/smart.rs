//! Coleções automáticas (LIB-003) na interface: salvar os filtros da biblioteca, mostrar na barra lateral e
//! excluir em Configurações › Biblioteca.

use super::ctx::keys;
use super::keyboard::KeyboardLayer;
use super::layer::{Flow, MenuItem, MenuLayer};
use super::Ctx;
use crate::model::PanelRow;
use tako_core::library::smart::{self, SmartCollection};
use tako_core::library::{LibraryCriteria, LibraryFilter, query};

/// Rota da barra lateral: `library:smart:<id>`.
pub const ROUTE: &str = "library:smart:";

pub fn list(ctx: &Ctx) -> Vec<SmartCollection> {
    smart::parse(&ctx.settings.string(keys::SMART_COLLECTIONS))
}

fn save(ctx: &mut Ctx, list: &[SmartCollection]) {
    ctx.settings.set(keys::SMART_COLLECTIONS, &smart::to_json(list));
    ctx.dirty = true;
}

pub fn route_of(id: &str) -> String {
    format!("{ROUTE}{id}")
}

pub fn find(ctx: &Ctx, route: &str) -> Option<SmartCollection> {
    let id = route.strip_prefix(ROUTE)?;
    list(ctx).into_iter().find(|c| c.id == id)
}

/// Quantos jogos a coleção mostra agora.
pub fn count(ctx: &Ctx, c: &SmartCollection) -> usize {
    query::apply(ctx.library.games(), LibraryFilter::All, &c.criteria).len()
}

/// Nome da coleção pelo teclado virtual; salva os critérios de agora (a ordenação junto).
pub fn save_as(ctx: &Ctx, criteria: LibraryCriteria) -> KeyboardLayer {
    KeyboardLayer::new(ctx, ctx.t("smart.save"), "", move |ctx, name| {
        let mut all = list(ctx);
        match smart::create(&mut all, name, criteria.clone()) {
            Ok(_) => {
                save(ctx, &all);
                ctx.toast(ctx.t("smart.saved"));
                Flow::CloseAll
            }
            Err(p) => {
                ctx.toast(p.message(ctx.lang()));
                Flow::Stay
            }
        }
    })
    .hint(ctx.t("smart.name_rule"))
    .check(|ctx, name| smart::name_problem(name, &list(ctx), None).map(|p| p.message(ctx.lang())))
}

/// Configurações › Biblioteca › Coleções automáticas: cada uma com quantos jogos mostra; A pergunta se exclui.
pub fn panel(ctx: &Ctx) -> MenuLayer {
    MenuLayer::panel(ctx.t("smart.title"), |ctx| {
        let all = list(ctx);
        if all.is_empty() {
            return vec![MenuItem::new(PanelRow::info("empty", ctx.t("smart.empty"), ""))];
        }
        all.into_iter()
            .map(|c| {
                let n = count(ctx, &c);
                let (id, name) = (c.id.clone(), c.name.clone());
                MenuItem::new(
                    PanelRow::action(c.id, c.name)
                        .value(tako_core::i18n::tn("common.games", n as i64, &[])),
                )
                .on(move |ctx| Flow::Push(Box::new(confirm_delete(ctx, &id, &name))))
            })
            .collect()
    })
    .text(ctx.t("smart.text"))
}

fn confirm_delete(ctx: &Ctx, id: &str, name: &str) -> MenuLayer {
    let id = id.to_owned();
    MenuLayer::dialog(ctx.tf("smart.delete_title", &[("name", name)]), move |ctx| {
        let id = id.clone();
        vec![
            MenuItem::new(PanelRow::button("delete", ctx.t("settings.library.delete")).danger())
                .on(move |ctx| {
                    let mut all = list(ctx);
                    all.retain(|c| c.id != id);
                    save(ctx, &all);
                    Flow::Close
                }),
            MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel"))).on(|_| Flow::Close),
        ]
    })
    .text(ctx.t("smart.delete_text"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::Layer;
    use tako_core::paths::DataPaths;
    use tako_platform::Platform;

    #[test]
    fn saved_filters_become_a_sidebar_route_that_updates_itself() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = crate::app::build_ctx(DataPaths::at(dir.path()), Platform::simulated(), true);
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        ctx.library.refresh();
        let criteria = LibraryCriteria {
            favorites_only: true,
            ..Default::default()
        };
        // ☰ no teclado virtual confirma o nome.
        let mut kb = save_as(&ctx, criteria);
        kb.text("Favoritos do sofá", &mut ctx);
        assert!(matches!(
            kb.handle(crate::shell::Cmd::Menu, &mut ctx),
            Flow::CloseAll
        ));
        let saved = list(&ctx);
        assert_eq!(saved.len(), 1);
        let route = route_of(&saved[0].id);
        assert!(crate::shell::sidebar::items(&ctx).iter().any(|i| i.id == route));

        // A coleção acompanha a biblioteca: favoritar um jogo faz ele aparecer.
        let before = count(&ctx, &saved[0]);
        let id = ctx
            .library
            .games()
            .iter()
            .find(|g| !g.is_favorite)
            .map(|g| g.id.clone())
            .unwrap();
        ctx.library.toggle_favorite(&id).unwrap();
        assert_eq!(count(&ctx, &saved[0]), before + 1);

        let page = crate::shell::library::LibraryPage::open(&route, &ctx);
        assert!(page.criteria.borrow().favorites_only);

        let p = panel(&ctx);
        assert_eq!(p.render(&ctx).rows[0].key, saved[0].id);
        let del = confirm_delete(&ctx, &saved[0].id, &saved[0].name);
        let item = (del.build)(&ctx).into_iter().next().unwrap();
        (item.accept.unwrap())(&mut ctx);
        assert!(list(&ctx).is_empty());
        let gone = crate::shell::library::LibraryPage::open(&route, &ctx);
        assert_eq!(*gone.criteria.borrow(), LibraryCriteria::default());
    }
}
