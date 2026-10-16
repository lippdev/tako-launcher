//! "Fontes e IDs" (port de `SourceIdsViewModel.cs`, ADR-0023): para cada fonte ligada, o id casado (com nome e ano),
//! os candidatos da busca pelo nome em todas as fontes ao mesmo tempo (✓ = o casado, ★ = o mais provável) e as ações
//! "Digitar ou colar o ID", "Nenhum" e "Automático".

use super::Ctx;
use super::keyboard::KeyboardLayer;
use super::layer::{Flow, Layer, MenuItem, MenuLayer};
use crate::metadata::{Pending, spawn};
use crate::model::PanelRow;
use chrono::Datelike;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use tako_core::library::Game;
use tako_core::metadata::MetadataMatch;
use tako_core::metadata::agent::MetadataAgent;
use tako_core::metadata::art::{parse_source_id, source_id_example};
use tako_core::metadata::art_catalog::known_ids;
use tako_core::metadata::title_matcher;

type Results = HashMap<String, Vec<MetadataMatch>>;

struct State {
    game: Game,
    term: String,
    results: Pending<Results>,
}

/// O id em uso numa fonte, o nome e o ano que ele representa e se foi o usuário que escolheu.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceState {
    pub source: String,
    pub id: Option<String>,
    pub title: Option<String>,
    pub year: Option<i32>,
    pub pinned: bool,
    pub none: bool,
}

pub fn states(ctx: &Ctx, game: &Game) -> Vec<SourceState> {
    let Some(m) = &ctx.metadata else {
        return Vec::new();
    };
    let cached = MetadataAgent::normalize(crate::metadata::cache(ctx).read(&game.id));
    let pinned = ctx.pinned_ids.get(&game.id).cloned().unwrap_or_default();
    let known = known_ids(game, cached.as_ref(), &pinned);
    m.agent
        .sources(game)
        .iter()
        .map(|p| {
            let name = p.name().to_owned();
            let part = cached
                .as_ref()
                .and_then(|c| c.parts.iter().find(|x| x.source == name));
            let none = pinned.get(&name).is_some_and(String::is_empty);
            SourceState {
                id: if none {
                    None
                } else {
                    known.get(&name).cloned()
                },
                title: part.and_then(|p| p.title.clone()),
                year: part.and_then(|p| p.release_date).map(|d| d.year()),
                pinned: pinned.contains_key(&name),
                none,
                source: name,
            }
        })
        .collect()
}

fn describe(ctx: &Ctx, s: &SourceState) -> String {
    if s.none {
        return ctx.t("source_ids.none_chosen");
    }
    let Some(id) = &s.id else {
        return ctx.t("source_ids.no_match");
    };
    let mut text = format!("ID {id}");
    if let Some(t) = &s.title {
        text.push_str(&format!(" · {t}"));
    }
    if let Some(y) = s.year {
        text.push_str(&format!(" ({y})"));
    }
    text.push_str(" · ");
    text.push_str(&ctx.t(if s.pinned {
        "source_ids.chosen"
    } else {
        "source_ids.automatic_lower"
    }));
    text
}

/// Busca o termo em todas as fontes ao mesmo tempo (cada uma no próprio ritmo).
fn search(ctx: &Ctx, game: &Game, term: &str) -> Pending<Results> {
    let agent = ctx.metadata.as_ref().map(|m| m.agent.clone());
    let (game, term) = (game.clone(), term.to_owned());
    spawn(move || {
        let Some(agent) = agent else {
            return Results::new();
        };
        let sources: Vec<String> = agent
            .sources(&game)
            .iter()
            .map(|p| p.name().to_owned())
            .collect();
        std::thread::scope(|s| {
            let jobs: Vec<_> = sources
                .iter()
                .map(|src| {
                    let (agent, game, term) = (&agent, &game, &term);
                    s.spawn(move || (src.clone(), agent.search_source(game, src, term)))
                })
                .collect();
            jobs.into_iter().filter_map(|j| j.join().ok()).collect()
        })
    })
}

/// Fixa o id da fonte (`Some("")` = "nenhum", `None` = volta ao automático) e busca de novo os dados do jogo.
pub fn pin(ctx: &mut Ctx, game: &Game, source: &str, id: Option<&str>, name: &str) {
    if let Some(c) = &ctx.choices
        && let Err(e) = c.set_id(&game.id, source, id)
    {
        tracing::warn!("ids fixados de {}: {e}", game.id);
        ctx.toast(ctx.t("source_ids.save_failed"));
        return;
    }
    let ids = ctx.pinned_ids.entry(game.id.clone()).or_default();
    match id {
        None => ids.remove(source),
        Some(i) => ids.insert(source.to_owned(), i.trim().to_owned()),
    };
    crate::metadata::refresh(ctx, Some(&game.id));
    let text = match id {
        None => ctx.tf("source_ids.auto_done", &[("source", source)]),
        Some("") => ctx.tf(
            "source_ids.none_done",
            &[("source", source), ("game", &game.title)],
        ),
        Some(_) => ctx.tf(
            "source_ids.pinned_done",
            &[("source", source), ("game", &game.title), ("name", name)],
        ),
    };
    ctx.toast(text);
}

