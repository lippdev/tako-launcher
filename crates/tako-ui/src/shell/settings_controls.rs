//! Configurações › Controles: atalhos da Central de Controle (combinação do controle e tecla), os botões das
//! ações do Tako (INPUT-001) e o teste de controle ao vivo.
//!
//! As camadas só recebem `Cmd`; a captura e o teste leem `ctx.pressed_buttons` a cada desenho e pedem
//! desenho contínuo pelo `ctx.live_pad` enquanto estão abertas.

use super::{Ctx, Flow, MenuItem, action, described, toggle};
use crate::gamepad::PadFamily;
use crate::model::{Hint, Overlay, PanelRow};
use crate::shell::ctx::{InputKind, button_glyph, keys};
use crate::shell::keyboard::KeyboardLayer;
use crate::shell::layer::Pick;
use crate::shell::{Cmd, Effect, Layer, MenuLayer};
use std::cell::{Cell, RefCell};
use std::time::Instant;
use tako_core::devices::controller_hiding;
use tako_core::input::profiles::{self, Profile, Profiles};
use tako_core::input::shortcut::{
    ChordCapture, ChordCaptureState, GamepadFamily, KeyModifiers, KeyShortcut, check_chord,
    check_keyboard, chord,
};
use tako_core::input::{AppCommand, ButtonMap, GamepadButtons, GamepadSnapshot, controller_test};

pub(crate) fn family(ctx: &Ctx) -> GamepadFamily {
    match ctx.pad {
        PadFamily::PlayStation => GamepadFamily::PlayStation5,
        _ => GamepadFamily::Xbox,
    }
}

fn chord_label(ctx: &Ctx) -> String {
    chord::format(ctx.profile().chord, family(ctx))
}

fn key_label(ctx: &Ctx) -> String {
    KeyShortcut::parse(ctx.settings.get(keys::CONTROL_CENTER_KEY)).display()
}

/// Nome do perfil para mostrar (o padrão não tem nome gravado).
fn profile_name(ctx: &Ctx, p: &Profile) -> String {
    if p.is_default() {
        ctx.t("settings.controls.profile_default")
    } else {
        p.name.clone()
    }
}

/// Muda os perfis e grava.
fn edit_profiles(ctx: &mut Ctx, change: impl FnOnce(&mut Profiles)) {
    let mut p = ctx.profiles.clone();
    change(&mut p);
    ctx.save_profiles(p);
}

pub fn rows(ctx: &Ctx) -> Vec<MenuItem> {
    let pad = if ctx.pad_name.is_empty() {
        ctx.t("settings.controls.no_pad")
    } else {
        ctx.pad_name.clone()
    };
    let defaults = ctx.tf(
        "settings.controls.defaults",
        &[
            ("chord", &chord::format(chord::DEFAULT, family(ctx))),
            ("key", &KeyShortcut::default().display()),
        ],
    );
    // No desktop (TelaControles) o controle conectado vira a linha dele, com a bateria à direita.
    let controller = if ctx.desktop && !ctx.pad_name.is_empty() {
        let battery = ctx.pad_battery.map(|p| format!("{p}%")).unwrap_or_default();
        let desc = match ctx.pad_battery {
            Some(p) if p < 20 => ctx.tf(
                "status.battery_low",
                &[("pad", &ctx.pad_name), ("percent", &p.to_string())],
            ),
            _ => ctx.t("settings.controls.controller"),
        };
        PanelRow::info("controller", pad, battery)
            .desc(desc)
            .icon("gamepad")
    } else {
        PanelRow::info("controller", ctx.t("settings.controls.controller"), pad)
            .desc(ctx.t("settings.controls.controller_desc"))
    };
    let mut v = vec![
        MenuItem::new(controller),
        MenuItem::new(PanelRow::header(ctx.t("settings.controls.center_header"))),
        MenuItem::new(described(ctx, "settings.controls.key").value(key_label(ctx)))
            .on(|_| Flow::Push(Box::new(KeyLayer::default()))),
        MenuItem::new(
            PanelRow::action("reset", ctx.t("settings.controls.reset"))
                .desc(defaults)
                .value(ctx.t("settings.controls.reset_value")),
        )
        .on(|ctx| {
            let id = ctx.profile().id;
            edit_profiles(ctx, |p| {
                if let Some(p) = p.get_mut(id) {
                    p.chord = chord::DEFAULT;
                }
            });
            ctx.settings.set(
                keys::CONTROL_CENTER_KEY,
                &KeyShortcut::default().serialize(),
            );
            ctx.toast(ctx.t("settings.controls.reset_done"));
            Flow::Stay
        }),
        MenuItem::new(action(ctx, "settings.controls.test"))
            .on(|_| Flow::Push(Box::new(TestLayer))),
    ];
    profile_rows(ctx, &mut v);
    button_rows(ctx, &mut v);
    v.extend(hide_pads_row(ctx));
    v.extend([
        MenuItem::new(PanelRow::header(ctx.t("settings.controls.keyboard_header"))),
        MenuItem::new(
            PanelRow::info("keys", ctx.t("settings.controls.keys"), "")
                .desc(ctx.t("settings.controls.keys_desc")),
        ),
    ]);
    v
}

