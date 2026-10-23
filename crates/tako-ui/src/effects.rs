//! Pedidos da interface que não dependem de como o app roda (com ou sem supervisor): abrir endereços e
//! pastas, cache de imagens, relatório do sistema, atualizações e troca de tema.

use crate::app::HostAction;
use crate::shell::layer::{Flow, MenuItem, MenuLayer};
use crate::shell::{Ctx, Effect, Shell};
use crate::{images, model::PanelRow, tasks};
use std::path::{Path, PathBuf};
use tako_core::paths::{DataPaths, Os};
use tako_core::updates::{self, UpdatePreferences, feed};

/// `None` = o host trata.
pub fn common(effect: &Effect, shell: &mut Shell) -> Option<HostAction> {
    let ctx = &mut shell.ctx;
    match effect {
        Effect::OpenUrl(u) => {
            if let Err(e) = ctx.platform.processes.launch(u, None, None) {
                ctx.toast(e.to_string());
            }
        }
        Effect::OpenFolder(p) => {
            let _ = std::fs::create_dir_all(p);
            if let Err(e) = ctx.platform.processes.open_folder(p) {
                ctx.toast(e.to_string());
            }
        }
        Effect::ClearArtCache => {
            let freed = ctx.art.clear();
            images::clear();
            ctx.toast(ctx.tf("effects.cache_cleared", &[("size", &format_bytes(freed))]));
        }
        Effect::SaveDiagnostics => save_report(ctx),
        Effect::CheckUpdates => check_updates(ctx, true),
        Effect::InstallUpdate => return Some(install_update(ctx, true)),
        Effect::ExitToDesktop
            if ctx.desktop
                && ctx.update_ready.is_some()
                && ctx.running.is_none()
                && !ctx.app_update.installing =>
        {
            return Some(install_update(ctx, false));
        }
        Effect::ApplyTheme(_) => return Some(HostAction::ReloadTheme),
        Effect::SwitchMode(mode) => return Some(HostAction::SwitchMode(*mode)),
        Effect::Window(action) => return Some(HostAction::Window(action)),
        Effect::TestGameScreen => crate::screen_test::start(ctx),
        Effect::ImportPlaynite => crate::playnite::start(ctx),
        Effect::AddGame => crate::shell::add_game::add_game(ctx),
        Effect::InstallTheme => install_theme(ctx),
        Effect::RefreshMetadata(id) => crate::metadata::refresh(ctx, id.as_deref()),
        _ => return None,
    }
    Some(HostAction::Stay)
}

pub fn format_bytes(n: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut v = n as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{n} B")
    } else {
        format!("{v:.1} {}", UNITS[u])
    }
}

fn package(paths: &DataPaths) -> feed::Package {
    let exe = std::env::current_exe().unwrap_or_default();
    let appimage = std::env::var_os("APPIMAGE").map(PathBuf::from);
    let flatpak = std::env::var_os("FLATPAK_ID").is_some() || Path::new("/.flatpak-info").exists();
    feed::Package::detect(
        Os::current(),
        paths.kind,
        &exe,
        appimage.as_deref(),
        flatpak,
    )
}

