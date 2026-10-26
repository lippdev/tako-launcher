//! Regras dos controles que não dependem do SDL nem da interface (INPUT-003, INPUT-004): família para os glifos,
//! estado somado de vários controles e os avisos de conexão. A leitura pelo SDL3 fica em `tako-platform`
//! (`gamepad`); a interface Slint e o núcleo da versão web usam as mesmas regras.

/// Botões por posição (padrão Xbox: A embaixo). Mesmos bits de [`super::GamepadButtons`] (e do C#).
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

/// Estado somado: botões em OU; em cada eixo, o de maior módulo; gatilhos, o mais apertado.
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

/// Aviso de conexão para a tela (INPUT-004).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadNotice {
    Connected,
    Disconnected,
}

/// Quando avisar que o controle caiu ou voltou. Durante o jogo não há aviso (o controle é do jogo); fora dele, só
/// avisa quem estava usando o controle. "Conectado" só aparece depois de o último controle cair com aviso: ligar o
/// primeiro controle não é notícia.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PadNotices {
    lost: bool,
}

impl PadNotices {
    pub fn new() -> Self {
        Self::default()
    }

    /// Um controle conectou (ou voltou).
    pub fn connected(&mut self) -> Option<PadNotice> {
        std::mem::take(&mut self.lost).then_some(PadNotice::Connected)
    }

    /// Um controle saiu. `remaining`: os que ainda estão conectados; `playing`: há um jogo aberto; `using_pad`: a
    /// última entrada foi do controle.
    pub fn disconnected(
        &mut self,
        remaining: usize,
        playing: bool,
        using_pad: bool,
    ) -> Option<PadNotice> {
        if playing || !using_pad {
            return None;
        }
        self.lost = remaining == 0;
        Some(PadNotice::Disconnected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_ors_buttons_and_keeps_strongest_axis() {
        let a = PadState {
            buttons: buttons::A,
            left_x: 0.2,
            left_trigger: 0.3,
            ..Default::default()
        };
        let b = PadState {
            buttons: buttons::B,
            left_x: -0.9,
            left_trigger: 0.1,
            ..Default::default()
        };
        let m = merge([a, b].into_iter());
        assert_eq!(m.buttons, buttons::A | buttons::B);
        assert_eq!(m.left_x, -0.9);
        assert_eq!(m.left_trigger, 0.3);
        assert_eq!(merge(std::iter::empty()), PadState::default());
    }

    #[test]
    fn button_bits_match_the_core_buttons() {
        use super::super::GamepadButtons as B;
        assert_eq!(buttons::A, B::A.0);
        assert_eq!(buttons::B, B::B.0);
        assert_eq!(buttons::BACK, B::VIEW.0);
        assert_eq!(buttons::START, B::START.0);
        assert_eq!(buttons::GUIDE, B::GUIDE.0);
        assert_eq!(buttons::LEFT_SHOULDER, B::LEFT_SHOULDER.0);
        assert_eq!(buttons::DPAD_RIGHT, B::DPAD_RIGHT.0);
    }

    #[test]
    fn losing_the_last_pad_warns_and_the_next_one_says_it_is_back() {
        let mut n = PadNotices::new();
        // O primeiro controle que liga não é notícia.
        assert_eq!(n.connected(), None);
        assert_eq!(
            n.disconnected(0, false, true),
            Some(PadNotice::Disconnected)
        );
        assert_eq!(n.connected(), Some(PadNotice::Connected));
        // Só uma vez por queda.
        assert_eq!(n.connected(), None);
    }

    #[test]
    fn another_pad_still_connected_is_not_a_loss() {
        let mut n = PadNotices::new();
        assert_eq!(
            n.disconnected(1, false, true),
            Some(PadNotice::Disconnected)
        );
        assert_eq!(n.connected(), None);
    }

    #[test]
    fn no_notice_during_the_game_or_for_who_is_not_using_the_pad() {
        let mut n = PadNotices::new();
        assert_eq!(n.disconnected(0, true, true), None);
        assert_eq!(n.connected(), None);
        assert_eq!(n.disconnected(0, false, false), None);
        assert_eq!(n.connected(), None);
    }
}
