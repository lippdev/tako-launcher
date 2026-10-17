//! Links externos aceitam apenas destinos conhecidos e IDs da biblioteca, nunca caminhos ou comandos.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    Screen(&'static str),
    Game {
        id: String,
        play: bool,
    },
    /// Imagem do site do SteamGridDB (ART-002): só o tipo e o número dela; o endereço vem da API.
    SteamGridDb(SteamGridDbAsset),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteamGridDbAsset {
    /// "grid", "hero", "logo" ou "icon", como o site manda.
    pub kind: &'static str,
    pub id: u64,
    /// O site pediu para um jogo fora da Steam (`/nonsteam` do SGDBoop).
    pub nonsteam: bool,
}

pub const MAX_URL: usize = 2048;

/// Esquemas aceitos: o do Tako e o do botão "BOOP" do SteamGridDB (o mesmo do SGDBoop).
pub const SCHEMES: [&str; 2] = ["tako://", "sgdb://"];

pub fn parse(url: &str) -> Option<Link> {
    if url.len() > MAX_URL || url.chars().any(char::is_control) {
        return None;
    }
    let (scheme, target) = url.split_once("://")?;
    if scheme.eq_ignore_ascii_case("sgdb") {
        return steamgriddb(target.strip_prefix("boop/")?);
    }
    if !scheme.eq_ignore_ascii_case("tako") {
        return None;
    }
    if let Some(rest) = target.strip_prefix("steamgriddb/") {
        return steamgriddb(rest);
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
            let mut bytes = id.bytes();
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
            let id = String::from_utf8(decoded).ok()?;
            Some(Link::Game {
                id,
                play: action == "play",
            })
        }
    }
}

/// `<tipo>/<número>[/nonsteam]`. Um só tipo e um só número: o site manda um botão por imagem.
fn steamgriddb(target: &str) -> Option<Link> {
    let mut parts = target.trim_end_matches('/').split('/');
    let first = parts.next()?;
    let kind = ["grid", "hero", "logo", "icon"]
        .into_iter()
        .find(|k| first.eq_ignore_ascii_case(k))?;
    let id = parts.next()?;
    if id.is_empty() || id.len() > 12 || !id.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let nonsteam = match parts.next() {
        None => false,
        Some(m) if m.eq_ignore_ascii_case("nonsteam") => true,
        Some(_) => return None,
    };
    if parts.next().is_some() {
        return None;
    }
    Some(Link::SteamGridDb(SteamGridDbAsset {
        kind,
        id: id.parse().ok()?,
        nonsteam,
    }))
}

pub fn is_link(arg: &str) -> bool {
    SCHEMES.iter().any(|s| {
        arg.get(..s.len())
            .is_some_and(|p| p.eq_ignore_ascii_case(s))
    })
}

/// Um argumento de protocolo é um único valor, inclusive quando chamado pelo navegador.
pub fn argument(args: &[String]) -> Option<&str> {
    args.windows(2)
        .find(|w| w[0] == "--open-url")
        .map(|w| w[1].as_str())
        .or_else(|| args.iter().find(|a| is_link(a)).map(String::as_str))
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
    fn steamgriddb_boop_buttons_and_the_tako_form_name_one_image() {
        let asset =
            |kind, id, nonsteam| Some(Link::SteamGridDb(SteamGridDbAsset { kind, id, nonsteam }));
        assert_eq!(parse("sgdb://boop/grid/80000"), asset("grid", 80000, false));
        assert_eq!(parse("SGDB://boop/hero/12/"), asset("hero", 12, false));
        assert_eq!(parse("sgdb://boop/logo/7/nonsteam"), asset("logo", 7, true));
        assert_eq!(parse("tako://steamgriddb/icon/5"), asset("icon", 5, false));
        for url in [
            "sgdb://boop/grid/",
            "sgdb://boop/grid/1,2",
            "sgdb://boop/grid,hero/1",
            "sgdb://boop/video/1",
            "sgdb://boop/grid/-1",
            "sgdb://boop/grid/1/x",
            "sgdb://boop/grid/1/nonsteam/2",
            "sgdb://boop/grid/9999999999999",
            "sgdb://grid/1",
            "sgdb://boop/grid/1?u=https://x",
            "tako://steamgriddb/grid/abc",
        ] {
            assert!(parse(url).is_none(), "{url:?}");
        }
        let args = ["tako".to_owned(), "sgdb://boop/grid/1".to_owned()];
        assert_eq!(argument(&args), Some("sgdb://boop/grid/1"));
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
}
