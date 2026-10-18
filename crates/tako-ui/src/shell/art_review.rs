//! ART-001: "Revisar capas". As fontes casam a biblioteca inteira sozinhas (capa, fundo, logo e ícone); aqui ficam
//! só os jogos que valem conferir: sem algum tipo de imagem ou achados só pelo nome, com outro nome. Cada um abre
//! o seletor de imagens, o "Fontes e IDs" ou confirma que está certo (fixa o id achado).

use super::layer::{Flow, MenuItem, MenuLayer};
use super::{Ctx, Effect};
use crate::metadata::{Pending, spawn};
use crate::model::{CoverRow, PanelRow};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use tako_core::library::Game;
use tako_core::metadata::agent::MetadataAgent;
use tako_core::metadata::art::kind_label;
use tako_core::metadata::review::{Confidence, Doubt, Review, review};

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

/// Quantas linhas da fila o desktop desenha de uma vez (as Configurações não viram lista virtual).
const SHOWN: usize = 30;

/// Fila de capas do desktop (FilaDeCapas): lida uma vez numa thread, fica no `Ctx` (a lista não some ao trocar de
/// categoria) e cada jogo tratado sai dela na hora.
#[derive(Default)]
pub struct Queue {
    found: RefCell<Option<Pending<(usize, Found)>>>,
    done: RefCell<HashSet<String>>,
}

impl Queue {
    /// `(total de jogos, o que falta revisar)`; `None` enquanto a biblioteca é conferida.
    fn left(&self, ctx: &Ctx) -> Option<(usize, Found)> {
        let (total, found) = self
            .found
            .borrow_mut()
            .get_or_insert_with(|| compute(ctx))
            .get()?;
        let done = self.done.borrow();
        Some((
            total,
            found
                .into_iter()
                .filter(|(g, _)| !done.contains(&g.id))
                .collect(),
        ))
    }

    /// "Buscar de novo": conferir tudo outra vez e esquecer o que já foi tratado.
    pub fn reset(&self) {
        *self.found.borrow_mut() = None;
        self.done.borrow_mut().clear();
    }
}

/// Quantos jogos esperam revisão (para a contagem no item da categoria); `None` sem nenhum ou ainda conferindo.
pub fn pending(ctx: &Ctx) -> Option<usize> {
    ctx.art_review
        .left(ctx)
        .map(|(_, f)| f.len())
        .filter(|n| *n > 0)
}

/// Os que o "Aceitar as N" troca de uma vez: só confiança alta.
fn high_confidence(found: &[(Game, Review)]) -> Vec<&(Game, Review)> {
    found
        .iter()
        .filter(|(_, r)| {
            r.doubt
                .as_ref()
                .is_some_and(|d| d.confidence() == Confidence::High)
        })
        .collect()
}

/// Aceita a correspondência (fixa o id da fonte) e devolve o id fixado antes, para o Desfazer.
fn accept(ctx: &mut Ctx, game: &Game, d: &Doubt) -> Option<String> {
    let before = ctx
        .pinned_ids
        .get(&game.id)
        .and_then(|p| p.get(&d.source))
        .cloned();
    super::source_ids::pin(ctx, game, &d.source, Some(&d.id), &d.found);
    ctx.art_review.done.borrow_mut().insert(game.id.clone());
    before
}

/// Aceita e avisa com "Desfazer": o desfazer volta o id de antes e a linha volta para a fila.
fn accept_with_undo(ctx: &mut Ctx, picks: Vec<(Game, Doubt)>) {
    let undo: Vec<(Game, String, Option<String>)> = picks
        .iter()
        .map(|(g, d)| (g.clone(), d.source.clone(), accept(ctx, g, d)))
        .collect();
    let text = match undo.as_slice() {
        [(g, ..)] => ctx.tf("art_review.accepted", &[("game", &g.title)]),
        many => ctx.tf(
            "art_review.accepted_many",
            &[("n", &many.len().to_string())],
        ),
    };
    ctx.toast_with_undo(text, move |ctx| {
        for (game, source, before) in &undo {
            super::source_ids::pin(ctx, game, source, before.as_deref(), "");
            ctx.art_review.done.borrow_mut().remove(&game.id);
        }
    });
}

fn confidence_label(ctx: &Ctx, d: &Doubt) -> String {
    let key = match d.confidence() {
        Confidence::High => "art_review.conf_high",
        Confidence::Medium => "art_review.conf_medium",
        Confidence::Low => "art_review.conf_low",
    };
    ctx.tf(key, &[("percent", &d.percent().to_string())])
}

/// Desktop (FilaDeCapas): "Aceitar as N" de confiança alta e uma linha por jogo, com a capa atual, a proposta, a
/// confiança escrita e as ações ali mesmo.
pub fn queue_rows(ctx: &Ctx) -> Vec<MenuItem> {
    let mut v = vec![MenuItem::new(PanelRow::header(ctx.t("art_review.title")))];
    // Conferindo a biblioteca (uma vez, em segundo plano): sem seção até terminar.
    let Some((total, found)) = ctx.art_review.left(ctx) else {
        return Vec::new();
    };
    if found.is_empty() {
        v.push(MenuItem::new(PanelRow::info(
            "review-ok",
            ctx.tf("art_review.all_good", &[("total", &total.to_string())]),
            "",
        )));
        return v;
    }
    let high: Vec<(Game, Doubt)> = high_confidence(&found)
        .into_iter()
        .filter_map(|(g, r)| Some((g.clone(), r.doubt.clone()?)))
        .collect();
    if !high.is_empty() {
        let n = high.len().to_string();
        v.push(
            MenuItem::new(
                PanelRow::action("accept-all", ctx.tf("art_review.high_title", &[("n", &n)]))
                    .desc(ctx.t("art_review.high_desc"))
                    .value(ctx.tf("art_review.accept_all", &[("n", &n)])),
            )
            .on(move |ctx| {
                accept_with_undo(ctx, high.clone());
                Flow::Stay
            }),
        );
    }
    v.push(MenuItem::new(PanelRow::header(ctx.tf(
        "art_review.to_review",
        &[("n", &found.len().to_string())],
    ))));
    for (game, r) in found.iter().take(SHOWN) {
        v.push(queue_row(ctx, game, r));
    }
    if found.len() > SHOWN {
        v.push(MenuItem::new(PanelRow::info(
            "review-more",
            ctx.tf(
                "art_review.more",
                &[
                    ("shown", &SHOWN.to_string()),
                    ("n", &found.len().to_string()),
                ],
            ),
            "",
        )));
    }
    v
}

