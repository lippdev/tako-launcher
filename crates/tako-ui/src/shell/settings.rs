//! Configurações (ADR-0020): coluna de categorias à esquerda e as linhas da categoria à direita. LB/RB trocam
//! de categoria; → ou A entram nas linhas; B nas linhas volta à categoria e B na coluna sai.

#[path = "settings_controls.rs"]
pub(crate) mod controls;
#[path = "settings_emulators.rs"]
pub(crate) mod emulators;
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
            id: "integrations",
            icon: "tv",
            rows: sunshine::rows,
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

/// Modo Xbox do Windows 11 pelo AnyFSE (PLAT-007): diz se está instalado e ensina a pôr o Tako como app inicial.
#[cfg(windows)]
fn xbox_mode_row(ctx: &Ctx) -> MenuItem {
    let installed = tako_platform::windows::anyfse::installed();
    MenuItem::new(
        PanelRow::action("xbox-mode", ctx.t("settings.general.xbox_mode"))
            .desc(ctx.t("settings.general.xbox_mode_desc"))
            .value(ctx.t(if installed {
                "settings.general.anyfse_installed"
            } else {
                "settings.general.anyfse_missing"
            })),
    )
    .on(|ctx| Flow::Push(Box::new(xbox_mode_panel(ctx))))
}

#[cfg(windows)]
fn xbox_mode_panel(ctx: &Ctx) -> MenuLayer {
    let exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    MenuLayer::panel(ctx.t("settings.general.xbox_mode"), |ctx| {
        vec![
            MenuItem::new(
                PanelRow::action("page", ctx.t("settings.general.anyfse_page"))
                    .value("github.com/ashpynov/AnyFSE"),
            )
            .on(effect(Effect::OpenUrl(
                tako_platform::windows::anyfse::PAGE.into(),
            ))),
        ]
    })
    .text(ctx.tf("settings.general.xbox_mode_steps", &[("exe", &exe)]))
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
    ]);
    // Antes de "Abrir o Tako ao entrar no sistema" (só no Windows).
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
                .value(ctx.t("desktop.mode.go_value")),
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

/// Temas instalados (lidos do disco no máximo a cada 30 s; instalar ou atualizar esquece a leitura).
pub(crate) const THEMES_CACHE: &str = "themes";

fn installed_themes(ctx: &Ctx) -> Vec<tako_themes::InstalledTheme> {
    let root = ctx.paths.themes();
    cached(THEMES_CACHE, Duration::from_secs(30), || {
        tako_themes::InstalledTheme::scan(&root)
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
        let installed = installed_themes(ctx);
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
        if !sources.is_empty() {
            v.push(
                MenuItem::new(
                    PanelRow::action("update-themes", ctx.t("settings.appearance.update_themes"))
                        .desc(ctx.t("settings.appearance.update_themes_desc"))
                        .value(ctx.t("settings.appearance.update_themes_value")),
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
    if !ctx.desktop {
        if tako_themes::selected(&ctx.paths.themes()).as_deref() == id {
            return Flow::Stay;
        }
        return super::theme_preview::start(ctx, id, name);
    }
    match tako_themes::select(&ctx.paths.themes(), id) {
        Ok(()) => ctx.effect(Effect::ApplyTheme(id.unwrap_or(BUILTIN_THEME).to_owned())),
        Err(e) => ctx.toast(e.to_string()),
    }
    Flow::Stay
}

fn appearance(ctx: &Ctx) -> Vec<MenuItem> {
    vec![
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
            PanelRow {
                // Volume é contínuo: no desktop, controle deslizante (o valor vai em 0–100).
                kind: if ctx.desktop { "slider" } else { "choice" }.into(),
                ..PanelRow::choice(
                    "navigation-volume",
                    ctx.t("accessibility.sound_volume"),
                    if ctx.desktop {
                        // O controle mostra "70%"; em 0, "Mudo" (o valor que ele lê vira 0).
                        match crate::navigation_feedback::volume(&ctx.settings) {
                            0 => ctx.t("accessibility.sound_muted"),
                            v => v.to_string(),
                        }
                    } else {
                        format!("{}%", crate::navigation_feedback::volume(&ctx.settings))
                    },
                )
            }
            .desc(ctx.t("accessibility.sound_desc")),
        )
        .on(|ctx| change_navigation_volume(ctx, 1))
        .on_adjust(change_navigation_volume)
        .on_slide(|ctx, v, done| {
            use crate::navigation_feedback as feedback;
            ctx.settings.set(feedback::SOUND_VOLUME, &v.to_string());
            ctx.dirty = true;
            // Ao soltar, um som de amostra no volume novo.
            if done {
                feedback::play(ctx, Cmd::Right);
            }
            Flow::Stay
        }),
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
    .into_iter()
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
        ctx.desktop
            .then(|| MenuItem::new(PanelRow::header(ctx.t("settings.appearance.layout_header")))),
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
    /// Busca de configurações do desktop: as linhas de todas as categorias que combinam com o texto.
    query: String,
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
