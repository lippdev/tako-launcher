//! Criação de novos temas a partir de um tema base (THEME-006).

use std::fs;
use std::io;
use std::path::Path;

use crate::manifest::{DEFAULT_ENTRY, MAX_ID_LEN, is_device_name, is_safe_id};
use crate::overrides::{
    ALLOWED_BACKGROUND_EXTENSIONS, BackgroundMode, MAX_IMAGE_BYTES, MAX_IMAGE_DIMENSION,
    ThemeTokensOverride,
};
use crate::package::validate_package;
use crate::{DEFAULT_THEME_ID, InstalledTheme, Problem};

pub const MAX_NAME_LEN: usize = 50;

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

fn reserved(id: &str) -> bool {
    id.eq_ignore_ascii_case(DEFAULT_THEME_ID)
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

/// Verifica se um tema instalado usa o `Ui` padrão (reexporta ou importa `@tako/main.slint`).
pub fn uses_default_ui(theme_dir: &Path) -> bool {
    let Ok(manifest) = validate_package(theme_dir) else {
        return false;
    };
    let entry = theme_dir.join(&manifest.entry);
    let Ok(text) = fs::read_to_string(entry) else {
        return false;
    };
    text.contains("@tako/main.slint") || text.contains("@tako/")
}

/// Verifica se o tema instalado é baseado exclusivamente em tokens (THEME-006).
pub fn is_tokens_only_theme(theme_dir: &Path) -> bool {
    theme_dir.join("tokens.toml").is_file() && uses_default_ui(theme_dir)
}

/// Opções para criação de um novo tema.
#[derive(Debug, Clone)]
pub struct CreateThemeOptions<'a> {
    pub name: &'a str,
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

    let id = derive_theme_id(options.name);
    let dest = themes_root.join(&id);

    if dest.exists() && !options.overwrite {
        return Err(CreateThemeError::AlreadyExists(id));
    }

    fs::create_dir_all(themes_root)?;
    let staging = tempfile::Builder::new()
        .prefix(".create-")
        .tempdir_in(themes_root)?;

    let version = options.version.unwrap_or("1.0.0");
    let author = options.author.unwrap_or("Você");
    let desc_field = if let Some(d) = options.description {
        format!("description = \"{}\"\n", d.replace('"', "\\\""))
    } else {
        String::new()
    };

    let manifest_toml = format!(
        "id = \"{id}\"\nname = \"{}\"\nversion = \"{version}\"\nauthor = \"{}\"\napi = 1\nentry = \"{DEFAULT_ENTRY}\"\n{desc_field}",
        options.name.replace('"', "\\\""),
        author.replace('"', "\\\""),
    );

    fs::write(staging.path().join("theme.toml"), manifest_toml)?;

    let slint_content = "export { App, Ui, Hint, PanelRow } from \"@tako/main.slint\";\n";
    fs::write(staging.path().join(DEFAULT_ENTRY), slint_content)?;

    let mut tokens = options.tokens.clone();

    // Copia e valida imagem de fundo se informada
    if let Some(img_src) = options.background_image_source {
        if !img_src.is_file() {
            return Err(CreateThemeError::InvalidImage);
        }
        let ext = img_src
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();
        if !ALLOWED_BACKGROUND_EXTENSIONS.contains(&ext.as_str()) {
            return Err(CreateThemeError::InvalidImage);
        }
        let meta = fs::metadata(img_src)?;
        if meta.len() == 0 || meta.len() > MAX_IMAGE_BYTES {
            return Err(CreateThemeError::InvalidImage);
        }

        // Valida dimensões da imagem
        if let Ok(reader) = image::ImageReader::open(img_src) {
            if let Ok(dimensions) = reader.into_dimensions() {
                if dimensions.0 > MAX_IMAGE_DIMENSION || dimensions.1 > MAX_IMAGE_DIMENSION {
                    return Err(CreateThemeError::InvalidImage);
                }
            } else {
                return Err(CreateThemeError::InvalidImage);
            }
        } else {
            return Err(CreateThemeError::InvalidImage);
        }

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

    // Troca atômica
    let old = themes_root.join(format!(".old-{id}"));
    let _ = fs::remove_dir_all(&old);
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

    tracing::info!(id = %id, name = %options.name, "novo tema criado com sucesso");

    Ok(InstalledTheme {
        dir: dest,
        manifest: Ok(manifest),
    })
}
