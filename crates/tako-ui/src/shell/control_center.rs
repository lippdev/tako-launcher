//! Central rápida durante o jogo (DESIGN.md › Menu rápido), reduzida às funções que ficam no Tako (PRD §6):
//! Início (sem fechar o jogo), Voltar ao jogo, Áudio, Tela, Ponteiro e Encerrar jogo.

use super::{Cmd, Ctx, Effect, Out};
use crate::model::{Hint, PanelRow};
use tako_core::devices::DisplayMode;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Icon {
    Home,
    Game,
    Audio,
    Screen,
    Pointer,
    End,
}

const ICONS: [Icon; 6] = [
    Icon::Home,
    Icon::Game,
    Icon::Audio,
    Icon::Screen,
    Icon::Pointer,
    Icon::End,
];

pub struct ControlCenter {
    icon: usize,
    /// Painel aberto e a linha em foco.
    panel: Option<usize>,
}

impl Default for ControlCenter {
    fn default() -> Self {
        Self::new()
    }
}

impl ControlCenter {
    pub fn new() -> Self {
        // O foco abre no jogo (o cartão maior no PS5; aqui, o ícone do jogo).
        ControlCenter {
            icon: 1,
            panel: None,
        }
    }

    fn rows(&self, ctx: &Ctx, icon: Icon) -> Vec<PanelRow> {
        match icon {
            Icon::Audio => {
                let Ok(state) = ctx.platform.audio.capture() else {
                    return vec![PanelRow::info("none", ctx.t("cc.audio_unavailable"), "")];
                };
                let default = state.default_id.clone().unwrap_or_default();
                let level = state.level_of(&default).cloned();
                let mut v = vec![
                    PanelRow::new("volume", "slider", ctx.t("cc.volume")).value(format!(
                        "{}",
                        level
                            .as_ref()
                            .map(|l| (l.volume * 100.0).round() as i32)
                            .unwrap_or(0)
                    )),
                    PanelRow::toggle("mute", ctx.t("cc.mute"), level.is_some_and(|l| l.muted)),
                    PanelRow::header(ctx.t("cc.output")),
                ];
                v.extend(
                    state
                        .devices
                        .iter()
                        .map(|d| PanelRow::radio(d.id.clone(), d.name.clone(), d.id == default)),
                );
                v
            }
            Icon::Screen => {
                let id = ctx.settings.string(super::ctx::keys::GAME_SCREEN_ID);
                let displays = ctx.platform.displays.list().unwrap_or_default();
                let target = displays
                    .iter()
                    .find(|d| d.id == id)
                    .or_else(|| displays.iter().find(|d| d.is_primary));
                let Some(target) = target else {
                    return vec![PanelRow::info("none", ctx.t("cc.screen_unavailable"), "")];
                };
                let modes = ctx.platform.displays.modes(&target.id).unwrap_or_default();
                let mut v = vec![PanelRow::header(target.name.clone())];
                v.extend(DisplayMode::normalize(modes).into_iter().map(|m| {
                    PanelRow::radio(m.key(), m.to_string(), target.current_mode == Some(m))
                }));
                v
            }
            Icon::Pointer => vec![
                PanelRow::toggle(
                    "hide",
                    ctx.t("cc.pointer_hide"),
                    ctx.platform.cursor.is_hidden(),
                )
                .desc(ctx.platform.cursor.limitation().unwrap_or_default()),
                PanelRow::toggle("stick", ctx.t("cc.pointer_stick"), ctx.controller_mouse),
            ],
            Icon::End => vec![
                PanelRow::button("end", ctx.t("cc.end_confirm")).danger(),
                PanelRow::button("cancel", ctx.t("common.cancel")),
            ],
            Icon::Home | Icon::Game => Vec::new(),
        }
    }

