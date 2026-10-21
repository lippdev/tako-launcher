//! Fluxo desktop de atualização: a rede não bloqueia a UI e fechar o diálogo não cancela o download.
use crate::shell::activities::UPDATE_CHECK;
use crate::{
    model::{Hint, Overlay, PanelRow},
    shell::layer::Flow,
    shell::{Cmd, Ctx, Effect, Layer},
    tasks,
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tako_core::activities::{Action, Kind, State as Phase};
use tako_core::updates::{
    self, UpdatePreferences,
    feed::{self, UpdateInfo},
};

pub const BRAND_MENU: &str = "tako-brand";
const MIN_CHECK: Duration = Duration::from_millis(600);
// A rede aplica o teto; a UI dá tempo para o resultado chegar antes de invalidá-lo.
const MAX_CHECK: Duration = updates::CHECK_TIMEOUT.saturating_add(Duration::from_secs(1));

#[derive(Default, Debug, PartialEq, Eq)]
pub enum Check {
    #[default]
    Idle,
    Searching,
    Current,
    Failed,
}

#[derive(Default)]
pub struct State {
    pub check: Check,
    pub menu_open: bool,
    pub close_menu: bool,
    pub toast_label: String,
    generation: u64,
    changed: Option<Instant>,
    pub info: Option<UpdateInfo>,
    progress: Arc<Mutex<f64>>,
    pub downloading: bool,
    pub installing: bool,
    started: Option<Instant>,
    last_progress: i32,
    error: String,
    failed_checked: bool,
    /// A busca em curso foi pedida pelo usuário (o andamento vai para a AtividadeNaBarra com o menu fechado).
    explicit: bool,
}

pub fn menu_row(ctx: &Ctx) -> PanelRow {
    let (key, icon) = match ctx.app_update.check {
        Check::Idle => ("desktop.menu.check_updates", "refresh"),
        Check::Searching => ("updates.desktop.searching", "refresh"),
        Check::Current => ("updates.desktop.current", "tick"),
        Check::Failed => ("updates.desktop.check_failed", "alert"),
    };
    let mut row = PanelRow::button("updates", ctx.t(key)).icon(icon);
    row.enabled = ctx.app_update.check != Check::Searching;
    row.description = match ctx.app_update.check {
        Check::Idle => String::new(),
        Check::Searching => ctx.tf(
            "updates.desktop.current_version",
            &[("version", &ctx.version)],
        ),
        Check::Current => ctx.tf("updates.desktop.checked", &[("version", &ctx.version)]),
        Check::Failed => ctx.app_update.error.clone(),
    };
    row.style = match ctx.app_update.check {
        Check::Current => "success",
        Check::Failed => "warning",
        _ => "",
    }
    .into();
    row
}

pub fn check(ctx: &mut Ctx, explicit: bool, pkg: feed::Package, arch: feed::Arch) {
    if ctx.app_update.check == Check::Searching || ctx.app_update.downloading {
        return;
    }
    ctx.app_update.generation += 1;
    let generation = ctx.app_update.generation;
    ctx.app_update.check = Check::Searching;
    ctx.app_update.explicit = explicit;
    ctx.app_update.changed = Some(Instant::now());
    ctx.update_status = ctx.t("updates.checking");
    ctx.dirty = true;
    let channel = UpdatePreferences::load(&ctx.paths.updates()).channel();
    tasks::run(
        ctx,
        move || {
            let start = Instant::now();
            let result = updates::check(
                &updates::agent(),
                &updates::SemVer::current(),
                channel,
                pkg,
                arch,
            );
            std::thread::sleep(MIN_CHECK.saturating_sub(start.elapsed()));
            result
        },
        move |ctx, result| {
            if generation != ctx.app_update.generation || ctx.app_update.check != Check::Searching {
                return;
            }
            match result {
                Ok(None) => finish_check(ctx, Check::Current, explicit, ""),
                Err(e) => {
                    tracing::warn!("atualizações: {e}");
                    finish_check(ctx, Check::Failed, explicit, "updates.desktop.connection");
                }
                Ok(Some(info)) => found(ctx, info, pkg, explicit),
            }
        },
    );
}

/// Chegou uma versão nova. Com o menu fechado o aviso vai para a pílula (no desktop) ou para o toast.
fn found(ctx: &mut Ctx, info: UpdateInfo, pkg: feed::Package, explicit: bool) {
    ctx.activity.list.dismiss(UPDATE_CHECK);
    ctx.app_update.check = Check::Idle;
    ctx.app_update.close_menu = ctx.app_update.menu_open;
    let installable = info.can_install() && pkg.self_updates();
    let version = info.version.clone();
    let page = info.page_url.clone();
    ctx.app_update.info = Some(info);
    ctx.app_update.error.clear();
    if installable {
        download(ctx, ctx.app_update.menu_open || !explicit);
    } else {
        ctx.update_status = ctx.tf("updates.available_page", &[("version", &version)]);
        if explicit {
            ctx.effect(Effect::OpenUrl(page));
        }
    }
    if explicit && !ctx.app_update.menu_open {
        let text = ctx.tf("updates.desktop.found", &[("version", &version)]);
        if ctx.desktop {
            result_on_pill(ctx, &text, "");
        } else {
            toast_action(ctx, text, "updates.desktop.view", open);
        }
    }
}

pub(crate) fn finish_check(ctx: &mut Ctx, state: Check, explicit: bool, error: &str) {
    ctx.app_update.check = state;
    ctx.app_update.changed = Some(Instant::now());
    ctx.app_update.error = if error.is_empty() {
        String::new()
    } else {
        ctx.t(error)
    };
    ctx.update_status = ctx.t(if ctx.app_update.check == Check::Current {
        "updates.desktop.current"
    } else {
        "updates.desktop.check_failed"
    });
    if explicit && !ctx.app_update.menu_open {
        if ctx.app_update.check == Check::Failed {
            retry_toast(ctx, "updates.desktop.failed_toast");
        } else if ctx.desktop {
            // Com o menu fechado, o resultado vai para a pílula (o menu aberto já mostra o dele).
            let (title, detail) = (
                ctx.t("updates.desktop.current"),
                ctx.tf(
                    "updates.desktop.current_version",
                    &[("version", &ctx.version)],
                ),
            );
            result_on_pill(ctx, &title, &detail);
        } else {
            let text = ctx.tf(
                "updates.desktop.current_toast",
                &[("version", &ctx.version)],
            );
            ctx.toast(text);
        }
    } else {
        ctx.activity.list.dismiss(UPDATE_CHECK);
    }
    ctx.dirty = true;
}

/// O resultado da busca na pílula: ✓ por 4 s (a pílula cuida do resto).
fn result_on_pill(ctx: &mut Ctx, title: &str, detail: &str) {
    let now = Instant::now();
    let list = &mut ctx.activity.list;
    list.start(UPDATE_CHECK, Kind::UpdateCheck, title, "", now);
    list.finish(UPDATE_CHECK, title, detail, now);
}

/// Falha com "Tentar de novo": na pílula do desktop; fora dele, no aviso.
fn retry_toast(ctx: &mut Ctx, key: &str) {
    let text = ctx.t(key);
    if ctx.desktop {
        let now = Instant::now();
        let list = &mut ctx.activity.list;
        list.start(UPDATE_CHECK, Kind::UpdateCheck, &text, "", now);
        list.fail(UPDATE_CHECK, &text, Some(Action::Retry), now);
        return;
    }
    toast_action(ctx, text, "updates.desktop.retry", |ctx| {
        ctx.effect(Effect::CheckUpdates)
    });
}

/// O script de troca deixa `failed` ao lado do download quando não conseguiu instalar (a versão antiga foi restaurada).
fn take_failed(dir: &std::path::Path) -> bool {
    std::fs::remove_file(dir.join("failed")).is_ok()
}

fn toast_action(ctx: &mut Ctx, text: String, label: &str, action: fn(&mut Ctx)) {
    ctx.toast_with_undo(text, action);
    ctx.app_update.toast_label = ctx.t(label);
}

fn download(ctx: &mut Ctx, show: bool) {
    let Some(info) = ctx.app_update.info.clone() else {
        return;
    };
    ctx.app_update.downloading = true;
    ctx.app_update.started = Some(Instant::now());
    ctx.app_update.progress = Arc::new(Mutex::new(0.));
    let progress = ctx.app_update.progress.clone();
    if show {
        ctx.pending_layer = Some(Box::new(UpdateDialog::new(false)));
    }
    tasks::run(
        ctx,
        move || {
            updates::download(
                &updates::agent(),
                &info,
                &updates::download_dir(),
                &mut |n| {
                    *progress.lock().unwrap_or_else(|p| p.into_inner()) = n;
                },
            )
        },
        |ctx, result| {
            ctx.app_update.downloading = false;
            match result {
                Ok(path) => {
                    ctx.update_ready = Some(path.to_string_lossy().into_owned());
                    ctx.update_status = ctx.t("desktop.window.update_ready");
                    if !ctx.desktop {
                        ctx.toast(ctx.update_status.clone());
                    }
                }
                Err(e) => {
                    tracing::warn!("download da atualização: {e}");
                    retry_toast(ctx, "updates.desktop.download_failed");
                }
            }
        },
    );
}

/// Chamado no relógio da UI: invalida respostas atrasadas e redesenha só quando o progresso muda.
pub fn tick(ctx: &mut Ctx, now: Instant) {
    if !ctx.desktop {
        return;
    }
    if !std::mem::replace(&mut ctx.app_update.failed_checked, true)
        && take_failed(&updates::download_dir())
    {
        retry_toast(ctx, "updates.desktop.download_failed");
    }
    // Fechou o menu antes da resposta: o andamento passa para a pílula.
    if ctx.app_update.check == Check::Searching
        && ctx.app_update.explicit
        && !ctx.app_update.menu_open
        && !ctx
            .activity
            .list
            .get(UPDATE_CHECK)
            .is_some_and(|a| a.state == Phase::Running)
    {
        let title = ctx.t("desktop.activity.checking");
        ctx.activity
            .list
            .start(UPDATE_CHECK, Kind::UpdateCheck, &title, "", now);
        ctx.dirty = true;
    }
    if ctx.app_update.check == Check::Searching
        && ctx
            .app_update
            .changed
            .is_some_and(|t| now.duration_since(t) >= MAX_CHECK)
    {
        ctx.app_update.generation += 1;
        finish_check(ctx, Check::Failed, true, "updates.desktop.connection");
    } else if ctx.app_update.check == Check::Current
        && ctx
            .app_update
            .changed
            .is_some_and(|t| now.duration_since(t) >= Duration::from_secs(4))
    {
        ctx.app_update.check = Check::Idle;
        ctx.dirty = true;
    }
    if ctx.app_update.downloading {
        let value = (*ctx
            .app_update
            .progress
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            * 1000.) as i32;
        if value != ctx.app_update.last_progress {
            ctx.app_update.last_progress = value;
            ctx.dirty = true;
        }
    }
}

pub fn open(ctx: &mut Ctx) {
    ctx.pending_layer = Some(Box::new(UpdateDialog::new(true)));
}

pub struct UpdateDialog {
    focus: usize,
    quiet: bool,
}
impl UpdateDialog {
    fn new(quiet: bool) -> Self {
        Self { focus: 1, quiet }
    }
}
impl Layer for UpdateDialog {
    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Flow {
        match cmd {
            Cmd::Back => Flow::Close,
            Cmd::Left | Cmd::Up => {
                self.focus = self.focus.saturating_sub(1);
                Flow::Stay
            }
            Cmd::Right | Cmd::Down => {
                self.focus = (self.focus + 1).min(2);
                Flow::Stay
            }
            Cmd::Accept => self.click("row", self.focus, ctx),
            _ => Flow::Stay,
        }
    }
    fn click(&mut self, zone: &str, index: usize, ctx: &mut Ctx) -> Flow {
        if zone == "outside" {
            return Flow::Close;
        }
        if zone == "notes" {
            if let Some(info) = &ctx.app_update.info {
                ctx.effect(Effect::OpenUrl(info.page_url.clone()));
            }
            return Flow::Stay;
        }
        if index == 1 && !ctx.app_update.downloading {
            if ctx.update_ready.is_some() {
                ctx.effect(Effect::InstallUpdate);
            } else {
                download(ctx, false);
                return Flow::Stay;
            }
        }
        Flow::Close
    }
    fn key(
        &mut self,
        text: &str,
        mods: tako_core::input::shortcut::KeyModifiers,
        ctx: &mut Ctx,
    ) -> Option<Flow> {
        if text == "\t" {
            let order = if ctx.app_update.downloading {
                &[0usize, 2][..]
            } else {
                &[0usize, 1, 2][..]
            };
            let current = order.iter().position(|i| *i == self.focus).unwrap_or(0);
            self.focus = order[(current
                + if mods.contains(tako_core::input::shortcut::KeyModifiers::SHIFT) {
                    order.len() - 1
                } else {
                    1
                })
                % order.len()];
            Some(Flow::Stay)
        } else {
            None
        }
    }
    fn hints(&self, _ctx: &Ctx) -> Vec<Hint> {
        Vec::new()
    }
    fn render(&self, ctx: &Ctx) -> Overlay {
        let state = &ctx.app_update;
        let progress = *state.progress.lock().unwrap_or_else(|p| p.into_inner());
        let info = state.info.as_ref();
        let ready = ctx.update_ready.is_some();
        let total = info
            .and_then(|i| i.asset.as_ref())
            .map(|a| a.size)
            .unwrap_or(0);
        let mb = |bytes: f64| format!("{:.1}", bytes / 1048576.).replace('.', ",");
        let (mut text, mut time) = (String::new(), String::new());
        if state.downloading {
            let elapsed = state
                .started
                .map(|t| t.elapsed().as_secs_f64())
                .unwrap_or(0.);
            let remaining = if progress > 0. {
                (elapsed * (1. - progress) / progress).ceil() as u64
            } else {
                0
            };
            if total > 0 {
                text = ctx.tf(
                    "updates.desktop.progress",
                    &[
                        ("done", &mb(total as f64 * progress)),
                        ("total", &mb(total as f64)),
                    ],
                );
            }
            if remaining > 0 {
                time = ctx.tf(
                    "updates.desktop.remaining",
                    &[("seconds", &remaining.to_string())],
                );
            }
        }
        // Versão no cabeçalho: nova em 600 (`overline`) + o resto em `tabs2[0]`; ajuda do rodapé em `tabs2[1]`;
        // tempo restante em `tabs2[2]`.
        let suffix = if state.downloading {
            ctx.tf("updates.desktop.on_version", &[("version", &ctx.version)])
        } else if ready && total > 0 {
            ctx.tf("updates.desktop.size", &[("size", &mb(total as f64))])
        } else {
            String::new()
        };
        let help = if state.downloading {
            "updates.desktop.help_downloading"
        } else if ready {
            "updates.desktop.help_ready"
        } else {
            ""
        };
        let mut rows = vec![
            PanelRow::button(
                "later",
                ctx.t(if state.downloading {
                    "updates.desktop.hide"
                } else {
                    "updates.desktop.later"
                }),
            )
            .link(),
        ];
        if !state.downloading {
            let mut primary = PanelRow::button(
                "now",
                ctx.t(if ready {
                    "updates.desktop.restart"
                } else {
                    "updates.desktop.retry"
                }),
            );
            primary.style = "primary".into();
            rows.push(primary);
        }
        Overlay {
            kind: "update".into(),
            title: ctx.t(if state.downloading {
                "updates.desktop.downloading"
            } else if ready {
                "desktop.window.update_ready"
            } else {
                "updates.desktop.download_failed"
            }),
            overline: info.map(|i| i.version.clone()).unwrap_or_default(),
            text,
            tab: (progress * 1000.) as i32,
            tab2: i32::from(state.downloading),
            secret: self.quiet,
            tabs: info.map(|i| release_notes(&i.notes)).unwrap_or_default(),
            input_hint: ctx.t("updates.desktop.notes"),
            preview_text: ctx.t("updates.desktop.all_notes"),
            input: ctx
                .running
                .as_ref()
                .filter(|_| ready && !state.downloading)
                .map(|r| ctx.tf("updates.desktop.running", &[("game", &r.title)]))
                .unwrap_or_default(),
            tabs2: vec![
                suffix,
                if help.is_empty() {
                    String::new()
                } else {
                    ctx.t(help)
                },
                time,
            ],
            rows,
            sel: self.focus as i32,
            ..Default::default()
        }
    }
}

fn release_notes(notes: &str) -> Vec<String> {
    notes
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            line.strip_prefix("- ")
                .or_else(|| line.strip_prefix("* "))
                .map(|s| s.chars().take(150).collect())
        })
        .take(3)
        .collect()
}

