//! Fontes de download `.json` que o próprio usuário adiciona (DL-001, ADR-0028 §1), no formato de fonte .json:
//! `{ "name": "…", "downloads": [ { "title": "…", "uris": ["magnet:?…"], "uploadDate": "…", "fileSize": "…" } ] }`.
//!
//! Parser puro, sem rede. A fonte não é confiável: limites de tamanho, item quebrado é descartado (com contagem)
//! sem derrubar a fonte, e só `magnet:` com infohash ou `https://…/*.torrent` passam.

use serde_json::Value;

/// Tamanho máximo do arquivo da fonte.
pub const MAX_SOURCE_BYTES: usize = 5 * 1024 * 1024;
/// Itens lidos por fonte; o resto é descartado.
pub const MAX_ITEMS: usize = 20_000;
pub const MAX_TITLE_CHARS: usize = 200;
pub const MAX_URIS: usize = 8;
pub const MAX_URI_BYTES: usize = 4 * 1024;
/// Nome da fonte, cortado (só aparece na tela).
const MAX_NAME_CHARS: usize = 120;
/// Campos de texto livre (data, tamanho): só dica, cortados.
const MAX_HINT_CHARS: usize = 40;

#[derive(Debug, PartialEq, Eq)]
pub enum SourceError {
    TooLarge,
    InvalidJson,
    MissingDownloads,
}

impl std::error::Error for SourceError {}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceError::TooLarge => write!(f, "{}", crate::i18n::t("downloads.error.source_too_large")),
            SourceError::InvalidJson => write!(f, "{}", crate::i18n::t("downloads.error.invalid_json")),
            SourceError::MissingDownloads => write!(f, "{}", crate::i18n::t("downloads.error.missing_downloads")),
        }
    }
}

/// Um item da fonte. `uris` já validadas; o texto vem da fonte e é mostrado só como texto simples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceItem {
    pub title: String,
    pub uris: Vec<String>,
    /// Texto livre da fonte ("2024-05-01T…"), só dica.
    pub upload_date: String,
    /// Texto livre da fonte ("12.4 GB"): só dica; o tamanho real vem do torrent.
    pub file_size: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub name: String,
    pub items: Vec<SourceItem>,
    /// Itens descartados (quebrados, sem URI válida ou além do limite).
    pub dropped: usize,
}

/// Lê a fonte. Erro só quando nada se aproveita; item ruim vira `dropped`.
pub fn parse(bytes: &[u8]) -> Result<Source, SourceError> {
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err(SourceError::TooLarge);
    }
    let root: Value = serde_json::from_slice(bytes).map_err(|_| SourceError::InvalidJson)?;
    let list = root
        .get("downloads")
        .and_then(Value::as_array)
        .ok_or(SourceError::MissingDownloads)?;
    let name = root
        .get("name")
        .and_then(Value::as_str)
        .map(|n| clean_text(n, MAX_NAME_CHARS))
        .unwrap_or_default();
    let mut items = Vec::new();
    let mut dropped = list.len().saturating_sub(MAX_ITEMS);
    for v in list.iter().take(MAX_ITEMS) {
        match item(v) {
            Some(i) => items.push(i),
            None => dropped += 1,
        }
    }
    if dropped > 0 {
        // Só a contagem: o conteúdo da fonte não vai para o log.
        tracing::info!(
            dropped,
            kept = items.len(),
            "fonte de download: itens descartados"
        );
    }
    Ok(Source {
        name,
        items,
        dropped,
    })
}

fn item(v: &Value) -> Option<SourceItem> {
    let raw = v.get("title")?.as_str()?;
    if raw.chars().count() > MAX_TITLE_CHARS {
        return None;
    }
    let title = clean_text(raw, MAX_TITLE_CHARS);
    if title.is_empty() {
        return None;
    }
    let uris: Vec<String> = v
        .get("uris")?
        .as_array()?
        .iter()
        .take(MAX_URIS)
        .filter_map(Value::as_str)
        .filter_map(valid_uri)
        .collect();
    if uris.is_empty() {
        return None;
    }
    let hint = |k: &str| {
        v.get(k)
            .and_then(Value::as_str)
            .map(|s| clean_text(s, MAX_HINT_CHARS))
            .unwrap_or_default()
    };
    Some(SourceItem {
        title,
        uris,
        upload_date: hint("uploadDate"),
        file_size: hint("fileSize"),
    })
}

