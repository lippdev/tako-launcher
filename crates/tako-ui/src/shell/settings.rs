//! Configurações (ADR-0020): coluna de categorias à esquerda e as linhas da categoria à direita. LB/RB trocam
//! de categoria; → ou A entram nas linhas; B nas linhas volta à categoria e B na coluna sai.

#[path = "settings_controls.rs"]
pub(crate) mod controls;
#[path = "settings_desk_theme.rs"]
mod desk_theme;
#[path = "settings_emulators.rs"]
pub(crate) mod emulators;
#[path = "settings_integrations.rs"]
pub(crate) mod integrations;
#[path = "settings_library.rs"]
pub(crate) mod library;
#[path = "settings_metadata.rs"]
pub(crate) mod metadata;
#[path = "settings_screen.rs"]
pub(crate) mod screen;
#[path = "settings_sunshine.rs"]
pub(crate) mod sunshine;
#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;

use super::ctx::keys;
use super::layer::{Flow, MenuItem, MenuLayer, Pick};
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
            id: "appearance",
            icon: "palette",
            rows: appearance,
        },
        Category {
            id: "sound",
            icon: "volume",
            rows: sound,
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
            id: "downloads",
            icon: "download",
            rows: super::downloads::settings_rows,
        },
        Category {
            id: "integrations",
            icon: "tv",
            rows: integrations::rows,
        },
        Category {
            id: "system",
            icon: "gear",
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
        if ctx.restoring {
            ctx.toast(ctx.t("settings.system.restore_busy"));
            return Flow::Stay;
        }
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
    described(ctx, text).action_btn(ctx.t(&format!("{text}_value")))
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

#[cfg(any(windows, test))]
#[derive(Default)]
struct FseState {
    status: Option<tako_platform::fse::Status>,
    error: Option<String>,
    checked: Option<Instant>,
    pending: bool,
}

#[cfg(any(windows, test))]
impl FseState {
    fn begin(&mut self) -> bool {
        if self.pending
            || self
                .checked
                .is_some_and(|t| t.elapsed() < Duration::from_secs(30))
        {
            return false;
        }
        self.pending = true;
        true
    }

    fn finish(&mut self, result: Result<tako_platform::fse::Status, String>) {
        self.pending = false;
        let retry = match result {
            Ok(status) => {
                self.status = Some(status);
                status == tako_platform::fse::Status::Applying
            }
            Err(error) => {
                tracing::warn!(%error, "Falha ao consultar modo tela cheia do Windows");
                true
            }
        };
        // Retry transient failures and another launcher's operation after one second.
        self.checked = Some(Instant::now() - Duration::from_secs(if retry { 29 } else { 0 }));
    }
}

#[cfg(any(windows, test))]
thread_local! {
    static FSE: RefCell<FseState> = RefCell::new(FseState::default());
}

#[cfg(any(windows, test))]
fn fse_status() -> (Option<tako_platform::fse::Status>, bool) {
    FSE.with(|state| {
        let mut state = state.borrow_mut();
        if state.begin() {
            crate::tasks::run_bg(tako_platform::fse::status, |_, result| {
                FSE.with(|state| state.borrow_mut().finish(result));
            });
        }
        (state.status, state.pending)
    })
}

#[cfg(windows)]
fn xbox_mode_row(ctx: &Ctx) -> MenuItem {
    MenuItem::new(
        PanelRow::action("xbox-mode", ctx.t("settings.general.xbox_mode"))
            .desc(ctx.t("settings.general.xbox_mode_desc")),
    )
    .on(|ctx| Flow::Push(Box::new(xbox_mode_panel(ctx))))
}

#[cfg(any(windows, test))]
fn fse_error_code(error: &str) -> Option<String> {
    error.as_bytes().windows(10).find_map(|code| {
        (code.starts_with(b"0x") && code[2..].iter().all(u8::is_ascii_hexdigit))
            .then(|| String::from_utf8_lossy(code).into_owned())
    })
}

#[cfg(test)]
#[test]
fn fse_error_only_extracts_hresult() {
    assert_eq!(
        fse_error_code("HRESULT: 0x80073CF6, detalhes"),
        Some("0x80073CF6".into())
    );
    assert_eq!(fse_error_code("0xbroken!!"), None);
    assert_eq!(fse_error_code("UAC cancelado"), None);
}

#[cfg(any(windows, test))]
fn xbox_mode_panel(ctx: &Ctx) -> MenuLayer {
    MenuLayer::panel(ctx.t("settings.general.xbox_mode"), |ctx| {
        let (status, pending) = fse_status();
        let busy = pending || status == Some(tako_platform::fse::Status::Applying);
        let recovering = status == Some(tako_platform::fse::Status::RecoveryRequired);
        let enabled = status == Some(tako_platform::fse::Status::Enabled);
        let available = matches!(
            status,
            Some(
                tako_platform::fse::Status::Enabled
                    | tako_platform::fse::Status::Disabled
                    | tako_platform::fse::Status::RecoveryRequired
            )
        );
        vec![
            MenuItem::new(
                (if recovering {
                    PanelRow::button("fse", ctx.t("settings.general.fse_recover"))
                } else {
                    PanelRow::toggle("fse", ctx.t("settings.general.fse_toggle"), enabled)
                })
                .desc(if busy {
                    ctx.t("settings.general.fse_applying")
                } else if !available {
                    ctx.t("settings.general.fse_unavailable")
                } else {
                    FSE.with(|state| state.borrow().error.clone())
                        .unwrap_or_default()
                }),
            )
            .on(move |ctx| {
                if FSE.with(|state| state.borrow().pending) || busy {
                    return Flow::Stay;
                }
                if !available {
                    ctx.toast(ctx.t("settings.general.fse_unavailable"));
                    return Flow::Stay;
                }
                FSE.with(|state| state.borrow_mut().pending = true);
                crate::tasks::run(
                    ctx,
                    move || {
                        if enabled || recovering {
                            tako_platform::fse::disable()
                        } else {
                            tako_platform::fse::enable()
                        }
                    },
                    |ctx, result| {
                        FSE.with(|state| {
                            let mut state = state.borrow_mut();
                            state.pending = false;
                            state.checked = None;
                            state.error = result.as_ref().err().map(|error| {
                                if let Some(code) = fse_error_code(error) {
                                    ctx.tf("settings.general.fse_package_error", &[("code", &code)])
                                } else {
                                    ctx.t("settings.general.fse_error_detail")
                                }
                            });
                        });
                        if let Err(error) = result {
                            tracing::error!(%error, "Falha ao alterar modo tela cheia do Windows");
                            ctx.toast(ctx.t("settings.general.fse_error"));
                        }
                    },
                );
                Flow::Stay
            }),
        ]
    })
    .text(ctx.t("settings.general.fse_notice"))
}

fn general(ctx: &Ctx) -> Vec<MenuItem> {
    let status = if ctx.update_status.is_empty() {
        ctx.t("settings.general.updates_desc")
    } else {
        ctx.update_status.clone()
    };
    let mut v = mode_rows(ctx);
    v.extend([
        MenuItem::new(
            PanelRow::action("profiles", ctx.t("profiles.title"))
                .desc(ctx.t("profiles.desc"))
                .value(super::profiles::active_name(ctx)),
        )
        .on(|ctx| Flow::Push(Box::new(super::profiles::panel(ctx)))),
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
        })
        .options(|ctx| {
            let langs = [Lang::PtBr, Lang::EnUs];
            let cur = langs.iter().position(|l| *l == ctx.lang()).unwrap_or(0);
            let picks = langs
                .into_iter()
                .map(|lang| {
                    Pick::new(lang_name(lang)).set(move |ctx| {
                        ctx.settings.set(keys::LANGUAGE, lang.code());
                        i18n::set_lang(lang);
                        ctx.dirty = true;
                        Flow::Stay
                    })
                })
                .collect();
            (picks, cur)
        }),
        MenuItem::new(PanelRow::header(ctx.t("settings.general.updates_header"))),
        MenuItem::new(
            PanelRow::action("updates", ctx.t("settings.general.updates"))
                .desc(status)
                .action_btn(ctx.t("settings.general.updates_value")),
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
    ]);
    // Antes de "Abrir o Tako ao entrar no sistema", quando presente.
    #[cfg(windows)]
    {
        let at = v
            .iter()
            .position(|i| i.row.key == "startup")
            .unwrap_or(v.len());
        v.insert(at, xbox_mode_row(ctx));
    }
    v
}

/// Modo (ADR-0027): trocar entre o desktop e a tela cheia e a pergunta do controle ao abrir.
fn mode_rows(ctx: &Ctx) -> Vec<MenuItem> {
    use crate::mode::{ASK_SETTING, UiMode};
    let (text, to) = if ctx.desktop {
        ("desktop.mode.go_big_picture", UiMode::BigPicture)
    } else {
        ("desktop.mode.go_desktop", UiMode::Desktop)
    };
    let mut v = vec![
        MenuItem::new(PanelRow::header(ctx.t("desktop.mode.header"))),
        MenuItem::new(
            PanelRow::action("mode", ctx.t(text))
                .desc(ctx.t(&format!("{text}_desc")))
                .action_btn(ctx.t("desktop.mode.go_value")),
        )
        .on(effect(Effect::SwitchMode(to))),
        MenuItem::new(
            PanelRow::toggle(
                "ask-big-picture",
                ctx.t("desktop.mode.ask"),
                ctx.settings.bool(ASK_SETTING, true),
            )
            .desc(ctx.t("desktop.mode.ask_desc")),
        )
        .on(|ctx| {
            ctx.settings.toggle(ASK_SETTING, true);
            Flow::Stay
        }),
    ];
    // "Abrir em" (OnboardingModo): o modo da abertura sem argumentos. Só no desktop; a tela cheia não muda.
    if ctx.desktop {
        use crate::mode::{OPEN_IN, OPEN_IN_BIG_PICTURE, OPEN_IN_DESKTOP};
        let big = ctx.settings.get(OPEN_IN) == Some(OPEN_IN_BIG_PICTURE);
        v.push(
            MenuItem::new(PanelRow::choice(
                "open-in",
                ctx.t("desktop.mode.open_in"),
                ctx.t(if big {
                    "first_run.full.mode_full"
                } else {
                    "first_run.full.mode_window"
                }),
            ))
            .on(move |ctx| {
                ctx.settings.set(
                    OPEN_IN,
                    if big {
                        OPEN_IN_DESKTOP
                    } else {
                        OPEN_IN_BIG_PICTURE
                    },
                );
                Flow::Stay
            })
            .options(move |ctx| {
                let picks = [
                    (OPEN_IN_DESKTOP, "first_run.full.mode_window"),
                    (OPEN_IN_BIG_PICTURE, "first_run.full.mode_full"),
                ]
                .into_iter()
                .map(|(value, label)| {
                    Pick::new(ctx.t(label)).set(move |ctx| {
                        ctx.settings.set(OPEN_IN, value);
                        Flow::Stay
                    })
                })
                .collect();
                (picks, usize::from(big))
            }),
        );
    }
    v.push(MenuItem::new(PanelRow::header(
        ctx.t("desktop.mode.general_header"),
    )));
    v
}

