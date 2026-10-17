//! ART-001: "Revisar capas". As fontes casam a biblioteca inteira sozinhas (capa, fundo, logo e ícone); aqui ficam
//! só os jogos que valem conferir: sem algum tipo de imagem ou achados só pelo nome, com outro nome. Cada um abre
//! o seletor de imagens, o "Fontes e IDs" ou confirma que está certo (fixa o id achado).

use super::layer::{Flow, MenuItem, MenuLayer};
use super::{Ctx, Effect};
use crate::metadata::{Pending, spawn};
use crate::model::PanelRow;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use tako_core::library::Game;
use tako_core::metadata::agent::MetadataAgent;
use tako_core::metadata::art::kind_label;
use tako_core::metadata::review::{Review, review};

type Found = Vec<(Game, Review)>;

/// Lê o cache de cada jogo numa thread: a biblioteca pode ter milhares.
fn compute(ctx: &Ctx) -> Pending<(usize, Found)> {
    let games = ctx.library.games().to_vec();
    let cache = crate::metadata::cache(ctx);
    spawn(move || {
        let total = games.len();
        let mut found: Found = games
            .into_iter()
            .filter_map(|g| {
                let cached = MetadataAgent::normalize(cache.read(&g.id));
                let r = review(&g, cached.as_ref());
                r.needs_attention().then_some((g, r))
            })
            .collect();
        found.sort_by(|a, b| {
            a.1.rank()
                .cmp(&b.1.rank())
                .then_with(|| a.0.title.to_lowercase().cmp(&b.0.title.to_lowercase()))
        });
        (total, found)
    })
}

fn describe(ctx: &Ctx, r: &Review) -> String {
    if let Some(d) = &r.doubt {
        return ctx.tf(
            "art_review.doubt",
            &[("source", &d.source), ("found", &d.found)],
        );
    }
    let kinds: Vec<String> = r
        .missing
        .iter()
        .map(|k| kind_label(*k).to_lowercase())
        .collect();
    ctx.tf("art_review.missing", &[("kinds", &kinds.join(", "))])
}

struct State {
    pending: Pending<(usize, Found)>,
    /// Jogos já tratados aqui: saem da lista sem esperar a nova consulta.
    done: HashSet<String>,
}

/// Configurações › Metadados › Revisar capas.
pub fn panel(ctx: &Ctx) -> MenuLayer {
    let state = Rc::new(RefCell::new(State {
        pending: compute(ctx),
        done: HashSet::new(),
    }));
    MenuLayer::panel(ctx.t("art_review.title"), move |ctx| {
        let s = state.borrow();
        let refresh = MenuItem::new(
            PanelRow::action("refresh", ctx.t("art_review.refresh"))
                .desc(ctx.t("art_review.refresh_desc")),
        )
        .on(|ctx| {
            ctx.effect(Effect::RefreshMetadata(None));
            Flow::Stay
        });
        let Some((total, found)) = s.pending.get() else {
            return vec![
                MenuItem::new(PanelRow::header(ctx.t("art_review.computing"))),
                refresh,
            ];
        };
        let left: Vec<&(Game, Review)> = found
            .iter()
            .filter(|(g, _)| !s.done.contains(&g.id))
            .collect();
        let total = total.to_string();
        let summary = if left.is_empty() {
            ctx.tf("art_review.all_good", &[("total", &total)])
        } else {
            ctx.tf(
                "art_review.summary",
                &[("count", &left.len().to_string()), ("total", &total)],
            )
        };
        let mut v = vec![MenuItem::new(PanelRow::header(summary)), refresh];
        for (game, r) in left {
            let (game, r, state) = (game.clone(), r.clone(), state.clone());
            v.push(
                MenuItem::new(
                    PanelRow::action(game.id.clone(), game.title.clone()).desc(describe(ctx, &r)),
                )
                .on(move |ctx| Flow::Push(Box::new(game_dialog(ctx, &game, &r, state.clone())))),
            );
        }
        v
    })
    .text(ctx.t("art_review.text"))
}

