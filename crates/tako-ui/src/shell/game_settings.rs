//! Configurações deste jogo (port de `GameSettingsViewModel.cs`, UX-007): painel à direita. Cada linha começa em
//! "Padrão: <valor do Tako>"; o que o jogo muda fica em negrito e é salvo na hora.

use super::Ctx;
use super::ctx::keys;
use super::keyboard::KeyboardLayer;
use super::layer::{Flow, Layer, MenuItem, MenuLayer};
use crate::model::PanelRow;
use std::path::Path;
use tako_core::devices::DisplayMode;
use tako_core::session::settings::{GameSettingsOverride as Override, normalize_arguments};

/// O que o jogo muda (lido do banco na primeira vez; no `--simulate`, só na memória).
pub fn current(ctx: &Ctx, id: &str) -> Override {
    if let Some(o) = ctx.game_overrides.get(id) {
        return o.clone();
    }
    ctx.game_settings
        .as_ref()
        .and_then(|s| {
            s.load(id)
                .map_err(|e| tracing::error!("configurações do jogo {id}: {e}"))
                .ok()
        })
        .unwrap_or_default()
}

fn change(ctx: &mut Ctx, id: &str, f: impl FnOnce(&mut Override)) {
    let mut o = current(ctx, id);
    f(&mut o);
    if let Some(s) = &ctx.game_settings
        && let Err(e) = s.save(id, &o)
    {
        tracing::error!("configurações do jogo {id}: {e}");
        ctx.toast(ctx.t("game_settings.save_failed"));
    }
    ctx.game_overrides.insert(id.to_owned(), o);
    ctx.dirty = true;
}

/// Próxima opção da lista (volta ao começo); a atual fora da lista conta como antes da primeira.
fn cycle<T: PartialEq + Clone>(options: &[T], cur: &T, dir: i32) -> T {
    let n = options.len() as i32;
    let i = options
        .iter()
        .position(|o| o == cur)
        .map_or(-1, |i| i as i32);
    options[(i + dir).rem_euclid(n) as usize].clone()
}

fn yes_no(ctx: &Ctx, v: bool) -> String {
    ctx.t(if v { "common.yes" } else { "common.no" })
}

fn on_off(ctx: &Ctx, v: bool) -> String {
    ctx.t(if v {
        "game_settings.on"
    } else {
        "game_settings.off"
    })
}

fn default_of(ctx: &Ctx, value: &str) -> String {
    ctx.tf("game_settings.default", &[("value", value)])
}

fn mode_label(key: &str) -> String {
    DisplayMode::parse(key).map_or_else(|| key.to_owned(), |m| m.to_string())
}

/// Linha que muda ao toque: em negrito quando o jogo muda o valor do Tako.
fn row(key: &str, title: String, value: String, overridden: bool) -> PanelRow {
    PanelRow::choice(key, title, value).emphasized(overridden)
}

/// A tela que o jogo usa: a dele (PERF-002) ou a tela de jogo do Tako (vazio sem nenhuma).
fn screen_of(ctx: &Ctx, o: &Override) -> String {
    normalize_arguments(o.screen_id.as_deref())
        .unwrap_or_else(|| ctx.settings.string(keys::GAME_SCREEN_ID))
}

/// Resoluções da tela do jogo (vazio sem tela escolhida).
fn modes(ctx: &Ctx, o: &Override) -> Vec<String> {
    let screen = screen_of(ctx, o);
    if screen.is_empty() {
        return Vec::new();
    }
    ctx.platform
        .displays
        .modes(&screen)
        .unwrap_or_default()
        .iter()
        .map(DisplayMode::key)
        .collect()
}

/// Saídas de áudio (id, nome); vazio quando o SO não deixa trocar.
fn outputs(ctx: &Ctx) -> Vec<(String, String)> {
    let audio = &ctx.platform.audio;
    if !audio.can_set_default() {
        return Vec::new();
    }
    audio
        .capture()
        .map(|s| s.devices.into_iter().map(|d| (d.id, d.name)).collect())
        .unwrap_or_default()
}

