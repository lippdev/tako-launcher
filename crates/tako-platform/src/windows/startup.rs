//! Linha de comando gravada em `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (port de
//! `Launcher.Core/Platform/StartupCommand.cs`). Só conta como ligada quando aponta para o executável atual:
//! um valor antigo (portátil movido, outra cópia) abriria outro programa ou nada.

pub fn command_for(executable_path: &str) -> String {
    format!("\"{executable_path}\" --startup")
}

/// Caminho do executável numa linha de comando (com ou sem aspas, ignorando argumentos).
pub fn executable_of(command: &str) -> Option<&str> {
    let text = command.trim();
    if let Some(rest) = text.strip_prefix('"') {
        return rest
            .find('"')
            .filter(|&end| end > 0)
            .map(|end| &rest[..end]);
    }
    let lower = text.to_ascii_lowercase();
    match lower.find(".exe") {
        Some(i) if i > 0 => Some(&text[..i + 4]),
        _ => text.split(' ').next().filter(|s| !s.is_empty()),
    }
}

pub fn points_to(command: Option<&str>, executable_path: &str) -> bool {
    let normalize = |p: &str| {
        p.trim()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    };
    command
        .and_then(executable_of)
        .is_some_and(|exe| normalize(exe) == normalize(executable_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE: &str = r"C:\Users\Ana\AppData\Local\Programs\Tako\Tako.exe";

    #[test]
    fn startup_command_quotes_the_path() {
        assert_eq!(
            command_for(r"C:\Program Files\Tako\Tako.exe"),
            r#""C:\Program Files\Tako\Tako.exe" --startup"#
        );
        assert!(points_to(Some(&command_for(EXE)), EXE));
    }

    #[test]
    fn startup_entry_counts_only_when_it_points_to_this_executable() {
        for (command, expected) in [
            (
                Some(r#""C:\Users\Ana\AppData\Local\Programs\Tako\Tako.exe""#),
                true,
            ),
            (
                Some(r#""c:\users\ana\appdata\local\programs\tako\TAKO.EXE" --fullscreen"#),
                true,
            ),
            (
                Some(r"C:\Users\Ana\AppData\Local\Programs\Tako\Tako.exe"),
                true,
            ),
            (Some(r#""D:\Portatil\Tako.exe""#), false),
            (Some(""), false),
            (Some("\"\""), false),
            (Some("\"sem fim"), false),
            (None, false),
        ] {
            assert_eq!(points_to(command, EXE), expected, "{command:?}");
        }
    }
}