// ---------------------------------------------------------------- Aparência

pub(crate) const BUILTIN_THEME: &str = tako_themes::DEFAULT_THEME_ID;

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

#[derive(Clone)]
pub(crate) struct BaseThemeCandidate {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
}

#[derive(Clone)]
pub(crate) struct CachedThemesData {
    pub installed: Vec<tako_themes::InstalledTheme>,
    pub exportable: Vec<tako_themes::InstalledTheme>,
    pub base_candidates: Vec<BaseThemeCandidate>,
}

/// Temas instalados (lidos do disco no máximo a cada 30 s; instalar ou atualizar esquece a leitura).
pub(crate) const THEMES_CACHE: &str = "themes";

pub(crate) fn cached_themes(ctx: &Ctx) -> CachedThemesData {
    let root = ctx.paths.themes();
    cached(THEMES_CACHE, Duration::from_secs(30), || {
        let installed = tako_themes::InstalledTheme::scan(&root);
        let exportable: Vec<_> = installed
            .iter()
            .filter(|t| tako_themes::create::is_tokens_only_theme(&t.dir))
            .cloned()
            .collect();
        let mut base_candidates = Vec::new();
        for theme in &installed {
            if tako_themes::create::uses_default_ui(&theme.dir) {
                let id = theme.id();
                let (name, version, author) = match &theme.manifest {
                    Ok(m) => (m.name.clone(), m.version.clone(), m.author.clone()),
                    Err(_) => (id.clone(), String::new(), String::new()),
                };
                base_candidates.push(BaseThemeCandidate {
                    id,
                    name,
                    version,
                    author,
                });
            }
        }
        CachedThemesData {
            installed,
            exportable,
            base_candidates,
        }
    })
}

/// "Instalar por endereço" (THEME-004): o endereço do repositório no GitHub; baixa fora da interface.
fn install_from_url(ctx: &Ctx) -> Flow {
    Flow::Push(Box::new(
        super::keyboard::KeyboardLayer::new(
            ctx,
            ctx.t("settings.appearance.install_url"),
            "",
            |ctx, text| {
                let text = text.to_owned();
                let root = ctx.paths.themes();
                ctx.toast(ctx.t("settings.appearance.downloading"));
                crate::tasks::run(
                    ctx,
                    move || tako_themes::remote::install_from(&text, &root),
                    |ctx, result| {
                        forget(THEMES_CACHE);
                        match result {
                            Ok(t) => {
                                let name = t
                                    .manifest
                                    .as_ref()
                                    .map(|m| m.name.clone())
                                    .unwrap_or_else(|_| t.id());
                                ctx.toast(ctx.tf("effects.theme_installed", &[("name", &name)]));
                            }
                            Err(e) => ctx.toast(e.to_string()),
                        }
                    },
                );
                Flow::Close
            },
        )
        .hint(ctx.t("settings.appearance.install_url_hint")),
    ))
}

/// "Atualizar temas": baixa a release nova de cada tema instalado por endereço.
fn update_themes(ctx: &mut Ctx) -> Flow {
    let root = ctx.paths.themes();
    ctx.toast(ctx.t("settings.appearance.checking"));
    crate::tasks::run(
        ctx,
        move || tako_themes::remote::update_all(&root),
        |ctx, results| {
            use tako_themes::remote::UpdateOutcome;
            forget(THEMES_CACHE);
            let updated = results
                .iter()
                .filter(|(_, o)| matches!(o, UpdateOutcome::Updated(_)))
                .count();
            let failed: Vec<String> = results
                .iter()
                .filter_map(|(id, o)| match o {
                    UpdateOutcome::Failed(e) => Some(format!("{id}: {e}")),
                    _ => None,
                })
                .collect();
            let mut text = if updated == 0 {
                ctx.t("settings.appearance.themes_up_to_date")
            } else {
                i18n::tn("settings.appearance.themes_updated", updated as i64, &[])
            };
            if !failed.is_empty() {
                text = format!("{text} · {}", failed.join(" · "));
            }
            ctx.toast(text);
            // O tema em uso mudou: aplica a versão nova.
            if updated > 0
                && let Some(id) = tako_themes::selected(&ctx.paths.themes())
            {
                ctx.effect(Effect::ApplyTheme(id));
            }
        },
    );
    Flow::Stay
}

