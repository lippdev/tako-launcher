//! Entrada (MIG-014): comandos do launcher, interpretação do controle, fusão de fontes e filtro do eco
//! do teclado da Steam. Port de `Input/{GamepadInterpreter,InputEchoFilter,GamepadFusion}.cs`.
//!
//! A fonte das leituras é o SDL3 (em `tako-ui`); aqui só há regras, com relógio monotônico (`Duration`).

pub mod controller_mouse;
pub mod controller_test;
pub mod profiles;
pub mod shortcut;

use std::collections::{BTreeMap, BTreeSet};
use std::ops::{BitAnd, BitOr, BitOrAssign, Not};
use std::time::Duration;

/// Comandos do launcher, independentes do dispositivo (controle, teclado, mouse).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AppCommand {
    Up,
    Down,
    Left,
    Right,
    Accept,
    Back,
    Favorite,
    Search,
    PreviousSection,
    NextSection,
    Menu,
    Guide,
}

impl AppCommand {
    fn is_direction(self) -> bool {
        matches!(
            self,
            AppCommand::Up | AppCommand::Down | AppCommand::Left | AppCommand::Right
        )
    }
}

/// Botões do controle. Os valores são os do C# (`GamepadButtons`), gravados em `ControlCenterChord`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct GamepadButtons(pub u32);

impl GamepadButtons {
    pub const NONE: Self = Self(0);
    pub const DPAD_UP: Self = Self(1 << 0);
    pub const DPAD_DOWN: Self = Self(1 << 1);
    pub const DPAD_LEFT: Self = Self(1 << 2);
    pub const DPAD_RIGHT: Self = Self(1 << 3);
    pub const START: Self = Self(1 << 4);
    pub const VIEW: Self = Self(1 << 5);
    pub const LEFT_SHOULDER: Self = Self(1 << 8);
    pub const RIGHT_SHOULDER: Self = Self(1 << 9);
    pub const GUIDE: Self = Self(1 << 10);
    pub const A: Self = Self(1 << 12);
    pub const B: Self = Self(1 << 13);
    pub const X: Self = Self(1 << 14);
    pub const Y: Self = Self(1 << 15);

