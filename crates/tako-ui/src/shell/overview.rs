//! TelaBiblioteca (UX-007): a Visão geral da biblioteca do desktop. O destaque é o último jogo jogado, "Voltar a jogar"
//! traz os quatro seguintes e o resto vem só do que o Tako já sabe (sessões, instalação, arte). O que não tem fonte
//! não aparece: nenhuma métrica é inventada. Aqui só se monta o estado; o desenho é do tema (`desk-overview.slint`).

use super::{Ctx, desktop, details, profile, tiles};
use crate::model::{DeskBar, DeskInfo, DeskOverview, OvItem};
use crate::values::parse_color;
use chrono::{Datelike, Local};
use std::time::Duration;
use tako_core::activities::State;
use tako_core::library::{Game, PlaySession};
use tako_core::presentation::{game_text, library_text};

/// Arte do destaque (912 × 400 na janela de 1600): uma textura só, no tamanho em que aparece (2× para telas densas).
const HERO_ART: (u32, u32) = (1920, 840);
/// Uma textura por capa; o Slint reduz conforme o tamanho de cada peça.
const COVER: (u32, u32) = (400, 600);
/// Quantos jogos Voltar a jogar mostra.
const BACK: usize = 4;
// shortcut: a fileira Instalados carrega 12 capas (uma textura cada) e as setas só rolam essas; subir se alguém
// sentir falta da biblioteca inteira ali (a Grade já a tem).
const SHELF: usize = 12;
/// Linhas de Precisa de atenção na tela; o resto fica na contagem.
const ATTENTION_ROWS: usize = 4;

/// Onde pode estar o foco do teclado e do controle, na ordem da tela: destaque, Voltar a jogar…
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Target {
    Hero,
    Back(usize),
    /// O Jogo da semana, dentro de Sua semana.
    Best,
    Attention(usize),
    /// Uma capa da fileira Instalados.
    Cover(usize),
}

