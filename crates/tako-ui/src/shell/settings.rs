//! Configurações (ADR-0020): coluna de categorias à esquerda e as linhas da categoria à direita. LB/RB trocam
//! de categoria; → ou A entram nas linhas; B nas linhas volta à categoria e B na coluna sai.

#[path = "settings_controls.rs"]
mod controls;
#[path = "settings_library.rs"]
pub(crate) mod library;
#[path = "settings_metadata.rs"]
pub(crate) mod metadata;
#[path = "settings_screen.rs"]
mod screen;
#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;

use super::ctx::keys;
use super::layer::{Flow, MenuItem, MenuLayer};
use super::{Cmd, Ctx, Effect, Nav, Out, Page};
use crate::model::{Hint, NavItem, PanelRow};
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};
use tako_core::i18n::{self, Lang};
use tako_core::updates::UpdatePreferences;

/// Uma categoria: id, ícone e as linhas (reconstruídas a cada desenho).
pub struct Category {
    pub id: &'static str,
    pub icon: &'static str,
    pub rows: fn(&Ctx) -> Vec<MenuItem>,
}

pub fn categories() -> Vec<Category> {
    vec![
        Category {
            id: "general",
            icon: "gear",
            rows: general,
        },
        Category {
            id: "library",
            icon: "grid",
            rows: library::rows,
        },
        Category {
            id: "metadata",
            icon: "image",
            rows: metadata::rows,
        },
        Category {
            id: "appearance",
            icon: "palette",
            rows: appearance,
        },
        Category {
            id: "screen",
            icon: "monitor",
            rows: screen::rows,
        },
        Category {
            id: "controls",
            icon: "gamepad",
            rows: controls::rows,
        },
        Category {
            id: "system",
            icon: "alert",
            rows: system,
        },
        Category {
            id: "about",
            icon: "info",
            rows: about,
        },
    ]
}

type Cache = HashMap<&'static str, (Instant, Rc<dyn Any>)>;

thread_local! {
    static CACHE: RefCell<Cache> = RefCell::new(HashMap::new());
}

/// As linhas são refeitas a cada tecla; o que custa (telas, cofre, disco) é lido no máximo a cada `ttl`.
pub(crate) fn cached<T: Clone + 'static>(
    name: &'static str,
    ttl: Duration,
    read: impl FnOnce() -> T,
) -> T {
    let hit = CACHE.with(|c| {
        c.borrow()
            .get(name)
            .filter(|(at, _)| at.elapsed() < ttl)
            .and_then(|(_, v)| v.downcast_ref::<T>().cloned())
    });
    hit.unwrap_or_else(|| {
        let v = read();
        CACHE.with(|c| {
            c.borrow_mut()
                .insert(name, (Instant::now(), Rc::new(v.clone())))
        });
        v
    })
}

pub(crate) fn forget(name: &'static str) {
    CACHE.with(|c| c.borrow_mut().remove(name));
}

/// Liga/desliga de uma configuração simples: título em `<text>`, descrição em `<text>_desc`.
pub(crate) fn toggle(ctx: &Ctx, text: &str, key: &'static str, default: bool) -> MenuItem {
    MenuItem::new(
        PanelRow::toggle(text, ctx.t(text), ctx.settings.bool(key, default))
            .desc(ctx.t(&format!("{text}_desc"))),
    )
    .on(move |ctx| {
        ctx.settings.toggle(key, default);
        Flow::Stay
    })
}

/// Ação com título e descrição (`<text>`, `<text>_desc`); o valor vem de quem chama.
pub(crate) fn described(ctx: &Ctx, text: &str) -> PanelRow {
    PanelRow::action(text, ctx.t(text)).desc(ctx.t(&format!("{text}_desc")))
}

/// Ação com título, descrição e valor fixo (`<text>_value`).
pub(crate) fn action(ctx: &Ctx, text: &str) -> PanelRow {
    described(ctx, text).value(ctx.t(&format!("{text}_value")))
}

/// Diálogo só de leitura com "Fechar".
pub(crate) fn text_dialog(title: String, text: String) -> MenuLayer {
    MenuLayer::dialog(title, |ctx| {
        vec![MenuItem::new(PanelRow::button("close", ctx.t("hint.close"))).on(|_| Flow::Close)]
    })
    .text(text)
    .wide()
}

fn effect(e: Effect) -> impl Fn(&mut Ctx) -> Flow {
    move |ctx| {
        ctx.effect(e.clone());
        Flow::Stay
    }
}

