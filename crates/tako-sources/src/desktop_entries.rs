//! Atalhos `.desktop` do Linux (Desktop Entry Specification) e detecção de executáveis soltos
//! (AppImage, script, binário) para os jogos manuais (MIG-026).

use std::path::{Path, PathBuf};

/// Grupo `[Desktop Entry]` de um arquivo `.desktop`, só com o que o Tako usa.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DesktopEntry {
    /// Arquivo de origem (vazio quando veio de texto).
    pub file: PathBuf,
    pub name: String,
    pub exec: Option<String>,
    /// Nome de ícone do tema ou caminho absoluto.
    pub icon: Option<String>,
    /// Pasta de trabalho (`Path`).
    pub working_dir: Option<String>,
    pub categories: Vec<String>,
    pub is_application: bool,
    /// `NoDisplay` ou `Hidden`: o usuário não deveria ver.
    pub hidden: bool,
}

impl DesktopEntry {
    pub fn is_game(&self) -> bool {
        self.is_application
            && !self.hidden
            && self.exec.is_some()
            && self.categories.iter().any(|c| c == "Game")
    }

    /// Alvo e argumentos para abrir como jogo manual: o programa do `Exec`, sem os códigos de
    /// campo (`%f`, `%U`…), e o resto numa linha só.
    pub fn launch(&self) -> Option<(String, Option<String>)> {
        let mut args = split_exec(self.exec.as_deref()?)?.into_iter();
        let target = args.next()?;
        let rest: Vec<String> = args.map(|a| quote_arg(&a)).collect();
        Some((target, (!rest.is_empty()).then(|| rest.join(" "))))
    }
}

/// Lê o grupo `[Desktop Entry]`. Sem `Name`, não é um atalho válido.
// shortcut: ignora nomes traduzidos (`Name[pt_BR]`); usar o idioma da UI se pedirem.
pub fn parse_desktop_entry(text: &str) -> Option<DesktopEntry> {
    let mut entry = DesktopEntry::default();
    let mut in_group = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_group = line == "[Desktop Entry]";
            continue;
        }
        if !in_group || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = unescape(value.trim());
        match key.trim() {
            "Name" => entry.name = value,
            "Exec" => entry.exec = Some(value).filter(|v| !v.is_empty()),
            "Icon" => entry.icon = Some(value).filter(|v| !v.is_empty()),
            "Path" => entry.working_dir = Some(value).filter(|v| !v.is_empty()),
            "Type" => entry.is_application = value == "Application",
            "Categories" => {
                entry.categories = value
                    .split(';')
                    .filter(|c| !c.is_empty())
                    .map(str::to_owned)
                    .collect()
            }
            "NoDisplay" | "Hidden" if value == "true" => entry.hidden = true,
            _ => {}
        }
    }
    (!entry.name.is_empty()).then_some(entry)
}

/// Lê um `.desktop` do disco (limite de tamanho: é arquivo de terceiros).
pub fn read_desktop_file(path: &Path) -> Option<DesktopEntry> {
    let bytes = crate::read_limited(path, 256 * 1024).ok()?;
    let mut entry = parse_desktop_entry(&String::from_utf8_lossy(&bytes))?;
    entry.file = path.to_path_buf();
    Some(entry)
}

/// Escapes de valor do formato (`\s`, `\n`, `\t`, `\r`, `\\`).
fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('s') => out.push(' '),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

/// Divide o `Exec` em argumentos (aspas duplas com `\` de escape) e tira os códigos de campo.
/// Aspas sem fechar: `None`.
pub fn split_exec(exec: &str) -> Option<Vec<String>> {
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut in_arg = false;
    let mut chars = exec.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                in_arg = true;
                loop {
                    match chars.next()? {
                        '"' => break,
                        '\\' => cur.push(chars.next()?),
                        c => cur.push(c),
                    }
                }
            }
            c if c.is_whitespace() => {
                if in_arg {
                    args.push(std::mem::take(&mut cur));
                    in_arg = false;
                }
            }
            // `%%` é um `%`; os outros códigos (%f %F %u %U %i %c %k…) somem.
            '%' => {
                if chars.next() == Some('%') {
                    cur.push('%');
                    in_arg = true;
                }
            }
            c => {
                cur.push(c);
                in_arg = true;
            }
        }
    }
    if in_arg {
        args.push(cur);
    }
    Some(args)
}

fn quote_arg(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains(|c: char| c.is_whitespace() || c == '"' || c == '\\') {
        return arg.to_owned();
    }
    format!("\"{}\"", arg.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Dá para abrir direto: AppImage, script `.sh` ou arquivo com bit de execução.
pub fn is_launchable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase());
    if matches!(ext.as_deref(), Some("appimage" | "sh")) {
        return true;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    false
}

/// Atalhos de jogo nas pastas dadas (o primeiro com o mesmo nome de arquivo vale, como no XDG).
/// Fica de fora o que já vem por uma fonte (Steam, Heroic, Lutris), para não duplicar.
// shortcut: não desce em subpastas de `applications` (raro); varrer recursivamente se aparecer.
pub fn game_desktop_entries_in(dirs: &[PathBuf]) -> Vec<DesktopEntry> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for dir in dirs {
        let Ok(read) = std::fs::read_dir(dir) else {
            continue;
        };
        let mut files: Vec<PathBuf> = read
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "desktop"))
            .collect();
        files.sort();
        for file in files {
            let Some(id) = file.file_name().map(|n| n.to_owned()) else {
                continue;
            };
            if !seen.insert(id) {
                continue;
            }
            let Some(entry) = read_desktop_file(&file).filter(DesktopEntry::is_game) else {
                continue;
            };
            let exec = entry.exec.as_deref().unwrap_or_default();
            if ["steam://", "heroic://", "lutris:"]
                .iter()
                .any(|s| exec.contains(s))
            {
                continue;
            }
            out.push(entry);
        }
    }
    out.sort_by_cached_key(|e| e.name.to_lowercase());
    out
}