fn cycle_mode(ctx: &mut Ctx, id: &str, dir: i32) -> Flow {
    let c = current(ctx, id);
    let found = modes(ctx, &c);
    let mut options = vec![None, Some(Override::KEEP_CURRENT_MODE.to_owned())];
    options.extend(found.iter().cloned().map(Some));
    if let Some(saved) = c.screen_mode.clone().filter(|s| !s.is_empty())
        && !options.contains(&Some(saved.clone()))
    {
        options.push(Some(saved)); // resolução de uma tela desligada agora: continua escolhível
    }
    if found.is_empty() && c.screen_mode.is_none() {
        let no_screen = screen_of(ctx, &c).is_empty();
        ctx.toast(ctx.t(if no_screen {
            "game_settings.choose_screen_first"
        } else {
            "game_settings.enable_screen_first"
        }));
    }
    let next = cycle(&options, &c.screen_mode, dir);
    change(ctx, id, |o| o.screen_mode = next);
    Flow::Stay
}

/// Padrão (a tela de jogo do Tako) e as telas conectadas; a escolhida e desconectada continua na lista.
fn cycle_screen(ctx: &mut Ctx, id: &str, dir: i32) -> Flow {
    let c = current(ctx, id);
    let mut screens: Vec<(String, String)> = ctx
        .platform
        .displays
        .list()
        .unwrap_or_default()
        .into_iter()
        .map(|d| (d.id, d.name))
        .collect();
    if let Some(saved) = normalize_arguments(c.screen_id.as_deref())
        && !screens.iter().any(|(i, _)| i.eq_ignore_ascii_case(&saved))
    {
        let name = c.screen_name.clone().unwrap_or_else(|| saved.clone());
        screens.push((saved, name));
    }
    let mut options: Vec<Option<(String, String)>> = vec![None];
    options.extend(screens.into_iter().map(Some));
    let cur = options
        .iter()
        .find(|o| {
            o.as_ref().map(|(i, _)| i.to_lowercase())
                == normalize_arguments(c.screen_id.as_deref()).map(|i| i.to_lowercase())
        })
        .cloned()
        .unwrap_or(None);
    let next = cycle(&options, &cur, dir);
    change(ctx, id, |o| {
        o.screen_name = next.as_ref().map(|(_, n)| n.clone());
        o.screen_id = next.map(|(i, _)| i);
        // As resoluções são de cada tela: a escolhida para a outra não vale aqui.
        o.screen_mode = None;
    });
    Flow::Stay
}

fn cycle_audio(ctx: &mut Ctx, id: &str, dir: i32) -> Flow {
    let c = current(ctx, id);
    let mut outs = outputs(ctx);
    if let Some(saved) = c.audio_device_id.clone().filter(|s| !s.is_empty())
        && !outs.iter().any(|(i, _)| i.eq_ignore_ascii_case(&saved))
    {
        let name = c.audio_device_name.clone().unwrap_or_else(|| saved.clone());
        outs.push((saved, name)); // desconectada agora
    }
    let mut options: Vec<Option<(String, String)>> =
        vec![None, Some((Override::KEEP_AUDIO.to_owned(), String::new()))];
    options.extend(outs.into_iter().map(Some));
    let cur = options
        .iter()
        .find(|o| {
            o.as_ref().map(|(i, _)| i.to_lowercase())
                == c.audio_device_id.as_ref().map(|i| i.to_lowercase())
        })
        .cloned()
        .unwrap_or(None);
    let next = cycle(&options, &cur, dir);
    change(ctx, id, |o| {
        o.audio_device_name = next
            .as_ref()
            .map(|(_, n)| n.clone())
            .filter(|n| !n.is_empty());
        o.audio_device_id = next.map(|(i, _)| i);
    });
    Flow::Stay
}

