//! Teclado físico → comandos do Tako (mesmo mapa do WPF): Enter/Espaço = A, Esc/Backspace = B, F = X,
//! / = Y, Q/E = LB/RB, M = ☰. Em campos de texto, letras e números viram texto.

use super::Cmd;
use tako_core::input::shortcut::{KeyModifiers, KeyShortcut};

pub const UP: char = '\u{F700}';
pub const DOWN: char = '\u{F701}';
pub const LEFT: char = '\u{F702}';
pub const RIGHT: char = '\u{F703}';
pub const BACKSPACE: char = '\u{0008}';
pub const TAB: char = '\u{0009}';
pub const ENTER: char = '\u{000a}';
pub const ESCAPE: char = '\u{001b}';
pub const DELETE: char = '\u{007f}';
/// F11 (texto da tecla no Slint): tela cheia no modo desktop.
pub const F11: &str = "\u{F70E}";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyAction {
    Cmd(Cmd),
    /// Texto digitado (só vale em campos de texto).
    Text(String),
    Erase,
    Ignore,
}

/// `text` é o texto do evento do Slint; `typing` = há um campo de texto aberto.
pub fn map(text: &str, ctrl: bool, alt: bool, typing: bool) -> KeyAction {
    let mut chars = text.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        // Texto com mais de um caractere (colar, IME): só em campos de texto.
        return if typing && !text.is_empty() {
            KeyAction::Text(text.to_owned())
        } else {
            KeyAction::Ignore
        };
    };
    match c {
        UP => return KeyAction::Cmd(Cmd::Up),
        DOWN => return KeyAction::Cmd(Cmd::Down),
        LEFT => return KeyAction::Cmd(Cmd::Left),
        RIGHT => return KeyAction::Cmd(Cmd::Right),
        ENTER => return KeyAction::Cmd(Cmd::Accept),
        ESCAPE => return KeyAction::Cmd(Cmd::Back),
        BACKSPACE | DELETE => {
            return if typing {
                KeyAction::Erase
            } else {
                KeyAction::Cmd(Cmd::Back)
            };
        }
        TAB => return KeyAction::Ignore,
        _ => {}
    }
    if ctrl || alt {
        return KeyAction::Ignore;
    }
    if typing {
        return if c.is_control() || ('\u{F700}'..='\u{F8FF}').contains(&c) {
            KeyAction::Ignore
        } else {
            KeyAction::Text(c.to_string())
        };
    }
    match c.to_ascii_lowercase() {
        ' ' => KeyAction::Cmd(Cmd::Accept),
        'f' => KeyAction::Cmd(Cmd::Favorite),
        '/' => KeyAction::Cmd(Cmd::Search),
        'q' => KeyAction::Cmd(Cmd::PrevSection),
        'e' => KeyAction::Cmd(Cmd::NextSection),
        'm' => KeyAction::Cmd(Cmd::Menu),
        _ => KeyAction::Ignore,
    }
}

/// Texto da tecla do Slint → nome do `Key` do WPF, como o atalho da Central guarda ("T", "D5", "F10", "Tab").
/// `None` = tecla sem nome no atalho (Caps Lock, Menu, letras acentuadas...).
/// A tecla é o atalho de teclado da Central guardado nas configurações (Ctrl+Shift+Tab por padrão): dentro do Tako
/// ele abre a Central, como o botão Xbox/PS; durante o jogo quem atende é o atalho global do supervisor.
pub fn is_control_center(setting: Option<&str>, text: &str, mods: KeyModifiers) -> bool {
    let shortcut = KeyShortcut::parse(setting);
    shortcut.modifiers == mods && wpf_key(text).is_some_and(|k| k == shortcut.key)
}

