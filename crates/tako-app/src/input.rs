//! Controle pela ponte (WEB-05b, PRD INPUT-001, INPUT-003, INPUT-004): as leituras do SDL3 viram comandos de
//! navegação, o estado do controle em uso e os avisos para a tela, com as mesmas regras da interface Slint.
//!
//! O SDL3 roda no processo do núcleo (`tako --web`), numa thread própria, e nunca no filho CEF. A thread entrega
//! cada leitura ([`PadFrame`]) a um [`InputBridge`], que diz o que vai para a interface ([`InputOutput`]). As regras
//! são as do `tako-core`: perfil de botões por controle, zona morta de 0,5 e repetição de 380/110 ms
//! (`GamepadInterpreter`), combinação da Central segurada por 500 ms e os avisos de conexão (`PadNotices`).
//!
//! Durante o jogo o controle é do jogo: nenhum comando vai para a interface, nem a que ainda mostra "Abrindo…".
//! Só a combinação da Central vale ([`InputOutput::Central`]): o hospedeiro reabre a interface por cima do jogo e,
//! com ela aberta assim, os comandos voltam a valer.

use crate::AppService;
use crate::protocol::{
    ControllerInfo, ControllerStyle, Event, InputAction, InputNav, InputNotice, InputNoticeKind,
};
use std::time::Duration;
use tako_core::input::pad::{PadFamily, PadNotice, PadNotices};
use tako_core::input::profiles::Profiles;
use tako_core::input::shortcut::{ButtonChordDetector, chord};
use tako_core::input::{AppCommand, GamepadButtons, GamepadInterpreter, GamepadSnapshot};

/// Um controle entrou ou saiu (evento do SDL).
#[derive(Debug, Clone, PartialEq)]
pub enum PadChange {
    Connected {
        name: String,
        family: PadFamily,
        guid: String,
    },
    Disconnected {
        /// Controles que continuam conectados.
        remaining: usize,
    },
}

/// Bateria lida agora (de tempos em tempos, nunca por quadro).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BatteryReading {
    /// A do controle em uso; `None` com fio, carregando ou desconhecida.
    pub active: Option<i32>,
    /// `(nome, %)` dos controles que acabaram de ficar com bateria fraca (um aviso por conexão).
    pub low: Vec<(String, i32)>,
}

/// Uma leitura de todos os controles.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PadFrame {
    pub changes: Vec<PadChange>,
    /// Botões físicos de cada controle conectado, pelo GUID do SDL (cada um passa pelo seu perfil).
    pub pads: Vec<(String, GamepadButtons)>,
    /// Analógico esquerdo somado, -1 a 1, como o SDL dá: Y positivo para baixo.
    pub left_x: f32,
    pub left_y: f32,
    pub count: usize,
    /// Família e nome do controle em uso (o último que conectou ou apertou um botão).
    pub family: PadFamily,
    pub name: String,
    /// `Some` quando o hospedeiro leu a bateria nesta volta ([`InputBridge::wants_battery`]).
    pub battery: Option<BatteryReading>,
}

/// Onde a interface está, visto do núcleo.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InputScene {
    /// Há uma página aberta que recebe eventos.
    pub listening: bool,
    /// Há um jogo aberto (abrindo, jogando ou fechando).
    pub playing: bool,
    /// A interface foi reaberta por cima do jogo pela combinação da Central.
    pub over_game: bool,
}

impl InputScene {
    /// Os comandos do controle vão para a interface.
    pub fn navigable(self) -> bool {
        self.listening && (!self.playing || self.over_game)
    }
}

/// O que o hospedeiro faz com uma leitura.
#[derive(Debug, Clone, PartialEq)]
pub enum InputOutput {
    /// Para a interface (`input.nav`, `input.controller`, `input.notice`).
    Event(Event),
    /// A combinação da Central foi segurada com um jogo aberto: reabrir a interface por cima dele ou, se já está
    /// assim, voltar ao jogo.
    Central,
}

