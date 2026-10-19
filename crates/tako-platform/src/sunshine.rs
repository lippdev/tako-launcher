//! Descoberta local e edição conservadora da lista de aplicativos do Sunshine/Apollo.

use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const MARKER: &str = "[Tako] ";
const MAX_APPS_JSON: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SunshineError {
    InvalidGameId,
    InvalidExecutablePath,
    ExecutableNotFound(String),
    AppsJsonMalformed(String),
    AppsJsonNotAnObject,
    AppsListMissing,
    InvalidAppEntry,
    AppsJsonTooLarge,
    CannotReadAppsJson(String),
    CannotCreateBackup(String),
    CannotCreateFolder(String),
    PermissionDenied,
    CannotWriteAppsJson(String),
    SerializationFailed(String),
    SunshineFolderMissing,
}

impl std::fmt::Display for SunshineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidGameId => write!(f, "Identificador de jogo inválido para o link do Tako"),
            Self::InvalidExecutablePath => write!(f, "Caminho do executável contém aspas duplas"),
            Self::ExecutableNotFound(e) => write!(f, "Executável do Tako não encontrado: {e}"),
            Self::AppsJsonMalformed(e) => write!(f, "apps.json malformado: {e}"),
            Self::AppsJsonNotAnObject => write!(f, "apps.json precisa ser um objeto JSON"),
            Self::AppsListMissing => write!(f, "apps.json precisa conter uma lista 'apps'"),
            Self::InvalidAppEntry => write!(f, "Entrada inválida em apps.json"),
            Self::AppsJsonTooLarge => write!(f, "apps.json excede o limite de 4 MB"),
            Self::CannotReadAppsJson(e) => write!(f, "Não foi possível ler apps.json: {e}"),
            Self::CannotCreateBackup(e) => write!(f, "Não foi possível criar backup: {e}"),
            Self::CannotCreateFolder(e) => write!(f, "Não foi possível criar pasta: {e}"),
            Self::PermissionDenied => write!(f, "Sem permissão de gravação"),
            Self::CannotWriteAppsJson(e) => write!(f, "Não foi possível gravar apps.json: {e}"),
            Self::SerializationFailed(e) => write!(f, "Falha na serialização do JSON: {e}"),
            Self::SunshineFolderMissing => write!(f, "Pasta do Sunshine não encontrada"),
        }
    }
}

impl std::error::Error for SunshineError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    pub id: String,
    pub title: String,
    pub executable: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detection {
    pub installed: bool,
    pub running: bool,
    pub apps_json: Option<PathBuf>,
}

/// Identifica se o processo está em uma sandbox Flatpak.
pub fn is_flatpak() -> bool {
    std::env::var_os("FLATPAK_ID").is_some() || Path::new("/.flatpak-info").exists()
}

/// Caminho ou comando de execução do Tako para registrar no Sunshine.
/// No AppImage reaproveita $APPIMAGE; no Flatpak usa `flatpak run <app-id>`.
pub fn tako_executable() -> Result<String, SunshineError> {
    if is_flatpak() {
        let id = std::env::var("FLATPAK_ID")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "io.github.lippdev.Tako".to_owned());
        return Ok(format!("flatpak run {id}"));
    }
    if let Some(appimage) = std::env::var_os("APPIMAGE").filter(|v| !v.is_empty()) {
        return Ok(appimage.to_string_lossy().into_owned());
    }
    std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| SunshineError::ExecutableNotFound(e.to_string()))
}

/// Locais padrão da configuração usada pelo Sunshine e pelo Apollo.
pub fn apps_json_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    #[cfg(target_os = "windows")]
    {
        for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(root) = std::env::var_os(variable) {
                for product in ["Sunshine", "Apollo"] {
                    paths.push(PathBuf::from(&root).join(product).join("config/apps.json"));
                }
            }
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            for product in ["Sunshine", "Apollo"] {
                paths.push(PathBuf::from(&local).join(product).join("config/apps.json"));
            }
        }
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    if let Some(home) = std::env::var_os("HOME") {
        let home_path = PathBuf::from(home);
        paths.push(home_path.join(".config/sunshine/apps.json"));
        paths.push(home_path.join(".config/Apollo/apps.json"));
        #[cfg(target_os = "linux")]
        paths
            .push(home_path.join(".var/app/dev.lizardbyte.app.Sunshine/config/sunshine/apps.json"));
    }
    paths
}

