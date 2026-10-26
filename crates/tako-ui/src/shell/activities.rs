//! AtividadeNaBarra (UX-006/007): alimenta a fila de `tako_core::activities` com o que o Tako faz em segundo plano
//! (biblioteca, capas, atualização), monta o botão Downloads da barra de cima e o popover com a lista. Os downloads
//! e a busca de atualização alimentam a fila nos próprios módulos (`downloads.rs`, `app_updates.rs`).

use super::ctx::ToastKind;
use super::layer::{Flow, Layer};
use super::{Cmd, Ctx, Effect, Out};
use crate::model::{Hint, Overlay, PanelRow};
use std::time::Instant;
use tako_core::activities::{Action, Activity, Kind, State as Phase};
use tako_core::library::GameStore;
use tako_core::presentation::game_text;

const LIBRARY: &str = "library";
const COVERS: &str = "covers";
const UPDATE_READY: &str = "update-ready";
/// A busca de atualização do Tako (e a falha do download dela) usa uma chave só.
pub(crate) const UPDATE_CHECK: &str = "update-check";

crate::theme_struct! {
    /// O botão Downloads da barra de cima (`desk-activity`): o contador do que está em andamento (0 = sem bolinha),
    /// o nome acessível e o `state` (`error` com algum erro na lista, `ready` com atualização pronta, senão vazio);
    /// `announce` é a região viva (início, conclusão e erro de qualquer atividade).
    pub struct ActivityData {
        pub count: i32,
        pub label: String,
        pub state: String,
        pub announce: String,
    }
}

