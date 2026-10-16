//! Controles pelo SDL3 (PRD §4): banco de mapeamentos da comunidade, DualSense/Edge, Switch Pro, Xbox,
//! 8BitDo e reconexão sem reiniciar. Junta todos os controles conectados num estado só (como o
//! `GamepadFusion` do C#): qualquer controle navega.
//!
//! O SDL é bombeado por quem chama [`Pads::poll`] (na thread da UI por um timer, ou no supervisor).

use sdl3::event::Event;
use sdl3::gamepad::{Axis, Button, Gamepad, GamepadType};
use std::collections::HashMap;

/// Botões por posição (padrão Xbox: A embaixo). Mesmos bits de `GamepadButtons` do C#.
pub mod buttons {
    pub const DPAD_UP: u32 = 0x0001;
    pub const DPAD_DOWN: u32 = 0x0002;
    pub const DPAD_LEFT: u32 = 0x0004;
    pub const DPAD_RIGHT: u32 = 0x0008;
    pub const START: u32 = 0x0010;
    pub const BACK: u32 = 0x0020;
    pub const LEFT_THUMB: u32 = 0x0040;
    pub const RIGHT_THUMB: u32 = 0x0080;
    pub const LEFT_SHOULDER: u32 = 0x0100;
    pub const RIGHT_SHOULDER: u32 = 0x0200;
    pub const GUIDE: u32 = 0x0400;
    pub const A: u32 = 0x1000;
    pub const B: u32 = 0x2000;
    pub const X: u32 = 0x4000;
    pub const Y: u32 = 0x8000;
}

/// Família do controle, para os glifos das dicas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PadFamily {
    #[default]
    Xbox,
    PlayStation,
    Nintendo,
}

/// Estado somado de todos os controles. Eixos de -1 a 1 (Y para baixo positivo); gatilhos de 0 a 1.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PadState {
    pub buttons: u32,
    pub left_x: f32,
    pub left_y: f32,
    pub right_x: f32,
    pub right_y: f32,
    pub left_trigger: f32,
    pub right_trigger: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PadEvent {
    Connected { name: String, family: PadFamily },
    Disconnected { remaining: usize },
}

struct Pad {
    pad: Gamepad,
    state: PadState,
}

pub struct Pads {
    _sdl: sdl3::Sdl,
    subsystem: sdl3::GamepadSubsystem,
    pump: sdl3::EventPump,
    pads: HashMap<u32, Pad>,
    last_family: PadFamily,
    last_name: String,
}

fn button_bit(b: Button) -> u32 {
    use buttons::*;
    match b {
        Button::South => A,
        Button::East => B,
        Button::West => X,
        Button::North => Y,
        Button::Back => BACK,
        Button::Start => START,
        Button::Guide => GUIDE,
        Button::LeftStick => LEFT_THUMB,
        Button::RightStick => RIGHT_THUMB,
        Button::LeftShoulder => LEFT_SHOULDER,
        Button::RightShoulder => RIGHT_SHOULDER,
        Button::DPadUp => DPAD_UP,
        Button::DPadDown => DPAD_DOWN,
        Button::DPadLeft => DPAD_LEFT,
        Button::DPadRight => DPAD_RIGHT,
        // O "Create"/Touchpad do DualSense não navega; o Misc1 (Share/Capture) também não.
        _ => 0,
    }
}

pub fn family_of(t: GamepadType) -> PadFamily {
    match t {
        GamepadType::PS3 | GamepadType::PS4 | GamepadType::PS5 => PadFamily::PlayStation,
        GamepadType::NintendoSwitchPro
        | GamepadType::NintendoSwitchJoyconLeft
        | GamepadType::NintendoSwitchJoyconRight
        | GamepadType::NintendoSwitchJoyconPair => PadFamily::Nintendo,
        _ => PadFamily::Xbox,
    }
}

/// Evento vazio que só acorda a espera.
struct Wake;

impl Pads {
    /// Liga só o subsistema de controles (sem vídeo nem áudio do SDL).
    pub fn open() -> Result<Pads, String> {
        // O controle continua sendo lido com a janela do Tako em segundo plano (jogo na frente).
        sdl3::hint::set("SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS", "1");
        let sdl = sdl3::init().map_err(|e| e.to_string())?;
        let subsystem = sdl.gamepad().map_err(|e| e.to_string())?;
        let pump = sdl.event_pump().map_err(|e| e.to_string())?;
        Ok(Pads {
            _sdl: sdl,
            subsystem,
            pump,
            pads: HashMap::new(),
            last_family: PadFamily::Xbox,
            last_name: String::new(),
        })
    }

