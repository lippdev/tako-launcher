//! Atalhos da Central de Controle (UX-005): combinação do controle segurada, captura da combinação e
//! atalho de teclado, com as regras de conflito. Port de `Input/ControlCenterShortcut.cs` (sem o `RowReveal`).
//!
//! Os textos gravados (`ControlCenterChord`, `ControlCenterKey`) são os mesmos do C#.

use super::GamepadButtons as B;
use crate::i18n::{t, tf, tn};
use std::time::Duration;

/// Família do controle, para os nomes dos botões nas dicas e nas mensagens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamepadFamily {
    Xbox,
    PlayStation5,
    PlayStation4,
}

/// Combinação de botões segurada que abre e fecha a Central. Padrão View + Y (Create/Share + △).
pub mod chord {
    use super::*;

    pub const DEFAULT: B = B(B::VIEW.0 | B::Y.0);
    /// Botões sem ação própria no Tako: a combinação precisa de pelo menos um deles.
    pub const FREE: B = B(B::VIEW.0 | B::GUIDE.0);
    /// Botões que o Tako reconhece.
    pub const KNOWN: B = B(B::DPAD_UP.0
        | B::DPAD_DOWN.0
        | B::DPAD_LEFT.0
        | B::DPAD_RIGHT.0
        | B::START.0
        | B::VIEW.0
        | B::LEFT_SHOULDER.0
        | B::RIGHT_SHOULDER.0
        | B::GUIDE.0
        | B::A.0
        | B::B.0
        | B::X.0
        | B::Y.0);
    pub const MAX_BUTTONS: usize = 3;
    pub const HOLD_TIME: Duration = Duration::from_millis(500);

    /// Ordem de exibição: modificadores, ombros, faces e direcional.
    const DISPLAY_ORDER: [B; 13] = [
        B::VIEW,
        B::GUIDE,
        B::START,
        B::LEFT_SHOULDER,
        B::RIGHT_SHOULDER,
        B::A,
        B::B,
        B::X,
        B::Y,
        B::DPAD_UP,
        B::DPAD_DOWN,
        B::DPAD_LEFT,
        B::DPAD_RIGHT,
    ];

    pub fn count(buttons: B) -> usize {
        DISPLAY_ORDER
            .iter()
            .filter(|b| buttons.contains(**b))
            .count()
    }

    pub fn names(buttons: B, family: GamepadFamily) -> Vec<String> {
        DISPLAY_ORDER
            .iter()
            .filter(|b| buttons.contains(**b))
            .map(|b| name(*b, family))
            .collect()
    }

    /// "View + Y", "Create + △".
    pub fn format(buttons: B, family: GamepadFamily) -> String {
        names(buttons, family).join(" + ")
    }

    pub fn name(button: B, family: GamepadFamily) -> String {
        let ps = family != GamepadFamily::Xbox;
        let s = match button {
            B::A if ps => "✕",
            B::B if ps => "○",
            B::X if ps => "□",
            B::Y if ps => "△",
            B::LEFT_SHOULDER if ps => "L1",
            B::RIGHT_SHOULDER if ps => "R1",
            B::START if ps => "Options",
            B::GUIDE if ps => "PS",
            B::VIEW if family == GamepadFamily::PlayStation5 => "Create",
            B::VIEW if family == GamepadFamily::PlayStation4 => "Share",
            B::A => "A",
            B::B => "B",
            B::X => "X",
            B::Y => "Y",
            B::LEFT_SHOULDER => "LB",
            B::RIGHT_SHOULDER => "RB",
            B::START => "Menu",
            B::GUIDE => return t("input.button.guide"),
            B::VIEW => "View",
            B::DPAD_UP => "↑",
            B::DPAD_DOWN => "↓",
            B::DPAD_LEFT => "←",
            B::DPAD_RIGHT => "→",
            other => return other.0.to_string(),
        };
        s.to_owned()
    }

    /// Valor das configurações (número); inválido, vazio ou com botão desconhecido volta ao padrão.
    pub fn parse(text: Option<&str>) -> B {
        text.and_then(|s| s.trim().parse::<i64>().ok())
            .filter(|v| *v > 0 && *v <= u32::MAX as i64)
            .map(|v| B(v as u32))
            .filter(|b| (*b & !KNOWN).is_empty())
            .unwrap_or(DEFAULT)
    }

    pub fn serialize(buttons: B) -> String {
        buttons.0.to_string()
    }
}