/// Perfis (INPUT-001): o perfil de cada controle conhecido e criar, renomear e excluir o do controle em uso.
fn profile_rows(ctx: &Ctx, v: &mut Vec<MenuItem>) {
    v.push(MenuItem::new(PanelRow::header(
        ctx.t("settings.controls.profiles_header"),
    )));
    // Os conectados e os que já ganharam perfil (o nome gravado é só para mostrar).
    let mut pads: Vec<(String, String)> = ctx.pads.clone();
    for c in ctx.profiles.controllers() {
        if !pads.iter().any(|(g, _)| g.eq_ignore_ascii_case(&c.guid)) {
            pads.push((c.guid.clone(), c.name.clone()));
        }
    }
    if pads.is_empty() {
        v.push(MenuItem::new(PanelRow::info(
            "profile-no-pad",
            ctx.t("settings.controls.profile_no_pad"),
            "",
        )));
    }
    for (i, (guid, name)) in pads.into_iter().enumerate() {
        let profile = ctx.profiles.of(&guid);
        let connected = ctx.pads.iter().any(|(g, _)| *g == guid);
        let desc = if guid.eq_ignore_ascii_case(&ctx.pad_guid) {
            ctx.t("settings.controls.profile_in_use")
        } else if connected {
            String::new()
        } else {
            ctx.t("settings.controls.profile_offline")
        };
        let label = if name.is_empty() {
            ctx.t("settings.controls.no_pad")
        } else {
            name.clone()
        };
        let (g1, n1) = (guid.clone(), name.clone());
        v.push(
            MenuItem::new(
                PanelRow::choice(format!("pad-{i}"), label, profile_name(ctx, profile)).desc(desc),
            )
            .on(move |ctx| cycle_profile(ctx, &g1, &n1, 1))
            .on_adjust({
                let (guid, name) = (guid.clone(), name.clone());
                move |ctx, d| cycle_profile(ctx, &guid, &name, d)
            })
            .options(move |ctx| {
                let all = ctx.profiles.all();
                let now = ctx.profiles.of(&guid).id;
                let cur = all.iter().position(|p| p.id == now).unwrap_or(0);
                let picks = all
                    .iter()
                    .map(|p| {
                        let (guid, name, id) = (guid.clone(), name.clone(), p.id);
                        Pick::new(profile_name(ctx, p))
                            .set(move |ctx| assign_profile(ctx, &guid, &name, id))
                    })
                    .collect();
                (picks, cur)
            }),
        );
    }
    let current = ctx.profile();
    let custom = !current.is_default();
    let name = profile_name(ctx, current);
    v.push(
        MenuItem::new(
            PanelRow::action("profile-new", ctx.t("settings.controls.profile_new"))
                .desc(ctx.tf("settings.controls.profile_new_desc", &[("name", &name)])),
        )
        .on(|ctx| Flow::Push(Box::new(new_profile(ctx)))),
    );
    v.push(
        MenuItem::new(
            PanelRow::action("profile-rename", ctx.t("settings.controls.profile_rename"))
                .value(if custom { name.clone() } else { String::new() })
                .enabled(custom),
        )
        .on(|ctx| Flow::Push(Box::new(rename_profile(ctx)))),
    );
    v.push(
        MenuItem::new(
            PanelRow::action("profile-delete", ctx.t("settings.controls.profile_delete"))
                .desc(ctx.tf("settings.controls.profile_delete_desc", &[("name", &name)]))
                .enabled(custom),
        )
        .on(|ctx| Flow::Push(Box::new(confirm_delete(ctx)))),
    );
}