    /// `true` = fechar a Central.
    pub fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> bool {
        let icon = ICONS[self.icon];
        match (self.panel, cmd) {
            (None, Cmd::Left) => self.icon = self.icon.saturating_sub(1),
            (None, Cmd::Right) => self.icon = (self.icon + 1).min(ICONS.len() - 1),
            (None, Cmd::Back) | (None, Cmd::Guide) | (Some(_), Cmd::Guide) => {
                ctx.effect(Effect::ResumeGame);
                return true;
            }
            (None, Cmd::Accept) | (None, Cmd::Down) => match icon {
                Icon::Home => {
                    ctx.effect(Effect::GoHomeKeepingGame);
                    return true;
                }
                Icon::Game => {
                    if cmd == Cmd::Accept {
                        ctx.effect(Effect::ResumeGame);
                        return true;
                    }
                }
                _ => {
                    let rows = self.rows(ctx, icon);
                    self.panel = rows.iter().position(PanelRow::is_focusable);
                }
            },
            (Some(_), Cmd::Back) => self.panel = None,
            (Some(r), Cmd::Up) => {
                let rows = self.rows(ctx, icon);
                self.panel = (0..r).rev().find(|i| rows[*i].is_focusable()).or(Some(r));
                if self.panel == Some(r)
                    && r == rows.iter().position(PanelRow::is_focusable).unwrap_or(0)
                {
                    self.panel = None;
                }
            }
            (Some(r), Cmd::Down) => {
                let rows = self.rows(ctx, icon);
                self.panel = (r + 1..rows.len())
                    .find(|i| rows[*i].is_focusable())
                    .or(Some(r));
            }
            (Some(r), Cmd::Left) | (Some(r), Cmd::Right) => {
                let rows = self.rows(ctx, icon);
                if rows.get(r).is_some_and(|row| row.key == "volume") {
                    self.adjust_volume(ctx, if cmd == Cmd::Left { -0.05 } else { 0.05 });
                } else {
                    // ← → levam o painel ao ícone vizinho com painel (Encerrar nunca abre deslizando).
                    let next = if cmd == Cmd::Left {
                        self.icon.checked_sub(1)
                    } else {
                        Some(self.icon + 1)
                    };
                    if let Some(n) = next.filter(|n| {
                        matches!(
                            ICONS.get(*n),
                            Some(Icon::Audio | Icon::Screen | Icon::Pointer)
                        )
                    }) {
                        self.icon = n;
                        let rows = self.rows(ctx, ICONS[n]);
                        self.panel = rows.iter().position(PanelRow::is_focusable);
                    }
                }
            }
            (Some(r), Cmd::Accept) => return self.accept(ctx, icon, r),
            _ => {}
        }
        false
    }

    fn adjust_volume(&self, ctx: &mut Ctx, delta: f32) {
        let Ok(state) = ctx.platform.audio.capture() else {
            return;
        };
        if let Some(id) = state.default_id.clone() {
            let v = state.level_of(&id).map(|l| l.volume).unwrap_or(0.5);
            let _ = ctx
                .platform
                .audio
                .set_volume(&id, (v + delta).clamp(0.0, 1.0));
        }
    }