    /// Todos os bits de `other` estão aqui (como `HasFlag`).
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl BitOr for GamepadButtons {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GamepadButtons {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GamepadButtons {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl Not for GamepadButtons {
    type Output = Self;
    fn not(self) -> Self {
        Self(!self.0)
    }
}

/// Leitura do controle num instante. Analógico esquerdo em -1..1 (Y positivo = para cima).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GamepadSnapshot {
    pub buttons: GamepadButtons,
    pub left_x: f64,
    pub left_y: f64,
}

impl GamepadSnapshot {
    pub const fn new(buttons: GamepadButtons, left_x: f64, left_y: f64) -> Self {
        GamepadSnapshot {
            buttons,
            left_x,
            left_y,
        }
    }
}

const BUTTON_MAP: [(GamepadButtons, AppCommand); 7] = [
    (GamepadButtons::A, AppCommand::Accept),
    (GamepadButtons::B, AppCommand::Back),
    (GamepadButtons::X, AppCommand::Favorite),
    (GamepadButtons::Y, AppCommand::Search),
    (GamepadButtons::LEFT_SHOULDER, AppCommand::PreviousSection),
    (GamepadButtons::RIGHT_SHOULDER, AppCommand::NextSection),
    (GamepadButtons::START, AppCommand::Menu),
    // Guia e View não são comandos sozinhos: a Central abre pela combinação segurada.
];

/// Botões dos comandos do Tako, remapeáveis (INPUT-001). É sempre uma permutação dos botões do padrão: cada
/// comando tem um botão e nenhum botão faz dois comandos, então ninguém fica sem "aceitar" ou "voltar".
/// View e Guia ficam de fora porque são da Central (a combinação precisa de um deles).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonMap([(GamepadButtons, AppCommand); 7]);

impl Default for ButtonMap {
    fn default() -> Self {
        ButtonMap(BUTTON_MAP)
    }
}

impl ButtonMap {
    /// Comandos remapeáveis, na ordem da tela e do valor gravado.
    pub fn commands() -> impl Iterator<Item = AppCommand> {
        BUTTON_MAP.into_iter().map(|(_, c)| c)
    }

    /// Botões remapeáveis, na mesma ordem de `commands`.
    pub fn buttons() -> impl Iterator<Item = GamepadButtons> {
        BUTTON_MAP.into_iter().map(|(b, _)| b)
    }

    pub fn button(&self, command: AppCommand) -> Option<GamepadButtons> {
        self.0.iter().find(|(_, c)| *c == command).map(|(b, _)| *b)
    }

    pub fn pairs(&self) -> impl Iterator<Item = (GamepadButtons, AppCommand)> + '_ {
        self.0.iter().copied()
    }

    /// Põe `button` no comando; o comando que usava esse botão fica com o botão antigo deste (troca).
    pub fn assign(&mut self, command: AppCommand, button: GamepadButtons) {
        let (Some(old), Some(other)) = (
            self.button(command),
            self.0.iter().position(|(b, _)| *b == button),
        ) else {
            return;
        };
        if let Some(mine) = self.0.iter().position(|(_, c)| *c == command) {
            self.0[other].0 = old;
            self.0[mine].0 = button;
        }
    }

    /// Próximo botão para o comando (`dir` = 1 ou -1), trocando com quem o usava.
    pub fn cycle(&mut self, command: AppCommand, dir: i32) {
        let all: Vec<_> = Self::buttons().collect();
        let Some(i) = self
            .button(command)
            .and_then(|b| all.iter().position(|x| *x == b))
        else {
            return;
        };
        let next = all[(i as i32 + dir).rem_euclid(all.len() as i32) as usize];
        self.assign(command, next);
    }

    /// Botões apertados num controle com este mapa, como se fossem do mapa padrão: o resto do Tako (o
    /// intérprete, as dicas) só conhece o padrão. Direcional, View e Guia passam como estão.
    pub fn to_default(&self, pressed: GamepadButtons) -> GamepadButtons {
        let mapped = Self::buttons().fold(GamepadButtons::NONE, |a, b| a | b);
        let mut out = pressed & !mapped;
        for (button, command) in self.pairs() {
            if pressed.contains(button)
                && let Some(default) = ButtonMap::default().button(command)
            {
                out |= default;
            }
        }
        out
    }

    /// Valor das configurações: o botão de cada comando, na ordem de [`ButtonMap::commands`].
    pub fn serialize(&self) -> String {
        Self::commands()
            .filter_map(|c| self.button(c))
            .map(|b| b.0.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Inválido (vazio, faltando, repetido ou com botão de fora) volta ao padrão.
    pub fn parse(text: Option<&str>) -> ButtonMap {
        let Some(text) = text.filter(|t| !t.trim().is_empty()) else {
            return ButtonMap::default();
        };
        let values: Vec<_> = text
            .split(',')
            .map(|v| v.trim().parse::<u32>().ok().map(GamepadButtons))
            .collect();
        let mut map = BUTTON_MAP;
        let valid = values.len() == map.len()
            && values.iter().enumerate().all(|(i, v)| {
                v.is_some_and(|b| {
                    Self::buttons().any(|x| x == b) && !values[..i].contains(&Some(b))
                })
            });
        if !valid {
            return ButtonMap::default();
        }
        for (slot, b) in map.iter_mut().zip(values.into_iter().flatten()) {
            slot.0 = b;
        }
        ButtonMap(map)
    }
}

/// Converte leituras do controle em comandos: dispara na borda de pressão e repete direções mantidas
/// (380 ms de atraso, depois a cada 110 ms).
#[derive(Debug, Default)]
pub struct GamepadInterpreter {
    /// Próxima repetição de cada comando mantido; `None` = só vale de novo depois de solto.
    held: BTreeMap<AppCommand, Option<Duration>>,
    pressed: BTreeSet<AppCommand>,
    suppress_held: bool,
}

impl GamepadInterpreter {
    pub const REPEAT_DELAY: Duration = Duration::from_millis(380);
    pub const REPEAT_INTERVAL: Duration = Duration::from_millis(110);
    const DEAD_ZONE: f64 = 0.5;

    pub fn new() -> Self {
        Self::default()
    }

    /// Comandos mantidos pelo controle na última leitura (o filtro de eco consulta).
    pub fn pressed(&self) -> &BTreeSet<AppCommand> {
        &self.pressed
    }

    /// Controle desconectado, trocado ou janela sem foco: esquece o estado e ignora o que ainda estiver
    /// pressionado até ser solto (uma direção mantida não volta "andando sozinha").
    pub fn reset(&mut self) {
        self.held.clear();
        self.pressed.clear();
        self.suppress_held = true;
    }

    /// `ignore`: botões que não viram comando agora (o Y de View + Y); só valem de novo depois de soltos.
    pub fn update(
        &mut self,
        snapshot: GamepadSnapshot,
        now: Duration,
        ignore: GamepadButtons,
    ) -> Vec<AppCommand> {
        let b = snapshot.buttons;
        let mut pressed = BTreeSet::new();
        for (button, command) in BUTTON_MAP {
            if b.contains(button) {
                pressed.insert(command);
                if ignore.contains(button) {
                    self.held.entry(command).or_insert(None);
                }
            }
        }
        let dz = Self::DEAD_ZONE;
        if b.contains(GamepadButtons::DPAD_UP) || snapshot.left_y > dz {
            pressed.insert(AppCommand::Up);
        }
        if b.contains(GamepadButtons::DPAD_DOWN) || snapshot.left_y < -dz {
            pressed.insert(AppCommand::Down);
        }
        if b.contains(GamepadButtons::DPAD_LEFT) || snapshot.left_x < -dz {
            pressed.insert(AppCommand::Left);
        }
        if b.contains(GamepadButtons::DPAD_RIGHT) || snapshot.left_x > dz {
            pressed.insert(AppCommand::Right);
        }

        let mut fired = Vec::new();
        if self.suppress_held {
            // Primeira leitura depois do reset: o que já está pressionado só vale depois de solto.
            self.suppress_held = false;
            for c in &pressed {
                self.held.insert(*c, None);
            }
            self.pressed = pressed;
            return fired;
        }

        for &c in &pressed {
            match self.held.get(&c) {
                None => {
                    self.held.insert(c, Some(now + Self::REPEAT_DELAY));
                    fired.push(c);
                }
                Some(Some(next)) if c.is_direction() && now >= *next => {
                    self.held.insert(c, Some(now + Self::REPEAT_INTERVAL));
                    fired.push(c);
                }
                _ => {}
            }
        }
        self.held.retain(|c, _| pressed.contains(c));
        self.pressed = pressed;
        fired
    }
}

/// Evita o comando em dobro quando o mesmo toque chega pelo controle e pelo teclado (a Steam aberta
/// transforma o controle em setas/Enter fora dos jogos). O teclado é ignorado enquanto o controle mantém o
/// mesmo comando ou logo depois de dispará-lo; o controle é ignorado logo depois de o teclado disparar.
#[derive(Debug, Default)]
pub struct InputEchoFilter {
    gamepad: BTreeMap<AppCommand, Duration>,
    keyboard: BTreeMap<AppCommand, Duration>,
}

impl InputEchoFilter {
    /// Distância máxima entre o toque no controle e o eco no teclado (ou o contrário).
    pub const WINDOW: Duration = Duration::from_millis(150);

    pub fn new() -> Self {
        Self::default()
    }

    /// Falso se for eco de uma tecla que acabou de chegar.
    pub fn accept_gamepad(&mut self, command: AppCommand, now: Duration) -> bool {
        if recent(&self.keyboard, command, now) {
            // A tecla já fez o trabalho; a repetição do controle segue normal.
            self.keyboard.remove(&command);
            return false;
        }
        self.gamepad.insert(command, now);
        true
    }

    /// Falso se for eco do controle (`gamepad_held` = comandos mantidos agora).
    pub fn accept_keyboard(
        &mut self,
        command: AppCommand,
        now: Duration,
        gamepad_held: &BTreeSet<AppCommand>,
    ) -> bool {
        if gamepad_held.contains(&command) || recent(&self.gamepad, command, now) {
            return false;
        }
        self.keyboard.insert(command, now);
        true
    }
}

fn recent(last: &BTreeMap<AppCommand, Duration>, command: AppCommand, now: Duration) -> bool {
    last.get(&command)
        .is_some_and(|at| now.saturating_sub(*at) <= InputEchoFilter::WINDOW)
}

/// Junta as leituras de todas as fontes num estado só. Botões são a união (um "solto" de uma fonte não
/// cancela o "pressionado" de outra); em cada eixo vale a fonte mais inclinada. `None` = nenhum controle.
pub fn merge(
    sources: impl IntoIterator<Item = Option<GamepadSnapshot>>,
) -> Option<GamepadSnapshot> {
    let stronger = |a: f64, b: f64| if b.abs() > a.abs() { b } else { a };
    sources.into_iter().flatten().reduce(|m, p| {
        GamepadSnapshot::new(
            m.buttons | p.buttons,
            stronger(m.left_x, p.left_x),
            stronger(m.left_y, p.left_y),
        )
    })
}

/// Bateria (%) a partir da qual o controle sem fio ganha o aviso de bateria fraca.
pub const LOW_BATTERY: i32 = 15;

/// Aviso de bateria fraca: `true` só na primeira leitura baixa desde que o controle conectou (`warned`
/// começa falso a cada conexão). `None` = com fio ou desconhecida.
pub fn low_battery_warning(warned: &mut bool, percent: Option<i32>) -> bool {
    let low = percent.is_some_and(|p| p <= LOW_BATTERY);
    let fire = low && !*warned;
    *warned |= low;
    fire
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    type B = GamepadButtons;
    const IDLE: GamepadSnapshot = GamepadSnapshot::new(B::NONE, 0.0, 0.0);
    const UP: GamepadSnapshot = GamepadSnapshot::new(B::DPAD_UP, 0.0, 0.0);

    fn ms(v: u64) -> Duration {
        Duration::from_millis(v)
    }
    fn pressed(b: B) -> GamepadSnapshot {
        GamepadSnapshot::new(b, 0.0, 0.0)
    }
    fn upd(p: &mut GamepadInterpreter, s: GamepadSnapshot, t: Duration) -> Vec<AppCommand> {
        p.update(s, t, B::NONE)
    }

    #[test]
    fn button_map_swaps_so_every_command_keeps_a_button() {
        let mut map = ButtonMap::default();
        assert_eq!(map.serialize(), "4096,8192,16384,32768,256,512,16");
        // Aceitar no B: voltar fica com o A.
        map.assign(AppCommand::Accept, B::B);
        assert_eq!(map.button(AppCommand::Accept), Some(B::B));
        assert_eq!(map.button(AppCommand::Back), Some(B::A));
        // View é da Central: não entra.
        map.assign(AppCommand::Menu, B::VIEW);
        assert_eq!(map.button(AppCommand::Menu), Some(B::START));
        // Ciclo: do Menu (último) volta ao primeiro botão, que era do Voltar.
        map.cycle(AppCommand::Menu, 1);
        assert_eq!(map.button(AppCommand::Menu), Some(B::A));
        assert_eq!(map.button(AppCommand::Back), Some(B::START));
        map.cycle(AppCommand::Menu, -1);
        assert_eq!(map.button(AppCommand::Menu), Some(B::START));

        let saved = map.serialize();
        assert_eq!(ButtonMap::parse(Some(&saved)), map);
        for bad in [
            "",
            "lixo",
            "4096,8192",
            "4096,4096,16384,32768,256,512,16",
            "32,8192,16384,32768,256,512,16",
            "4096,8192,16384,32768,256,512,16,1",
        ] {
            assert_eq!(ButtonMap::parse(Some(bad)), ButtonMap::default(), "{bad}");
        }
        assert_eq!(ButtonMap::parse(None), ButtonMap::default());
    }

    #[test]
    fn mapped_buttons_reach_the_interpreter_as_the_default_ones() {
        let mut map = ButtonMap::default();
        map.assign(AppCommand::Accept, B::B);
        assert_eq!(map.to_default(B::B | B::DPAD_UP), B::A | B::DPAD_UP);
        assert_eq!(map.to_default(B::A | B::VIEW), B::B | B::VIEW);
        let mut pad = GamepadInterpreter::new();
        let s = pressed(map.to_default(B::B));
        assert_eq!(upd(&mut pad, s, ms(0)), [AppCommand::Accept]);
        let s = pressed(map.to_default(B::A));
        assert_eq!(upd(&mut pad, s, ms(10)), [AppCommand::Back]);
    }

    // GamepadInterpreterTests

    #[test]
    fn buttons_fire_once_per_press() {
        let mut pad = GamepadInterpreter::new();
        assert_eq!(upd(&mut pad, pressed(B::A), ms(0)), [AppCommand::Accept]);
        assert!(upd(&mut pad, pressed(B::A), ms(2000)).is_empty());
        assert!(upd(&mut pad, pressed(B::NONE), ms(3000)).is_empty());
        assert_eq!(upd(&mut pad, pressed(B::A), ms(4000)), [AppCommand::Accept]);
    }

    #[test]
    fn held_direction_repeats_after_delay() {
        let mut pad = GamepadInterpreter::new();
        let d = GamepadInterpreter::REPEAT_DELAY;
        assert_eq!(upd(&mut pad, pressed(B::DPAD_RIGHT), ms(0)).len(), 1);
        assert!(upd(&mut pad, pressed(B::DPAD_RIGHT), ms(300)).is_empty());
        assert_eq!(upd(&mut pad, pressed(B::DPAD_RIGHT), d).len(), 1);
        let next = d + GamepadInterpreter::REPEAT_INTERVAL;
        assert_eq!(upd(&mut pad, pressed(B::DPAD_RIGHT), next).len(), 1);
    }

    #[test]
    fn left_stick_beyond_dead_zone_navigates() {
        for (x, y, expected) in [
            (0.9, 0.0, AppCommand::Right),
            (-0.9, 0.0, AppCommand::Left),
            (0.0, 0.9, AppCommand::Up),
            (0.0, -0.9, AppCommand::Down),
        ] {
            let mut pad = GamepadInterpreter::new();
            let s = GamepadSnapshot::new(B::NONE, x, y);
            assert_eq!(upd(&mut pad, s, ms(0)), [expected]);
        }
    }

    #[test]
    fn small_stick_movement_is_ignored() {
        let s = GamepadSnapshot::new(B::NONE, 0.3, -0.3);
        assert!(upd(&mut GamepadInterpreter::new(), s, ms(0)).is_empty());
    }

    #[test]
    fn view_and_guide_alone_are_not_commands() {
        for b in [B::VIEW, B::GUIDE] {
            assert!(upd(&mut GamepadInterpreter::new(), pressed(b), ms(0)).is_empty());
        }
    }

    #[test]
    fn ignored_button_stays_silent_until_released() {
        let mut pad = GamepadInterpreter::new();
        assert!(pad.update(pressed(B::VIEW | B::Y), ms(0), B::Y).is_empty());
        assert!(upd(&mut pad, pressed(B::Y), ms(100)).is_empty());
        assert!(upd(&mut pad, pressed(B::NONE), ms(200)).is_empty());
        assert_eq!(upd(&mut pad, pressed(B::Y), ms(300)), [AppCommand::Search]);
    }

    #[test]
    fn direction_held_during_disconnect_does_not_keep_moving() {
        let mut pad = GamepadInterpreter::new();
        let d = GamepadInterpreter::REPEAT_DELAY;
        assert_eq!(upd(&mut pad, pressed(B::DPAD_RIGHT), ms(0)).len(), 1);
        assert_eq!(upd(&mut pad, pressed(B::DPAD_RIGHT), d).len(), 1);
        pad.reset();
        let later = ms(5000);
        assert!(upd(&mut pad, pressed(B::DPAD_RIGHT), later).is_empty());
        assert!(upd(&mut pad, pressed(B::DPAD_RIGHT), later + d).is_empty());
        assert!(upd(&mut pad, pressed(B::DPAD_RIGHT), later + ms(10_000)).is_empty());
        assert!(upd(&mut pad, pressed(B::NONE), later + ms(11_000)).is_empty());
        let again = upd(&mut pad, pressed(B::DPAD_RIGHT), later + ms(12_000));
        assert_eq!(again, [AppCommand::Right]);
    }

    #[test]
    fn button_held_while_launcher_was_in_background_does_not_fire() {
        let mut pad = GamepadInterpreter::new();
        pad.reset();
        assert!(upd(&mut pad, pressed(B::A), ms(0)).is_empty());
        assert_eq!(
            upd(&mut pad, pressed(B::A | B::B), ms(16)),
            [AppCommand::Back]
        );
        assert!(upd(&mut pad, pressed(B::NONE), ms(32)).is_empty());
        assert_eq!(upd(&mut pad, pressed(B::A), ms(48)), [AppCommand::Accept]);
    }

    #[test]
    fn reconnected_controller_with_nothing_pressed_works_immediately() {
        let mut pad = GamepadInterpreter::new();
        assert_eq!(upd(&mut pad, pressed(B::DPAD_DOWN), ms(0)).len(), 1);
        pad.reset();
        assert!(upd(&mut pad, pressed(B::NONE), ms(1000)).is_empty());
        assert_eq!(
            upd(&mut pad, pressed(B::DPAD_DOWN), ms(1016)),
            [AppCommand::Down]
        );
    }

    #[test]
    fn low_battery_warns_once_per_connection() {
        let mut warned = false;
        let fired: Vec<_> = [Some(40), None, Some(15), Some(9), Some(20), Some(12)]
            .into_iter()
            .map(|p| low_battery_warning(&mut warned, p))
            .collect();
        assert_eq!(fired, [false, false, true, false, false, false]);
        // Reconectou: avisa de novo.
        let mut warned = false;
        assert!(low_battery_warning(&mut warned, Some(10)));
    }

    // InputFusionTests

    #[test]
    fn short_tap_up_fires_once() {
        let mut pad = GamepadInterpreter::new();
        assert!(upd(&mut pad, IDLE, ms(0)).is_empty());
        assert_eq!(upd(&mut pad, UP, ms(16)), [AppCommand::Up]);
        assert!(upd(&mut pad, IDLE, ms(80)).is_empty());
        assert_eq!(upd(&mut pad, UP, ms(120)), [AppCommand::Up]);
    }

    #[test]
    fn holding_up_repeats_like_the_other_directions() {
        let mut pad = GamepadInterpreter::new();
        let fired: Vec<_> = (0..=1000)
            .step_by(16)
            .flat_map(|t| upd(&mut pad, UP, ms(t)))
            .collect();
        // 1 na borda + repetições a partir de 380 ms a cada 110 ms (até 1 s).
        assert_eq!(fired.iter().filter(|c| **c == AppCommand::Up).count(), 7);
        assert!(pad.pressed().contains(&AppCommand::Up));
    }

    #[test]
    fn stick_resting_slightly_down_does_not_cancel_dpad_up() {
        let s = GamepadSnapshot::new(B::DPAD_UP, 0.05, -0.3);
        assert_eq!(
            upd(&mut GamepadInterpreter::new(), s, ms(0)),
            [AppCommand::Up]
        );
    }

    #[test]
    fn idle_source_does_not_hide_the_other_controller() {
        assert_eq!(
            merge([Some(IDLE), Some(UP)]).map(|m| m.buttons),
            Some(B::DPAD_UP)
        );
    }

    #[test]
    fn release_on_one_source_does_not_cancel_press_on_the_other() {
        let mut pad = GamepadInterpreter::new();
        let frames = [
            [Some(UP), None],
            [Some(UP), Some(UP)],
            [Some(UP), Some(IDLE)],
            [Some(UP), Some(UP)],
            [Some(IDLE), Some(IDLE)],
        ];
        let mut fired = Vec::new();
        for (i, f) in frames.into_iter().enumerate() {
            let m = merge(f).unwrap();
            fired.extend(upd(&mut pad, m, ms(16 * i as u64)));
        }
        assert_eq!(fired, [AppCommand::Up]);
    }

    #[test]
    fn merge_keeps_the_more_tilted_stick_per_axis() {
        let m = merge([
            Some(GamepadSnapshot::new(B::A, 0.9, -0.05)),
            Some(GamepadSnapshot::new(B::B, 0.1, 0.8)),
        ])
        .unwrap();
        assert_eq!(m.buttons, B::A | B::B);
        assert!((m.left_x - 0.9).abs() < 1e-3);
        assert!((m.left_y - 0.8).abs() < 1e-3);
    }

    #[test]
    fn merge_of_nothing_is_no_controller() {
        assert_eq!(merge([None, None]), None);
        assert_eq!(merge([]), None);
    }

    #[test]
    fn steam_keyboard_echo_after_the_gamepad_is_dropped() {
        let mut echo = InputEchoFilter::new();
        let held = BTreeSet::from([AppCommand::Up]);
        assert!(echo.accept_gamepad(AppCommand::Up, ms(0)));
        assert!(!echo.accept_keyboard(AppCommand::Up, ms(10), &held));
        assert!(!echo.accept_keyboard(AppCommand::Up, ms(600), &held));
        assert!(echo.accept_gamepad(AppCommand::Up, ms(380)));
    }

    #[test]
    fn steam_keyboard_echo_before_the_gamepad_wins_once() {
        let mut echo = InputEchoFilter::new();
        assert!(echo.accept_keyboard(AppCommand::Up, ms(0), &BTreeSet::new()));
        assert!(!echo.accept_gamepad(AppCommand::Up, ms(16)));
        assert!(echo.accept_gamepad(AppCommand::Up, ms(396)));
    }

    #[test]
    fn real_keyboard_and_other_commands_pass() {
        let mut echo = InputEchoFilter::new();
        let none = BTreeSet::new();
        assert!(echo.accept_gamepad(AppCommand::Up, ms(0)));
        assert!(echo.accept_keyboard(AppCommand::Down, ms(10), &none));
        assert!(echo.accept_keyboard(AppCommand::Up, ms(400), &none));
        assert!(echo.accept_keyboard(AppCommand::Up, ms(800), &none));
    }

    #[test]
    fn reset_clears_the_held_set() {
        let mut pad = GamepadInterpreter::new();
        upd(&mut pad, UP, ms(0));
        pad.reset();
        assert!(pad.pressed().is_empty());
    }
}
