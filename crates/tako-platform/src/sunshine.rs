//! Descoberta local e edição conservadora da lista de aplicativos do Sunshine/Apollo.

use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const MARKER: &str = "[Tako] ";
const MAX_APPS_JSON: u64 = 4 * 1024 * 1024;

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
        paths.push(PathBuf::from(&home).join(".config/sunshine/apps.json"));
        paths.push(PathBuf::from(home).join(".config/Apollo/apps.json"));
    }
    paths
}

/// Estado local. A checagem de processo não abre conexões nem consulta serviços remotos.
pub fn detect() -> Detection {
    let apps_json = apps_json_candidates()
        .into_iter()
        .find(|path| path.is_file());
    let installed = apps_json.is_some()
        || known_executable_paths().iter().any(|p| p.is_file())
        || executable_on_path();
    Detection {
        installed,
        running: process_running(),
        apps_json,
    }
}

fn executable_on_path() -> bool {
    #[cfg(target_os = "windows")]
    let command = std::process::Command::new("where.exe")
        .args(["Sunshine.exe", "Apollo.exe"])
        .output();
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
    paths.extend([
        PathBuf::from("/usr/bin/sunshine"),
        PathBuf::from("/usr/local/bin/sunshine"),
        PathBuf::from("/app/bin/sunshine"),
        PathBuf::from("/usr/bin/apollo"),
        PathBuf::from("/usr/local/bin/apollo"),
    ]);
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
    ["Sunshine.exe", "Apollo.exe"].iter().any(|name| {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("IMAGENAME eq {name}"), "/NH"])
            .output()
            .is_ok_and(|output| {
                output.status.success()
                    && String::from_utf8_lossy(&output.stdout)
                        .to_ascii_lowercase()
                        .contains(&name.to_ascii_lowercase())
            })
    })
}

/// Constrói um registro de `apps.json`; a identidade do jogo segue pela rota validada do Tako.
pub fn app_entry(app: &App) -> Result<Value, String> {
    if app.id.is_empty()
        || app.id.len() > 256
        || !app
            .id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
    {
        return Err("O identificador do jogo não pode ser aberto por um link do Tako".into());
    }
    if app.executable.contains('"') {
        return Err(
            "O caminho do executável contém aspas duplas, não suportadas pelo Sunshine".into(),
        );
    }
    let uri = format!("tako://play/{}", app.id);
    let command = format!(
        "{} --open-url {}",
        quote_argument(&app.executable, cfg!(windows)),
        quote_argument(&uri, cfg!(windows))
    );
    Ok(json!({
        "name": format!("{MARKER}{}", app.title),
        "cmd": command,
        "working-dir": Path::new(&app.executable)
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
    }))
}

/// Mantém todos os campos da configuração e todas as entradas que não pertencem ao Tako.
pub fn merge_apps_json(contents: &str, entries: &[Value]) -> Result<String, String> {
    let mut root = if contents.trim().is_empty() {
        json!({"env": {}, "apps": []})
    } else {
        serde_json::from_str::<Value>(contents).map_err(|e| format!("apps.json inválido: {e}"))?
    };
    let object = root
        .as_object_mut()
        .ok_or_else(|| "apps.json precisa ser um objeto JSON".to_owned())?;
    let apps = object
        .entry("apps")
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or_else(|| "apps.json precisa ter uma lista 'apps'".to_owned())?;
    if apps.iter().any(|item| !item.is_object()) {
        return Err("apps.json contém uma entrada de aplicativo inválida".into());
    }
    apps.retain(|item| !is_tako_entry(item));
    apps.extend(entries.iter().cloned());
    serde_json::to_string_pretty(&root)
        .map_err(|e| format!("Não foi possível gerar apps.json: {e}"))
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
pub fn export(path: &Path, entries: &[Value]) -> Result<PathBuf, String> {
    let bytes = if path.exists() {
        let metadata =
            std::fs::metadata(path).map_err(|e| format!("Não foi possível ler apps.json: {e}"))?;
        if metadata.len() > MAX_APPS_JSON {
            return Err("apps.json excede o limite de 4 MB".into());
        }
        std::fs::read_to_string(path).map_err(|e| format!("Não foi possível ler apps.json: {e}"))?
    } else {
        String::new()
    };
    let merged = merge_apps_json(&bytes, entries)?;
    if path.exists() {
        let backup = backup_path(path);
        std::fs::copy(path, &backup)
            .map_err(|e| format!("Não foi possível criar o backup de apps.json: {e}"))?;
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Não foi possível criar a pasta do Sunshine: {e}"))?;
    }
    std::fs::write(path, merged).map_err(|e| format!("Não foi possível gravar apps.json: {e}"))?;
    Ok(path.to_path_buf())
}

fn backup_path(path: &Path) -> PathBuf {
    let first = path.with_extension("json.bak");
    if !first.exists() {
        return first;
    }
    for index in 1..10_000 {
        let candidate = path.with_extension(format!("json.bak.{index}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    path.with_extension("json.bak.last")
}

/// Aspas próprias do shell/comando do SO que executará `cmd`.
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
    fn export_backs_up_existing_file_before_replacing_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("apps.json");
        let original = include_str!("../tests/fixtures/sunshine_apps.json");
        std::fs::write(&path, original).unwrap();
        let backup = export(&path, &[entry("Jogo")]).unwrap();
        assert_eq!(
            std::fs::read_to_string(backup.with_extension("json.bak")).unwrap(),
            original
        );
        assert!(backup.exists());
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .contains("[Tako] Jogo")
        );
    }
    #[test]
    fn rejects_executable_path_with_double_quotes() {
        assert!(
            app_entry(&App {
                id: "manual:game-1".into(),
                title: "Jogo".into(),
                executable: r#"/path/with/"quotes"/tako"#.into(),
            })
            .is_err()
        );
    }
}