/// Procura (e, se puder instalar, baixa) a próxima versão. `explicit` = pedido do usuário (mostra o resultado).
pub fn check_updates(ctx: &mut Ctx, explicit: bool) {
    if ctx.simulated() || cfg!(debug_assertions) && !explicit {
        return;
    }
    let Some(arch) = feed::Arch::current() else {
        return;
    };
    let pkg = package(&ctx.paths);
    if ctx.desktop {
        crate::app_updates::check(ctx, explicit, pkg, arch);
        return;
    }
    let prefs = UpdatePreferences::load(&ctx.paths.updates());
    ctx.update_status = ctx.t("updates.checking");
    tasks::run(
        ctx,
        move || {
            let agent = updates::agent();
            let found = updates::check(
                &agent,
                &updates::SemVer::current(),
                prefs.channel(),
                pkg,
                arch,
            )
            .map_err(|e| e.to_string())?;
            let Some(info) = found else { return Ok(None) };
            if !info.can_install() || !pkg.self_updates() {
                return Ok(Some((info, None)));
            }
            let path = updates::download(&agent, &info, &updates::download_dir(), &mut |_| {})
                .map_err(|e| e.to_string())?;
            Ok::<_, String>(Some((info, Some(path))))
        },
        move |ctx, result| match result {
            Ok(None) => {
                ctx.update_status = ctx.t("updates.up_to_date_toast");
                if explicit {
                    ctx.toast(ctx.update_status.clone());
                }
            }
            Ok(Some((info, None))) => {
                ctx.update_status = ctx.tf("updates.available_page", &[("version", &info.version)]);
                if explicit {
                    ctx.effect(Effect::OpenUrl(info.page_url.clone()));
                }
            }
            Ok(Some((info, Some(path)))) => {
                ctx.update_status = ctx.tf("updates.ready", &[("version", &info.version)]);
                ctx.update_ready = Some(path.to_string_lossy().into_owned());
                ctx.pending_layer = Some(Box::new(ready_dialog(ctx, &info.version)));
            }
            Err(e) => {
                tracing::warn!("atualizações: {e}");
                ctx.update_status = ctx.t("updates.check_failed");
                if explicit {
                    ctx.toast(ctx.update_status.clone());
                }
            }
        },
    );
}

fn ready_dialog(ctx: &Ctx, version: &str) -> MenuLayer {
    MenuLayer::dialog(ctx.tf("updates.ready", &[("version", version)]), |ctx| {
        vec![
            MenuItem::new(PanelRow::button("now", ctx.t("updates.restart"))).on(|ctx| {
                ctx.effect(Effect::InstallUpdate);
                Flow::Close
            }),
            MenuItem::new(PanelRow::button("later", ctx.t("updates.later")).link())
                .on(|_| Flow::Close),
        ]
    })
}

/// Roda o plano de troca (instalador ou script) e fecha o Tako; o script espera o Tako sair.
fn install_update(ctx: &mut Ctx, relaunch: bool) -> HostAction {
    let Some(file) = ctx.update_ready.clone() else {
        return HostAction::Stay;
    };
    if ctx.running.is_some() {
        ctx.toast(ctx.t("updates.finish_game"));
        return HostAction::Stay;
    }
    let exe = std::env::current_exe().unwrap_or_default();
    let pkg = package(&ctx.paths);
    let target = match pkg {
        feed::Package::MacApp => updates::mac_bundle_of(&exe).unwrap_or(exe.clone()),
        feed::Package::LinuxAppImage => std::env::var_os("APPIMAGE")
            .map(PathBuf::from)
            .unwrap_or(exe.clone()),
        feed::Package::LinuxTarGz => exe.parent().map(PathBuf::from).unwrap_or_default(),
        _ => exe.clone(),
    };
    let mut wait = vec![std::process::id()];
    if let Some(parent) = supervisor_pid() {
        wait.push(parent);
    }
    let Some(plan) =
        updates::UpdateApplyPlan::build(pkg, std::path::Path::new(&file), &target, &wait, relaunch)
    else {
        return HostAction::Stay;
    };
    if let Some((path, body)) = &plan.script
        && let Err(e) = std::fs::write(path, body)
    {
        ctx.toast(e.to_string());
        return HostAction::Stay;
    }
    match std::process::Command::new(&plan.program)
        .args(&plan.args)
        .spawn()
    {
        Ok(_) => {
            ctx.app_update.installing = true;
            ctx.effect(Effect::ExitToDesktop);
            HostAction::Stay
        }
        Err(e) => {
            ctx.toast(e.to_string());
            HostAction::Stay
        }
    }
}

/// PID do supervisor (definido por ele no ambiente da interface).
fn supervisor_pid() -> Option<u32> {
    std::env::var("TAKO_SUPERVISOR_PID").ok()?.parse().ok()
}