    /// Função que acorda um [`Pads::wait`] em outra thread (ex.: o atalho de teclado durante o jogo).
    pub fn waker(&self) -> Option<impl Fn() + Send + 'static> {
        let events = self._sdl.event().ok()?;
        events.register_custom_event::<Wake>().ok()?;
        let sender = events.event_sender();
        Some(move || drop(sender.push_custom_event(Wake)))
    }

    pub fn count(&self) -> usize {
        self.pads.len()
    }

    pub fn family(&self) -> PadFamily {
        self.last_family
    }

    pub fn name(&self) -> &str {
        &self.last_name
    }

    /// Processa os eventos pendentes sem bloquear.
    pub fn poll(&mut self) -> Vec<PadEvent> {
        let mut out = Vec::new();
        while let Some(ev) = self.pump.poll_event() {
            self.handle(ev, &mut out);
        }
        out
    }

    /// Espera o próximo evento até `timeout` (supervisor: sem polling ativo).
    pub fn wait(&mut self, timeout: std::time::Duration) -> Vec<PadEvent> {
        let mut out = Vec::new();
        if let Some(ev) = self.pump.wait_event_timeout(timeout) {
            self.handle(ev, &mut out);
        }
        out.extend(self.poll());
        out
    }

    fn handle(&mut self, ev: Event, out: &mut Vec<PadEvent>) {
        match ev {
            Event::GamepadAdded { which, .. } => {
                if let Ok(pad) = self.subsystem.open(which) {
                    let name = pad.name().unwrap_or_default();
                    let family = family_of(pad.r#type());
                    self.last_family = family;
                    self.last_name = name.clone();
                    self.pads.insert(
                        which_id(&which),
                        Pad {
                            pad,
                            state: PadState::default(),
                        },
                    );
                    out.push(PadEvent::Connected { name, family });
                }
            }
            Event::GamepadRemoved { which, .. } => {
                self.pads.remove(&which_id(&which));
                out.push(PadEvent::Disconnected {
                    remaining: self.pads.len(),
                });
            }
            Event::GamepadButtonDown { which, button, .. } => {
                self.set_button(which_id(&which), button, true)
            }
            Event::GamepadButtonUp { which, button, .. } => {
                self.set_button(which_id(&which), button, false)
            }
            Event::GamepadAxisMotion {
                which, axis, value, ..
            } => {
                if let Some(p) = self.pads.get_mut(&which_id(&which)) {
                    let v = (value as f32 / 32767.0).clamp(-1.0, 1.0);
                    match axis {
                        Axis::LeftX => p.state.left_x = v,
                        Axis::LeftY => p.state.left_y = v,
                        Axis::RightX => p.state.right_x = v,
                        Axis::RightY => p.state.right_y = v,
                        Axis::TriggerLeft => p.state.left_trigger = v.max(0.0),
                        Axis::TriggerRight => p.state.right_trigger = v.max(0.0),
                    }
                }
            }
            _ => {}
        }
    }

    fn set_button(&mut self, id: u32, b: Button, down: bool) {
        let bit = button_bit(b);
        if let Some(p) = self.pads.get_mut(&id) {
            if down {
                p.state.buttons |= bit;
                // O último controle usado define os glifos.
                self.last_family = family_of(p.pad.r#type());
                self.last_name = p.pad.name().unwrap_or_default();
            } else {
                p.state.buttons &= !bit;
            }
        }
    }

    /// Estado somado: botões em OU; em cada eixo, o de maior módulo.
    pub fn state(&self) -> PadState {
        merge(self.pads.values().map(|p| p.state))
    }
}

fn which_id(w: &sdl3::joystick::JoystickId) -> u32 {
    w.raw()
}

pub fn merge(states: impl Iterator<Item = PadState>) -> PadState {
    let pick = |a: f32, b: f32| if b.abs() > a.abs() { b } else { a };
    states.fold(PadState::default(), |acc, s| PadState {
        buttons: acc.buttons | s.buttons,
        left_x: pick(acc.left_x, s.left_x),
        left_y: pick(acc.left_y, s.left_y),
        right_x: pick(acc.right_x, s.right_x),
        right_y: pick(acc.right_y, s.right_y),
        left_trigger: acc.left_trigger.max(s.left_trigger),
        right_trigger: acc.right_trigger.max(s.right_trigger),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_ors_buttons_and_keeps_strongest_axis() {
        let a = PadState {
            buttons: buttons::A,
            left_x: 0.2,
            ..Default::default()
        };
        let b = PadState {
            buttons: buttons::B,
            left_x: -0.9,
            ..Default::default()
        };
        let m = merge([a, b].into_iter());
        assert_eq!(m.buttons, buttons::A | buttons::B);
        assert_eq!(m.left_x, -0.9);
    }
}
