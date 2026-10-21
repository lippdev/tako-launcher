//! AtividadeNaBarra (UX-006): alimenta a fila de `tako_core::activities` com o que o Tako faz em segundo plano
//! (biblioteca, capas, atualização), monta a pílula da barra de cima e a lista "Atividades". Os downloads e a
//! busca de atualização alimentam a fila nos próprios módulos (`downloads.rs`, `app_updates.rs`).

use super::layer::{Flow, Layer};
use super::{Cmd, Ctx, Effect, Out};
use crate::model::{Hint, Overlay, PanelRow};
use std::time::Instant;
use tako_core::activities::{Action, Activities, Activity, Kind, Shown, State as Phase};
use tako_core::presentation::game_text;

const LIBRARY: &str = "library";
const COVERS: &str = "covers";
const UPDATE_READY: &str = "update-ready";
/// A busca de atualização do Tako (e a falha do download dela) usa uma chave só.
pub(crate) const UPDATE_CHECK: &str = "update-check";

#[derive(Default)]
pub struct State {
    pub list: Activities,
    /// Jogos na biblioteca quando a atualização começou (para o "N jogos novos" do resumo).
    library_games: usize,
}

/// A mais importante primeiro (a ordem da pílula), igual à lista.
fn ordered(list: &Activities) -> Vec<&Activity> {
    let mut all: Vec<&Activity> = list.all().iter().collect();
    all.sort_by_key(|a| (a.state != Phase::Failed, a.kind));
    all
}

/// Ícone da pílula e das linhas: `spin` sem %, `ring` com %, `done`, `warn` e `dot` (atualização pronta).
fn icon(a: &Activity) -> &'static str {
    match (a.state, a.kind, a.progress) {
        (Phase::Failed, ..) => "warn",
        (Phase::Done, ..) => "done",
        (_, Kind::UpdateReady, _) => "dot",
        (_, _, Some(_)) => "ring",
        _ => "spin",
    }
}

fn action_label(ctx: &Ctx, action: Action) -> String {
    match action {
        Action::Retry => ctx.t("updates.desktop.retry"),
    }
}

/// O que o leitor de tela anuncia: início, conclusão e erro, nunca a porcentagem.
fn announcement(ctx: &Ctx, a: &Activity) -> String {
    match a.state {
        Phase::Running => a.title.clone(),
        Phase::Done => [a.title.as_str(), a.detail.as_str()]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(". "),
        Phase::Failed => ctx.tf("desktop.activity.error", &[("reason", &a.title)]),
    }
}

/// A pílula da barra de cima: some com a fila vazia.
pub fn render(ctx: &Ctx, out: &mut Out, now: Instant) {
    let shown = ctx.activity.list.shown(now);
    out.set("desk-activity-on", shown.is_some());
    let (mut id, mut title, mut detail, mut icon_name) = Default::default();
    let (mut progress, mut action, mut more, mut tip, mut label, mut announce) = (
        0f32,
        String::new(),
        0,
        String::new(),
        String::new(),
        String::new(),
    );
    if let Some(Shown {
        activity: a,
        more: n,
    }) = shown
    {
        id = format!("{}:{:?}", a.key, a.state);
        title = a.title.clone();
        detail = a.detail.clone();
        icon_name = icon(a).to_owned();
        progress = f32::from(a.progress.unwrap_or(0));
        action = a.action.map(|x| action_label(ctx, x)).unwrap_or_default();
        more = n as i32;
        tip = if a.detail.is_empty() {
            a.title.clone()
        } else {
            format!("{} · {}", a.title, a.detail)
        };
        label = if n > 0 {
            format!(
                "{}, {}",
                a.title,
                ctx.tf("desktop.activity.more", &[("n", &n.to_string())])
            )
        } else {
            a.title.clone()
        };
        announce = announcement(ctx, a);
    }
    out.set("desk-activity-id", id);
    out.set("desk-activity-title", title);
    out.set("desk-activity-detail", detail);
    out.set("desk-activity-icon", icon_name);
    out.set("desk-activity-progress", progress);
    out.set("desk-activity-action", action);
    out.set("desk-activity-more", more);
    out.set("desk-activity-tip", tip);
    out.set("desk-activity-label", label);
    out.set("desk-activity-announce", announce);
}

/// Há pílula na barra agora (entra na ordem do teclado).
pub fn visible(ctx: &Ctx, now: Instant) -> bool {
    ctx.activity.list.shown(now).is_some()
}