/// Sem caracteres de controle nem bidi (texto de fonte não pode esconder nada na tela), espaços juntos, cortado.
fn clean_text(s: &str, max: usize) -> String {
    let s: String = s
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}') {
                ' '
            } else {
                c
            }
        })
        .collect();
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max)
        .collect()
}

/// `magnet:?…xt=urn:btih:<40 hex | 32 base32>` ou `https://host/…/*.torrent`. Qualquer outro esquema sai.
pub fn valid_uri(uri: &str) -> Option<String> {
    let uri = uri.trim();
    if uri.is_empty()
        || uri.len() > MAX_URI_BYTES
        || uri.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return None;
    }
    let lower = uri.to_ascii_lowercase();
    if let Some(query) = lower.strip_prefix("magnet:?") {
        let ok = query.split('&').any(|p| {
            p.strip_prefix("xt=urn:btih:").is_some_and(|h| {
                (h.len() == 40 && h.bytes().all(|b| b.is_ascii_hexdigit()))
                    || (h.len() == 32 && h.bytes().all(|b| matches!(b, b'a'..=b'z' | b'2'..=b'7')))
            })
        });
        return ok.then(|| uri.to_owned());
    }
    let rest = lower.strip_prefix("https://")?;
    let (host, path) = rest.split_once('/')?;
    let path = path.split(['?', '#']).next().unwrap_or_default();
    let host_ok = !host.is_empty() && !host.contains('@');
    (host_ok && path.ends_with(".torrent")).then(|| uri.to_owned())
}

/// Infohash do magnet em minúsculas (chave para "já está baixando"); `None` em `.torrent` por https.
pub fn infohash(uri: &str) -> Option<String> {
    let lower = uri.to_ascii_lowercase();
    lower
        .strip_prefix("magnet:?")?
        .split('&')
        .find_map(|p| p.strip_prefix("xt=urn:btih:"))
        .map(str::to_owned)
}

/// Arquivo do cache da fonte em `dir` (nome pelo hash da URL: nada da URL vira caminho).
pub fn cache_file(dir: &std::path::Path, url: &str) -> std::path::PathBuf {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(url.as_bytes());
    let name: String = hash.iter().map(|b| format!("{b:02x}")).collect();
    dir.join(format!("{name}.json"))
}