fn edit_arguments(ctx: &Ctx, id: &str) -> Flow {
    let text = current(ctx, id).extra_arguments.unwrap_or_default();
    let id = id.to_owned();
    Flow::Push(Box::new(
        KeyboardLayer::new(
            ctx,
            ctx.t("game_settings.arguments"),
            &text,
            move |ctx, t| {
                let args = normalize_arguments(Some(t));
                change(ctx, &id, |o| o.extra_arguments = args);
                Flow::Close
            },
        )
        .hint(ctx.t("game_settings.arguments_hint")),
    ))
}

/// O usuário permitiu os comandos como estão (SESSION-004).
pub(crate) fn approve_commands(ctx: &mut Ctx, id: &str) {
    change(ctx, id, Override::approve_commands);
}

/// Texto livre de uma linha (variáveis e comandos): vazio volta ao padrão.
fn edit_text(
    ctx: &Ctx,
    id: &str,
    title: &str,
    hint: &str,
    get: fn(&Override) -> Option<String>,
    set: fn(&mut Override, Option<String>),
) -> Flow {
    let text = get(&current(ctx, id)).unwrap_or_default();
    let id = id.to_owned();
    Flow::Push(Box::new(
        KeyboardLayer::new(ctx, ctx.t(title), &text, move |ctx, t| {
            let value = normalize_arguments(Some(t));
            change(ctx, &id, |o| set(o, value));
            Flow::Close
        })
        .hint(ctx.t(hint)),
    ))
}

/// Pasta que não existe não é aceita; vazio = a pasta do executável.
pub fn folder_problem(ctx: &Ctx, folder: &str) -> Option<String> {
    let f = folder.trim();
    (!f.is_empty() && !Path::new(f).is_dir())
        .then(|| ctx.tf("game_settings.folder_missing", &[("folder", f)]))
}

fn edit_folder(ctx: &Ctx, id: &str) -> Flow {
    let text = current(ctx, id).working_directory.unwrap_or_default();
    let id = id.to_owned();
    Flow::Push(Box::new(
        KeyboardLayer::new(ctx, ctx.t("game_settings.folder"), &text, move |ctx, t| {
            let folder = (!t.is_empty()).then(|| t.to_owned());
            change(ctx, &id, |o| o.working_directory = folder);
            Flow::Close
        })
        .hint(ctx.t("game_settings.folder_hint"))
        .check(folder_problem),
    ))
}

pub fn panel(ctx: &Ctx, id: &str) -> Box<dyn Layer> {
    let title = ctx
        .library
        .find(id)
        .map(|g| g.title.clone())
        .unwrap_or_default();
    // No desktop (ConfiguracoesDoJogo): painel largo, com "Jogo · N mudanças" embaixo do título.
    let overline = if ctx.desktop {
        let n = current(ctx, id).changed_count();
        if n == 0 {
            title
        } else {
            format!(
                "{title} · {}",
                tako_core::i18n::tn("game_settings.changes", n as i64, &[])
            )
        }
    } else {
        title.to_uppercase()
    };
    let desktop = ctx.desktop;
    let id = id.to_owned();
    let layer = MenuLayer::panel(ctx.t("details.game_settings"), move |ctx| rows(ctx, &id))
        .overline(overline);
    Box::new(if desktop { layer.wide() } else { layer })
}

