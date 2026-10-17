//! Assistente da primeira execução (UX-003): idioma, lojas e provedores de metadados, em três diálogos.
//! Aparece uma vez numa instalação nova; tudo dá para mudar depois em Configurações.

use super::ctx::keys;
use super::layer::{Flow, MenuItem, MenuLayer};
use super::settings::{library, metadata};
use super::{Ctx, Shell};
use crate::model::PanelRow;
use tako_core::i18n::{self, Lang};

const STEPS: usize = 3;

/// Instalação nova: nunca concluiu o assistente, nada gravado nas configurações e biblioteca vazia. Quem atualiza
/// de uma alpha não vê o assistente; marca como concluído para ele não aparecer depois.
fn needed(ctx: &mut Ctx) -> bool {
    if ctx.settings.bool(keys::FIRST_RUN_DONE, false) {
        return false;
    }
    if !ctx.settings.is_empty() || !ctx.library.games().is_empty() {
        ctx.settings.set_bool(keys::FIRST_RUN_DONE, true);
        return false;
    }
    true
}

/// Abre o assistente se for a primeira vez (fora do `--simulate`, do modo seguro e da volta de um jogo).
pub fn open_if_needed(shell: &mut Shell) {
    if shell.ctx.simulated() || shell.ctx.safe_mode || shell.ctx.running.is_some() {
        return;
    }
    if needed(&mut shell.ctx) {
        let layer = welcome(&shell.ctx);
        shell.push_layer(Box::new(layer));
    }
}

fn finish(ctx: &mut Ctx) -> Flow {
    ctx.settings.set_bool(keys::FIRST_RUN_DONE, true);
    ctx.toast(ctx.t("first_run.done_toast"));
    Flow::Close
}

fn step(
    ctx: &Ctx,
    n: usize,
    title: &str,
    text: &str,
    build: impl Fn(&Ctx) -> Vec<MenuItem> + 'static,
) -> MenuLayer {
    MenuLayer::dialog(ctx.t(title), build)
        .overline(ctx.tf(
            "first_run.step",
            &[("n", &n.to_string()), ("total", &STEPS.to_string())],
        ))
        .text(ctx.t(text))
        .wide()
}

/// Passo 1: idioma. O diálogo é refeito ao trocar, para o próprio texto mudar de idioma.
pub fn welcome(ctx: &Ctx) -> MenuLayer {
    step(
        ctx,
        1,
        "first_run.welcome",
        "first_run.welcome_text",
        |ctx| {
            let mut v: Vec<MenuItem> = [Lang::PtBr, Lang::EnUs]
                .into_iter()
                .enumerate()
                .map(|(i, lang)| {
                    MenuItem::new(PanelRow::radio(
                        lang.code(),
                        super::settings::lang_name(lang),
                        ctx.lang() == lang,
                    ))
                    .on(move |ctx| {
                        ctx.settings.set(keys::LANGUAGE, lang.code());
                        i18n::set_lang(lang);
                        ctx.dirty = true;
                        Flow::Replace(Box::new(welcome(ctx).focus_at(i)))
                    })
                })
                .collect();
            v.push(
                MenuItem::new(PanelRow::button("next", ctx.t("first_run.next")))
                    .on(|ctx| Flow::Replace(Box::new(stores(ctx)))),
            );
            v.push(MenuItem::new(PanelRow::button("skip", ctx.t("first_run.skip"))).on(finish));
            v
        },
    )
    .focus_at(2)
    .on_back(finish)
}

/// Passo 2: lojas que este sistema lê, com o que a leitura achou em cada uma.
fn stores(ctx: &Ctx) -> MenuLayer {
    step(ctx, 2, "first_run.stores", "first_run.stores_text", |ctx| {
        let mut v = library::store_rows(ctx);
        v.push(
            MenuItem::new(PanelRow::button("next", ctx.t("first_run.next")))
                .on(|ctx| Flow::Replace(Box::new(providers(ctx)))),
        );
        v
    })
    // Foco em "Continuar": A seguido não desliga a primeira loja sem querer.
    .focus_at(library::store_rows(ctx).len())
    .on_back(|ctx| Flow::Replace(Box::new(welcome(ctx))))
}