/// Fonte do cache em disco; `None` se faltar ou estiver quebrada.
pub fn read_cache(dir: &std::path::Path, url: &str) -> Option<Source> {
    let bytes = std::fs::read(cache_file(dir, url)).ok()?;
    parse(&bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Só magnets fictícios (ADR-0028 §8).
    const HASH: &str = "0123456789abcdef0123456789abcdef01234567";

    fn magnet() -> String {
        format!("magnet:?xt=urn:btih:{HASH}&dn=exemplo")
    }

    fn source(items: &str) -> Vec<u8> {
        format!(r#"{{"name":"Minha fonte","downloads":[{items}]}}"#).into_bytes()
    }

    #[test]
    fn reads_the_source_format_and_ignores_extra_fields() {
        let s = parse(&source(&format!(
            r#"{{"title":"Jogo de Teste","uris":["{}"],"uploadDate":"2026-01-01","fileSize":"1.2 GB","extra":1}}"#,
            magnet()
        )))
        .unwrap();
        assert_eq!(s.name, "Minha fonte");
        assert_eq!(s.items.len(), 1);
        assert_eq!(s.items[0].title, "Jogo de Teste");
        assert_eq!(s.items[0].file_size, "1.2 GB");
        assert_eq!(s.dropped, 0);
    }

    #[test]
    fn broken_input_is_a_typed_error_never_a_panic() {
        assert_eq!(
            parse(b"{\"name\":\"x\",\"downloads\":["),
            Err(SourceError::InvalidJson)
        );
        assert_eq!(parse(b""), Err(SourceError::InvalidJson));
        assert_eq!(parse(b"[1,2]"), Err(SourceError::MissingDownloads));
        assert_eq!(
            parse(br#"{"downloads":"no"}"#),
            Err(SourceError::MissingDownloads)
        );
        assert_eq!(
            parse(&vec![b' '; MAX_SOURCE_BYTES + 1]),
            Err(SourceError::TooLarge)
        );
    }

    #[test]
    fn bad_items_are_dropped_and_counted_without_losing_the_source() {
        let m = magnet();
        let s = parse(&source(&format!(
            r#"{{"title":"Bom","uris":["{m}"]}},
               {{"title":5,"uris":["{m}"]}},
               {{"uris":["{m}"]}},
               {{"title":"Sem uri","uris":[]}},
               {{"title":"Tipo trocado","uris":"{m}"}},
               {{"title":"   ","uris":["{m}"]}},
               {{"title":"{long}","uris":["{m}"]}},
               "texto",
               null"#,
            long = "a".repeat(MAX_TITLE_CHARS + 1)
        )))
        .unwrap();
        assert_eq!(s.items.len(), 1);
        assert_eq!(s.dropped, 8);
    }

    #[test]
    fn only_infohash_magnets_and_https_torrent_files_pass() {
        assert!(valid_uri(&magnet()).is_some());
        assert!(valid_uri("MAGNET:?XT=URN:BTIH:ABCDEFGHIJKLMNOPQRSTUVWXYZ234567").is_some());
        assert!(valid_uri("https://exemplo.invalid/a/b.torrent?x=1").is_some());
        for bad in [
            "magnet:?xt=urn:btih:123",
            "magnet:?dn=sem-hash",
            "magnet:?xt=urn:btih:zzzz456789abcdef0123456789abcdef01234567",
            "http://exemplo.invalid/a.torrent",
            "https://exemplo.invalid/setup.exe",
            "https://user@exemplo.invalid/a.torrent",
            "https:///a.torrent",
            "file:///etc/passwd",
            "javascript:alert(1)",
            "C:\\jogos\\a.torrent",
            "/tmp/a.torrent",
            "https://exemplo.invalid/a b.torrent",
            "https://exemplo.invalid/a\n.torrent",
        ] {
            assert_eq!(valid_uri(bad), None, "{bad}");
        }
        let long = format!("{}&dn={}", magnet(), "x".repeat(MAX_URI_BYTES));
        assert_eq!(valid_uri(&long), None);
    }

    #[test]
    fn uris_and_items_are_limited() {
        let m = magnet();
        let many = vec![format!("\"{m}\""); MAX_URIS + 4].join(",");
        let s = parse(&source(&format!(r#"{{"title":"A","uris":[{many}]}}"#))).unwrap();
        assert_eq!(s.items[0].uris.len(), MAX_URIS);

        let one = format!(r#"{{"title":"A","uris":["{m}"]}}"#);
        let all = vec![one; MAX_ITEMS + 3].join(",");
        let s = parse(&source(&all)).unwrap();
        assert_eq!(s.items.len(), MAX_ITEMS);
        assert_eq!(s.dropped, 3);
    }

    #[test]
    fn odd_unicode_is_shown_as_plain_text() {
        let s = parse(&source(&format!(
            r#"{{"title":"Jogo\u0000\u202Etset\u200B  ✓ 日本語 🎮\n","uris":["{}"]}}"#,
            magnet()
        )))
        .unwrap();
        assert_eq!(s.items[0].title, "Jogo tset ✓ 日本語 🎮");
    }

    #[test]
    fn cache_file_name_does_not_come_from_the_url() {
        let f = cache_file(std::path::Path::new("/c"), "https://x.invalid/../../a.json");
        assert_eq!(f.parent(), Some(std::path::Path::new("/c")));
        assert_eq!(f.file_name().unwrap().len(), 64 + 5);
    }

    #[test]
    fn infohash_is_lowercase_and_only_for_magnets() {
        assert_eq!(infohash(&magnet().to_uppercase()).as_deref(), Some(HASH));
        assert_eq!(infohash("https://exemplo.invalid/a.torrent"), None);
    }
}