// ---------------------------------------------------------------- Geral

pub(crate) fn lang_name(lang: Lang) -> &'static str {
    match lang {
        Lang::PtBr => "Português (Brasil)",
        Lang::EnUs => "English (US)",
    }
}

fn update_toggle(
    ctx: &Ctx,
    text: &str,
    get: fn(&UpdatePreferences) -> bool,
    flip: fn(&mut UpdatePreferences),
) -> MenuItem {
    let prefs = UpdatePreferences::load(&ctx.paths.updates());
    MenuItem::new(
        PanelRow::toggle(text, ctx.t(text), get(&prefs)).desc(ctx.t(&format!("{text}_desc"))),
    )
    .on(move |ctx| {
        let path = ctx.paths.updates();
        let mut p = UpdatePreferences::load(&path);
        flip(&mut p);
        if let Err(e) = p.save(&path) {
            ctx.toast(e.to_string());
        }
        Flow::Stay
    })
}

fn general(ctx: &Ctx) -> Vec<MenuItem> {
    let status = if ctx.update_status.is_empty() {
        ctx.t("settings.general.updates_desc")
    } else {
        ctx.update_status.clone()
    };
    vec![
        MenuItem::new(
            PanelRow::toggle(
                "startup",
                ctx.t("settings.general.startup"),
                ctx.platform.autostart.is_enabled(),
            )
            .desc(ctx.t("settings.general.startup_desc")),
        )
        .on(|ctx| {
            let on = !ctx.platform.autostart.is_enabled();
            if let Err(e) = ctx.platform.autostart.set_enabled(on) {
                ctx.toast(e.to_string());
            }
            Flow::Stay
        }),
        MenuItem::new(
            PanelRow::choice(
                "language",
                ctx.t("settings.general.language"),
                lang_name(ctx.lang()),
            )
            .desc(ctx.t("settings.general.language_desc")),
        )
        .on_adjust(|ctx, _| {
            let next = match ctx.lang() {
                Lang::PtBr => Lang::EnUs,
                Lang::EnUs => Lang::PtBr,
            };
            ctx.settings.set(keys::LANGUAGE, next.code());
            i18n::set_lang(next);
            ctx.dirty = true;
            Flow::Stay
        }),
        MenuItem::new(PanelRow::header(ctx.t("settings.general.updates_header"))),
        MenuItem::new(
            PanelRow::action("updates", ctx.t("settings.general.updates"))
                .desc(status)
                .value(ctx.t("settings.general.updates_value")),
        )
        .on(effect(Effect::CheckUpdates)),
        update_toggle(
            ctx,
            "settings.general.autoupdate",
            |p| p.auto_update,
            |p| p.auto_update = !p.auto_update,
        ),
        update_toggle(
            ctx,
            "settings.general.prerelease",
            |p| p.prereleases,
            |p| p.prereleases = !p.prereleases,
        ),
        update_toggle(
            ctx,
            "settings.general.nightly",
            |p| p.nightly,
            |p| p.nightly = !p.nightly,
        ),
    ]
}

// ---------------------------------------------------------------- Aparência

const BUILTIN_THEME: &str = tako_themes::DEFAULT_THEME_ID;

fn theme_name(ctx: &Ctx) -> String {
    let root = ctx.paths.themes();
    tako_themes::selected(&root)
        .and_then(|id| {
            let text = std::fs::read_to_string(root.join(&id).join(tako_themes::MANIFEST_FILE));
            text.ok()
                .and_then(|t| tako_themes::ThemeManifest::parse(&t).ok())
                .map(|m| m.name)
                .or(Some(id))
        })
        .unwrap_or_else(|| ctx.t("settings.appearance.builtin"))
}

