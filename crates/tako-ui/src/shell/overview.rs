//! TelaBiblioteca (UX-007): a Visão geral da biblioteca do desktop. O destaque é o último jogo jogado, "Voltar a jogar"
//! traz os quatro seguintes e o resto vem só do que o Tako já sabe (sessões, instalação, arte). O que não tem fonte
//! não aparece: nenhuma métrica é inventada. Aqui só se monta o estado; o desenho é do tema (`desk-overview.slint`).

use super::{Ctx, desktop, details, tiles};
use crate::model::{DeskInfo, DeskOverview, OvItem};
use crate::values::parse_color;
use chrono::Local;
use tako_core::library::Game;
use tako_core::presentation::{game_text, library_text};

/// Arte do destaque (912 × 400 na janela de 1600): uma textura só, no tamanho em que aparece (2× para telas densas).
const HERO_ART: (u32, u32) = (1920, 840);
/// Capa de Voltar a jogar (46 × 60, 2× para telas densas).
const BACK_COVER: (u32, u32) = (92, 120);
/// Quantos jogos Voltar a jogar mostra.
const BACK: usize = 4;

/// Onde pode estar o foco do teclado e do controle, na ordem da tela: destaque, Voltar a jogar…
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Target {
    Hero,
    Back(usize),
}

impl Target {
    fn parts(self) -> (&'static str, i32) {
        match self {
            Target::Hero => ("hero", 0),
            Target::Back(i) => ("back", i as i32),
        }
    }
}

/// Jogos já jogados, o mais recente primeiro; o que está aberto agora conta como o mais recente.
fn recent(ctx: &Ctx) -> Vec<&Game> {
    let mut v: Vec<&Game> = ctx
        .library
        .games()
        .iter()
        .filter(|g| g.last_played_at.is_some())
        .collect();
    v.sort_by_key(|g| std::cmp::Reverse(g.last_played_at));
    if let Some(i) = ctx
        .running
        .as_ref()
        .and_then(|r| v.iter().position(|g| g.id == r.id))
    {
        let g = v.remove(i);
        v.insert(0, g);
    }
    v
}

/// O jogo do destaque: o último jogado; sem nenhum jogado, o primeiro instalado e, por fim, o primeiro da biblioteca.
pub(super) fn hero(ctx: &Ctx) -> Option<&Game> {
    recent(ctx)
        .first()
        .copied()
        .or_else(|| ctx.library.games().iter().find(|g| g.is_installed))
        .or_else(|| ctx.library.games().first())
}

fn back(ctx: &Ctx) -> Vec<&Game> {
    let hero = hero(ctx).map(|g| g.id.as_str());
    recent(ctx)
        .into_iter()
        .filter(|g| Some(g.id.as_str()) != hero)
        .take(BACK)
        .collect()
}

/// O que se pode focar, na ordem da tela.
pub(super) fn targets(ctx: &Ctx) -> Vec<Target> {
    let mut v = Vec::new();
    if hero(ctx).is_some() {
        v.push(Target::Hero);
        v.extend((0..back(ctx).len()).map(Target::Back));
    }
    v
}

/// O jogo de um alvo do foco.
pub(super) fn game_of(ctx: &Ctx, target: Target) -> Option<&Game> {
    match target {
        Target::Hero => hero(ctx),
        Target::Back(i) => back(ctx).get(i).copied(),
    }
}

/// "Hoje, 04:57", "Ontem, 22:10" ou "21 out".
fn when(ctx: &Ctx, g: &Game) -> Option<String> {
    let at = g.last_played_at?.with_timezone(&Local);
    let recent = at.date_naive() >= Local::now().date_naive().pred_opt()?;
    let day = desktop::day(ctx, at);
    Some(if recent {
        format!("{day}, {}", at.format("%H:%M"))
    } else {
        day
    })
}