/// Localiza o primeiro apps.json existente entre os locais padrão, sem checar processos.
pub fn find_apps_json() -> Option<PathBuf> {
    apps_json_candidates()
        .into_iter()
        .find(|path| path.is_file())
}

/// Detecção rápida (apenas caminhos no disco), sem abrir processos nem verificar PATH.
pub fn detect_quick() -> (bool, Option<PathBuf>) {
    let apps_json = find_apps_json();
    let installed = apps_json.is_some() || known_executable_paths().iter().any(|p| p.is_file());
    (installed, apps_json)
}

/// Estado local. A checagem de processo e PATH roda fora da thread da UI (em segundo plano).
pub fn detect() -> Detection {
    let (quick_installed, apps_json) = detect_quick();
    let installed = quick_installed || executable_on_path();
    Detection {
        installed,
        running: process_running(),
        apps_json,
    }
}

fn executable_on_path() -> bool {
    #[cfg(target_os = "windows")]
    let command = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        std::process::Command::new("where.exe")
            .args(["Sunshine.exe", "Apollo.exe"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
    };
    #[cfg(not(target_os = "windows"))]
    let command = std::process::Command::new("which")
        .args(["sunshine", "apollo"])
        .output();
    command.is_ok_and(|output| {
        output.status.success()
            && String::from_utf8_lossy(&output.stdout)
                .lines()
                .any(|line| Path::new(line.trim()).is_file())
    })
}

fn known_executable_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    #[cfg(target_os = "windows")]
    {
        for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(root) = std::env::var_os(variable) {
                for product in ["Sunshine", "Apollo"] {
                    paths.push(
                        PathBuf::from(&root)
                            .join(product)
                            .join(format!("{product}.exe")),
                    );
                }
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        paths.extend([
            PathBuf::from("/usr/bin/sunshine"),
            PathBuf::from("/usr/local/bin/sunshine"),
            PathBuf::from("/app/bin/sunshine"),
            PathBuf::from("/usr/bin/apollo"),
            PathBuf::from("/usr/local/bin/apollo"),
            PathBuf::from("/var/lib/flatpak/app/dev.lizardbyte.app.Sunshine"),
        ]);
        if let Some(home) = std::env::var_os("HOME") {
            paths.push(
                PathBuf::from(home).join(".local/share/flatpak/app/dev.lizardbyte.app.Sunshine"),
            );
        }
    }
    #[cfg(target_os = "macos")]
    paths.extend([
        PathBuf::from("/Applications/Sunshine.app"),
        PathBuf::from("/Applications/Apollo.app"),
        PathBuf::from("/opt/homebrew/bin/sunshine"),
        PathBuf::from("/usr/local/bin/sunshine"),
        PathBuf::from("/opt/homebrew/bin/apollo"),
        PathBuf::from("/usr/local/bin/apollo"),
    ]);
    paths
}

#[cfg(target_os = "linux")]
fn process_running() -> bool {
    let Ok(processes) = std::fs::read_dir("/proc") else {
        return false;
    };
    processes.flatten().any(|entry| {
        entry.file_name().to_string_lossy().parse::<u32>().is_ok()
            && std::fs::read(entry.path().join("comm")).is_ok_and(|comm| {
                matches!(
                    String::from_utf8_lossy(&comm)
                        .trim()
                        .to_ascii_lowercase()
                        .as_str(),
                    "sunshine" | "apollo"
                )
            })
    })
}

#[cfg(target_os = "macos")]
fn process_running() -> bool {
    ["sunshine", "apollo"].iter().any(|name| {
        std::process::Command::new("pgrep")
            .args(["-x", name])
            .status()
            .is_ok_and(|status| status.success())
    })
}

#[cfg(target_os = "windows")]
fn process_running() -> bool {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    ["Sunshine.exe", "Apollo.exe"].iter().any(|name| {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("IMAGENAME eq {name}"), "/NH"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .is_ok_and(|output| {
                output.status.success()
                    && String::from_utf8_lossy(&output.stdout)
                        .to_ascii_lowercase()
                        .contains(&name.to_ascii_lowercase())
            })
    })
}

/// Valida se o identificador de jogo é seguro e aceito para a rota do Tako.
pub fn is_valid_game_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 256
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
}

