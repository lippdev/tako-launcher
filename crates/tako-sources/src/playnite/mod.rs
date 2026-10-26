//! Importação da biblioteca do Playnite (port de `Library/Playnite`): leitor do LiteDB 4, a biblioteca,
//! a prévia da importação e onde o Playnite guarda os dados.

pub mod import;
pub mod library;
pub mod litedb;
#[cfg(test)]
pub(crate) mod sample;

use std::path::{Path, PathBuf};

/// Pastas candidatas, na ordem: o `DatabasePath` do `config.json` (variáveis resolvidas por `expand`)
/// e `<appdata>\Playnite\library`. `config.json` inválido é ignorado.
pub fn candidates(
    app_data: &Path,
    config_json: Option<&str>,
    expand: impl Fn(&str) -> String,
) -> Vec<PathBuf> {
    let mut folders = Vec::new();
    // config.json sendo gravado ou corrompido: fica a pasta padrão.
    if let Some(Ok(serde_json::Value::Object(root))) =
        config_json.map(serde_json::from_str::<serde_json::Value>)
        && let Some(path) = root.get("DatabasePath").and_then(|v| v.as_str())
        && !path.is_empty()
        && !path.to_ascii_lowercase().contains("{playnitedir}")
    {
        folders.push(PathBuf::from(expand(path)));
    }
    let default = app_data.join("Playnite").join("library");
    if !folders.iter().any(|f| {
        f.to_string_lossy()
            .eq_ignore_ascii_case(&default.to_string_lossy())
    }) {
        folders.push(default);
    }
    folders
}

/// A pasta `library` do Playnite instalado (`%APPDATA%\Playnite`); `None` se não está neste PC. A
/// versão portátil guarda a biblioteca na própria pasta: o usuário a escolhe e ela vai direto para
/// [`library::PlayniteLibrary::read`].
pub fn find(app_data: &Path) -> Option<PathBuf> {
    let config = crate::read_limited(
        &app_data.join("Playnite").join("config.json"),
        crate::MAX_STORE_FILE,
    )
    .ok()
    .map(|b| crate::text_of(&b));
    candidates(app_data, config.as_deref(), expand_env)
        .into_iter()
        .find(|f| f.join("games.db").is_file())
}

/// `%VAR%` do Windows trocado pelo valor (variável desconhecida fica como está).
fn expand_env(path: &str) -> String {
    let mut out = String::new();
    let mut rest = path;
    while let Some(start) = rest.find('%') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('%') else {
            break;
        };
        out.push_str(&rest[..start]);
        match std::env::var(&after[..end]) {
            Ok(v) if end > 0 => out.push_str(&v),
            _ => out.push_str(&rest[start..start + end + 2]),
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locator_prefers_the_database_path_from_config() {
        let app = Path::new("/appdata");
        let c = candidates(app, Some(r#"{"DatabasePath":"%TAKO%\\biblioteca"}"#), |p| {
            p.replace("%TAKO%", "E:")
        });
        assert_eq!(
            c,
            [
                PathBuf::from(r"E:\biblioteca"),
                app.join("Playnite").join("library")
            ]
        );
        assert_eq!(
            candidates(app, Some("{ inválido"), |p| p.to_owned()).len(),
            1
        );
        assert_eq!(
            candidates(
                app,
                Some(r#"{"DatabasePath":"{PlayniteDir}\\library"}"#),
                |p| p.to_owned()
            )
            .len(),
            1
        );
    }

    #[test]
    fn env_variables_expand_like_windows() {
        assert_eq!(expand_env("%TAKO_NAO_EXISTE%\\x"), "%TAKO_NAO_EXISTE%\\x");
        assert_eq!(expand_env("100% certo"), "100% certo");
        let home = std::env::var("PATH").unwrap_or_default();
        assert_eq!(expand_env("%PATH%"), home);
    }

    #[test]
    fn find_reads_the_default_folder() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(find(dir.path()), None);
        let lib = dir.path().join("Playnite").join("library");
        sample::write(&lib);
        assert_eq!(find(dir.path()), Some(lib));
    }
}
