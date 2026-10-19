//! Suporte a Proton e Wine para jogos de Windows manuais no Linux (SESSION-005).
//!
//! Detecta runtimes disponíveis no sistema (Wine do sistema, Proton da Steam em bibliotecas e em
//! `compatibilitytools.d`, e runners do Lutris), valida caminhos de prefixo e monta o comando e o ambiente
//! de execução com segurança (nunca executa nada sem a escolha explícita do usuário).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Tipo de runtime de compatibilidade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeKind {
    Wine,
    Proton,
}

/// Um runtime de compatibilidade encontrado no sistema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityRuntime {
    /// Identificador estável (ex.: "system-wine", "proton:/path/to/proton", "wine:/path/to/bin/wine").
    pub id: String,
    /// Nome para exibição na interface (ex.: "Wine do sistema", "Proton 8.0", "GE-Proton8-25", "Lutris: lutris-7.2").
    pub name: String,
    /// Tipo de runtime (Wine ou Proton).
    pub kind: RuntimeKind,
    /// Caminho do executável (`wine` ou script `proton`).
    pub executable: PathBuf,
}

/// Erro de validação da pasta de prefixo do Wine/Proton.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixValidationError {
    /// O caminho está vazio.
    Empty,
    /// A raiz `/` não é permitida como prefixo por segurança.
    RootNotAllowed,
    /// A pasta pessoal do usuário (`$HOME` ou `~`) não é permitida como prefixo por segurança.
    HomeNotAllowed,
}

impl std::fmt::Display for PrefixValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "o caminho do prefixo não pode ser vazio"),
            Self::RootNotAllowed => write!(f, "o prefixo não pode ser a raiz do sistema (/)"),
            Self::HomeNotAllowed => {
                write!(f, "o prefixo não pode ser a pasta pessoal do usuário (~)")
            }
        }
    }
}

impl std::error::Error for PrefixValidationError {}

/// Valida um caminho de prefixo do Wine ou Proton.
///
/// # Segurança:
/// - O prefixo **nunca** pode ser `/` (raiz do sistema);
/// - O prefixo **nunca** pode ser a pasta pessoal (`$HOME` ou `~`).
pub fn validate_prefix_path(path: &Path, home: Option<&Path>) -> Result<(), PrefixValidationError> {
    let raw = path.to_string_lossy();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(PrefixValidationError::Empty);
    }
    // Remove barras redundantes no fim
    let clean = trimmed.trim_end_matches(['/', '\\']);
    if clean.is_empty() || clean == "/" {
        return Err(PrefixValidationError::RootNotAllowed);
    }
    if let Some(h) = home {
        let clean_home = h
            .to_string_lossy()
            .trim_end_matches(['/', '\\'])
            .to_string();
        if clean == clean_home || clean == "~" {
            return Err(PrefixValidationError::HomeNotAllowed);
        }
    }
    Ok(())
}

/// Expande `~` e valida o texto digitado pelo usuário.
pub fn expand_and_validate_prefix(
    prefix: &str,
    home: Option<&Path>,
) -> Result<PathBuf, PrefixValidationError> {
    let trimmed = prefix.trim();
    if trimmed.is_empty() {
        return Err(PrefixValidationError::Empty);
    }
    let path = if let Some(stripped) = trimmed
        .strip_prefix("~/")
        .or_else(|| trimmed.strip_prefix("~\\"))
    {
        if let Some(h) = home {
            h.join(stripped)
        } else {
            PathBuf::from(trimmed)
        }
    } else if trimmed == "~" {
        if let Some(h) = home {
            h.to_path_buf()
        } else {
            PathBuf::from(trimmed)
        }
    } else {
        PathBuf::from(trimmed)
    };
    validate_prefix_path(&path, home)?;
    Ok(path)
}

/// Configuração montada para abrir o jogo com o runtime de compatibilidade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchConfig {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub environment: Vec<(String, String)>,
}

