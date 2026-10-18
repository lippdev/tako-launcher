//! Assistente da primeira execução (UX-003): idioma, lojas e provedores de metadados (com o driver opcional do
//! INPUT-005 no Windows), em três diálogos.
//! Aparece uma vez numa instalação nova; tudo dá para mudar depois em Configurações.

use super::ctx::keys;
use super::layer::{Flow, MenuItem, MenuLayer};
use super::settings::{library, metadata};
use super::{Ctx, Shell};
use crate::model::PanelRow;
use tako_core::i18n::{self, Lang};

const STEPS: usize = 3;
/// No desktop o onboarding é em tela inteira e tem mais dois passos: Modo (abrir em janela ou em tela cheia) e Pronto.
const DESKTOP_STEPS: usize = 5;

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
    if ctx.desktop {
        // TelaOnboarding: "n de 5", título grande e a nota de rodapé de cada passo.
        let total = DESKTOP_STEPS.to_string();
        // "Pular configuração" em todo passo (fica no topo da tela).
        let build = move |ctx: &Ctx| {
            let mut v: Vec<MenuItem> = build(ctx)
                .into_iter()
                .filter(|i| i.row.key != "skip")
                .collect();
            v.push(
                MenuItem::new(PanelRow::button("skip", ctx.t("first_run.full.skip"))).on(finish),
            );
            v
        };
        return MenuLayer::dialog(ctx.t(&format!("first_run.full.{title}")), build)
            .overline(ctx.tf(
                "first_run.count",
                &[("n", &n.to_string()), ("total", &total)],
            ))
            .text(ctx.t(&format!("first_run.full.{text}")))
            .steps((1..=DESKTOP_STEPS).map(|i| i.to_string()).collect(), n - 1)
            .full(ctx.t(&format!("first_run.full.{title}_note")));
    }
    MenuLayer::dialog(ctx.t(&format!("first_run.{title}")), build)
        .overline(ctx.tf(
            "first_run.step",
            &[("n", &n.to_string()), ("total", &STEPS.to_string())],
        ))
        .text(ctx.t(&format!("first_run.{text}")))
        .steps(
            vec![
                ctx.t("first_run.welcome_short"),
                ctx.t("first_run.stores"),
                ctx.t("first_run.metadata"),
            ],
            n - 1,
        )
        .wide()
}

/// Capas da parede ao fundo do onboarding: os jogos que já estão na biblioteca (ambiente, sem foco nem clique).
pub fn wall(ctx: &Ctx) -> Vec<crate::model::Tile> {
    ctx.library
        .games()
        .iter()
        .take(15)
        .map(|g| crate::model::Tile {
            id: g.id.clone(),
            title: g.title.clone(),
            cover: super::tiles::image(g.shown_cover(), super::tiles::COVER),
            accent: crate::values::parse_color(&g.accent),
            installed: g.is_installed,
            ..Default::default()
        })
        .collect()
}

/// Passo 1: idioma. O diálogo é refeito ao trocar, para o próprio texto mudar de idioma. No desktop o onboarding em
/// tela inteira começa pelas lojas (uma pergunta por tela; o idioma segue o sistema e fica em Configurações › Geral).
pub fn welcome(ctx: &Ctx) -> MenuLayer {
    if ctx.desktop {
        return stores(ctx);
    }
    step(ctx, 1, "welcome", "welcome_text", |ctx| {
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
    })
    .focus_at(2)
    .on_back(finish)
}

/// Passo 2 (no desktop, 1): lojas que este sistema lê, com o que a leitura achou em cada uma.
fn stores(ctx: &Ctx) -> MenuLayer {
    if ctx.desktop {
        return desktop_stores(ctx);
    }
    let n = if ctx.desktop { 1 } else { 2 };
    let layer = step(ctx, n, "stores", "stores_text", |ctx| {
        let mut v = library::store_rows(ctx);
        v.push(
            MenuItem::new(PanelRow::button("next", ctx.t("first_run.next"))).on(|ctx| {
                // Desktop: emuladores só quando há algum; senão direto para as capas.
                if ctx.desktop && super::settings::emulators::anything_found(ctx) {
                    Flow::Replace(Box::new(emulators(ctx)))
                } else {
                    Flow::Replace(Box::new(providers(ctx)))
                }
            }),
        );
        v
    })
    // Foco em "Continuar": A seguido não desliga a primeira loja sem querer.
    .focus_at(library::store_rows(ctx).len());
    if ctx.desktop {
        layer.on_back(finish)
    } else {
        layer.on_back(|ctx| Flow::Replace(Box::new(welcome(ctx))))
    }
}

