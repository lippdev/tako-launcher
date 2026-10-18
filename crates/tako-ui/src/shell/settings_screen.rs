//! Configurações › Tela e áudio (no C# ficava no menu do Console Mode): tela de jogo, resolução, HDR, VRR, saída
//! de áudio e o que muda durante o jogo. Toda chamada ao SO pode falhar: o erro vira texto na linha.

use super::{Ctx, Effect, Flow, MenuItem, cached, toggle};
use crate::model::PanelRow;
use crate::shell::ctx::keys;
use crate::shell::layer::Pick;
use std::time::Duration;
use tako_core::devices::{AudioDevice, ConnectedDisplay, DisplayMode, HdrStatus, VrrStatus};
use tako_core::session::settings::{MonitorPreset, MonitorPresets};

/// Ler as telas pode rodar programas do SO (xrandr, kscreen): no máximo a cada 3 s.
const TTL: Duration = Duration::from_secs(3);

pub const HIDE_DISCONNECT: &str = "Disconnect";
pub const HIDE_NONE: &str = "None";

fn displays(ctx: &Ctx) -> Result<Vec<ConnectedDisplay>, String> {
    let d = ctx.platform.displays.clone();
    cached("settings.displays", TTL, move || {
        d.list().map_err(|e| e.to_string())
    })
}

fn modes(ctx: &Ctx, id: &str) -> Result<Vec<DisplayMode>, String> {
    let read = || {
        let m = ctx.platform.displays.modes(id);
        (
            id.to_owned(),
            m.map(DisplayMode::normalize).map_err(|e| e.to_string()),
        )
    };
    // Guarda os modos de uma tela só: outra tela lê de novo.
    let (seen, modes) = cached("settings.modes", TTL, read);
    if seen == id {
        return modes;
    }
    super::forget("settings.modes");
    cached("settings.modes", TTL, read).1
}

fn audio(ctx: &Ctx) -> Result<(bool, Vec<AudioDevice>), String> {
    let a = ctx.platform.audio.clone();
    cached("settings.audio", TTL, move || {
        a.capture()
            .map(|s| (a.can_set_default(), s.devices))
            .map_err(|e| e.to_string())
    })
}

/// Presets de cada monitor (PERF-002).
pub(crate) fn presets(ctx: &Ctx) -> MonitorPresets {
    MonitorPresets::parse(&ctx.settings.string(keys::GAME_SCREEN_PRESETS))
}

/// Guarda a resolução, o HDR e o VRR da tela de jogo como o preset dela, para voltarem quando ela for escolhida de novo.
fn remember_preset(ctx: &mut Ctx) {
    let id = ctx.settings.string(keys::GAME_SCREEN_ID);
    if id.trim().is_empty() {
        return;
    }
    let mut all = presets(ctx);
    all.set(
        &id,
        MonitorPreset {
            mode: DisplayMode::parse(&ctx.settings.string(keys::GAME_SCREEN_MODE)),
            hdr: ctx.settings.bool(keys::GAME_SCREEN_HDR, false),
            vrr: ctx.settings.bool(keys::GAME_SCREEN_VRR, false),
        },
    );
    ctx.settings.set(keys::GAME_SCREEN_PRESETS, &all.to_text());
}

fn pick_display(ctx: &mut Ctx, d: &ConnectedDisplay) {
    remember_preset(ctx);
    ctx.settings.set(keys::GAME_SCREEN_ID, &d.id);
    ctx.settings.set(keys::GAME_SCREEN_NAME, &d.name);
    // Volta o que esta tela tinha; sem preset, o padrão (manter a atual, sem HDR nem VRR).
    let p = presets(ctx).get(&d.id).unwrap_or_default();
    let mode = p.mode.map(|m| m.key()).unwrap_or_default();
    ctx.settings.set(keys::GAME_SCREEN_MODE, &mode);
    ctx.settings.set_bool(keys::GAME_SCREEN_HDR, p.hdr);
    ctx.settings.set_bool(keys::GAME_SCREEN_VRR, p.vrr);
}