/// Identifica se o caminho do apps.json pertence a uma instalação Flatpak do Sunshine.
pub fn is_flatpak_sunshine_path(path: &Path) -> bool {
    path.to_string_lossy()
        .contains(".var/app/dev.lizardbyte.app.Sunshine")
}

/// Constrói um registro de `apps.json`; a identidade do jogo segue pela rota validada do Tako.
pub fn app_entry(app: &App) -> Result<Value, SunshineError> {
    app_entry_for_target(app, None)
}

/// Constrói um registro de `apps.json` ajustando o comando caso o Sunshine alvo esteja em Flatpak.
pub fn app_entry_for_target(app: &App, target: Option<&Path>) -> Result<Value, SunshineError> {
    if !is_valid_game_id(&app.id) {
        return Err(SunshineError::InvalidGameId);
    }
    if app.executable.contains('"') {
        return Err(SunshineError::InvalidExecutablePath);
    }
    let uri = format!("tako://play/{}", app.id);
    let (mut command, working_dir) = if app.executable.starts_with("flatpak run ") {
        (
            format!(
                "{} --open-url {}",
                app.executable,
                quote_argument(&uri, false)
            ),
            None,
        )
    } else {
        (
            format!(
                "{} --open-url {}",
                quote_argument(&app.executable, cfg!(windows)),
                quote_argument(&uri, cfg!(windows))
            ),
            Path::new(&app.executable)
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map(|p| p.to_string_lossy().into_owned()),
        )
    };
    if target.is_some_and(is_flatpak_sunshine_path) && !command.starts_with("flatpak-spawn --host ")
    {
        command = format!("flatpak-spawn --host {command}");
    }
    let mut entry = serde_json::Map::new();
    entry.insert(
        "name".into(),
        Value::String(format!("{MARKER}{}", app.title)),
    );
    entry.insert("cmd".into(), Value::String(command));
    if let Some(wd) = working_dir {
        entry.insert("working-dir".into(), Value::String(wd));
    }
    Ok(Value::Object(entry))
}

/// Mantém todos os campos da configuração e todas as entradas que não pertencem ao Tako.
pub fn merge_apps_json(contents: &str, entries: &[Value]) -> Result<String, SunshineError> {
    let mut root = if contents.trim().is_empty() {
        json!({"env": {}, "apps": []})
    } else {
        serde_json::from_str::<Value>(contents)
            .map_err(|e| SunshineError::AppsJsonMalformed(e.to_string()))?
    };
    let object = root
        .as_object_mut()
        .ok_or(SunshineError::AppsJsonNotAnObject)?;
    let apps = object
        .entry("apps")
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or(SunshineError::AppsListMissing)?;
    if apps.iter().any(|item| !item.is_object()) {
        return Err(SunshineError::InvalidAppEntry);
    }
    apps.retain(|item| !is_tako_entry(item));
    apps.extend(entries.iter().cloned());
    serde_json::to_string_pretty(&root)
        .map_err(|e| SunshineError::SerializationFailed(e.to_string()))
}

fn is_tako_entry(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    object
        .get("name")
        .and_then(Value::as_str)
        .is_some_and(|name| name.starts_with(MARKER))
        && object
            .get("cmd")
            .and_then(Value::as_str)
            .is_some_and(|cmd| cmd.contains("tako://play/"))
}

/// Faz backup sem substituir uma cópia anterior e grava apenas depois da validação do documento inteiro.
pub fn export(path: &Path, entries: &[Value]) -> Result<PathBuf, SunshineError> {
    let bytes = if path.exists() {
        let metadata = std::fs::metadata(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                SunshineError::PermissionDenied
            } else {
                SunshineError::CannotReadAppsJson(e.to_string())
            }
        })?;
        if metadata.len() > MAX_APPS_JSON {
            return Err(SunshineError::AppsJsonTooLarge);
        }
        std::fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                SunshineError::PermissionDenied
            } else {
                SunshineError::CannotReadAppsJson(e.to_string())
            }
        })?
    } else {
        String::new()
    };
    let merged = merge_apps_json(&bytes, entries)?;
    if path.exists() {
        let backup = backup_path(path);
        std::fs::copy(path, &backup).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                SunshineError::PermissionDenied
            } else {
                SunshineError::CannotCreateBackup(e.to_string())
            }
        })?;
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                SunshineError::PermissionDenied
            } else {
                SunshineError::CannotCreateFolder(e.to_string())
            }
        })?;
    }
    tako_core::metadata::cache::write_atomic(path, merged.as_bytes()).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            SunshineError::PermissionDenied
        } else {
            SunshineError::CannotWriteAppsJson(e.to_string())
        }
    })?;
    Ok(path.to_path_buf())
}

