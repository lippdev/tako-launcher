//! Diálogos e painéis do jogo e da biblioteca (os `DialogViewModel` do WPF).

use super::keyboard::KeyboardLayer;
use super::layer::{Flow, MenuItem, MenuLayer};
use super::library::Criteria;
use super::{Ctx, Effect};
use crate::model::PanelRow;
use tako_core::library::{GameStore, LibraryFilter, collections, query};
use tako_core::presentation::library_text;

pub fn toggle_favorite(ctx: &mut Ctx, id: &str) {
    match ctx.library.toggle_favorite(id) {
        Ok(()) => {
            let fav = ctx.library.find(id).is_some_and(|g| g.is_favorite);
            ctx.toast(ctx.t(if fav {
                "game.favorited"
            } else {
                "game.unfavorited"
            }));
        }
        Err(e) => tracing::error!("favoritar: {e}"),
    }
}

pub fn confirm_exit(ctx: &Ctx) -> MenuLayer {
    MenuLayer::dialog(ctx.t("exit.title"), |ctx| {
        vec![
            MenuItem::new(PanelRow::button("exit", ctx.t("exit.confirm")).danger()).on(|ctx| {
                ctx.effect(Effect::ExitToDesktop);
                Flow::Close
            }),
            MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel"))).on(|_| Flow::Close),
        ]
    })
    .text(ctx.t("exit.text"))
}

/// Outro jogo está aberto: termine antes de abrir este (o C# pedia o mesmo).
pub fn other_game_running(ctx: &Ctx, id: &str) -> MenuLayer {
    let running = ctx
        .running
        .as_ref()
        .map(|r| r.title.clone())
        .unwrap_or_default();
    let id = id.to_owned();
    MenuLayer::dialog(
        ctx.tf("session.other_running", &[("game", &running)]),
        move |ctx| {
            let id = id.clone();
            vec![
                MenuItem::new(PanelRow::button("resume", ctx.t("game.resume"))).on(|ctx| {
                    ctx.effect(Effect::ResumeGame);
                    Flow::Close
                }),
                MenuItem::new(PanelRow::button("end", ctx.t("session.end_and_play")).danger()).on(
                    move |ctx| {
                        ctx.effect(Effect::EndGame);
                        ctx.queued_launch = Some(id.clone());
                        Flow::Close
                    },
                ),
                MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel")))
                    .on(|_| Flow::Close),
            ]
        },
    )
}

/// ☰ numa capa: Jogar, Favoritar, Coleção, Configurações deste jogo, Detalhes.
pub fn quick_actions(ctx: &Ctx, id: &str) -> MenuLayer {
    let title = ctx
        .library
        .find(id)
        .map(|g| g.title.clone())
        .unwrap_or_default();
    let id = id.to_owned();
    MenuLayer::dialog(title, move |ctx| {
        let Some(g) = ctx.library.find(&id) else {
            return Vec::new();
        };
        let (a, b, c, d, e) = (id.clone(), id.clone(), id.clone(), id.clone(), id.clone());
        vec![
            MenuItem::new(PanelRow::button("play", super::tiles::play_label(ctx, g)).icon("play"))
                .on(move |ctx| match super::details::primary_action(ctx, &a) {
                    super::Nav::Layer(layer) => Flow::Replace(layer),
                    _ => Flow::Close,
                }),
            MenuItem::new(
                PanelRow::button(
                    "favorite",
                    ctx.t(if g.is_favorite {
                        "details.unfavorite"
                    } else {
                        "details.favorite"
                    }),
                )
                .icon("star"),
            )
            .on(move |ctx| {
                toggle_favorite(ctx, &b);
                Flow::Stay
            }),
            MenuItem::new(
                PanelRow::button("collections", ctx.t("dialog.add_to_collection")).icon("grid"),
            )
            .on(move |ctx| Flow::Replace(Box::new(choose_collections(ctx, &c)))),
            MenuItem::new(
                PanelRow::button("settings", ctx.t("details.game_settings")).icon("gear"),
            )
            .on(move |ctx| Flow::Replace(game_settings(ctx, &d))),
            MenuItem::new(PanelRow::button("details", ctx.t("hint.details")).icon("info")).on(
                move |ctx| {
                    ctx.open_details = Some(e.clone());
                    Flow::Close
                },
            ),
        ]
    })
}

