//! Sobreposição de `tokens.toml` por tema (THEME-003).
//!
//! Permite que a pessoa usuária personalize a cor de destaque (`accent`), a fonte (`font`) e o fundo (`canvas`
//! ou imagem local copiada para a pasta de dados do Tako) sem modificar os arquivos originais do tema.
//! As sobreposições ficam guardadas em `<themes>/.overrides/<id>/tokens.toml`, isoladas de cada tema e da
//! galeria de temas instalados.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::DEFAULT_THEME_ID;
use crate::manifest::is_safe_id;

pub const OVERRIDES_DIR: &str = ".overrides";
pub const TOKENS_FILE: &str = "tokens.toml";

/// Extensões de imagem permitidas para o fundo personalizado.
pub const ALLOWED_BACKGROUND_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];

/// Valida se uma string é uma cor hexadecimal válida (`#RRGGBB` ou `#RRGGBBAA`).
pub fn validate_color(s: &str) -> bool {
    let Some(hex) = s.strip_prefix('#') else {
        return false;
    };
    if hex.len() != 6 && hex.len() != 8 {
        return false;
    }
    hex.chars().all(|c| c.is_ascii_hexdigit())
}

/// Normaliza o id do tema (`None` ou `"default"` vira `"default"`).
pub fn normalize_id(id: Option<&str>) -> &str {
    match id {
        Some(s) if !s.trim().is_empty() => s.trim(),
        _ => DEFAULT_THEME_ID,
    }
}

/// Pasta de sobreposição para o tema (`<themes_root>/.overrides/<id>`).
pub fn override_dir(themes_root: &Path, theme_id: Option<&str>) -> PathBuf {
    let id = normalize_id(theme_id);
    let safe_id = if is_safe_id(id) { id } else { DEFAULT_THEME_ID };
    themes_root.join(OVERRIDES_DIR).join(safe_id)
}

/// Caminho do arquivo `tokens.toml` da sobreposição do tema.
pub fn tokens_path(themes_root: &Path, theme_id: Option<&str>) -> PathBuf {
    override_dir(themes_root, theme_id).join(TOKENS_FILE)
}

/// Informa se existe sobreposição gravada para o tema.
pub fn has_override(themes_root: &Path, theme_id: Option<&str>) -> bool {
    tokens_path(themes_root, theme_id).is_file()
}

/// Carrega o conteúdo do `tokens.toml` da sobreposição do tema, se existir.
pub fn load_override(themes_root: &Path, theme_id: Option<&str>) -> Option<String> {
    let path = tokens_path(themes_root, theme_id);
    fs::read_to_string(path).ok()
}

/// Grava a sobreposição em `<themes_root>/.overrides/<id>/tokens.toml`.
pub fn save_override(
    themes_root: &Path,
    theme_id: Option<&str>,
    toml_content: &str,
) -> io::Result<PathBuf> {
    let dir = override_dir(themes_root, theme_id);
    fs::create_dir_all(&dir)?;
    let path = dir.join(TOKENS_FILE);
    fs::write(&path, toml_content)?;
    tracing::info!(dir = %dir.display(), "sobreposição de tema gravada");
    Ok(path)
}

/// Remove a sobreposição do tema, voltando ao padrão original.
pub fn reset_override(themes_root: &Path, theme_id: Option<&str>) -> io::Result<bool> {
    let dir = override_dir(themes_root, theme_id);
    if !dir.exists() {
        return Ok(false);
    }
    fs::remove_dir_all(&dir)?;
    tracing::info!(dir = %dir.display(), "sobreposição de tema removida");
    Ok(true)
}

/// Copia uma imagem local do usuário para a pasta da sobreposição do tema.
///
/// Apenas formatos conhecidos (`png`, `jpg`, `jpeg`, `webp`) são aceitos. Devolve o nome do arquivo copiado.
pub fn copy_background_image(
    themes_root: &Path,
    theme_id: Option<&str>,
    source: &Path,
) -> io::Result<String> {
    if !source.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("arquivo não encontrado: {}", source.display()),
        ));
    }
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "arquivo sem extensão válida de imagem",
            )
        })?;

    if !ALLOWED_BACKGROUND_EXTENSIONS.contains(&ext.as_str()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("extensão .{ext} não permitida para imagem de fundo"),
        ));
    }

    let dir = override_dir(themes_root, theme_id);
    fs::create_dir_all(&dir)?;

    let target_name = format!("background.{ext}");
    let dest = dir.join(&target_name);
    fs::copy(source, &dest)?;
    tracing::info!(from = %source.display(), to = %dest.display(), "imagem de fundo copiada");
    Ok(target_name)
}

/// Representação em memória dos tokens editáveis pelo Tako.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThemeTokensOverride {
    pub accent: Option<String>,
    pub font: Option<String>,
    pub canvas: Option<String>,
    pub background_image: Option<String>,
}

