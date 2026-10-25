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

/// Pedido do segundo Tako ao que já está aberto: um arquivo na pasta de dados, que o primeiro lê e apaga. Vale para o
/// Tako Slint (o supervisor) e para o núcleo da versão web; sem porta de rede, só o disco do próprio usuário.
pub const ACTIVATE_FILE: &str = "tako.activate";

/// O que o segundo Tako pediu ao que já estava aberto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Activation {
    /// Só vir para a frente.
    Raise,
    /// Executar o link (já validado).
    Open(Link),
}

/// Deixa o pedido para o Tako aberto atender. `url` é o link que o segundo Tako recebeu, se houver. Escreve num
/// arquivo temporário e troca de nome, para o outro lado nunca ler um link pela metade.
pub fn request_activation(root: &std::path::Path, url: Option<&str>) -> std::io::Result<()> {
    let temp = root.join(format!("{ACTIVATE_FILE}.{}", std::process::id()));
    std::fs::write(&temp, url.unwrap_or_default())?;
    std::fs::rename(&temp, root.join(ACTIVATE_FILE)).inspect_err(|_| {
        let _ = std::fs::remove_file(&temp);
    })
}

/// Atende o pedido do segundo Tako, se houver, e o consome. Texto grande demais ou que não é um link do Tako é
/// descartado (o arquivo é de fora do processo, como qualquer entrada externa).
pub fn take_activation(root: &std::path::Path) -> Option<Activation> {
    use std::io::Read;
    let path = root.join(ACTIVATE_FILE);
    let file = std::fs::File::open(&path).ok()?;
    let mut url = String::new();
    let read = file.take(MAX_URL as u64 + 1).read_to_string(&mut url);
    if let Err(e) = std::fs::remove_file(path) {
        tracing::warn!("consumir ativação: {e}");
    }
    if read.is_err() {
        tracing::warn!("pedido de ativação inválido ignorado");
        return None;
    }
    if url.is_empty() {
        return Some(Activation::Raise);
    }
    match parse(&url) {
        Some(link) => Some(Activation::Open(link)),
        None => {
            tracing::warn!("pedido de ativação inválido ignorado");
            None
        }
    }
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
    fn activation_is_bounded_validated_and_consumed_once() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(ACTIVATE_FILE);
        assert_eq!(take_activation(dir.path()), None);
        request_activation(dir.path(), None).unwrap();
        assert_eq!(take_activation(dir.path()), Some(Activation::Raise));
        assert_eq!(take_activation(dir.path()), None, "consumido uma vez");
        request_activation(dir.path(), Some("tako://play/steam:570")).unwrap();
        assert_eq!(
            take_activation(dir.path()),
            Some(Activation::Open(Link::Game {
                id: "steam:570".into(),
                play: true
            }))
        );
        // Outro pedido por cima do anterior: vale o último.
        request_activation(dir.path(), Some("tako://settings")).unwrap();
        request_activation(dir.path(), Some("tako://home")).unwrap();
        assert_eq!(
            take_activation(dir.path()),
            Some(Activation::Open(Link::Screen("home")))
        );
        for url in [
            "https://example.com".to_owned(),
            "tako://game/../x".to_owned(),
            "x".repeat(MAX_URL + 1),
        ] {
            std::fs::write(&file, url).unwrap();
            assert_eq!(take_activation(dir.path()), None);
            assert!(!file.exists(), "o pedido inválido também é apagado");
        }
        // Não deixa arquivos temporários para trás.
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
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
