//! Links externos aceitam apenas destinos conhecidos e IDs da biblioteca, nunca caminhos ou comandos.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    Screen(&'static str),
    Game { id: String, play: bool },
}

pub const MAX_URL: usize = 2048;

/// Decodifica percent-encoding de um identificador e valida caracteres permitidos.
pub fn decode_id(raw: &str) -> Option<String> {
    if raw.len() > MAX_URL || raw.chars().any(char::is_control) {
        return None;
    }
    let mut bytes = raw.bytes();
    let mut decoded = Vec::new();
    while let Some(b) = bytes.next() {
        decoded.push(if b == b'%' {
            let hi = (bytes.next()? as char).to_digit(16)?;
            let lo = (bytes.next()? as char).to_digit(16)?;
            (hi * 16 + lo) as u8
        } else {
            b
        });
    }
    if decoded.is_empty()
        || decoded.len() > 256
        || !decoded
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(b))
    {
        return None;
    }
    String::from_utf8(decoded).ok()
}

pub fn parse(url: &str) -> Option<Link> {
    if url.len() > MAX_URL || url.chars().any(char::is_control) {
        return None;
    }
    let (scheme, target) = url.split_once("://")?;
    if !scheme.eq_ignore_ascii_case("tako") {
        return None;
    }
    match target {
        "home" | "home/" => Some(Link::Screen("home")),
        "library" | "library/" => Some(Link::Screen("library:all")),
        "library/installed" => Some(Link::Screen("library:installed")),
        "library/favorites" => Some(Link::Screen("library:favorites")),
        "settings" | "settings/" => Some(Link::Screen("settings")),
        _ => {
            let (action, id) = target.split_once('/')?;
            if !matches!(action, "game" | "play") {
                return None;
            }
            let id = decode_id(id)?;
            Some(Link::Game {
                id,
                play: action == "play",
            })
        }
    }
}

/// Um argumento de protocolo é um único valor, inclusive quando chamado pelo navegador.
pub fn argument(args: &[String]) -> Option<&str> {
    args.windows(2)
        .find(|w| w[0] == "--open-url")
        .map(|w| w[1].as_str())
        .or_else(|| {
            args.iter()
                .find(|a| {
                    a.get(..7)
                        .is_some_and(|s| s.eq_ignore_ascii_case("tako://"))
                })
                .map(String::as_str)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_screens_and_encoded_game_ids() {
        assert_eq!(
            parse("tako://library/favorites"),
            Some(Link::Screen("library:favorites"))
        );
        assert_eq!(
            parse("TAKO://game/steam%3A570"),
            Some(Link::Game {
                id: "steam:570".into(),
                play: false
            })
        );
        assert_eq!(
            parse("tako://play/manual:abc-123"),
            Some(Link::Game {
                id: "manual:abc-123".into(),
                play: true
            })
        );
    }

    #[test]
    fn external_input_cannot_become_a_path_command_or_another_url() {
        for url in [
            "https://home",
            "tako://settings?x=1",
            "tako://game/",
            "tako://game/../x",
            "tako://game/%00",
            "tako://game/%2fexe",
            "tako://game/%GG",
            "tako://game/%",
            "tako://game/a%22--flag",
            "tako://game/a\n",
            "tako://unknown",
            "tako://play/https://x",
        ] {
            assert!(parse(url).is_none(), "{url:?}");
        }
        assert!(parse(&format!("tako://game/{}", "x".repeat(MAX_URL))).is_none());
    }

    #[test]
    fn test_decode_id() {
        assert_eq!(decode_id("steam%3A570"), Some("steam:570".into()));
        assert_eq!(decode_id("manual:abc-123"), Some("manual:abc-123".into()));
        assert_eq!(decode_id("invalid%20space"), None);
        assert_eq!(decode_id("bad%GG"), None);
        assert_eq!(decode_id(""), None);
    }
}