    fn accept(&mut self, ctx: &mut Ctx, icon: Icon, r: usize) -> bool {
        let rows = self.rows(ctx, icon);
        let Some(row) = rows.get(r) else { return false };
        match (icon, row.key.as_str()) {
            (Icon::Audio, "mute") => {
                if let Ok(s) = ctx.platform.audio.capture()
                    && let Some(id) = s.default_id.clone()
                {
                    let muted = s.level_of(&id).is_some_and(|l| l.muted);
                    let _ = ctx.platform.audio.set_mute(&id, !muted);
                }
            }
            (Icon::Audio, "volume") => {}
            (Icon::Audio, id) => {
                match ctx.platform.audio.set_default(id) {
                    Ok(()) => ctx.toast(ctx.tf("cc.audio_set", &[("device", &row.title)])),
                    Err(e) => ctx.toast(e.to_string()),
                }
                self.panel = None;
            }
            (Icon::Screen, key) => {
                if let Some(mode) = DisplayMode::parse(key) {
                    let id = ctx.settings.string(super::ctx::keys::GAME_SCREEN_ID);
                    let displays = ctx.platform.displays.list().unwrap_or_default();
                    let target = displays
                        .iter()
                        .find(|d| d.id == id)
                        .or_else(|| displays.iter().find(|d| d.is_primary));
                    if let Some(t) = target {
                        match ctx.platform.displays.set_mode(&t.id, mode) {
                            Ok(()) => ctx.toast(mode.to_string()),
                            Err(e) => ctx.toast(e.to_string()),
                        }
                    }
                    self.panel = None;
                }
            }
            (Icon::Pointer, "hide") => {
                let hidden = !ctx.platform.cursor.is_hidden();
                if let Err(e) = ctx.platform.cursor.set_hidden(hidden) {
                    ctx.toast(e.to_string());
                }
                ctx.effect(Effect::PointerChanged);
            }
            (Icon::Pointer, "stick") => {
                if !ctx.controller_mouse
                    && let Err(e) = ctx.platform.pointer.check_access()
                {
                    ctx.toast(e.to_string());
                    return false;
                }
                ctx.controller_mouse = !ctx.controller_mouse;
                ctx.effect(Effect::PointerChanged);
            }
            (Icon::End, "end") => {
                ctx.effect(Effect::EndGame);
                return true;
            }
            (Icon::End, _) => self.panel = None,
            _ => {}
        }
        false
    }

    pub fn render(&self, ctx: &Ctx, out: &mut Out) {
        let icons: Vec<PanelRow> = ICONS
            .iter()
            .map(|i| match i {
                Icon::Home => PanelRow::button("home", ctx.t("cc.home")).icon("home"),
                Icon::Game => PanelRow::button("game", ctx.t("cc.back_to_game")).icon("play"),
                Icon::Audio => PanelRow::button("audio", ctx.t("cc.audio")).icon("volume"),
                Icon::Screen => PanelRow::button("screen", ctx.t("cc.screen")).icon("monitor"),
                Icon::Pointer => PanelRow::button("pointer", ctx.t("cc.pointer")).icon("pointer"),
                Icon::End => PanelRow::button("end", ctx.t("cc.end"))
                    .icon("power")
                    .danger(),
            })
            .collect();
        out.set("control-center", true);
        out.set("cc-icons", icons);
        out.set("cc-icon-focus", self.icon as i32);
        out.set(
            "cc-panel-for",
            if self.panel.is_some() {
                self.icon as i32
            } else {
                -1
            },
        );
        out.set(
            "cc-panel",
            if self.panel.is_some() {
                self.rows(ctx, ICONS[self.icon])
            } else {
                Vec::new()
            },
        );
        out.set("cc-panel-focus", self.panel.map(|p| p as i32).unwrap_or(-1));
        let running = ctx.running.as_ref();
        out.set(
            "cc-game-title",
            running.map(|r| r.title.clone()).unwrap_or_default(),
        );
        let mut meta = ctx.t("cc.playing");
        if let Some(p) = ctx.pad_battery {
            meta = format!(
                "{meta} · {}",
                ctx.tf("cc.pad_battery", &[("percent", &p.to_string())])
            );
        }
        out.set("cc-game-meta", meta);
    }

    pub fn hints(&self, ctx: &Ctx) -> Vec<Hint> {
        let mut h = vec![ctx.hint(Cmd::Accept, &ctx.t("hint.select"))];
        if self.panel.is_some() {
            h.push(ctx.hint_raw("←→", &ctx.t("hint.adjust")));
            h.push(ctx.hint(Cmd::Back, &ctx.t("hint.close")));
        } else {
            h.push(ctx.hint(Cmd::Back, &ctx.t("hint.back_to_game")));
        }
        h
    }

    pub fn click(&mut self, zone: &str, index: usize, ctx: &mut Ctx) -> bool {
        match zone {
            "icon" => {
                self.icon = index.min(ICONS.len() - 1);
                self.panel = None;
                self.handle(Cmd::Accept, ctx)
            }
            "panel" => {
                self.panel = Some(index);
                self.handle(Cmd::Accept, ctx)
            }
            _ => false,
        }
    }
}