fn themes_panel(ctx: &Ctx) -> MenuLayer {
    let installed = Rc::new(tako_themes::InstalledTheme::scan(&ctx.paths.themes()));
    MenuLayer::panel(ctx.t("settings.appearance.theme"), move |ctx| {
        let selected = tako_themes::selected(&ctx.paths.themes());
        let mut v = vec![
            MenuItem::new(
                PanelRow::radio(
                    BUILTIN_THEME,
                    ctx.t("settings.appearance.builtin"),
                    selected.is_none(),
                )
                .desc(ctx.t("settings.appearance.builtin_desc")),
            )
            .on(|ctx| pick_theme(ctx, None)),
        ];
        for theme in installed.iter() {
            let id = theme.id();
            let (title, desc, ok) = match &theme.manifest {
                Ok(m) => (
                    m.name.clone(),
                    ctx.tf(
                        "settings.appearance.theme_by",
                        &[("version", &m.version), ("author", &m.author)],
                    ),
                    true,
                ),
                Err(p) => (
                    id.clone(),
                    p.iter()
                        .map(|p| p.message.clone())
                        .collect::<Vec<_>>()
                        .join(" · "),
                    false,
                ),
            };
            let desc = if ctx.safe_mode && ok {
                ctx.t("settings.appearance.safe_mode")
            } else {
                desc
            };
            let on = selected.as_deref() == Some(id.as_str());
            v.push(
                MenuItem::new(
                    PanelRow::radio(id.clone(), title, on)
                        .desc(desc)
                        .enabled(ok && !ctx.safe_mode),
                )
                .on(move |ctx| pick_theme(ctx, Some(&id))),
            );
        }
        v.push(
            MenuItem::new(
                PanelRow::action("install", ctx.t("settings.appearance.install")).icon("folder"),
            )
            .on(effect(Effect::InstallTheme)),
        );
        v
    })
}

fn pick_theme(ctx: &mut Ctx, id: Option<&str>) -> Flow {
    match tako_themes::select(&ctx.paths.themes(), id) {
        Ok(()) => ctx.effect(Effect::ApplyTheme(id.unwrap_or(BUILTIN_THEME).to_owned())),
        Err(e) => ctx.toast(e.to_string()),
    }
    Flow::Stay
}

fn appearance(ctx: &Ctx) -> Vec<MenuItem> {
    vec![
        MenuItem::new(
            PanelRow::action("theme", ctx.t("settings.appearance.theme"))
                .desc(ctx.t("settings.appearance.theme_desc"))
                .value(theme_name(ctx)),
        )
        .on(|ctx| Flow::Push(Box::new(themes_panel(ctx)))),
        toggle(
            ctx,
            "settings.appearance.dynamic",
            keys::DYNAMIC_COLOR,
            true,
        ),
        toggle(
            ctx,
            "settings.appearance.motion",
            keys::REDUCED_MOTION,
            false,
        ),
    ]
}

// ---------------------------------------------------------------- Sistema

/// O que dá para saber daqui; o arquivo completo é gravado pelo app (`Effect::SaveDiagnostics`).
fn report_text(ctx: &Ctx) -> String {
    use tako_core::diagnostics::{LogRedactor, report};
    use tako_core::library::GameStore;
    let games = ctx.library.games();
    let displays = ctx
        .platform
        .displays
        .list()
        .unwrap_or_default()
        .into_iter()
        .filter(|d| d.is_active)
        .map(|d| report::DisplayInfo {
            device: d.name,
            adapter: None,
            width: d.current_mode.map(|m| m.width).unwrap_or_default(),
            height: d.current_mode.map(|m| m.height).unwrap_or_default(),
            scale_percent: None,
            refresh_hz: d.current_mode.map(|m| m.refresh_hz),
            is_primary: d.is_primary,
        })
        .collect();
    let snap = report::SystemSnapshot {
        generated_at: chrono::Local::now().fixed_offset(),
        tako_version: ctx.version.clone(),
        data_folder: Some(ctx.paths.root.display().to_string()),
        architecture: std::env::consts::ARCH.into(),
        logical_processors: std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1),
        displays,
        controllers: (!ctx.pad_name.is_empty())
            .then(|| ctx.pad_name.clone())
            .into_iter()
            .collect(),
        active_controller: (!ctx.pad_name.is_empty()).then(|| ctx.pad_name.clone()),
        steam_found: games.iter().any(|g| g.store() == GameStore::Steam),
        steam_games: games
            .iter()
            .filter(|g| g.store() == GameStore::Steam)
            .count(),
        manual_games: games
            .iter()
            .filter(|g| g.store() == GameStore::Manual)
            .count(),
        total_games: games.len(),
        safe_mode: ctx.safe_mode,
        theme: Some(theme_name(ctx)),
        ..Default::default()
    };
    let text = report::format(
        &snap,
        &LogRedactor::for_current_user(),
        None,
        None,
        ctx.lang(),
    );
    let saved = ctx.tf(
        "settings.system.report_saved",
        &[("path", &ctx.paths.logs().display().to_string())],
    );
    format!("{saved}\n\n{text}")
}