/// Os botões e a combinação da Central do perfil do controle em uso.
fn button_rows(ctx: &Ctx, v: &mut Vec<MenuItem>) {
    let profile = ctx.profile();
    v.push(MenuItem::new(PanelRow::header(ctx.tf(
        "settings.controls.buttons_header",
        &[("name", &profile_name(ctx, profile))],
    ))));
    // Um botão por ação do Tako: A ou →/← passa ao próximo botão, trocando com a ação que o usava.
    for command in ButtonMap::commands() {
        let value = profile
            .buttons
            .button(command)
            .map_or("", |b| button_glyph(ctx.pad, b));
        v.push(
            MenuItem::new(PanelRow::choice(
                format!("button-{}", action_key(command)),
                ctx.t(&format!("settings.controls.action.{}", action_key(command))),
                value,
            ))
            .on(move |ctx| cycle_button(ctx, command, 1))
            .on_adjust(move |ctx, d| cycle_button(ctx, command, d))
            .options(move |ctx| {
                let now = ctx.profile().buttons.button(command);
                let all: Vec<GamepadButtons> = ButtonMap::buttons().collect();
                let cur = all.iter().position(|b| Some(*b) == now).unwrap_or(0);
                let picks = all
                    .into_iter()
                    .map(|b| {
                        Pick::new(button_glyph(ctx.pad, b))
                            .set(move |ctx| change_buttons(ctx, |m| m.assign(command, b)))
                    })
                    .collect();
                (picks, cur)
            }),
        );
    }
    // A combinação da Central também é do perfil (precisa de View ou Guia, como antes).
    v.push(
        MenuItem::new(
            PanelRow::action("chord", ctx.t("settings.controls.chord"))
                .desc(ctx.tf(
                    "settings.controls.chord_desc",
                    &[("accept", ctx.pad_glyph(Cmd::Accept))],
                ))
                .value(chord_label(ctx)),
        )
        .on(|ctx| {
            if ctx.pads_connected == 0 && ctx.pad_name.is_empty() {
                ctx.toast(ctx.t("settings.controls.connect_pad"));
                return Flow::Stay;
            }
            Flow::Push(Box::new(ChordLayer::new(ctx)))
        }),
    );
    let default = ButtonMap::default();
    let glyph = |c| default.button(c).map_or("", |b| button_glyph(ctx.pad, b));
    let reset_desc = ctx.tf(
        "settings.controls.buttons_reset_desc",
        &[
            ("accept", glyph(AppCommand::Accept)),
            ("back", glyph(AppCommand::Back)),
            ("favorite", glyph(AppCommand::Favorite)),
            ("search", glyph(AppCommand::Search)),
            ("previous", glyph(AppCommand::PreviousSection)),
            ("next", glyph(AppCommand::NextSection)),
            ("menu", glyph(AppCommand::Menu)),
        ],
    );
    v.push(
        MenuItem::new(
            PanelRow::action("buttons-reset", ctx.t("settings.controls.buttons_reset"))
                .desc(reset_desc)
                .value(ctx.t("settings.controls.reset_value"))
                .enabled(profile.buttons != default),
        )
        .on(|ctx| {
            let id = ctx.profile().id;
            edit_profiles(ctx, |p| {
                if let Some(p) = p.get_mut(id) {
                    p.buttons = ButtonMap::default();
                }
            });
            ctx.toast(ctx.t("settings.controls.buttons_reset_done"));
            Flow::Stay
        }),
    );
}

fn action_key(command: AppCommand) -> &'static str {
    match command {
        AppCommand::Accept => "accept",
        AppCommand::Back => "back",
        AppCommand::Favorite => "favorite",
        AppCommand::Search => "search",
        AppCommand::PreviousSection => "previous",
        AppCommand::NextSection => "next",
        AppCommand::Menu => "menu",
        _ => "",
    }
}