/// Opções do jogo (detalhes): abrir na Steam, imagens, fontes e ids, editar, abrir pasta, remover.
pub fn game_options(ctx: &Ctx, id: &str) -> MenuLayer {
    let title = ctx
        .library
        .find(id)
        .map(|g| g.title.clone())
        .unwrap_or_default();
    let id = id.to_owned();
    MenuLayer::dialog(title, move |ctx| {
        let Some(g) = ctx.library.find(&id) else {
            return Vec::new();
        };
        let mut v = Vec::new();
        if let Some(app) = g.steam_app_id {
            v.push(
                MenuItem::new(
                    PanelRow::button("steam", ctx.t("options.open_steam")).icon("store-steam"),
                )
                .on(move |ctx| {
                    ctx.effect(Effect::OpenUrl(format!("steam://nav/games/details/{app}")));
                    Flow::Close
                }),
            );
        }
        let i = id.clone();
        v.push(
            MenuItem::new(PanelRow::button("art", ctx.t("options.change_art")).icon("image"))
                .on(move |ctx| Flow::Replace(super::art::picker(ctx, &i))),
        );
        let i = id.clone();
        v.push(
            MenuItem::new(PanelRow::button("ids", ctx.t("options.source_ids")).icon("search"))
                .on(move |ctx| Flow::Replace(super::source_ids::panel(ctx, &i))),
        );
        if g.store() == GameStore::Manual {
            let i = id.clone();
            v.push(
                MenuItem::new(PanelRow::button("edit", ctx.t("options.edit")).icon("gear"))
                    .on(move |ctx| Flow::Replace(super::edit_game::panel(ctx, &i))),
            );
        }
        if let Some(path) = g.install_path.clone() {
            v.push(
                MenuItem::new(
                    PanelRow::button("folder", ctx.t("options.open_folder")).icon("folder"),
                )
                .on(move |ctx| {
                    ctx.effect(Effect::OpenFolder(path.clone()));
                    Flow::Close
                }),
            );
        }
        if g.store() == GameStore::Manual {
            let i = id.clone();
            v.push(
                MenuItem::new(PanelRow::button("remove", ctx.t("options.remove")).danger())
                    .on(move |ctx| Flow::Replace(Box::new(confirm_remove(ctx, &i)))),
            );
        }
        v
    })
}

/// Jogo quebrado (LIB-004): o arquivo ou a pasta sumiu; corrigir pelo "Editar" ou remover da biblioteca.
pub fn broken_game(ctx: &Ctx, id: &str) -> MenuLayer {
    let title = ctx
        .library
        .find(id)
        .map(|g| g.title.clone())
        .unwrap_or_default();
    let id = id.to_owned();
    MenuLayer::dialog(ctx.t("game.missing_file"), move |ctx| {
        let (a, b) = (id.clone(), id.clone());
        vec![
            MenuItem::new(PanelRow::button("edit", ctx.t("options.edit")).icon("gear"))
                .on(move |ctx| Flow::Replace(super::edit_game::panel(ctx, &a))),
            MenuItem::new(PanelRow::button("remove", ctx.t("options.remove")).danger())
                .on(move |ctx| Flow::Replace(Box::new(confirm_remove(ctx, &b)))),
            MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel"))).on(|_| Flow::Close),
        ]
    })
    .overline(title.to_uppercase())
    .text(ctx.t("game.broken_text"))
}

fn confirm_remove(ctx: &Ctx, id: &str) -> MenuLayer {
    let title = ctx
        .library
        .find(id)
        .map(|g| g.title.clone())
        .unwrap_or_default();
    let id = id.to_owned();
    MenuLayer::dialog(
        ctx.tf("options.remove_title", &[("game", &title)]),
        move |ctx| {
            let id = id.clone();
            vec![
                MenuItem::new(PanelRow::button("remove", ctx.t("options.remove")).danger()).on(
                    move |ctx| {
                        match ctx.library.remove_manual(&id) {
                            Ok(()) => ctx.toast(ctx.t("options.removed")),
                            Err(e) => ctx.toast(e.to_string()),
                        }
                        ctx.leave_details = true;
                        Flow::Close
                    },
                ),
                MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel")))
                    .on(|_| Flow::Close),
            ]
        },
    )
    .text(ctx.t("options.remove_text"))
}