fn system(ctx: &Ctx) -> Vec<MenuItem> {
    let mut v = vec![
        MenuItem::new(PanelRow::header(
            ctx.t("settings.system.diagnostics_header"),
        )),
        MenuItem::new(action(ctx, "settings.system.report")).on(|ctx| {
            ctx.effect(Effect::SaveDiagnostics);
            Flow::Push(Box::new(text_dialog(
                ctx.t("settings.system.report"),
                report_text(ctx),
            )))
        }),
        MenuItem::new(action(ctx, "settings.system.logs")).on(|ctx| {
            ctx.effect(Effect::OpenFolder(ctx.paths.logs().display().to_string()));
            Flow::Stay
        }),
        MenuItem::new(PanelRow::header(ctx.t("settings.system.recovery_header"))),
    ];
    v.push(if ctx.safe_mode {
        MenuItem::new(
            PanelRow::action("safemode", ctx.t("settings.system.safemode"))
                .desc(ctx.t("settings.system.safemode_on_desc"))
                .value(ctx.t("settings.system.safemode_exit")),
        )
        .on(effect(Effect::ExitSafeMode))
    } else {
        MenuItem::new(
            PanelRow::info(
                "safemode",
                ctx.t("settings.system.safemode"),
                ctx.t("settings.system.safemode_off"),
            )
            .desc(ctx.t("settings.system.safemode_off_desc")),
        )
    });
    if crate::discord::available() {
        v.push(MenuItem::new(PanelRow::header(
            ctx.t("settings.system.integrations_header"),
        )));
        v.push(toggle(
            ctx,
            "settings.system.discord",
            keys::DISCORD_PRESENCE,
            false,
        ));
    }
    v.push(MenuItem::new(PanelRow::header(
        ctx.t("settings.system.advanced_header"),
    )));
    v.push(toggle(
        ctx,
        "settings.system.developer",
        keys::DEVELOPER,
        false,
    ));
    if ctx.settings.bool(keys::DEVELOPER, false) {
        v.push(
            MenuItem::new(action(ctx, "settings.system.reload_theme")).on(|ctx| {
                let id = tako_themes::selected(&ctx.paths.themes())
                    .unwrap_or_else(|| BUILTIN_THEME.into());
                ctx.effect(Effect::ApplyTheme(id));
                Flow::Stay
            }),
        );
    }
    v
}

// ---------------------------------------------------------------- Sobre

const SOURCE_URL: &str = "https://github.com/lippdev/tako-launcher";

fn about(ctx: &Ctx) -> Vec<MenuItem> {
    vec![
        MenuItem::new(PanelRow::info(
            "version",
            ctx.t("settings.about.version"),
            format!("Tako {}", ctx.version),
        )),
        MenuItem::new(action(ctx, "settings.about.license")).on(|ctx| {
            Flow::Push(Box::new(text_dialog(
                ctx.t("settings.about.license_title"),
                ctx.t("settings.about.license_text"),
            )))
        }),
        MenuItem::new(action(ctx, "settings.about.notices")).on(|ctx| {
            Flow::Push(Box::new(text_dialog(
                ctx.t("settings.about.notices"),
                ctx.t("settings.about.notices_text"),
            )))
        }),
        MenuItem::new(
            PanelRow::action("source", ctx.t("settings.about.source"))
                .desc(ctx.t("settings.about.source_desc"))
                .value("github.com/lippdev/tako-launcher"),
        )
        .on(effect(Effect::OpenUrl(SOURCE_URL.into()))),
    ]
}

pub struct SettingsPage {
    cats: Vec<Category>,
    cat: usize,
    /// `None` = coluna de categorias.
    row: Option<usize>,
}

impl Default for SettingsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsPage {
    pub fn new() -> SettingsPage {
        SettingsPage {
            cats: categories(),
            cat: 0,
            row: None,
        }
    }

    pub fn at(category: &str) -> SettingsPage {
        let mut p = Self::new();
        p.cat = p.cats.iter().position(|c| c.id == category).unwrap_or(0);
        p
    }

    fn items(&self, ctx: &Ctx) -> Vec<MenuItem> {
        (self.cats[self.cat].rows)(ctx)
    }

    fn step(items: &[MenuItem], from: usize, d: i32) -> usize {
        let mut i = from as i32;
        loop {
            i += d;
            if i < 0 || i >= items.len() as i32 {
                return from;
            }
            if items[i as usize].row.is_focusable() {
                return i as usize;
            }
        }
    }