pub(crate) fn themes_panel(ctx: &Ctx) -> MenuLayer {
    MenuLayer::panel(ctx.t("settings.appearance.theme"), |ctx| {
        let themes_data = cached_themes(ctx);
        let installed = &themes_data.installed;
        let sources = tako_themes::remote::sources(&ctx.paths.themes());
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
            .on(|ctx| {
                let name = ctx.t("settings.appearance.builtin");
                pick_theme(ctx, None, &name)
            }),
        ];
        for theme in installed.iter() {
            let id = theme.id();
            let (title, desc, ok) = match &theme.manifest {
                Ok(m) => (
                    m.name.clone(),
                    if m.author.trim().is_empty() {
                        ctx.tf(
                            "settings.appearance.theme_version",
                            &[("version", &m.version)],
                        )
                    } else {
                        ctx.tf(
                            "settings.appearance.theme_by",
                            &[("version", &m.version), ("author", &m.author)],
                        )
                    },
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
            // Instalado por endereço: mostra de onde veio.
            let desc = match sources.get(&id) {
                Some(s) if ok => format!("{desc} · {}", s.repo.trim_start_matches("https://")),
                _ => desc,
            };
            let on = selected.as_deref() == Some(id.as_str());
            let name = title.clone();
            v.push(
                MenuItem::new(
                    PanelRow::radio(id.clone(), title, on)
                        .desc(desc)
                        .enabled(ok && !ctx.safe_mode),
                )
                .on(move |ctx| pick_theme(ctx, Some(&id), &name)),
            );
        }
        v.push(
            MenuItem::new(
                PanelRow::action("install", ctx.t("settings.appearance.install")).icon("folder"),
            )
            .on(effect(Effect::InstallTheme)),
        );
        v.push(
            MenuItem::new(
                PanelRow::action("install-url", ctx.t("settings.appearance.install_url"))
                    .desc(ctx.t("settings.appearance.install_url_desc"))
                    .icon("link"),
            )
            .on(|ctx| install_from_url(ctx)),
        );
        let has_exportable = !themes_data.exportable.is_empty();
        if has_exportable {
            v.push(
                MenuItem::new(
                    PanelRow::action("export-theme", ctx.t("settings.appearance.export_theme"))
                        .desc(ctx.t("settings.appearance.export_theme_desc"))
                        .icon("folder"),
                )
                .on(export_theme_panel),
            );
        }
        if !sources.is_empty() {
            v.push(
                MenuItem::new(
                    PanelRow::action("update-themes", ctx.t("settings.appearance.update_themes"))
                        .desc(ctx.t("settings.appearance.update_themes_desc"))
                        .action_btn(ctx.t("settings.appearance.update_themes_value")),
                )
                .on(update_themes),
            );
        }
        v
    })
}

/// Escolher um tema. Na tela cheia ele entra em prévia (THEME-002) e só vira o escolhido ao confirmar; o desktop não
/// tem tema (ADR-0027), então ali a escolha vale direto, para a próxima tela cheia.
fn pick_theme(ctx: &mut Ctx, id: Option<&str>, name: &str) -> Flow {
    ctx.discard_theme_draft();
    if !ctx.desktop {
        if tako_themes::selected(&ctx.paths.themes()).as_deref() == id {
            return Flow::Stay;
        }
        return super::theme_preview::start(ctx, id, name);
    }
    match tako_themes::select(&ctx.paths.themes(), id) {
        Ok(()) => {
            ctx.reload_theme_cache();
            ctx.effect(Effect::ApplyTheme(id.unwrap_or(BUILTIN_THEME).to_owned()))
        }
        Err(e) => ctx.toast(e.to_string()),
    }
    Flow::Stay
}

const ACCENT_PRESETS: [(&str, &str); 8] = [
    ("#2B3A55", "accent_blue"),
    ("#6C5CE7", "accent_purple"),
    ("#E3161E", "accent_red"),
    ("#0B7A3B", "accent_green"),
    ("#E67E22", "accent_orange"),
    ("#E84393", "accent_pink"),
    ("#00CEC9", "accent_cyan"),
    ("#F1C40F", "accent_yellow"),
];

const FONTS: [&str; 4] = ["Inter", "Source Sans 3", "Poppins", "Plus Jakarta Sans"];

const BG_TYPES: [(&str, &str); 3] = [
    ("default", "bg_default"),
    ("solid", "bg_solid"),
    ("image", "bg_image"),
];

const BG_COLORS: [(&str, &str); 5] = [
    ("#0E1116", "bg_black"),
    ("#1C1C1E", "bg_dark_gray"),
    ("#0B1A33", "bg_navy"),
    ("#1B1429", "bg_dark_purple"),
    ("#0D201A", "bg_dark_green"),
];

fn current_draft(ctx: &Ctx) -> tako_themes::overrides::ThemeTokensOverride {
    if let Some(ref d) = ctx.theme_draft {
        return d.clone();
    }
    let theme_id = tako_themes::selected(&ctx.paths.themes());
    tako_themes::overrides::load_override(&ctx.paths.themes(), theme_id.as_deref())
        .map(|s| tako_themes::overrides::ThemeTokensOverride::from_toml_str(&s))
        .unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ThemeEditorMode {
    OverrideCurrent,
    CreateNew { base_id: Option<String> },
}

pub(crate) fn edit_theme_panel(ctx: &mut Ctx) -> MenuLayer {
    let theme_id = tako_themes::selected(&ctx.paths.themes());
    let expected_dir =
        tako_themes::overrides::override_dir(&ctx.paths.themes(), theme_id.as_deref());
    let foreign = ctx
        .theme_draft
        .as_ref()
        .and_then(|d| d.draft_dir.as_ref())
        .is_some_and(|dir| dir != &expected_dir);
    if foreign {
        ctx.discard_theme_draft();
    }
    build_theme_editor_panel(ctx, ThemeEditorMode::OverrideCurrent)
}

pub(crate) fn create_theme_editor_panel(ctx: &Ctx, base_id: Option<String>) -> MenuLayer {
    build_theme_editor_panel(ctx, ThemeEditorMode::CreateNew { base_id })
}

pub(crate) fn build_theme_editor_panel(ctx: &Ctx, mode: ThemeEditorMode) -> MenuLayer {
    let panel_title = match &mode {
        ThemeEditorMode::OverrideCurrent => ctx.t("settings.appearance.edit_theme"),
        ThemeEditorMode::CreateNew { .. } => ctx.t("settings.appearance.create_theme"),
    };
    let mode_clone = mode.clone();
    MenuLayer::panel(panel_title, move |ctx| {
        let draft = current_draft(ctx);
        let theme_id_opt = match &mode_clone {
            ThemeEditorMode::OverrideCurrent => tako_themes::selected(&ctx.paths.themes()),
            ThemeEditorMode::CreateNew { base_id } => base_id.clone(),
        };

        let cur_accent_hex = draft.accent.as_deref().unwrap_or("#2B3A55");
        let accent_label = ACCENT_PRESETS
            .iter()
            .find(|(hex, _)| hex.eq_ignore_ascii_case(cur_accent_hex))
            .map(|(_, l)| ctx.t(&format!("settings.appearance.{l}")))
            .unwrap_or_else(|| cur_accent_hex.to_owned());

        let accent_row = MenuItem::new(
            PanelRow::choice(
                "accent",
                ctx.t("settings.appearance.accent_color"),
                accent_label,
            )
            .desc(ctx.t("settings.appearance.accent_color_desc")),
        )
        .on_adjust(|ctx, dir| {
            let mut d = current_draft(ctx);
            let cur_hex = d.accent.as_deref().unwrap_or("#2B3A55");
            let cur_idx = ACCENT_PRESETS
                .iter()
                .position(|(hex, _)| hex.eq_ignore_ascii_case(cur_hex))
                .unwrap_or(0);
            let next_idx = if dir >= 0 {
                (cur_idx + 1) % ACCENT_PRESETS.len()
            } else {
                (cur_idx + ACCENT_PRESETS.len() - 1) % ACCENT_PRESETS.len()
            };
            d.accent = Some(ACCENT_PRESETS[next_idx].0.to_owned());
            ctx.theme_draft = Some(d);
            ctx.dirty = true;
            Flow::Stay
        })
        .options(|ctx| {
            let draft = current_draft(ctx);
            let cur_hex = draft.accent.as_deref().unwrap_or("#2B3A55");
            let cur_idx = ACCENT_PRESETS
                .iter()
                .position(|(hex, _)| hex.eq_ignore_ascii_case(cur_hex))
                .unwrap_or(0);
            let picks = ACCENT_PRESETS
                .iter()
                .map(|(hex, label)| {
                    let hex = (*hex).to_owned();
                    Pick::new(ctx.t(&format!("settings.appearance.{label}"))).set(move |ctx| {
                        let mut d = current_draft(ctx);
                        d.accent = Some(hex.clone());
                        ctx.theme_draft = Some(d);
                        ctx.dirty = true;
                        Flow::Stay
                    })
                })
                .collect();
            (picks, cur_idx)
        });

        let cur_font = draft.font.as_deref().unwrap_or("Inter");
        let font_label = cur_font.to_owned();
        let font_row = MenuItem::new(
            PanelRow::choice("font", ctx.t("settings.appearance.font"), font_label)
                .desc(ctx.t("settings.appearance.font_desc")),
        )
        .on_adjust(|ctx, dir| {
            let mut d = current_draft(ctx);
            let cur_f = d.font.as_deref().unwrap_or("Inter");
            let cur_idx = FONTS.iter().position(|f| *f == cur_f).unwrap_or(0);
            let next_idx = if dir >= 0 {
                (cur_idx + 1) % FONTS.len()
            } else {
                (cur_idx + FONTS.len() - 1) % FONTS.len()
            };
            d.font = Some(FONTS[next_idx].to_owned());
            ctx.theme_draft = Some(d);
            ctx.dirty = true;
            Flow::Stay
        })
        .options(|ctx| {
            let draft = current_draft(ctx);
            let cur_f = draft.font.as_deref().unwrap_or("Inter");
            let cur_idx = FONTS.iter().position(|f| *f == cur_f).unwrap_or(0);
            let picks = FONTS
                .iter()
                .map(|font_name| {
                    let f_name = (*font_name).to_owned();
                    Pick::new(*font_name).set(move |ctx| {
                        let mut d = current_draft(ctx);
                        d.font = Some(f_name.clone());
                        ctx.theme_draft = Some(d);
                        ctx.dirty = true;
                        Flow::Stay
                    })
                })
                .collect();
            (picks, cur_idx)
        });

        let bg_type_idx = match draft.background_mode {
            Some(tako_themes::overrides::BackgroundMode::Image) => 2,
            Some(tako_themes::overrides::BackgroundMode::Solid) => 1,
            Some(tako_themes::overrides::BackgroundMode::Default) => 0,
            None => {
                if draft.background_image.is_some() {
                    2
                } else if draft.canvas.is_some() {
                    1
                } else {
                    0
                }
            }
        };
        let bg_type_label = ctx.t(&format!("settings.appearance.{}", BG_TYPES[bg_type_idx].1));

        let bg_type_row = MenuItem::new(
            PanelRow::choice(
                "bg_type",
                ctx.t("settings.appearance.bg_type"),
                bg_type_label,
            )
            .desc(ctx.t("settings.appearance.bg_type_desc")),
        )
        .on_adjust(|ctx, dir| {
            let mut d = current_draft(ctx);
            let cur_idx = match d.background_mode {
                Some(tako_themes::overrides::BackgroundMode::Image) => 2,
                Some(tako_themes::overrides::BackgroundMode::Solid) => 1,
                Some(tako_themes::overrides::BackgroundMode::Default) => 0,
                None => {
                    if d.background_image.is_some() {
                        2
                    } else if d.canvas.is_some() {
                        1
                    } else {
                        0
                    }
                }
            };
            let next_idx = if dir >= 0 {
                (cur_idx + 1) % BG_TYPES.len()
            } else {
                (cur_idx + BG_TYPES.len() - 1) % BG_TYPES.len()
            };
            match next_idx {
                0 => {
                    d.background_mode = Some(tako_themes::overrides::BackgroundMode::Default);
                    d.canvas = None;
                    d.background_image = None;
                }
                1 => {
                    d.background_mode = Some(tako_themes::overrides::BackgroundMode::Solid);
                    d.background_image = None;
                    if d.canvas.is_none() {
                        d.canvas = Some("#0E1116".into());
                    }
                }
                _ => {
                    d.background_mode = Some(tako_themes::overrides::BackgroundMode::Image);
                    d.canvas = None;
                }
            }
            ctx.theme_draft = Some(d);
            ctx.dirty = true;
            Flow::Stay
        })
        .options(|ctx| {
            let draft = current_draft(ctx);
            let cur_idx = match draft.background_mode {
                Some(tako_themes::overrides::BackgroundMode::Image) => 2,
                Some(tako_themes::overrides::BackgroundMode::Solid) => 1,
                Some(tako_themes::overrides::BackgroundMode::Default) => 0,
                None => {
                    if draft.background_image.is_some() {
                        2
                    } else if draft.canvas.is_some() {
                        1
                    } else {
                        0
                    }
                }
            };
            let picks = BG_TYPES
                .iter()
                .map(|(mode, label)| {
                    let mode = (*mode).to_owned();
                    Pick::new(ctx.t(&format!("settings.appearance.{label}"))).set(move |ctx| {
                        let mut d = current_draft(ctx);
                        match mode.as_str() {
                            "default" => {
                                d.background_mode =
                                    Some(tako_themes::overrides::BackgroundMode::Default);
                                d.canvas = None;
                                d.background_image = None;
                            }
                            "solid" => {
                                d.background_mode =
                                    Some(tako_themes::overrides::BackgroundMode::Solid);
                                d.background_image = None;
                                if d.canvas.is_none() {
                                    d.canvas = Some("#0E1116".into());
                                }
                            }
                            "image" => {
                                d.background_mode =
                                    Some(tako_themes::overrides::BackgroundMode::Image);
                                d.canvas = None;
                            }
                            _ => {}
                        }
                        ctx.theme_draft = Some(d);
                        ctx.dirty = true;
                        Flow::Stay
                    })
                })
                .collect();
            (picks, cur_idx)
        });

        let mut rows = vec![accent_row];
        if ctx.theme_supports_font {
            rows.push(font_row);
        }
        if ctx.theme_supports_background {
            rows.push(bg_type_row);
        }

        if ctx.theme_supports_background && bg_type_idx == 1 {
            let cur_canvas = draft.canvas.as_deref().unwrap_or("#0E1116");
            let bg_color_label = BG_COLORS
                .iter()
                .find(|(hex, _)| hex.eq_ignore_ascii_case(cur_canvas))
                .map(|(_, l)| ctx.t(&format!("settings.appearance.{l}")))
                .unwrap_or_else(|| cur_canvas.to_owned());

            let bg_color_row = MenuItem::new(
                PanelRow::choice(
                    "bg_color",
                    ctx.t("settings.appearance.bg_color"),
                    bg_color_label,
                )
                .desc(ctx.t("settings.appearance.bg_color_desc")),
            )
            .on_adjust(|ctx, dir| {
                let mut d = current_draft(ctx);
                let cur_canvas = d.canvas.as_deref().unwrap_or("#0E1116");
                let cur_idx = BG_COLORS
                    .iter()
                    .position(|(hex, _)| hex.eq_ignore_ascii_case(cur_canvas))
                    .unwrap_or(0);
                let next_idx = if dir >= 0 {
                    (cur_idx + 1) % BG_COLORS.len()
                } else {
                    (cur_idx + BG_COLORS.len() - 1) % BG_COLORS.len()
                };
                d.canvas = Some(BG_COLORS[next_idx].0.to_owned());
                d.background_image = None;
                ctx.theme_draft = Some(d);
                ctx.dirty = true;
                Flow::Stay
            })
            .options(|ctx| {
                let draft = current_draft(ctx);
                let cur_canvas = draft.canvas.as_deref().unwrap_or("#0E1116");
                let cur_idx = BG_COLORS
                    .iter()
                    .position(|(hex, _)| hex.eq_ignore_ascii_case(cur_canvas))
                    .unwrap_or(0);
                let picks = BG_COLORS
                    .iter()
                    .map(|(hex, label)| {
                        let hex = (*hex).to_owned();
                        Pick::new(ctx.t(&format!("settings.appearance.{label}"))).set(move |ctx| {
                            let mut d = current_draft(ctx);
                            d.canvas = Some(hex.clone());
                            d.background_image = None;
                            ctx.theme_draft = Some(d);
                            ctx.dirty = true;
                            Flow::Stay
                        })
                    })
                    .collect();
                (picks, cur_idx)
            });
            rows.push(bg_color_row);
        } else if ctx.theme_supports_background && bg_type_idx == 2 {
            let img_label = if let Some(ref name) = draft.background_source_name {
                name.clone()
            } else if let Some(ref img_val) = draft.background_image {
                if img_val.is_empty() {
                    ctx.t("settings.appearance.bg_no_image")
                } else if let Some(clean) =
                    img_val.strip_prefix(tako_themes::overrides::DRAFT_BACKGROUND_PREFIX)
                {
                    format!("background.{clean}")
                } else {
                    img_val.clone()
                }
            } else {
                ctx.t("settings.appearance.bg_no_image")
            };

            let theme_id_for_img = theme_id_opt.clone();
            let choose_img_row = MenuItem::new(
                PanelRow::action("choose_image", ctx.t("settings.appearance.bg_choose_image"))
                    .desc(ctx.t("settings.appearance.bg_choose_image_desc"))
                    .value(img_label),
            )
            .on(move |ctx| {
                let title = ctx.t("settings.appearance.bg_choose_image");
                let filter_name = ctx.t("settings.appearance.bg_filter_images");
                let themes_root = ctx.paths.themes();
                let theme_id = theme_id_for_img.clone();
                let theme_id_res = theme_id_for_img.clone();
                crate::tasks::run(
                    ctx,
                    move || {
                        let picked = rfd::FileDialog::new()
                            .set_title(&title)
                            .add_filter(
                                &filter_name,
                                tako_themes::overrides::ALLOWED_BACKGROUND_EXTENSIONS,
                            )
                            .pick_file();
                        picked.map(|src| {
                            let orig_name =
                                src.file_name().and_then(|n| n.to_str()).map(str::to_owned);
                            tako_themes::overrides::copy_draft_background_image(
                                &themes_root,
                                theme_id.as_deref(),
                                &src,
                            )
                            .map(|fname| (fname, orig_name))
                        })
                    },
                    move |ctx, result| match result {
                        None => {}
                        Some(Ok((filename, orig_name))) => {
                            let mut d = current_draft(ctx);
                            d.background_image = Some(filename);
                            d.background_source_name = orig_name;
                            d.background_mode = Some(tako_themes::overrides::BackgroundMode::Image);
                            d.canvas = None;
                            d.draft_dir = Some(tako_themes::overrides::override_dir(
                                &ctx.paths.themes(),
                                theme_id_res.as_deref(),
                            ));
                            ctx.theme_draft = Some(d);
                            ctx.dirty = true;
                        }
                        Some(Err(err)) => {
                            let msg = match err {
                                tako_themes::overrides::CopyImageError::NotFound(_) => {
                                    ctx.t("settings.appearance.bg_image_err_not_found")
                                }
                                tako_themes::overrides::CopyImageError::InvalidExtension => {
                                    ctx.t("settings.appearance.bg_image_err_invalid_ext")
                                }
                                tako_themes::overrides::CopyImageError::InvalidContent => {
                                    ctx.t("settings.appearance.bg_image_err_invalid_header")
                                }
                                tako_themes::overrides::CopyImageError::TooLarge => {
                                    ctx.t("settings.appearance.bg_image_err_too_large")
                                }
                                tako_themes::overrides::CopyImageError::Io(e) => {
                                    ctx.tf("settings.appearance.bg_image_err_io", &[("error", &e)])
                                }
                            };
                            ctx.toast(msg);
                        }
                    },
                );
                Flow::Stay
            });
            rows.push(choose_img_row);
        }

        let mode_for_prev = mode_clone.clone();
        let preview_row = MenuItem::new(
            PanelRow::action("preview", ctx.t("settings.appearance.live_preview"))
                .desc(ctx.t("settings.appearance.live_preview_desc")),
        )
        .on(move |ctx| {
            let draft = current_draft(ctx);
            match &mode_for_prev {
                ThemeEditorMode::OverrideCurrent => {
                    let theme_id = tako_themes::selected(&ctx.paths.themes());
                    super::theme_preview::start_override(ctx, theme_id.as_deref(), draft)
                }
                ThemeEditorMode::CreateNew { base_id } => {
                    super::theme_preview::start_override_create(ctx, base_id.as_deref(), draft)
                }
            }
        });
        if !ctx.desktop {
            rows.push(preview_row);
        }

        match &mode_clone {
            ThemeEditorMode::CreateNew { base_id } => {
                let base_clone = base_id.clone();
                let save_new_row = MenuItem::new(
                    PanelRow::button("save_new", ctx.t("settings.appearance.save_as_new_theme"))
                        .primary(),
                )
                .on(move |ctx| Flow::Push(Box::new(prompt_theme_name(ctx, base_clone.clone(), false))));
                rows.push(save_new_row);
            }
            ThemeEditorMode::OverrideCurrent => {
                let save_row = MenuItem::new(
                    PanelRow::button("save", ctx.t("settings.appearance.save_theme")).primary(),
                )
                .on(|ctx| {
                    let theme_id = tako_themes::selected(&ctx.paths.themes());
                    let mut draft = current_draft(ctx);
                    if let Some(ref bg_draft) = draft.background_image {
                        match tako_themes::overrides::promote_background_draft(
                            &ctx.paths.themes(),
                            theme_id.as_deref(),
                            bg_draft,
                        ) {
                            Ok(final_name) => {
                                draft.background_image = Some(final_name);
                            }
                            Err(e) => {
                                tracing::error!("promover rascunho de fundo: {e}");
                                ctx.toast(ctx.t("settings.appearance.save_override_error"));
                                return Flow::Stay;
                            }
                        }
                    } else {
                        let _ = tako_themes::overrides::clean_background_orphans(
                            &ctx.paths.themes(),
                            theme_id.as_deref(),
                            None,
                        );
                    }
                    let toml_str = draft.to_toml_string();
                    match tako_themes::overrides::save_override(
                        &ctx.paths.themes(),
                        theme_id.as_deref(),
                        &toml_str,
                    ) {
                        Ok(_) => {
                            let text = ctx.t("settings.appearance.theme_customized_saved");
                            ctx.toast(text);
                            ctx.theme_draft = None;
                            ctx.reload_theme_cache();
                            let apply_id = theme_id.unwrap_or_else(|| BUILTIN_THEME.to_owned());
                            ctx.effect(Effect::ApplyTheme(apply_id));
                            Flow::Close
                        }
                        Err(e) => {
                            tracing::error!("salvar sobreposição: {e}");
                            ctx.toast(ctx.t("settings.appearance.save_override_error"));
                            Flow::Stay
                        }
                    }
                });
                rows.push(save_row);

                let show_save_as_new = match &theme_id_opt {
                    Some(id) => tako_themes::create::uses_default_ui(&ctx.paths.themes().join(id)),
                    None => true,
                };
                if show_save_as_new {
                    let theme_id_for_new = theme_id_opt.clone();
                    let save_as_new_row = MenuItem::new(
                        PanelRow::action("save_as_new", ctx.t("settings.appearance.save_as_new_theme"))
                            .desc(ctx.t("settings.appearance.save_as_new_theme_desc")),
                    )
                    .on(move |ctx| Flow::Push(Box::new(prompt_theme_name(ctx, theme_id_for_new.clone(), false))));
                    rows.push(save_as_new_row);
                }

                if let Some(ref id) = theme_id_opt {
                    let theme_dir = ctx.paths.themes().join(id);
                    if tako_themes::create::is_tokens_only_theme(&theme_dir) {
                        let id_export = id.clone();
                        rows.push(
                            MenuItem::new(
                                PanelRow::action("export", ctx.t("settings.appearance.export_theme"))
                                    .desc(ctx.t("settings.appearance.export_theme_desc"))
                                    .icon("folder"),
                            )
                            .on(move |ctx| {
                                let draft = current_draft(ctx);
                                run_export_theme(ctx, &id_export, Some(draft))
                            }),
                        );
                    }
                }

                let has_ov = tako_themes::overrides::has_override(
                    &ctx.paths.themes(),
                    tako_themes::selected(&ctx.paths.themes()).as_deref(),
                );
                if has_ov || ctx.theme_draft.is_some() {
                    let reset_row = MenuItem::new(
                        PanelRow::action("reset", ctx.t("settings.appearance.reset_theme"))
                            .desc(ctx.t("settings.appearance.reset_theme_desc"))
                            .style("danger"),
                    )
                    .on(|ctx| {
                        Flow::Push(Box::new(
                            MenuLayer::dialog(
                                ctx.t("settings.appearance.reset_theme_confirm_title"),
                                |ctx| {
                                    vec![
                                        MenuItem::new(
                                            PanelRow::button(
                                                "reset",
                                                ctx.t("settings.appearance.reset_theme_confirm_button"),
                                            )
                                            .danger(),
                                        )
                                        .on(|ctx| {
                                            let theme_id = tako_themes::selected(&ctx.paths.themes());
                                            ctx.discard_theme_draft();
                                            match tako_themes::overrides::reset_override(
                                                &ctx.paths.themes(),
                                                theme_id.as_deref(),
                                            ) {
                                                Ok(_) => {
                                                    let text =
                                                        ctx.t("settings.appearance.theme_reset_applied");
                                                    ctx.toast(text);
                                                    ctx.reload_theme_cache();
                                                    let apply_id = theme_id
                                                        .unwrap_or_else(|| BUILTIN_THEME.to_owned());
                                                    ctx.effect(Effect::ApplyTheme(apply_id));
                                                    Flow::CloseAll
                                                }
                                                Err(e) => {
                                                    tracing::error!("restaurar sobreposição: {e}");
                                                    ctx.toast(
                                                        ctx.t("settings.appearance.reset_override_error"),
                                                    );
                                                    Flow::Stay
                                                }
                                            }
                                        }),
                                        MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel")))
                                            .on(|_| Flow::Close),
                                    ]
                                },
                            )
                            .text(ctx.t("settings.appearance.reset_theme_confirm_text"))
                            .focus_at(1),
                        ))
                    });
                    rows.push(reset_row);
                }
            }
        }

        rows
    })
    .on_back(|ctx| {
        ctx.discard_theme_draft();
        Flow::Close
    })
}

pub(crate) fn choose_base_theme_panel(ctx: &Ctx) -> MenuLayer {
    MenuLayer::panel(ctx.t("settings.appearance.choose_base_theme"), |ctx| {
        let mut rows = vec![
            MenuItem::new(
                PanelRow::action("base-default", ctx.t("settings.appearance.builtin"))
                    .desc(ctx.t("settings.appearance.builtin_desc"))
                    .icon("palette"),
            )
            .on(|ctx| start_create_from_base(ctx, None)),
        ];

        let themes_data = cached_themes(ctx);
        for candidate in &themes_data.base_candidates {
            let id = &candidate.id;
            let title = &candidate.name;
            let desc = if candidate.author.trim().is_empty() {
                ctx.tf(
                    "settings.appearance.theme_version",
                    &[("version", &candidate.version)],
                )
            } else {
                ctx.tf(
                    "settings.appearance.theme_by",
                    &[
                        ("version", &candidate.version),
                        ("author", &candidate.author),
                    ],
                )
            };
            let id_clone = id.clone();
            rows.push(
                MenuItem::new(
                    PanelRow::action(format!("base-{id}"), title)
                        .desc(desc)
                        .icon("palette"),
                )
                .on(move |ctx| start_create_from_base(ctx, Some(&id_clone))),
            );
        }
        rows
    })
}

pub(crate) fn start_create_from_base(ctx: &mut Ctx, base_id: Option<&str>) -> Flow {
    ctx.discard_theme_draft();
    let themes_root = ctx.paths.themes();

    let draft = match base_id {
        Some(id) => {
            let theme_dir = themes_root.join(id);
            let text = std::fs::read_to_string(theme_dir.join("tokens.toml")).unwrap_or_default();
            let mut d = tako_themes::overrides::ThemeTokensOverride::from_toml_str(&text);
            d.extra.retain(|k, _| !tako_themes::create::is_audio_key(k));
            if let Some(ref bg_img) = d.background_image {
                let src_path = theme_dir.join(bg_img);
                if let Ok(draft_name) = tako_themes::overrides::copy_draft_background_image(
                    &themes_root,
                    Some(id),
                    &src_path,
                ) {
                    d.background_image = Some(draft_name);
                }
            }
            d
        }
        None => tako_themes::overrides::ThemeTokensOverride::default(),
    };

    let mut draft = draft;
    draft.draft_dir = Some(tako_themes::overrides::override_dir(&themes_root, base_id));
    ctx.theme_draft = Some(draft);
    Flow::Replace(Box::new(create_theme_editor_panel(
        ctx,
        base_id.map(str::to_owned),
    )))
}

/// `from_preview`: o teclado substituiu a prévia, então não há editor embaixo e cancelar descarta o rascunho.
pub(crate) fn prompt_theme_name(
    ctx: &Ctx,
    base_id: Option<String>,
    from_preview: bool,
) -> super::keyboard::KeyboardLayer {
    let base_clone = base_id.clone();
    super::keyboard::KeyboardLayer::new(
        ctx,
        ctx.t("settings.appearance.theme_name_title"),
        "",
        move |ctx, name| finish_create_theme(ctx, name, None, base_clone.as_deref(), false),
    )
    .hint(ctx.t("settings.appearance.theme_name_hint"))
    .on_cancel(move |ctx| {
        if from_preview {
            ctx.discard_theme_draft();
        }
    })
    .check(
        |ctx, name| match tako_themes::create::validate_theme_name(name) {
            Ok(()) => None,
            Err(tako_themes::create::ThemeNameProblem::Empty) => {
                Some(ctx.t("settings.appearance.theme_name_err_empty"))
            }
            Err(tako_themes::create::ThemeNameProblem::TooLong(_)) => {
                Some(ctx.t("settings.appearance.theme_name_err_too_long"))
            }
            Err(tako_themes::create::ThemeNameProblem::InvalidCharacters) => {
                Some(ctx.t("settings.appearance.theme_name_err_invalid"))
            }
        },
    )
}

pub(crate) fn finish_create_theme(
    ctx: &mut Ctx,
    name: &str,
    custom_id: Option<&str>,
    base_id: Option<&str>,
    overwrite: bool,
) -> Flow {
    let name = name.trim().to_owned();
    if let Err(e) = tako_themes::create::validate_theme_name(&name) {
        let msg = match e {
            tako_themes::create::ThemeNameProblem::Empty => {
                ctx.t("settings.appearance.theme_name_err_empty")
            }
            tako_themes::create::ThemeNameProblem::TooLong(_) => {
                ctx.t("settings.appearance.theme_name_err_too_long")
            }
            tako_themes::create::ThemeNameProblem::InvalidCharacters => {
                ctx.t("settings.appearance.theme_name_err_invalid")
            }
        };
        ctx.toast(msg);
        return Flow::Stay;
    }

    let id = custom_id
        .map(str::to_owned)
        .unwrap_or_else(|| tako_themes::create::derive_theme_id(&name));
    let themes_root = ctx.paths.themes();
    let dest = themes_root.join(&id);

    if dest.exists() && !overwrite {
        let existing_manifest = tako_themes::validate_package(&dest).ok();
        let existing_name = existing_manifest
            .as_ref()
            .map(|m| m.name.clone())
            .unwrap_or_else(|| id.clone());
        let is_created_by_tako = existing_manifest
            .as_ref()
            .is_some_and(|m| m.created_by_tako);
        let is_reserved = tako_themes::create::is_reserved_theme_id(&id);
        let unique_id = tako_themes::create::unique_theme_id(&themes_root, &name);

        let name_clone = name.clone();
        let base_clone = base_id.map(str::to_owned);
        let unique_id_clone = unique_id.clone();

        if !is_reserved && is_created_by_tako {
            // Oferece sobrescrever tema criado pelo usuário
            return Flow::Push(Box::new(
                MenuLayer::dialog(
                    ctx.t("settings.appearance.theme_overwrite_title"),
                    move |ctx| {
                        let name_sub1 = name_clone.clone();
                        let base_sub1 = base_clone.clone();
                        let name_sub2 = name_clone.clone();
                        let base_sub2 = base_clone.clone();
                        let unique_sub = unique_id_clone.clone();
                        let unique_btn_label = ctx.tf(
                            "settings.appearance.theme_overwrite_suggest",
                            &[("name", &unique_id_clone)],
                        );
                        vec![
                            MenuItem::new(
                                PanelRow::button(
                                    "overwrite",
                                    ctx.t("settings.appearance.theme_overwrite_button"),
                                )
                                .danger(),
                            )
                            .on(move |ctx| {
                                finish_create_theme(
                                    ctx,
                                    &name_sub1,
                                    None,
                                    base_sub1.as_deref(),
                                    true,
                                )
                            }),
                            MenuItem::new(PanelRow::button("suggest", unique_btn_label)).on(
                                move |ctx| {
                                    finish_create_theme(
                                        ctx,
                                        &name_sub2,
                                        Some(&unique_sub),
                                        base_sub2.as_deref(),
                                        false,
                                    )
                                },
                            ),
                            MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel")))
                                .on(|_| Flow::Close),
                        ]
                    },
                )
                .text(ctx.tf(
                    "settings.appearance.theme_overwrite_confirm",
                    &[("name", &existing_name), ("suggested", &unique_id)],
                ))
                .focus_at(2),
            ));
        } else {
            // Tema reservado ou de terceiros: nunca pode ser sobrescrito por aqui
            let cannot_text = ctx.tf(
                "settings.appearance.theme_cannot_overwrite_confirm",
                &[("name", &existing_name), ("suggested", &unique_id)],
            );
            return Flow::Push(Box::new(
                MenuLayer::dialog(
                    ctx.t("settings.appearance.theme_cannot_overwrite_title"),
                    move |ctx| {
                        let name_sub = name_clone.clone();
                        let base_sub = base_clone.clone();
                        let unique_sub = unique_id_clone.clone();
                        let unique_btn_label = ctx.tf(
                            "settings.appearance.theme_overwrite_suggest",
                            &[("name", &unique_id_clone)],
                        );
                        vec![
                            MenuItem::new(PanelRow::button("suggest", unique_btn_label).primary())
                                .on(move |ctx| {
                                    finish_create_theme(
                                        ctx,
                                        &name_sub,
                                        Some(&unique_sub),
                                        base_sub.as_deref(),
                                        false,
                                    )
                                }),
                            MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel")))
                                .on(|_| Flow::Close),
                        ]
                    },
                )
                .text(cannot_text)
                .focus_at(0),
            ));
        }
    }

    let draft = current_draft(ctx);
    let bg_source = if let Some(ref bg_file) = draft.background_image {
        let p_draft = draft.draft_dir.as_ref().map(|d| d.join(bg_file));
        let p1 = tako_themes::overrides::override_dir(&themes_root, base_id).join(bg_file);
        if p_draft.as_ref().is_some_and(|p| p.is_file()) {
            p_draft
        } else if p1.is_file() {
            Some(p1)
        } else {
            None
        }
    } else {
        None
    };

    let profile_name = super::profiles::active_name(ctx);
    let author_opt = if profile_name.is_empty() {
        None
    } else {
        Some(profile_name.as_str())
    };

    let base_dir_buf = base_id.map(|b| themes_root.join(b));
    let opts = tako_themes::create::CreateThemeOptions {
        name: &name,
        custom_id,
        author: author_opt,
        version: Some("1.0.0"),
        description: None,
        tokens: &draft,
        background_image_source: bg_source.as_deref(),
        overwrite,
        base_theme_dir: base_dir_buf.as_deref(),
    };

    match tako_themes::create::create_theme(&themes_root, &opts) {
        Ok(installed) => {
            let created_id = installed.id();
            forget(THEMES_CACHE);
            ctx.discard_theme_draft();
            ctx.reload_theme_cache();

            let id_apply = created_id.clone();
            let name_apply = name.clone();
            let name_dialog = name.clone();
            Flow::Replace(Box::new(
                MenuLayer::dialog(
                    ctx.t("settings.appearance.theme_created_title"),
                    move |ctx| {
                        let id_now = id_apply.clone();
                        let name_now = name_apply.clone();
                        let name_keep = name_apply.clone();
                        vec![
                            MenuItem::new(
                                PanelRow::button(
                                    "apply",
                                    ctx.t("settings.appearance.theme_apply_now"),
                                )
                                .primary(),
                            )
                            .on(move |ctx| {
                                match tako_themes::select(&ctx.paths.themes(), Some(&id_now)) {
                                    Ok(()) => {
                                        ctx.reload_theme_cache();
                                        ctx.effect(Effect::ApplyTheme(id_now.clone()));
                                        ctx.toast(ctx.tf(
                                            "settings.appearance.theme_created_applied",
                                            &[("name", &name_now)],
                                        ));
                                        Flow::CloseAll
                                    }
                                    Err(e) => {
                                        let err_str = e.to_string();
                                        ctx.toast(ctx.tf(
                                            "settings.appearance.theme_create_error",
                                            &[("error", &err_str)],
                                        ));
                                        Flow::CloseAll
                                    }
                                }
                            }),
                            MenuItem::new(PanelRow::button(
                                "keep",
                                ctx.t("settings.appearance.theme_keep_current"),
                            ))
                            .on(move |ctx| {
                                ctx.toast(ctx.tf(
                                    "settings.appearance.theme_created",
                                    &[("name", &name_keep)],
                                ));
                                Flow::CloseAll
                            }),
                        ]
                    },
                )
                .text(ctx.tf(
                    "settings.appearance.theme_created_apply_prompt",
                    &[("name", &name_dialog)],
                )),
            ))
        }
        Err(e) => {
            tracing::error!("erro ao criar tema: {e}");
            let msg = match e {
                tako_themes::create::CreateThemeError::InvalidName(
                    tako_themes::create::ThemeNameProblem::Empty,
                ) => ctx.t("settings.appearance.theme_name_err_empty"),
                tako_themes::create::CreateThemeError::InvalidName(
                    tako_themes::create::ThemeNameProblem::TooLong(_),
                ) => ctx.t("settings.appearance.theme_name_err_too_long"),
                tako_themes::create::CreateThemeError::InvalidName(
                    tako_themes::create::ThemeNameProblem::InvalidCharacters,
                ) => ctx.t("settings.appearance.theme_name_err_invalid"),
                tako_themes::create::CreateThemeError::AlreadyExists(existing_id) => ctx.tf(
                    "settings.appearance.theme_err_already_exists",
                    &[("id", &existing_id)],
                ),
                tako_themes::create::CreateThemeError::Reserved(res_id) => {
                    ctx.tf("settings.appearance.theme_err_reserved", &[("id", &res_id)])
                }
                tako_themes::create::CreateThemeError::CannotOverwrite(target_name) => ctx.tf(
                    "settings.appearance.theme_err_cannot_overwrite",
                    &[("name", &target_name)],
                ),
                tako_themes::create::CreateThemeError::InvalidImage => {
                    ctx.t("settings.appearance.theme_err_invalid_image")
                }
                tako_themes::create::CreateThemeError::Package(_) => {
                    ctx.t("settings.appearance.theme_err_invalid_package")
                }
                tako_themes::create::CreateThemeError::Io(io_err) => {
                    let io_str = io_err.to_string();
                    ctx.tf("settings.appearance.theme_err_io", &[("error", &io_str)])
                }
            };
            ctx.toast(msg);
            Flow::Stay
        }
    }
}

pub(crate) fn export_theme_panel(ctx: &mut Ctx) -> Flow {
    let themes_data = cached_themes(ctx);
    let exportable = &themes_data.exportable;

    if exportable.is_empty() {
        return Flow::Stay;
    }

    if exportable.len() == 1 {
        let id = exportable[0].id();
        return run_export_theme(ctx, &id, None);
    }

    Flow::Push(Box::new(
        MenuLayer::dialog(
            ctx.t("settings.appearance.export_theme_title"),
            move |_ctx| {
                let themes_data = cached_themes(_ctx);
                themes_data
                    .exportable
                    .into_iter()
                    .map(|t| {
                        let id = t.id();
                        let name = t
                            .manifest
                            .as_ref()
                            .map_or_else(|_| id.clone(), |m| m.name.clone());
                        let id_run = id.clone();
                        MenuItem::new(PanelRow::action(id, name).icon("folder"))
                            .on(move |ctx| run_export_theme(ctx, &id_run, None))
                    })
                    .collect()
            },
        )
        .text(ctx.t("settings.appearance.export_theme_choose")),
    ))
}

pub(crate) fn run_export_theme(
    ctx: &mut Ctx,
    theme_id: &str,
    override_draft: Option<tako_themes::overrides::ThemeTokensOverride>,
) -> Flow {
    let theme_dir = ctx.paths.themes().join(theme_id);
    let default_filename = format!("{theme_id}.takotheme");
    let dialog_title = ctx.t("settings.appearance.export_theme_title");
    let filter_name = ctx.t("settings.appearance.theme_export_filter");
    let themes_root = ctx.paths.themes();
    let override_dir = tako_themes::overrides::override_dir(&themes_root, Some(theme_id));
    let theme_id_owned = theme_id.to_owned();

    crate::tasks::run(
        ctx,
        move || {
            let picked = rfd::FileDialog::new()
                .set_title(&dialog_title)
                .set_file_name(&default_filename)
                .add_filter(&filter_name, &["takotheme"])
                .save_file();
            let dest_path = picked?;
            let ov_tokens = override_draft.or_else(|| {
                tako_themes::overrides::load_override(&themes_root, Some(&theme_id_owned))
                    .map(|s| tako_themes::overrides::ThemeTokensOverride::from_toml_str(&s))
            });
            let res = tako_themes::export::export_theme_with_override(
                &theme_dir,
                &dest_path,
                ov_tokens.as_ref(),
                Some(&override_dir),
            );
            Some((res, dest_path))
        },
        move |ctx, result| {
            let Some((res, dest_path)) = result else {
                return;
            };
            match res {
                Ok(()) => {
                    let path_str = dest_path.display().to_string();
                    ctx.toast(ctx.tf("settings.appearance.theme_exported", &[("path", &path_str)]));
                }
                Err(e) => {
                    tracing::error!("falha ao exportar tema: {e}");
                    let err_str = match e {
                        tako_themes::export::ExportThemeError::NotFound(p) => ctx.tf(
                            "settings.appearance.theme_export_err_not_found",
                            &[("path", &p)],
                        ),
                        tako_themes::export::ExportThemeError::InvalidPackage(_) => {
                            ctx.t("settings.appearance.theme_err_invalid_package")
                        }
                        tako_themes::export::ExportThemeError::Io(io_e) => {
                            let s = io_e.to_string();
                            ctx.tf("settings.appearance.theme_err_io", &[("error", &s)])
                        }
                        tako_themes::export::ExportThemeError::Zip(z_e) => {
                            let s = z_e.to_string();
                            ctx.tf("settings.appearance.theme_export_err_zip", &[("error", &s)])
                        }
                    };
                    ctx.toast(ctx.tf(
                        "settings.appearance.theme_export_error",
                        &[("error", &err_str)],
                    ));
                }
            }
        },
    );
    Flow::Close
}

fn appearance(ctx: &Ctx) -> Vec<MenuItem> {
    let mut items = vec![
        MenuItem::new(PanelRow::choice(
            "text-size",
            ctx.t("accessibility.text_size"),
            format!("{}%", crate::navigation_feedback::text_size(&ctx.settings)),
        ))
        .on(|ctx| change_text_size(ctx, 1))
        .on_adjust(change_text_size)
        .options(|ctx| {
            use crate::navigation_feedback::{self as feedback, TEXT_SIZES};
            let now = feedback::text_size(&ctx.settings);
            let cur = TEXT_SIZES.iter().position(|n| *n == now).unwrap_or(0);
            let picks = TEXT_SIZES
                .iter()
                .map(|n| Pick::new(format!("{n}%")))
                .collect();
            (picks, cur)
        }),
        MenuItem::new(
            PanelRow::action("theme", ctx.t("settings.appearance.theme"))
                .desc(ctx.t("settings.appearance.theme_desc"))
                .value(theme_name(ctx)),
        )
        .on(|ctx| Flow::Push(Box::new(themes_panel(ctx)))),
    ];
    if ctx.theme_supports_ui {
        items.push(
            MenuItem::new(
                PanelRow::action("edit-theme", ctx.t("settings.appearance.edit_theme"))
                    .desc(ctx.t("settings.appearance.edit_theme_desc")),
            )
            .on(|ctx| Flow::Push(Box::new(edit_theme_panel(ctx)))),
        );
    }
    items.push(
        MenuItem::new(
            PanelRow::action("create-theme", ctx.t("settings.appearance.create_theme"))
                .desc(ctx.t("settings.appearance.create_theme_desc")),
        )
        .on(|ctx| {
            let data = cached_themes(ctx);
            if data.base_candidates.is_empty() {
                start_create_from_base(ctx, None)
            } else {
                Flow::Push(Box::new(choose_base_theme_panel(ctx)))
            }
        }),
    );
    items.extend([
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
    ]);
    items
        .into_iter()
        // Tema, cor de destaque e Liquid Glass (só no desktop).
        .chain(
            ctx.desktop
                .then(|| desk_theme::rows(ctx))
                .into_iter()
                .flatten(),
        )
        // "Reduzir transparência" só existe no desktop (a tela cheia não tem vidro).
        .chain(ctx.desktop.then(|| {
            toggle(
                ctx,
                "settings.appearance.transparency",
                keys::REDUCED_TRANSPARENCY,
                false,
            )
        }))
        .chain(ctx.desktop.then(|| title_row(ctx)))
        // Layout: BarraLateralUnida e a largura da barra lateral (só no desktop).
        .chain(
            ctx.desktop.then(|| {
                MenuItem::new(PanelRow::header(ctx.t("settings.appearance.layout_header")))
            }),
        )
        .chain(ctx.desktop.then(|| {
            toggle(
                ctx,
                "settings.appearance.sidebar_unified",
                keys::SIDEBAR_UNIFIED,
                false,
            )
        }))
        .chain(ctx.desktop.then(|| sidebar_width_row(ctx)))
        .collect()
}

/// Largura da barra lateral: Normal (padrão) ou Estreita; vale para a lista flutuante e para a unida.
fn sidebar_width_row(ctx: &Ctx) -> MenuItem {
    const MODES: [(&str, &str); 2] = [("", "sidebar_normal"), ("narrow", "sidebar_narrow")];
    let narrow = ctx.settings.string(keys::SIDEBAR_WIDTH) == "narrow";
    MenuItem::new(
        PanelRow::choice(
            "sidebar-width",
            ctx.t("settings.appearance.sidebar_width"),
            ctx.t(&format!(
                "settings.appearance.{}",
                MODES[usize::from(narrow)].1
            )),
        )
        .desc(ctx.t("settings.appearance.sidebar_width_desc")),
    )
    .options(|ctx| {
        let narrow = ctx.settings.string(keys::SIDEBAR_WIDTH) == "narrow";
        let picks = MODES
            .iter()
            .map(|(value, label)| {
                Pick::new(ctx.t(&format!("settings.appearance.{label}"))).set(move |ctx| {
                    ctx.settings.set(keys::SIDEBAR_WIDTH, value);
                    Flow::Stay
                })
            })
            .collect();
        (picks, usize::from(narrow))
    })
}

/// TituloDoJogo: "Logo do jogo quando houver" (padrão) ou "Sempre o nome em texto".
fn title_row(ctx: &Ctx) -> MenuItem {
    const MODES: [(&str, &str); 2] = [("", "title_logo"), ("text", "title_text")];
    let text = ctx.settings.string(keys::TITLE_LOGO) == "text";
    MenuItem::new(
        PanelRow::choice(
            "title",
            ctx.t("settings.appearance.game_title"),
            ctx.t(&format!(
                "settings.appearance.{}",
                MODES[usize::from(text)].1
            )),
        )
        .desc(ctx.t("settings.appearance.game_title_desc")),
    )
    .options(|ctx| {
        let text = ctx.settings.string(keys::TITLE_LOGO) == "text";
        let picks = MODES
            .iter()
            .map(|(value, label)| {
                Pick::new(ctx.t(&format!("settings.appearance.{label}"))).set(move |ctx| {
                    ctx.settings.set(keys::TITLE_LOGO, value);
                    Flow::Stay
                })
            })
            .collect();
        (picks, usize::from(text))
    })
}

fn change_text_size(ctx: &mut Ctx, direction: i32) -> Flow {
    use crate::navigation_feedback::{self as feedback, TEXT_SIZES};
    let index = TEXT_SIZES
        .iter()
        .position(|n| *n == feedback::text_size(&ctx.settings))
        .unwrap_or(0) as i32;
    let next = (index + direction.signum()).rem_euclid(TEXT_SIZES.len() as i32) as usize;
    ctx.settings
        .set(feedback::TEXT_SIZE, &TEXT_SIZES[next].to_string());
    ctx.dirty = true;
    Flow::Stay
}

fn sound(ctx: &Ctx) -> Vec<MenuItem> {
    use crate::navigation_feedback as feedback;
    let music_on = ctx.settings.bool(feedback::THEME_MUSIC_ENABLED, true);
    let mut rows = vec![
        toggle(
            ctx,
            "settings.sound.music_enabled",
            feedback::THEME_MUSIC_ENABLED,
            true,
        ),
        MenuItem::new(
            PanelRow {
                kind: if ctx.desktop { "slider" } else { "choice" }.into(),
                ..PanelRow::choice(
                    "theme-music-volume",
                    ctx.t("settings.sound.music_volume"),
                    if ctx.desktop {
                        match feedback::music_volume(&ctx.settings) {
                            0 => ctx.t("settings.sound.music_muted"),
                            v => v.to_string(),
                        }
                    } else {
                        format!("{}%", feedback::music_volume(&ctx.settings))
                    },
                )
            }
            .desc(ctx.t("settings.sound.music_volume_desc"))
            .enabled(music_on),
        )
        .on(|ctx| change_theme_music_volume(ctx, 1))
        .on_adjust(change_theme_music_volume)
        .on_slide(|ctx, v, _done| {
            use crate::navigation_feedback as feedback;
            ctx.settings
                .set(feedback::THEME_MUSIC_VOLUME, &v.to_string());
            ctx.dirty = true;
            Flow::Stay
        })
        .options(|ctx| {
            use crate::navigation_feedback as feedback;
            let now = feedback::music_volume(&ctx.settings);
            let steps: Vec<u8> = (0..=100).step_by(10).collect();
            let cur = steps.iter().position(|v| *v == now).unwrap_or(7);
            let picks = steps
                .into_iter()
                .map(|v| Pick::new(format!("{v}%")))
                .collect();
            (picks, cur)
        }),
        MenuItem::new(
            PanelRow {
                kind: if ctx.desktop { "slider" } else { "choice" }.into(),
                ..PanelRow::choice(
                    "navigation-volume",
                    ctx.t("settings.sound.navigation_volume"),
                    if ctx.desktop {
                        match feedback::volume(&ctx.settings) {
                            0 => ctx.t("accessibility.sound_muted"),
                            v => v.to_string(),
                        }
                    } else {
                        format!("{}%", feedback::volume(&ctx.settings))
                    },
                )
            }
            .desc(ctx.t("settings.sound.navigation_volume_desc")),
        )
        .on(|ctx| change_navigation_volume(ctx, 1))
        .on_adjust(change_navigation_volume)
        .on_slide(|ctx, v, done| {
            use crate::navigation_feedback as feedback;
            ctx.settings.set(feedback::SOUND_VOLUME, &v.to_string());
            ctx.dirty = true;
            if done {
                feedback::play(ctx, Cmd::Right);
            }
            Flow::Stay
        })
        .options(|ctx| {
            use crate::navigation_feedback as feedback;
            let now = feedback::volume(&ctx.settings);
            let steps: Vec<u8> = (0..=100).step_by(10).collect();
            let cur = steps.iter().position(|v| *v == now).unwrap_or(0);
            let picks = steps
                .into_iter()
                .map(|v| Pick::new(format!("{v}%")))
                .collect();
            (picks, cur)
        }),
    ];
    rows.extend(screen::audio_rows(ctx));
    rows
}

fn change_theme_music_volume(ctx: &mut Ctx, direction: i32) -> Flow {
    use crate::navigation_feedback as feedback;
    let step = if ctx.desktop { 5 } else { 10 };
    let next = (i32::from(feedback::music_volume(&ctx.settings)) + direction.signum() * step)
        .clamp(0, 100);
    ctx.settings
        .set(feedback::THEME_MUSIC_VOLUME, &next.to_string());
    ctx.dirty = true;
    Flow::Stay
}

fn change_navigation_volume(ctx: &mut Ctx, direction: i32) -> Flow {
    use crate::navigation_feedback as feedback;
    // No desktop ← → andam 5 (Shift 10, em `MenuItem::slide_key`); na tela cheia, 10.
    let step = if ctx.desktop { 5 } else { 10 };
    let next =
        (i32::from(feedback::volume(&ctx.settings)) + direction.signum() * step).clamp(0, 100);
    ctx.settings.set(feedback::SOUND_VOLUME, &next.to_string());
    ctx.dirty = true;
    Flow::Stay
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
        MenuItem::new(action(ctx, "settings.system.performance"))
            .on(|ctx| Flow::Push(Box::new(super::performance::panel(ctx)))),
        MenuItem::new(PanelRow::header(ctx.t("settings.system.recovery_header"))),
    ];
    v.push(if ctx.safe_mode {
        MenuItem::new(
            PanelRow::action("safemode", ctx.t("settings.system.safemode"))
                .desc(ctx.t("settings.system.safemode_on_desc"))
                .action_btn(ctx.t("settings.system.safemode_exit")),
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

    v.push(MenuItem::new(PanelRow::header(
        ctx.t("settings.system.backup_header"),
    )));
    v.push(
        MenuItem::new(action(ctx, "settings.system.backup")).on(|ctx| {
            choose_backup(ctx);
            Flow::Stay
        }),
    );
    v.push(
        MenuItem::new({
            let row = action(ctx, "settings.system.restore");
            if ctx.desktop { row.danger() } else { row }
        })
        .on(|ctx| {
            choose_restore(ctx);
            Flow::Stay
        }),
    );
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

fn choose_backup(ctx: &mut Ctx) {
    if ctx.restoring {
        ctx.toast(ctx.t("settings.system.restore_busy"));
        return;
    }
    let title = ctx.t("settings.system.backup_choose_title");
    let file_name = format!(
        "tako-backup-{}.takobackup",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );
    super::add_game::pick(
        ctx,
        move || {
            rfd::FileDialog::new()
                .set_title(&title)
                .set_file_name(&file_name)
                .add_filter("Tako Backup", &["takobackup"])
                .save_file()
        },
        |ctx, path| {
            let paths = ctx.paths.clone();
            let p = path.clone();
            crate::tasks::run(
                ctx,
                move || tako_core::persistence::backup::create_backup(&paths, &p),
                move |ctx, res| match res {
                    Ok(_) => {
                        ctx.toast_success(ctx.tf(
                            "settings.system.backup_done",
                            &[("path", &path.display().to_string())],
                        ));
                    }
                    Err(e) => {
                        ctx.toast_error(ctx.tf(
                            "settings.system.backup_failed",
                            &[("error", &e.to_string())],
                        ));
                    }
                },
            );
        },
    );
}

fn choose_restore(ctx: &mut Ctx) {
    if ctx.restoring {
        ctx.toast(ctx.t("settings.system.restore_busy"));
        return;
    }
    if ctx.running.is_some() {
        ctx.toast(ctx.t("settings.system.restore_blocked_game_running"));
        return;
    }
    if ctx.scanning {
        ctx.toast(ctx.t("settings.system.restore_blocked_scanning"));
        return;
    }
    let title = ctx.t("settings.system.restore_choose_title");
    super::add_game::pick(
        ctx,
        move || {
            rfd::FileDialog::new()
                .set_title(&title)
                .add_filter("Tako Backup", &["takobackup"])
                .pick_file()
        },
        |ctx, path| {
            let layer = Box::new(confirm_restore_layer(ctx, path));
            super::add_game::open_layer(ctx, layer);
        },
    );
}

pub(crate) fn confirm_restore_layer(ctx: &Ctx, path: std::path::PathBuf) -> MenuLayer {
    let title = ctx.t("settings.system.restore_confirm_title");
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let size_bytes = std::fs::metadata(&path).map(|m| m.len()).ok();
    let size_str = size_bytes
        .map(tako_core::metadata::art::format_bytes)
        .unwrap_or_default();
    let desc = if size_str.is_empty() {
        ctx.tf(
            "settings.system.restore_confirm_desc",
            &[("file", &file_name)],
        )
    } else {
        ctx.tf(
            "settings.system.restore_confirm_desc_size",
            &[("file", &file_name), ("size", &size_str)],
        )
    };
    let confirm_btn = ctx.t("settings.system.restore_confirm_button");
    let cancel_btn = ctx.t("common.cancel");
    MenuLayer::dialog(title, move |_ctx| {
        let p = path.clone();
        vec![
            MenuItem::new(PanelRow::button("confirm", confirm_btn.clone()).danger()).on(
                move |ctx| {
                    if ctx.restoring {
                        ctx.toast(ctx.t("settings.system.restore_busy"));
                        return Flow::Close;
                    }
                    if ctx.running.is_some() {
                        ctx.toast(ctx.t("settings.system.restore_blocked_game_running"));
                        return Flow::Close;
                    }
                    if ctx.scanning {
                        ctx.toast(ctx.t("settings.system.restore_blocked_scanning"));
                        return Flow::Close;
                    }
                    if ctx
                        .downloads
                        .manager
                        .as_ref()
                        .is_some_and(|m| m.summary().downloading > 0)
                    {
                        ctx.toast(ctx.t("settings.system.restore_blocked_download_active"));
                        return Flow::Close;
                    }
                    ctx.restoring = true;
                    ctx.settings.locked = true;
                    let paths = ctx.paths.clone();
                    let p = p.clone();
                    crate::tasks::run(
                        ctx,
                        move || tako_core::persistence::backup::restore_backup(&paths, &p),
                        move |ctx, res| {
                            ctx.restoring = false;
                            ctx.settings.locked = false;
                            match res {
                                Ok(res) => {
                                    ctx.reload_data();
                                    ctx.downloads = crate::shell::downloads::State {
                                        store: ctx.downloads.store.take(),
                                        simulated: ctx.downloads.simulated,
                                        ..Default::default()
                                    };
                                    let _ = crate::shell::downloads::manager(ctx);
                                    let lang = match ctx.settings.string(keys::LANGUAGE).as_str() {
                                        "en-US" => Lang::EnUs,
                                        _ => Lang::PtBr,
                                    };
                                    tako_core::i18n::set_lang(lang);
                                    let theme_id = tako_themes::selected(&ctx.paths.themes())
                                        .unwrap_or_else(|| BUILTIN_THEME.into());
                                    ctx.effect(Effect::ApplyTheme(theme_id));
                                    crate::local_api::sync(ctx);

                                    let msg = if let Some(auto) = &res.auto_backup_path {
                                        let auto_name = auto
                                            .file_name()
                                            .map(|n| n.to_string_lossy().into_owned())
                                            .unwrap_or_default();
                                        ctx.tf(
                                            "settings.system.restore_done_with_auto",
                                            &[("auto", &auto_name)],
                                        )
                                    } else {
                                        ctx.t("settings.system.restore_done")
                                    };
                                    ctx.toast_success(msg);
                                }
                                Err(e) => {
                                    ctx.reload_data();
                                    crate::local_api::sync(ctx);
                                    ctx.toast_error(ctx.tf(
                                        "settings.system.restore_failed",
                                        &[("error", &e.to_string())],
                                    ));
                                }
                            }
                        },
                    );
                    Flow::Close
                },
            ),
            MenuItem::new(PanelRow::button("cancel", cancel_btn.clone())).on(|_| Flow::Close),
        ]
    })
    .text(desc)
    .focus_at(1)
}

pub(crate) fn confirm_regenerate_token(ctx: &Ctx) -> MenuLayer {
    MenuLayer::dialog(
        ctx.t("settings.system.api_regenerate_confirm_title"),
        |ctx| {
            vec![
                MenuItem::new(
                    PanelRow::button(
                        "regenerate",
                        ctx.t("settings.system.api_regenerate_confirm_btn"),
                    )
                    .danger(),
                )
                .on(|ctx| {
                    let new_token = tako_core::api::generate_token();
                    ctx.settings.set(keys::API_TOKEN, &new_token);
                    crate::local_api::sync(ctx);
                    ctx.toast_success(ctx.t("settings.system.api_token_regenerated"));
                    Flow::Close
                }),
                MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel")))
                    .on(|_| Flow::Close),
            ]
        },
    )
    .text(ctx.t("settings.system.api_regenerate_confirm_desc"))
    .focus_at(1)
}

pub(crate) fn token_dialog(ctx: &Ctx, token: String) -> MenuLayer {
    let token_for_copy = token.clone();
    let title = ctx.t("settings.system.api_token_dialog_title");
    let desc = ctx.t("settings.system.api_token_dialog_desc");
    let text = format!("{desc}\n\n{token}");
    MenuLayer::dialog(title, move |ctx| {
        let token = token_for_copy.clone();
        vec![
            MenuItem::new(
                PanelRow::button("copy", ctx.t("settings.system.api_token_copy")).emphasized(true),
            )
            .on(move |ctx| {
                if tako_platform::copy_to_clipboard(&token) {
                    ctx.toast_success(ctx.t("settings.system.api_token_copied"));
                } else {
                    ctx.toast_error(ctx.t("settings.system.api_token_copy_failed"));
                }
                Flow::Stay
            }),
            MenuItem::new(PanelRow::button("close", ctx.t("hint.close"))).on(|_| Flow::Close),
        ]
    })
    .text(text)
    .wide()
}

pub(crate) fn api_panel(ctx: &Ctx) -> MenuLayer {
    MenuLayer::panel(ctx.t("settings.system.api_panel_title"), |ctx| {
        let enabled = ctx.settings.bool(keys::API_ENABLED, false);
        let port = ctx.settings.get(keys::API_PORT).unwrap_or("47774");
        vec![
            MenuItem::new(
                PanelRow::toggle("api-enabled", ctx.t("settings.system.api_enabled"), enabled)
                    .desc(ctx.t("settings.system.api_enabled_desc")),
            )
            .on(|ctx| {
                let on = ctx.settings.toggle(keys::API_ENABLED, false);
                let current_token = ctx.settings.string(keys::API_TOKEN);
                if on
                    && (current_token.is_empty() || !tako_core::api::is_valid_token(&current_token))
                {
                    let token = tako_core::api::generate_token();
                    ctx.settings.set(keys::API_TOKEN, &token);
                }
                crate::local_api::sync(ctx);
                Flow::Stay
            }),
            MenuItem::new(
                PanelRow::info("api-port", ctx.t("settings.system.api_port"), port)
                    .desc(ctx.t("settings.system.api_port_desc")),
            ),
            MenuItem::new(
                PanelRow::action("api-token", ctx.t("settings.system.api_token"))
                    .desc(ctx.t("settings.system.api_token_desc"))
                    .action_btn(ctx.t("settings.system.api_token_show")),
            )
            .on(|ctx| {
                let mut token = ctx.settings.string(keys::API_TOKEN);
                if token.is_empty() || !tako_core::api::is_valid_token(&token) {
                    token = tako_core::api::generate_token();
                    ctx.settings.set(keys::API_TOKEN, &token);
                    crate::local_api::sync(ctx);
                }
                Flow::Push(Box::new(token_dialog(ctx, token)))
            }),
            MenuItem::new(
                PanelRow::action("api-regenerate", ctx.t("settings.system.api_regenerate"))
                    .desc(ctx.t("settings.system.api_regenerate_desc"))
                    .danger(),
            )
            .on(|ctx| Flow::Push(Box::new(confirm_regenerate_token(ctx)))),
        ]
    })
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
        MenuItem::new(
            described(ctx, "settings.about.license").value(ctx.t("settings.about.license_value")),
        )
        .on(|ctx| {
            Flow::Push(Box::new(text_dialog(
                ctx.t("settings.about.license_title"),
                ctx.t("settings.about.license_text"),
            )))
        }),
        MenuItem::new(
            described(ctx, "settings.about.notices").value(ctx.t("settings.about.notices_value")),
        )
        .on(|ctx| {
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
    /// Busca de configurações do desktop: as linhas de todas as categorias que combinam com o texto.
    query: String,
}

impl Default for SettingsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsPage {
    fn sync_categories(&mut self, ctx: &Ctx) {
        if self.cats.iter().any(|c| c.id == "downloads") == ctx.desktop {
            return;
        }
        let selected = self.cats[self.cat].id;
        self.cats = categories()
            .into_iter()
            .filter(|c| ctx.desktop || c.id != "downloads")
            .collect();
        self.cat = self.cats.iter().position(|c| c.id == selected).unwrap_or(0);
        self.row = None;
    }

    pub fn new() -> SettingsPage {
        SettingsPage {
            cats: categories(),
            cat: 0,
            row: None,
            query: String::new(),
        }
    }

    pub fn at(category: &str) -> SettingsPage {
        let mut p = Self::new();
        p.cat = p.cats.iter().position(|c| c.id == category).unwrap_or(0);
        p
    }

    fn items(&self, ctx: &Ctx) -> Vec<MenuItem> {
        if self.query.trim().is_empty() {
            return (self.cats[self.cat].rows)(ctx);
        }
        // Busca: as linhas que combinam, agrupadas sob o nome da categoria (cabeçalhos da categoria não contam).
        let mut v = Vec::new();
        for c in &self.cats {
            let found: Vec<MenuItem> = (c.rows)(ctx)
                .into_iter()
                .filter(|i| {
                    i.row.kind != "header"
                        && tako_core::library::search_text::matches(
                            &format!("{} {} {}", i.row.title, i.row.description, i.row.value),
                            &self.query,
                        )
                })
                .collect();
            if !found.is_empty() {
                v.push(MenuItem::new(PanelRow::header(
                    ctx.t(&format!("settings.{}.title", c.id)),
                )));
                v.extend(found);
            }
        }
        v
    }

    /// Quantas linhas combinam com a busca em cada categoria (as sem nada ficam apagadas no painel).
    fn matches_in(&self, ctx: &Ctx, cat: usize) -> usize {
        (self.cats[cat].rows)(ctx)
            .iter()
            .filter(|i| {
                i.row.kind != "header"
                    && tako_core::library::search_text::matches(
                        &format!("{} {} {}", i.row.title, i.row.description, i.row.value),
                        &self.query,
                    )
            })
            .count()
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

    fn key(
        &mut self,
        text: &str,
        mods: tako_core::input::shortcut::KeyModifiers,
        ctx: &mut Ctx,
    ) -> Option<Nav> {
        if !ctx.desktop {
            return None;
        }
        let shift = mods == tako_core::input::shortcut::KeyModifiers::SHIFT;
        let item = self.items(ctx).into_iter().nth(self.row?)?;
        let flow = item.slide_key(text, shift, ctx)?;
        Some(Self::run(flow, ctx))
    }

    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Nav {
        self.sync_categories(ctx);
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
            // No desktop um select só muda pela lista.
            (Some(r), Cmd::Left) | (Some(r), Cmd::Right)
                if ctx.desktop && items.get(r).is_some_and(|i| i.options.is_some()) => {}
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
                if let Some(layer) = item.select(ctx) {
                    return Self::run(Flow::Push(layer), ctx);
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

    fn edited(&mut self, zone: &str, text: &str, _ctx: &mut Ctx) {
        if zone == "search" {
            self.query = text.to_owned();
            self.row = None;
        }
    }

    fn render(&mut self, ctx: &Ctx, out: &mut Out) {
        self.sync_categories(ctx);
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
                selected: i == self.cat && self.query.is_empty(),
                // Na busca, `count` diz quantas linhas da categoria combinam ("0" apaga a categoria).
                count: if !self.query.is_empty() {
                    self.matches_in(ctx, i).to_string()
                } else if ctx.desktop && c.id == "metadata" {
                    // FilaDeCapas: quantos jogos esperam revisão, no item da categoria.
                    super::art_review::pending(ctx).map_or_else(String::new, |n| n.to_string())
                } else {
                    String::new()
                },
                ..Default::default()
            })
            .collect();
        out.set("settings-categories", cats);
        // Só o desktop tem o cabeçalho do painel de categorias (a API de tema da tela cheia não muda).
        if ctx.desktop {
            out.set(
                "settings-count",
                ctx.tf(
                    "desktop.settings_count",
                    &[("n", &self.cats.len().to_string())],
                ),
            );
            out.set("settings-search", self.query.clone());
        }
        out.set("settings-category-focus", self.cat as i32);
        let id = self.cats[self.cat].id;
        if self.query.trim().is_empty() {
            out.set("settings-heading", ctx.t(&format!("settings.{id}.title")));
            out.set(
                "settings-subtitle",
                ctx.t(&format!("settings.{id}.subtitle")),
            );
        } else {
            out.set("settings-heading", format!("“{}”", self.query.trim()));
            let n = self
                .items(ctx)
                .iter()
                .filter(|i| i.row.kind != "header")
                .count();
            out.set(
                "settings-subtitle",
                ctx.tf("desktop.settings_found", &[("n", &n.to_string())]),
            );
        }
        if ctx.desktop {
            let covers: Vec<crate::model::CoverRow> = items
                .iter()
                .map(|i| i.cover.clone().unwrap_or_default())
                .collect();
            out.set("settings-covers", covers);
        }
        let rows: Vec<PanelRow> = items.into_iter().map(|i| i.shown(ctx)).collect();
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

    fn click(&mut self, zone: &str, index: usize, ctx: &mut Ctx) -> Nav {
        match zone {
            z if z.starts_with("row-slide:") || z.starts_with("row-drop:") => {
                self.row = Some(index);
                let flow = self
                    .items(ctx)
                    .get(index)
                    .and_then(|i| i.slide_zone(z, ctx));
                flow.map_or(Nav::Stay, |f| Self::run(f, ctx))
            }
            // FilaDeCapas: as ações dentro da linha (0 = Aceitar, 1 = Escolher outra).
            z if z.starts_with("row-act:") => {
                self.row = Some(index);
                let item = self.items(ctx).into_iter().nth(index);
                let flow = item.and_then(|i| {
                    let f = if z.ends_with('1') { i.alt } else { i.accept };
                    f.map(|f| f(ctx))
                });
                flow.map_or(Nav::Stay, |f| Self::run(f, ctx))
            }
            // No desktop a categoria só troca as linhas; o foco não pula para elas (e sai da busca).
            "category" if ctx.desktop => {
                self.query.clear();
                self.hover(zone, index, ctx);
                Nav::Stay
            }
            _ => {
                self.hover(zone, index, ctx);
                self.handle(Cmd::Accept, ctx)
            }
        }
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