fn rows(ctx: &Ctx, id: &str) -> Vec<MenuItem> {
    let c = current(ctx, id);
    let s = &ctx.settings;
    let mut v = Vec::new();
    let i = || id.to_owned();

    v.push(MenuItem::new(PanelRow::header(
        ctx.t("game_settings.screen"),
    )));
    let screen_value = match c.use_game_screen {
        Some(b) => yes_no(ctx, b),
        // Com uma tela própria escolhida, o jogo usa a tela (ver `resolve`).
        None => default_of(
            ctx,
            &yes_no(
                ctx,
                normalize_arguments(c.screen_id.as_deref()).is_some()
                    || s.bool(keys::GAME_SCREEN_ENABLED, false),
            ),
        ),
    };
    let (a, b) = (i(), i());
    let toggle_screen = move |ctx: &mut Ctx, dir: i32, id: &str| {
        let next = cycle(
            &[None, Some(true), Some(false)],
            &current(ctx, id).use_game_screen,
            dir,
        );
        change(ctx, id, |o| o.use_game_screen = next);
        Flow::Stay
    };
    v.push(
        MenuItem::new(
            row(
                "screen",
                ctx.t("game_settings.use_screen"),
                screen_value,
                c.use_game_screen.is_some(),
            )
            .desc(ctx.t("game_settings.screen_note")),
        )
        .on(move |ctx| toggle_screen(ctx, 1, &a))
        .on_adjust(move |ctx, d| toggle_screen(ctx, d, &b)),
    );

    let screen_value = match normalize_arguments(c.screen_id.as_deref()) {
        Some(sid) => {
            let connected = ctx.platform.displays.list().unwrap_or_default();
            match connected.iter().find(|d| d.id.eq_ignore_ascii_case(&sid)) {
                Some(d) => d.name.clone(),
                None => ctx.tf(
                    "game_settings.disconnected",
                    &[("screen", c.screen_name.as_deref().unwrap_or(&sid))],
                ),
            }
        }
        None => {
            let name = s.string(keys::GAME_SCREEN_NAME);
            default_of(
                ctx,
                &if name.is_empty() {
                    ctx.t("game_settings.no_screen")
                } else {
                    name
                },
            )
        }
    };
    let (a, b) = (i(), i());
    v.push(
        MenuItem::new(
            row(
                "which_screen",
                ctx.t("game_settings.which_screen"),
                screen_value,
                normalize_arguments(c.screen_id.as_deref()).is_some(),
            )
            .desc(ctx.t("game_settings.which_screen_desc")),
        )
        .on(move |ctx| cycle_screen(ctx, &a, 1))
        .on_adjust(move |ctx, d| cycle_screen(ctx, &b, d)),
    );

    // Em outra tela, o padrão é o preset dela (a resolução do Tako é da tela de jogo dele).
    let (default_mode, default_hdr) =
        crate::session::global_settings(ctx).screen_defaults(&screen_of(ctx, &c));
    let global_mode = default_mode.map(|m| m.key()).unwrap_or_default();
    let mode_value = match c.screen_mode.as_deref() {
        None => default_of(
            ctx,
            &if global_mode.is_empty() {
                ctx.t("game_settings.keep_current_lower")
            } else {
                mode_label(&global_mode)
            },
        ),
        Some(Override::KEEP_CURRENT_MODE) => ctx.t("game_settings.keep_current"),
        Some(k) => mode_label(k),
    };
    let (a, b) = (i(), i());
    v.push(
        MenuItem::new(row(
            "mode",
            ctx.t("game_settings.mode"),
            mode_value,
            c.screen_mode.is_some(),
        ))
        .on(move |ctx| cycle_mode(ctx, &a, 1))
        .on_adjust(move |ctx, d| cycle_mode(ctx, &b, d)),
    );

    let hdr_value = match c.hdr {
        Some(h) => on_off(ctx, h),
        None => default_of(ctx, &on_off(ctx, default_hdr)),
    };
    let (a, b) = (i(), i());
    let toggle_hdr = move |ctx: &mut Ctx, dir: i32, id: &str| {
        let next = cycle(&[None, Some(true), Some(false)], &current(ctx, id).hdr, dir);
        change(ctx, id, |o| o.hdr = next);
        Flow::Stay
    };
    v.push(
        MenuItem::new(row(
            "hdr",
            ctx.t("game_settings.hdr"),
            hdr_value,
            c.hdr.is_some(),
        ))
        .on(move |ctx| toggle_hdr(ctx, 1, &a))
        .on_adjust(move |ctx, d| toggle_hdr(ctx, d, &b)),
    );

    v.push(MenuItem::new(PanelRow::header(
        ctx.t("game_settings.sound"),
    )));
    let audio_value = match c.audio_device_id.as_deref() {
        None => {
            let name = s.string(keys::GAME_AUDIO_NAME);
            default_of(
                ctx,
                &if name.is_empty() {
                    ctx.t("game_settings.keep_audio_lower")
                } else {
                    name
                },
            )
        }
        Some(Override::KEEP_AUDIO) => ctx.t("game_settings.keep_audio"),
        Some(id) => c.audio_device_name.clone().unwrap_or_else(|| id.to_owned()),
    };
    let (a, b) = (i(), i());
    v.push(
        MenuItem::new(row(
            "audio",
            ctx.t("game_settings.audio"),
            audio_value,
            c.audio_device_id.is_some(),
        ))
        .on(move |ctx| cycle_audio(ctx, &a, 1))
        .on_adjust(move |ctx, d| cycle_audio(ctx, &b, d)),
    );

    v.push(MenuItem::new(PanelRow::header(
        ctx.t("game_settings.launch"),
    )));
    let args = normalize_arguments(c.extra_arguments.as_deref());
    let a = i();
    v.push(
        MenuItem::new(
            row(
                "args",
                ctx.t("game_settings.arguments"),
                args.clone().unwrap_or_else(|| ctx.t("game_settings.none")),
                args.is_some(),
            )
            .desc(ctx.t("game_settings.launch_note")),
        )
        .on(move |ctx| edit_arguments(ctx, &a)),
    );
    let folder = c.working_directory.clone().filter(|f| !f.trim().is_empty());
    let a = i();
    v.push(
        MenuItem::new(row(
            "folder",
            ctx.t("game_settings.folder"),
            folder
                .clone()
                .unwrap_or_else(|| ctx.t("game_settings.exe_folder")),
            folder.is_some(),
        ))
        .on(move |ctx| edit_folder(ctx, &a)),
    );
    let env = tako_core::session::settings::parse_environment(c.environment.as_deref());
    let a = i();
    v.push(
        MenuItem::new(row(
            "env",
            ctx.t("game_settings.environment"),
            if env.is_empty() {
                ctx.t("game_settings.none")
            } else {
                tako_core::i18n::tn("game_settings.environment_count", env.len() as i64, &[])
            },
            !env.is_empty(),
        ))
        .on(move |ctx| {
            edit_text(
                ctx,
                &a,
                "game_settings.environment",
                "game_settings.environment_hint",
                |o| o.environment.clone(),
                |o, v| o.environment = v,
            )
        }),
    );
    for (key, text, hint, get, set) in [
        (
            "before",
            "game_settings.before_command",
            "game_settings.commands_hint",
            (|o: &Override| o.before_command.clone()) as fn(&Override) -> Option<String>,
            (|o: &mut Override, v| o.before_command = v) as fn(&mut Override, Option<String>),
        ),
        (
            "after",
            "game_settings.after_command",
            "game_settings.commands_hint",
            |o: &Override| o.after_command.clone(),
            |o: &mut Override, v| o.after_command = v,
        ),
    ] {
        let value = normalize_arguments(get(&c).as_deref());
        let a = i();
        v.push(
            MenuItem::new(row(
                key,
                ctx.t(text),
                value.clone().unwrap_or_else(|| ctx.t("game_settings.none")),
                value.is_some(),
            ))
            .on(move |ctx| edit_text(ctx, &a, text, hint, get, set)),
        );
    }

    v.push(MenuItem::new(PanelRow::header(
        ctx.t("game_settings.defaults"),
    )));
    let n = c.changed_count();
    let changes = if n == 0 {
        String::new()
    } else {
        tako_core::i18n::tn("game_settings.changes", n as i64, &[])
    };
    let a = i();
    v.push(
        MenuItem::new(
            PanelRow::action("reset", ctx.t("game_settings.reset"))
                .value(changes)
                .enabled(n > 0),
        )
        .on(move |ctx| {
            change(ctx, &a, |o| *o = Override::default());
            Flow::Stay
        }),
    );
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Overlay;
    use crate::shell::Cmd;
    use tako_core::paths::DataPaths;
    use tako_core::session::settings::{MonitorPreset, MonitorPresets};
    use tako_platform::Platform;

    fn rows_of(o: &Overlay) -> Vec<(String, String, bool)> {
        o.rows
            .iter()
            .filter(|r| r.kind == "choice")
            .map(|r| (r.key.clone(), r.value.clone(), r.emphasized))
            .collect()
    }

    fn value(o: &Overlay, key: &str) -> (String, bool) {
        rows_of(o)
            .into_iter()
            .find(|r| r.0 == key)
            .map(|r| (r.1, r.2))
            .unwrap()
    }

    #[test]
    fn rows_start_at_the_tako_default_and_overrides_are_bold_and_saved() {
        let dir = tempfile::tempdir().unwrap();
        // Banco real: as mudanças precisam sobreviver a um novo contexto.
        let (_l, mut ctx) = crate::metadata::test_ctx(dir.path(), false);
        ctx.library = tako_core::library::GameLibrary::in_memory(
            crate::demo::games(),
            tako_core::library::system_clock(),
        );
        ctx.settings.set_bool(keys::GAME_SCREEN_ENABLED, true);
        let id = ctx.library.games()[0].id.clone();
        let mut layer = panel(&ctx, &id);

        let o = layer.render(&ctx);
        assert_eq!(value(&o, "screen"), ("Padrão: Sim".into(), false));
        assert_eq!(value(&o, "hdr"), ("Padrão: Desligado".into(), false));
        assert_eq!(value(&o, "args").0, "Nenhum");

        // Foco começa em "Usar a tela de jogo": A troca para "Sim" (negrito), de novo para "Não".
        layer.handle(Cmd::Accept, &mut ctx);
        assert_eq!(value(&layer.render(&ctx), "screen"), ("Sim".into(), true));
        layer.handle(Cmd::Accept, &mut ctx);
        assert_eq!(value(&layer.render(&ctx), "screen"), ("Não".into(), true));
        // HDR por ←/→ (depois de "Tela deste jogo" e da resolução).
        layer.handle(Cmd::Down, &mut ctx);
        layer.handle(Cmd::Down, &mut ctx);
        layer.handle(Cmd::Down, &mut ctx);
        layer.handle(Cmd::Right, &mut ctx);
        assert_eq!(value(&layer.render(&ctx), "hdr"), ("Ligado".into(), true));

        // Gravado: um contexto novo lê do banco.
        let fresh = crate::app::build_ctx(DataPaths::at(dir.path()), Platform::simulated(), false);
        let saved = current(&fresh, &id);
        assert_eq!(saved.use_game_screen, Some(false));
        assert_eq!(saved.hdr, Some(true));
        assert_eq!(saved.changed_count(), 2);

        // "Usar tudo do Tako" volta ao padrão.
        let reset = layer
            .render(&ctx)
            .rows
            .iter()
            .position(|r| r.key == "reset")
            .unwrap();
        layer.hover("row", reset);
        layer.handle(Cmd::Accept, &mut ctx);
        assert!(current(&ctx, &id).is_default());
        assert_eq!(
            value(&layer.render(&ctx), "screen"),
            ("Padrão: Sim".into(), false)
        );
    }

    #[test]
    fn simulated_overrides_stay_in_memory_and_modes_cycle() {
        let dir = tempfile::tempdir().unwrap();
        let (_l, mut ctx) = crate::metadata::test_ctx(dir.path(), true);
        let id = ctx.library.games()[0].id.clone();
        let screen = ctx.platform.displays.list().unwrap()[0].id.clone();
        ctx.settings.set(keys::GAME_SCREEN_ID, &screen);
        let mut layer = panel(&ctx, &id);
        let mode = layer
            .render(&ctx)
            .rows
            .iter()
            .position(|r| r.key == "mode")
            .unwrap();
        layer.hover("row", mode);
        layer.handle(Cmd::Accept, &mut ctx);
        assert_eq!(
            value(&layer.render(&ctx), "mode"),
            ("Manter a atual".into(), true)
        );
        layer.handle(Cmd::Accept, &mut ctx);
        let first = ctx.platform.displays.modes(&screen).unwrap()[0];
        assert_eq!(value(&layer.render(&ctx), "mode").0, first.to_string());
        assert_eq!(current(&ctx, &id).screen_mode, Some(first.key()));
        // Voltar: ← duas vezes chega ao "Padrão".
        layer.handle(Cmd::Left, &mut ctx);
        layer.handle(Cmd::Left, &mut ctx);
        assert!(current(&ctx, &id).screen_mode.is_none());
    }

    #[test]
    fn game_picks_its_own_screen_and_the_modes_follow_it() {
        let dir = tempfile::tempdir().unwrap();
        let (_l, mut ctx) = crate::metadata::test_ctx(dir.path(), true);
        let id = ctx.library.games()[0].id.clone();
        let screens = ctx.platform.displays.list().unwrap();
        // Tela de jogo do Tako: a segunda, com uma resolução própria.
        ctx.settings.set(keys::GAME_SCREEN_ID, &screens[1].id);
        ctx.settings.set(keys::GAME_SCREEN_NAME, &screens[1].name);
        ctx.settings.set(keys::GAME_SCREEN_MODE, "1280x720@60");
        let mut layer = panel(&ctx, &id);
        let o = layer.render(&ctx);
        assert_eq!(
            value(&o, "which_screen"),
            (format!("Padrão: {}", screens[1].name), false)
        );
        assert_eq!(value(&o, "mode").0, "Padrão: 1280 × 720 · 60 Hz");
        let at = o.rows.iter().position(|r| r.key == "which_screen").unwrap();
        layer.hover("row", at);
        layer.handle(Cmd::Accept, &mut ctx);
        let o = layer.render(&ctx);
        assert_eq!(value(&o, "which_screen"), (screens[0].name.clone(), true));
        // Na outra tela, a resolução do Tako não vale: mantém a atual.
        assert_eq!(value(&o, "mode").0, "Padrão: manter a atual");
        assert_eq!(value(&o, "screen").0, "Padrão: Sim");
        // Com um preset do monitor (Configurações › Tela de jogo), ele passa a ser o padrão do jogo.
        let full_hd = DisplayMode::new(1920, 1080, 60);
        let mut presets = MonitorPresets::default();
        let preset = MonitorPreset {
            mode: Some(full_hd),
            hdr: true,
        };
        presets.set(&screens[0].id, preset);
        ctx.settings
            .set(keys::GAME_SCREEN_PRESETS, &presets.to_text());
        let o = layer.render(&ctx);
        assert_eq!(
            value(&o, "mode"),
            ("Padrão: 1920 × 1080 · 60 Hz".into(), false)
        );
        assert_eq!(value(&o, "hdr"), ("Padrão: Ligado".into(), false));
        // O que a sessão usa (no --simulate as escolhas do jogo ficam só na memória).
        let global = crate::session::global_settings(&ctx);
        let s = tako_core::session::settings::resolve(&global, Some(&current(&ctx, &id)));
        let s = s.game_screen.unwrap();
        assert_eq!((s.mode, s.enable_hdr), (Some(full_hd), true));
        ctx.settings.set(keys::GAME_SCREEN_PRESETS, "");
        let saved = current(&ctx, &id);
        assert_eq!(saved.screen_id.as_deref(), Some(screens[0].id.as_str()));
        assert_eq!(
            modes(&ctx, &saved),
            ctx.platform
                .displays
                .modes(&screens[0].id)
                .unwrap()
                .iter()
                .map(DisplayMode::key)
                .collect::<Vec<_>>()
        );
        // Trocar de tela esquece a resolução escolhida para a anterior.
        change(&mut ctx, &id, |o| {
            o.screen_mode = Some("1920x1080@60".into())
        });
        layer.handle(Cmd::Left, &mut ctx);
        let back = current(&ctx, &id);
        assert!(back.is_default(), "{back:?}");
    }

    #[test]
    fn working_folder_must_exist() {
        let dir = tempfile::tempdir().unwrap();
        let (_l, ctx) = crate::metadata::test_ctx(dir.path(), true);
        assert!(folder_problem(&ctx, "").is_none());
        assert!(folder_problem(&ctx, &dir.path().to_string_lossy()).is_none());
        assert!(folder_problem(&ctx, "/nao/existe/mesmo").is_some());
    }
}