/// Clique na pílula: o destino da atividade mostrada; com várias, erro ou concluída, a lista.
pub fn open(ctx: &mut Ctx, now: Instant) {
    let Some(Shown { activity: a, more }) = ctx.activity.list.shown(now) else {
        return;
    };
    let kind = (more == 0 && a.state == Phase::Running).then_some(a.kind);
    open_kind(ctx, kind);
}

fn open_kind(ctx: &mut Ctx, kind: Option<Kind>) {
    let layer: Box<dyn Layer> = match kind {
        Some(Kind::Download) => {
            super::downloads::open(ctx);
            return;
        }
        Some(Kind::UpdateReady) => {
            crate::app_updates::open(ctx);
            return;
        }
        Some(Kind::UpdateCheck) => Box::new(super::desktop::brand_menu()),
        Some(Kind::Covers) => Box::new(super::art_review::panel(ctx)),
        _ => Box::new(ListLayer { sel: 0 }),
    };
    ctx.pending_layer = Some(layer);
}

/// A ação do erro mostrado ("Tentar de novo"): refaz o trabalho e tira o erro; quem alimenta a fila começa outro.
pub fn run_action(ctx: &mut Ctx, key: &str) {
    if ctx
        .activity
        .list
        .get(key)
        .is_none_or(|a| a.action.is_none())
    {
        return;
    }
    ctx.activity.list.dismiss(key);
    if key == UPDATE_CHECK {
        ctx.effect(Effect::CheckUpdates);
    } else if let Some(id) = key.strip_prefix("download:").and_then(|i| i.parse().ok())
        && let Some(m) = super::downloads::manager(ctx)
    {
        let _ = m.resume(id);
    }
    ctx.dirty = true;
}

/// Relógio da UI (por quadro, só compara): biblioteca, capas e "Atualização pronta".
pub fn tick(ctx: &mut Ctx, now: Instant) {
    if !ctx.desktop {
        return;
    }
    sync_library(ctx, now);
    sync_covers(ctx, now);
    let ready = ctx.update_ready.is_some();
    if ready != ctx.activity.list.get(UPDATE_READY).is_some() {
        if ready {
            let title = ctx.t("desktop.window.update_ready");
            ctx.activity
                .list
                .start(UPDATE_READY, Kind::UpdateReady, &title, "", now);
        } else {
            ctx.activity.list.dismiss(UPDATE_READY);
        }
        ctx.dirty = true;
    }
    // A conclusão sai da pílula sozinha: redesenha no instante.
    if let Some(t) = ctx.activity.list.next_expiry(now) {
        ctx.redraw_at = Some(ctx.redraw_at.map_or(t, |r| r.min(t)));
    }
}

fn is_running(ctx: &Ctx, key: &str) -> bool {
    ctx.activity
        .list
        .get(key)
        .is_some_and(|a| a.state == Phase::Running)
}

/// "Steam · 3 de 5 lojas": a última loja lida pela leitura em curso.
fn sync_library(ctx: &mut Ctx, now: Instant) {
    let running = is_running(ctx, LIBRARY);
    if ctx.scanning {
        let step = *crate::app::SCAN_STEP
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let detail = step
            .map(|(store, done, total)| {
                ctx.tf(
                    "desktop.activity.library_step",
                    &[
                        ("store", &game_text::store_name(ctx.lang(), store)),
                        ("done", &done.to_string()),
                        ("total", &total.to_string()),
                    ],
                )
            })
            .unwrap_or_default();
        if !running {
            ctx.activity.library_games = ctx.library.games().len();
            let title = ctx.t("desktop.activity.library");
            ctx.activity
                .list
                .start(LIBRARY, Kind::LibraryRefresh, &title, &detail, now);
            ctx.dirty = true;
        } else if ctx
            .activity
            .list
            .get(LIBRARY)
            .is_some_and(|a| a.detail != detail)
        {
            ctx.activity.list.progress(LIBRARY, None, &detail);
            ctx.dirty = true;
        }
    } else if running {
        let new = ctx
            .library
            .games()
            .len()
            .saturating_sub(ctx.activity.library_games);
        let detail = if new > 0 {
            tako_core::i18n::tn("desktop.activity.library_new", new as i64, &[])
        } else {
            String::new()
        };
        let title = ctx.t("desktop.activity.library_done");
        ctx.activity.list.finish(LIBRARY, &title, &detail, now);
        ctx.dirty = true;
    }
}