/// Passo 3: provedores de capas e descrições (os mesmos painéis de Configurações).
fn providers(ctx: &Ctx) -> MenuLayer {
    step(
        ctx,
        3,
        "first_run.metadata",
        "first_run.metadata_text",
        |ctx| {
            let mut v = metadata::provider_rows(ctx);
            v.push(MenuItem::new(PanelRow::button("done", ctx.t("first_run.finish"))).on(finish));
            v
        },
    )
    .focus_at(metadata::provider_rows(ctx).len())
    .on_back(|ctx| Flow::Replace(Box::new(stores(ctx))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::{Cmd, Layer};
    use tako_core::paths::DataPaths;
    use tako_platform::Platform;

    fn fresh() -> (tempfile::TempDir, Ctx) {
        let dir = tempfile::tempdir().unwrap();
        let ctx = crate::app::build_ctx(DataPaths::at(dir.path()), Platform::simulated(), true);
        i18n::set_lang(Lang::PtBr);
        (dir, ctx)
    }

    fn keys_of(layer: &dyn Layer, ctx: &Ctx) -> Vec<String> {
        layer
            .render(ctx)
            .rows
            .iter()
            .map(|r| r.key.to_string())
            .collect()
    }

    #[test]
    fn shows_once_on_a_new_install_and_not_after_an_upgrade() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = fresh();
        // `--simulate` acha os jogos de demonstração; sem eles, é uma instalação nova.
        ctx.found.lock().unwrap().clear();
        ctx.library.refresh();
        assert!(needed(&mut ctx));
        let _ = finish(&mut ctx);
        assert!(!needed(&mut ctx));

        // Com jogos: como quem atualiza de uma alpha.
        let (_d, mut ctx) = fresh();
        assert!(!needed(&mut ctx));
        assert!(ctx.settings.bool(keys::FIRST_RUN_DONE, false));
    }

    #[test]
    fn walks_language_stores_and_providers() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = fresh();
        let mut layer: Box<dyn Layer> = Box::new(welcome(&ctx));
        // Foco começa em "Continuar"; sobe para o inglês e escolhe.
        layer.handle(Cmd::Up, &mut ctx);
        let Flow::Replace(l) = layer.handle(Cmd::Accept, &mut ctx) else {
            panic!("idioma")
        };
        layer = l;
        assert_eq!(i18n::lang(), Lang::EnUs);
        assert_eq!(ctx.settings.string(keys::LANGUAGE), "en-US");
        assert_eq!(layer.render(&ctx).title, i18n::t("first_run.welcome"));
        i18n::set_lang(Lang::PtBr);

        // Continuar → lojas.
        layer.handle(Cmd::Down, &mut ctx);
        let Flow::Replace(l) = layer.handle(Cmd::Accept, &mut ctx) else {
            panic!("lojas")
        };
        layer = l;
        let rows = keys_of(layer.as_ref(), &ctx);
        assert!(rows.iter().any(|k| k.starts_with("store:")));
        assert_eq!(rows.last().map(String::as_str), Some("next"));

        // B volta um passo; Continuar de novo e chega aos provedores.
        let Flow::Replace(back) = layer.handle(Cmd::Back, &mut ctx) else {
            panic!("voltar")
        };
        assert_eq!(
            keys_of(back.as_ref(), &ctx).last().map(String::as_str),
            Some("skip")
        );
        // O foco das lojas já começa em "Continuar".
        let mut s = stores(&ctx);
        let n = keys_of(&s, &ctx).len();
        assert_eq!(s.render(&ctx).sel, n as i32 - 1);
        let Flow::Replace(l) = s.handle(Cmd::Accept, &mut ctx) else {
            panic!("provedores")
        };
        layer = l;
        let rows = keys_of(layer.as_ref(), &ctx);
        assert!(rows.contains(&"settings.metadata.igdb".to_string()));
        assert_eq!(rows.last().map(String::as_str), Some("done"));
        assert!(!ctx.settings.bool(keys::FIRST_RUN_DONE, false));

        // B no primeiro passo pula e também conta como concluído.
        let mut w = welcome(&ctx);
        assert!(matches!(w.handle(Cmd::Back, &mut ctx), Flow::Close));
        assert!(ctx.settings.bool(keys::FIRST_RUN_DONE, false));
    }
}