/// Coleções do jogo (✓ = está nela); continua aberto para marcar várias.
pub fn choose_collections(ctx: &Ctx, id: &str) -> MenuLayer {
    let title = ctx
        .library
        .find(id)
        .map(|g| g.title.clone())
        .unwrap_or_default();
    let id = id.to_owned();
    MenuLayer::panel(ctx.t("collections.title"), move |ctx| {
        let mut v: Vec<MenuItem> = ctx
            .collections
            .collections()
            .iter()
            .map(|c| {
                let (cid, gid) = (c.id.clone(), id.clone());
                MenuItem::new(PanelRow::check(
                    c.id.clone(),
                    c.name.clone(),
                    c.contains(&id),
                ))
                .on(move |ctx| {
                    if let Err(e) = ctx.collections.toggle(&cid, &gid) {
                        ctx.toast(e.to_string());
                    }
                    ctx.sync_collections();
                    Flow::Stay
                })
            })
            .collect();
        let gid = id.clone();
        v.push(
            MenuItem::new(PanelRow::action("new", ctx.t("collections.new")).icon("more")).on(
                move |ctx| {
                    let gid = gid.clone();
                    Flow::Push(Box::new(new_collection(ctx, Some(gid))))
                },
            ),
        );
        v.push(MenuItem::new(PanelRow::button("done", ctx.t("common.done"))).on(|_| Flow::Close));
        v
    })
    .overline(title.to_uppercase())
}

