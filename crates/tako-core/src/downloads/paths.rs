//! Onde o download grava (ADR-0028 §3): `<pasta de downloads>/<título saneado>/`, e os caminhos de dentro do
//! torrent validados contra *path traversal*, nomes reservados do Windows e links simbólicos.

use std::path::{Component, Path, PathBuf};

const MAX_NAME_CHARS: usize = 80;
/// Nomes que o Windows reserva, com ou sem extensão (`CON.txt`, `CON .txt`), comparados em minúsculas.
const RESERVED: [&str; 32] = [
    "con", "prn", "aux", "nul", "conin$", "conout$", "com0", "com1", "com2", "com3", "com4",
    "com5", "com6", "com7", "com8", "com9", "com¹", "com²", "com³", "lpt0", "lpt1", "lpt2", "lpt3",
    "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9", "lpt¹", "lpt²", "lpt³",
];

/// O nome é reservado no Windows: a parte antes do primeiro ponto, sem espaços no fim, é um nome de dispositivo.
pub fn is_reserved(name: &str) -> bool {
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end()
        .to_lowercase();
    RESERVED.contains(&stem.as_str())
}

/// Nome de pasta seguro nos três SOs: sem separadores, caracteres proibidos, nomes reservados nem ponto ou espaço
/// no fim; vazio vira `download`. Os pontos viram `_`: no macOS uma pasta `Jogo.app` ou `Jogo.pkg` é um pacote, e
/// "Abrir pasta" o executaria.
pub fn folder_name(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            '.' => '_',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    let name: String = cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_NAME_CHARS)
        .collect();
    let name = name.trim_matches(['_', ' ']).to_owned();
    if name.is_empty() {
        return "download".into();
    }
    if is_reserved(&name) {
        return format!("_{name}");
    }
    name
}

/// Caminho de um arquivo do torrent, relativo à pasta do download, se for seguro: só nomes normais (sem `..`,
/// raiz, letra de disco, `.`), cada parte sem separador, caractere de controle, nome reservado do Windows nem ponto
/// ou espaço no fim. `None` = o torrent é recusado.
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
            || p.ends_with(['.', ' '])
            || is_reserved(p)
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

/// Verifica se `folder` é uma subpasta estritamente contida em `root` (sem `..`, raiz própria ou igual a `root`).
pub fn is_inside_root(root: &Path, folder: &Path) -> bool {
    match folder.strip_prefix(root) {
        Ok(rel) => {
            let mut comps = rel.components();
            match comps.next() {
                Some(Component::Normal(first)) if !first.is_empty() => {
                    rel.components().all(|c| matches!(c, Component::Normal(_)))
                }
                _ => false,
            }
        }
        Err(_) => false,
    }
}

/// O caminho final fica dentro de `root` (checagem léxica, depois de [`safe_relative`]).
pub fn inside(root: &Path, relative: &Path) -> bool {
    relative
        .components()
        .all(|c| matches!(c, Component::Normal(_)))
        && root.join(relative).starts_with(root)
}

/// Algum trecho de `base/relative` que já existe é link simbólico (ou junção): `base` e cada parte de `relative`,
/// até a primeira que ainda não existe. Gravar através dele sairia da pasta do download.
pub fn has_link(base: &Path, relative: &Path) -> bool {
    let link = |p: &Path| {
        std::fs::symlink_metadata(p)
            .map(|m| m.file_type().is_symlink() || is_junction(&m))
            .ok()
    };
    if base.ancestors().any(|p| link(p) == Some(true)) {
        return true;
    }
    let mut at = base.to_path_buf();
    for c in relative.components() {
        at.push(c);
        match link(&at) {
            Some(true) => return true,
            Some(false) => {}
            None => return false,
        }
    }
    false
}

#[cfg(windows)]
fn is_junction(m: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    // FILE_ATTRIBUTE_REPARSE_POINT: junções e outros pontos de nova análise.
    m.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
fn is_junction(_: &std::fs::Metadata) -> bool {
    false
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
        assert_eq!(folder_name("con .txt"), "con _txt");
        assert!(!is_reserved(&folder_name("CON .txt")));
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
    fn folder_names_never_look_like_a_macos_package_or_a_file() {
        for t in ["Jogo.app", "Instalador.pkg", "setup.exe", "Jogo v1.2.dmg"] {
            let n = folder_name(t);
            assert!(!n.contains('.'), "{t} → {n}");
        }
        assert_eq!(folder_name("Jogo.app"), "Jogo_app");
    }

    #[test]
    fn windows_reserved_names_are_refused_inside_the_torrent() {
        for bad in [
            "CON",
            "con.txt",
            "CON .txt",
            "aux",
            "NUL.tar.gz",
            "COM0",
            "com9",
            "LPT0",
            "lpt¹",
            "COM²",
            "COM³.log",
            "CONIN$",
            "conout$",
        ] {
            assert!(is_reserved(bad), "{bad}");
            assert_eq!(safe_relative(&["a", bad]), None, "{bad}");
        }
        for bad in ["nome.", "nome ", "pasta. "] {
            assert_eq!(safe_relative(&[bad]), None, "{bad}");
        }
        assert!(!is_reserved("console"));
        assert!(!is_reserved("com10"));
        assert!(safe_relative(&["console.txt"]).is_some());
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
    fn is_inside_root_validates_subfolder() {
        let root = Path::new("/d/Tako");
        assert!(is_inside_root(root, Path::new("/d/Tako/Jogo")));
        assert!(is_inside_root(root, Path::new("/d/Tako/Jogo/sub")));
        assert!(!is_inside_root(root, Path::new("/d/Tako")));
        assert!(!is_inside_root(root, Path::new("/d/Tako/../outside")));
        assert!(!is_inside_root(root, Path::new("/etc/passwd")));
        assert!(!is_inside_root(root, Path::new("relativo")));
    }

    #[test]
    fn inside_rejects_anything_but_plain_names() {
        let root = Path::new("/d/Tako/Jogo");
        assert!(inside(root, Path::new("a/b")));
        assert!(!inside(root, Path::new("../b")));
        assert!(!inside(root, Path::new("/b")));
    }

    #[cfg(unix)]
    #[test]
    fn links_anywhere_in_the_path_are_found() {
        let dir = tempfile::tempdir().unwrap();
        let resolved = dir.path().canonicalize().unwrap();
        let base = resolved.join("Tako");
        std::fs::create_dir_all(base.join("Jogo/data")).unwrap();
        assert!(!has_link(&base, Path::new("Jogo/data/a.bin")));
        assert!(!has_link(&base, Path::new("Novo/x")));
        let outside = dir.path().join("fora");
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, base.join("Jogo/link")).unwrap();
        assert!(has_link(&base, Path::new("Jogo/link/a.bin")));
        std::os::unix::fs::symlink(&outside, resolved.join("TakoLink")).unwrap();
        assert!(has_link(&resolved.join("TakoLink"), Path::new("x")));
        assert!(has_link(
            &resolved.join("TakoLink/subpasta/Tako"),
            Path::new("Novo/a.bin")
        ));
    }
}
