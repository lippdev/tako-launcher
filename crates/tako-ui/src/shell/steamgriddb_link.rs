//! ART-002: o botão "BOOP" do site do SteamGridDB (ou `tako://steamgriddb/…`) aplica a imagem num jogo. Quando
//! o app da Steam da imagem está na biblioteca, aplica direto; senão pergunta em qual jogo, com os de nome
//! parecido primeiro. A imagem é fixada como as escolhidas no seletor (e pode voltar à automática por lá).

use super::Ctx;
use super::art;
use super::layer::{Flow, MenuItem, MenuLayer};
use crate::model::PanelRow;
use tako_core::links::SteamGridDbAsset;
use tako_core::metadata::art::{ArtCandidate, kind_label};
use tako_core::metadata::sources::STEAMGRIDDB;
use tako_core::metadata::steamgriddb::{
    BoopImage, BoopProblem, SteamGridDbFailure, by_id, parse_boop, parse_games,
};
use tako_core::metadata::title_matcher;

/// Por que a imagem não veio (traduzido na thread da interface).
#[derive(Debug, PartialEq)]
enum Problem {
    Api(SteamGridDbFailure),
    Missing,
    Boop(BoopProblem),
}

pub fn open(ctx: &mut Ctx, asset: SteamGridDbAsset) {
    let Some(provider) = ctx
        .metadata
        .as_ref()
        .and_then(|m| m.catalog.steamgriddb.clone())
    else {
        ctx.toast(SteamGridDbFailure::NotConfigured.message());
        return;
    };
    ctx.toast(ctx.t("links.steamgriddb.loading"));
    crate::tasks::run(
        ctx,
        move || {
            let client = provider.client();
            let text = client
                .boop(&asset)
                .map_err(|e| Problem::Api(e.failure))?
                .ok_or(Problem::Missing)?;
            let image = parse_boop(&text).map_err(Problem::Boop)?;
            // Fora da Steam o site dá o jogo do SteamGridDB: o nome dele ordena a lista de jogos.
            let title = image
                .game
                .as_deref()
                .and_then(|id| client.get(&by_id(id), true).ok().flatten())
                .and_then(|d| parse_games(&d).into_iter().next())
                .map(|m| m.title);
            Ok((image, title))
        },
        |ctx, result: Result<(BoopImage, Option<String>), Problem>| match result {
            Ok((image, title)) => apply_or_ask(ctx, image, title.as_deref()),
            Err(p) => {
                tracing::warn!("imagem do SteamGridDB não aplicada: {p:?}");
                let text = match p {
                    Problem::Api(f) => f.message(),
                    Problem::Missing => ctx.t("links.steamgriddb.missing"),
                    Problem::Boop(BoopProblem::Horizontal) => ctx.t("links.steamgriddb.horizontal"),
                    Problem::Boop(BoopProblem::Unexpected) => ctx.t("links.steamgriddb.unexpected"),
                };
                ctx.toast(text);
            }
        },
    );
}

fn apply_or_ask(ctx: &mut Ctx, image: BoopImage, title: Option<&str>) {
    let candidate = ArtCandidate::new(STEAMGRIDDB, image.kind, image.url);
    if let Some(app) = image.steam_app {
        let id = format!("steam:{app}");
        if ctx.library.find(&id).is_some() {
            art::apply(ctx, &id, &candidate);
            return;
        }
    }
    if ctx.library.games().is_empty() {
        ctx.toast(ctx.t("links.missing_game"));
        return;
    }
    ctx.pending_layer = Some(Box::new(chooser(ctx, candidate, title)));
}

/// Jogos da biblioteca: o de nome mais parecido primeiro e, sem nome, o que está aberto nos detalhes.
fn ordered_games(ctx: &Ctx, title: Option<&str>) -> Vec<(String, String)> {
    let mut games: Vec<(f64, String, String)> = ctx
        .library
        .games()
        .iter()
        .map(|g| {
            let mut score = title.map_or(0., |t| title_matcher::score(&g.title, t));
            if ctx.open_details.as_deref() == Some(g.id.as_str()) {
                score += 0.5;
            }
            (score, g.title.clone(), g.id.clone())
        })
        .collect();
    games.sort_by(|a, b| {
        b.0.total_cmp(&a.0)
            .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
    });
    games.into_iter().map(|(_, t, id)| (id, t)).collect()
}

fn chooser(ctx: &Ctx, candidate: ArtCandidate, title: Option<&str>) -> MenuLayer {
    let games = ordered_games(ctx, title);
    let kind = candidate.kind.map(kind_label).unwrap_or_default();
    MenuLayer::panel(
        ctx.tf("links.steamgriddb.pick", &[("kind", &kind)]),
        move |_| {
            games
                .iter()
                .map(|(id, name)| {
                    let (id, c) = (id.clone(), candidate.clone());
                    MenuItem::new(PanelRow::action(id.clone(), name.clone())).on(move |ctx| {
                        art::apply(ctx, &id, &c);
                        Flow::Close
                    })
                })
                .collect()
        },
    )
    .text(title.map(str::to_owned).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tako_core::library::ArtKind;

    fn image(steam_app: Option<u32>) -> BoopImage {
        BoopImage {
            steam_app,
            game: None,
            kind: ArtKind::Cover,
            url: "https://cdn2.steamgriddb.com/grid/a.png".into(),
        }
    }

    #[test]
    fn a_steam_game_in_the_library_gets_the_image_in_one_click() {
        let dir = tempfile::tempdir().unwrap();
        let (_guard, mut ctx) = crate::metadata::test_ctx(dir.path(), true);
        ctx.library.refresh();
        ctx.library.games_mut()[0].id = "steam:570".into();
        apply_or_ask(&mut ctx, image(Some(570)), None);
        assert!(ctx.pending_layer.is_none());
        let game = ctx.library.find("steam:570").unwrap();
        assert_eq!(
            tako_core::metadata::art::pins::shown(game, ArtKind::Cover),
            Some("https://cdn2.steamgriddb.com/grid/a.png")
        );
        assert!(ctx.toast.is_some());
    }

    #[test]
    fn otherwise_it_asks_which_game_with_the_closest_title_first() {
        let dir = tempfile::tempdir().unwrap();
        let (_guard, mut ctx) = crate::metadata::test_ctx(dir.path(), true);
        ctx.library.refresh();
        let target = ctx.library.games().last().unwrap().clone();
        apply_or_ask(&mut ctx, image(Some(1)), Some(&target.title));
        let mut layer = ctx.pending_layer.take().expect("pergunta o jogo");
        let rows = layer.render(&ctx).rows;
        assert_eq!(rows.len(), ctx.library.games().len());
        assert_eq!(rows[0].key, target.id);
        layer.handle(crate::shell::Cmd::Accept, &mut ctx);
        let game = ctx.library.find(&target.id).unwrap();
        assert_eq!(
            tako_core::metadata::art::pins::shown(game, ArtKind::Cover),
            Some("https://cdn2.steamgriddb.com/grid/a.png")
        );
    }
}