/// Próximo perfil para o controle (`dir` = 1 ou -1).
fn cycle_profile(ctx: &mut Ctx, guid: &str, name: &str, dir: i32) -> Flow {
    let ids: Vec<u32> = ctx.profiles.all().iter().map(|p| p.id).collect();
    let now = ctx.profiles.of(guid).id;
    let i = ids.iter().position(|id| *id == now).unwrap_or(0) as i32;
    let next = ids[(i + dir).rem_euclid(ids.len() as i32) as usize];
    assign_profile(ctx, guid, name, next)
}

/// Põe o perfil no controle e avisa.
fn assign_profile(ctx: &mut Ctx, guid: &str, name: &str, next: u32) -> Flow {
    edit_profiles(ctx, |p| p.assign(guid, name, next));
    let label = profile_name(ctx, ctx.profiles.of(guid));
    let pad = if name.is_empty() {
        ctx.t("settings.controls.no_pad")
    } else {
        name.to_owned()
    };
    ctx.toast(ctx.tf(
        "settings.controls.profile_assigned",
        &[("pad", &pad), ("profile", &label)],
    ));
    Flow::Stay
}

fn name_rule(ctx: &Ctx, name: &str) -> Option<String> {
    profiles::clean_name(name)
        .is_none()
        .then(|| ctx.t("settings.controls.profile_name_rule"))
}

/// "Novo perfil": copia o perfil do controle em uso e passa a valer para ele.
fn new_profile(ctx: &Ctx) -> KeyboardLayer {
    KeyboardLayer::new(
        ctx,
        ctx.t("settings.controls.profile_new_title"),
        "",
        |ctx, name| {
            let from = ctx.profile().id;
            let (guid, pad) = (ctx.pad_guid.clone(), ctx.pad_name.clone());
            let mut p = ctx.profiles.clone();
            let Some(id) = p.create(name, from) else {
                let max = profiles::MAX_PROFILES.to_string();
                ctx.toast(ctx.tf("settings.controls.profile_limit", &[("max", &max)]));
                return Flow::Stay;
            };
            p.assign(&guid, &pad, id);
            ctx.save_profiles(p);
            let name = ctx
                .profiles
                .get(id)
                .map(|p| p.name.clone())
                .unwrap_or_default();
            ctx.toast(ctx.tf("settings.controls.profile_created", &[("name", &name)]));
            Flow::Close
        },
    )
    .check(name_rule)
}

fn rename_profile(ctx: &Ctx) -> KeyboardLayer {
    let current = ctx.profile();
    let id = current.id;
    KeyboardLayer::new(
        ctx,
        ctx.t("settings.controls.profile_rename"),
        &current.name,
        move |ctx, name| {
            let mut p = ctx.profiles.clone();
            if !p.rename(id, name) {
                return Flow::Stay;
            }
            ctx.save_profiles(p);
            let name = ctx
                .profiles
                .get(id)
                .map(|p| p.name.clone())
                .unwrap_or_default();
            ctx.toast(ctx.tf("settings.controls.profile_renamed", &[("name", &name)]));
            Flow::Close
        },
    )
    .check(name_rule)
}

fn confirm_delete(ctx: &Ctx) -> MenuLayer {
    let current = ctx.profile();
    let (id, name) = (current.id, current.name.clone());
    MenuLayer::dialog(
        ctx.tf("settings.controls.profile_delete_title", &[("name", &name)]),
        move |ctx| {
            let name = name.clone();
            vec![
                MenuItem::new(
                    PanelRow::button("delete", ctx.t("settings.controls.profile_delete")).danger(),
                )
                .on(move |ctx| {
                    edit_profiles(ctx, |p| {
                        p.delete(id);
                    });
                    ctx.toast(ctx.tf("settings.controls.profile_deleted", &[("name", &name)]));
                    Flow::Close
                }),
                MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel")))
                    .on(|_| Flow::Close),
            ]
        },
    )
    .text(ctx.t("settings.controls.profile_delete_text"))
}

