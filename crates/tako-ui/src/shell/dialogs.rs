//! Diálogos e painéis do jogo e da biblioteca (os `DialogViewModel` do WPF).

use super::keyboard::KeyboardLayer;
use super::layer::{Flow, MenuItem, MenuLayer};
use super::library::Criteria;
use super::{Ctx, Effect};
use crate::model::PanelRow;
use tako_core::i18n::tn;
use tako_core::library::{Game, GameStore, LibraryFilter, PlayStatus, collections, query, tags};
use tako_core::presentation::{game_text, library_text};

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

/// "Status: Zerado" (o rótulo da linha que abre a escolha e o aviso depois de mudar).
fn status_text(ctx: &Ctx, status: Option<PlayStatus>) -> String {
    ctx.tf(
        "game.status_set",
        &[("status", &library_text::status_label(ctx.lang(), status))],
    )
}

/// Linha "Status: ..." do menu do card e das opções dos detalhes (LIB-001).
fn status_item(ctx: &Ctx, id: &str) -> MenuItem {
    let status = ctx.library.find(id).and_then(|g| g.play_status);
    let id = id.to_owned();
    MenuItem::new(PanelRow::button("status", status_text(ctx, status)).icon("check"))
        .on(move |ctx| Flow::Replace(Box::new(choose_status(ctx, &id))))
}