/// Leituras do controle → o que vai para a interface. Uma instância na thread do SDL.
#[derive(Debug)]
pub struct InputBridge {
    interp: GamepadInterpreter,
    chord: ButtonChordDetector,
    notices: PadNotices,
    profiles: Profiles,
    scene: InputScene,
    info: ControllerInfo,
    /// A última entrada vista foi do controle (o núcleo não vê teclado nem mouse: conectar já conta, como no Slint).
    using_pad: bool,
    /// Próxima leitura da bateria; `None` = já.
    next_battery: Option<Duration>,
}

impl InputBridge {
    /// Com a interface aberta, uma volta a cada quadro (o timer de 16 ms do Slint): a repetição precisa do relógio
    /// mesmo sem evento novo do SDL.
    pub const FRAME: Duration = Duration::from_millis(16);
    /// Sem interface, segurando a combinação da Central: acorda antes dos 500 ms dela (supervisor do Slint).
    pub const HOLDING: Duration = Duration::from_millis(50);
    /// Sem interface: só espera eventos do SDL (o supervisor do Slint espera 2 s).
    pub const IDLE: Duration = Duration::from_secs(2);
    /// Bateria a cada 30 s com a interface aberta, e logo depois de um controle entrar ou sair.
    pub const BATTERY_EVERY: Duration = Duration::from_secs(30);

    pub fn new(profiles: Profiles) -> Self {
        Self {
            interp: GamepadInterpreter::new(),
            chord: ButtonChordDetector::new(chord::DEFAULT),
            notices: PadNotices::new(),
            profiles,
            scene: InputScene::default(),
            info: ControllerInfo::default(),
            using_pad: false,
            next_battery: None,
        }
    }

    /// Perfis de botões e combinações da Central (INPUT-001), lidos das configurações do perfil em uso.
    pub fn set_profiles(&mut self, profiles: Profiles) {
        self.profiles = profiles;
    }

    /// A interface abriu, fechou ou o jogo mudou. Ao voltar a navegar, o que já está apertado só vale depois de
    /// solto (o A ou a combinação que reabriu a interface não viram comando).
    pub fn set_scene(&mut self, scene: InputScene) {
        if scene.navigable() && !self.scene.navigable() {
            self.interp.reset();
        }
        self.scene = scene;
    }

    pub fn scene(&self) -> InputScene {
        self.scene
    }

    pub fn info(&self) -> &ControllerInfo {
        &self.info
    }

    /// Quanto esperar pelo próximo evento do SDL antes de uma volta sem evento.
    pub fn timeout(&self) -> Duration {
        if self.scene.listening {
            Self::FRAME
        } else if self.chord.holding() {
            Self::HOLDING
        } else {
            Self::IDLE
        }
    }

    /// Hora de ler a bateria (só com a interface aberta: fechada, o aviso espera por ela, como no Slint).
    pub fn wants_battery(&self, now: Duration) -> bool {
        self.scene.listening && self.info.count > 0 && self.next_battery.is_none_or(|at| now >= at)
    }