impl Target {
    fn parts(self) -> (&'static str, i32) {
        match self {
            Target::Hero => ("hero", 0),
            Target::Back(i) => ("back", i as i32),
            Target::Best => ("best", 0),
            Target::Attention(i) => ("attention", i as i32),
            Target::Cover(i) => ("cover", i as i32),
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
pub(super) fn targets(ctx: &Ctx, show_back: bool) -> Vec<Target> {
    let mut v = Vec::new();
    if hero(ctx).is_some() {
        v.push(Target::Hero);
        if show_back {
            v.extend((0..back(ctx).len()).map(Target::Back));
        }
        v.extend(week(ctx).and_then(|w| w.best).map(|_| Target::Best));
        v.extend((0..attention(ctx).len().min(ATTENTION_ROWS)).map(Target::Attention));
        v.extend((0..installed(ctx).len()).map(Target::Cover));
    }
    v
}

/// O jogo de um alvo do foco (um erro de segundo plano em Precisa de atenção não tem jogo).
pub(super) fn game_of(ctx: &Ctx, target: Target) -> Option<&Game> {
    match target {
        Target::Hero => hero(ctx),
        Target::Back(i) => back(ctx).get(i).copied(),
        Target::Best => week(ctx)
            .and_then(|w| w.best)
            .and_then(|b| ctx.library.find(&b.id)),
        Target::Attention(i) => attention(ctx)
            .get(i)
            .filter(|a| a.state != "error")
            .and_then(|a| ctx.library.find(&a.id)),
        Target::Cover(i) => installed(ctx).get(i).copied(),
    }
}

/// A linha de Precisa de atenção de um alvo (`id` é o jogo, ou a chave da atividade no erro de segundo plano).
pub(super) fn attention_of(ctx: &Ctx, index: usize) -> Option<OvItem> {
    attention(ctx).into_iter().nth(index)
}

/// Os instalados da fileira: os jogados há pouco primeiro, depois os outros na ordem da biblioteca.
fn installed(ctx: &Ctx) -> Vec<&Game> {
    let mut v: Vec<&Game> = ctx
        .library
        .games()
        .iter()
        .filter(|g| g.is_installed && !g.is_broken)
        .collect();
    v.sort_by_key(|g| std::cmp::Reverse(g.last_played_at));
    v.truncate(SHELF);
    v
}

/// Sua semana: o que as sessões gravadas dizem. Sem uma sessão sequer na biblioteca (ou sem jogo de verdade), o bloco
/// some; Conquistas e Capturas não entram porque o Tako não tem essas fontes.
struct Week {
    groups: Vec<Vec<DeskInfo>>,
    bars: Vec<DeskBar>,
    text: String,
    best: Option<OvItem>,
}

fn week(ctx: &Ctx) -> Option<Week> {
    let sessions = profile::all_sessions(ctx);
    let today = Local::now().date_naive();
    let start = profile::monday(today);
    let this: Vec<&(&Game, &PlaySession)> = sessions
        .iter()
        .filter(|(_, s)| profile::local_day(s) >= start && !profile::counted(s).is_zero())
        .collect();
    if this.is_empty() {
        return None;
    }
    let days = profile::week(sessions.iter().map(|(_, s)| *s), today);
    let today_i = today.weekday().num_days_from_monday() as usize;
    let max = days.iter().max().copied().unwrap_or_default();
    let bars: Vec<DeskBar> = days
        .iter()
        .enumerate()
        .map(|(i, d)| DeskBar {
            // A inicial do dia: S T Q Q S S D.
            label: ctx
                .t(&format!("profile.weekday.{}", profile::WEEKDAYS[i]))
                .chars()
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_default(),
            fraction: if max.is_zero() {
                0.0
            } else {
                d.as_secs_f32() / max.as_secs_f32()
            },
            today: i == today_i,
            future: i > today_i,
            ..Default::default()
        })
        .collect();
    let tile = |icon: &str, key: &str, value: String| DeskInfo {
        icon: icon.into(),
        label: ctx.t(key),
        value,
        ..Default::default()
    };
    let total: Duration = days.iter().sum();
    let mut played: Vec<&str> = this.iter().map(|(g, _)| g.id.as_str()).collect();
    played.sort_unstable();
    played.dedup();
    let mut groups = vec![
        vec![
            tile("clock", "desktop.home.week_time", profile::hm(total)),
            tile("bars", "desktop.home.week_chart", String::new()),
        ],
        vec![
            tile(
                "gamepad",
                "desktop.home.week_games",
                played.len().to_string(),
            ),
            tile(
                "calendar",
                "desktop.home.week_sessions",
                this.len().to_string(),
            ),
        ],
    ];
    // O jogo da semana: o de mais tempo desde segunda.
    let best = profile::top_since(this.iter().map(|(g, s)| (g.id.as_str(), *s)), start)
        .and_then(|id| ctx.library.find(id))
        .map(|g| {
            let time: Duration = this
                .iter()
                .filter(|(o, _)| o.id == g.id)
                .map(|(_, s)| s.duration)
                .sum();
            OvItem {
                id: g.id.clone(),
                title: g.title.clone(),
                detail: ctx.tf("desktop.home.week_best", &[("time", &profile::hm(time))]),
                thumb: tiles::image(g.shown_cover(), COVER),
                accent: parse_color(&g.accent),
                ..Default::default()
            }
        });
    if best.is_some() {
        groups.push(vec![tile(
            "game",
            "desktop.home.week_best_label",
            String::new(),
        )]);
    }
    // A duração da última sessão e a maior sessão desta semana.
    let mut last = Vec::new();
    if let Some((_, s)) = this.first() {
        last.push(tile(
            "clock",
            "desktop.home.week_last",
            profile::hm(s.duration),
        ));
    }
    if let Some(longest) = this.iter().map(|(_, s)| s.duration).max() {
        last.push(tile(
            "trophy",
            "desktop.home.week_longest",
            profile::hm(longest),
        ));
    }
    if !last.is_empty() {
        groups.push(last);
    }
    Some(Week {
        groups,
        bars,
        text: profile::week_text(ctx, &days, today_i),
        best,
    })
}

/// Precisa de atenção, do mais grave ao menos: erro de segundo plano com "Tentar de novo", jogo sem os arquivos,
/// instalação em andamento e jogo instalado sem capa. Só entra o que o Tako já sabe; sem nada, a seção some.
fn attention(ctx: &Ctx) -> Vec<OvItem> {
    let mut v: Vec<OvItem> = ctx
        .activity
        .list
        .all()
        .iter()
        .filter(|a| a.state == State::Failed && a.action.is_some())
        .map(|a| OvItem {
            id: a.key.clone(),
            title: a.title.clone(),
            detail: ctx.t("desktop.home.error_background"),
            action: ctx.t("updates.desktop.retry"),
            state: "error".into(),
            ..Default::default()
        })
        .collect();
    let row = |g: &Game, state: &str, detail: String, action: String| OvItem {
        id: g.id.clone(),
        title: g.title.clone(),
        detail,
        action,
        thumb: tiles::image(g.shown_cover(), COVER),
        accent: parse_color(&g.accent),
        state: state.into(),
        ..Default::default()
    };
    let games = ctx.library.games();
    v.extend(games.iter().filter(|g| g.is_broken).map(|g| {
        row(
            g,
            "broken",
            ctx.t("desktop.home.broken"),
            ctx.t("desktop.home.see_game"),
        )
    }));
    v.extend(games.iter().filter(|g| g.installing.is_some()).map(|g| {
        let fraction = g
            .installing
            .and_then(|p| p.fraction)
            .filter(|f| f.is_finite())
            .map(|f| f.clamp(0.0, 1.0));
        let detail = match fraction {
            Some(f) => ctx.tf(
                "desktop.home.installing",
                &[("percent", &format!("{}%", (f * 100.0).round() as i32))],
            ),
            None => ctx.t("desktop.home.installing_unknown"),
        };
        OvItem {
            progress: fraction.unwrap_or(-1.0),
            ..row(g, "busy", detail, String::new())
        }
    }));
    let no_art: Vec<&Game> = games
        .iter()
        .filter(|g| g.is_installed && !g.is_broken && g.shown_cover().is_none())
        .collect();
    if let Some(g) = no_art.first() {
        v.push(row(
            g,
            "art",
            ctx.tn("desktop.home.no_art_count", no_art.len() as i64, &[]),
            ctx.t("desktop.home.no_art_action"),
        ));
    }
    v
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
        thumb: tiles::image(g.shown_cover(), COVER),
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
    let name = super::profiles::name(ctx, &ctx.active_profile);
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
    if loading {
        o.week_title = ctx.t("desktop.home.week_title");
        return o;
    }
    if let Some(w) = week(ctx) {
        o.week_title = ctx.t("desktop.home.week_title");
        o.week_groups = w.groups;
        o.bars = w.bars;
        o.bars_text = w.text;
        o.best = w.best.unwrap_or_default();
    }
    let attention = attention(ctx);
    o.attention_title = ctx.t("desktop.home.attention_title");
    o.attention_count = ctx.tn("desktop.home.attention_count", attention.len() as i64, &[]);
    o.attention = attention.into_iter().take(ATTENTION_ROWS).collect();
    let installed = installed(ctx);
    o.installed_title = ctx.t("desktop.home.installed_title");
    o.installed_count = library_text::game_count(ctx.lang(), installed.len());
    o.installed = installed
        .into_iter()
        .map(|g| OvItem {
            id: g.id.clone(),
            title: g.title.clone(),
            detail: if g.play_time.is_zero() {
                ctx.t("desktop.home.installed_never")
            } else {
                ctx.tf(
                    "desktop.home.installed_played",
                    &[("time", &game_text::play_time(ctx.lang(), g.play_time))],
                )
            },
            thumb: tiles::image(g.shown_cover(), COVER),
            accent: parse_color(&g.accent),
            state: if g.is_favorite { "favorite" } else { "" }.into(),
            playable: true,
            ..Default::default()
        })
        .collect();
    o.prev_label = ctx.t("desktop.home.prev");
    o.next_label = ctx.t("desktop.home.next");
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
        // Destaque e Voltar a jogar abrem a lista de alvos, as seções e as capas vêm depois.
        let all = targets(ctx, true);
        assert_eq!(
            &all[..1 + BACK],
            &[
                Target::Hero,
                Target::Back(0),
                Target::Back(1),
                Target::Back(2),
                Target::Back(3)
            ]
        );
        assert!(all.len() > 1 + BACK);
        assert_eq!(targets(ctx, false).len(), all.len() - BACK);
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
        assert!(targets(&shell.ctx, true).is_empty());
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

    #[test]
    fn the_week_comes_only_from_sessions_and_vanishes_without_them() {
        let (_dir, mut shell) = shell();
        assert!(week(&shell.ctx).is_none(), "sem sessão, sem Sua semana");
        assert!(build(&shell.ctx, None).week_groups.is_empty());
        crate::demo::sessions(&mut shell.ctx);
        let w = week(&shell.ctx).expect("com sessões, há semana");
        assert_eq!(w.bars.len(), 7);
        // Tempo + gráfico, jogos + sessões, e a última sessão; nada de Conquistas ou Capturas.
        assert!(w.groups.len() >= 2 && w.groups[0].len() == 2 && w.groups[1].len() == 2);
        let labels: Vec<String> = w.groups.iter().flatten().map(|t| t.label.clone()).collect();
        assert!(
            labels
                .iter()
                .all(|l| !l.contains("Conquistas") && !l.contains("Capturas"))
        );
        assert!(w.bars.iter().filter(|b| b.today).count() == 1);
        assert!(w.bars.iter().all(|b| (0.0..=1.0).contains(&b.fraction)));
        let o = build(&shell.ctx, None);
        assert_eq!(o.week_groups.len(), w.groups.len());
        assert!(!o.bars_text.is_empty());
        type StateChange = Box<
            dyn FnOnce(
                Option<tako_core::library::GameStateRecord>,
            ) -> tako_core::library::GameStateRecord,
        >;
        let old_states = shell
            .ctx
            .library
            .games()
            .iter()
            .map(|g| {
                let sessions = shell
                    .ctx
                    .library
                    .sessions(&g.id)
                    .iter()
                    .cloned()
                    .map(|mut s| {
                        s.started_at -= chrono::Duration::days(14);
                        s
                    })
                    .collect();
                let change: StateChange = Box::new(move |_| tako_core::library::GameStateRecord {
                    sessions,
                    ..Default::default()
                });
                (g.id.clone(), change)
            })
            .collect();
        shell.ctx.library.import(Vec::new(), old_states).unwrap();
        assert!(
            week(&shell.ctx).is_none(),
            "sessões antigas não mantêm Sua semana"
        );
    }

    #[test]
    fn attention_lists_only_what_the_app_knows_worst_first_and_caps_the_rows() {
        use tako_core::activities::Action;
        let (_dir, mut shell) = shell();
        {
            let mut found = shell.ctx.found.lock().unwrap();
            for g in found.iter_mut() {
                g.cover_uri = Some("file:///capa.png".into());
                g.is_installed = true;
            }
            found[1].installing = Some(tako_core::library::InstallProgress {
                fraction: Some(0.42),
            });
            found[1].is_installed = false;
            found[2].cover_uri = None;
            found[4].cover_uri = None;
        }
        shell.ctx.library.refresh();
        let broken = shell.ctx.library.games()[3].id.clone();
        for g in shell.ctx.library.games_mut() {
            g.is_broken = g.id == broken;
        }
        let now = std::time::Instant::now();
        shell.ctx.activity.list.start(
            "download:1",
            tako_core::activities::Kind::Download,
            "x",
            "",
            now,
        );
        shell
            .ctx
            .activity
            .list
            .fail("download:1", "Não baixou X", Some(Action::Retry), now);
        let items = attention(&shell.ctx);
        let states: Vec<&str> = items.iter().map(|i| i.state.as_str()).collect();
        assert_eq!(states, ["error", "broken", "busy", "art"]);
        assert_eq!(items[0].id, "download:1");
        assert_eq!(
            items[3].detail,
            shell.ctx.tn("desktop.home.no_art_count", 2, &[])
        );
        assert!((items[2].progress - 0.42).abs() < 1e-6);
        assert!(items[2].detail.contains("42%"));
        let o = build(&shell.ctx, None);
        assert_eq!(o.attention.len(), 4);
        assert_eq!(o.attention_count, "4 itens");
        // Sem nada para cuidar, a seção some.
        shell.ctx.activity.list.dismiss("download:1");
        for g in shell.ctx.library.games_mut() {
            g.is_broken = false;
        }
        shell.ctx.found.lock().unwrap().iter_mut().for_each(|g| {
            g.installing = None;
            g.is_installed = true;
            g.cover_uri = Some("file:///capa.png".into());
        });
        shell.ctx.library.refresh();
        assert!(attention(&shell.ctx).is_empty());
        assert!(build(&shell.ctx, None).attention.is_empty());
    }

    #[test]
    fn the_row_of_installed_covers_puts_recent_first_and_stops_at_the_cap() {
        let (_dir, mut shell) = shell();
        let ids = played(&mut shell);
        let row = installed(&shell.ctx);
        assert!(row.len() <= SHELF && !row.is_empty());
        assert_eq!(row[0].id, ids[0], "o último jogado vem primeiro");
        assert!(row.iter().all(|g| g.is_installed));
        let o = build(&shell.ctx, None);
        assert_eq!(o.installed.len(), row.len());
        assert!(o.installed_count.contains(&row.len().to_string()));
        let t = targets(&shell.ctx, shell.ctx.overview_show_back);
        assert_eq!(t.last(), Some(&Target::Cover(row.len() - 1)));
    }
}
