//! Configurações › Capas e metadados: IGDB, ScreenScraper e SteamGridDB (chaves no cofre do SO), capas e
//! manutenção do cache.

use super::{Ctx, Effect, Flow, MenuItem, MenuLayer, action, cached, described, effect, forget};
use crate::model::PanelRow;
use crate::shell::keyboard::KeyboardLayer;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use tako_core::metadata::SecretStore;
use tako_core::metadata::art::format_bytes;
use tako_core::metadata::http::{SystemClock, UreqHttp};
use tako_core::metadata::igdb::{IgdbClient, IgdbSettings};
use tako_core::metadata::screenscraper::{
    ScreenScraperClient, ScreenScraperSettings, ScreenScraperSoftware,
};
use tako_core::metadata::steamgriddb::{SteamGridDbClient, SteamGridDbSettings};
use tako_platform::secrets::Secrets;

/// O cofre da plataforma visto como o `SecretStore` do core.
pub(crate) struct Vault<'a>(pub &'a dyn Secrets);

impl SecretStore for Vault<'_> {
    fn get(&self, key: &str) -> Option<String> {
        self.0.get(key)
    }
    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        self.0.set(key, value)
    }
}

#[derive(Clone, Default, PartialEq)]
pub(crate) struct Providers {
    pub igdb: IgdbSettings,
    pub ss: ScreenScraperSettings,
    pub sgdb: SteamGridDbSettings,
}

const CACHE: &str = "settings.providers";

/// Lidas do cofre uma vez (o cofre do Linux é um serviço D-Bus; ler a cada tecla travaria a tela).
pub(crate) fn providers(ctx: &Ctx) -> Providers {
    cached(CACHE, Duration::MAX, || {
        let vault = Vault(ctx.platform.secrets.as_ref());
        Providers {
            igdb: IgdbSettings::load(&vault),
            ss: ScreenScraperSettings::load(&vault),
            sgdb: SteamGridDbSettings::load(&vault),
        }
    })
}

/// Muda e grava só o provedor que mudou.
pub(crate) fn update(ctx: &mut Ctx, change: impl FnOnce(&mut Providers)) {
    let old = providers(ctx);
    let mut new = old.clone();
    change(&mut new);
    let vault = Vault(ctx.platform.secrets.as_ref());
    let mut result = Ok(());
    if new.igdb != old.igdb {
        result = result.and(new.igdb.save(&vault));
    }
    if new.ss != old.ss {
        result = result.and(new.ss.save(&vault));
    }
    if new.sgdb != old.sgdb {
        result = result.and(new.sgdb.save(&vault));
    }
    forget(CACHE);
    if let Err(e) = result {
        ctx.toast(ctx.tf("settings.metadata.save_failed", &[("error", &e)]));
    }
    ctx.dirty = true;
}

fn software() -> ScreenScraperSoftware {
    ScreenScraperSoftware::from_build()
}

fn summary(ctx: &Ctx, enabled: bool, active: bool) -> String {
    ctx.t(match (enabled, active) {
        (true, true) => "settings.metadata.on",
        (true, false) => "settings.metadata.on_missing",
        _ => "settings.metadata.off",
    })
}

fn http() -> Arc<UreqHttp> {
    Arc::new(UreqHttp::new(Duration::from_secs(15)))
}

pub fn rows(ctx: &Ctx) -> Vec<MenuItem> {
    let p = providers(ctx);
    let ss_summary = if software().is_available() {
        summary(ctx, p.ss.enabled, p.ss.is_active())
    } else {
        ctx.t("settings.metadata.unavailable")
    };
    let art = ctx.art.clone();
    let cache_size = cached("settings.art_bytes", Duration::from_secs(5), move || {
        art.disk_bytes()
    });
    let sgdb_active = p.sgdb.is_active();
    vec![
        MenuItem::new(PanelRow::header(ctx.t("settings.metadata.sources_header"))),
        MenuItem::new(described(ctx, "settings.metadata.igdb").value(summary(
            ctx,
            p.igdb.enabled,
            p.igdb.is_active(),
        )))
        .on(|ctx| Flow::Push(Box::new(igdb_panel(ctx)))),
        MenuItem::new(described(ctx, "settings.metadata.screenscraper").value(ss_summary))
            .on(|ctx| Flow::Push(Box::new(screenscraper_panel(ctx)))),
        MenuItem::new(
            described(ctx, "settings.metadata.steamgriddb").value(summary(
                ctx,
                p.sgdb.enabled,
                sgdb_active,
            )),
        )
        .on(|ctx| Flow::Push(Box::new(steamgriddb_panel(ctx)))),
        MenuItem::new(PanelRow::header(ctx.t("settings.metadata.covers_header"))),
        MenuItem::new(
            PanelRow::toggle(
                "sgdb-autofill",
                ctx.t("settings.metadata.autofill"),
                sgdb_active && p.sgdb.auto_fill,
            )
            .desc(ctx.t(if sgdb_active {
                "settings.metadata.autofill_desc"
            } else {
                "settings.metadata.autofill_off_desc"
            }))
            .enabled(sgdb_active),
        )
        .on(|ctx| {
            update(ctx, |p| p.sgdb.auto_fill = !p.sgdb.auto_fill);
            Flow::Stay
        }),
        MenuItem::new(
            PanelRow::info(
                "art-order",
                ctx.t("settings.metadata.order"),
                "Steam › IGDB › ScreenScraper › SteamGridDB",
            )
            .desc(ctx.t("settings.metadata.order_desc")),
        ),
        MenuItem::new(PanelRow::header(
            ctx.t("settings.metadata.maintenance_header"),
        )),
        MenuItem::new(action(ctx, "settings.metadata.refresh"))
            .on(effect(Effect::RefreshMetadata(None))),
        MenuItem::new(described(ctx, "settings.metadata.clear").value(format_bytes(cache_size)))
            .on(|ctx| Flow::Push(Box::new(confirm_clear(ctx)))),
    ]
}