/// Relatório do sistema em `logs/diagnostico-*.txt` (sem dados pessoais: passa pela redação dos logs).
fn save_report(ctx: &mut Ctx) {
    use tako_core::diagnostics::{LogRedactor, report};
    let now = chrono::Local::now().fixed_offset();
    let displays = ctx.platform.displays.list().unwrap_or_default();
    let snapshot = report::SystemSnapshot {
        generated_at: now,
        tako_version: ctx.version.clone(),
        install_kind: ctx.t(
            if ctx.paths.kind == tako_core::paths::InstallKind::Portable {
                "effects.portable"
            } else {
                "effects.installed"
            },
        ),
        data_folder: Some(ctx.paths.root.display().to_string()),
        os_name: Some(std::env::consts::OS.into()),
        architecture: std::env::consts::ARCH.into(),
        desktop_session: std::env::var("XDG_CURRENT_DESKTOP").ok(),
        logical_processors: std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1),
        displays: displays
            .iter()
            .map(|d| report::DisplayInfo {
                device: d.name.clone(),
                width: d.current_mode.map(|m| m.width).unwrap_or(0),
                height: d.current_mode.map(|m| m.height).unwrap_or(0),
                refresh_hz: d.current_mode.map(|m| m.refresh_hz),
                is_primary: d.is_primary,
                adapter: None,
                scale_percent: None,
            })
            .collect(),
        controllers: if ctx.pad_name.is_empty() {
            Vec::new()
        } else {
            vec![ctx.pad_name.clone()]
        },
        active_controller: (!ctx.pad_name.is_empty()).then(|| ctx.pad_name.clone()),
        steam_found: ctx
            .library
            .games()
            .iter()
            .any(|g| g.store() == tako_core::library::GameStore::Steam),
        steam_games: ctx
            .library
            .games()
            .iter()
            .filter(|g| g.store() == tako_core::library::GameStore::Steam)
            .count(),
        manual_games: ctx
            .library
            .games()
            .iter()
            .filter(|g| g.store() == tako_core::library::GameStore::Manual)
            .count(),
        total_games: ctx.library.games().len(),
        safe_mode: ctx.safe_mode,
        theme: tako_themes::selected(&ctx.paths.themes()),
        ..Default::default()
    };
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok();
    let redactor = LogRedactor::new(home.as_deref());
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .ok();
    let text = report::format(&snapshot, &redactor, user.as_deref(), None, ctx.lang());
    let path = ctx.paths.logs().join(report::file_name(now));
    let _ = std::fs::create_dir_all(ctx.paths.logs());
    match std::fs::write(&path, &text) {
        Ok(()) => {
            let shown = path.display().to_string();
            ctx.pending_layer = Some(Box::new(
                MenuLayer::dialog(ctx.t("effects.report_title"), move |ctx| {
                    let p = shown.clone();
                    vec![
                        MenuItem::new(PanelRow::button("folder", ctx.t("options.open_folder"))).on(
                            move |ctx| {
                                let dir = std::path::Path::new(&p)
                                    .parent()
                                    .map(|d| d.display().to_string())
                                    .unwrap_or_default();
                                ctx.effect(Effect::OpenFolder(dir));
                                Flow::Close
                            },
                        ),
                        MenuItem::new(PanelRow::button("close", ctx.t("hint.close")))
                            .on(|_| Flow::Close),
                    ]
                })
                .text(ctx.tf(
                    "effects.report_saved",
                    &[("path", &path.display().to_string())],
                ))
                .wide(),
            ));
        }
        Err(e) => ctx.toast(e.to_string()),
    }
}

/// Aparência › Instalar tema…: escolhe um `.takotheme` e o instala ao lado dos outros (não o aplica).
fn install_theme(ctx: &mut Ctx) {
    let title = ctx.t("settings.appearance.install");
    crate::shell::add_game::pick(
        ctx,
        move || {
            rfd::FileDialog::new()
                .set_title(&title)
                .add_filter("Tako", &["takotheme"])
                .pick_file()
        },
        |ctx, path| match tako_themes::install(&path, &ctx.paths.themes()) {
            Ok(t) => {
                crate::shell::settings::forget(crate::shell::settings::THEMES_CACHE);
                let name = t
                    .manifest
                    .as_ref()
                    .map(|m| m.name.clone())
                    .unwrap_or_else(|_| t.id());
                ctx.toast(ctx.tf("effects.theme_installed", &[("name", &name)]));
            }
            Err(problems) => {
                let text: Vec<String> = problems.into_iter().map(|p| p.message).collect();
                ctx.toast(text.join(" "));
            }
        },
    );
}