/// "12 de 24 jogos": a rodada da fila de metadados e capas.
fn sync_covers(ctx: &mut Ctx, now: Instant) {
    let Some((done, total)) = ctx
        .metadata
        .as_ref()
        .and_then(|m| m.worker.as_ref())
        .map(|w| w.progress())
    else {
        return;
    };
    let step = |n: usize| {
        ctx.tf(
            "desktop.activity.covers_step",
            &[("done", &n.to_string()), ("total", &total.to_string())],
        )
    };
    let running = is_running(ctx, COVERS);
    if done < total {
        let detail = step(done);
        if !running {
            let title = ctx.t("desktop.activity.covers");
            ctx.activity
                .list
                .start(COVERS, Kind::Covers, &title, &detail, now);
            ctx.dirty = true;
        } else if ctx
            .activity
            .list
            .get(COVERS)
            .is_some_and(|a| a.detail != detail)
        {
            ctx.activity.list.progress(COVERS, None, &detail);
            ctx.dirty = true;
        }
    } else if running {
        let (title, detail) = (ctx.t("desktop.activity.covers_done"), step(total));
        ctx.activity.list.finish(COVERS, &title, &detail, now);
        ctx.dirty = true;
    }
}

/// A lista "Atividades" (camada de 400, ancorada na pílula).
pub struct ListLayer {
    sel: usize,
}

impl ListLayer {
    /// Linhas e depois as duas ações do rodapé.
    fn total(ctx: &Ctx) -> usize {
        ctx.activity.list.all().len() + 2
    }
}

impl Layer for ListLayer {
    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Flow {
        match cmd {
            Cmd::Back => Flow::Close,
            Cmd::Up => {
                self.sel = self.sel.saturating_sub(1);
                Flow::Stay
            }
            Cmd::Down => {
                self.sel = (self.sel + 1).min(Self::total(ctx) - 1);
                Flow::Stay
            }
            Cmd::Accept => {
                let rows = ctx.activity.list.all().len();
                match self.sel.checked_sub(rows) {
                    None => self.click("row", self.sel, ctx),
                    Some(i) => self.click("footer", i, ctx),
                }
            }
            _ => Flow::Stay,
        }
    }

    fn click(&mut self, zone: &str, index: usize, ctx: &mut Ctx) -> Flow {
        match zone {
            "row" => {
                let Some((key, kind, phase)) = ordered(&ctx.activity.list)
                    .get(index)
                    .map(|a| (a.key.clone(), a.kind, a.state))
                else {
                    return Flow::Stay;
                };
                if phase == Phase::Failed {
                    // Fechar o erro.
                    ctx.activity.list.dismiss(&key);
                    return Flow::Stay;
                }
                open_kind(ctx, Some(kind));
                Flow::Close
            }
            "footer" if index == 0 => {
                super::downloads::open(ctx);
                Flow::Close
            }
            "footer" => {
                ctx.activity.list.clear_done();
                self.sel = self.sel.min(Self::total(ctx) - 1);
                Flow::Stay
            }
            _ => Flow::Close,
        }
    }

    fn hints(&self, _ctx: &Ctx) -> Vec<Hint> {
        Vec::new()
    }

