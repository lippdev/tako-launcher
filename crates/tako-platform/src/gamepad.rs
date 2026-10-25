//! Controles pelo SDL3 (PRD §4): banco de mapeamentos da comunidade, DualSense/Edge, Switch Pro, Xbox,
//! 8BitDo e reconexão sem reiniciar. Junta todos os controles conectados num estado só (como o
//! `GamepadFusion` do C#): qualquer controle navega.
//!
//! O SDL é bombeado por quem chama [`Pads::poll`] ou [`Pads::wait`], sempre na thread que chamou [`Pads::open`]:
//! a interface Slint (timer na thread da UI), o supervisor durante o jogo e o núcleo do `tako --web` (thread
//! própria). As regras que não dependem do SDL estão em [`tako_core::input::pad`].

use sdl3::event::Event;
use sdl3::gamepad::{Axis, Button, Gamepad, GamepadType};
use sdl3::joystick::PowerLevel;
use std::collections::HashMap;
use tako_core::input::{GamepadButtons, low_battery_warning};

pub use tako_core::input::pad::{PadFamily, PadState, buttons, merge};

#[derive(Debug, Clone, PartialEq)]
pub enum PadEvent {
    /// `guid`: o do SDL (identifica o modelo; os perfis de botões são por ele).
    Connected {
        name: String,
        family: PadFamily,
        guid: String,
    },
    Disconnected {
        remaining: usize,
    },
}

struct Pad {
    pad: Gamepad,
    guid: String,
    state: PadState,
    /// Já avisou de bateria fraca nesta conexão.
    low_warned: bool,
}

pub struct Pads {
    _sdl: sdl3::Sdl,
    subsystem: sdl3::GamepadSubsystem,
    pump: sdl3::EventPump,
    pads: HashMap<u32, Pad>,
    last_family: PadFamily,
    last_name: String,
    last_guid: String,
    /// Controle em uso (o último que conectou ou apertou um botão).
    last_id: u32,
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
            last_guid: String::new(),
            last_id: 0,
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

    /// GUID do controle em uso.
    pub fn guid(&self) -> &str {
        &self.last_guid
    }

    /// Controles conectados: `(guid, nome)`.
    pub fn connected(&self) -> Vec<(String, String)> {
        let mut v: Vec<_> = self
            .pads
            .values()
            .map(|p| (p.guid.clone(), p.pad.name().unwrap_or_default()))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    /// `(vendor, product)` do USB de cada controle conectado (para esconder do jogo: INPUT-005).
    pub fn ids(&self) -> Vec<(u16, u16)> {
        let mut v: Vec<_> = self
            .pads
            .values()
            .filter_map(|p| Some((p.pad.vendor_id()?, p.pad.product_id()?)))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    /// Botões de cada controle (`(guid, botões)`), para traduzir pelo perfil de cada um.
    pub fn buttons(&self) -> impl Iterator<Item = (&str, GamepadButtons)> {
        self.pads
            .values()
            .map(|p| (p.guid.as_str(), GamepadButtons(p.state.buttons)))
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
                    let guid = self.subsystem.guid_for_id(which).string();
                    self.last_family = family;
                    self.last_name = name.clone();
                    self.last_guid = guid.clone();
                    self.last_id = which_id(&which);
                    self.pads.insert(
                        which_id(&which),
                        Pad {
                            pad,
                            guid: guid.clone(),
                            state: PadState::default(),
                            low_warned: false,
                        },
                    );
                    out.push(PadEvent::Connected { name, family, guid });
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
                self.last_guid = p.guid.clone();
                self.last_id = id;
            } else {
                p.state.buttons &= !bit;
            }
        }
    }

    /// Lê a bateria pelo SDL (chamar de tempos em tempos, não a cada quadro): a do controle em uso e
    /// `(nome, %)` dos que acabaram de ficar com bateria fraca (um aviso por conexão).
    pub fn battery(&mut self) -> (Option<i32>, Vec<(String, i32)>) {
        let mut low = Vec::new();
        let mut active = None;
        for (id, p) in &mut self.pads {
            let level = battery_of(&p.pad);
            if *id == self.last_id {
                active = level;
            }
            if let Some(pct) = level
                && low_battery_warning(&mut p.low_warned, level)
            {
                low.push((p.pad.name().unwrap_or_default(), pct));
            }
        }
        (active, low)
    }

    /// Estado somado: botões em OU; em cada eixo, o de maior módulo.
    pub fn state(&self) -> PadState {
        merge(self.pads.values().map(|p| p.state))
    }
}

/// Porcentagem de um controle sem fio na bateria; com fio, carregando ou desconhecida = `None`.
fn battery_of(pad: &Gamepad) -> Option<i32> {
    let info = pad.power_info();
    (info.state == PowerLevel::OnBattery && info.percentage >= 0).then_some(info.percentage)
}

fn which_id(w: &sdl3::joystick::JoystickId) -> u32 {
    w.raw()
}
