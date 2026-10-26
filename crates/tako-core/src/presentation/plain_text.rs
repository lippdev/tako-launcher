//! Port de `Presentation/PlainText.cs`: HTML de descrições (lojas, temas do Playnite) → texto puro. Não interpreta
//! nada: tags somem, entidades viram caracteres e quebras de bloco viram linhas. Sem imagens, links, estilos nem
//! scripts (MVP-018).

/// Limite do texto exibido: descrições enormes não travam a UI.
pub const MAX_LENGTH: usize = 20_000;

/// Tags que começam ou terminam uma linha.
const LINE_TAGS: [&str; 4] = ["li", "div", "tr", "hr"];
/// Tags que separam parágrafos (uma linha em branco).
const PARAGRAPH_TAGS: [&str; 11] = [
    "p",
    "ul",
    "ol",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "table",
    "blockquote",
];
/// Conteúdo dessas tags não é texto para o usuário e é descartado inteiro.
const SKIPPED_CONTENT_TAGS: [&str; 4] = ["script", "style", "head", "title"];

fn is_in(list: &[&str], name: &str) -> bool {
    list.iter().any(|t| t.eq_ignore_ascii_case(name))
}

/// Texto em construção, com o tamanho em caracteres.
struct Out {
    text: String,
    len: usize,
}

impl Out {
    fn push(&mut self, c: char) {
        self.text.push(c);
        self.len += 1;
    }

    fn full(&self) -> bool {
        self.len >= MAX_LENGTH
    }
}

pub fn from_html(html: &str) -> String {
    let mut out = Out {
        text: String::with_capacity(html.len().min(MAX_LENGTH)),
        len: 0,
    };
    let mut skipping: Option<String> = None;
    let mut i = 0;
    while i < html.len() && !out.full() {
        if html[i..].starts_with('<') {
            let Some(end) = tag_end(html, i) else {
                // "<" solto (ex.: "a < b"): texto comum.
                if skipping.is_none() {
                    out.push('<');
                }
                i += 1;
                continue;
            };
            let (name, closing) = tag_name(&html[i + 1..end]);
            if let Some(skip) = &skipping {
                if closing && name.eq_ignore_ascii_case(skip) {
                    skipping = None;
                }
            } else if !closing && is_in(&SKIPPED_CONTENT_TAGS, name) && !html[..end].ends_with('/')
            {
                skipping = Some(name.to_owned());
            } else if name.eq_ignore_ascii_case("br") {
                out.push('\n');
            } else if is_in(&LINE_TAGS, name) || is_in(&PARAGRAPH_TAGS, name) {
                line_break(&mut out, if is_in(&PARAGRAPH_TAGS, name) { 2 } else { 1 });
            }
            i = end + 1;
            continue;
        }
        let next = html[i..].find('<').map_or(html.len(), |n| i + n);
        if skipping.is_none() {
            for c in decode_entities(&html[i..next]).chars() {
                if out.full() {
                    break;
                }
                // Espaços e quebras do código-fonte HTML não são quebras de linha visíveis.
                out.push(if c.is_whitespace() { ' ' } else { c });
            }
        }
        i = next;
    }
    collapse(&out.text)
}

/// Posição do `>` que fecha a tag, respeitando aspas; `None` se não for uma tag.
fn tag_end(html: &str, start: usize) -> Option<usize> {
    let first = html[start + 1..].chars().next()?;
    if !(first.is_alphabetic() || matches!(first, '/' | '!' | '?')) {
        return None;
    }
    let mut quote = None;
    for (offset, b) in html.as_bytes()[start + 1..].iter().enumerate() {
        match (quote, b) {
            (Some(q), _) if *b == q => quote = None,
            (Some(_), _) => {}
            (None, b'"' | b'\'') => quote = Some(*b),
            (None, b'>') => return Some(start + 1 + offset),
            _ => {}
        }
    }
    None
}

fn tag_name(inner: &str) -> (&str, bool) {
    let closing = inner.starts_with('/');
    let rest = if closing { &inner[1..] } else { inner };
    let len = rest
        .char_indices()
        .find(|(_, c)| !(c.is_alphanumeric() || matches!(c, '-' | ':')))
        .map_or(rest.len(), |(i, _)| i);
    (&rest[..len], closing)
}

/// Garante `lines` quebras no fim do texto (nada no começo).
fn line_break(out: &mut Out, lines: usize) {
    if out.text.is_empty() {
        return;
    }
    let trailing = out
        .text
        .chars()
        .rev()
        .take_while(|c| matches!(c, '\n' | ' '))
        .filter(|c| *c == '\n')
        .count();
    for _ in trailing..lines {
        out.push('\n');
    }
}

