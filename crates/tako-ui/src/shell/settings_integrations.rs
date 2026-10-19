//! Categoria Integrações das Configurações: Console Mode e extensões locais.

use super::super::ctx::Ctx;
use super::super::layer::{Flow, MenuItem};
use crate::model::PanelRow;
use std::cell::{Cell, RefCell};
use tako_core::console_mode::ConsoleModeStatus;

thread_local! {
    static CACHED_STATUS: RefCell<Option<ConsoleModeStatus>> = const { RefCell::new(None) };
    static CHECKING: Cell<bool> = const { Cell::new(false) };
    #[cfg(test)]
    static CUSTOM_DETECTOR: Cell<Option<fn() -> ConsoleModeStatus>> = const { Cell::new(None) };
}

#[cfg(test)]
pub(crate) fn set_status_for_test(status: ConsoleModeStatus) {
    CACHED_STATUS.with(|c| *c.borrow_mut() = Some(status));
    CHECKING.set(false);
}

#[cfg(test)]
pub(crate) fn set_checking_for_test(checking: bool) {
    CHECKING.set(checking);
    if checking {
        CACHED_STATUS.with(|c| *c.borrow_mut() = None);
    }
}

#[cfg(test)]
pub(crate) fn set_detector_for_test(f: fn() -> ConsoleModeStatus) {
    CUSTOM_DETECTOR.with(|c| c.set(Some(f)));
}

fn do_detect() -> ConsoleModeStatus {
    #[cfg(test)]
    {
        if let Some(f) = CUSTOM_DETECTOR.get() {
            return f();
        }
        return ConsoleModeStatus::NaoInstalado;
    }
    #[allow(unreachable_code)]
    tako_platform::console_mode::detect()
}

pub(crate) fn check_now(ctx: &mut Ctx) {
    if CHECKING.get() {
        return;
    }
    CHECKING.set(true);
    ctx.dirty = true;
    crate::tasks::run(ctx, do_detect, |ctx, status| {
        CHECKING.set(false);
        CACHED_STATUS.with(|c| *c.borrow_mut() = Some(status));
        ctx.dirty = true;
    });
}

pub(crate) fn rows(ctx: &Ctx) -> Vec<MenuItem> {
    let cached = CACHED_STATUS.with(|cell| cell.borrow().clone());
    let checking = CHECKING.get();

    if cached.is_none() && !checking {
        CHECKING.set(true);
        crate::tasks::run_bg(do_detect, |_, status| {
            CHECKING.set(false);
            CACHED_STATUS.with(|c| *c.borrow_mut() = Some(status));
        });
    }

    let status_text = if checking {
        ctx.t("settings.integrations.console_mode_checking")
    } else {
        match cached.as_ref() {
            None => ctx.t("settings.integrations.console_mode_checking"),
            Some(ConsoleModeStatus::NaoInstalado) => {
                ctx.t("settings.integrations.console_mode_not_installed")
            }
            Some(ConsoleModeStatus::Instalado { versao_api, .. }) => ctx.tf(
                "settings.integrations.console_mode_installed",
                &[("version", &versao_api.to_string())],
            ),
            Some(ConsoleModeStatus::Incompativel { .. }) => {
                ctx.t("settings.integrations.console_mode_incompatible")
            }
            Some(ConsoleModeStatus::Erro) => ctx.t("settings.integrations.console_mode_error"),
        }
    };

    let status_style = match cached.as_ref() {
        Some(ConsoleModeStatus::Incompativel { .. }) => "warning",
        Some(ConsoleModeStatus::Erro) => "error",
        _ => "",
    };
    let mut rows = vec![
        MenuItem::new(
            PanelRow::action("console-mode", ctx.t("settings.integrations.console_mode"))
                .value(status_text)
                .style(status_style)
                .action_btn(ctx.t("settings.integrations.console_mode_check_again"))
                .enabled(!checking),
        )
        .on(|ctx| {
            check_now(ctx);
            Flow::Stay
        }),
    ];

    rows.extend(super::sunshine::rows(ctx));
    rows
}
