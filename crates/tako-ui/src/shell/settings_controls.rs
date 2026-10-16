//! Configurações › Controles: atalhos da Central de Controle (combinação do controle e tecla) e o teste
//! de controle ao vivo.
//!
//! As camadas só recebem `Cmd`; a captura e o teste leem `ctx.pressed_buttons` a cada desenho e pedem
//! desenho contínuo pelo `ctx.live_pad` enquanto estão abertas.

use super::{Ctx, Flow, MenuItem, MenuLayer, action, described};
use crate::gamepad::PadFamily;
use crate::model::{Hint, Overlay, PanelRow};
use crate::shell::ctx::{InputKind, keys};
use crate::shell::{Cmd, Layer};
use std::cell::{Cell, RefCell};
use std::time::Instant;
use tako_core::input::shortcut::{
    ChordCapture, ChordCaptureState, GamepadFamily, KeyModifiers, KeyShortcut, check_chord, chord,
};
use tako_core::input::{GamepadButtons, GamepadSnapshot, controller_test};

pub(crate) fn family(ctx: &Ctx) -> GamepadFamily {
    match ctx.pad {
        PadFamily::PlayStation => GamepadFamily::PlayStation5,
        _ => GamepadFamily::Xbox,
    }
}

fn chord_label(ctx: &Ctx) -> String {
    let b = chord::parse(ctx.settings.get(keys::CONTROL_CENTER_CHORD));
    chord::format(b, family(ctx))
}

fn key_label(ctx: &Ctx) -> String {
    KeyShortcut::parse(ctx.settings.get(keys::CONTROL_CENTER_KEY)).display()
}

/// Atalhos de teclado seguros para a Central (sem captura de tecla: o tema não entrega as teclas cruas).
fn key_presets() -> [KeyShortcut; 3] {
    [
        KeyShortcut::default(),
        KeyShortcut::new(KeyModifiers::CTRL | KeyModifiers::ALT, "T"),
        KeyShortcut::new(KeyModifiers::CTRL | KeyModifiers::SHIFT, "F12"),
    ]
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
    vec![
        MenuItem::new(
            PanelRow::info("controller", ctx.t("settings.controls.controller"), pad)
                .desc(ctx.t("settings.controls.controller_desc")),
        ),
        MenuItem::new(PanelRow::header(ctx.t("settings.controls.center_header"))),
        MenuItem::new(described(ctx, "settings.controls.chord").value(chord_label(ctx))).on(
            |ctx| {
                if ctx.pads_connected == 0 && ctx.pad_name.is_empty() {
                    ctx.toast(ctx.t("settings.controls.connect_pad"));
                    return Flow::Stay;
                }
                Flow::Push(Box::new(ChordLayer::new()))
            },
        ),
        MenuItem::new(described(ctx, "settings.controls.key").value(key_label(ctx)))
            .on(|ctx| Flow::Push(Box::new(key_panel(ctx)))),
        MenuItem::new(
            PanelRow::action("reset", ctx.t("settings.controls.reset"))
                .desc(defaults)
                .value(ctx.t("settings.controls.reset_value")),
        )
        .on(|ctx| {
            ctx.settings.set(
                keys::CONTROL_CENTER_CHORD,
                &chord::serialize(chord::DEFAULT),
            );
            ctx.settings.set(
                keys::CONTROL_CENTER_KEY,
                &KeyShortcut::default().serialize(),
            );
            ctx.toast(ctx.t("settings.controls.reset_done"));
            Flow::Stay
        }),
        MenuItem::new(action(ctx, "settings.controls.test"))
            .on(|_| Flow::Push(Box::new(TestLayer))),
        MenuItem::new(PanelRow::header(ctx.t("settings.controls.keyboard_header"))),
        MenuItem::new(
            PanelRow::info("keys", ctx.t("settings.controls.keys"), "")
                .desc(ctx.t("settings.controls.keys_desc")),
        ),
    ]
}

fn key_panel(ctx: &Ctx) -> MenuLayer {
    MenuLayer::panel(ctx.t("settings.controls.key_title"), |ctx| {
        let current = KeyShortcut::parse(ctx.settings.get(keys::CONTROL_CENTER_KEY));
        key_presets()
            .into_iter()
            .map(|k| {
                let on = k == current;
                MenuItem::new(PanelRow::radio(k.serialize(), k.display(), on)).on(move |ctx| {
                    ctx.settings.set(keys::CONTROL_CENTER_KEY, &k.serialize());
                    ctx.toast(ctx.tf("settings.controls.saved", &[("shortcut", &k.display())]));
                    Flow::Stay
                })
            })
            .collect()
    })
    .text(ctx.t("settings.controls.key_text"))
}

/// "Segure os botões que quer usar": vale o conjunto segurado por 1 s; A salva, B cancela.
pub(crate) struct ChordLayer {
    cap: RefCell<ChordCapture>,
    started: Instant,
    /// Depois de capturar, A só salva quando tudo foi solto (o A segurado da combinação não conta).
    released: Cell<bool>,
}

impl ChordLayer {
    pub fn new() -> Self {
        ChordLayer {
            cap: RefCell::new(ChordCapture::new()),
            started: Instant::now(),
            released: Cell::new(false),
        }
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
                ctx.settings
                    .set(keys::CONTROL_CENTER_CHORD, &chord::serialize(buttons));
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
                ctx.toast(ctx.tf("settings.controls.kept", &[("shortcut", &chord_label(ctx))]));
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
            chord_label(ctx),
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
                    ctx.t(if self.released.get() {
                        "settings.controls.press_save"
                    } else {
                        "settings.controls.release"
                    })
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
                    ("b", &chord::name(GamepadButtons::B, fam)),
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
            text: ctx.t("settings.controls.test_text"),
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