pub fn wpf_key(text: &str) -> Option<String> {
    let mut chars = text.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        return None;
    };
    if let Some(n) = (c as u32).checked_sub(0xF704).filter(|n| *n < 24) {
        return Some(format!("F{}", n + 1));
    }
    if c.is_ascii_alphabetic() {
        return Some(c.to_ascii_uppercase().to_string());
    }
    if c.is_ascii_digit() {
        return Some(format!("D{c}"));
    }
    // shortcut: Shift+número pelo layout US/ABNT2 (o Slint entrega o símbolo); outros layouts caem em `None`.
    if let Some(d) = ")!@#$%¨&*(".chars().position(|x| x == c) {
        return Some(format!("D{d}"));
    }
    Some(
        match c {
            BACKSPACE => "Back",
            TAB | '\u{0019}' => "Tab",
            ENTER => "Return",
            ESCAPE => "Escape",
            DELETE => "Delete",
            ' ' => "Space",
            '\u{0010}' => "LeftShift",
            '\u{0011}' => "LeftCtrl",
            '\u{0012}' => "LeftAlt",
            '\u{0013}' => "RightAlt",
            '\u{0015}' => "RightShift",
            '\u{0016}' => "RightCtrl",
            '\u{0017}' => "LWin",
            '\u{0018}' => "RWin",
            UP => "Up",
            DOWN => "Down",
            LEFT => "Left",
            RIGHT => "Right",
            '\u{F727}' => "Insert",
            '\u{F729}' => "Home",
            '\u{F72B}' => "End",
            '\u{F72C}' => "PageUp",
            '\u{F72D}' => "PageDown",
            '\u{F730}' => "Pause",
            // A tecla à esquerda do 1 (' no ABNT2, ` no US); as outras Oem ficam de fora: o atalho global do
            // Windows não as conhece (`KeyShortcut::virtual_key`).
            '`' | '~' | '\'' | '"' => "OemTilde",
            _ => return None,
        }
        .to_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_control_center_shortcut_opens_the_central_inside_tako() {
        let cs = KeyModifiers::CTRL | KeyModifiers::SHIFT;
        // Padrão Ctrl+Shift+Tab: o Slint manda Tab ou Backtab (Shift+Tab), conforme o SO.
        assert!(is_control_center(None, "\t", cs));
        assert!(is_control_center(None, "\u{19}", cs));
        assert!(!is_control_center(None, "\u{19}", KeyModifiers::SHIFT));
        assert!(!is_control_center(None, "t", cs));
        let custom = KeyShortcut::new(KeyModifiers::ALT, "F10").serialize();
        assert!(is_control_center(
            Some(&custom),
            "\u{F70D}",
            KeyModifiers::ALT
        ));
        assert!(!is_control_center(Some(&custom), "\t", cs));
    }

    #[test]
    fn slint_keys_become_wpf_key_names() {
        for (text, name) in [
            ("t", "T"),
            ("T", "T"),
            ("5", "D5"),
            ("%", "D5"),
            ("\u{F70D}", "F10"),
            ("\u{F71B}", "F24"),
            ("\t", "Tab"),
            ("\u{19}", "Tab"),
            ("\n", "Return"),
            ("\u{1b}", "Escape"),
            (" ", "Space"),
            ("\u{F700}", "Up"),
            ("\u{F72C}", "PageUp"),
            ("\u{11}", "LeftCtrl"),
            ("\u{15}", "RightShift"),
            ("'", "OemTilde"),
        ] {
            assert_eq!(wpf_key(text).as_deref(), Some(name), "{text:?}");
        }
        for text in ["", "ab", "é", "-", "\u{14}", "\u{F735}"] {
            assert_eq!(wpf_key(text), None, "{text:?}");
        }
    }

    #[test]
    fn navigation_keys() {
        assert_eq!(
            map("\u{F700}", false, false, false),
            KeyAction::Cmd(Cmd::Up)
        );
        assert_eq!(map("\n", false, false, true), KeyAction::Cmd(Cmd::Accept));
        assert_eq!(
            map("\u{1b}", false, false, false),
            KeyAction::Cmd(Cmd::Back)
        );
        assert_eq!(map("\u{8}", false, false, false), KeyAction::Cmd(Cmd::Back));
        assert_eq!(map("\u{8}", false, false, true), KeyAction::Erase);
    }

    #[test]
    fn letters_are_shortcuts_or_text() {
        assert_eq!(map("F", false, false, false), KeyAction::Cmd(Cmd::Favorite));
        assert_eq!(
            map("q", false, false, false),
            KeyAction::Cmd(Cmd::PrevSection)
        );
        assert_eq!(map("q", false, false, true), KeyAction::Text("q".into()));
        assert_eq!(map("é", false, false, true), KeyAction::Text("é".into()));
        assert_eq!(map("x", true, false, false), KeyAction::Ignore);
        assert_eq!(map("\u{F704}", false, false, true), KeyAction::Ignore);
    }
}
