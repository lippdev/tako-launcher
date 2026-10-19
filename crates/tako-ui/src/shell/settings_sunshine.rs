//! A integração é local: a prévia vem da biblioteca e a gravação só ocorre após confirmação.

use super::super::ctx::{Ctx, keys};
use super::super::layer::{Flow, MenuItem, MenuLayer};
use crate::model::PanelRow;
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tako_platform::sunshine::{self, App, SunshineError};

thread_local! {
    static CACHED_DETECTION: RefCell<Option<(Instant, sunshine::Detection)>> = const { RefCell::new(None) };
    static CHECKING: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn rows(ctx: &Ctx) -> Vec<MenuItem> {
    let detection = CACHED_DETECTION.with(|cell| {
        let borrow = cell.borrow_mut();
        if let Some((at, det)) = borrow.as_ref()
            && at.elapsed() < Duration::from_secs(5)
        {
            return det.clone();
        }
        let fallback = borrow
            .as_ref()
            .map(|(_, det)| det.clone())
            .unwrap_or_else(|| {
                let (installed, apps_json) = sunshine::detect_quick();
                sunshine::Detection {
                    installed,
                    running: false,
                    apps_json,
                }
            });
        if !CHECKING.get() {
            CHECKING.set(true);
            crate::tasks::run_bg(sunshine::detect, |_, det| {
                CHECKING.set(false);
                CACHED_DETECTION.with(|c| *c.borrow_mut() = Some((Instant::now(), det)));
            });
        }
        fallback
    });

    let status = match (detection.installed, detection.running) {
        (true, true) => ctx.t("settings.system.sunshine_running"),
        (true, false) => ctx.t("settings.system.sunshine_installed"),
        (false, _) => {
            if sunshine::is_flatpak() {
                ctx.t("settings.system.sunshine_err_flatpak_permission")
            } else {
                ctx.t("settings.system.sunshine_missing")
            }
        }
    };
    let export_enabled = ctx.simulated() || detection.installed || detection.apps_json.is_some();
    let mut rows = vec![
        MenuItem::new(
            PanelRow::info("sunshine-status", ctx.t("settings.system.sunshine"), status)
                .desc(ctx.t("settings.system.sunshine_desc")),
        ),
        MenuItem::new(
            PanelRow::action("sunshine-export", ctx.t("settings.system.sunshine_export"))
                .desc(ctx.t("settings.system.sunshine_export_desc"))
                .value(ctx.t("settings.system.sunshine_export_value"))
                .enabled(export_enabled),
        )
        .on(|ctx| {
            let has_sunshine = sunshine::find_apps_json().is_some()
                || CACHED_DETECTION.with(|c| {
                    c.borrow()
                        .as_ref()
                        .is_some_and(|(_, d)| d.installed || d.apps_json.is_some())
                })
                || sunshine::detect_quick().0;
            if !ctx.simulated() && !has_sunshine {
                if sunshine::is_flatpak() {
                    ctx.toast(ctx.t("settings.system.sunshine_err_flatpak_permission"));
                } else {
                    ctx.toast(ctx.t("settings.system.sunshine_missing"));
                }
                Flow::Stay
            } else {
                Flow::Push(Box::new(preview_dialog(ctx, false)))
            }
        }),
        MenuItem::new(
            PanelRow::toggle(
                "sunshine-sync",
                ctx.t("settings.system.sunshine_sync"),
                ctx.settings.bool(keys::SUNSHINE_SYNC, false),
            )
            .desc(ctx.t("settings.system.sunshine_sync_desc")),
        )
        .on(|ctx| {
            if ctx.settings.bool(keys::SUNSHINE_SYNC, false) {
                ctx.settings.set_bool(keys::SUNSHINE_SYNC, false);
                Flow::Stay
            } else {
                Flow::Push(Box::new(preview_dialog(ctx, true)))
            }
        }),
        MenuItem::new(
            PanelRow::info(
                "moonlight-client",
                ctx.t("settings.system.moonlight"),
                ctx.t("settings.system.moonlight_value"),
            )
            .desc(ctx.t("settings.system.moonlight_desc")),
        ),
    ];
    if crate::discord::available() {
        rows.push(super::toggle(
            ctx,
            "settings.system.discord",
            keys::DISCORD_PRESENCE,
            false,
        ));
    }
    rows
}

fn apps(ctx: &Ctx) -> Result<Vec<sunshine::App>, SunshineError> {
    let executable = sunshine::tako_executable()?;
    Ok(ctx
        .library
        .games()
        .iter()
        .map(|game| App {
            id: game.id.clone(),
            title: game.title.clone(),
            executable: executable.clone(),
        })
        .collect())
}

fn format_sunshine_error(ctx: &Ctx, error: &SunshineError) -> String {
    match error {
        SunshineError::PermissionDenied => {
            if sunshine::is_flatpak() {
                ctx.t("settings.system.sunshine_err_flatpak_permission")
            } else if cfg!(windows) {
                ctx.t("settings.system.sunshine_err_windows_permission")
            } else {
                ctx.t("settings.system.sunshine_err_permission")
            }
        }
        SunshineError::InvalidGameId => ctx.t("settings.system.sunshine_err_invalid_id"),
        SunshineError::InvalidExecutablePath => ctx.t("settings.system.sunshine_err_invalid_path"),
        SunshineError::ExecutableNotFound(e) => {
            ctx.tf("settings.system.sunshine_err_no_exe", &[("detail", e)])
        }
        SunshineError::AppsJsonMalformed(_) => ctx.t("settings.system.sunshine_err_malformed"),
        SunshineError::AppsJsonNotAnObject
        | SunshineError::AppsListMissing
        | SunshineError::InvalidAppEntry => ctx.t("settings.system.sunshine_err_invalid_format"),
        SunshineError::AppsJsonTooLarge => ctx.t("settings.system.sunshine_err_too_large"),
        SunshineError::CannotReadAppsJson(_) => ctx.t("settings.system.sunshine_err_read"),
        SunshineError::CannotCreateBackup(_) => ctx.t("settings.system.sunshine_err_backup"),
        SunshineError::CannotCreateFolder(_) => ctx.t("settings.system.sunshine_err_folder"),
        SunshineError::CannotWriteAppsJson(_) => ctx.t("settings.system.sunshine_err_write"),
        SunshineError::SerializationFailed(_) => ctx.t("settings.system.sunshine_err_generate"),
        SunshineError::SunshineFolderMissing => {
            if sunshine::is_flatpak() {
                ctx.t("settings.system.sunshine_err_flatpak_permission")
            } else {
                ctx.t("settings.system.sunshine_path_missing")
            }
        }
    }
}

fn collect_entries(
    apps: &[App],
    target: Option<&Path>,
) -> (Vec<serde_json::Value>, usize) {
    let mut entries = Vec::new();
    let mut skipped = 0;
    for app in apps {
        match sunshine::app_entry_for_target(app, target) {
            Ok(entry) => entries.push(entry),
            Err(e) => {
                tracing::warn!(game_id = %app.id, title = %app.title, "jogo ignorado ao exportar para o Sunshine: {e}");
                skipped += 1;
            }
        }
    }
    (entries, skipped)
}

pub(crate) fn preview_dialog(ctx: &Ctx, enable_sync: bool) -> MenuLayer {
    let all_apps = match apps(ctx) {
        Ok(apps) => apps,
        Err(error) => {
            return super::text_dialog(
                ctx.t("settings.system.sunshine_export"),
                format_sunshine_error(ctx, &error),
            );
        }
    };
    let valid_apps: Vec<App> = all_apps
        .iter()
        .filter(|app| sunshine::is_valid_game_id(&app.id))
        .cloned()
        .collect();
    let titles = valid_apps
        .iter()
        .take(40)
        .map(|app| format!("• {}", app.title))
        .collect::<Vec<_>>()
        .join("
");
    let extra = valid_apps.len().saturating_sub(40);
    let title_list = if extra > 0 {
        format!(
            "{titles}
{}",
            ctx.tf(
                "settings.system.sunshine_preview_more",
                &[("count", &extra.to_string())]
            )
        )
    } else {
        titles
    };
    let text = ctx.tf(
        "settings.system.sunshine_preview",
        &[("count", &valid_apps.len().to_string()), ("games", &title_list)],
    );
    MenuLayer::dialog(
        ctx.t("settings.system.sunshine_preview_title"),
        move |ctx| {
            let all_apps = all_apps.clone();
            vec![
                MenuItem::new(PanelRow::button(
                    "sunshine-confirm",
                    ctx.t("settings.system.sunshine_confirm"),
                ))
                .on(move |ctx| {
                    match export(ctx, &all_apps) {
                        Ok((backup, skipped)) => {
                            if enable_sync {
                                ctx.settings.set_bool(keys::SUNSHINE_SYNC, true);
                            }
                            let path = backup.display().to_string();
                            let exported = ctx.tf("settings.system.sunshine_exported", &[("path", &path)]);
                            if skipped > 0 {
                                let skipped_msg = ctx.tf(
                                    "settings.system.sunshine_skipped_games",
                                    &[("count", &skipped.to_string())],
                                );
                                ctx.toast(format!("{exported} · {skipped_msg}"));
                            } else {
                                ctx.toast(exported);
                            }
                        }
                        Err(error) => ctx.toast(format_sunshine_error(ctx, &error)),
                    }
                    Flow::Close
                }),
            ]
        },
    )
    .text(text)
    .wide()
}

fn export(_ctx: &mut Ctx, apps: &[App]) -> Result<(PathBuf, usize), SunshineError> {
    let path = sunshine::find_apps_json().ok_or(SunshineError::SunshineFolderMissing)?;
    let (entries, skipped) = collect_entries(apps, Some(&path));
    let backup = sunshine::export(&path, &entries)?;
    Ok((backup, skipped))
}

/// Chamado apenas depois que uma leitura da biblioteca terminou e após opt-in confirmado.
pub(crate) fn sync_after_rescan(ctx: &mut Ctx) {
    if !ctx.settings.bool(keys::SUNSHINE_SYNC, false) {
        return;
    }
    let Some(path) = sunshine::find_apps_json() else {
        return;
    };
    let apps = match apps(ctx) {
        Ok(a) => a,
        Err(e) => {
            tracing::warn!("não foi possível obter apps do Tako para o Sunshine: {e}");
            return;
        }
    };
    let (entries, _) = collect_entries(&apps, Some(&path));
    crate::tasks::run(
        ctx,
        move || sunshine::export(&path, &entries),
        |ctx, result| match result {
            Ok(p) => {
                tracing::info!(path = %p.display(), "apps do Sunshine sincronizados após reexaminar a biblioteca")
            }
            Err(e) => {
                tracing::warn!("não foi possível sincronizar os apps do Sunshine: {e}");
                ctx.toast(format_sunshine_error(ctx, &e));
            }
        },
    );
}