fn confirm_clear(ctx: &Ctx) -> MenuLayer {
    MenuLayer::dialog(ctx.t("settings.metadata.clear"), |ctx| {
        vec![
            MenuItem::new(
                PanelRow::button("clear", ctx.t("settings.metadata.clear_confirm")).danger(),
            )
            .on(|ctx| {
                let freed = ctx.art.clear();
                crate::images::clear();
                forget("settings.art_bytes");
                ctx.toast(ctx.tf(
                    "settings.metadata.cleared",
                    &[("size", &format_bytes(freed))],
                ));
                Flow::Close
            }),
            MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel"))).on(|_| Flow::Close),
        ]
    })
    .text(ctx.t("settings.metadata.clear_desc"))
}

/// Linha de texto que abre o teclado. Segredo: o campo começa vazio e vazio mantém o salvo.
fn field(
    ctx: &Ctx,
    id: &str,
    title: String,
    current: &str,
    secret: bool,
    set: fn(&mut Providers, &str),
) -> MenuItem {
    let shown = match (current.is_empty(), secret) {
        (true, _) => ctx.t("settings.metadata.not_set"),
        (false, true) => "••••••••".into(),
        (false, false) => current.to_owned(),
    };
    let start = if secret {
        String::new()
    } else {
        current.to_owned()
    };
    let t = title.clone();
    MenuItem::new(PanelRow::new(id, "text", title).value(shown)).on(move |ctx| {
        let k = KeyboardLayer::new(ctx, t.clone(), &start, move |ctx, text| {
            if !(secret && text.is_empty()) {
                let text = tako_core::metadata::clean_key(text);
                update(ctx, |p| set(p, &text));
            }
            Flow::Close
        });
        Flow::Push(Box::new(if secret {
            k.secret().hint(ctx.t("settings.metadata.keep_secret"))
        } else {
            k
        }))
    })
}

type Status = Rc<RefCell<String>>;

/// "Testar": roda o teste fora da UI e mostra o resultado na linha e num aviso.
fn test_row<R: Send + 'static>(
    ctx: &Ctx,
    status: &Status,
    problem: Option<String>,
    job: impl Fn(Providers) -> R + Clone + 'static + Send,
    show: fn(&Ctx, R) -> String,
) -> MenuItem {
    let value = status.borrow().clone();
    let status = status.clone();
    MenuItem::new(
        PanelRow::action("test", ctx.t("settings.metadata.test"))
            .desc(if value.is_empty() {
                ctx.t("settings.metadata.test_desc")
            } else {
                value
            })
            .value(ctx.t("settings.metadata.test_value")),
    )
    .on(move |ctx| {
        if let Some(p) = &problem {
            *status.borrow_mut() = p.clone();
            ctx.toast(p.clone());
            return Flow::Stay;
        }
        *status.borrow_mut() = ctx.t("settings.metadata.testing");
        let (s, p, job) = (status.clone(), providers(ctx), job.clone());
        crate::tasks::run(
            ctx,
            move || job(p),
            move |ctx, r| {
                let text = show(ctx, r);
                *s.borrow_mut() = text.clone();
                ctx.toast(text);
            },
        );
        Flow::Stay
    })
}

fn enabled_row(ctx: &Ctx, text: &str, on: bool, flip: fn(&mut Providers)) -> MenuItem {
    MenuItem::new(PanelRow::toggle("enabled", ctx.t(text), on)).on(move |ctx| {
        update(ctx, flip);
        Flow::Stay
    })
}