#[cfg(test)]
pub(crate) fn snapshot(ctx: &mut Ctx, downloading: bool) {
    ctx.app_update.info = Some(UpdateInfo {
        version: "0.2.0-alpha.14".into(), name: String::new(),
        notes: "- Barra de cima unificada\n- Status e filtros na biblioteca\n- Melhorias nas atualizações".into(),
        page_url: "https://github.com/lippdev/tako-launcher/releases".into(),
        asset: Some(feed::Asset { url: String::new(), name: String::new(), digest: String::new(), size: 19084083 }),
        is_prerelease: true,
    });
    ctx.app_update.downloading = downloading;
    *ctx.app_update.progress.lock().unwrap() = 0.68;
    ctx.app_update.started = Some(Instant::now() - Duration::from_secs(12));
    if !downloading {
        ctx.update_ready = Some("snapshot-only".into());
    }
    ctx.pending_layer = Some(Box::new(UpdateDialog::new(false)));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notes_limit_and_ignore_headers() {
        assert_eq!(
            release_notes("# Changes\n- Um\n* Dois\n- Três\n- Quatro"),
            vec!["Um", "Dois", "Três"]
        );
    }
    #[test]
    fn timeout_invalidates_late_results_and_current_resets_after_four_seconds() {
        let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = crate::app::build_ctx(
            tako_core::paths::DataPaths::at(dir.path()),
            tako_platform::Platform::simulated(),
            true,
        );
        ctx.desktop = true;
        let start = Instant::now();
        ctx.app_update.check = Check::Searching;
        ctx.app_update.changed = Some(start);
        ctx.app_update.generation = 3;
        ctx.app_update.menu_open = true;
        tick(&mut ctx, start + Duration::from_secs(11));
        assert_eq!(ctx.app_update.check, Check::Searching);
        tick(&mut ctx, start + MAX_CHECK - Duration::from_millis(1));
        assert_eq!(ctx.app_update.check, Check::Searching);
        assert!(!menu_row(&ctx).enabled);
        tick(&mut ctx, start + MAX_CHECK);
        assert_eq!(ctx.app_update.check, Check::Failed);
        assert_eq!(ctx.app_update.generation, 4);
        assert_eq!(ctx.app_update.error, ctx.t("updates.desktop.connection"));
        assert!(ctx.toast.is_none());
        assert!(menu_row(&ctx).enabled);
        ctx.app_update.menu_open = false;
        finish_check(&mut ctx, Check::Current, true, "");
        // Menu fechado: o resultado vai para a pílula, não para o aviso.
        assert!(ctx.toast.is_none());
        let done = ctx.activity.list.get(UPDATE_CHECK).unwrap();
        assert_eq!(done.state, Phase::Done);
        assert_eq!(done.title, ctx.t("updates.desktop.current"));
        let at = ctx.app_update.changed.unwrap();
        tick(&mut ctx, at + Duration::from_secs(4));
        assert_eq!(ctx.app_update.check, Check::Idle);
    }

    fn desktop_ctx() -> (tempfile::TempDir, Ctx) {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = crate::app::build_ctx(
            tako_core::paths::DataPaths::at(dir.path()),
            tako_platform::Platform::simulated(),
            true,
        );
        ctx.desktop = true;
        (dir, ctx)
    }

    #[test]
    fn search_moves_to_the_pill_when_the_menu_closes_before_the_answer() {
        let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_dir, mut ctx) = desktop_ctx();
        ctx.app_update.check = Check::Searching;
        ctx.app_update.explicit = true;
        ctx.app_update.changed = Some(Instant::now());
        ctx.app_update.menu_open = true;
        tick(&mut ctx, Instant::now());
        assert!(
            ctx.activity.list.all().is_empty(),
            "menu aberto: o item mostra"
        );
        ctx.app_update.menu_open = false;
        tick(&mut ctx, Instant::now());
        let a = ctx.activity.list.get(UPDATE_CHECK).unwrap();
        assert_eq!((a.kind, a.state), (Kind::UpdateCheck, Phase::Running));
        assert_eq!(a.title, ctx.t("desktop.activity.checking"));
    }

    #[test]
    fn menu_item_changes_in_place_per_state() {
        let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_dir, mut ctx) = desktop_ctx();
        let row = menu_row(&ctx);
        assert_eq!((row.icon.as_str(), row.enabled), ("refresh", true));
        assert!(row.description.is_empty());
        ctx.app_update.check = Check::Searching;
        let row = menu_row(&ctx);
        assert!(!row.enabled);
        assert!(row.description.contains(&ctx.version));
        ctx.app_update.check = Check::Current;
        let row = menu_row(&ctx);
        assert_eq!((row.style.as_str(), row.icon.as_str()), ("success", "tick"));
        ctx.app_update.check = Check::Failed;
        ctx.app_update.error = ctx.t("updates.desktop.connection");
        let row = menu_row(&ctx);
        assert_eq!(
            (row.style.as_str(), row.icon.as_str()),
            ("warning", "alert")
        );
        assert!(row.description.contains("Tentar de novo"));
    }

    #[test]
    fn closed_menu_turns_failure_into_a_retry_activity() {
        let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_dir, mut ctx) = desktop_ctx();
        ctx.app_update.menu_open = false;
        finish_check(&mut ctx, Check::Failed, true, "updates.desktop.connection");
        assert!(ctx.toast.is_none());
        let failed = ctx.activity.list.get(UPDATE_CHECK).unwrap();
        assert_eq!(
            (failed.state, failed.action),
            (Phase::Failed, Some(Action::Retry))
        );
        assert_eq!(failed.title, ctx.t("updates.desktop.failed_toast"));
        // "Tentar de novo" na pílula refaz a busca.
        crate::shell::activities::run_action(&mut ctx, UPDATE_CHECK);
        assert!(ctx.activity.list.get(UPDATE_CHECK).is_none());
        assert!(matches!(ctx.effects.last(), Some(Effect::CheckUpdates)));
        // Com o menu aberto o resultado fica no item, sem aviso.
        let (_dir, mut ctx) = desktop_ctx();
        ctx.app_update.menu_open = true;
        finish_check(&mut ctx, Check::Failed, true, "updates.desktop.connection");
        assert!(ctx.toast.is_none());
    }

    #[test]
    fn a_new_version_found_with_the_menu_closed_goes_to_the_pill_not_a_toast() {
        let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let info = || UpdateInfo {
            version: "9.9.9".into(),
            name: String::new(),
            notes: String::new(),
            page_url: "https://example.com".into(),
            asset: None,
            is_prerelease: false,
        };
        let (_dir, mut ctx) = desktop_ctx();
        ctx.app_update.menu_open = false;
        found(&mut ctx, info(), feed::Package::WindowsSetup, true);
        assert!(ctx.toast.is_none());
        let a = ctx.activity.list.get(UPDATE_CHECK).unwrap();
        assert_eq!(a.state, Phase::Done);
        assert_eq!(
            a.title,
            ctx.tf("updates.desktop.found", &[("version", "9.9.9")])
        );
        // Menu aberto: o item do menu mostra, a pílula não.
        let (_dir, mut ctx) = desktop_ctx();
        ctx.app_update.menu_open = true;
        found(&mut ctx, info(), feed::Package::WindowsSetup, true);
        assert!(ctx.activity.list.all().is_empty() && ctx.toast.is_none());
        // Tela cheia não tem a barra: continua o aviso com "Ver".
        let (_dir, mut ctx) = desktop_ctx();
        ctx.desktop = false;
        found(&mut ctx, info(), feed::Package::WindowsSetup, true);
        assert!(ctx.toast.is_some() && ctx.activity.list.all().is_empty());
    }

    #[test]
    fn download_dialog_shows_progress_and_only_hide() {
        let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_dir, mut ctx) = desktop_ctx();
        snapshot(&mut ctx, true);
        ctx.pending_layer = None;
        ctx.running = Some(crate::shell::ctx::RunningGame {
            id: "1".into(),
            title: "Hades II".into(),
            pid: None,
            started: None,
        });
        let dialog = UpdateDialog::new(false);
        let o = dialog.render(&ctx);
        assert_eq!(o.title, "Baixando");
        assert_eq!(o.rows.len(), 1);
        assert_eq!(o.rows[0].title, "Esconder");
        assert_eq!(o.tab, 680);
        assert_eq!(o.text, "12,4 de 18,2 MB");
        assert!(o.tabs2[2].ends_with("s restantes"));
        assert!(o.input.is_empty(), "o aviso do jogo só vale na tela Pronta");
        assert_eq!(o.tabs.len(), 3);
    }

    #[test]
    fn ready_dialog_warns_about_open_game_and_focuses_restart() {
        let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_dir, mut ctx) = desktop_ctx();
        snapshot(&mut ctx, false);
        ctx.pending_layer = None;
        let dialog = UpdateDialog::new(false);
        assert!(dialog.render(&ctx).input.is_empty());
        ctx.running = Some(crate::shell::ctx::RunningGame {
            id: "1".into(),
            title: "Hades II".into(),
            pid: None,
            started: None,
        });
        let o = dialog.render(&ctx);
        assert_eq!(o.title, "Atualização pronta");
        assert!(o.input.starts_with("Hades II está aberto"));
        assert_eq!(
            (o.rows[0].title.as_str(), o.rows[1].title.as_str()),
            ("Depois", "Reiniciar agora")
        );
        assert_eq!(o.rows[1].style, "primary");
        assert_eq!(o.sel, 1);
        assert_eq!(o.overline, "0.2.0-alpha.14");
    }

    #[test]
    fn failed_marker_is_consumed_once() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!take_failed(dir.path()));
        std::fs::write(dir.path().join("failed"), "").unwrap();
        assert!(take_failed(dir.path()));
        assert!(!take_failed(dir.path()));
    }

    #[test]
    fn hiding_download_keeps_it_running_and_ready_actions_are_ordered() {
        let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = crate::app::build_ctx(
            tako_core::paths::DataPaths::at(dir.path()),
            tako_platform::Platform::simulated(),
            true,
        );
        ctx.desktop = true;
        ctx.app_update.downloading = true;
        let mut dialog = UpdateDialog::new(false);
        assert!(matches!(dialog.handle(Cmd::Back, &mut ctx), Flow::Close));
        assert!(ctx.app_update.downloading);
        ctx.app_update.downloading = false;
        ctx.update_ready = Some("verified-update".into());
        let overlay = dialog.render(&ctx);
        assert_eq!(overlay.rows[0].key, "later");
        assert_eq!(overlay.rows[1].key, "now");
        assert_eq!(overlay.sel, 1);
        dialog.click("row", 1, &mut ctx);
        assert!(
            ctx.effects
                .iter()
                .any(|e| matches!(e, Effect::InstallUpdate))
        );
    }
}