fn pick_mode(ctx: &mut Ctx, m: Option<DisplayMode>) {
    let key = m.map(|m| m.key()).unwrap_or_default();
    ctx.settings.set(keys::GAME_SCREEN_MODE, &key);
    remember_preset(ctx);
}

fn pick_audio(ctx: &mut Ctx, d: Option<&AudioDevice>) {
    let (id, name) = d
        .map(|d| (d.id.clone(), d.name.clone()))
        .unwrap_or_default();
    ctx.settings.set(keys::GAME_AUDIO_ID, &id);
    ctx.settings.set(keys::GAME_AUDIO_NAME, &name);
}

/// "Manter a atual" e depois os modos da tela de jogo, do maior para o menor.
fn mode_list(ctx: &Ctx) -> Result<Vec<Option<DisplayMode>>, String> {
    let id = ctx.settings.string(keys::GAME_SCREEN_ID);
    if id.is_empty() {
        return Ok(vec![None]);
    }
    let mut all: Vec<Option<DisplayMode>> = vec![None];
    all.extend(modes(ctx, &id)?.into_iter().map(Some));
    Ok(all)
}

/// Próximo da lista (com volta), a partir do atual.
fn cycle<T>(items: &[T], current: Option<usize>, dir: i32) -> Option<&T> {
    if items.is_empty() {
        return None;
    }
    let n = items.len() as i32;
    let i = match current {
        Some(i) => (i as i32 + dir).rem_euclid(n),
        None if dir < 0 => n - 1,
        None => 0,
    };
    items.get(i as usize)
}