/// Texto digitado ou colado: confere na fonte (nome e ano) e fixa.
pub fn enter_id(ctx: &mut Ctx, game: &Game, source: &str, text: &str) {
    let input = parse_source_id(source, text);
    if let Some(p) = input.problem.clone() {
        ctx.toast(p);
        return;
    }
    let Some(agent) = ctx.metadata.as_ref().map(|m| m.agent.clone()) else {
        return;
    };
    ctx.toast(ctx.tf("source_ids.checking", &[("source", source)]));
    let (game, src) = (game.clone(), source.to_owned());
    crate::tasks::run(
        ctx,
        {
            let src = src.clone();
            move || agent.validate(&src, &input)
        },
        move |ctx, found| match found {
            Some((id, m)) => {
                let mut name = m.title.clone().unwrap_or_else(|| id.clone());
                if let Some(d) = m.release_date {
                    name.push_str(&format!(" ({})", d.year()));
                }
                pin(ctx, &game, &src, Some(&id), &name);
            }
            None => ctx.toast(ctx.tf("source_ids.unknown_id", &[("source", &src)])),
        },
    );
}

pub fn panel(ctx: &Ctx, id: &str) -> Box<dyn Layer> {
    let Some(game) = ctx.library.find(id).cloned() else {
        return Box::new(MenuLayer::dialog(ctx.t("options.source_ids"), |_| {
            Vec::new()
        }));
    };
    let title = ctx.tf("source_ids.title", &[("game", &game.title)]);
    let state = Rc::new(RefCell::new(State {
        results: search(ctx, &game, &game.title),
        term: game.title.clone(),
        game,
    }));
    Box::new(MenuLayer::dialog(title, move |ctx| rows(ctx, &state)).wide())
}