/// Espaços repetidos viram um; no máximo uma linha em branco entre parágrafos; sem bordas vazias.
fn collapse(text: &str) -> String {
    let mut result = String::new();
    let mut blank = 0;
    for line in text.split('\n') {
        let line = line
            .split(' ')
            .filter(|w| !w.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if line.is_empty() {
            blank += 1;
            continue;
        }
        if !result.is_empty() {
            result.push_str(if blank > 0 { "\n\n" } else { "\n" });
        }
        result.push_str(&line);
        blank = 0;
    }
    result
}

/// Entidades HTML: numéricas e as nomeadas comuns em descrições. Desconhecida fica como está.
fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_owned();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let decoded = rest[1..]
            .find(';')
            .filter(|&n| n > 0 && n <= 10)
            .and_then(|n| entity(&rest[1..=n]).map(|c| (c, n + 2)));
        match decoded {
            Some((c, used)) => {
                out.push(c);
                rest = &rest[used..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn entity(name: &str) -> Option<char> {
    if let Some(num) = name.strip_prefix('#') {
        let code = match num.strip_prefix(['x', 'X']) {
            Some(hex) => u32::from_str_radix(hex, 16).ok()?,
            None => num.parse().ok()?,
        };
        return char::from_u32(code).filter(|c| *c != '\0');
    }
    let c = match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => '\u{a0}',
        "copy" => '©',
        "reg" => '®',
        "trade" => '™',
        "hellip" => '…',
        "mdash" => '—',
        "ndash" => '–',
        "lsquo" => '‘',
        "rsquo" => '’',
        "ldquo" => '“',
        "rdquo" => '”',
        "laquo" => '«',
        "raquo" => '»',
        "bull" => '•',
        "middot" => '·',
        "deg" => '°',
        "eacute" => 'é',
        "Eacute" => 'É',
        "aacute" => 'á',
        "Aacute" => 'Á',
        "iacute" => 'í',
        "Iacute" => 'Í',
        "oacute" => 'ó',
        "Oacute" => 'Ó',
        "uacute" => 'ú',
        "Uacute" => 'Ú',
        "atilde" => 'ã',
        "Atilde" => 'Ã',
        "otilde" => 'õ',
        "Otilde" => 'Õ',
        "acirc" => 'â',
        "Acirc" => 'Â',
        "ecirc" => 'ê',
        "Ecirc" => 'Ê',
        "ocirc" => 'ô',
        "Ocirc" => 'Ô',
        "agrave" => 'à',
        "Agrave" => 'À',
        "ccedil" => 'ç',
        "Ccedil" => 'Ç',
        "uuml" => 'ü',
        "ntilde" => 'ñ',
        _ => return None,
    };
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_html_to_plain_text() {
        for (html, expected) in [
            ("", ""),
            ("Texto simples", "Texto simples"),
            ("<b>Negrito</b> e <i>itálico</i>", "Negrito e itálico"),
            (
                "Linha 1<br>Linha 2<br/>Linha 3",
                "Linha 1\nLinha 2\nLinha 3",
            ),
            ("<p>Um</p><p>Dois</p>", "Um\n\nDois"),
            ("<ul><li>A</li><li>B</li></ul>", "A\nB"),
            (
                "Tom &amp; Jerry &lt;3 &quot;ok&quot; &#233;",
                "Tom & Jerry <3 \"ok\" é",
            ),
            ("a < b e c > d", "a < b e c > d"),
            ("  muitos\n\n   espaços\t aqui  ", "muitos espaços aqui"),
            (
                "&#x1F600; &naoexiste; & &#0; &nbsp;fim",
                "😀 &naoexiste; & &#0; fim",
            ),
        ] {
            assert_eq!(from_html(html), expected, "{html}");
        }
    }

    #[test]
    fn drops_active_content_and_urls() {
        for (html, expected) in [
            ("<script>alert('x')</script>Visível", "Visível"),
            ("<style>body{color:red}</style>Visível", "Visível"),
            (
                "<img src=\"https://rastreio.example/p.gif\" onerror=\"x()\">Visível",
                "Visível",
            ),
            ("<a href=\"https://example.com\">link</a>", "link"),
            ("<iframe src=\"file:///C:/\"></iframe>Visível", "Visível"),
            ("<img alt='a > b' src=x>Visível", "Visível"),
        ] {
            let text = from_html(html);
            assert_eq!(text, expected, "{html}");
            assert!(!text.contains("://"));
        }
    }

    #[test]
    fn limits_huge_descriptions() {
        let html = "<p>bloco de texto</p>".repeat(10_000);
        assert!(from_html(&html).chars().count() <= MAX_LENGTH);
    }

    #[test]
    fn unclosed_script_hides_the_rest() {
        assert_eq!(from_html("Antes<script>while(true){}"), "Antes");
        assert_eq!(from_html("<"), "<");
        assert_eq!(from_html("<é>á"), "á");
    }
}
