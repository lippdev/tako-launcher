//! Teclado físico → comandos do Tako (mesmo mapa do WPF): Enter/Espaço = A, Esc/Backspace = B, F = X,
//! / = Y, Q/E = LB/RB, M = ☰. Em campos de texto, letras e números viram texto.

use super::Cmd;

pub const UP: char = '\u{F700}';
pub const DOWN: char = '\u{F701}';
pub const LEFT: char = '\u{F702}';
pub const RIGHT: char = '\u{F703}';
pub const BACKSPACE: char = '\u{0008}';
pub const TAB: char = '\u{0009}';
pub const ENTER: char = '\u{000a}';
pub const ESCAPE: char = '\u{001b}';
pub const DELETE: char = '\u{007f}';

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

#[cfg(test)]
mod tests {
    use super::*;

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