impl ThemeTokensOverride {
    /// Carrega a partir do TOML salvo na sobreposição.
    pub fn from_toml_str(text: &str) -> Self {
        let Ok(table) = text.parse::<toml::Table>() else {
            return Self::default();
        };
        let get_str = |key: &str| table.get(key).and_then(|v| v.as_str()).map(str::to_owned);

        let accent = get_str("accent").filter(|c| validate_color(c));
        let font = get_str("font")
            .or_else(|| get_str("font-text"))
            .or_else(|| get_str("font-title"));
        let canvas = get_str("canvas").filter(|c| validate_color(c));
        let background_image = get_str("custom-background").or_else(|| get_str("background-image"));

        Self {
            accent,
            font,
            canvas,
            background_image,
        }
    }

    /// Converte para texto TOML compatível com `tokens.toml`.
    pub fn to_toml_string(&self) -> String {
        let mut out = String::new();
        out.push_str("# Sobreposição de tokens do tema pelo Tako (THEME-003)\n\n");
        if let Some(ref a) = self.accent {
            out.push_str(&format!("accent = \"{a}\"\n"));
        }
        if let Some(ref f) = self.font {
            out.push_str(&format!("font = \"{f}\"\n"));
            out.push_str(&format!("font-title = \"{f}\"\n"));
            out.push_str(&format!("font-text = \"{f}\"\n"));
        }
        if let Some(ref c) = self.canvas {
            out.push_str(&format!("canvas = \"{c}\"\n"));
        }
        if let Some(ref bg) = self.background_image {
            out.push_str(&format!("custom-background = \"{bg}\"\n"));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_colors() {
        assert!(validate_color("#112233"));
        assert!(validate_color("#11223344"));
        assert!(validate_color("#aabbcc"));
        assert!(validate_color("#AABBCCDD"));

        assert!(!validate_color("112233"));
        assert!(!validate_color("#123"));
        assert!(!validate_color("#12345"));
        assert!(!validate_color("#1122334455"));
        assert!(!validate_color("#zzxxcc"));
        assert!(!validate_color("red"));
    }

    #[test]
    fn saves_loads_and_resets_override() {
        let tmp = tempfile::tempdir().unwrap();
        let themes_root = tmp.path();

        assert!(!has_override(themes_root, None));
        assert_eq!(load_override(themes_root, None), None);

        let toml_sample = "accent = \"#E3161E\"\nfont = \"Poppins\"\n";
        let saved_path = save_override(themes_root, None, toml_sample).unwrap();
        assert!(saved_path.is_file());
        assert!(has_override(themes_root, None));

        let loaded = load_override(themes_root, None).unwrap();
        assert_eq!(loaded, toml_sample);

        // Também funciona com id específico de tema
        let custom_id = "autor.meu-tema";
        save_override(themes_root, Some(custom_id), toml_sample).unwrap();
        assert!(has_override(themes_root, Some(custom_id)));
        assert_eq!(
            load_override(themes_root, Some(custom_id)).unwrap(),
            toml_sample
        );

        // Reset remove a sobreposição
        assert!(reset_override(themes_root, None).unwrap());
        assert!(!has_override(themes_root, None));
        assert_eq!(load_override(themes_root, None), None);

        // Reset repetido retorna Ok(false)
        assert!(!reset_override(themes_root, None).unwrap());

        // A outra sobreposição permaneceu intacta
        assert!(has_override(themes_root, Some(custom_id)));
    }

    #[test]
    fn copies_background_image_and_validates_extensions() {
        let tmp = tempfile::tempdir().unwrap();
        let themes_root = tmp.path().join("themes");
        let src_dir = tmp.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();

        let img_file = src_dir.join("fundo.png");
        fs::write(&img_file, b"fake png bytes").unwrap();

        let copied = copy_background_image(&themes_root, None, &img_file).unwrap();
        assert_eq!(copied, "background.png");
        assert!(
            override_dir(&themes_root, None)
                .join("background.png")
                .is_file()
        );

        // Extensão inválida é rejeitada
        let bad_ext = src_dir.join("script.sh");
        fs::write(&bad_ext, b"echo hello").unwrap();
        assert!(copy_background_image(&themes_root, None, &bad_ext).is_err());

        // Arquivo inexistente é rejeitado
        let missing = src_dir.join("nao-existe.png");
        assert!(copy_background_image(&themes_root, None, &missing).is_err());
    }

    #[test]
    fn theme_tokens_override_serialization() {
        let tokens = ThemeTokensOverride {
            accent: Some("#6C5CE7".into()),
            font: Some("Plus Jakarta Sans".into()),
            canvas: Some("#0B1A33".into()),
            background_image: Some("background.jpg".into()),
        };
        let toml_str = tokens.to_toml_string();
        let parsed = ThemeTokensOverride::from_toml_str(&toml_str);
        assert_eq!(parsed, tokens);
    }
}