/// Passo 1 do desktop (TelaOnboarding, LojaEncontrada): as lojas com o que o Tako achou nelas (ponto verde quando
/// achou, "Não instalado" apagado), **Procurar jogos** (procura e segue) e **Escolher pastas…** (liga e desliga lojas).
fn desktop_stores(ctx: &Ctx) -> MenuLayer {
    step(ctx, 1, "stores", "stores_text", |ctx| {
        let mut v: Vec<MenuItem> = library::stores()
            .iter()
            .map(|&store| {
                let report = ctx.sources.iter().find(|r| r.store == store && r.enabled);
                let found = report.is_some_and(|r| r.client_installed);
                let desc = match report {
                    Some(r) if r.client_installed => r.summary(),
                    Some(_) => ctx.t("first_run.full.stores_missing"),
                    None => ctx.t("first_run.full.stores_unchecked"),
                };
                let mut row = PanelRow::info(
                    format!("store:{}", store.key()),
                    tako_core::presentation::game_text::store_name(ctx.lang(), store),
                    "",
                )
                .desc(desc)
                .icon(super::sidebar::store_icon(store))
                .enabled(report.is_none_or(|r| r.client_installed))
                .emphasized(true);
                row.on = found;
                MenuItem::new(row)
            })
            .collect();
        v.push(
            MenuItem::new(PanelRow {
                style: "primary".into(),
                ..PanelRow::button("search", ctx.t("first_run.full.stores_search"))
            })
            .on(|ctx| {
                ctx.effect(super::Effect::Rescan);
                if super::settings::emulators::anything_found(ctx) {
                    Flow::Replace(Box::new(emulators(ctx)))
                } else {
                    Flow::Replace(Box::new(providers(ctx)))
                }
            }),
        );
        v.push(
            MenuItem::new(PanelRow::button(
                "choose",
                ctx.t("first_run.full.stores_choose"),
            ))
            .on(|ctx| {
                let layer = MenuLayer::dialog(ctx.t("settings.library.stores_header"), |ctx| {
                    let mut v = library::store_rows(ctx);
                    v.push(
                        MenuItem::new(PanelRow::button("done", ctx.t("common.done")))
                            .on(|_| Flow::Close),
                    );
                    v
                });
                Flow::Push(Box::new(layer))
            }),
        );
        v
    })
    .focus_at(library::stores().len())
    .on_back(finish)
}

/// Passo 2 (desktop): emuladores e pastas de ROMs (os mesmos controles de Configurações › Emuladores e ROMs).
pub(crate) fn emulators(ctx: &Ctx) -> MenuLayer {
    let cache = super::settings::emulators::Cache::default();
    step(ctx, 2, "emulators", "emulators_text", move |ctx| {
        let mut v = super::settings::emulators::rows(ctx, &cache);
        v.push(
            MenuItem::new(PanelRow::button("next", ctx.t("first_run.next")))
                .on(|ctx| Flow::Replace(Box::new(providers(ctx)))),
        );
        v
    })
    .on_back(|ctx| Flow::Replace(Box::new(stores(ctx))))
}

/// Passo 3: provedores de capas e descrições (os mesmos painéis de Configurações) e, no Windows, o driver opcional
/// que esconde o controle do jogo com a Central aberta (INPUT-005).
fn providers_rows(ctx: &Ctx) -> Vec<MenuItem> {
    let mut v = metadata::provider_rows(ctx);
    v.extend(super::settings::controls::hide_pads_row(ctx));
    v
}

fn providers(ctx: &Ctx) -> MenuLayer {
    step(ctx, 3, "metadata", "metadata_text", |ctx| {
        let mut v = providers_rows(ctx);
        if ctx.desktop {
            v.push(
                MenuItem::new(PanelRow::button("next", ctx.t("first_run.next")))
                    .on(|ctx| Flow::Replace(Box::new(mode(ctx)))),
            );
            v.push(
                MenuItem::new(PanelRow::button("back", ctx.t("first_run.full.back"))).on(|ctx| {
                    if super::settings::emulators::anything_found(ctx) {
                        Flow::Replace(Box::new(emulators(ctx)))
                    } else {
                        Flow::Replace(Box::new(stores(ctx)))
                    }
                }),
            );
        } else {
            v.push(MenuItem::new(PanelRow::button("done", ctx.t("first_run.finish"))).on(finish));
        }
        v
    })
    .focus_at(providers_rows(ctx).len())
    .on_back(|ctx| {
        if ctx.desktop && super::settings::emulators::anything_found(ctx) {
            Flow::Replace(Box::new(emulators(ctx)))
        } else {
            Flow::Replace(Box::new(stores(ctx)))
        }
    })
}

