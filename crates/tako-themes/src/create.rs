//! Criação de novos temas a partir de um tema base (THEME-006).

use std::fs;
use std::io;
use std::path::Path;

use crate::manifest::{DEFAULT_ENTRY, MAX_ID_LEN, ThemeManifest, is_device_name, is_safe_id};
use crate::overrides::{BackgroundMode, ThemeTokensOverride};
use crate::package::validate_package;
use crate::{DEFAULT_THEME_ID, InstalledTheme, Problem};

pub const MAX_NAME_LEN: usize = 50;

/// Identificadores de temas reservados pelo sistema (embutidos e extras).
pub const RESERVED_THEME_IDS: &[&str] = &[
    DEFAULT_THEME_ID,
    "tako-default",
    "tako-fita",
    "tako-alto-contraste",
];

/// Confere se um identificador é reservado pelo Tako.
pub fn is_reserved_theme_id(id: &str) -> bool {
    RESERVED_THEME_IDS
        .iter()
        .any(|r| r.eq_ignore_ascii_case(id))
}

fn reserved(id: &str) -> bool {
    is_reserved_theme_id(id)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeNameProblem {
    Empty,
    TooLong(usize),
    InvalidCharacters,
}

/// Valida o nome de um tema digitado pela pessoa usuária.
pub fn validate_theme_name(name: &str) -> Result<(), ThemeNameProblem> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(ThemeNameProblem::Empty);
    }
    if trimmed.chars().count() > MAX_NAME_LEN {
        return Err(ThemeNameProblem::TooLong(MAX_NAME_LEN));
    }
    if trimmed.chars().any(|c| {
        matches!(
            c,
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' | '\n' | '\r' | '\t'
        )
    }) || trimmed.contains("..")
    {
        return Err(ThemeNameProblem::InvalidCharacters);
    }
    Ok(())
}

/// Transforma o nome em um id seguro em minúsculas (alfanumérico, hífen e ponto).
pub fn derive_theme_id(name: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = false;

    for c in name.chars() {
        let mapped = match c {
            'A'..='Z' => c.to_ascii_lowercase(),
            'a'..='z' | '0'..='9' | '_' | '.' => c,
            ' ' | '-' => '-',
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            _ => '-',
        };

        if mapped == '-' {
            if !prev_dash && !slug.is_empty() {
                slug.push('-');
                prev_dash = true;
            }
        } else {
            slug.push(mapped);
            prev_dash = false;
        }
    }

    let trimmed = slug.trim_matches(|c| c == '-' || c == '.');
    let mut id = if trimmed.is_empty() {
        "meu-tema".to_owned()
    } else {
        trimmed.to_owned()
    };

    if id.len() > MAX_ID_LEN {
        id.truncate(MAX_ID_LEN);
        let cleaned = id.trim_matches(|c| c == '-' || c == '.');
        id = if cleaned.is_empty() {
            "meu-tema".to_owned()
        } else {
            cleaned.to_owned()
        };
    }

    if is_device_name(&id) || reserved(&id) || !is_safe_id(&id) {
        id = format!("{id}-tema");
        if id.len() > MAX_ID_LEN {
            id.truncate(MAX_ID_LEN);
        }
    }

    if !is_safe_id(&id) || reserved(&id) {
        "meu-tema".to_owned()
    } else {
        id
    }
}

/// Gera um id único em `themes_root` a partir do nome.
pub fn unique_theme_id(themes_root: &Path, name: &str) -> String {
    let base = derive_theme_id(name);
    if !themes_root.join(&base).exists() && !reserved(&base) {
        return base;
    }
    for i in 2..10000 {
        let candidate = format!("{base}-{i}");
        if !themes_root.join(&candidate).exists() && !reserved(&candidate) {
            return candidate;
        }
    }
    format!("{base}-novo")
}