pub fn rows(ctx: &Ctx) -> Vec<MenuItem> {
    let managed = ctx.platform.displays.is_managed_by_session();
    let screen_on = ctx.settings.bool(keys::GAME_SCREEN_ENABLED, false);
    let id = ctx.settings.string(keys::GAME_SCREEN_ID);
    let list = displays(ctx);
    let mut v = Vec::new();
    if managed {
        v.push(MenuItem::new(
            PanelRow::info(
                "managed",
                ctx.t("settings.screen.managed"),
                ctx.t("settings.screen.managed_value"),
            )
            .desc(ctx.t("settings.screen.managed_desc")),
        ));
    }
    let usable = !managed;
    v.push(disable(
        toggle(
            ctx,
            "settings.screen.enabled",
            keys::GAME_SCREEN_ENABLED,
            false,
        ),
        usable,
    ));

    let name = ctx.settings.string(keys::GAME_SCREEN_NAME);
    let mut display_row = PanelRow::choice(
        "display",
        ctx.t("settings.screen.display"),
        if name.is_empty() {
            ctx.t("settings.screen.choose")
        } else {
            name
        },
    )
    // No desktop é um select: sem o "← → trocam" da tela cheia.
    .desc(ctx.t(if ctx.desktop {
        "settings.screen.display_desc_desktop"
    } else {
        "settings.screen.display_desc"
    }));
    if let Err(e) = &list {
        display_row = display_row.desc(e.clone());
    }
    v.push(disable(
        MenuItem::new(display_row)
            .on_adjust(|ctx, dir| {
                match displays(ctx) {
                    Err(e) => ctx.toast(e),
                    Ok(list) => {
                        let id = ctx.settings.string(keys::GAME_SCREEN_ID);
                        let cur = list.iter().position(|d| d.id.eq_ignore_ascii_case(&id));
                        match cycle(&list, cur, dir) {
                            None => ctx.toast(ctx.t("settings.screen.no_displays")),
                            Some(d) => pick_display(ctx, d),
                        }
                    }
                }
                Flow::Stay
            })
            .options(|ctx| {
                let list = displays(ctx).unwrap_or_default();
                let id = ctx.settings.string(keys::GAME_SCREEN_ID);
                let cur = list.iter().position(|d| d.id.eq_ignore_ascii_case(&id));
                let picks = list
                    .into_iter()
                    .map(|d| {
                        Pick::new(d.name.clone()).set(move |ctx| {
                            let before = crate::screen_test::snapshot(ctx);
                            pick_display(ctx, &d);
                            crate::screen_test::confirm(ctx, before);
                            Flow::Stay
                        })
                    })
                    .collect();
                (picks, cur.unwrap_or(usize::MAX))
            }),
        usable,
    ));

    let hide = ctx.settings.get(keys::GAME_SCREEN_HIDE_OTHERS) != Some(HIDE_NONE);
    v.push(disable(
        MenuItem::new(
            PanelRow::toggle("hide", ctx.t("settings.screen.hide"), hide)
                .desc(ctx.t("settings.screen.hide_desc")),
        )
        .on(|ctx| {
            let hide = ctx.settings.get(keys::GAME_SCREEN_HIDE_OTHERS) != Some(HIDE_NONE);
            let next = if hide { HIDE_NONE } else { HIDE_DISCONNECT };
            ctx.settings.set(keys::GAME_SCREEN_HIDE_OTHERS, next);
            Flow::Stay
        }),
        usable,
    ));

    let mode = DisplayMode::parse(&ctx.settings.string(keys::GAME_SCREEN_MODE));
    let mut mode_row = PanelRow::choice(
        "mode",
        ctx.t("settings.screen.mode"),
        mode.map(|m| m.to_string())
            .unwrap_or_else(|| ctx.t("settings.screen.keep_mode")),
    )
    .desc(ctx.t("settings.screen.mode_desc"));
    if !id.is_empty()
        && let Err(e) = modes(ctx, &id)
    {
        mode_row = mode_row.desc(e);
    }
    v.push(disable(
        MenuItem::new(mode_row)
            .on_adjust(|ctx, dir| {
                let id = ctx.settings.string(keys::GAME_SCREEN_ID);
                if id.is_empty() {
                    ctx.toast(ctx.t("settings.screen.choose_first"));
                    return Flow::Stay;
                }
                match mode_list(ctx) {
                    Err(e) => ctx.toast(e),
                    Ok(all) => {
                        let cur = DisplayMode::parse(&ctx.settings.string(keys::GAME_SCREEN_MODE));
                        let i = all.iter().position(|m| *m == cur);
                        if let Some(m) = cycle(&all, i, dir) {
                            pick_mode(ctx, *m);
                        }
                    }
                }
                Flow::Stay
            })
            .options(|ctx| {
                let all = mode_list(ctx).unwrap_or_else(|_| vec![None]);
                let now = DisplayMode::parse(&ctx.settings.string(keys::GAME_SCREEN_MODE));
                let cur = all.iter().position(|m| *m == now).unwrap_or(0);
                let picks = all
                    .into_iter()
                    .map(|m| {
                        let label =
                            m.map_or_else(|| ctx.t("settings.screen.keep_mode"), |m| m.to_string());
                        Pick::new(label).set(move |ctx| {
                            let before = crate::screen_test::snapshot(ctx);
                            pick_mode(ctx, m);
                            crate::screen_test::confirm(ctx, before);
                            Flow::Stay
                        })
                    })
                    .collect();
                (picks, cur)
            }),
        usable,
    ));

    let hdr = list
        .as_ref()
        .ok()
        .and_then(|l| l.iter().find(|d| d.id.eq_ignore_ascii_case(&id)))
        .is_some_and(|d| d.hdr != HdrStatus::Unsupported);
    if hdr {
        let item = toggle(ctx, "settings.screen.hdr", keys::GAME_SCREEN_HDR, false).on(|ctx| {
            ctx.settings.toggle(keys::GAME_SCREEN_HDR, false);
            remember_preset(ctx);
            Flow::Stay
        });
        v.push(disable(item, usable));
    }
    // VRR aparece mesmo sem suporte, desligado e com o motivo (no Windows, falta API pública).
    let vrr = list
        .as_ref()
        .ok()
        .and_then(|l| l.iter().find(|d| d.id.eq_ignore_ascii_case(&id)))
        .map(|d| d.vrr);
    if let Some(status) = vrr {
        let mut item = toggle(ctx, "vrr.toggle", keys::GAME_SCREEN_VRR, false).on(|ctx| {
            ctx.settings.toggle(keys::GAME_SCREEN_VRR, false);
            remember_preset(ctx);
            Flow::Stay
        });
        let supported = status != VrrStatus::Unsupported;
        if !supported {
            let why = ctx.platform.displays.vrr_limitation();
            item.row = item
                .row
                .desc(why.unwrap_or_else(|| ctx.t("vrr.unsupported")));
        }
        v.push(disable(item, usable && supported));
    }
    v.push(disable(
        MenuItem::new(super::action(ctx, "settings.screen.test")).on(|ctx| {
            if ctx.settings.string(keys::GAME_SCREEN_ID).is_empty() {
                ctx.toast(ctx.t("settings.screen.choose_first"));
            } else {
                ctx.effect(Effect::TestGameScreen);
            }
            Flow::Stay
        }),
        usable && screen_on,
    ));

    v.push(MenuItem::new(PanelRow::header(
        ctx.t("settings.screen.audio_header"),
    )));
    let audio_name = ctx.settings.string(keys::GAME_AUDIO_NAME);
    let mut audio_row = PanelRow::choice(
        "audio",
        ctx.t("settings.screen.audio"),
        if ctx.settings.string(keys::GAME_AUDIO_ID).is_empty() {
            ctx.t("settings.screen.audio_keep")
        } else {
            audio_name
        },
    )
    .desc(ctx.t("settings.screen.audio_desc"));
    match audio(ctx) {
        Err(e) => audio_row = audio_row.desc(e).enabled(false),
        Ok((false, _)) => {
            audio_row = audio_row
                .desc(ctx.t("settings.screen.audio_locked"))
                .enabled(false)
        }
        Ok(_) => {}
    }
    v.push(
        MenuItem::new(audio_row)
            .on_adjust(|ctx, dir| {
                if let Ok((_, devices)) = audio(ctx) {
                    let mut all: Vec<Option<AudioDevice>> = vec![None];
                    all.extend(devices.into_iter().map(Some));
                    let id = ctx.settings.string(keys::GAME_AUDIO_ID);
                    let i = all
                        .iter()
                        .position(|d| d.as_ref().map(|d| d.id.as_str()).unwrap_or("") == id);
                    if let Some(d) = cycle(&all, i, dir) {
                        pick_audio(ctx, d.as_ref());
                    }
                }
                Flow::Stay
            })
            .options(|ctx| {
                let mut all: Vec<Option<AudioDevice>> = vec![None];
                all.extend(
                    audio(ctx)
                        .map(|(_, d)| d)
                        .unwrap_or_default()
                        .into_iter()
                        .map(Some),
                );
                let id = ctx.settings.string(keys::GAME_AUDIO_ID);
                let cur = all
                    .iter()
                    .position(|d| d.as_ref().map(|d| d.id.as_str()).unwrap_or("") == id)
                    .unwrap_or(0);
                let picks = all
                    .into_iter()
                    .map(|d| {
                        let label = d.as_ref().map_or_else(
                            || ctx.t("settings.screen.audio_keep"),
                            |d| d.name.clone(),
                        );
                        Pick::new(label).set(move |ctx| {
                            pick_audio(ctx, d.as_ref());
                            Flow::Stay
                        })
                    })
                    .collect();
                (picks, cur)
            }),
    );

    v.push(MenuItem::new(PanelRow::header(
        ctx.t("settings.screen.during_header"),
    )));
    v.push(toggle(
        ctx,
        "settings.screen.restore",
        keys::RESTORE_ENVIRONMENT,
        true,
    ));
    let mut cursor = toggle(ctx, "settings.screen.cursor", keys::HIDE_CURSOR, false);
    if let Some(why) = ctx.platform.cursor.limitation() {
        cursor.row.description = why;
    }
    v.push(cursor);
    let mut mouse = toggle(ctx, "settings.screen.mouse", keys::CONTROLLER_MOUSE, false);
    if let Err(e) = ctx.platform.pointer.check_access() {
        mouse.row.description = e.to_string();
    }
    v.push(mouse);
    v
}

/// Com o gamescope (ou sem algo que a linha precisa), a linha aparece mas não muda nada.
fn disable(mut item: MenuItem, usable: bool) -> MenuItem {
    if !usable {
        item.row.enabled = false;
    }
    item
}