fn game_dialog(ctx: &Ctx, game: &Game, r: &Review, state: Rc<RefCell<State>>) -> MenuLayer {
    let (game, r) = (game.clone(), r.clone());
    let text = describe(ctx, &r);
    MenuLayer::dialog(game.title.clone(), move |ctx| {
        let mark = {
            let (state, id) = (state.clone(), game.id.clone());
            move || {
                state.borrow_mut().done.insert(id.clone());
            }
        };
        let (g1, g2, m1, m2) = (game.clone(), game.clone(), mark.clone(), mark.clone());
        let mut v = vec![
            MenuItem::new(PanelRow::button("art", ctx.t("art_review.pick_art"))).on(move |ctx| {
                m1();
                Flow::Replace(super::art::picker(ctx, &g1.id))
            }),
            MenuItem::new(PanelRow::button("match", ctx.t("art_review.fix_match"))).on(
                move |ctx| {
                    m2();
                    Flow::Replace(super::source_ids::panel(ctx, &g2.id))
                },
            ),
        ];
        if let Some(d) = r.doubt.clone() {
            let (g3, m3) = (game.clone(), mark.clone());
            v.push(
                MenuItem::new(PanelRow::button("ok", ctx.t("art_review.correct"))).on(move |ctx| {
                    m3();
                    super::source_ids::pin(ctx, &g3, &d.source, Some(&d.id), &d.found);
                    Flow::Close
                }),
            );
        }
        v.push(
            MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel"))).on(|_| Flow::Close),
        );
        v
    })
    .text(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::Cmd;
    use crate::shell::layer::Layer;
    use tako_core::metadata::GameMetadata;

    #[test]
    fn lists_what_to_check_and_confirming_a_doubt_pins_the_found_id() {
        let dir = tempfile::tempdir().unwrap();
        let (_guard, mut ctx) = crate::metadata::test_ctx(dir.path(), true);
        ctx.library.refresh();
        let game = ctx.library.games()[0].clone();
        let part = GameMetadata {
            source_ids: [("IGDB".to_owned(), "77".to_owned())].into(),
            title: Some(format!("{} Remaster", game.title)),
            matched_by_title: true,
            ..GameMetadata::new("IGDB")
        };
        let cached = GameMetadata {
            parts: vec![part],
            ..GameMetadata::new("IGDB")
        };
        crate::metadata::cache(&ctx)
            .write(&game.id, &cached)
            .unwrap();

        let mut layer = panel(&ctx);
        let rows = layer.render(&ctx).rows;
        assert!(
            rows[0]
                .title
                .contains(&ctx.library.games().len().to_string())
        );
        assert_eq!(rows[1].key, "refresh");
        // A dúvida pelo nome vem antes de quem só não tem logo ou ícone.
        let at = rows
            .iter()
            .position(|r| r.key == game.id)
            .expect("o jogo duvidoso");
        assert!(rows[at].description.contains("Remaster"));

        // Abre o jogo pela lista: Usar imagens, Fontes e IDs, Está certo, Cancelar.
        for _ in 1..at {
            layer.handle(Cmd::Down, &mut ctx);
        }
        let Flow::Push(mut dialog) = layer.handle(Cmd::Accept, &mut ctx) else {
            panic!("abre o jogo");
        };
        let keys: Vec<String> = dialog
            .render(&ctx)
            .rows
            .into_iter()
            .map(|r| r.key)
            .collect();
        assert_eq!(keys, ["art", "match", "ok", "cancel"]);
        for _ in 0..2 {
            dialog.handle(Cmd::Down, &mut ctx);
        }
        assert!(matches!(dialog.handle(Cmd::Accept, &mut ctx), Flow::Close));
        assert_eq!(
            ctx.pinned_ids
                .get(&game.id)
                .and_then(|p| p.get("IGDB"))
                .map(String::as_str),
            Some("77")
        );
        // O jogo tratado sai da lista sem esperar a nova consulta.
        let rows = layer.render(&ctx).rows;
        assert!(!rows.iter().any(|r| r.key == game.id));
    }
}