#[derive(Default)]
pub struct State {
    pub list: tako_core::activities::Activities,
    /// Jogos na biblioteca quando a atualização começou (para o "N jogos novos" do resumo).
    library_games: usize,
    /// O último passo (feitos, total) de cada [`Job`], para só montar o texto quando ele muda.
    steps: [(usize, usize); 2],
    /// O último resultado que foi para o Aviso: cada conclusão ou erro avisa uma vez.
    announced: Option<Activity>,
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

/// O botão Downloads: o contador é o mesmo "N ativos" do popover (só o que está em andamento). Erro e atualização
/// pronta aparecem no `state` (um selo no botão) e no nome acessível, nunca só pela cor.
pub fn render(ctx: &Ctx, out: &mut Out) {
    let list = &ctx.activity.list;
    let announce = list
        .last_event()
        .map(|a| announcement(ctx, a))
        .unwrap_or_default();
    let running = list.running_count();
    let failed = list
        .all()
        .iter()
        .filter(|a| a.state == Phase::Failed)
        .count();
    let ready = list.get(UPDATE_READY).is_some();
    let label = [
        Some(ctx.t("desktop.activity.button")),
        (running > 0).then(|| ctx.tn("desktop.activity.running", running as i64, &[])),
        (failed > 0).then(|| ctx.tn("desktop.activity.errors", failed as i64, &[])),
        ready.then(|| ctx.t("desktop.window.update_ready")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ");
    let state = match (failed > 0, ready) {
        (true, _) => "error",
        (_, true) => "ready",
        _ => "",
    };
    out.set(
        "desk-activity",
        ActivityData {
            count: running as i32,
            label,
            state: state.into(),
            announce,
        },
    );
}

/// A lista "Atividades" é a camada de cima (clicar na pílula de novo a fecha).
pub fn list_is_open(layers: &[Box<dyn Layer>], ctx: &Ctx) -> bool {
    layers
        .last()
        .is_some_and(|l| l.render(ctx).kind == "activities")
}

/// Abre o popover Downloads. É o que o botão faz com o mouse, o teclado e o controle: refazer um erro é da linha
/// dele no popover, onde o usuário vê o que vai ser refeito.
pub fn open_list(ctx: &mut Ctx) {
    open_kind(ctx, None);
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
        _ => Box::<ListLayer>::default(),
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
    let scan = *crate::app::SCAN_STEP
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    sync_library(ctx, now, scan);
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
    announce_result(ctx);
}

/// O botão só mostra o número: a conclusão e o erro vão para o Aviso (a tela cheia já avisa pelos módulos, e o
/// fim do download pela fila, em `downloads.rs`). Uma vez por evento; um sucesso não cobre um aviso de atenção
/// ainda na tela (ex.: a loja que falhou na leitura da biblioteca).
fn announce_result(ctx: &mut Ctx) {
    let Some(event) = ctx.activity.list.last_event() else {
        return;
    };
    if event.state == Phase::Running || ctx.activity.announced.as_ref() == Some(event) {
        return;
    }
    let event = event.clone();
    let text = announcement(ctx, &event);
    let attention =
        ctx.toast.is_some() && matches!(ctx.toast_kind, ToastKind::Warning | ToastKind::Error);
    match event.state {
        Phase::Failed => ctx.toast_error(text),
        _ if !attention => ctx.toast_success(text),
        _ => {}
    }
    ctx.activity.announced = Some(event);
}

fn is_running(ctx: &Ctx, key: &str) -> bool {
    ctx.activity
        .list
        .get(key)
        .is_some_and(|a| a.state == Phase::Running)
}

/// Uma atividade que o app acompanha pelo estado (biblioteca, capas): começa, avança por passos e termina.
struct Job {
    key: &'static str,
    kind: Kind,
    /// Chave do título em andamento.
    title: &'static str,
    /// Onde o último passo fica em [`State::steps`].
    slot: usize,
}

const LIBRARY_JOB: Job = Job {
    key: LIBRARY,
    kind: Kind::LibraryRefresh,
    title: "desktop.activity.library",
    slot: 0,
};
const COVERS_JOB: Job = Job {
    key: COVERS,
    kind: Kind::Covers,
    title: "desktop.activity.covers",
    slot: 1,
};

/// `step` = (feitos, total) enquanto trabalha, `None` quando parou. Compara os números antes de montar o texto:
/// `detail` (andamento) e `end` (título e resumo) só rodam quando algo muda, não a cada quadro.
fn sync_job(
    ctx: &mut Ctx,
    now: Instant,
    job: &Job,
    step: Option<(usize, usize)>,
    detail: impl FnOnce(&Ctx) -> String,
    end: impl FnOnce(&Ctx) -> (String, String),
) {
    let running = is_running(ctx, job.key);
    match step {
        Some(step) if running && ctx.activity.steps[job.slot] == step => return,
        Some(step) => {
            ctx.activity.steps[job.slot] = step;
            let detail = detail(ctx);
            if running {
                ctx.activity.list.progress(job.key, None, &detail);
            } else {
                let title = ctx.t(job.title);
                ctx.activity
                    .list
                    .start(job.key, job.kind, &title, &detail, now);
            }
        }
        None if running => {
            let (title, detail) = end(ctx);
            ctx.activity.list.finish(job.key, &title, &detail, now);
        }
        None => return,
    }
    ctx.dirty = true;
}

/// "Steam · 3 de 5 lojas": a última loja lida pela leitura em curso (`scan`).
fn sync_library(ctx: &mut Ctx, now: Instant, scan: Option<(GameStore, usize, usize)>) {
    if ctx.scanning && !is_running(ctx, LIBRARY) {
        ctx.activity.library_games = ctx.library.games().len();
    }
    let step = ctx
        .scanning
        .then(|| scan.map_or((0, 0), |(_, done, total)| (done, total)));
    sync_job(
        ctx,
        now,
        &LIBRARY_JOB,
        step,
        |ctx| {
            scan.map(|(store, done, total)| {
                ctx.tf(
                    "desktop.activity.library_step",
                    &[
                        ("store", &game_text::store_name(ctx.lang(), store)),
                        ("done", &done.to_string()),
                        ("total", &total.to_string()),
                    ],
                )
            })
            .unwrap_or_default()
        },
        |ctx| {
            let new = (ctx.library.games().len()).saturating_sub(ctx.activity.library_games);
            let detail = if new > 0 {
                tako_core::i18n::tn("desktop.activity.library_new", new as i64, &[])
            } else {
                String::new()
            };
            (ctx.t("desktop.activity.library_done"), detail)
        },
    );
}

/// "12 de 24 jogos": a rodada da fila de metadados e capas.
fn sync_covers(ctx: &mut Ctx, now: Instant) {
    let Some((done, total)) = (ctx.metadata.as_ref())
        .and_then(|m| m.worker.as_ref())
        .map(|w| w.progress())
    else {
        return;
    };
    let step = |ctx: &Ctx, n: usize| {
        ctx.tf(
            "desktop.activity.covers_step",
            &[("done", &n.to_string()), ("total", &total.to_string())],
        )
    };
    sync_job(
        ctx,
        now,
        &COVERS_JOB,
        (done < total).then_some((done, total)),
        |ctx| step(ctx, done),
        |ctx| (ctx.t("desktop.activity.covers_done"), step(ctx, total)),
    );
}

/// A lista "Atividades" (camada de 400, ancorada na pílula).
#[derive(Default)]
pub struct ListLayer {
    sel: usize,
    /// O foco está no "fechar" da linha com erro (→ chega lá, ← volta à linha).
    close: bool,
}

impl ListLayer {
    /// Linhas e depois as duas ações do rodapé.
    fn total(ctx: &Ctx) -> usize {
        ctx.activity.list.all().len() + 2
    }

    fn row_is_failed(&self, ctx: &Ctx) -> bool {
        (ctx.activity.list.ordered().get(self.sel)).is_some_and(|a| a.state == Phase::Failed)
    }
}

impl Layer for ListLayer {
    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Flow {
        match cmd {
            Cmd::Back => return Flow::Close,
            Cmd::Up => self.sel = self.sel.saturating_sub(1),
            Cmd::Down => self.sel = (self.sel + 1).min(Self::total(ctx) - 1),
            Cmd::Right if self.row_is_failed(ctx) => {
                self.close = true;
                return Flow::Stay;
            }
            Cmd::Left => self.close = false,
            Cmd::Accept => {
                let rows = ctx.activity.list.all().len();
                return match self.sel.checked_sub(rows) {
                    None if self.close => self.click("close", self.sel, ctx),
                    None => self.click("row", self.sel, ctx),
                    Some(i) => self.click("footer", i, ctx),
                };
            }
            _ => return Flow::Stay,
        }
        self.close = false;
        Flow::Stay
    }

    fn click(&mut self, zone: &str, index: usize, ctx: &mut Ctx) -> Flow {
        let row = (ctx.activity.list.ordered().get(index))
            .map(|a| (a.key.clone(), a.kind, a.state, a.action));
        let flow = match (zone, row) {
            // O erro: a linha refaz o trabalho (ou fecha, sem ação) e o "fechar" tira o erro.
            ("row", Some((key, _, Phase::Failed, Some(_)))) => {
                run_action(ctx, &key);
                Flow::Stay
            }
            ("row" | "close", Some((key, _, Phase::Failed, _))) => {
                ctx.activity.list.dismiss(&key);
                Flow::Stay
            }
            ("row", Some((_, kind, ..))) => {
                open_kind(ctx, Some(kind));
                Flow::Close
            }
            ("footer", _) if index == 0 => {
                super::downloads::open(ctx);
                Flow::Close
            }
            ("footer", _) => {
                ctx.activity.list.clear_done();
                Flow::Stay
            }
            ("outside", _) => Flow::Close,
            _ => Flow::Stay,
        };
        self.sel = self.sel.min(Self::total(ctx) - 1);
        self.close = false;
        flow
    }

    fn hints(&self, _ctx: &Ctx) -> Vec<Hint> {
        Vec::new()
    }

    fn render(&self, ctx: &Ctx) -> Overlay {
        let running = ctx.activity.list.running_count();
        let rows: Vec<PanelRow> = ctx
            .activity
            .list
            .ordered()
            .into_iter()
            .enumerate()
            .map(|(i, a)| {
                let mut row = PanelRow::new(a.key.clone(), "activity", a.title.clone())
                    .desc(a.detail.clone());
                row.icon = icon(a).into();
                row.action = a.progress.map(|p| p.to_string()).unwrap_or_default();
                // O erro oferece a ação no lugar da %.
                row.value = match (a.action, a.progress) {
                    (Some(x), _) => action_label(ctx, x),
                    (None, p) => p.map(|p| format!("{p}%")).unwrap_or_default(),
                };
                row.on = self.close && i == self.sel;
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
                ctx.tn("desktop.activity.running", running as i64, &[])
            } else {
                ctx.t("desktop.activity.idle")
            },
            // Sem nada na fila.
            preview_text: if rows.is_empty() {
                ctx.t("desktop.activity.none")
            } else {
                String::new()
            },
            rows,
            tabs: vec![
                ctx.t("desktop.activity.view_downloads"),
                ctx.t("desktop.activity.clear_done"),
            ],
            // O nome do "fechar" das linhas com erro.
            text: ctx.t("desktop.activity.close"),
            sel: self.sel as i32,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::{Shell, bar};
    use slint_interpreter::Value;

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

    /// Um campo do botão (`desk-activity`), como a janela o recebe.
    fn prop(shell: &mut Shell, name: &str, now: Instant) -> Value {
        let out = shell.render(now);
        let Value::Struct(pill) = out
            .props
            .into_iter()
            .rev()
            .find(|(n, _)| *n == "desk-activity")
            .expect("sem desk-activity")
            .1
        else {
            panic!("desk-activity não é struct");
        };
        pill.get_field(name)
            .unwrap_or_else(|| panic!("sem {name}"))
            .clone()
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
    fn button_counts_what_is_running_and_names_it_with_the_errors_for_the_reader() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        let now = Instant::now();
        assert_eq!(prop(&mut shell, "count", now), Value::Number(0.));
        assert_eq!(text(prop(&mut shell, "label", now)), "Downloads");

        let list = &mut shell.ctx.activity.list;
        list.start(
            LIBRARY,
            Kind::LibraryRefresh,
            "Atualizando biblioteca",
            "Steam · 3 de 5 lojas",
            now,
        );
        assert_eq!(prop(&mut shell, "count", now), Value::Number(1.));
        // Singular com um, plural com mais.
        assert_eq!(text(prop(&mut shell, "label", now)), "Downloads, 1 ativo");

        let list = &mut shell.ctx.activity.list;
        list.start("download:1", Kind::Download, "Hades II", "", now);
        list.progress("download:1", Some(42), "42% · faltam 4 min");
        assert_eq!(prop(&mut shell, "count", now), Value::Number(2.));
        assert_eq!(text(prop(&mut shell, "label", now)), "Downloads, 2 ativos");
        // O leitor de tela não ouve a porcentagem.
        assert!(!text(prop(&mut shell, "announce", now)).contains('%'));

        // O erro não conta como ativo: aparece no selo (`state`) e no nome acessível.
        let list = &mut shell.ctx.activity.list;
        list.fail(
            "download:1",
            "Não baixou Hades II",
            Some(Action::Retry),
            now,
        );
        assert_eq!(prop(&mut shell, "count", now), Value::Number(1.));
        assert_eq!(
            text(prop(&mut shell, "label", now)),
            "Downloads, 1 ativo, 1 erro"
        );
        assert_eq!(text(prop(&mut shell, "state", now)), "error");
        assert_eq!(
            text(prop(&mut shell, "announce", now)),
            "Erro: Não baixou Hades II"
        );
        // O contador do botão é o mesmo do título do popover.
        shell.click("desk-activity", 0);
        assert_eq!(top(&shell).overline, "1 ativo");
    }

    #[test]
    fn finished_activity_is_not_counted_as_running() {
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
        assert_eq!(prop(&mut shell, "count", now), Value::Number(0.));
        assert_eq!(text(prop(&mut shell, "state", now)), "");
    }

    #[test]
    fn a_finished_activity_on_the_desktop_toasts_once_per_event() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        let ctx = &mut shell.ctx;
        let now = Instant::now();
        // A leitura de biblioteca da própria fixture termina no primeiro quadro: descarta esse aviso.
        tick(ctx, now);
        ctx.toast = None;
        let finish = |ctx: &mut Ctx, now: Instant| {
            let list = &mut ctx.activity.list;
            list.start("cloud", Kind::CloudSync, "Sincronizando a nuvem", "", now);
            list.finish("cloud", "Nuvem sincronizada", "Hades II", now);
        };
        // Em andamento não avisa.
        ctx.activity
            .list
            .start("cloud", Kind::CloudSync, "Sincronizando a nuvem", "", now);
        tick(ctx, now);
        assert!(ctx.toast.is_none(), "{:?}", ctx.toast);
        finish(ctx, now);
        tick(ctx, now);
        let toast = |ctx: &Ctx| ctx.toast.as_ref().map(|(t, _)| t.clone());
        assert_eq!(toast(ctx).as_deref(), Some("Nuvem sincronizada. Hades II"));
        assert_eq!(ctx.toast_kind, ToastKind::Success);
        // O mesmo evento não avisa de novo (o quadro seguinte, o aviso já saiu).
        ctx.toast = None;
        tick(ctx, now);
        assert!(ctx.toast.is_none());
        // Outro evento, mesmo da mesma chave, avisa.
        let later = now + std::time::Duration::from_secs(1);
        finish(ctx, later);
        tick(ctx, later);
        assert!(ctx.toast.is_some());
        // Tela cheia: os módulos já avisam, a fila não repete.
        ctx.toast = None;
        ctx.desktop = false;
        finish(ctx, later + std::time::Duration::from_secs(1));
        tick(ctx, later);
        assert!(ctx.toast.is_none());
    }

    #[test]
    fn an_error_on_the_desktop_toasts_as_an_error_and_a_success_never_covers_it() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        let ctx = &mut shell.ctx;
        let now = Instant::now();
        // A leitura de biblioteca da própria fixture termina no primeiro quadro: descarta esse aviso.
        tick(ctx, now);
        ctx.toast = None;
        let list = &mut ctx.activity.list;
        list.start("download:1", Kind::Download, "Hades II", "", now);
        list.fail(
            "download:1",
            "Não baixou Hades II",
            Some(Action::Retry),
            now,
        );
        tick(ctx, now);
        let text = |ctx: &Ctx| ctx.toast.as_ref().map(|(t, _)| t.clone());
        assert_eq!(text(ctx).as_deref(), Some("Erro: Não baixou Hades II"));
        assert_eq!(ctx.toast_kind, ToastKind::Error);
        // A nuvem conclui com o erro ainda na tela: o aviso de atenção fica.
        let list = &mut ctx.activity.list;
        list.start("cloud", Kind::CloudSync, "Sincronizando a nuvem", "", now);
        list.finish("cloud", "Nuvem sincronizada", "", now);
        tick(ctx, now);
        assert_eq!(text(ctx).as_deref(), Some("Erro: Não baixou Hades II"));
        assert_eq!(ctx.toast_kind, ToastKind::Error);
    }

    #[test]
    fn the_button_opens_the_popover_and_each_row_opens_its_target() {
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
            // O botão sempre abre o popover, mesmo com uma atividade só.
            shell.click("desk-activity", 0);
            assert_eq!(top(&shell).kind, "activities");
            shell.click("overlay-row", 0);
            shell.layers.last().map(|l| l.render(&shell.ctx))
        };
        // Procurando atualizações → MenuDoTako; Atualização pronta → o diálogo; download → PainelDownloads;
        // capas → a revisão; erro e concluída ficam na lista (o erro fecha, a concluída abre o destino).
        assert_eq!(
            open(Kind::UpdateCheck, Phase::Running).unwrap().rows[0].key,
            "settings"
        );
        assert_eq!(
            open(Kind::UpdateReady, Phase::Running).unwrap().kind,
            "update"
        );
        assert_eq!(open(Kind::Download, Phase::Running).unwrap().kind, "panel");
        assert!(open(Kind::Covers, Phase::Running).is_some());
        // O erro sem ação fecha na linha e a lista continua aberta.
        let closed = open(Kind::CloudSync, Phase::Failed).unwrap();
        assert_eq!((closed.kind.as_str(), closed.rows.len()), ("activities", 0));
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
            ("activities", "Downloads")
        );
        assert_eq!(o.overline, "2 ativos");
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
    fn closing_an_error_row_removes_it_and_the_row_retries_the_work() {
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
        shell.click("desk-activity", 0);
        shell.click("overlay-row", 0);
        assert!(shell.ctx.activity.list.all().is_empty());
        assert!(matches!(
            shell.ctx.effects.last(),
            Some(Effect::CheckUpdates)
        ));
        let list = &mut shell.ctx.activity.list;
        list.start("cloud", Kind::CloudSync, "x", "", now);
        list.fail("cloud", "Sem rede", None, now);
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
        // O passo entra pelo parâmetro: `SCAN_STEP` é global e outros testes de `app.rs` o escrevem.
        sync_library(ctx, now, Some((tako_core::library::GameStore::Steam, 3, 5)));
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
        sync_library(ctx, now, None);
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
    fn update_ready_shows_a_dot_row_and_a_ready_state_until_the_update_goes_away() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_dir, mut shell) = shell();
        let now = Instant::now();
        shell.ctx.update_ready = Some("x".into());
        tick(&mut shell.ctx, now);
        // Esperar o usuário não é "ativo": o botão não conta e mostra o ponto de pronto, também no nome acessível.
        assert_eq!(prop(&mut shell, "count", now), Value::Number(0.));
        assert_eq!(text(prop(&mut shell, "state", now)), "ready");
        let ready = shell.ctx.t("desktop.window.update_ready");
        assert_eq!(
            text(prop(&mut shell, "label", now)),
            format!("Downloads, {ready}")
        );
        shell.click("desk-activity", 0);
        assert_eq!(top(&shell).rows[0].icon, "dot");
        shell.ctx.update_ready = None;
        tick(&mut shell.ctx, now);
        assert_eq!(text(prop(&mut shell, "state", now)), "");
    }

    #[test]
    fn accepting_the_button_always_opens_the_popover_and_the_row_retries() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        let failed = |shell: &mut Shell| {
            let list = &mut shell.ctx.activity.list;
            list.start(UPDATE_CHECK, Kind::UpdateCheck, "x", "", now);
            list.fail(UPDATE_CHECK, "Sem rede", Some(Action::Retry), now);
        };
        // Teclado/controle: Aceitar no botão com erro no topo abre a lista e não refaz nada às cegas.
        let (_dir, mut shell) = shell();
        failed(&mut shell);
        bar::activate(bar::Item::Downloads, &mut shell.ctx);
        shell.after_action();
        assert_eq!(top(&shell).kind, "activities");
        assert_eq!(
            shell.ctx.activity.list.all().len(),
            1,
            "o erro continua na lista"
        );
        assert!(!matches!(
            shell.ctx.effects.last(),
            Some(Effect::CheckUpdates)
        ));
        // A linha do erro refaz com Aceitar.
        shell.handle(Cmd::Accept);
        assert!(shell.ctx.activity.list.all().is_empty());
        assert!(matches!(
            shell.ctx.effects.last(),
            Some(Effect::CheckUpdates)
        ));
        // Erro sem ação: a lista abre e o "fechar" alcança o erro pelo teclado.
        shell.close_layers();
        let list = &mut shell.ctx.activity.list;
        list.start("cloud", Kind::CloudSync, "x", "", now);
        list.fail("cloud", "Sem rede", None, now);
        bar::activate(bar::Item::Downloads, &mut shell.ctx);
        shell.after_action();
        assert_eq!(top(&shell).kind, "activities");
        shell.handle(Cmd::Right);
        assert!(top(&shell).rows[0].on, "o foco foi para o fechar");
        shell.handle(Cmd::Accept);
        assert!(shell.ctx.activity.list.all().is_empty());
    }

