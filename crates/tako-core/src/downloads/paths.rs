//! Onde o download grava (ADR-0028 §3): `<pasta de downloads>/<título saneado>/`, e os caminhos de dentro do
//! torrent validados contra *path traversal*. Puro: não toca o disco.

use std::path::{Component, Path, PathBuf};

const MAX_NAME_CHARS: usize = 80;
/// Nomes que o Windows reserva (com ou sem extensão).
const RESERVED: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Nome de pasta seguro nos três SOs: sem separadores, `..`, caracteres proibidos, nomes reservados nem ponto ou
/// espaço no fim; vazio vira `download`.
pub fn folder_name(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    let mut name: String = cleaned
        .split_whitespace()
        .filter(|w| !w.chars().all(|c| c == '.'))
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_NAME_CHARS)
        .collect();
    while name.ends_with(['.', ' ']) {
        name.pop();
    }
    let name = name.trim_start_matches('.').trim().to_owned();
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if name.is_empty() || RESERVED.contains(&stem.as_str()) {
        return if name.is_empty() {
            "download".into()
        } else {
            format!("_{name}")
        };
    }
    name
}

/// Caminho de um arquivo do torrent, relativo à pasta do download, se for seguro: só nomes normais (sem `..`,
/// raiz, letra de disco, `.`), cada parte sem separador nem caractere de controle. `None` = o torrent é recusado.
pub fn safe_relative<S: AsRef<str>>(parts: &[S]) -> Option<PathBuf> {
    if parts.is_empty() {
        return None;
    }
    let mut out = PathBuf::new();
    for p in parts {
        let p = p.as_ref();
        if p.is_empty()
            || p == "."
            || p == ".."
            || p.contains(['/', '\\', ':', '\0'])
            || p.chars().any(char::is_control)
        {
            return None;
        }
        let mut c = Path::new(p).components();
        match (c.next(), c.next()) {
            (Some(Component::Normal(_)), None) => out.push(p),
            _ => return None,
        }
    }
    Some(out)
}

/// O caminho final fica dentro de `root` (checagem léxica, depois de [`safe_relative`]).
pub fn inside(root: &Path, relative: &Path) -> bool {
    relative
        .components()
        .all(|c| matches!(c, Component::Normal(_)))
        && root.join(relative).starts_with(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_names_are_safe_on_every_os() {
        assert_eq!(
            folder_name("Jogo: Edição / Completa"),
            "Jogo Edição Completa"
        );
        assert_eq!(folder_name("../../etc"), "etc");
        assert_eq!(folder_name("..\\..\\Windows"), "Windows");
        assert_eq!(folder_name("CON"), "_CON");
        assert_eq!(folder_name("lpt1.txt"), "_lpt1.txt");
        assert_eq!(folder_name("nome. . "), "nome");
        assert_eq!(folder_name("   "), "download");
        assert_eq!(folder_name("..."), "download");
        assert_eq!(
            folder_name(&"a".repeat(500)).chars().count(),
            MAX_NAME_CHARS
        );
        assert!(!folder_name("a\u{0}b\nc").contains(['\0', '\n']));
    }

    #[test]
    fn torrent_paths_cannot_escape_the_download_folder() {
        assert_eq!(
            safe_relative(&["Jogo", "data", "a.bin"]),
            Some(PathBuf::from("Jogo").join("data").join("a.bin"))
        );
        for bad in [
            vec![".."],
            vec!["a", "..", "b"],
            vec!["/etc/passwd"],
            vec!["a/../../b"],
            vec!["..\\..\\b"],
            vec!["C:"],
            vec!["C:\\x"],
            vec!["."],
            vec![""],
            vec!["a\0b"],
            vec!["a\nb"],
        ] {
            assert_eq!(safe_relative(&bad), None, "{bad:?}");
        }
        assert_eq!(safe_relative::<&str>(&[]), None);
    }

    #[test]
    fn inside_rejects_anything_but_plain_names() {
        let root = Path::new("/d/Tako/Jogo");
        assert!(inside(root, Path::new("a/b")));
        assert!(!inside(root, Path::new("../b")));
        assert!(!inside(root, Path::new("/b")));
    }
}
