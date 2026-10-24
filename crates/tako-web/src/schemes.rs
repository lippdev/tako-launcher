//! Esquemas próprios da interface (docs/web/ponte.md §4). Tudo que chega aqui veio de uma URL que a página
//! montou: é dado não confiável. Nada sai da pasta da interface nem do cache de imagens.

use std::path::{Component, Path, PathBuf};

/// `tako-app://app/<caminho>`: o pacote da interface (build do `web/`).
pub const APP_SCHEME: &str = "tako-app";
/// `tako-img://<tipo>?id=<id do jogo>&w=<largura>&h=<altura>`: imagens do cache do núcleo.
pub const IMG_SCHEME: &str = "tako-img";

/// Maior lado pedido; acima disso a imagem nunca aparece maior na tela (4K e meio de folga).
const MAX_SIDE: u32 = 7680;
const MAX_ID_BYTES: usize = 512;

/// Arquivo do pacote da interface para o caminho pedido, ou `None` se ele sair da pasta ou não existir.
/// Caminho vazio é `index.html` (rotas do React também caem nele).
pub fn resolve_app_path(root: &Path, url_path: &str) -> Option<PathBuf> {
    let path = url_path.split(['?', '#']).next().unwrap_or_default();
    let decoded = percent_decode(path.trim_start_matches('/'))?;
    if decoded.contains('\\') || decoded.contains('\0') {
        return None;
    }
    let rel = Path::new(&decoded);
    if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return None;
    }
    let candidate = if decoded.is_empty() {
        root.join("index.html")
    } else {
        root.join(rel)
    };
    // Defesa extra contra links simbólicos que apontem para fora do pacote.
    let root = root.canonicalize().ok()?;
    let file = candidate.canonicalize().ok()?;
    (file.starts_with(&root) && file.is_file()).then_some(file)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    /// Capa 2:3 (600×900 da Steam).
    Cover,
    /// Arte de fundo larga (3840×1240).
    Hero,
    Logo,
    Icon,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageRequest {
    pub kind: ImageKind,
    pub game_id: String,
    /// Tamanho em que a imagem aparece (pixels físicos); o núcleo decodifica nesse tamanho.
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// Lê `tako-img://cover?id=steam%3A570&w=300&h=450`. Qualquer coisa fora do formato é `None` (404).
pub fn parse_image_url(url: &str) -> Option<ImageRequest> {
    let rest = url.strip_prefix(IMG_SCHEME)?.strip_prefix("://")?;
    let (host, query) = rest.split_once('?')?;
    let kind = match host.trim_end_matches('/') {
        "cover" => ImageKind::Cover,
        "hero" => ImageKind::Hero,
        "logo" => ImageKind::Logo,
        "icon" => ImageKind::Icon,
        _ => return None,
    };
    let (mut id, mut width, mut height) = (None, None, None);
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=')?;
        match key {
            "id" => id = Some(percent_decode(value)?),
            "w" => width = Some(side(value)?),
            "h" => height = Some(side(value)?),
            _ => return None,
        }
    }
    let game_id = id?;
    if game_id.is_empty() || game_id.len() > MAX_ID_BYTES || game_id.chars().any(char::is_control) {
        return None;
    }
    Some(ImageRequest {
        kind,
        game_id,
        width,
        height,
    })
}

fn side(value: &str) -> Option<u32> {
    value.parse().ok().filter(|v| (1..=MAX_SIDE).contains(v))
}

/// `%XX` → byte, como o `decodeURIComponent` do navegador; sequência inválida ou UTF-8 quebrado é `None`.
fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = s.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "<html>").unwrap();
        std::fs::create_dir(dir.path().join("assets")).unwrap();
        std::fs::write(dir.path().join("assets/app.js"), "").unwrap();
        dir
    }

    #[test]
    fn serves_files_inside_the_package() {
        let dir = pack();
        let root = dir.path();
        assert!(resolve_app_path(root, "").unwrap().ends_with("index.html"));
        assert!(
            resolve_app_path(root, "/assets/app.js?v=2")
                .unwrap()
                .ends_with("app.js")
        );
        assert!(resolve_app_path(root, "/assets/%61pp.js").is_some());
    }

    #[test]
    fn never_leaves_the_package() {
        let dir = pack();
        let root = dir.path().join("assets");
        for path in [
            "../index.html",
            "/../index.html",
            "%2e%2e/index.html",
            "..%2findex.html",
            "..\\index.html",
            "C:/Windows/win.ini",
            "/etc/passwd",
            "%00",
            "%zz",
            "nao-existe.js",
        ] {
            assert_eq!(resolve_app_path(&root, path), None, "{path}");
        }
    }

    #[test]
    fn parses_image_requests() {
        assert_eq!(
            parse_image_url("tako-img://cover?id=heroic%3Alegendary%3AFortnite&w=300&h=450"),
            Some(ImageRequest {
                kind: ImageKind::Cover,
                game_id: "heroic:legendary:Fortnite".into(),
                width: Some(300),
                height: Some(450),
            })
        );
        assert_eq!(
            parse_image_url("tako-img://hero/?id=steam%3A570").map(|r| r.kind),
            Some(ImageKind::Hero)
        );
    }

    #[test]
    fn rejects_bad_image_requests() {
        for url in [
            "tako-img://cover",
            "tako-img://cover?w=10",
            "tako-img://poster?id=a",
            "tako-img://cover?id=",
            "tako-img://cover?id=a&w=0",
            "tako-img://cover?id=a&w=99999",
            "tako-img://cover?id=a&path=../x",
            "tako-img://cover?id=%0a",
            "tako-img://cover?id=%ff",
            "http://cover?id=a",
        ] {
            assert_eq!(parse_image_url(url), None, "{url}");
        }
    }
}