/// Passo 4 (desktop): abrir o Tako em janela ou em tela cheia. Já vem marcado pelo que está em uso: com um controle
/// conectado, tela cheia.
pub(crate) fn mode(ctx: &Ctx) -> MenuLayer {
    use crate::mode::{OPEN_IN, OPEN_IN_BIG_PICTURE, OPEN_IN_DESKTOP};
    step(ctx, 4, "mode", "mode_text", |ctx| {
        let current = ctx
            .settings
            .get(OPEN_IN)
            .map(str::to_owned)
            .unwrap_or_else(|| {
                if ctx.pads_connected > 0 {
                    OPEN_IN_BIG_PICTURE.into()
                } else {
                    OPEN_IN_DESKTOP.into()
                }
            });
        let mut v: Vec<MenuItem> = [(OPEN_IN_DESKTOP, "window"), (OPEN_IN_BIG_PICTURE, "full")]
            .into_iter()
            .map(|(value, key)| {
                MenuItem::new(
                    PanelRow::radio(
                        value,
                        ctx.t(&format!("first_run.full.mode_{key}")),
                        current == value,
                    )
                    .desc(ctx.t(&format!("first_run.full.mode_{key}_desc")))
                    .value(ctx.t(&format!("first_run.full.mode_{key}_hint")))
                    .icon(if key == "window" { "monitor" } else { "tv" }),
                )
                .on(move |ctx| {
                    ctx.settings.set(OPEN_IN, value);
                    Flow::Stay
                })
            })
            .collect();
        v.push(
            MenuItem::new(PanelRow::button("next", ctx.t("first_run.next"))).on(move |ctx| {
                if ctx.settings.get(OPEN_IN).is_none() {
                    let value = current.clone();
                    ctx.settings.set(OPEN_IN, &value);
                }
                Flow::Replace(Box::new(ready(ctx)))
            }),
        );
        v.push(
            MenuItem::new(PanelRow::button("back", ctx.t("first_run.full.back")))
                .on(|ctx| Flow::Replace(Box::new(providers(ctx)))),
        );
        v
    })
    .focus_at(2)
    .on_back(|ctx| Flow::Replace(Box::new(providers(ctx))))
}