fn queue_row(ctx: &Ctx, game: &Game, r: &Review) -> MenuItem {
    let cover = game.shown_cover();
    let proposed = r.doubt.as_ref().and_then(|d| d.cover.as_deref());
    let info = CoverRow {
        current: crate::shell::tiles::image(cover, (96, 144)),
        has_current: cover.is_some(),
        proposed: crate::shell::tiles::image(proposed, (128, 192)),
        has_proposed: proposed.is_some(),
        current_label: ctx.t("art_review.no_cover"),
        level: r.doubt.as_ref().map_or(0, |d| match d.confidence() {
            Confidence::High => 3,
            Confidence::Medium => 2,
            Confidence::Low => 1,
        }),
        confidence: r
            .doubt
            .as_ref()
            .map_or_else(String::new, |d| confidence_label(ctx, d)),
        reason: describe(ctx, r),
        accept: if r.doubt.is_some() {
            ctx.t("art_review.accept")
        } else {
            String::new()
        },
        other: ctx.t("art_review.other"),
    };
    let row = PanelRow::new(game.id.clone(), "cover", game.title.clone()).desc(
        tako_core::presentation::game_text::store_name(ctx.lang(), game.store()),
    );
    let id = game.id.clone();
    let pick = move |ctx: &mut Ctx| {
        ctx.art_review.done.borrow_mut().insert(id.clone());
        Flow::Push(super::art::picker(ctx, &id))
    };
    let item = MenuItem::new(row).cover(info, pick.clone());
    match r.doubt.clone() {
        // Enter na linha aceita; sem proposta, escolhe a imagem.
        Some(d) => {
            let game = game.clone();
            item.on(move |ctx| {
                accept_with_undo(ctx, vec![(game.clone(), d.clone())]);
                Flow::Stay
            })
        }
        None => item.on(pick),
    }
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

    fn doubt(score: f64) -> Review {
        Review {
            doubt: Some(Doubt {
                source: "IGDB".into(),
                id: "77".into(),
                found: "x".into(),
                score,
                cover: None,
            }),
            ..Default::default()
        }
    }

    #[test]
    fn accept_all_takes_only_high_confidence_and_undo_restores_the_queue() {
        let found: Found = [0.95, 0.7, 0.2, 0.9]
            .into_iter()
            .map(|s| (Game::default(), doubt(s)))
            .chain([(Game::default(), Review::default())])
            .collect();
        let high: Vec<f64> = high_confidence(&found)
            .iter()
            .filter_map(|(_, r)| r.doubt.as_ref().map(|d| d.score))
            .collect();
        assert_eq!(high, [0.95, 0.9]);

        let dir = tempfile::tempdir().unwrap();
        let (_guard, mut ctx) = crate::metadata::test_ctx(dir.path(), true);
        ctx.library.refresh();
        let game = ctx.library.games()[0].clone();
        let part = GameMetadata {
            source_ids: [("IGDB".to_owned(), "77".to_owned())].into(),
            title: Some(format!("{} Remaster", game.title)),
            matched_by_title: true,
            cover_url: Some("https://x/c.png".into()),
            ..GameMetadata::new("IGDB")
        };
        let cached = GameMetadata {
            parts: vec![part],
            ..GameMetadata::new("IGDB")
        };
        crate::metadata::cache(&ctx)
            .write(&game.id, &cached)
            .unwrap();
        ctx.desktop = true;

        let rows = queue_rows(&ctx);
        let row = rows.iter().find(|r| r.row.key == game.id).expect("na fila");
        let cover = row.cover.as_ref().unwrap();
        assert!(cover.has_proposed && cover.level > 0 && cover.confidence.contains('%'));
        assert_eq!(
            pending(&ctx),
            Some(rows.iter().filter(|r| r.cover.is_some()).count())
        );

        // Aceitar fixa o id, tira a linha e avisa com Desfazer.
        let accept = row.accept.clone().unwrap();
        accept(&mut ctx);
        assert_eq!(ctx.pinned_ids[&game.id]["IGDB"], "77");
        assert!(ctx.toast_undo.is_some());
        assert!(!queue_rows(&ctx).iter().any(|r| r.row.key == game.id));

        // Desfazer volta o id de antes e a linha volta para a fila.
        let undo = ctx.toast_undo.clone().unwrap();
        undo(&mut ctx);
        assert!(
            ctx.pinned_ids
                .get(&game.id)
                .is_none_or(|p| !p.contains_key("IGDB"))
        );
        assert!(queue_rows(&ctx).iter().any(|r| r.row.key == game.id));
    }
}