/// Guarda apenas o primeiro backup (.bak) e o último (.bak.last).
fn backup_path(path: &Path) -> PathBuf {
    let first = path.with_extension("json.bak");
    if !first.exists() {
        first
    } else {
        path.with_extension("json.bak.last")
    }
}

/// Aspas duplas usadas pelo Sunshine em todos os sistemas (boost.process).
pub fn quote_argument(value: &str, windows: bool) -> String {
    if windows {
        let mut output = String::from("\"");
        let mut slashes = 0;
        for ch in value.chars() {
            if ch == '\\' {
                slashes += 1;
            } else if ch == '"' {
                output.push_str(&"\\".repeat(slashes * 2 + 1));
                output.push('"');
                slashes = 0;
            } else {
                output.push_str(&"\\".repeat(slashes));
                slashes = 0;
                output.push(ch);
            }
        }
        output.push_str(&"\\".repeat(slashes * 2));
        output.push('"');
        output
    } else {
        format!("\"{value}\"")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(title: &str) -> Value {
        app_entry(&App {
            id: "manual:game-1".into(),
            title: title.into(),
            executable: "/Applications/Tako Launcher.app/Contents/MacOS/tako".into(),
        })
        .unwrap()
    }

    #[test]
    fn preserves_user_entries_and_replaces_only_tako_entries() {
        let fixture = include_str!("../tests/fixtures/sunshine_apps.json");
        let merged = merge_apps_json(fixture, &[entry("Nova versão")]).unwrap();
        let value: Value = serde_json::from_str(&merged).unwrap();
        assert_eq!(value["apps"][0]["name"], "Desktop");
        assert_eq!(value["apps"][1]["name"], "Ferramenta do usuário");
        assert_eq!(value["apps"][2]["name"], "[Tako] Marcador editado");
        assert_eq!(value["apps"][3]["name"], "[Tako] Nova versão");
    }

    #[test]
    fn export_is_idempotent() {
        let once = merge_apps_json(r#"{"apps":[]}"#, &[entry("Jogo")]).unwrap();
        let twice = merge_apps_json(&once, &[entry("Jogo")]).unwrap();
        let count = |s: &str| {
            serde_json::from_str::<Value>(s).unwrap()["apps"]
                .as_array()
                .unwrap()
                .len()
        };
        assert_eq!(count(&once), 1);
        assert_eq!(count(&twice), 1);
        assert_eq!(once, twice);
    }

    #[test]
    fn command_quotes_paths_and_uri_arguments_for_both_shells() {
        let posix = entry("Ação 'especial'");
        let command = posix["cmd"].as_str().unwrap();
        assert!(command.contains("\"/Applications/Tako Launcher.app/Contents/MacOS/tako\""));
        assert!(command.ends_with("\"tako://play/manual:game-1\""));
        let encoded = serde_json::to_string(&posix).unwrap();
        let decoded: Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded["cmd"], command);
        assert_eq!(
            decoded["working-dir"],
            "/Applications/Tako Launcher.app/Contents/MacOS"
        );
        assert_eq!(
            quote_argument(r#"C:\Program Files\Tako\tako.exe"#, true),
            r#""C:\Program Files\Tako\tako.exe""#
        );
        assert_eq!(
            quote_argument("tako://play/manual:1", true),
            "\"tako://play/manual:1\""
        );
        assert_eq!(
            quote_argument("/Applications/Tako.app/Contents/MacOS/tako", false),
            "\"/Applications/Tako.app/Contents/MacOS/tako\""
        );
    }

    #[test]
    fn command_formats_flatpak_executable_without_quotes() {
        let flat = app_entry(&App {
            id: "manual:game-1".into(),
            title: "Jogo".into(),
            executable: "flatpak run dev.tako.Tako".into(),
        })
        .unwrap();
        assert_eq!(
            flat["cmd"],
            "flatpak run dev.tako.Tako --open-url \"tako://play/manual:game-1\""
        );
        assert!(!flat.as_object().unwrap().contains_key("working-dir"));
        assert!(
            !serde_json::to_string(&flat)
                .unwrap()
                .contains("working-dir")
        );
    }

    #[test]
    fn flatpak_detection_and_executable() {
        // SAFETY: teste isolado para validar detecção e executável em Flatpak
        unsafe { std::env::set_var("FLATPAK_ID", "io.github.lippdev.Tako") };
        assert!(is_flatpak());
        assert_eq!(
            tako_executable().unwrap(),
            "flatpak run io.github.lippdev.Tako"
        );
        // SAFETY: restaura o ambiente após o teste
        unsafe { std::env::remove_var("FLATPAK_ID") };
    }

    #[test]
    fn omits_working_dir_key_when_none() {
        let flat = app_entry(&App {
            id: "manual:game-1".into(),
            title: "Jogo".into(),
            executable: "flatpak run io.github.lippdev.Tako".into(),
        })
        .unwrap();
        assert!(!flat.as_object().unwrap().contains_key("working-dir"));
        let json_str = serde_json::to_string(&flat).unwrap();
        assert!(!json_str.contains("working-dir"));
    }

    #[test]
    fn generates_flatpak_spawn_host_when_sunshine_is_in_flatpak() {
        let app = App {
            id: "manual:game-1".into(),
            title: "Jogo".into(),
            executable: "/usr/bin/tako".into(),
        };
        let target =
            Path::new("/home/user/.var/app/dev.lizardbyte.app.Sunshine/config/sunshine/apps.json");
        let entry = app_entry_for_target(&app, Some(target)).unwrap();
        assert_eq!(
            entry["cmd"],
            "flatpak-spawn --host \"/usr/bin/tako\" --open-url \"tako://play/manual:game-1\""
        );
        assert_eq!(entry["working-dir"], "/usr/bin");

        let flat_app = App {
            id: "manual:game-2".into(),
            title: "Jogo 2".into(),
            executable: "flatpak run io.github.lippdev.Tako".into(),
        };
        let flat_entry = app_entry_for_target(&flat_app, Some(target)).unwrap();
        assert_eq!(
            flat_entry["cmd"],
            "flatpak-spawn --host flatpak run io.github.lippdev.Tako --open-url \"tako://play/manual:game-2\""
        );
        assert!(!flat_entry.as_object().unwrap().contains_key("working-dir"));
    }

    #[test]
    fn rejects_malformed_json_without_touching_the_file() {
        let fixture = include_str!("../tests/fixtures/sunshine_malformed.json");
        assert!(merge_apps_json(fixture, &[entry("Jogo")]).is_err());
        assert!(merge_apps_json(r#"{"apps":{}}"#, &[entry("Jogo")]).is_err());
    }

    #[test]
    fn export_refuses_malformed_file_without_changing_or_backing_it_up() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("apps.json");
        let original = include_str!("../tests/fixtures/sunshine_malformed.json");
        std::fs::write(&path, original).unwrap();
        assert!(export(&path, &[entry("Jogo")]).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        assert!(!path.with_extension("json.bak").exists());
    }

    #[test]
    fn export_backs_up_first_and_last_before_replacing_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("apps.json");
        let original = include_str!("../tests/fixtures/sunshine_apps.json");
        std::fs::write(&path, original).unwrap();
        let backup = export(&path, &[entry("Jogo 1")]).unwrap();
        assert_eq!(
            std::fs::read_to_string(backup.with_extension("json.bak")).unwrap(),
            original
        );
        assert!(backup.exists());
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .contains("[Tako] Jogo 1")
        );

        // Segundo export: grava .bak.last e não .bak.1
        export(&path, &[entry("Jogo 2")]).unwrap();
        assert!(path.with_extension("json.bak").exists());
        assert!(path.with_extension("json.bak.last").exists());
        assert!(!path.with_extension("json.bak.1").exists());
    }

    #[test]
    fn rejects_executable_path_with_double_quotes() {
        assert_eq!(
            app_entry(&App {
                id: "manual:game-1".into(),
                title: "Jogo".into(),
                executable: r#"/path/with/"quotes"/tako"#.into(),
            }),
            Err(SunshineError::InvalidExecutablePath)
        );
    }
}