/// Passo 5 (desktop): o número de verdade e o caminho para a biblioteca.
pub(crate) fn ready(ctx: &Ctx) -> MenuLayer {
    use tako_core::library::GameStore;
    let total = ctx.library.games().len();
    let title = if total == 0 {
        ctx.t("first_run.full.ready_none")
    } else {
        ctx.tf("first_run.full.ready", &[("n", &total.to_string())])
    };
    MenuLayer::dialog(title, |ctx| {
        let lang = ctx.lang();
        let mut v: Vec<MenuItem> = GameStore::ALL
            .into_iter()
            .filter_map(|store| {
                let n = ctx
                    .library
                    .games()
                    .iter()
                    .filter(|g| g.store() == store)
                    .count();
                (n > 0).then(|| {
                    MenuItem::new(
                        PanelRow::info(
                            "store",
                            tako_core::presentation::game_text::store_name(lang, store),
                            tako_core::presentation::library_text::game_count(lang, n),
                        )
                        .icon(super::sidebar::store_icon(store)),
                    )
                })
            })
            .collect();
        let big = ctx.settings.get(crate::mode::OPEN_IN) == Some(crate::mode::OPEN_IN_BIG_PICTURE);
        v.push(
            MenuItem::new(
                PanelRow::action(
                    "change-mode",
                    ctx.t(if big {
                        "first_run.full.opens_full"
                    } else {
                        "first_run.full.opens_window"
                    }),
                )
                .icon(if big { "tv" } else { "pointer" })
                .value(ctx.t("first_run.full.change")),
            )
            .on(|ctx| Flow::Replace(Box::new(mode(ctx)))),
        );
        v.push(
            MenuItem::new(PanelRow::button(
                "done",
                ctx.t("first_run.full.open_library"),
            ))
            .on(finish),
        );
        v
    })
    .overline(ctx.tf(
        "first_run.count",
        &[
            ("n", &DESKTOP_STEPS.to_string()),
            ("total", &DESKTOP_STEPS.to_string()),
        ],
    ))
    .text(ctx.t(if total == 0 {
        "first_run.full.ready_none_text"
    } else {
        "first_run.full.ready_text"
    }))
    .steps(
        (1..=DESKTOP_STEPS).map(|i| i.to_string()).collect(),
        DESKTOP_STEPS - 1,
    )
    .full(ctx.t("first_run.full.ready_note"))
    // Foco em "Abrir a biblioteca": depois das lojas e da linha do modo.
    .focus_at(
        GameStore::ALL
            .into_iter()
            .filter(|s| ctx.library.games().iter().any(|g| g.store() == *s))
            .count()
            + 1,
    )
    .on_back(|ctx| Flow::Replace(Box::new(mode(ctx))))
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
    fn the_desktop_onboarding_has_five_steps_and_saves_how_to_open() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = fresh();
        ctx.desktop = true;
        let accept = |layer: &MenuLayer, ctx: &mut Ctx, key: &str| {
            let item = (layer.build)(ctx)
                .into_iter()
                .find(|i| i.row.key == key)
                .unwrap();
            (item.accept.unwrap())(ctx)
        };
        let next = |flow: Flow| match flow {
            Flow::Replace(l) => l,
            _ => panic!("o passo não avançou"),
        };
        // No desktop o primeiro passo já são as lojas; com jogos de emuladores, o segundo são os emuladores.
        let first = welcome(&ctx);
        let o = first.render(&ctx);
        assert_eq!((o.tab2, o.tabs.len(), o.tab), (1, 5, 0));
        assert_eq!(o.title, ctx.t("first_run.full.stores"));
        assert!(keys_of(&first, &ctx).contains(&"skip".to_owned()));
        // Emuladores só aparecem quando há algum neste PC; senão o passo é pulado e vai direto para as capas.
        // "Procurar jogos" procura e segue.
        let second = next(accept(&first, &mut ctx, "search")).render(&ctx);
        if super::super::settings::emulators::anything_found(&ctx) {
            assert_eq!(
                (second.tab, second.title),
                (1, ctx.t("first_run.full.emulators"))
            );
        } else {
            assert_eq!(
                (second.tab, second.title),
                (2, ctx.t("first_run.full.metadata"))
            );
        }
        assert_eq!(emulators(&ctx).render(&ctx).tab, 1);
        // Capas → Modo (no desktop o terceiro passo não termina).
        let providers = providers(&ctx);
        let mode_layer = next(accept(&providers, &mut ctx, "next"));
        assert_eq!(mode_layer.render(&ctx).tab, 3);
        // Sem controle, "Em janela" já vem marcado; escolher tela cheia grava a preferência.
        let mode = mode(&ctx);
        assert!(
            mode.render(&ctx)
                .rows
                .iter()
                .any(|r| r.key == crate::mode::OPEN_IN_DESKTOP && r.on)
        );
        let _ = accept(&mode, &mut ctx, crate::mode::OPEN_IN_BIG_PICTURE);
        assert_eq!(
            ctx.settings.get(crate::mode::OPEN_IN),
            Some(crate::mode::OPEN_IN_BIG_PICTURE)
        );
        let ready = next(accept(&mode, &mut ctx, "next"));
        let o = ready.render(&ctx);
        assert_eq!(o.tab, 4);
        assert!(o.rows.iter().any(|r| r.key == "done"));
        // Na tela cheia continua o assistente de três passos.
        ctx.desktop = false;
        assert_eq!(super::welcome(&ctx).render(&ctx).tabs.len(), 3);
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

    #[test]
    fn offers_the_optional_controller_driver_where_it_exists() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = fresh();
        assert!(
            !keys_of(&providers(&ctx), &ctx).contains(&"settings.controls.hidhide".to_string())
        );
        ctx.platform.controller_hiding =
            std::sync::Arc::new(tako_core::devices::fake::FakeHiding::new(false));
        let p = providers(&ctx);
        let rows = keys_of(&p, &ctx);
        let at = rows.iter().position(|k| k == "settings.controls.hidhide");
        assert_eq!(at, Some(rows.len() - 2));
        // O foco continua em "Concluir".
        assert_eq!(p.render(&ctx).sel, rows.len() as i32 - 1);
    }
}
