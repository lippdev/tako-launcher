//! Caminhos do Windows em texto (port de `WindowsPath`), testáveis em qualquer SO: sempre com `\`.

/// Barras normalizadas para `\`, sem aspas nem barra no fim (exceto a raiz "C:\").
pub fn normalize(path: &str) -> String {
    let n = path.trim().trim_matches('"').replace('/', "\\");
    let n = n.trim_end_matches('\\');
    if n.len() == 2 && n.as_bytes()[1] == b':' {
        format!("{n}\\")
    } else {
        n.to_owned()
    }
}

/// Junta pasta e partes com `\`, aceitando `/` (o registro da Ubisoft usa `/`).
pub fn combine(folder: &str, parts: &[&str]) -> String {
    let mut path = normalize(folder);
    for part in parts {
        let clean = normalize(part);
        let clean = clean.trim_start_matches('\\');
        if !path.ends_with('\\') {
            path.push('\\');
        }
        path.push_str(clean);
    }
    path
}

/// Último nome do caminho ("D:\Jogos\Farol" → "Farol").
pub fn last_name(path: &str) -> String {
    let n = normalize(path);
    match n.rfind('\\') {
        Some(i) => n[i + 1..].to_owned(),
        None => n,
    }
}

/// "D:\Jogos" ou "\\servidor\…" são absolutos mesmo fora do Windows.
pub fn is_rooted(path: &str) -> bool {
    let b = path.as_bytes();
    (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/'))
        || path.starts_with("\\\\")
}

/// Pasta de um caminho do Windows; `None` sem barra.
pub fn directory_of(path: &str) -> Option<String> {
    let n = path.replace('/', "\\");
    match n.rfind('\\') {
        Some(i) if i > 0 => Some(n[..i].to_owned()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_paths_are_normalized_without_the_host_os() {
        assert_eq!(normalize("D:/Jogos/Farol/"), r"D:\Jogos\Farol");
        assert_eq!(normalize(r"C:\"), r"C:\");
        assert_eq!(combine(r"C:\", &["x", "y.exe"]), r"C:\x\y.exe");
        assert_eq!(last_name(r"D:\Jogos\Farol\"), "Farol");
        assert!(is_rooted(r"E:\x") && is_rooted(r"\\nas\x") && !is_rooted(r"bin\x"));
    }
}