/// Troca o botão da ação e avisa com quem trocou (o botão que o usuário aperta pode ter mudado de função).
fn cycle_button(ctx: &mut Ctx, command: AppCommand, dir: i32) -> Flow {
    change_buttons(ctx, |m| m.cycle(command, dir))
}

fn change_buttons(ctx: &mut Ctx, f: impl FnOnce(&mut ButtonMap)) -> Flow {
    let id = ctx.profile().id;
    let before = ctx.profile().buttons;
    edit_profiles(ctx, |p| {
        if let Some(p) = p.get_mut(id) {
            f(&mut p.buttons);
        }
    });
    let map = ctx.profile().buttons;
    let moved = ButtonMap::commands()
        .filter(|c| map.button(*c) != before.button(*c))
        .map(|c| {
            let glyph = map.button(c).map_or("", |b| button_glyph(ctx.pad, b));
            format!(
                "{} {glyph}",
                ctx.t(&format!("settings.controls.action.{}", action_key(c)))
            )
        })
        .collect::<Vec<_>>()
        .join(" · ");
    ctx.toast(moved);
    Flow::Stay
}

/// Esconder o controle do jogo com a Central aberta (INPUT-005): a opção, com o driver HidHide instalado, ou a
/// oferta dele (abre a página oficial; o Tako não baixa nem executa o instalador). Só no Windows.
pub(crate) fn hide_pads_row(ctx: &Ctx) -> Option<MenuItem> {
    let hiding = &ctx.platform.controller_hiding;
    if !hiding.supported() {
        return None;
    }
    if hiding.available() {
        return Some(toggle(
            ctx,
            "settings.controls.hide_pads",
            controller_hiding::SETTING,
            true,
        ));
    }
    Some(
        MenuItem::new(action(ctx, "settings.controls.hidhide")).on(|ctx| {
            ctx.effect(Effect::OpenUrl(controller_hiding::HIDHIDE_URL.into()));
            ctx.toast(ctx.t("settings.controls.hidhide_opened"));
            Flow::Stay
        }),
    )
}

/// "Aperte as teclas que quer usar" (port do `ShortcutCaptureViewModel` de teclado): a primeira tecla de verdade
/// com os modificadores pressionados; recusada, mostra o motivo e espera outra. Esc sozinho cancela.
#[derive(Default)]
pub(crate) struct KeyLayer {
    pressed: String,
    error: Option<String>,
}

fn kept(ctx: &mut Ctx) -> Flow {
    ctx.toast(ctx.tf("settings.controls.kept", &[("shortcut", &key_label(ctx))]));
    Flow::Close
}

impl Layer for KeyLayer {
    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Flow {
        // Do controle (as teclas chegam por `key`): B cancela.
        match cmd {
            Cmd::Back => kept(ctx),
            _ => Flow::Stay,
        }
    }

    fn key(&mut self, text: &str, mods: KeyModifiers, ctx: &mut Ctx) -> Option<Flow> {
        let Some(key) = crate::shell::keys::wpf_key(text) else {
            return Some(Flow::Stay);
        };
        if key == "Escape" && mods == KeyModifiers::NONE {
            return Some(kept(ctx));
        }
        if KeyShortcut::is_modifier_key(&key) {
            self.pressed = if mods == KeyModifiers::NONE {
                String::new()
            } else {
                KeyShortcut::new(mods, "…").display()
            };
            return Some(Flow::Stay);
        }
        let shortcut = KeyShortcut::new(mods, &key);
        self.pressed = shortcut.display();
        // O Tako em Rust não tem outro atalho global de teclado para conferir.
        let check = check_keyboard(&shortcut, &[]);
        if let Some(e) = check.error {
            self.error = Some(e);
            return Some(Flow::Stay);
        }
        ctx.settings
            .set(keys::CONTROL_CENTER_KEY, &shortcut.serialize());
        ctx.toast(match check.warning {
            Some(w) => ctx.tf("settings.controls.saved_warning", &[("warning", &w)]),
            None => ctx.tf(
                "settings.controls.saved",
                &[("shortcut", &shortcut.display())],
            ),
        });
        Some(Flow::Close)
    }

