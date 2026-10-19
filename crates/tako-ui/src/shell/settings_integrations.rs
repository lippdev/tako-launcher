//! Categoria Integrações das Configurações: Console Mode, Discord e API local.

use super::super::ctx::{Ctx, keys};
use super::super::layer::{Flow, MenuItem};
use super::api_panel;
use crate::model::PanelRow;
use std::cell::{Cell, RefCell};
use std::time::{Duration, Instant};
use tako_core::console_mode::ConsoleModeStatus;

thread_local! {
    static CACHED_STATUS: RefCell<Option<(Instant, ConsoleModeStatus)>> = const { RefCell::new(None) };
    static CHECKING: Cell<bool> = const { Cell::new(false) };
}

/// Invalida o cache da detecção (útil em testes ou ao trocar de tela).
#[allow(dead_code)]
pub(crate) fn invalidate_cache() {
    CACHED_STATUS.with(|c| *c.borrow_mut() = None);
}

/// Define o status em cache diretamente (útil para testes de UI e simulação).
#[allow(dead_code)]
pub(crate) fn set_status_for_test(status: ConsoleModeStatus) {
    CACHED_STATUS.with(|c| *c.borrow_mut() = Some((Instant::now(), status)));
}

pub(crate) fn check_now(ctx: &mut Ctx) {
    if CHECKING.get() {
        return;
    }
    CHECKING.set(true);
    ctx.dirty = true;
    crate::tasks::run(
        ctx,
        tako_platform::console_mode::detect,
        |ctx, status| {
            CHECKING.set(false);
            CACHED_STATUS.with(|c| *c.borrow_mut() = Some((Instant::now(), status)));
            ctx.dirty = true;
        },
    );
}

pub(crate) fn rows(ctx: &Ctx) -> Vec<MenuItem> {
    let checking = CHECKING.get();
    let status = CACHED_STATUS.with(|cell| {
        let borrow = cell.borrow();
        if let Some((at, st)) = borrow.as_ref()
            && at.elapsed() < Duration::from_secs(5)
        {
            st.clone()
        } else {
            ConsoleModeStatus::NaoInstalado
        }
    });

    let status_text = if checking {
        ctx.t("settings.integrations.console_mode_checking")
    } else {
        match status {
            ConsoleModeStatus::NaoInstalado => {
                ctx.t("settings.integrations.console_mode_not_installed")
            }
            ConsoleModeStatus::Instalado { versao_api, .. } => ctx.tf(
                "settings.integrations.console_mode_installed",
                &[("version", &versao_api.to_string())],
            ),
            ConsoleModeStatus::Incompativel { .. } => {
                ctx.t("settings.integrations.console_mode_incompatible")
            }
            ConsoleModeStatus::Erro => ctx.t("settings.integrations.console_mode_error"),
        }
    };

    let mut rows = vec![MenuItem::new(
        PanelRow::action("console-mode", ctx.t("settings.integrations.console_mode"))
            .desc(status_text)
            .value(ctx.t("settings.integrations.console_mode_check_again")),
    )
    .on(|ctx| {
        check_now(ctx);
        Flow::Stay
    })];

    if crate::discord::available() {
        rows.push(super::toggle(
            ctx,
            "settings.system.discord",
            keys::DISCORD_PRESENCE,
            false,
        ));
    }

    let api_on = ctx.settings.bool(keys::API_ENABLED, false);
    rows.push(
        MenuItem::new(
            PanelRow::action("settings.system.api", ctx.t("settings.system.api"))
                .desc(ctx.t("settings.system.api_desc"))
                .value(ctx.t(if api_on {
                    "settings.system.api_on"
                } else {
                    "settings.system.api_off"
                })),
        )
        .on(|ctx| Flow::Push(Box::new(api_panel(ctx)))),
    );

    rows
}