    fn render(&self, ctx: &Ctx) -> Overlay {
        let running = ctx.activity.list.running_count();
        let rows: Vec<PanelRow> = ordered(&ctx.activity.list)
            .into_iter()
            .map(|a| {
                let mut row = PanelRow::new(a.key.clone(), "activity", a.title.clone())
                    .desc(a.detail.clone());
                row.icon = icon(a).into();
                row.action = a.progress.map(|p| p.to_string()).unwrap_or_default();
                row.value = a.progress.map(|p| format!("{p}%")).unwrap_or_default();
                row.style = match a.state {
                    Phase::Done => "done",
                    Phase::Failed => "warning",
                    Phase::Running => "",
                }
                .into();
                row
            })
            .collect();
        Overlay {
            kind: "activities".into(),
            title: ctx.t("desktop.activity.title"),
            overline: if running > 0 {
                ctx.tf("desktop.activity.running", &[("n", &running.to_string())])
            } else {
                ctx.t("desktop.activity.idle")
            },
            rows,
            tabs: vec![
                ctx.t("desktop.activity.view_downloads"),
                ctx.t("desktop.activity.clear_done"),
            ],
            sel: self.sel as i32,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::Shell;
    use slint_interpreter::Value;
    use std::time::Duration;

    fn shell() -> (tempfile::TempDir, Shell) {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = crate::app::build_ctx(
            tako_core::paths::DataPaths::at(dir.path()),
            tako_platform::Platform::simulated(),
            true,
        );
        ctx.library.refresh();
        let mut shell = Shell::new(ctx);
        shell.enter_desktop();
        (dir, shell)
    }

    fn prop(shell: &mut Shell, name: &str, now: Instant) -> Value {
        let out = shell.render(now);
        out.props
            .into_iter()
            .rev()
            .find(|(n, _)| *n == name)
            .unwrap_or_else(|| panic!("sem {name}"))
            .1
    }

    fn text(v: Value) -> String {
        match v {
            Value::String(s) => s.to_string(),
            other => panic!("não é texto: {other:?}"),
        }
    }

    fn top(shell: &Shell) -> Overlay {
        shell.layers.last().unwrap().render(&shell.ctx)
    }

    #[test]
    fn pill_follows_the_queue_and_shows_the_most_important_with_more() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        let now = Instant::now();
        assert_eq!(
            prop(&mut shell, "desk-activity-on", now),
            Value::Bool(false)
        );

        let list = &mut shell.ctx.activity.list;
        list.start(
            LIBRARY,
            Kind::LibraryRefresh,
            "Atualizando biblioteca",
            "Steam · 3 de 5 lojas",
            now,
        );
        assert_eq!(prop(&mut shell, "desk-activity-on", now), Value::Bool(true));
        assert_eq!(text(prop(&mut shell, "desk-activity-icon", now)), "spin");
        assert_eq!(
            text(prop(&mut shell, "desk-activity-detail", now)),
            "Steam · 3 de 5 lojas"
        );

        let list = &mut shell.ctx.activity.list;
        list.start("download:1", Kind::Download, "Hades II", "", now);
        list.progress("download:1", Some(42), "42% · faltam 4 min");
        assert_eq!(
            text(prop(&mut shell, "desk-activity-title", now)),
            "Hades II"
        );
        assert_eq!(text(prop(&mut shell, "desk-activity-icon", now)), "ring");
        assert_eq!(
            prop(&mut shell, "desk-activity-more", now),
            Value::Number(1.)
        );
        assert!(text(prop(&mut shell, "desk-activity-label", now)).ends_with("e mais 1"));
        // O leitor de tela não ouve a porcentagem.
        assert!(!text(prop(&mut shell, "desk-activity-announce", now)).contains('%'));

        let list = &mut shell.ctx.activity.list;
        list.fail(
            "download:1",
            "Não baixou Hades II",
            Some(Action::Retry),
            now,
        );
        assert_eq!(text(prop(&mut shell, "desk-activity-icon", now)), "warn");
        assert_eq!(
            text(prop(&mut shell, "desk-activity-action", now)),
            "Tentar de novo"
        );
        assert_eq!(
            text(prop(&mut shell, "desk-activity-announce", now)),
            "Erro: Não baixou Hades II"
        );
    }

    #[test]
    fn finished_activity_leaves_the_pill_in_four_seconds_and_tick_asks_for_the_redraw() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_dir, mut shell) = shell();
        let now = Instant::now();
        let list = &mut shell.ctx.activity.list;
        list.start(
            LIBRARY,
            Kind::LibraryRefresh,
            "Atualizando biblioteca",
            "",
            now,
        );
        list.finish(LIBRARY, "Biblioteca atualizada", "4 jogos novos", now);
        assert_eq!(text(prop(&mut shell, "desk-activity-icon", now)), "done");
        tick(&mut shell.ctx, now);
        assert_eq!(shell.ctx.redraw_at, Some(now + Duration::from_secs(4)));
        let later = now + Duration::from_secs(4);
        assert_eq!(
            prop(&mut shell, "desk-activity-on", later),
            Value::Bool(false)
        );
    }

    #[test]
    fn clicking_the_pill_opens_the_target_of_each_kind() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let now = Instant::now();
        let open = |kind: Kind, state: Phase| {
            let (_dir, mut shell) = shell();
            let list = &mut shell.ctx.activity.list;
            list.start("k", kind, "Tarefa", "", now);
            match state {
                Phase::Done => list.finish("k", "Pronto", "", now),
                Phase::Failed => list.fail("k", "Erro", None, now),
                Phase::Running => {}
            }
            shell.click("desk-activity", 0);
            let layer = shell.layers.last().map(|l| l.render(&shell.ctx));
            layer.map(|o| {
                (
                    o.kind,
                    o.rows.first().map(|r| r.key.clone()).unwrap_or_default(),
                )
            })
        };
        assert_eq!(
            open(Kind::LibraryRefresh, Phase::Running).unwrap().0,
            "activities"
        );
        assert_eq!(
            open(Kind::CloudSync, Phase::Running).unwrap().0,
            "activities"
        );
        assert_eq!(open(Kind::Download, Phase::Failed).unwrap().0, "activities");
        assert_eq!(open(Kind::Covers, Phase::Done).unwrap().0, "activities");
        // Procurando atualizações → MenuDoTako; Atualização pronta → o diálogo; download → PainelDownloads.
        assert_eq!(
            open(Kind::UpdateCheck, Phase::Running).unwrap().1,
            "settings"
        );
        assert_eq!(open(Kind::UpdateReady, Phase::Running).unwrap().0, "update");
        assert_eq!(open(Kind::Download, Phase::Running).unwrap().0, "panel");
        assert!(open(Kind::Covers, Phase::Running).is_some());
    }

    #[test]
    fn list_shows_rows_by_importance_clears_done_and_closes_with_back() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        let now = Instant::now();
        let list = &mut shell.ctx.activity.list;
        list.start(
            "covers",
            Kind::Covers,
            "Buscando capas",
            "12 de 24 jogos",
            now,
        );
        list.start(
            LIBRARY,
            Kind::LibraryRefresh,
            "Atualizando biblioteca",
            "",
            now,
        );
        list.start("check", Kind::UpdateCheck, "Procurando", "", now);
        list.finish("check", "Você está em dia", "", now);
        shell.click("desk-activity", 0);
        let o = top(&shell);
        assert_eq!(
            (o.kind.as_str(), o.title.as_str()),
            ("activities", "Atividades")
        );
        assert_eq!(o.overline, "2 em andamento");
        let titles: Vec<_> = o.rows.iter().map(|r| r.title.as_str()).collect();
        assert_eq!(
            titles,
            [
                "Atualizando biblioteca",
                "Você está em dia",
                "Buscando capas"
            ]
        );
        assert_eq!(o.rows[1].style, "done");
        assert_eq!(o.tabs, ["Ver downloads", "Limpar concluídas"]);
        // "Limpar concluídas" tira só a concluída; a lista continua aberta.
        shell.click("overlay-footer", 1);
        assert_eq!(top(&shell).rows.len(), 2);
        shell.handle(Cmd::Back);
        assert!(!shell.has_layers());
    }

    #[test]
    fn closing_an_error_row_removes_it_and_the_retry_runs_from_the_pill() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_dir, mut shell) = shell();
        let now = Instant::now();
        let list = &mut shell.ctx.activity.list;
        list.start(UPDATE_CHECK, Kind::UpdateCheck, "x", "", now);
        list.fail(
            UPDATE_CHECK,
            "Não deu para procurar",
            Some(Action::Retry),
            now,
        );
        shell.click("desk-activity-action", 0);
        assert!(shell.ctx.activity.list.all().is_empty());
        assert!(matches!(
            shell.ctx.effects.last(),
            Some(Effect::CheckUpdates)
        ));
        let list = &mut shell.ctx.activity.list;
        list.start("cloud", Kind::CloudSync, "x", "", now);
        list.fail("cloud", "Sem rede", None, now);
        shell.click("desk-activity", 0);
        shell.click("overlay-row", 0);
        assert!(shell.ctx.activity.list.all().is_empty(), "fechar o erro");
    }

    #[test]
    fn library_scan_feeds_the_pill_with_the_store_step_and_a_summary() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        let now = Instant::now();
        let ctx = &mut shell.ctx;
        let games = ctx.library.games().len();
        ctx.scanning = true;
        *crate::app::SCAN_STEP.lock().unwrap() = Some((tako_core::library::GameStore::Steam, 3, 5));
        tick(ctx, now);
        let a = ctx.activity.list.get(LIBRARY).unwrap();
        assert_eq!(
            (a.title.as_str(), a.detail.as_str()),
            ("Atualizando biblioteca", "Steam · 3 de 5 lojas")
        );
        ctx.scanning = false;
        ctx.found
            .lock()
            .unwrap()
            .extend(crate::demo::not_installed());
        ctx.library.refresh();
        tick(ctx, now);
        let a = ctx.activity.list.get(LIBRARY).unwrap();
        assert_eq!(
            (a.state, a.title.as_str()),
            (Phase::Done, "Biblioteca atualizada")
        );
        assert_eq!(
            a.detail,
            format!("{} jogos novos", ctx.library.games().len() - games)
        );
    }

    #[test]
    fn update_ready_shows_a_dot_until_the_update_goes_away() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_dir, mut shell) = shell();
        let now = Instant::now();
        shell.ctx.update_ready = Some("x".into());
        tick(&mut shell.ctx, now);
        assert_eq!(text(prop(&mut shell, "desk-activity-icon", now)), "dot");
        shell.ctx.update_ready = None;
        tick(&mut shell.ctx, now);
        assert_eq!(
            prop(&mut shell, "desk-activity-on", now),
            Value::Bool(false)
        );
    }
}