    fn render(&self, ctx: &Ctx) -> Overlay {
        let pressed = if self.pressed.is_empty() {
            ctx.t("settings.controls.no_key_pressed")
        } else {
            self.pressed.clone()
        };
        let mut row = PanelRow::info("pressed", ctx.t("settings.controls.pressed"), pressed);
        if let Some(e) = &self.error {
            row = row.desc(e.clone()).emphasized(true);
        }
        Overlay {
            kind: "dialog".into(),
            title: ctx.t("settings.controls.key_title"),
            overline: ctx.t("settings.controls.key_prompt").to_uppercase(),
            text: ctx.t("settings.controls.key_text"),
            rows: vec![
                PanelRow::info(
                    "current",
                    ctx.t("settings.controls.current"),
                    key_label(ctx),
                ),
                row,
            ],
            sel: -1,
            ..Default::default()
        }
    }

    fn hints(&self, ctx: &Ctx) -> Vec<Hint> {
        vec![ctx.hint(Cmd::Back, &ctx.t("common.cancel"))]
    }

    fn click(&mut self, zone: &str, _index: usize, _ctx: &mut Ctx) -> Flow {
        if zone == "outside" {
            return Flow::Close;
        }
        Flow::Stay
    }
}

/// "Segure os botões que quer usar": vale o conjunto segurado por 1 s; aceitar salva, voltar cancela. Grava no
/// perfil do controle em uso quando a tela abriu.
pub(crate) struct ChordLayer {
    profile: u32,
    cap: RefCell<ChordCapture>,
    started: Instant,
    /// Depois de capturar, A só salva quando tudo foi solto (o A segurado da combinação não conta).
    released: Cell<bool>,
}

impl ChordLayer {
    pub fn new(ctx: &Ctx) -> Self {
        let mut cap = ChordCapture::new();
        let back = ctx.profile().buttons.button(AppCommand::Back);
        cap.cancel = back.unwrap_or(GamepadButtons::B);
        ChordLayer {
            profile: ctx.profile().id,
            cap: RefCell::new(cap),
            started: Instant::now(),
            released: Cell::new(false),
        }
    }

    /// Combinação atual do perfil desta tela.
    fn label(&self, ctx: &Ctx) -> String {
        let p = ctx
            .profiles
            .get(self.profile)
            .unwrap_or(ctx.profiles.standard());
        chord::format(p.chord, family(ctx))
    }

    fn state(&self) -> ChordCaptureState {
        self.cap.borrow().state
    }

    /// Lê os botões de agora (chamado a cada desenho).
    pub fn tick(&self, ctx: &Ctx) {
        let pressed = GamepadButtons(ctx.pressed_buttons);
        let state = self
            .cap
            .borrow_mut()
            .update(pressed, self.started.elapsed());
        if state == ChordCaptureState::Captured && pressed.is_empty() {
            self.released.set(true);
        }
    }
}

impl Layer for ChordLayer {
    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Flow {
        use ChordCaptureState::*;
        let keyboard = ctx.input == InputKind::Keyboard;
        match (self.state(), cmd) {
            (Captured, Cmd::Accept) if self.released.get() => {
                let buttons = self.cap.borrow().result;
                let check = check_chord(buttons, family(ctx));
                if let Some(e) = check.error {
                    ctx.toast(e);
                    self.cap.borrow_mut().restart();
                    self.released.set(false);
                    return Flow::Stay;
                }
                let id = self.profile;
                edit_profiles(ctx, |p| {
                    if let Some(p) = p.get_mut(id) {
                        p.chord = buttons;
                    }
                });
                let label = chord::format(buttons, family(ctx));
                ctx.toast(match check.warning {
                    Some(w) => ctx.tf("settings.controls.saved_warning", &[("warning", &w)]),
                    None => ctx.tf("settings.controls.saved", &[("shortcut", &label)]),
                });
                Flow::Close
            }
            // B no meio da combinação é parte dela; sozinho (ou Esc) cancela.
            (Holding, Cmd::Back) if !keyboard => Flow::Stay,
            (Captured, Cmd::Back) if !self.released.get() && !keyboard => Flow::Stay,
            (_, Cmd::Back) | (Cancelled, _) => {
                ctx.toast(ctx.tf("settings.controls.kept", &[("shortcut", &self.label(ctx))]));
                Flow::Close
            }
            _ => Flow::Stay,
        }
    }