    fn first(items: &[MenuItem]) -> Option<usize> {
        items.iter().position(|i| i.row.is_focusable())
    }

    fn run(flow: Flow, ctx: &mut Ctx) -> Nav {
        match flow {
            Flow::Push(l) | Flow::Replace(l) => Nav::Layer(l),
            _ => {
                ctx.dirty = true;
                Nav::Stay
            }
        }
    }
}

impl Page for SettingsPage {
    fn route(&self) -> String {
        "settings".into()
    }

    fn kind(&self) -> &'static str {
        "settings"
    }

    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Nav {
        let items = self.items(ctx);
        let n = self.cats.len();
        match (self.row, cmd) {
            (_, Cmd::PrevSection) => {
                self.cat = (self.cat + n - 1) % n;
                self.row = None;
            }
            (_, Cmd::NextSection) => {
                self.cat = (self.cat + 1) % n;
                self.row = None;
            }
            (None, Cmd::Up) => self.cat = self.cat.saturating_sub(1),
            (None, Cmd::Down) => self.cat = (self.cat + 1).min(n - 1),
            (None, Cmd::Right) | (None, Cmd::Accept) => self.row = Self::first(&items),
            (None, Cmd::Left) | (None, Cmd::Back) => return Nav::Sidebar,
            (Some(r), Cmd::Up) => self.row = Some(Self::step(&items, r, -1)),
            (Some(r), Cmd::Down) => self.row = Some(Self::step(&items, r, 1)),
            (Some(_), Cmd::Back) => self.row = None,
            (Some(r), Cmd::Left) | (Some(r), Cmd::Right) => {
                if let Some(adj) = items
                    .get(r)
                    .filter(|i| i.row.enabled)
                    .and_then(|i| i.adjust.clone())
                {
                    return Self::run(adj(ctx, if cmd == Cmd::Left { -1 } else { 1 }), ctx);
                }
                if cmd == Cmd::Left {
                    self.row = None;
                }
            }
            (Some(r), Cmd::Accept) => {
                let Some(item) = items.get(r) else {
                    return Nav::Stay;
                };
                if !item.row.enabled {
                    if !item.row.description.is_empty() {
                        ctx.toast(item.row.description.clone());
                    }
                    return Nav::Stay;
                }
                if let Some(f) = item.accept.clone() {
                    return Self::run(f(ctx), ctx);
                }
                if let Some(adj) = item.adjust.clone() {
                    return Self::run(adj(ctx, 1), ctx);
                }
            }
            _ => {}
        }
        Nav::Stay
    }

    fn render(&mut self, ctx: &Ctx, out: &mut Out) {
        let items = self.items(ctx);
        if let Some(r) = self.row
            && r >= items.len()
        {
            self.row = Self::first(&items);
        }
        out.set("settings-title", ctx.t("nav.settings"));
        let cats: Vec<NavItem> = self
            .cats
            .iter()
            .enumerate()
            .map(|(i, c)| NavItem {
                id: c.id.into(),
                label: ctx.t(&format!("settings.{}.title", c.id)),
                icon: c.icon.into(),
                selected: i == self.cat,
                ..Default::default()
            })
            .collect();
        out.set("settings-categories", cats);
        out.set("settings-category-focus", self.cat as i32);
        let id = self.cats[self.cat].id;
        out.set("settings-heading", ctx.t(&format!("settings.{id}.title")));
        out.set(
            "settings-subtitle",
            ctx.t(&format!("settings.{id}.subtitle")),
        );
        let rows: Vec<PanelRow> = items.into_iter().map(|i| i.row).collect();
        out.set("settings-rows", rows);
        out.set(
            "settings-row-focus",
            self.row.map(|r| r as i32).unwrap_or(-1),
        );
    }

    fn hints(&self, ctx: &Ctx) -> Vec<Hint> {
        vec![
            ctx.hint(Cmd::Accept, &ctx.t("hint.select")),
            ctx.hint(
                Cmd::Back,
                &ctx.t(if self.row.is_some() {
                    "hint.categories"
                } else {
                    "hint.back"
                }),
            ),
            ctx.hint(Cmd::PrevSection, &ctx.t("hint.category")),
        ]
    }

    fn hover(&mut self, zone: &str, index: usize, _ctx: &Ctx) {
        match zone {
            "category" => {
                self.cat = index.min(self.cats.len() - 1);
                self.row = None;
            }
            "row" => self.row = Some(index),
            _ => {}
        }
    }
}