/// Nome de uma coleção nova pelo teclado virtual (até 40 caracteres, sem repetir).
pub fn new_collection(ctx: &Ctx, add_game: Option<String>) -> KeyboardLayer {
    KeyboardLayer::new(
        ctx,
        ctx.t("collections.new"),
        "",
        move |ctx, name| match ctx.collections.create(name, add_game.as_deref()) {
            Ok(_) => {
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
    .check(|ctx, name| {
        collections::name_problem(name, ctx.collections.collections(), None)
            .map(|p| p.message(ctx.lang()))
    })
}

/// Painel de filtros da biblioteca (mostrar, ordenar, coleção, fonte, gênero).
pub fn library_filters(ctx: &Ctx, criteria: Criteria, route: LibraryFilter) -> MenuLayer {
    MenuLayer::panel(ctx.t("filters.title"), move |ctx| {
        let lang = ctx.lang();
        let c = criteria.borrow().clone();
        let mut v = vec![MenuItem::new(PanelRow::header(ctx.t("filters.show")))];
        let cr = criteria.clone();
        v.push(
            MenuItem::new(PanelRow::choice(
                "install",
                ctx.t("filters.install"),
                library_text::install_label(lang, c.install),
            ))
            .on(move |ctx| {
                let next = cr.borrow().next_install();
                *cr.borrow_mut() = next;
                ctx.dirty = true;
                Flow::Stay
            }),
        );
        let cr = criteria.clone();
        v.push(
            MenuItem::new(PanelRow::toggle(
                "fav",
                ctx.t("filters.favorites_only"),
                c.favorites_only,
            ))
            .on(move |_| {
                let mut c = cr.borrow_mut();
                c.favorites_only = !c.favorites_only;
                Flow::Stay
            }),
        );
        let cr = criteria.clone();
        v.push(
            MenuItem::new(PanelRow::toggle(
                "played",
                ctx.t("filters.played"),
                c.played_only,
            ))
            .on(move |_| {
                let mut c = cr.borrow_mut();
                c.played_only = !c.played_only;
                Flow::Stay
            }),
        );
        let cr = criteria.clone();
        v.push(
            MenuItem::new(PanelRow::toggle(
                "broken",
                ctx.t("filters.broken"),
                c.broken_only,
            ))
            .on(move |_| {
                let mut c = cr.borrow_mut();
                c.broken_only = !c.broken_only;
                Flow::Stay
            }),
        );
        v.push(MenuItem::new(PanelRow::header(ctx.t("filters.sort"))));
        for s in library_text::SORT_ORDER {
            let cr = criteria.clone();
            v.push(
                MenuItem::new(PanelRow::radio(
                    format!("{s:?}"),
                    library_text::sort_label(lang, s),
                    c.sort == s,
                ))
                .on(move |_| {
                    cr.borrow_mut().sort = s;
                    Flow::Stay
                }),
            );
        }
        if !ctx.collections.collections().is_empty() {
            v.push(MenuItem::new(PanelRow::header(ctx.t("filters.collection"))));
            for col in ctx.collections.collections() {
                let (cr, id) = (criteria.clone(), col.id.clone());
                v.push(
                    MenuItem::new(PanelRow::check(
                        col.id.clone(),
                        col.name.clone(),
                        c.has_collection(&col.id),
                    ))
                    .on(move |_| {
                        let next = cr.borrow().toggle_collection(&id);
                        *cr.borrow_mut() = next;
                        Flow::Stay
                    }),
                );
            }
        }
        let games: Vec<_> = ctx
            .library
            .games()
            .iter()
            .filter(|g| route.matches(g))
            .collect();
        let sources = query::sources(games.iter().copied(), |g| {
            tako_core::library::LibraryFacts::of(g)
        });
        if sources.len() > 1 {
            v.push(MenuItem::new(PanelRow::header(ctx.t("filters.source"))));
            for f in sources {
                let (cr, name) = (criteria.clone(), f.name.clone());
                v.push(
                    MenuItem::new(
                        PanelRow::check(f.name.clone(), f.name.clone(), c.has_source(&f.name))
                            .value(f.count.to_string()),
                    )
                    .on(move |_| {
                        let next = cr.borrow().toggle_source(&name);
                        *cr.borrow_mut() = next;
                        Flow::Stay
                    }),
                );
            }
        }
        let genres = query::genres(games.iter().copied(), |g| {
            tako_core::library::LibraryFacts::of(g)
        });
        if !genres.is_empty() {
            v.push(MenuItem::new(PanelRow::header(ctx.t("filters.genre"))));
            for f in genres {
                let (cr, name) = (criteria.clone(), f.name.clone());
                v.push(
                    MenuItem::new(
                        PanelRow::check(f.name.clone(), f.name.clone(), c.has_genre(&f.name))
                            .value(f.count.to_string()),
                    )
                    .on(move |_| {
                        let next = cr.borrow().toggle_genre(&name);
                        *cr.borrow_mut() = next;
                        Flow::Stay
                    }),
                );
            }
        }
        if c.has_filters() {
            let cr = criteria.clone();
            v.push(MenuItem::new(PanelRow::header(ctx.t("filters.filters"))));
            v.push(
                MenuItem::new(PanelRow::button("clear", ctx.t("filters.clear"))).on(move |_| {
                    let next = cr.borrow().clear_filters();
                    *cr.borrow_mut() = next;
                    Flow::Stay
                }),
            );
        }
        v
    })
}

/// "Configurações deste jogo": cada linha começa em "Padrão: <valor do Tako>"; o que o jogo muda fica em negrito.
pub fn game_settings(ctx: &Ctx, id: &str) -> Box<dyn super::Layer> {
    super::game_settings::panel(ctx, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tako_core::library::ManualGameRecord;

    fn accept(ctx: &mut Ctx, layer: &MenuLayer, key: &str) -> Flow {
        let item = (layer.build)(ctx)
            .into_iter()
            .find(|i| i.row.key == key)
            .unwrap_or_else(|| panic!("sem a linha {key}"));
        (item.accept.unwrap())(ctx)
    }

    #[test]
    fn broken_games_are_found_filtered_and_offer_edit_or_remove() {
        let dir = tempfile::tempdir().unwrap();
        let (_l, mut ctx) = crate::metadata::test_ctx(dir.path(), false);
        let exe = dir.path().join("ok.sh");
        std::fs::write(&exe, "").unwrap();
        let now = chrono::Utc::now();
        let ok = ManualGameRecord::create("Ok", &exe.to_string_lossy(), now);
        let gone = ManualGameRecord::create("Sumiu", "/nao/existe/jogo.sh", now);
        let store = ManualGameRecord::create("Loja", "steam://rungameid/570", now);
        let gone_id = gone.id.clone();
        for r in [ok, gone, store] {
            ctx.library.add_manual(r).unwrap();
        }
        ctx.check_broken();
        let broken: Vec<_> = ctx.library.games().iter().filter(|g| g.is_broken).collect();
        assert_eq!(broken.len(), 1);
        assert_eq!(broken[0].id, gone_id);

        let criteria: Criteria = Default::default();
        let filters = library_filters(&ctx, criteria.clone(), LibraryFilter::All);
        accept(&mut ctx, &filters, "broken");
        let shown = query::apply(ctx.library.games(), LibraryFilter::All, &criteria.borrow());
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].id, gone_id);

        // Jogar pelo menu do card abre o aviso com Editar e Remover.
        let menu = quick_actions(&ctx, &gone_id);
        let Flow::Replace(dialog) = accept(&mut ctx, &menu, "play") else {
            panic!("aviso de jogo quebrado");
        };
        let o = dialog.render(&ctx);
        assert_eq!(o.title, "Arquivo não encontrado");
        let keys: Vec<_> = o.rows.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(keys, ["edit", "remove", "cancel"]);
        let warning = broken_game(&ctx, &gone_id);
        let Flow::Replace(mut confirm) = accept(&mut ctx, &warning, "remove") else {
            panic!("confirmar remoção");
        };
        assert!(matches!(
            confirm.handle(super::super::Cmd::Accept, &mut ctx),
            Flow::Close
        ));
        assert!(ctx.library.find(&gone_id).is_none());
    }
}