/// Atalhos de jogo do usuário e do sistema, para o "Adicionar jogo".
#[cfg(target_os = "linux")]
pub fn game_desktop_entries() -> Vec<DesktopEntry> {
    let env_dir = |k: &str| {
        std::env::var_os(k)
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
    };
    let home = env_dir("HOME").unwrap_or_default();
    let mut dirs = vec![
        env_dir("XDG_DATA_HOME")
            .unwrap_or_else(|| home.join(".local/share"))
            .join("applications"),
    ];
    let system = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    dirs.extend(
        system
            .split(':')
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .map(|p| p.join("applications")),
    );
    game_desktop_entries_in(&dirs)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const GAME: &str = r#"[Desktop Entry]
# comentário
Type=Application
Name=Jogo Livre Fictício
Name[pt_BR]=Jogo Livre
Exec=env LANG=C "/opt/jogo livre/run.sh" --fullscreen %U
Icon=/opt/jogo livre/icon.png
Path=/opt/jogo livre
Categories=Game;ActionGame;

[Desktop Action Novo]
Name=Outro
Exec=outro
"#;

    #[test]
    fn parses_game_entry_and_launch() {
        let e = parse_desktop_entry(GAME).unwrap();
        assert_eq!(e.name, "Jogo Livre Fictício");
        assert!(e.is_game());
        assert_eq!(e.icon.as_deref(), Some("/opt/jogo livre/icon.png"));
        assert_eq!(e.working_dir.as_deref(), Some("/opt/jogo livre"));
        let (target, args) = e.launch().unwrap();
        assert_eq!(target, "env");
        assert_eq!(
            args.as_deref(),
            Some(r#"LANG=C "/opt/jogo livre/run.sh" --fullscreen"#)
        );
    }

    #[test]
    fn exec_field_codes_quotes_and_escapes() {
        assert_eq!(split_exec("app %f --x=%% %i").unwrap(), ["app", "--x=%"]);
        assert_eq!(split_exec(r#""a \"b\"" c"#).unwrap(), [r#"a "b""#, "c"]);
        assert_eq!(split_exec(r#"app "aberto"#), None);
        assert_eq!(split_exec("  ").unwrap(), Vec::<String>::new());
        // `\s` no arquivo vira espaço antes de dividir.
        let e = parse_desktop_entry("[Desktop Entry]\nName=X\nExec=\"a\\sb\" c").unwrap();
        assert_eq!(e.launch().unwrap(), ("a b".into(), Some("c".into())));
    }

    #[test]
    fn non_games_and_broken_entries() {
        let hidden = GAME.replace("Type=Application", "Type=Application\nNoDisplay=true");
        assert!(!parse_desktop_entry(&hidden).unwrap().is_game());
        let link = GAME.replace("Type=Application", "Type=Link");
        assert!(!parse_desktop_entry(&link).unwrap().is_game());
        let tool = GAME.replace("Categories=Game;ActionGame;", "Categories=Utility;");
        assert!(!parse_desktop_entry(&tool).unwrap().is_game());
        assert_eq!(parse_desktop_entry("lixo sem grupo"), None);
        assert_eq!(parse_desktop_entry("[Outro]\nName=X"), None);
        assert_eq!(parse_desktop_entry(""), None);
    }

    #[test]
    fn lists_games_skipping_store_shortcuts_and_shadowed_files() {
        let tmp = tempfile::tempdir().unwrap();
        let (user, system) = (tmp.path().join("user"), tmp.path().join("system"));
        std::fs::create_dir_all(&user).unwrap();
        std::fs::create_dir_all(&system).unwrap();
        std::fs::write(user.join("jogo.desktop"), GAME).unwrap();
        // Mesmo id no sistema: o do usuário vale.
        std::fs::write(
            system.join("jogo.desktop"),
            GAME.replace("Fictício", "Sistema"),
        )
        .unwrap();
        let steam = GAME.replace(
            "Exec=env LANG=C \"/opt/jogo livre/run.sh\" --fullscreen %U",
            "Exec=steam steam://rungameid/570",
        );
        std::fs::write(system.join("steam.desktop"), steam).unwrap();
        std::fs::write(system.join("nota.txt"), GAME).unwrap();

        let list = game_desktop_entries_in(&[user.clone(), system, tmp.path().join("nada")]);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Jogo Livre Fictício");
        assert_eq!(list[0].file, user.join("jogo.desktop"));
    }

    #[test]
    fn launchable_files() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("Jogo.AppImage");
        let sh = tmp.path().join("start.sh");
        let txt = tmp.path().join("leia.txt");
        for f in [&app, &sh, &txt] {
            std::fs::write(f, b"x").unwrap();
        }
        assert!(is_launchable(&app));
        assert!(is_launchable(&sh));
        assert!(!is_launchable(&txt));
        assert!(!is_launchable(tmp.path()));
        assert!(!is_launchable(&tmp.path().join("nada")));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&txt, std::fs::Permissions::from_mode(0o755)).unwrap();
            assert!(is_launchable(&txt));
        }
    }
}