fn back_item(ctx: &Ctx, g: &Game) -> OvItem {
    let detail = [
        when(ctx, g),
        (!g.play_time.is_zero()).then(|| game_text::play_time(ctx.lang(), g.play_time)),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ");
    OvItem {
        id: g.id.clone(),
        title: g.title.clone(),
        detail,
        action: if g.is_installed {
            ctx.tf("desktop.home.play_game", &[("title", &g.title)])
        } else {
            String::new()
        },
        thumb: tiles::image(g.shown_cover(), BACK_COVER),
        accent: parse_color(&g.accent),
        playable: g.is_installed,
        ..Default::default()
    }
}

/// "13 jogos · 598 h jogadas" (sem o tempo, só a contagem).
fn count(ctx: &Ctx) -> String {
    let games = ctx.library.games();
    let games_text = library_text::game_count(ctx.lang(), games.len());
    let total: std::time::Duration = games.iter().map(|g| g.play_time).sum();
    if total.is_zero() {
        return games_text;
    }
    ctx.tf(
        "desktop.home.summary",
        &[
            ("games", &games_text),
            ("time", &game_text::play_time(ctx.lang(), total)),
        ],
    )
}

/// A etiqueta de situação do destaque: Jogando, Quebrado, Continuar jogando ou a situação do jogo.
fn badge(ctx: &Ctx, g: &Game, running: bool) -> String {
    if running {
        ctx.t("desktop.badge.playing")
    } else if g.is_broken {
        ctx.t("desktop.badge.broken")
    } else if g.last_played_at.is_some() {
        ctx.t("desktop.home.continue")
    } else {
        game_text::status(ctx.lang(), g)
    }
    .to_uppercase()
}

/// Arte clara atrás da barra de dados e da identidade (embaixo): pede mais escurecimento (EstadoArteClara). Mesmo
/// limite do vidro (0,18).
fn bright(lum: &[f32]) -> bool {
    let (cols, rows) = crate::images::GLASS_GRID;
    if lum.len() != (cols * rows) as usize {
        return false;
    }
    let bottom = &lum[(cols * (rows - 3)) as usize..];
    bottom.iter().sum::<f32>() / bottom.len() as f32 > 0.18
}

/// O estado da Visão geral. `focus` é o alvo com o foco do teclado (se a zona estiver aqui).
pub(super) fn build(ctx: &Ctx, focus: Option<Target>) -> DeskOverview {
    let name = super::profiles::name(
        ctx,
        &tako_core::profiles::Profiles::load(&ctx.paths.machine).active(),
    );
    let loading = ctx.scanning && ctx.library.games().is_empty();
    let mut o = DeskOverview {
        on: true,
        loading,
        empty: !ctx.scanning && ctx.library.games().is_empty(),
        title: ctx.tf("desktop.home.welcome", &[("name", &name)]),
        count: if loading {
            ctx.t("desktop.empty.loading")
        } else {
            count(ctx)
        },
        back_title: ctx.t("desktop.home.back_title"),
        ..Default::default()
    };
    if let Some((kind, index)) = focus.map(Target::parts) {
        o.focus_kind = kind.into();
        o.focus_index = index;
    }
    let Some(g) = hero(ctx) else {
        return o;
    };
    let running = ctx.running.as_ref().is_some_and(|r| r.id == g.id);
    let install = details::install_stage(ctx, g);
    let action = desktop::play_row(ctx, g, running, &install);
    // A cópia de vidro também amostra a cor da arte (o destaque pinta o TAKO com ela) e mede a luz embaixo.
    let lum = g
        .shown_hero()
        .and_then(crate::images::glass)
        .map(|(_, lum)| lum)
        .unwrap_or_default();
    let logo = tiles::image(g.shown_logo(), tiles::LOGO);
    o.hero = crate::model::DeskGameView {
        has_game: true,
        title: g.title.clone(),
        overline: badge(ctx, g, running),
        description: desktop::about(&g.description).0,
        art: tiles::image(g.shown_hero(), HERO_ART),
        show_logo: desktop::show_logo(ctx, g, &logo, &lum),
        title_pending: desktop::title_pending(ctx, g, &logo, &lum),
        logo,
        accent: parse_color(&g.accent),
        running,
        ..Default::default()
    };
    o.hero_on = running || (g.is_installed && !g.is_broken);
    o.hero_bright = bright(&lum);
    o.strip = desktop::strip(ctx, g)
        .into_iter()
        .collect::<Vec<DeskInfo>>();
    o.action = action.title;
    o.action_icon = action.icon;
    o.action_enabled = action.enabled;
    o.back = back(ctx).into_iter().map(|g| back_item(ctx, g)).collect();
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::Shell;

    fn shell() -> (tempfile::TempDir, Shell) {
        let dir = tempfile::tempdir().unwrap();
        let ctx = crate::app::build_ctx(
            tako_core::paths::DataPaths::at(dir.path()),
            tako_platform::Platform::simulated(),
            true,
        );
        (dir, Shell::new(ctx))
    }

    /// Os seis primeiros jogos jogados a cada hora (o 0 é o mais recente) e o resto sem jogar.
    fn played(shell: &mut Shell) -> Vec<String> {
        let now = chrono::Utc::now();
        let ids = {
            let mut found = shell.ctx.found.lock().unwrap();
            for (n, g) in found.iter_mut().enumerate() {
                g.last_played_at = (n < 6).then(|| now - chrono::Duration::hours(n as i64));
            }
            found.iter().map(|g| g.id.clone()).collect()
        };
        shell.ctx.library.refresh();
        ids
    }

    #[test]
    fn hero_is_the_last_played_and_back_lists_the_next_four() {
        let (_dir, mut shell) = shell();
        let ids = played(&mut shell);
        let ctx = &shell.ctx;
        assert_eq!(hero(ctx).map(|g| g.id.as_str()), Some(ids[0].as_str()));
        let back_ids: Vec<&str> = back(ctx).iter().map(|g| g.id.as_str()).collect();
        assert_eq!(back_ids, [&ids[1], &ids[2], &ids[3], &ids[4]]);
        assert_eq!(targets(ctx).len(), 1 + BACK);
        let second = game_of(ctx, Target::Back(1)).map(|g| g.id.as_str());
        assert_eq!(second, Some(ids[2].as_str()));
        let o = build(ctx, Some(Target::Back(2)));
        assert_eq!((o.focus_kind.as_str(), o.focus_index), ("back", 2));
        assert_eq!(o.back.len(), BACK);
        assert!(!o.empty && !o.loading && o.hero.has_game);
    }

    #[test]
    fn running_game_takes_the_hero_and_empty_library_has_no_targets() {
        let (_dir, mut shell) = shell();
        let ids = played(&mut shell);
        shell.ctx.running = Some(crate::shell::ctx::RunningGame {
            id: ids[3].clone(),
            title: "x".into(),
            pid: None,
            started: None,
        });
        assert_eq!(
            hero(&shell.ctx).map(|g| g.id.as_str()),
            Some(ids[3].as_str())
        );
        shell.ctx.found.lock().unwrap().clear();
        shell.ctx.library.refresh();
        assert!(targets(&shell.ctx).is_empty());
        let o = build(&shell.ctx, None);
        assert!(o.empty && !o.hero.has_game);
    }

    #[test]
    fn bright_art_is_measured_at_the_bottom_of_the_light_grid() {
        let (cols, rows) = crate::images::GLASS_GRID;
        let mut lum = vec![0.0; (cols * rows) as usize];
        assert!(!bright(&lum));
        assert!(!bright(&[]), "sem a medida, nada de escurecer a mais");
        for v in lum.iter_mut().skip((cols * (rows - 3)) as usize) {
            *v = 0.5;
        }
        assert!(bright(&lum));
    }
}
