//! A integração é local: a prévia vem da biblioteca e a gravação só ocorre após confirmação.

use super::super::ctx::{Ctx, keys};
use super::super::layer::{Flow, MenuItem, MenuLayer};
use crate::model::PanelRow;
use std::path::PathBuf;
use tako_platform::sunshine::{self, App};

pub(crate) fn rows(ctx: &Ctx) -> Vec<MenuItem> {
    let detection = super::cached(
        "sunshine-detection",
        std::time::Duration::from_secs(5),
        sunshine::detect,
    );
    let status = match (detection.installed, detection.running) {
        (true, true) => ctx.t("settings.system.sunshine_running"),
        (true, false) => ctx.t("settings.system.sunshine_installed"),
        (false, _) => ctx.t("settings.system.sunshine_missing"),
    };
    let mut rows = vec![
        MenuItem::new(
            PanelRow::info("sunshine-status", ctx.t("settings.system.sunshine"), status)
                .desc(ctx.t("settings.system.sunshine_desc")),
        ),
        MenuItem::new(
            PanelRow::action("sunshine-export", ctx.t("settings.system.sunshine_export"))
                .desc(ctx.t("settings.system.sunshine_export_desc"))
                .value(ctx.t("settings.system.sunshine_export_value")),
        )
        .on(|ctx| Flow::Push(Box::new(preview_dialog(ctx, false)))),
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

fn apps(ctx: &Ctx) -> Result<Vec<sunshine::App>, String> {
    let executable = std::env::current_exe()
        .map_err(|e| format!("Não foi possível localizar o executável do Tako: {e}"))?
        .to_string_lossy()
        .into_owned();
    ctx.library
        .games()
        .iter()
        .map(|game| {
            Ok(App {
                id: game.id.clone(),
                title: game.title.clone(),
                executable: executable.clone(),
            })
        })
        .collect()
}

fn preview_dialog(ctx: &Ctx, enable_sync: bool) -> MenuLayer {
    let apps = match apps(ctx) {
        Ok(apps) => apps,
        Err(error) => {
            return super::text_dialog(ctx.t("settings.system.sunshine_export"), error);
        }
    };
    let titles = apps
        .iter()
        .take(40)
        .map(|app| format!("• {}", app.title))
        .collect::<Vec<_>>()
        .join("\n");
    let extra = apps.len().saturating_sub(40);
    let title_list = if extra > 0 {
        format!(
            "{titles}\n{}",
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
        &[("count", &apps.len().to_string()), ("games", &title_list)],
    );
    MenuLayer::dialog(
        ctx.t("settings.system.sunshine_preview_title"),
        move |ctx| {
            let apps = apps.clone();
            vec![
                MenuItem::new(PanelRow::button(
                    "sunshine-confirm",
                    ctx.t("settings.system.sunshine_confirm"),
                ))
                .on(move |ctx| {
                    match export(ctx, &apps) {
                        Ok(backup) => {
                            if enable_sync {
                                ctx.settings.set_bool(keys::SUNSHINE_SYNC, true);
                            }
                            let path = backup.display().to_string();
                            ctx.toast(
                                ctx.tf("settings.system.sunshine_exported", &[("path", &path)]),
                            );
                        }
                        Err(error) => ctx.toast(error),
                    }
                    Flow::Close
                }),
            ]
        },
    )
    .text(text)
    .wide()
}

fn export(ctx: &Ctx, apps: &[App]) -> Result<PathBuf, String> {
    let entries = apps
        .iter()
        .map(sunshine::app_entry)
        .collect::<Result<Vec<_>, _>>()?;
    let detection = sunshine::detect();
    let path = detection
        .apps_json
        .or_else(|| sunshine::apps_json_candidates().into_iter().next())
        .ok_or_else(|| ctx.t("settings.system.sunshine_path_missing"))?;
    sunshine::export(&path, &entries)
}

/// Chamado apenas depois que uma leitura da biblioteca terminou e após opt-in confirmado.
pub(crate) fn sync_after_rescan(ctx: &mut Ctx) {
    if !ctx.settings.bool(keys::SUNSHINE_SYNC, false) {
        return;
    }
    let result = apps(ctx).and_then(|apps| export(ctx, &apps));
    match result {
        Ok(path) => {
            tracing::info!(path = %path.display(), "apps do Sunshine sincronizados após reexaminar a biblioteca")
        }
        Err(error) => {
            tracing::warn!("não foi possível sincronizar os apps do Sunshine: {error}");
            ctx.toast(error);
        }
    }
}