/// Reconhece a combinação segurada: dispara uma vez depois de [`chord::HOLD_TIME`] e só volta a disparar
/// depois que algum botão for solto (a mesma pressão não abre e fecha a Central).
#[derive(Debug)]
pub struct ButtonChordDetector {
    chord: B,
    since: Option<Duration>,
    fired: bool,
    /// Fração do tempo de segurar já cumprida (0 a 1), para um indicador.
    pub progress: f64,
}

impl ButtonChordDetector {
    pub fn new(chord: B) -> Self {
        ButtonChordDetector {
            chord: chord & chord::KNOWN,
            since: None,
            fired: false,
            progress: 0.0,
        }
    }

    pub fn chord(&self) -> B {
        self.chord
    }

    /// Combinação nova: só vale depois de soltar tudo.
    pub fn set_chord(&mut self, value: B) {
        self.chord = value & chord::KNOWN;
        self.since = None;
        self.fired = true;
    }

    pub fn update(&mut self, pressed: B, now: Duration) -> bool {
        if self.chord.is_empty() || !pressed.contains(self.chord) {
            self.since = None;
            self.fired = false;
            self.progress = 0.0;
            return false;
        }
        if self.fired {
            return false;
        }
        let since = *self.since.get_or_insert(now);
        let held = now.saturating_sub(since);
        self.progress = (held.as_secs_f64() / chord::HOLD_TIME.as_secs_f64()).clamp(0.0, 1.0);
        if held < chord::HOLD_TIME {
            return false;
        }
        self.fired = true;
        self.progress = 0.0;
        true
    }