fn rows(ctx: &Ctx, state: &Rc<RefCell<State>>) -> Vec<MenuItem> {
    let st = state.borrow();
    let game = st.game.clone();
    let mut v = Vec::new();
    let s = state.clone();
    v.push(
        MenuItem::new(PanelRow::choice(
            "term",
            ctx.t("source_ids.term"),
            st.term.clone(),
        ))
        .on(move |ctx| {
            let s = s.clone();
            let term = s.borrow().term.clone();
            Flow::Push(Box::new(KeyboardLayer::new(
                ctx,
                ctx.t("source_ids.term"),
                &term,
                move |ctx, t| {
                    if !t.is_empty() {
                        let mut st = s.borrow_mut();
                        st.term = t.to_owned();
                        st.results = search(ctx, &st.game, t);
                    }
                    Flow::Close
                },
            )))
        }),
    );
    let all = states(ctx, &game);
    if all.is_empty() {
        v.push(MenuItem::new(PanelRow::info(
            "",
            ctx.t("source_ids.no_sources"),
            "",
        )));
    }
    let results = st.results.get();
    for src in all {
        let name = src.source.clone();
        v.push(MenuItem::new(PanelRow::header(name.to_uppercase())));
        v.push(MenuItem::new(PanelRow::info(
            format!("{name}:state"),
            describe(ctx, &src),
            "",
        )));
        let (g, n) = (game.clone(), name.clone());
        v.push(
            MenuItem::new(
                PanelRow::action(format!("{name}:type"), ctx.t("source_ids.type_id"))
                    .desc(source_id_example(&name)),
            )
            .on(move |ctx| {
                let (g, n) = (g.clone(), n.clone());
                let src = n.clone();
                Flow::Push(Box::new(
                    KeyboardLayer::new(
                        ctx,
                        ctx.tf("source_ids.id_in", &[("source", &n)]),
                        "",
                        move |ctx, t| {
                            enter_id(ctx, &g, &n, t);
                            Flow::Close
                        },
                    )
                    .hint(source_id_example(&src))
                    .check(move |_, t| {
                        (!t.trim().is_empty())
                            .then(|| parse_source_id(&src, t).problem)
                            .flatten()
                    }),
                ))
            }),
        );
        let (g, n) = (game.clone(), name.clone());
        v.push(
            MenuItem::new(
                PanelRow::action(format!("{name}:none"), ctx.t("source_ids.none"))
                    .desc(ctx.t("source_ids.none_text")),
            )
            .on(move |ctx| {
                pin(ctx, &g, &n, Some(""), "");
                Flow::Stay
            }),
        );
        if src.pinned {
            let (g, n) = (game.clone(), name.clone());
            v.push(
                MenuItem::new(
                    PanelRow::action(format!("{name}:auto"), ctx.t("source_ids.automatic"))
                        .desc(ctx.t("source_ids.automatic_text")),
                )
                .on(move |ctx| {
                    pin(ctx, &g, &n, None, "");
                    Flow::Stay
                }),
            );
        }
        let Some(results) = &results else {
            v.push(MenuItem::new(PanelRow::info(
                "",
                ctx.tf("source_ids.searching", &[("term", &st.term)]),
                "",
            )));
            continue;
        };
        let found = results.get(&name).cloned().unwrap_or_default();
        if found.is_empty() {
            v.push(MenuItem::new(PanelRow::info(
                "",
                ctx.tf("source_ids.nothing", &[("term", &st.term)]),
                "",
            )));
        }
        let best = found
            .iter()
            .find(|r| r.is_exact)
            .or_else(|| title_matcher::best(&st.term, &found))
            .map(|m| m.id.clone());
        for m in &found {
            let current = src.id.as_deref() == Some(m.id.as_str());
            let marker = if current {
                "✓ "
            } else if best.as_deref() == Some(m.id.as_str()) {
                "★ "
            } else {
                ""
            };
            let detail = [
                m.year.map(|y| y.to_string()),
                m.platforms.clone(),
                Some(format!("ID {}", m.id)),
            ]
            .into_iter()
            .flatten()
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
            let (g, n, chosen) = (game.clone(), name.clone(), m.clone());
            v.push(
                MenuItem::new(
                    PanelRow::action(format!("{name}:{}", m.id), format!("{marker}{}", m.title))
                        .desc(detail)
                        .emphasized(current),
                )
                .on(move |ctx| {
                    pin(ctx, &g, &n, Some(&chosen.id), &chosen.title);
                    Flow::Stay
                }),
            );
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::{Metadata, basic_catalog};
    use std::sync::Arc;
    use tako_core::metadata::http::SystemClock;
    use tako_core::metadata::{GameMetadata, MetadataLookup, MetadataProvider};

    struct Fake;

    impl MetadataProvider for Fake {
        fn name(&self) -> &str {
            "IGDB"
        }
        fn is_enabled(&self) -> bool {
            true
        }
        fn search(&self, _: &MetadataLookup) -> Vec<MetadataMatch> {
            vec![
                MetadataMatch {
                    year: Some(2018),
                    ..MetadataMatch::new("IGDB", "113112", "Hades")
                },
                MetadataMatch::new("IGDB", "7", "Hades II"),
            ]
        }
        fn find_exact(&self, _: &MetadataLookup) -> Option<String> {
            None
        }
        fn fetch(&self, id: &str) -> Option<GameMetadata> {
            (id == "113112").then(|| GameMetadata {
                title: Some("Hades".into()),
                ..GameMetadata::new("IGDB")
            })
        }
    }

    #[test]
    fn candidates_are_marked_and_typed_ids_are_checked_then_pinned() {
        let dir = tempfile::tempdir().unwrap();
        let (_l, mut ctx) = crate::metadata::test_ctx(dir.path(), true);
        ctx.metadata = Some(Rc::new(Metadata::offline(
            MetadataAgent::new(vec![Arc::new(Fake)], Arc::new(SystemClock)),
            basic_catalog(),
        )));
        let game = ctx
            .library
            .games()
            .iter()
            .find(|g| g.title == "Hades")
            .unwrap()
            .clone();
        let layer = panel(&ctx, &game.id);
        let o = layer.render(&ctx);
        let titles: Vec<&str> = o.rows.iter().map(|r| r.title.as_str()).collect();
        assert!(titles.contains(&"IGDB"));
        assert!(titles.contains(&"★ Hades"), "{titles:?}");
        assert!(titles.contains(&"Hades II"));
        assert!(!titles.contains(&"Automático"));

        // Texto inválido: avisa e não fixa nada.
        enter_id(&mut ctx, &game, "IGDB", "isso não é um id");
        assert!(!ctx.pinned_ids.contains_key(&game.id));
        // Endereço da página: confere na fonte e fixa.
        enter_id(&mut ctx, &game, "IGDB", "https://www.igdb.com/games/113112");
        assert!(
            !ctx.pinned_ids.contains_key(&game.id),
            "slug sem id_from_slug"
        );
        enter_id(&mut ctx, &game, "IGDB", "113112");
        assert_eq!(ctx.pinned_ids[&game.id]["IGDB"], "113112");
        assert_eq!(ctx.toast.as_ref().unwrap().0, "Hades usa Hades no IGDB");

        let o = layer.render(&ctx);
        let titles: Vec<&str> = o.rows.iter().map(|r| r.title.as_str()).collect();
        assert!(titles.contains(&"✓ Hades"), "{titles:?}");
        assert!(titles.contains(&"Automático"));
        // Id desconhecido.
        enter_id(&mut ctx, &game, "IGDB", "99");
        assert!(ctx.toast.as_ref().unwrap().0.contains("não conhece"));
        // "Nenhum" e "Automático".
        pin(&mut ctx, &game, "IGDB", Some(""), "");
        assert!(states(&ctx, &game)[0].none);
        pin(&mut ctx, &game, "IGDB", None, "");
        assert!(!states(&ctx, &game)[0].pinned);
    }
}