    /// Uma leitura. `now`: relógio monotônico de quem chama.
    pub fn update(&mut self, frame: PadFrame, now: Duration) -> Vec<InputOutput> {
        let mut out = Vec::new();
        let before = self.info.clone();
        for change in frame.changes {
            let notice = match change {
                PadChange::Connected { .. } => {
                    self.using_pad = true;
                    self.notices.connected()
                }
                PadChange::Disconnected { remaining } => {
                    self.notices
                        .disconnected(remaining, self.scene.playing, self.using_pad)
                }
            };
            if let Some(notice) = notice
                && self.scene.listening
            {
                out.push(notice_event(match notice {
                    PadNotice::Connected => InputNoticeKind::Connected,
                    PadNotice::Disconnected => InputNoticeKind::Disconnected,
                }));
            }
            // O que já estiver apertado no controle que voltou (ou no outro) só vale depois de solto.
            self.interp.reset();
            self.next_battery = None;
        }

        self.info.count = u32::try_from(frame.count).unwrap_or(u32::MAX);
        self.info.connected = frame.count > 0;
        self.info.style = style_of(frame.family);
        self.info.name = if frame.count > 0 {
            frame.name
        } else {
            String::new()
        };
        if frame.count == 0 {
            self.info.battery = None;
        }
        if let Some(battery) = frame.battery {
            self.next_battery = Some(now + Self::BATTERY_EVERY);
            self.info.battery = battery.active;
            if self.scene.listening {
                for (pad, percent) in battery.low {
                    out.push(Event::InputNotice(InputNotice {
                        kind: InputNoticeKind::BatteryLow,
                        pad: Some(pad),
                        percent: Some(percent),
                    }));
                }
            }
        }
        if self.info != before {
            out.push(Event::InputController(self.info.clone()));
        }

        // Cada controle pelo seu perfil (INPUT-001): botões e combinação da Central.
        let input = self
            .profiles
            .read(frame.pads.iter().map(|(guid, b)| (guid.as_str(), *b)));
        let central = self.chord.update_held(input.central, now) && self.scene.playing;
        let held = self.interp.pressed().clone();
        let snapshot = GamepadSnapshot::new(
            input.buttons,
            f64::from(frame.left_x),
            -f64::from(frame.left_y),
        );
        let fired = self.interp.update(snapshot, now, input.ignore);
        let mut out: Vec<InputOutput> = out.into_iter().map(InputOutput::Event).collect();
        if central {
            out.push(InputOutput::Central);
        } else if self.scene.navigable() {
            for command in fired {
                let Some(action) = action_of(command) else {
                    continue;
                };
                self.using_pad = true;
                out.push(InputOutput::Event(Event::InputNav(InputNav {
                    action,
                    repeat: held.contains(&command),
                })));
            }
        }
        out
    }
}

fn notice_event(kind: InputNoticeKind) -> Event {
    Event::InputNotice(InputNotice {
        kind,
        pad: None,
        percent: None,
    })
}

pub fn style_of(family: PadFamily) -> ControllerStyle {
    match family {
        PadFamily::Xbox => ControllerStyle::Xbox,
        PadFamily::PlayStation => ControllerStyle::Playstation,
        PadFamily::Nintendo => ControllerStyle::Nintendo,
    }
}

/// `None`: a Guia não é comando sozinha (a Central abre pela combinação).
fn action_of(command: AppCommand) -> Option<InputAction> {
    Some(match command {
        AppCommand::Up => InputAction::Up,
        AppCommand::Down => InputAction::Down,
        AppCommand::Left => InputAction::Left,
        AppCommand::Right => InputAction::Right,
        AppCommand::Accept => InputAction::Accept,
        AppCommand::Back => InputAction::Back,
        AppCommand::Favorite => InputAction::Favorite,
        AppCommand::Search => InputAction::Search,
        AppCommand::PreviousSection => InputAction::PreviousSection,
        AppCommand::NextSection => InputAction::NextSection,
        AppCommand::Menu => InputAction::Menu,
        AppCommand::Guide => return None,
    })
}

impl AppService {
    /// O controle em uso, para `input.controller`.
    pub fn controller(&self) -> &ControllerInfo {
        &self.controller
    }

    /// Guarda o controle em uso e avisa a interface, se mudou.
    pub fn set_controller(&mut self, info: ControllerInfo) {
        if self.controller != info {
            self.controller = info.clone();
            self.emit(Event::InputController(info));
        }
    }

    /// Perfis de botões das configurações do perfil em uso (os mesmos que a interface Slint lê).
    pub fn pad_profiles(&self) -> Profiles {
        match &self.core {
            Some(core) => Profiles::load(|key| core.setting(key).map(str::to_owned)),
            None => Profiles::default(),
        }
    }
}