/// Escolha do status do jogo: "Sem status" e os cinco status; fecha ao escolher.
pub fn choose_status(ctx: &Ctx, id: &str) -> MenuLayer {
    let title = ctx
        .library
        .find(id)
        .map(|g| g.title.clone())
        .unwrap_or_default();
    let id = id.to_owned();
    MenuLayer::dialog(ctx.t("game.status"), move |ctx| {
        let current = ctx.library.find(&id).and_then(|g| g.play_status);
        std::iter::once(None)
            .chain(PlayStatus::ALL.map(Some))
            .map(|s| {
                let id = id.clone();
                MenuItem::new(PanelRow::radio(
                    s.map_or("none", PlayStatus::key),
                    library_text::status_label(ctx.lang(), s),
                    current == s,
                ))
                .on(move |ctx| {
                    match ctx.library.set_status(&id, s) {
                        Ok(()) => ctx.toast(status_text(ctx, s)),
                        Err(e) => tracing::error!("status: {e}"),
                    }
                    Flow::Close
                })
            })
            .collect()
    })
    .overline(title.to_uppercase())
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

/// O app da loja não está instalado (SRC-003): baixar pela página oficial ou tentar mesmo assim (a leitura
/// pode ter errado, por exemplo com o app instalado depois dela).
pub fn client_missing(ctx: &Ctx, game: &Game, notice: String) -> MenuLayer {
    let id = game.id.clone();
    let download = tako_sources::report::client_download(game.store());
    MenuLayer::dialog(ctx.t("game.client_missing"), move |ctx| {
        let id = id.clone();
        let mut v = Vec::new();
        if let Some(url) = download {
            v.push(
                MenuItem::new(PanelRow::button("download", ctx.t("game.client_download"))).on(
                    move |ctx| {
                        ctx.effect(Effect::OpenUrl(url.into()));
                        Flow::Close
                    },
                ),
            );
        }
        v.push(
            MenuItem::new(PanelRow::button("anyway", ctx.t("game.client_try_anyway"))).on(
                move |ctx| {
                    ctx.effect(Effect::Launch(id.clone()));
                    Flow::Close
                },
            ),
        );
        v.push(
            MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel"))).on(|_| Flow::Close),
        );
        v
    })
    .overline(game.title.to_uppercase())
    .text(notice)
}

/// O jogo fechou sozinho logo ao abrir ou com código de erro, ou nem abriu (SESSION-003): o que se sabe (o código,
/// quando há) e o que tentar.
pub fn game_ended_badly(ctx: &Ctx, id: &str, end: tako_core::session::GameEnd) -> MenuLayer {
    use tako_core::session::GameEnd;
    let title = ctx
        .library
        .find(id)
        .map(|g| g.title.clone())
        .unwrap_or_default();
    let (head, text) = match end {
        GameEnd::NeverStarted => ("session.never_started", "session.never_started_text"),
        GameEnd::Failed(_) => ("session.ended_error", "session.ended_error_text"),
        _ => ("session.ended_quick", "session.ended_quick_text"),
    };
    // O código vem antes da sugestão: é o que mais ajuda a achar a causa.
    let text = match end.exit_code() {
        Some(code) => format!(
            "{}.\n\n{}",
            tako_core::session::tracking::exit_code_text(code),
            ctx.t(text)
        ),
        None => ctx.t(text),
    };
    let id = id.to_owned();
    MenuLayer::dialog(ctx.tf(head, &[("game", &title)]), move |ctx| {
        let id = id.clone();
        vec![
            MenuItem::new(PanelRow::button("again", ctx.t("session.try_again")).icon("play")).on(
                move |ctx| {
                    ctx.effect(Effect::Launch(id.clone()));
                    Flow::Close
                },
            ),
            MenuItem::new(PanelRow::button("report", ctx.t("session.save_report")).icon("info"))
                .on(|ctx| {
                    ctx.effect(Effect::SaveDiagnostics);
                    Flow::Close
                }),
            MenuItem::new(PanelRow::button("ok", ctx.t("common.ok"))).on(|_| Flow::Close),
        ]
    })
    .text(text)
}

/// Comandos deste jogo que o usuário ainda não permitiu (SESSION-004): mostra o texto exato antes de rodar.
pub fn approve_commands(ctx: &Ctx, id: &str) -> MenuLayer {
    let eff = crate::session::effective(ctx, id);
    let mut text = vec![ctx.t("session.commands_text")];
    if let Some(c) = &eff.before_command {
        text.push(ctx.tf("session.commands_before", &[("command", c)]));
    }
    if let Some(c) = &eff.after_command {
        text.push(ctx.tf("session.commands_after", &[("command", c)]));
    }
    let id = id.to_owned();
    MenuLayer::dialog(ctx.t("session.commands_title"), move |ctx| {
        let id = id.clone();
        vec![
            MenuItem::new(PanelRow::button("allow", ctx.t("session.commands_allow")).icon("play"))
                .on(move |ctx| {
                    super::game_settings::approve_commands(ctx, &id);
                    ctx.effect(Effect::Launch(id.clone()));
                    Flow::Close
                }),
            MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel"))).on(|_| Flow::Close),
        ]
    })
    .text(text.join("\n"))
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

/// ☰ numa capa: Jogar, Favoritar, Status, Coleção, Configurações deste jogo, Detalhes.
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
            status_item(ctx, &id),
            MenuItem::new(
                PanelRow::button("collections", ctx.t("dialog.add_to_collection")).icon("grid"),
            )
            .on(move |ctx| Flow::Replace(Box::new(choose_collections(ctx, &c)))),
            MenuItem::new(PanelRow::button("tags", ctx.t("tags.title")).icon("check")).on({
                let id = id.clone();
                move |ctx| {
                    let title = ctx.library.find(&id).map(|g| g.title.clone());
                    Flow::Replace(Box::new(choose_tags(ctx, vec![id.clone()], title)))
                }
            }),
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

/// Opções do jogo (detalhes): status, abrir na Steam, imagens, fontes e ids, editar, abrir pasta, remover.
pub fn game_options(ctx: &Ctx, id: &str) -> MenuLayer {
    let title = ctx
        .library
        .find(id)
        .map(|g| g.title.clone())
        .unwrap_or_default();
    let id = id.to_owned();
    MenuLayer::dialog(title, move |ctx| option_items(ctx, &id))
}

/// ⋯ do modo desktop (ADR-0027): as Opções do jogo com coleções, tags e as configurações deste jogo, num menu
/// junto do ponteiro.
pub fn more_actions(ctx: &Ctx, id: &str) -> MenuLayer {
    let title = ctx
        .library
        .find(id)
        .map(|g| g.title.clone())
        .unwrap_or_default();
    let id = id.to_owned();
    MenuLayer::menu(title, move |ctx| {
        let mut v = option_items(ctx, &id);
        let (c, t, s) = (id.clone(), id.clone(), id.clone());
        let extra = [
            MenuItem::new(
                PanelRow::button("collections", ctx.t("dialog.add_to_collection")).icon("grid"),
            )
            .on(move |ctx| Flow::Replace(Box::new(choose_collections(ctx, &c)))),
            MenuItem::new(PanelRow::button("tags", ctx.t("tags.title")).icon("check")).on(
                move |ctx| {
                    let title = ctx.library.find(&t).map(|g| g.title.clone());
                    Flow::Replace(Box::new(choose_tags(ctx, vec![t.clone()], title)))
                },
            ),
            MenuItem::new(
                PanelRow::button("settings", ctx.t("details.game_settings")).icon("gear"),
            )
            .on(move |ctx| Flow::Replace(game_settings(ctx, &s))),
        ];
        let at = v.len().min(1);
        v.splice(at..at, extra);
        v
    })
}

/// Linhas das Opções do jogo (vazio se o jogo sumiu).
fn option_items(ctx: &Ctx, id: &str) -> Vec<MenuItem> {
    {
        let id = id.to_owned();
        let Some(g) = ctx.library.find(&id) else {
            return Vec::new();
        };
        let mut v = vec![status_item(ctx, &id)];
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
        if let Some(uri) = super::details::repair_uri(g) {
            v.push(
                MenuItem::new(PanelRow::button("repair", ctx.t("options.repair")).icon("check"))
                    .on(move |ctx| {
                        ctx.effect(Effect::OpenUrl(uri.clone()));
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
        if !ctx.library.sessions(&id).is_empty() {
            let i = id.clone();
            v.push(
                MenuItem::new(PanelRow::button("sessions", ctx.t("options.sessions")).icon("play"))
                    .on(move |ctx| Flow::Replace(Box::new(sessions(ctx, &i)))),
            );
        }
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
    }
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

/// Histórico de sessões (SESSION-002): data, hora e duração, da mais recente para a mais antiga.
pub fn sessions(ctx: &Ctx, id: &str) -> MenuLayer {
    let id = id.to_owned();
    MenuLayer::panel(ctx.t("options.sessions"), move |ctx| {
        let lang = ctx.lang();
        ctx.library
            .sessions(&id)
            .iter()
            .rev()
            .map(|s| {
                let at = s.started_at.with_timezone(&chrono::Local);
                let when = format!(
                    "{} {}",
                    game_text::date(lang, at.date_naive()),
                    at.format("%H:%M")
                );
                MenuItem::new(PanelRow::info(
                    "session",
                    when,
                    game_text::session_played(lang, s.duration),
                ))
            })
            .collect()
    })
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

/// Coleções de vários jogos (SelecaoMultipla). Marcada = todos estão nela; clicar põe todos ou tira todos.
pub fn choose_collections_many(ctx: &Ctx, ids: Vec<String>) -> MenuLayer {
    let ids = std::rc::Rc::new(ids);
    let overline = tn("common.games", ids.len() as i64, &[]);
    MenuLayer::panel(ctx.t("collections.title"), move |ctx| {
        let ids = ids.clone();
        let mut v: Vec<MenuItem> = ctx
            .collections
            .collections()
            .iter()
            .map(|c| {
                let all = ids.iter().all(|id| c.contains(id));
                let (cid, name, ids) = (c.id.clone(), c.name.clone(), ids.clone());
                MenuItem::new(PanelRow::check(c.id.clone(), c.name.clone(), all)).on(move |ctx| {
                    let inside = ctx.collections.find(&cid).map(|c| ids.iter().filter(|id| c.contains(id)).cloned().collect::<Vec<_>>()).unwrap_or_default();
                    let change: Vec<String> = if all {
                        ids.iter().cloned().collect()
                    } else {
                        ids.iter().filter(|id| !inside.contains(id)).cloned().collect()
                    };
                    for id in &change {
                        if let Err(e) = ctx.collections.toggle(&cid, id) {
                            ctx.toast(e.to_string());
                        }
                    }
                    ctx.sync_collections();
                    let n = ids.len().to_string();
                    let key = if all { "desktop.bulk.removed_from" } else { "desktop.bulk.added_to" };
                    // Desfazer troca de volta só os jogos que esta ação mudou.
                    let cid = cid.clone();
                    ctx.toast_with_undo(ctx.tf(key, &[("n", &n), ("collection", &name)]), move |ctx| {
                        for id in &change {
                            if let Err(e) = ctx.collections.toggle(&cid, id) {
                                tracing::error!("coleção: {e}");
                            }
                        }
                        ctx.sync_collections();
                    });
                    Flow::Stay
                })
            })
            .collect();
        v.push(MenuItem::new(PanelRow::button("done", ctx.t("common.done"))).on(|_| Flow::Close));
        v
    })
    .overline(overline.to_uppercase())
}

/// ⋯ da seleção múltipla: marcar como zerado e apagar da biblioteca (só os adicionados à mão, com confirmação).
pub fn bulk_more(ids: Vec<String>) -> MenuLayer {
    MenuLayer::menu(String::new(), move |ctx| {
        let done = ids.clone();
        let mut v = vec![
            MenuItem::new(PanelRow::button("beaten", ctx.t("desktop.bulk.beaten")).icon("check")).on(
                move |ctx| {
                    for id in &done {
                        if let Err(e) = ctx.library.set_status(id, Some(PlayStatus::Beaten)) {
                            tracing::error!("status: {e}");
                        }
                    }
                    ctx.toast(status_text(ctx, Some(PlayStatus::Beaten)));
                    Flow::Close
                },
            ),
        ];
        let manual: Vec<String> = ids
            .iter()
            .filter(|id| ctx.library.find(id).is_some_and(|g| g.store() == GameStore::Manual))
            .cloned()
            .collect();
        if !manual.is_empty() {
            v.push(
                MenuItem::new(PanelRow::button("remove", ctx.t("options.remove")).danger()).on(
                    move |ctx| Flow::Replace(Box::new(confirm_remove_many(ctx, manual.clone()))),
                ),
            );
        }
        v
    })
}

/// "Apagar 3 jogos da biblioteca?": confirmação destrutiva com o número.
fn confirm_remove_many(ctx: &Ctx, ids: Vec<String>) -> MenuLayer {
    let n = ids.len().to_string();
    MenuLayer::dialog(ctx.tf("desktop.bulk.remove_title", &[("n", &n)]), move |ctx| {
        let ids = ids.clone();
        vec![
            MenuItem::new(PanelRow::button("remove", ctx.t("options.remove")).danger()).on(
                move |ctx| {
                    for id in &ids {
                        if let Err(e) = ctx.library.remove_manual(id) {
                            ctx.toast(e.to_string());
                        }
                    }
                    ctx.toast(ctx.t("options.removed"));
                    Flow::Close
                },
            ),
            MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel"))).on(|_| Flow::Close),
        ]
    })
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

/// Tags de um ou de vários jogos (LIB-002). Marcada = todos os jogos têm; A liga em todos ou tira de todos.
/// `overline`: o nome do jogo (um só) ou `None` para "N jogos".
pub fn choose_tags(ctx: &Ctx, ids: Vec<String>, overline: Option<String>) -> MenuLayer {
    let ids = std::rc::Rc::new(ids);
    let overline = overline.unwrap_or_else(|| tn("common.games", ids.len() as i64, &[]));
    let panel_ids = ids.clone();
    MenuLayer::panel(ctx.t("tags.title"), move |ctx| {
        let ids = panel_ids.clone();
        let games: Vec<_> = ids.iter().filter_map(|id| ctx.library.find(id)).collect();
        let mut v: Vec<MenuItem> = tags::all(ctx.library.games())
            .into_iter()
            .map(|f| {
                let have = games.iter().filter(|g| g.tags.contains(&f.name)).count();
                let all = have == games.len() && have > 0;
                let mut row = PanelRow::check(f.name.clone(), f.name.clone(), all);
                if games.len() > 1 && have > 0 {
                    row = row.value(format!("{have}/{}", games.len()));
                }
                let (ids, name) = (ids.clone(), f.name.clone());
                MenuItem::new(row).on(move |ctx| {
                    if let Err(e) = ctx.library.set_tag(&ids, &name, !all) {
                        ctx.toast(e.to_string());
                    }
                    ctx.dirty = true;
                    Flow::Stay
                })
            })
            .collect();
        let new_ids = ids.clone();
        v.push(
            MenuItem::new(PanelRow::action("new", ctx.t("tags.new")).icon("more"))
                .on(move |ctx| Flow::Push(Box::new(new_tag(ctx, new_ids.as_ref().clone())))),
        );
        v.push(MenuItem::new(PanelRow::button("done", ctx.t("common.done"))).on(|_| Flow::Close));
        v
    })
    .overline(overline.to_uppercase())
    .text(ctx.t("tags.text"))
}

/// Nome de uma tag nova pelo teclado virtual; já entra nos jogos de quem abriu.
fn new_tag(ctx: &Ctx, ids: Vec<String>) -> KeyboardLayer {
    KeyboardLayer::new(ctx, ctx.t("tags.new"), "", move |ctx, name| {
        match ctx.library.set_tag(&ids, name, true) {
            Ok(()) => {
                ctx.dirty = true;
                Flow::Close
            }
            Err(e) => {
                ctx.toast(e.to_string());
                Flow::Stay
            }
        }
    })
    .hint(ctx.t("tags.name_rule"))
    .check(|ctx, name| tags::name_problem(name).map(|p| p.message(ctx.lang())))
}

/// Painel de filtros da biblioteca (mostrar, ordenar, coleção, status, fonte, gênero).
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
        let cr = criteria.clone();
        v.push(
            MenuItem::new(PanelRow::choice(
                "play-time",
                ctx.t("smart.play_time"),
                ctx.t(c.play_time.key()),
            ))
            .on(move |ctx| {
                let mut c = cr.borrow_mut();
                c.play_time = c.play_time.next();
                ctx.dirty = true;
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
        let all_tags = tags::all(games.iter().copied());
        if !all_tags.is_empty() {
            v.push(MenuItem::new(PanelRow::header(ctx.t("tags.title"))));
            for f in all_tags {
                let (cr, name) = (criteria.clone(), f.name.clone());
                v.push(
                    MenuItem::new(
                        PanelRow::check(
                            format!("tag:{}", f.name),
                            f.name.clone(),
                            c.has_tag(&f.name),
                        )
                        .value(f.count.to_string()),
                    )
                    .on(move |_| {
                        let next = cr.borrow().toggle_tag(&name);
                        *cr.borrow_mut() = next;
                        Flow::Stay
                    }),
                );
            }
        }
        v.push(MenuItem::new(PanelRow::header(ctx.t("filters.status"))));
        for s in PlayStatus::ALL {
            let cr = criteria.clone();
            let n = games.iter().filter(|g| g.play_status == Some(s)).count();
            v.push(
                MenuItem::new(
                    PanelRow::check(
                        s.key(),
                        library_text::status_label(lang, Some(s)),
                        c.statuses.contains(&s),
                    )
                    .value(n.to_string()),
                )
                .on(move |_| {
                    let next = cr.borrow().toggle_status(s);
                    *cr.borrow_mut() = next;
                    Flow::Stay
                }),
            );
        }
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
        // Coleção automática (LIB-003): estes filtros, a rota da página e a ordenação, com um nome.
        if c.has_filters() || c.has_search() || route != LibraryFilter::All {
            if !c.has_filters() {
                v.push(MenuItem::new(PanelRow::header(ctx.t("filters.filters"))));
            }
            let saved = c.clone();
            v.push(
                MenuItem::new(
                    PanelRow::action("save-smart", ctx.t("smart.save"))
                        .desc(ctx.t("smart.save_desc")),
                )
                .on(move |ctx| {
                    Flow::Push(Box::new(super::smart::save_as(ctx, saved.clone(), route)))
                }),
            );
        }
        // Tag em lote (LIB-002): os jogos que a página mostra com estes filtros.
        let shown: Vec<String> = query::apply(ctx.library.games(), route, &c)
            .into_iter()
            .map(|g| g.id.clone())
            .collect();
        if !shown.is_empty() {
            v.push(MenuItem::new(PanelRow::header(ctx.t("tags.bulk_header"))));
            v.push(
                MenuItem::new(
                    PanelRow::action("tag-shown", ctx.t("tags.tag_shown"))
                        .desc(ctx.t("tags.tag_shown_desc"))
                        .value(tn("common.games", shown.len() as i64, &[])),
                )
                .on(move |ctx| Flow::Push(Box::new(choose_tags(ctx, shown.clone(), None)))),
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
    use crate::shell::layer::Layer;
    use tako_core::library::{LibraryCriteria, ManualGameRecord};

    fn accept(ctx: &mut Ctx, layer: &MenuLayer, key: &str) -> Flow {
        let item = (layer.build)(ctx)
            .into_iter()
            .find(|i| i.row.key == key)
            .unwrap_or_else(|| panic!("sem a linha {key}"));
        (item.accept.unwrap())(ctx)
    }

    #[test]
    fn playing_without_the_store_app_offers_the_official_download() {
        let dir = tempfile::tempdir().unwrap();
        let (_l, mut ctx) = crate::metadata::test_ctx(dir.path(), false);
        let game = Game {
            id: "epic:corrida".into(),
            title: "Corrida".into(),
            store: Some(GameStore::Epic),
            launch_target: Some("com.epicgames.launcher://apps/x?action=launch".into()),
            is_installed: true,
            ..Default::default()
        };
        *ctx.found.lock().unwrap() = vec![game.clone()];
        ctx.library.refresh();
        let report = |installed: bool| tako_sources::report::LibrarySourceReport {
            store: GameStore::Epic,
            enabled: true,
            game_count: 1,
            client_installed: installed,
            notice: (!installed).then(|| "Epic Games Launcher não encontrado".into()),
        };

        ctx.sources = vec![report(false)];
        let super::super::Nav::Layer(dialog) =
            super::super::details::primary_action(&mut ctx, &game.id)
        else {
            panic!("aviso de app ausente");
        };
        assert!(ctx.effects.is_empty());
        let o = dialog.render(&ctx);
        assert_eq!(o.title, "Falta o app da loja");
        let keys: Vec<_> = o.rows.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(keys, ["download", "anyway", "cancel"]);
        let warning = client_missing(&ctx, &game, String::new());
        accept(&mut ctx, &warning, "download");
        assert!(
            matches!(&ctx.effects[..], [Effect::OpenUrl(u)] if u.starts_with("https://store.epicgames.com"))
        );

        // Com o app instalado, Jogar abre direto.
        ctx.effects.clear();
        ctx.sources = vec![report(true)];
        super::super::details::primary_action(&mut ctx, &game.id);
        assert!(matches!(&ctx.effects[..], [Effect::Launch(id)] if id == &game.id));
    }

    #[test]
    fn steam_games_can_ask_the_store_to_repair_their_files() {
        let dir = tempfile::tempdir().unwrap();
        let (_l, mut ctx) = crate::metadata::test_ctx(dir.path(), false);
        let steam = Game {
            id: "steam:570".into(),
            title: "Dota".into(),
            store: Some(GameStore::Steam),
            steam_app_id: Some(570),
            launch_target: Some("steam://rungameid/570".into()),
            is_installed: true,
            ..Default::default()
        };
        let epic = Game {
            id: "epic:x".into(),
            store: Some(GameStore::Epic),
            is_installed: true,
            ..Default::default()
        };
        *ctx.found.lock().unwrap() = vec![steam.clone(), epic.clone()];
        ctx.library.refresh();

        let menu = game_options(&ctx, &steam.id);
        assert!(matches!(accept(&mut ctx, &menu, "repair"), Flow::Close));
        assert!(matches!(&ctx.effects[..], [Effect::OpenUrl(u)] if u == "steam://validate/570"));
        let keys: Vec<_> = (game_options(&ctx, &epic.id).build)(&ctx)
            .into_iter()
            .map(|i| i.row.key)
            .collect();
        assert!(!keys.iter().any(|k| k == "repair"));
    }

    #[test]
    fn game_commands_run_only_after_the_user_allows_them() {
        let dir = tempfile::tempdir().unwrap();
        let (_l, mut ctx) = crate::metadata::test_ctx(dir.path(), false);
        let exe = dir.path().join("jogo.exe");
        std::fs::write(&exe, "").unwrap();
        let record = ManualGameRecord::create("Jogo", &exe.to_string_lossy(), chrono::Utc::now());
        let id = record.id.clone();
        ctx.library.add_manual(record).unwrap();
        let store = ctx.game_settings.clone().unwrap();
        let mut o = store.load(&id).unwrap();
        o.before_command = Some("montar.bat".into());
        store.save(&id, &o).unwrap();

        let super::super::Nav::Layer(dialog) = super::super::details::primary_action(&mut ctx, &id)
        else {
            panic!("pedido de permissão");
        };
        assert!(ctx.effects.is_empty());
        let out = dialog.render(&ctx);
        assert_eq!(out.title, "Permitir os comandos deste jogo?");
        let ask = approve_commands(&ctx, &id);
        assert!(matches!(accept(&mut ctx, &ask, "allow"), Flow::Close));
        assert!(matches!(&ctx.effects[..], [Effect::Launch(g)] if g == &id));
        assert!(crate::session::effective(&ctx, &id).commands_approved);

        // Permitido: Jogar abre direto até o comando mudar.
        ctx.effects.clear();
        super::super::details::primary_action(&mut ctx, &id);
        assert!(matches!(&ctx.effects[..], [Effect::Launch(_)]));
        let mut o = store.load(&id).unwrap();
        o.before_command = Some("outro.bat".into());
        store.save(&id, &o).unwrap();
        assert!(!crate::session::effective(&ctx, &id).commands_approved);
    }

    #[test]
    fn installing_from_steam_waits_for_the_game_and_says_when_it_is_ready() {
        let dir = tempfile::tempdir().unwrap();
        let (_l, mut ctx) = crate::metadata::test_ctx(dir.path(), false);
        let mut game = Game {
            id: "steam:1145360".into(),
            title: "Hades".into(),
            store: Some(GameStore::Steam),
            steam_app_id: Some(1145360),
            is_installed: false,
            ..Default::default()
        };
        *ctx.found.lock().unwrap() = vec![game.clone()];
        ctx.library.refresh();

        super::super::details::primary_action(&mut ctx, &game.id);
        assert!(matches!(&ctx.effects[..], [Effect::OpenUrl(u)] if u == "steam://install/1145360"));
        assert_eq!(ctx.installing.len(), 1);
        assert!(ctx.check_installs());

        game.is_installed = true;
        game.launch_target = Some("steam://rungameid/1145360".into());
        *ctx.found.lock().unwrap() = vec![game];
        ctx.library.refresh();
        assert!(!ctx.check_installs());
        assert_eq!(ctx.toast.as_ref().unwrap().0, "Hades foi instalado");
    }

    #[test]
    fn installing_from_other_stores_opens_the_address_the_source_gave() {
        let dir = tempfile::tempdir().unwrap();
        let (_l, mut ctx) = crate::metadata::test_ctx(dir.path(), false);
        let mut game = Game {
            id: "gog:1207658924".into(),
            title: "Tabuleiro".into(),
            store: Some(GameStore::Gog),
            is_installed: false,
            install_uri: Some("goggalaxy://openGameView/1207658924".into()),
            ..Default::default()
        };
        *ctx.found.lock().unwrap() = vec![game.clone()];
        ctx.library.refresh();

        super::super::details::primary_action(&mut ctx, &game.id);
        assert!(
            matches!(&ctx.effects[..], [Effect::OpenUrl(u)] if u == "goggalaxy://openGameView/1207658924")
        );
        assert!(ctx.check_installs());

        game.is_installed = true;
        game.install_uri = None;
        game.launch_target = Some(r"C:\GOG Galaxy\GalaxyClient.exe".into());
        *ctx.found.lock().unwrap() = vec![game];
        ctx.library.refresh();
        assert!(!ctx.check_installs());
        assert_eq!(ctx.toast.as_ref().unwrap().0, "Tabuleiro foi instalado");
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

    #[test]
    fn status_set_from_the_card_menu_filters_and_persists() {
        let dir = tempfile::tempdir().unwrap();
        let (_l, mut ctx) = crate::metadata::test_ctx(dir.path(), false);
        let record = ManualGameRecord::create("Jogo", "/nao/existe.sh", chrono::Utc::now());
        let id = record.id.clone();
        ctx.library.add_manual(record).unwrap();

        let menu = quick_actions(&ctx, &id);
        let Flow::Replace(_) = accept(&mut ctx, &menu, "status") else {
            panic!("escolha do status");
        };
        let picker = choose_status(&ctx, &id);
        assert!(matches!(accept(&mut ctx, &picker, "beaten"), Flow::Close));
        assert_eq!(ctx.toast.as_ref().unwrap().0, "Status: Zerado");
        let rows = picker.render(&ctx).rows;
        assert_eq!(rows[2].title, "Zerado");
        assert!(rows[2].on);

        let criteria: Criteria = Default::default();
        let filters = library_filters(&ctx, criteria.clone(), LibraryFilter::All);
        accept(&mut ctx, &filters, "abandoned");
        let shown = |c: &Criteria, ctx: &Ctx| {
            query::apply(ctx.library.games(), LibraryFilter::All, &c.borrow()).len()
        };
        assert_eq!(shown(&criteria, &ctx), 0);
        accept(&mut ctx, &filters, "beaten");
        assert_eq!(shown(&criteria, &ctx), 1);
        assert_eq!(
            *criteria.borrow(),
            LibraryCriteria::default()
                .toggle_status(PlayStatus::Abandoned)
                .toggle_status(PlayStatus::Beaten)
        );

        // Reabrir o banco mantém o status.
        drop(ctx);
        let ctx = crate::app::build_ctx(
            tako_core::paths::DataPaths::at(dir.path()),
            tako_platform::Platform::simulated(),
            false,
        );
        assert_eq!(
            ctx.library.find(&id).unwrap().play_status,
            Some(PlayStatus::Beaten)
        );
    }

    #[test]
    fn tags_apply_to_one_or_many_games_filter_and_persist() {
        let dir = tempfile::tempdir().unwrap();
        let (_l, mut ctx) = crate::metadata::test_ctx(dir.path(), false);
        let ids: Vec<String> = ["A", "B"]
            .into_iter()
            .map(|t| {
                let r = ManualGameRecord::create(t, "/nao/existe.sh", chrono::Utc::now());
                let id = r.id.clone();
                ctx.library.add_manual(r).unwrap();
                id
            })
            .collect();

        // Um jogo pelo menu do card; a tag nova entra pelo teclado (aqui direto na biblioteca).
        let menu = quick_actions(&ctx, &ids[0]);
        let Flow::Replace(_) = accept(&mut ctx, &menu, "tags") else {
            panic!("painel de tags");
        };
        ctx.library.set_tag(&ids[..1], "Coop", true).unwrap();

        // Os dois de uma vez: marcada só quando todos têm; A aplica em todos e depois tira de todos.
        let both = choose_tags(&ctx, ids.clone(), None);
        let row = |ctx: &Ctx| both.render(ctx).rows[0].clone();
        assert!(!row(&ctx).on);
        assert_eq!(row(&ctx).value, "1/2");
        accept(&mut ctx, &both, "Coop");
        assert!(row(&ctx).on);
        assert!(
            ids.iter()
                .all(|id| ctx.library.find(id).unwrap().tags.contains("Coop"))
        );

        let criteria: Criteria = Default::default();
        let filters = library_filters(&ctx, criteria.clone(), LibraryFilter::All);
        accept(&mut ctx, &filters, "tag:Coop");
        assert!(criteria.borrow().has_tag("Coop"));
        let Flow::Push(_) = accept(&mut ctx, &filters, "tag-shown") else {
            panic!("tag em lote");
        };

        accept(&mut ctx, &both, "Coop");
        assert!(
            ids.iter()
                .all(|id| ctx.library.find(id).unwrap().tags.is_empty())
        );
        ctx.library.set_tag(&ids[1..], "Sofá", true).unwrap();

        // Reabrir o banco mantém as tags.
        drop(ctx);
        let ctx = crate::app::build_ctx(
            tako_core::paths::DataPaths::at(dir.path()),
            tako_platform::Platform::simulated(),
            false,
        );
        assert!(ctx.library.find(&ids[1]).unwrap().tags.contains("Sofá"));
    }
}