impl LaunchConfig {
    /// Formata os argumentos como uma linha de comando com aspas nos caminhos com espaços.
    pub fn arguments_string(&self) -> String {
        self.arguments
            .iter()
            .map(|a| {
                if a.contains(' ') || a.contains('"') || a.contains('\'') {
                    format!("\"{}\"", a.replace('\\', "\\\\").replace('"', "\\\""))
                } else {
                    a.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Monta o executável, os argumentos e as variáveis de ambiente necessárias.
///
/// # Documentação dos runtimes fora da Steam:
/// - **Wine**: usa `WINEPREFIX=<prefixo>` para isolar o ambiente Windows do jogo. Abre executando
///   `wine <jogo.exe> [argumentos]`.
/// - **Proton fora da Steam**: o script `proton` da Valve exige obrigatoriamente:
///   - `STEAM_COMPAT_DATA_PATH`: pasta onde o Proton cria e gerencia o prefixo (dentro dela fica a subpasta `pfx`);
///   - `STEAM_COMPAT_CLIENT_INSTALL_PATH`: pasta de instalação do cliente Steam (onde residem os runtimes compartilhados);
///   - `STEAM_COMPAT_APP_ID`: identificador numérico da Steam (usamos "0" para jogos manuais externos).
///
///   Abre executando `proton run <jogo.exe> [argumentos]`.
pub fn prepare_launch(
    runtime: &CompatibilityRuntime,
    game_exe: &Path,
    game_args: Option<&str>,
    prefix: &Path,
    steam_root: Option<&Path>,
    extra_env: &[(String, String)],
) -> LaunchConfig {
    let mut env = Vec::new();
    let mut args = Vec::new();

    match runtime.kind {
        RuntimeKind::Wine => {
            env.push(("WINEPREFIX".into(), prefix.to_string_lossy().into_owned()));
            args.push(game_exe.to_string_lossy().into_owned());
        }
        RuntimeKind::Proton => {
            env.push((
                "STEAM_COMPAT_DATA_PATH".into(),
                prefix.to_string_lossy().into_owned(),
            ));
            if let Some(root) = steam_root {
                env.push((
                    "STEAM_COMPAT_CLIENT_INSTALL_PATH".into(),
                    root.to_string_lossy().into_owned(),
                ));
            }
            env.push(("STEAM_COMPAT_APP_ID".into(), "0".into()));
            args.push("run".into());
            args.push(game_exe.to_string_lossy().into_owned());
        }
    }

    if let Some(user_args) = game_args {
        for a in user_args.split_whitespace() {
            if !a.is_empty() {
                args.push(a.to_owned());
            }
        }
    }

    // Variáveis do usuário por cima (podem complementar ou sobrescrever)
    for (k, v) in extra_env {
        env.retain(|(key, _)| key != k);
        env.push((k.clone(), v.clone()));
    }

    LaunchConfig {
        executable: runtime.executable.clone(),
        arguments: args,
        environment: env,
    }
}

/// Extrai pastas de biblioteca de um `libraryfolders.vdf` da Steam.
pub fn parse_library_folders(content: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("\"path\"") {
            let mut parts = trimmed.split('"').filter(|p| !p.trim().is_empty());
            let _ = parts.next(); // "path"
            if let Some(dir) = parts.next() {
                let p = PathBuf::from(dir);
                if !paths.contains(&p) {
                    paths.push(p);
                }
            }
        }
    }
    paths
}

/// Procura versões do Proton dentro de `steamapps/common/Proton*` das bibliotecas da Steam.
pub fn scan_steam_common_proton(libraries: &[PathBuf]) -> Vec<CompatibilityRuntime> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for lib in libraries {
        let common = lib.join("steamapps/common");
        let Ok(entries) = std::fs::read_dir(&common) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with("Proton") && !name.starts_with("proton") {
                continue;
            }
            let proton_bin = path.join("proton");
            if !proton_bin.is_file() {
                continue;
            }
            let canon = proton_bin
                .canonicalize()
                .unwrap_or_else(|_| proton_bin.clone());
            if seen.insert(canon) {
                out.push(CompatibilityRuntime {
                    id: format!("proton:{}", proton_bin.display()),
                    name,
                    kind: RuntimeKind::Proton,
                    executable: proton_bin,
                });
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Procura Proton personalizado (ex.: GE-Proton) em pastas `compatibilitytools.d`.
pub fn scan_compatibility_tools(tool_dirs: &[PathBuf]) -> Vec<CompatibilityRuntime> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for dir in tool_dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let proton_bin = path.join("proton");
            if !proton_bin.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let canon = proton_bin
                .canonicalize()
                .unwrap_or_else(|_| proton_bin.clone());
            if seen.insert(canon) {
                out.push(CompatibilityRuntime {
                    id: format!("proton:{}", proton_bin.display()),
                    name,
                    kind: RuntimeKind::Proton,
                    executable: proton_bin,
                });
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Procura runners do Lutris (`runners/wine/*/bin/wine`).
pub fn scan_lutris_runners(wine_dirs: &[PathBuf]) -> Vec<CompatibilityRuntime> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for dir in wine_dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let wine_bin = path.join("bin/wine");
            if !wine_bin.is_file() {
                continue;
            }
            let name = format!("Lutris: {}", entry.file_name().to_string_lossy());
            let canon = wine_bin.canonicalize().unwrap_or_else(|_| wine_bin.clone());
            if seen.insert(canon) {
                out.push(CompatibilityRuntime {
                    id: format!("wine:{}", wine_bin.display()),
                    name,
                    kind: RuntimeKind::Wine,
                    executable: wine_bin,
                });
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Detecta o Wine do sistema pelo PATH ou executáveis conhecidos.
pub fn detect_system_wine(
    which_fn: impl Fn(&str) -> Option<PathBuf>,
) -> Option<CompatibilityRuntime> {
    which_fn("wine").map(|p| CompatibilityRuntime {
        id: "system-wine".into(),
        name: "Wine do sistema".into(),
        kind: RuntimeKind::Wine,
        executable: p,
    })
}

/// Opções de descoberta de runtimes para testes e execução no SO.
#[derive(Debug, Default)]
pub struct DetectOptions {
    pub system_wine: Option<PathBuf>,
    pub steam_libraries: Vec<PathBuf>,
    pub compat_tool_dirs: Vec<PathBuf>,
    pub lutris_runner_dirs: Vec<PathBuf>,
}

/// Reúne todos os runtimes disponíveis na ordem: Wine do sistema, Proton e Lutris.
pub fn detect_all(opts: &DetectOptions) -> Vec<CompatibilityRuntime> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    if let Some(p) = &opts.system_wine {
        let canon = p.canonicalize().unwrap_or_else(|_| p.clone());
        if seen.insert(canon) {
            out.push(CompatibilityRuntime {
                id: "system-wine".into(),
                name: "Wine do sistema".into(),
                kind: RuntimeKind::Wine,
                executable: p.clone(),
            });
        }
    }

    for r in scan_steam_common_proton(&opts.steam_libraries) {
        let canon = r
            .executable
            .canonicalize()
            .unwrap_or_else(|_| r.executable.clone());
        if seen.insert(canon) {
            out.push(r);
        }
    }

    for r in scan_compatibility_tools(&opts.compat_tool_dirs) {
        let canon = r
            .executable
            .canonicalize()
            .unwrap_or_else(|_| r.executable.clone());
        if seen.insert(canon) {
            out.push(r);
        }
    }

    for r in scan_lutris_runners(&opts.lutris_runner_dirs) {
        let canon = r
            .executable
            .canonicalize()
            .unwrap_or_else(|_| r.executable.clone());
        if seen.insert(canon) {
            out.push(r);
        }
    }

    out
}

/// Pastas padrão da Steam no Linux.
pub fn steam_roots(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join(".local/share/Steam"),
        home.join(".steam/steam"),
        home.join(".steam/root"),
        home.join(".var/app/com.valvesoftware.Steam/data/Steam"),
        home.join(".var/app/com.valvesoftware.Steam/.steam/steam"),
    ]
}

/// Encontra a raiz primária da Steam existente no Linux.
pub fn find_steam_root(home: Option<&Path>) -> Option<PathBuf> {
    let home = home
        .map(Path::to_path_buf)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))?;
    steam_roots(&home)
        .into_iter()
        .find(|p| p.join("steamapps").is_dir())
}

/// Detecta os runtimes reais na máquina (somente leitura).
pub fn detect_runtimes() -> Vec<CompatibilityRuntime> {
    let home = std::env::var_os("HOME").map(PathBuf::from);

    // Wine do sistema
    let system_wine = crate::unix::run::has_tool("wine").then(|| {
        for candidate in ["/usr/bin/wine", "/usr/local/bin/wine", "/bin/wine"] {
            let p = PathBuf::from(candidate);
            if p.is_file() {
                return p;
            }
        }
        PathBuf::from("wine")
    });
    let mut opts = DetectOptions {
        system_wine,
        ..Default::default()
    };

    if let Some(home) = &home {
        let xdg_data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/share"));

        let roots = steam_roots(home);
        for root in &roots {
            if !root.is_dir() {
                continue;
            }
            opts.steam_libraries.push(root.clone());
            let vdf = root.join("steamapps/libraryfolders.vdf");
            if let Ok(content) = std::fs::read_to_string(&vdf) {
                opts.steam_libraries.extend(parse_library_folders(&content));
            }
            let tools = root.join("compatibilitytools.d");
            if tools.is_dir() && !opts.compat_tool_dirs.contains(&tools) {
                opts.compat_tool_dirs.push(tools);
            }
        }

        let usr_steam_tools = PathBuf::from("/usr/share/steam/compatibilitytools.d");
        if usr_steam_tools.is_dir() {
            opts.compat_tool_dirs.push(usr_steam_tools);
        }

        let lutris_native = xdg_data.join("lutris/runners/wine");
        if lutris_native.is_dir() {
            opts.lutris_runner_dirs.push(lutris_native);
        }
        let lutris_flatpak = home.join(".var/app/net.lutris.Lutris/data/lutris/runners/wine");
        if lutris_flatpak.is_dir() {
            opts.lutris_runner_dirs.push(lutris_flatpak);
        }
    }

    detect_all(&opts)
}

/// Resolve um identificador salvo para um `CompatibilityRuntime`.
pub fn resolve_runtime(
    id_or_path: &str,
    detected: &[CompatibilityRuntime],
) -> Option<CompatibilityRuntime> {
    let clean = id_or_path.trim();
    if clean.is_empty() {
        return None;
    }
    // Casa por ID exato
    if let Some(found) = detected.iter().find(|r| r.id == clean) {
        return Some(found.clone());
    }
    // Casa por caminho exato do executável
    let path = clean
        .strip_prefix("proton:")
        .or_else(|| clean.strip_prefix("wine:"))
        .unwrap_or(clean);
    let p = Path::new(path);
    if let Some(found) = detected.iter().find(|r| r.executable == p) {
        return Some(found.clone());
    }
    // Caso especial: se o executável ainda existe em disco mesmo que não listado no scan
    if p.is_file() {
        let name = p
            .parent()
            .and_then(|parent| parent.file_name())
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_else(|| clean.to_owned());
        let kind = if clean.starts_with("proton:") || p.ends_with("proton") {
            RuntimeKind::Proton
        } else {
            RuntimeKind::Wine
        };
        return Some(CompatibilityRuntime {
            id: clean.to_owned(),
            name,
            kind,
            executable: p.to_path_buf(),
        });
    }
    // Caso especial: system-wine
    if clean == "system-wine" || clean == "wine" {
        return Some(CompatibilityRuntime {
            id: "system-wine".into(),
            name: "Wine do sistema".into(),
            kind: RuntimeKind::Wine,
            executable: PathBuf::from("wine"),
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_validation_rejects_root_and_home() {
        let home = Path::new("/home/usuario");
        assert_eq!(
            validate_prefix_path(Path::new("/"), Some(home)),
            Err(PrefixValidationError::RootNotAllowed)
        );
        assert_eq!(
            validate_prefix_path(Path::new("///"), Some(home)),
            Err(PrefixValidationError::RootNotAllowed)
        );
        assert_eq!(
            validate_prefix_path(Path::new(""), Some(home)),
            Err(PrefixValidationError::Empty)
        );
        assert_eq!(
            validate_prefix_path(Path::new("   "), Some(home)),
            Err(PrefixValidationError::Empty)
        );
        assert_eq!(
            validate_prefix_path(Path::new("/home/usuario"), Some(home)),
            Err(PrefixValidationError::HomeNotAllowed)
        );
        assert_eq!(
            validate_prefix_path(Path::new("/home/usuario/"), Some(home)),
            Err(PrefixValidationError::HomeNotAllowed)
        );
        assert_eq!(
            expand_and_validate_prefix("~", Some(home)),
            Err(PrefixValidationError::HomeNotAllowed)
        );
        assert_eq!(
            expand_and_validate_prefix("  /  ", Some(home)),
            Err(PrefixValidationError::RootNotAllowed)
        );

        // Caminhos válidos
        assert!(
            validate_prefix_path(Path::new("/home/usuario/prefixos/jogo1"), Some(home)).is_ok()
        );
        assert!(validate_prefix_path(Path::new("/mnt/jogos/winepfx"), Some(home)).is_ok());
        assert_eq!(
            expand_and_validate_prefix("~/prefixos/jogo1", Some(home)).unwrap(),
            PathBuf::from("/home/usuario/prefixos/jogo1")
        );
    }

    #[test]
    fn parses_library_folders_from_vdf() {
        let vdf = r#"
"libraryfolders"
{
    "0"
    {
        "path"		"/home/usuario/.local/share/Steam"
        "label"		""
    }
    "1"
    {
        "path"		"/mnt/disco/SteamLibrary"
        "label"		"Jogos"
    }
}
"#;
        let libs = parse_library_folders(vdf);
        assert_eq!(
            libs,
            vec![
                PathBuf::from("/home/usuario/.local/share/Steam"),
                PathBuf::from("/mnt/disco/SteamLibrary")
            ]
        );
    }

    #[test]
    fn detection_with_fixtures() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // 1. Steam Common Proton
        let steam_common = root.join("steam/steamapps/common");
        std::fs::create_dir_all(steam_common.join("Proton 8.0")).unwrap();
        std::fs::write(steam_common.join("Proton 8.0/proton"), "#!/bin/sh\n").unwrap();
        std::fs::create_dir_all(steam_common.join("Proton Experimental")).unwrap();
        std::fs::write(
            steam_common.join("Proton Experimental/proton"),
            "#!/bin/sh\n",
        )
        .unwrap();
        // Pasta que não é Proton
        std::fs::create_dir_all(steam_common.join("Hollow Knight")).unwrap();
        std::fs::write(steam_common.join("Hollow Knight/proton"), "#!/bin/sh\n").unwrap();

        // 2. Compatibility tools (GE-Proton)
        let compat = root.join("compatibilitytools.d");
        std::fs::create_dir_all(compat.join("GE-Proton8-25")).unwrap();
        std::fs::write(compat.join("GE-Proton8-25/proton"), "#!/bin/sh\n").unwrap();
        // Ferramenta sem o executável proton
        std::fs::create_dir_all(compat.join("Broken-Tool")).unwrap();

        // 3. Lutris runners
        let lutris = root.join("lutris/runners/wine");
        std::fs::create_dir_all(lutris.join("lutris-GE-Proton8-26-x86_64/bin")).unwrap();
        std::fs::write(
            lutris.join("lutris-GE-Proton8-26-x86_64/bin/wine"),
            "#!/bin/sh\n",
        )
        .unwrap();
        // Runner sem bin/wine
        std::fs::create_dir_all(lutris.join("broken-runner")).unwrap();

        // 4. Wine do sistema
        let sys_wine = root.join("bin/wine");
        std::fs::create_dir_all(sys_wine.parent().unwrap()).unwrap();
        std::fs::write(&sys_wine, "#!/bin/sh\n").unwrap();

        let opts = DetectOptions {
            system_wine: Some(sys_wine.clone()),
            steam_libraries: vec![root.join("steam")],
            compat_tool_dirs: vec![compat],
            lutris_runner_dirs: vec![lutris],
        };

        let detected = detect_all(&opts);
        let names: Vec<&str> = detected.iter().map(|r| r.name.as_str()).collect();

        assert_eq!(
            names,
            vec![
                "Wine do sistema",
                "Proton 8.0",
                "Proton Experimental",
                "GE-Proton8-25",
                "Lutris: lutris-GE-Proton8-26-x86_64"
            ]
        );

        let ge = resolve_runtime("proton:", &detected);
        assert!(ge.is_none());
        let ge = resolve_runtime(
            &format!(
                "proton:{}",
                root.join("compatibilitytools.d/GE-Proton8-25/proton")
                    .display()
            ),
            &detected,
        );
        assert_eq!(ge.unwrap().name, "GE-Proton8-25");
        let sys = resolve_runtime("system-wine", &detected);
        assert_eq!(sys.unwrap().name, "Wine do sistema");
    }

    #[test]
    fn command_and_environment_assembly_for_wine() {
        let runtime = CompatibilityRuntime {
            id: "system-wine".into(),
            name: "Wine do sistema".into(),
            kind: RuntimeKind::Wine,
            executable: PathBuf::from("/usr/bin/wine"),
        };
        let game = Path::new("/jogos/game.exe");
        let prefix = Path::new("/home/user/.local/share/tako/prefixes/manual_1");
        let extra_env = [("DXVK_HUD".to_string(), "fps".to_string())];

        let cfg = prepare_launch(
            &runtime,
            game,
            Some("-dx11 -windowed"),
            prefix,
            None,
            &extra_env,
        );

        assert_eq!(cfg.executable, PathBuf::from("/usr/bin/wine"));
        assert_eq!(cfg.arguments, vec!["/jogos/game.exe", "-dx11", "-windowed"]);
        assert_eq!(cfg.arguments_string(), "/jogos/game.exe -dx11 -windowed");
        let with_spaces = prepare_launch(
            &runtime,
            Path::new("/jogos/meu jogo/game.exe"),
            None,
            prefix,
            None,
            &[],
        );
        assert_eq!(
            with_spaces.arguments_string(),
            "\"/jogos/meu jogo/game.exe\""
        );
        assert!(cfg.environment.contains(&(
            "WINEPREFIX".into(),
            "/home/user/.local/share/tako/prefixes/manual_1".into()
        )));
        assert!(cfg.environment.contains(&("DXVK_HUD".into(), "fps".into())));
    }

    #[test]
    fn command_and_environment_assembly_for_proton() {
        let runtime = CompatibilityRuntime {
            id: "proton:/steam/Proton 8.0/proton".into(),
            name: "Proton 8.0".into(),
            kind: RuntimeKind::Proton,
            executable: PathBuf::from("/steam/Proton 8.0/proton"),
        };
        let game = Path::new("/jogos/game.exe");
        let prefix = Path::new("/home/user/.local/share/tako/prefixes/manual_1");
        let steam_root = Path::new("/home/user/.local/share/Steam");
        let extra_env = [("PROTON_LOG".to_string(), "1".to_string())];

        let cfg = prepare_launch(
            &runtime,
            game,
            Some("-silent"),
            prefix,
            Some(steam_root),
            &extra_env,
        );

        assert_eq!(cfg.executable, PathBuf::from("/steam/Proton 8.0/proton"));
        assert_eq!(cfg.arguments, vec!["run", "/jogos/game.exe", "-silent"]);
        assert_eq!(cfg.arguments_string(), "run /jogos/game.exe -silent");
        let with_spaces = prepare_launch(
            &runtime,
            Path::new("/jogos/meu jogo/game.exe"),
            None,
            prefix,
            Some(steam_root),
            &[],
        );
        assert_eq!(
            with_spaces.arguments_string(),
            "run \"/jogos/meu jogo/game.exe\""
        );
        assert!(cfg.environment.contains(&(
            "STEAM_COMPAT_DATA_PATH".into(),
            "/home/user/.local/share/tako/prefixes/manual_1".into()
        )));
        assert!(cfg.environment.contains(&(
            "STEAM_COMPAT_CLIENT_INSTALL_PATH".into(),
            "/home/user/.local/share/Steam".into()
        )));
        assert!(
            cfg.environment
                .contains(&("STEAM_COMPAT_APP_ID".into(), "0".into()))
        );
        assert!(cfg.environment.contains(&("PROTON_LOG".into(), "1".into())));
    }
}