/// Verifica se um tema instalado usa o `Ui` padrão (reexporta `App` e `Ui` de `@tako/main.slint`).
pub fn uses_default_ui(theme_dir: &Path) -> bool {
    let Ok(manifest) = validate_package(theme_dir) else {
        return false;
    };
    let entry = theme_dir.join(&manifest.entry);
    let Ok(text) = fs::read_to_string(entry) else {
        return false;
    };
    reexports_default_main_slint(&text)
}

fn reexports_default_main_slint(text: &str) -> bool {
    for part in text.split("export") {
        if let Some((inside, rest)) = part.split_once('}') {
            let Some(open_idx) = inside.find('{') else {
                continue;
            };
            let exported_items = &inside[open_idx + 1..];
            let rest_trimmed = rest.trim_start();
            if let Some(from_part) = rest_trimmed.strip_prefix("from") {
                let from_trimmed = from_part.trim_start();
                if from_trimmed.starts_with("\"@tako/main.slint\"")
                    || from_trimmed.starts_with("'@tako/main.slint'")
                {
                    let items: Vec<&str> = exported_items.split(',').map(|s| s.trim()).collect();
                    let has_app = items.iter().any(|&item| {
                        item == "App" || item.starts_with("App as ") || item.ends_with(" as App")
                    });
                    let has_ui = items.iter().any(|&item| {
                        item == "Ui" || item.starts_with("Ui as ") || item.ends_with(" as Ui")
                    });
                    if has_app && has_ui {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Verifica se o tema instalado é baseado exclusivamente em tokens (THEME-006).
pub fn is_tokens_only_theme(theme_dir: &Path) -> bool {
    theme_dir.join("tokens.toml").is_file() && uses_default_ui(theme_dir)
}

/// Verifica se a pasta contém um tema criado pelo próprio Tako (`created-by-tako = true`).
pub fn is_tako_created_theme(theme_dir: &Path) -> bool {
    validate_package(theme_dir).is_ok_and(|m| m.created_by_tako)
}

/// Opções para criação de um novo tema.
#[derive(Debug, Clone)]
pub struct CreateThemeOptions<'a> {
    pub name: &'a str,
    pub custom_id: Option<&'a str>,
    pub author: Option<&'a str>,
    pub version: Option<&'a str>,
    pub description: Option<&'a str>,
    pub tokens: &'a ThemeTokensOverride,
    pub background_image_source: Option<&'a Path>,
    pub overwrite: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum CreateThemeError {
    #[error("nome inválido: {0:?}")]
    InvalidName(ThemeNameProblem),
    #[error("o tema '{0}' já existe")]
    AlreadyExists(String),
    #[error("o tema '{0}' é reservado e não pode ser sobrescrito")]
    Reserved(String),
    #[error("o tema '{0}' não foi criado pelo Tako e não pode ser sobrescrito")]
    CannotOverwrite(String),
    #[error("falha de E/S: {0}")]
    Io(#[from] io::Error),
    #[error("imagem de fundo inválida ou excessivamente grande")]
    InvalidImage,
    #[error("pacote gerado inválido: {0:?}")]
    Package(Vec<Problem>),
}

/// Cria um novo tema instalado a partir de tokens e opções informadas.
pub fn create_theme(
    themes_root: &Path,
    options: &CreateThemeOptions,
) -> Result<InstalledTheme, CreateThemeError> {
    validate_theme_name(options.name).map_err(CreateThemeError::InvalidName)?;

    let id = if let Some(cid) = options.custom_id {
        if !is_safe_id(cid) {
            return Err(CreateThemeError::InvalidName(
                ThemeNameProblem::InvalidCharacters,
            ));
        }
        cid.to_owned()
    } else {
        derive_theme_id(options.name)
    };

    if is_reserved_theme_id(&id) {
        return Err(CreateThemeError::Reserved(id));
    }

    let dest = themes_root.join(&id);

    if dest.exists() {
        if !options.overwrite {
            return Err(CreateThemeError::AlreadyExists(id));
        }
        if !is_tako_created_theme(&dest) {
            return Err(CreateThemeError::CannotOverwrite(id));
        }
    }

    fs::create_dir_all(themes_root)?;
    let staging = tempfile::Builder::new()
        .prefix(".create-")
        .tempdir_in(themes_root)?;

    let version = options.version.unwrap_or("1.0.0");
    let author = options.author.unwrap_or("");

    // Constrói manifesto com o crate toml (sem format!)
    let mut manifest_table = toml::Table::new();
    manifest_table.insert("id".into(), toml::Value::String(id.clone()));
    manifest_table.insert("name".into(), toml::Value::String(options.name.to_owned()));
    manifest_table.insert("version".into(), toml::Value::String(version.to_owned()));
    manifest_table.insert("author".into(), toml::Value::String(author.to_owned()));
    manifest_table.insert("api".into(), toml::Value::Integer(1));
    manifest_table.insert("entry".into(), toml::Value::String(DEFAULT_ENTRY.into()));
    manifest_table.insert("created-by-tako".into(), toml::Value::Boolean(true));
    if let Some(desc) = options.description {
        manifest_table.insert("description".into(), toml::Value::String(desc.to_owned()));
    }
    let manifest_toml =
        toml::to_string_pretty(&manifest_table).map_err(|e| io::Error::other(e.to_string()))?;
    fs::write(staging.path().join("theme.toml"), manifest_toml)?;

    let slint_content = "export { App, Ui, Hint, PanelRow } from \"@tako/main.slint\";\n";
    fs::write(staging.path().join(DEFAULT_ENTRY), slint_content)?;

    let mut tokens = options.tokens.clone();
    // Zera background_source_name antes de gravar para não vazar nome local e não falhar no theme check
    tokens.background_source_name = None;

    // Copia e valida imagem de fundo se informada, usando validate_background_image comum
    if let Some(img_src) = options.background_image_source {
        let ext = crate::overrides::validate_background_image(img_src)
            .map_err(|_| CreateThemeError::InvalidImage)?;

        let dest_img_name = format!("background.{ext}");
        fs::copy(img_src, staging.path().join(&dest_img_name))?;

        tokens.background_image = Some(dest_img_name);
        tokens.background_mode = Some(BackgroundMode::Image);
        tokens.canvas = None;
    }

    let tokens_toml = tokens.to_toml_string();
    fs::write(staging.path().join("tokens.toml"), tokens_toml)?;

    // Valida que o tema na pasta temporária é íntegro
    let manifest = validate_package(staging.path()).map_err(CreateThemeError::Package)?;

    // Trata .old-<id> de queda anterior: só apaga se for de fato a cópia antiga desse id
    let old = themes_root.join(format!(".old-{id}"));
    if old.exists() {
        let is_old_copy = old.is_dir()
            && fs::read_to_string(old.join("theme.toml"))
                .ok()
                .and_then(|t| ThemeManifest::parse(&t).ok())
                .is_some_and(|m| m.id == id);
        if is_old_copy {
            let _ = fs::remove_dir_all(&old);
        } else {
            for i in 1..1000 {
                let backup = themes_root.join(format!(".old-{id}-bak-{i}"));
                if !backup.exists() {
                    let _ = fs::rename(&old, &backup);
                    break;
                }
            }
        }
    }

    let replacing = dest.exists();
    if replacing {
        fs::rename(&dest, &old)?;
    }

    if let Err(e) = fs::rename(staging.path(), &dest) {
        if replacing {
            let _ = fs::rename(&old, &dest);
        }
        return Err(CreateThemeError::Io(e));
    }
    let _ = fs::remove_dir_all(&old);

    // Se estava substituindo um tema existente, redefine suas sobreposições antigas
    if replacing {
        let _ = crate::overrides::reset_override(themes_root, Some(&id));
    }

    tracing::info!(id = %id, name = %options.name, "novo tema criado com sucesso");

    Ok(InstalledTheme {
        dir: dest,
        manifest: Ok(manifest),
    })
}