    /// Botões da combinação que não viram comando agora: com os "livres" dela (View, Guia) pressionados,
    /// o Y de View + Y não abre a busca.
    pub fn suppressed(&self, pressed: B) -> B {
        let free = self.chord & chord::FREE;
        if !free.is_empty() && pressed.contains(free) {
            self.chord & !free
        } else {
            B::NONE
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChordCaptureState {
    /// Esperando soltar o que já estava pressionado ao abrir a tela.
    Releasing,
    Waiting,
    Holding,
    Captured,
    Cancelled,
}

/// Tela "Segure os botões que quer usar": vale quando o conjunto fica igual por [`ChordCapture::HOLD_TIME`].
/// Tocar B sozinho cancela.
#[derive(Debug)]
pub struct ChordCapture {
    since: Duration,
    pub state: ChordCaptureState,
    pub holding: B,
    pub result: B,
    pub progress: f64,
}

impl Default for ChordCapture {
    fn default() -> Self {
        ChordCapture {
            since: Duration::ZERO,
            state: ChordCaptureState::Releasing,
            holding: B::NONE,
            result: B::NONE,
            progress: 0.0,
        }
    }
}

impl ChordCapture {
    pub const HOLD_TIME: Duration = Duration::from_secs(1);

    pub fn new() -> Self {
        Self::default()
    }

    /// Recomeça depois de uma combinação recusada.
    pub fn restart(&mut self) {
        *self = Self::default();
    }

    pub fn update(&mut self, pressed: B, now: Duration) -> ChordCaptureState {
        use ChordCaptureState::*;
        let pressed = pressed & chord::KNOWN;
        match self.state {
            Captured | Cancelled => return self.state,
            Releasing => {
                if pressed.is_empty() {
                    self.state = Waiting;
                }
                return self.state;
            }
            _ => {}
        }
        if pressed.is_empty() {
            let tapped_back = self.state == Holding && self.holding == B::B;
            self.state = if tapped_back { Cancelled } else { Waiting };
            self.holding = B::NONE;
            self.progress = 0.0;
            return self.state;
        }
        if self.state == Waiting || pressed != self.holding {
            // Botão novo ou um a menos: a contagem recomeça.
            self.state = Holding;
            self.holding = pressed;
            self.since = now;
        }
        let held = now.saturating_sub(self.since);
        self.progress = (held.as_secs_f64() / Self::HOLD_TIME.as_secs_f64()).clamp(0.0, 1.0);
        if held >= Self::HOLD_TIME {
            self.result = self.holding;
            self.state = Captured;
        }
        self.state
    }
}

/// Modificadores com os valores do `ModifierKeys` do WPF (gravados em `ControlCenterKey`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyModifiers(pub u8);

impl KeyModifiers {
    pub const NONE: Self = Self(0);
    pub const ALT: Self = Self(1);
    pub const CTRL: Self = Self(2);
    pub const SHIFT: Self = Self(4);
    pub const WIN: Self = Self(8);

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl std::ops::BitOr for KeyModifiers {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Atalho de teclado, com a tecla pelo nome do `Key` do WPF ("T", "F10", "OemTilde"). Padrão Ctrl+Shift+Tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyShortcut {
    pub modifiers: KeyModifiers,
    pub key: String,
}

impl Default for KeyShortcut {
    fn default() -> Self {
        KeyShortcut::new(KeyModifiers::CTRL | KeyModifiers::SHIFT, "Tab")
    }
}

impl KeyShortcut {
    pub fn new(modifiers: KeyModifiers, key: &str) -> Self {
        KeyShortcut {
            modifiers,
            key: key.to_owned(),
        }
    }

    /// Teclas que só modificam: o atalho espera a tecla de verdade.
    pub fn is_modifier_key(key: &str) -> bool {
        matches!(
            key,
            "LeftCtrl"
                | "RightCtrl"
                | "LeftAlt"
                | "RightAlt"
                | "LeftShift"
                | "RightShift"
                | "LWin"
                | "RWin"
                | "System"
                | "None"
                | "ImeProcessed"
                | "DeadCharProcessed"
        )
    }

    fn join(&self, key: String) -> String {
        let mut parts: Vec<String> = [
            (KeyModifiers::CTRL, "Ctrl"),
            (KeyModifiers::ALT, "Alt"),
            (KeyModifiers::SHIFT, "Shift"),
            (KeyModifiers::WIN, "Win"),
        ]
        .into_iter()
        .filter(|(m, _)| self.modifiers.contains(*m))
        .map(|(_, n)| n.to_owned())
        .collect();
        parts.push(key);
        parts.join("+")
    }

    /// "Ctrl+Shift+Tab", com o nome legível da tecla.
    pub fn display(&self) -> String {
        self.join(Self::key_name(&self.key))
    }

    /// Texto sem tradução, para comparar com as tabelas de conflito (`Enter` e `Return` são a mesma tecla).
    fn canonical(&self) -> String {
        self.join(if self.key == "Enter" {
            "Return".into()
        } else {
            self.key.clone()
        })
    }

    /// Formato guardado: modificadores (número) e tecla, ex.: "6|T".
    pub fn serialize(&self) -> String {
        format!("{}|{}", self.modifiers.0, self.key)
    }

    /// Inválido volta ao padrão.
    pub fn parse(text: Option<&str>) -> KeyShortcut {
        let Some((mods, key)) = text.and_then(|t| t.split_once('|')) else {
            return Self::default();
        };
        match mods.trim().parse::<u8>() {
            Ok(m)
                if m <= 15
                    && !key.is_empty()
                    && !key.contains('|')
                    && !Self::is_modifier_key(key) =>
            {
                KeyShortcut::new(KeyModifiers(m), key)
            }
            _ => Self::default(),
        }
    }

    pub fn key_name(key: &str) -> String {
        let b = key.as_bytes();
        if b.len() == 2 && b[0] == b'D' && b[1].is_ascii_digit() {
            return key[1..].to_owned();
        }
        if let Some(rest) = key.strip_prefix("NumPad") {
            return format!("Num {rest}");
        }
        match key {
            "Return" | "Enter" => "Enter",
            "Escape" => "Esc",
            "Back" => "Backspace",
            "Space" => return t("input.key_name.space"),
            "Prior" | "PageUp" => "Page Up",
            "Next" | "PageDown" => "Page Down",
            "OemTilde" | "Oem3" => "'",
            "OemMinus" => "-",
            "OemPlus" => "=",
            "OemComma" => ",",
            "OemPeriod" => ".",
            _ => key,
        }
        .to_owned()
    }
}

/// Validação de um atalho: erro impede salvar; aviso salva e explica o risco.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShortcutCheck {
    pub error: Option<String>,
    pub warning: Option<String>,
}

impl ShortcutCheck {
    pub const OK: ShortcutCheck = ShortcutCheck {
        error: None,
        warning: None,
    };

    pub fn is_valid(&self) -> bool {
        self.error.is_none()
    }

    fn error(text: String) -> Self {
        ShortcutCheck {
            error: Some(text),
            warning: None,
        }
    }

    fn warning(text: String) -> Self {
        ShortcutCheck {
            error: None,
            warning: Some(text),
        }
    }
}

/// Teclas sozinhas que já são ações do Tako, com a chave do texto da ação.
const TAKO_KEYS: [(&str, &str); 17] = [
    ("Up", "navigate"),
    ("Down", "navigate"),
    ("Left", "navigate"),
    ("Right", "navigate"),
    ("Return", "select"),
    ("Enter", "select"),
    ("Space", "select"),
    ("Escape", "back"),
    ("Back", "back"),
    ("Tab", "navigate"),
    ("F", "favorite"),
    ("Oem2", "search"),
    ("OemQuestion", "search"),
    ("Divide", "search"),
    ("Q", "section"),
    ("E", "section"),
    ("M", "menu"),
];

/// Do sistema: o Tako não consegue (nem deve) tomar.
const RESERVED: [&str; 8] = [
    "Alt+Tab",
    "Alt+F4",
    "Alt+Escape",
    "Alt+Space",
    "Ctrl+Escape",
    "Ctrl+Alt+Delete",
    "Ctrl+Shift+Escape",
    "Ctrl+Alt+Tab",
];

/// Atalhos de outros programas comuns em PCs de jogo, com a chave do texto: aceito, mas avisa.
const KNOWN_CONFLICTS: [(&str, &str); 19] = [
    ("Shift+Tab", "steam_overlay"),
    ("Shift+F1", "ea_overlay"),
    ("Shift+F2", "ubisoft_overlay"),
    ("Shift+F3", "epic_overlay"),
    ("Alt+Z", "nvidia_overlay"),
    ("Alt+R", "gpu_metrics"),
    ("Alt+F1", "nvidia_screenshot"),
    ("Alt+F9", "nvidia_record"),
    ("Alt+F10", "nvidia_replay"),
    ("Alt+Return", "fullscreen"),
    ("Ctrl+Shift+O", "amd_metrics"),
    ("Ctrl+Shift+E", "amd_record"),
    ("Ctrl+Shift+S", "amd_replay_save"),
    ("Ctrl+Shift+R", "amd_replay"),
    ("Ctrl+Shift+I", "amd_screenshot"),
    ("Ctrl+Shift+G", "amd_stream"),
    ("Ctrl+Shift+M", "amd_mic"),
    ("Ctrl+Shift+C", "amd_camera"),
    ("Ctrl+Shift+L", "amd_log"),
];

fn lookup(table: &[(&str, &'static str)], key: &str) -> Option<&'static str> {
    table.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

/// Atalho de teclado para a Central. `tako_shortcuts`: outros atalhos globais do Tako, pelo texto de
/// [`KeyShortcut::display`], com a ação (já traduzida).
pub fn check_keyboard(shortcut: &KeyShortcut, tako_shortcuts: &[(&str, &str)]) -> ShortcutCheck {
    let text = shortcut.display();
    let canonical = shortcut.canonical();
    let m = shortcut.modifiers;
    if KeyShortcut::is_modifier_key(&shortcut.key) {
        return ShortcutCheck::error(t("input.key.modifier_only"));
    }
    if m.contains(KeyModifiers::WIN) {
        return ShortcutCheck::error(t("input.key.win"));
    }
    if RESERVED.contains(&canonical.as_str()) {
        return ShortcutCheck::error(tf("input.key.reserved", &[("shortcut", &text)]));
    }
    if let Some((_, action)) = tako_shortcuts.iter().find(|(k, _)| *k == text) {
        let used = tf(
            "input.key.used_by_tako",
            &[("shortcut", &text), ("action", action)],
        );
        return ShortcutCheck::error(used);
    }
    if m.0 & (KeyModifiers::CTRL.0 | KeyModifiers::ALT.0) == 0 {
        let why = match (
            lookup(&TAKO_KEYS, &shortcut.key),
            lookup(&KNOWN_CONFLICTS, &canonical),
        ) {
            (Some(action), _) if m == KeyModifiers::NONE => {
                let name = KeyShortcut::key_name(&shortcut.key);
                let action = t(&format!("input.action.{action}"));
                tf(
                    "input.key.used_by_tako",
                    &[("shortcut", &name), ("action", &action)],
                )
            }
            (_, Some(conflict)) => format!("{text} {}", t(&format!("input.conflict.{conflict}"))),
            _ => tf("input.key.used_by_games", &[("shortcut", &text)]),
        };
        return ShortcutCheck::error(tf("input.key.add_modifier", &[("reason", &why)]));
    }
    if let Some(conflict) = lookup(&KNOWN_CONFLICTS, &canonical) {
        let what = t(&format!("input.conflict.{conflict}"));
        return ShortcutCheck::warning(tf(
            "input.key.also",
            &[("shortcut", &text), ("what", &what)],
        ));
    }
    if m.contains(KeyModifiers::CTRL)
        && m.contains(KeyModifiers::ALT)
        && is_character_key(&shortcut.key)
    {
        return ShortcutCheck::warning(t("input.key.altgr"));
    }
    ShortcutCheck::OK
}

/// Combinação do controle para a Central (sempre segurada por meio segundo).
pub fn check_chord(buttons: B, family: GamepadFamily) -> ShortcutCheck {
    let buttons = buttons & chord::KNOWN;
    let count = chord::count(buttons);
    if count == 0 {
        return ShortcutCheck::error(t("input.chord.empty"));
    }
    if count > chord::MAX_BUTTONS {
        let max = chord::MAX_BUTTONS.to_string();
        return ShortcutCheck::error(tf("input.chord.too_many", &[("max", &max)]));
    }
    let text = chord::format(buttons, family);
    if (buttons & chord::FREE).is_empty() {
        let view = chord::name(B::VIEW, family);
        let guide = chord::name(B::GUIDE, family);
        let free = tf("input.chord.or", &[("a", &view), ("b", &guide)]);
        let args = [("buttons", text.as_str()), ("free", free.as_str())];
        return ShortcutCheck::error(tn("input.chord.navigates", count as i64, &args));
    }
    if buttons == B::GUIDE {
        let key = if family == GamepadFamily::Xbox {
            "input.chord.guide_xbox"
        } else {
            "input.chord.guide_ps"
        };
        return ShortcutCheck::warning(t(key));
    }
    if buttons == B::VIEW {
        return ShortcutCheck::warning(tf("input.chord.view_alone", &[("button", &text)]));
    }
    ShortcutCheck::OK
}

fn is_character_key(key: &str) -> bool {
    let b = key.as_bytes();
    (b.len() == 1 && b[0].is_ascii_alphabetic())
        || (b.len() == 2 && b[0] == b'D' && b[1].is_ascii_digit())
        || key.starts_with("Oem")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::{AppCommand, GamepadInterpreter, GamepadSnapshot, InputEchoFilter};
    use super::*;

    const HOLD: Duration = chord::HOLD_TIME;
    fn ms(v: u64) -> Duration {
        Duration::from_millis(v)
    }

    #[test]
    fn default_chord_is_view_plus_y_named_per_controller() {
        assert_eq!(chord::DEFAULT, B::VIEW | B::Y);
        assert_eq!(
            chord::format(chord::DEFAULT, GamepadFamily::Xbox),
            "View + Y"
        );
        assert_eq!(
            chord::format(chord::DEFAULT, GamepadFamily::PlayStation5),
            "Create + △"
        );
        assert_eq!(
            chord::format(chord::DEFAULT, GamepadFamily::PlayStation4),
            "Share + △"
        );
    }

    #[test]
    fn chord_fires_once_after_holding_half_a_second() {
        let mut c = ButtonChordDetector::new(chord::DEFAULT);
        let both = B::VIEW | B::Y;
        assert!(!c.update(both, ms(0)));
        assert!(!c.update(both, ms(400)));
        assert!(c.update(both, HOLD));
        assert!(!c.update(both, ms(2000)));
        assert!(!c.update(B::VIEW, ms(2100)));
        assert!(!c.update(both, ms(2200)));
        assert!(c.update(both, ms(2200) + HOLD));
    }

    #[test]
    fn quick_press_does_not_open() {
        let mut c = ButtonChordDetector::new(chord::DEFAULT);
        let both = B::VIEW | B::Y;
        assert!(!c.update(both, ms(0)));
        assert!(!c.update(both, ms(300)));
        assert!(!c.update(B::NONE, ms(350)));
        assert!(!c.update(both, ms(400)));
        assert!(!c.update(both, ms(800)));
    }

    #[test]
    fn extra_buttons_do_not_block_the_chord() {
        let mut c = ButtonChordDetector::new(chord::DEFAULT);
        let with_stick = B::VIEW | B::Y | B::DPAD_UP;
        assert!(!c.update(with_stick, ms(0)));
        assert!(c.update(with_stick, HOLD));
    }

    #[test]
    fn changing_the_chord_waits_for_release() {
        let mut c = ButtonChordDetector::new(chord::DEFAULT);
        let new = B::VIEW | B::X;
        c.set_chord(new);
        assert!(!c.update(new, ms(0)));
        assert!(!c.update(new, ms(2000)));
        assert!(!c.update(B::NONE, ms(2100)));
        assert!(!c.update(new, ms(2200)));
        assert!(c.update(new, ms(2200) + HOLD));
    }

    #[test]
    fn free_buttons_held_suppress_the_rest_of_the_chord() {
        let c = ButtonChordDetector::new(chord::DEFAULT);
        assert_eq!(c.suppressed(B::VIEW), B::Y);
        assert_eq!(c.suppressed(B::VIEW | B::Y), B::Y);
        assert_eq!(c.suppressed(B::Y), B::NONE);
    }

    #[test]
    fn chord_buttons_still_count_as_held_for_the_steam_keyboard_echo_filter() {
        let mut pad = GamepadInterpreter::new();
        let mut echo = InputEchoFilter::new();
        let c = ButtonChordDetector::new(chord::DEFAULT);
        let both = B::VIEW | B::Y;
        let fired = pad.update(
            GamepadSnapshot::new(both, 0.0, 0.0),
            ms(0),
            c.suppressed(both),
        );
        assert!(fired.is_empty());
        assert!(pad.pressed().contains(&AppCommand::Search));
        assert!(!echo.accept_keyboard(AppCommand::Search, ms(20), pad.pressed()));
    }

    #[test]
    fn capture_ignores_buttons_held_when_it_opens_and_takes_the_stable_set() {
        use ChordCaptureState::*;
        let mut c = ChordCapture::new();
        assert_eq!(c.update(B::A, ms(0)), Releasing);
        assert_eq!(c.update(B::A, ms(2000)), Releasing);
        assert_eq!(c.update(B::NONE, ms(2100)), Waiting);
        assert_eq!(c.update(B::VIEW, ms(2200)), Holding);
        assert_eq!(c.update(B::VIEW | B::X, ms(2500)), Holding);
        assert_eq!(c.update(B::VIEW | B::X, ms(3400)), Holding);
        assert_eq!(c.update(B::VIEW | B::X, ms(3500)), Captured);
        assert_eq!(c.result, B::VIEW | B::X);
    }

    #[test]
    fn tapping_back_cancels_the_capture_but_holding_it_captures() {
        let mut tap = ChordCapture::new();
        tap.update(B::NONE, ms(0));
        tap.update(B::B, ms(100));
        assert_eq!(tap.update(B::NONE, ms(300)), ChordCaptureState::Cancelled);

        let mut held = ChordCapture::new();
        held.update(B::NONE, ms(0));
        held.update(B::B, ms(100));
        assert_eq!(held.update(B::B, ms(1100)), ChordCaptureState::Captured);
        assert_eq!(held.result, B::B);
        held.restart();
        assert_eq!(held.state, ChordCaptureState::Releasing);
    }

    #[test]
    fn chord_rules_reject_combinations_that_already_navigate() {
        for (c, valid) in [
            (B::VIEW | B::Y, true),
            (B::GUIDE, true),
            (B::VIEW | B::DPAD_UP, true),
            (B::A, false),
            (B::LEFT_SHOULDER | B::RIGHT_SHOULDER, false),
            (B::START | B::Y, false),
            (B::VIEW | B::A | B::B | B::X, false),
            (B::NONE, false),
        ] {
            assert_eq!(
                check_chord(c, GamepadFamily::Xbox).is_valid(),
                valid,
                "{c:?}"
            );
        }
    }

    #[test]
    fn chord_rules_explain_with_the_controller_names() {
        let c = check_chord(
            B::LEFT_SHOULDER | B::RIGHT_SHOULDER,
            GamepadFamily::PlayStation5,
        );
        assert_eq!(
            c.error.as_deref(),
            Some("L1 + R1 já servem para navegar no Tako · inclua Create ou PS")
        );
        let one = check_chord(B::A, GamepadFamily::Xbox);
        assert_eq!(
            one.error.as_deref(),
            Some("A já serve para navegar no Tako · inclua View ou Guia")
        );
        let guide = check_chord(B::GUIDE, GamepadFamily::Xbox).warning.unwrap();
        assert!(guide.contains("Barra de Jogo"));
        assert!(check_chord(B::VIEW, GamepadFamily::Xbox).warning.is_some());
        assert_eq!(
            check_chord(chord::DEFAULT, GamepadFamily::Xbox),
            ShortcutCheck::OK
        );
    }

    #[test]
    fn default_keyboard_shortcut_has_no_known_conflict() {
        assert_eq!(KeyShortcut::default().display(), "Ctrl+Shift+Tab");
        assert_eq!(
            check_keyboard(&KeyShortcut::default(), &[]),
            ShortcutCheck::OK
        );
    }

    #[test]
    fn keyboard_rules_reject_tako_and_windows_shortcuts() {
        type M = KeyModifiers;
        for (m, k) in [
            (M::SHIFT, "Tab"),
            (M::NONE, "F"),
            (M::NONE, "F10"),
            (M::ALT, "Tab"),
            (M::ALT, "F4"),
            (M::CTRL | M::SHIFT, "Escape"),
            (M::WIN, "G"),
            (M::CTRL, "LeftShift"),
            (M::ALT, "Space"),
        ] {
            assert!(
                !check_keyboard(&KeyShortcut::new(m, k), &[]).is_valid(),
                "{k}"
            );
        }
    }

    #[test]
    fn keyboard_rules_explain_the_conflict() {
        let shift_tab = check_keyboard(&KeyShortcut::new(KeyModifiers::SHIFT, "Tab"), &[]);
        assert_eq!(
            shift_tab.error.as_deref(),
            Some("Shift+Tab abre a sobreposição da Steam e do GOG Galaxy · use Ctrl ou Alt junto")
        );
        let f = check_keyboard(&KeyShortcut::new(KeyModifiers::NONE, "F"), &[]);
        assert_eq!(
            f.error.as_deref(),
            Some("F já serve para favoritar no Tako · use Ctrl ou Alt junto")
        );
    }

    #[test]
    fn another_tako_shortcut_cannot_be_reused() {
        let tako = [("Ctrl+Alt+Shift+D", "voltar as telas para a mesa")];
        let m = KeyModifiers::CTRL | KeyModifiers::ALT | KeyModifiers::SHIFT;
        let check = check_keyboard(&KeyShortcut::new(m, "D"), &tako);
        assert_eq!(
            check.error.as_deref(),
            Some("Ctrl+Alt+Shift+D já serve para voltar as telas para a mesa no Tako")
        );
    }

    #[test]
    fn keyboard_rules_warn_about_known_conflicts() {
        type M = KeyModifiers;
        for (m, k) in [
            (M::CTRL | M::SHIFT, "O"),
            (M::ALT, "Z"),
            (M::ALT, "Return"),
            (M::ALT, "Enter"),
            (M::CTRL | M::ALT, "Q"),
        ] {
            let check = check_keyboard(&KeyShortcut::new(m, k), &[]);
            assert!(check.is_valid() && check.warning.is_some(), "{k}");
        }
    }

    #[test]
    fn shortcuts_round_trip_through_settings_text() {
        let key = KeyShortcut::new(KeyModifiers::CTRL | KeyModifiers::ALT, "F10");
        assert_eq!(KeyShortcut::parse(Some(&key.serialize())), key);
        assert_eq!(key.serialize(), "3|F10");
        for bad in [
            Some("lixo"),
            None,
            Some("16|T"),
            Some("2|"),
            Some("2|LeftCtrl"),
            Some("x|T"),
        ] {
            assert_eq!(KeyShortcut::parse(bad), KeyShortcut::default(), "{bad:?}");
        }

        let c = B::VIEW | B::X;
        assert_eq!(chord::parse(Some(&chord::serialize(c))), c);
        assert_eq!(chord::serialize(chord::DEFAULT), "32800");
        for bad in [
            Some("0"),
            Some("64"),
            Some("-1"),
            Some("abc"),
            None,
            Some("99999999999"),
        ] {
            assert_eq!(chord::parse(bad), chord::DEFAULT, "{bad:?}");
        }
    }

    #[test]
    fn key_names_are_readable() {
        for (k, e) in [
            ("D5", "5"),
            ("NumPad3", "Num 3"),
            ("F10", "F10"),
            ("Escape", "Esc"),
        ] {
            assert_eq!(KeyShortcut::key_name(k), e);
        }
    }
}