fn igdb_panel(ctx: &Ctx) -> MenuLayer {
    let status: Status = Rc::default();
    MenuLayer::panel(ctx.t("settings.metadata.igdb"), move |ctx| {
        let p = providers(ctx).igdb;
        vec![
            enabled_row(ctx, "settings.metadata.igdb_enabled", p.enabled, |p| {
                p.igdb.enabled = !p.igdb.enabled
            }),
            field(
                ctx,
                "client-id",
                "Client ID".into(),
                &p.client_id,
                false,
                |p, v| p.igdb.client_id = v.into(),
            ),
            field(
                ctx,
                "client-secret",
                "Client Secret".into(),
                &p.client_secret,
                true,
                |p, v| p.igdb.client_secret = v.into(),
            ),
            test_row(
                ctx,
                &status,
                p.problem(),
                |p| {
                    IgdbClient::test(http(), &p.igdb, Arc::new(SystemClock))
                        .map_err(|e| e.failure.message())
                },
                |ctx, r| {
                    r.map(|()| ctx.t("settings.metadata.test_ok"))
                        .unwrap_or_else(|e| e)
                },
            ),
            MenuItem::new(
                PanelRow::action("site", ctx.t("settings.metadata.igdb_site"))
                    .value("dev.twitch.tv"),
            )
            .on(super::effect(Effect::OpenUrl(
                "https://dev.twitch.tv/console/apps".into(),
            ))),
        ]
    })
    .text(ctx.t("settings.metadata.igdb_steps"))
}

fn screenscraper_panel(ctx: &Ctx) -> MenuLayer {
    let status: Status = Rc::default();
    MenuLayer::panel(ctx.t("settings.metadata.screenscraper"), move |ctx| {
        if !software().is_available() {
            return vec![MenuItem::new(
                PanelRow::info(
                    "unavailable",
                    ctx.t("settings.metadata.screenscraper"),
                    ctx.t("settings.metadata.unavailable"),
                )
                .desc(ctx.t("settings.metadata.unavailable_desc")),
            )];
        }
        let p = providers(ctx).ss;
        vec![
            enabled_row(ctx, "settings.metadata.ss_enabled", p.enabled, |p| {
                p.ss.enabled = !p.ss.enabled
            }),
            field(
                ctx,
                "user",
                ctx.t("settings.metadata.ss_user"),
                &p.user,
                false,
                |p, v| p.ss.user = v.into(),
            ),
            field(
                ctx,
                "password",
                ctx.t("settings.metadata.ss_password"),
                &p.password,
                true,
                |p, v| p.ss.password = v.into(),
            ),
            test_row(
                ctx,
                &status,
                p.problem(),
                |p| {
                    ScreenScraperClient::test(http(), software(), &p.ss, Arc::new(SystemClock))
                        .map_err(|e| e.failure.message())
                },
                |ctx, r| match r {
                    Ok(a) => ctx.tf(
                        "settings.metadata.ss_ok",
                        &[
                            ("user", &a.user),
                            ("level", &a.level.to_string()),
                            ("left", &a.remaining_today().to_string()),
                            ("max", &a.max_requests_per_day.to_string()),
                            ("threads", &a.max_threads.to_string()),
                        ],
                    ),
                    Err(e) => e,
                },
            ),
            MenuItem::new(
                PanelRow::action("site", ctx.t("settings.metadata.ss_site"))
                    .value("screenscraper.fr"),
            )
            .on(super::effect(Effect::OpenUrl(
                "https://www.screenscraper.fr/membreinscription.php".into(),
            ))),
        ]
    })
    .text(ctx.t("settings.metadata.ss_steps"))
}

fn steamgriddb_panel(ctx: &Ctx) -> MenuLayer {
    let status: Status = Rc::default();
    MenuLayer::panel(ctx.t("settings.metadata.steamgriddb"), move |ctx| {
        let p = providers(ctx).sgdb;
        vec![
            enabled_row(ctx, "settings.metadata.sgdb_enabled", p.enabled, |p| {
                p.sgdb.enabled = !p.sgdb.enabled
            }),
            field(
                ctx,
                "api-key",
                ctx.t("settings.metadata.sgdb_key"),
                &p.api_key,
                true,
                |p, v| p.sgdb.api_key = v.into(),
            ),
            test_row(
                ctx,
                &status,
                p.problem(),
                |p| {
                    SteamGridDbClient::test(http(), &p.sgdb, Arc::new(SystemClock))
                        .map_err(|e| e.failure.message())
                },
                |ctx, r| {
                    r.map(|()| ctx.t("settings.metadata.test_ok"))
                        .unwrap_or_else(|e| e)
                },
            ),
            MenuItem::new(
                PanelRow::action("site", ctx.t("settings.metadata.sgdb_site"))
                    .value("steamgriddb.com"),
            )
            .on(super::effect(Effect::OpenUrl(
                "https://www.steamgriddb.com/profile/preferences/api".into(),
            ))),
        ]
    })
    .text(ctx.t("settings.metadata.sgdb_steps"))
}