    #[test]
    fn the_live_region_exists_without_activities_and_announces_any_queue_event() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_dir, mut shell) = shell();
        let now = Instant::now();
        assert_eq!(text(prop(&mut shell, "announce", now)), "");
        let list = &mut shell.ctx.activity.list;
        list.start(
            LIBRARY,
            Kind::LibraryRefresh,
            "Atualizando biblioteca",
            "",
            now,
        );
        list.start("download:1", Kind::Download, "Hades II", "", now);
        list.progress("download:1", Some(10), "10%");
        // A biblioteca termina atrás do download: o leitor ouve a conclusão e o contador fica só com o download.
        list.finish(LIBRARY, "Biblioteca atualizada", "4 jogos novos", now);
        assert_eq!(prop(&mut shell, "count", now), Value::Number(1.));
        assert_eq!(
            text(prop(&mut shell, "announce", now)),
            "Biblioteca atualizada. 4 jogos novos"
        );
    }

    #[test]
    fn clicking_the_button_with_the_popover_open_closes_it_instead_of_stacking_another() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_dir, mut shell) = shell();
        let now = Instant::now();
        let list = &mut shell.ctx.activity.list;
        list.start(LIBRARY, Kind::LibraryRefresh, "Biblioteca", "", now);
        list.start(COVERS, Kind::Covers, "Capas", "", now);
        shell.click("desk-activity", 0);
        assert_eq!(top(&shell).kind, "activities");
        shell.click("desk-activity", 0);
        assert!(!shell.has_layers(), "o segundo clique fecha a lista");
    }
}