    fn render(&self, ctx: &Ctx) -> Overlay {
        self.tick(ctx);
        ctx.live_pad.set(true);
        let cap = self.cap.borrow();
        let fam = family(ctx);
        let mut rows = vec![PanelRow::info(
            "current",
            ctx.t("settings.controls.current"),
            self.label(ctx),
        )];
        let holding = if cap.holding.is_empty() {
            ctx.t("settings.controls.nothing_pressed")
        } else {
            chord::format(cap.holding, fam)
        };
        match cap.state {
            ChordCaptureState::Captured => {
                let check = check_chord(cap.result, fam);
                let note = check.error.or(check.warning).unwrap_or_else(|| {
                    if self.released.get() {
                        let accept = ctx.pad_glyph(Cmd::Accept);
                        ctx.tf("settings.controls.press_save", &[("accept", accept)])
                    } else {
                        ctx.t("settings.controls.release")
                    }
                });
                rows.push(
                    PanelRow::info(
                        "result",
                        ctx.t("settings.controls.new"),
                        chord::format(cap.result, fam),
                    )
                    .desc(note)
                    .emphasized(true),
                );
            }
            _ => rows.push(PanelRow::info(
                "holding",
                ctx.t("settings.controls.holding"),
                format!("{holding} · {:.0}%", cap.progress * 100.0),
            )),
        }
        Overlay {
            kind: "dialog".into(),
            title: ctx.t("settings.controls.chord_title"),
            overline: ctx.t("settings.controls.chord_prompt").to_uppercase(),
            text: ctx.tf(
                "settings.controls.chord_rules",
                &[
                    ("view", &chord::name(GamepadButtons::VIEW, fam)),
                    ("guide", &chord::name(GamepadButtons::GUIDE, fam)),
                    ("b", &chord::name(cap.cancel, fam)),
                ],
            ),
            rows,
            sel: -1,
            ..Default::default()
        }
    }

    fn hints(&self, ctx: &Ctx) -> Vec<Hint> {
        let mut h = Vec::new();
        if self.state() == ChordCaptureState::Captured {
            h.push(ctx.hint(Cmd::Accept, &ctx.t("settings.controls.save")));
        }
        h.push(ctx.hint(Cmd::Back, &ctx.t("common.cancel")));
        h
    }

    fn click(&mut self, zone: &str, _index: usize, _ctx: &mut Ctx) -> Flow {
        if zone == "outside" {
            return Flow::Close;
        }
        Flow::Stay
    }
}

/// Teste de controle: o que cada botão e o analógico mandam agora. Sai com ☰ (B é um dos botões testados)
/// ou Esc no teclado.
pub(crate) struct TestLayer;

impl Layer for TestLayer {
    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Flow {
        match cmd {
            Cmd::Menu => Flow::Close,
            Cmd::Back if ctx.input == InputKind::Keyboard => Flow::Close,
            _ => Flow::Stay,
        }
    }

    fn render(&self, ctx: &Ctx) -> Overlay {
        ctx.live_pad.set(true);
        let (x, y) = ctx.pad_stick;
        let snap = GamepadSnapshot::new(GamepadButtons(ctx.pressed_buttons), x as f64, -y as f64);
        let pad = if ctx.pad_name.is_empty() {
            ctx.t("settings.controls.no_pad")
        } else {
            ctx.pad_name.clone()
        };
        Overlay {
            kind: "dialog".into(),
            title: ctx.t("settings.controls.test"),
            overline: pad.to_uppercase(),
            text: ctx.tf(
                "settings.controls.test_text",
                &[("menu", ctx.pad_glyph(Cmd::Menu))],
            ),
            rows: vec![
                PanelRow::info("state", ctx.t("settings.controls.test_now"), "")
                    .desc(controller_test::describe(snap)),
            ],
            sel: -1,
            wide: true,
            ..Default::default()
        }
    }

    fn hints(&self, ctx: &Ctx) -> Vec<Hint> {
        vec![ctx.hint(Cmd::Menu, &ctx.t("hint.close"))]
    }

    fn click(&mut self, _zone: &str, _index: usize, _ctx: &mut Ctx) -> Flow {
        Flow::Close
    }
}
