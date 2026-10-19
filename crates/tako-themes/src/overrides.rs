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

pub const DRAFT_BACKGROUND_PREFIX: &str = "background.draft.";
pub const BACKGROUND_PREFIX: &str = "background.";

pub const MAX_IMAGE_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_IMAGE_DIMENSION: u32 = 16384;

/// Extensões de imagem permitidas para o fundo personalizado.
pub const ALLOWED_BACKGROUND_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];

/// Valida se uma string é uma cor hexadecimal válida de 6 dígitos (`#RRGGBB`).
pub fn validate_color(s: &str) -> bool {
    let Some(hex) = s.strip_prefix('#') else {
        return false;
    };
    if hex.len() != 6 {
        return false;
    }
    hex.chars().all(|c| c.is_ascii_hexdigit())
}

/// Valida se a cor do canvas tem contraste de pelo menos 4.5:1 com o texto padrão (branco).
pub fn validate_canvas(s: &str) -> bool {
    if !validate_color(s) {
        return false;
    }
    let Some(canvas_rgb) = tako_core::presentation::Rgb::parse(s) else {
        return false;
    };
    let text_rgb = tako_core::presentation::Rgb::new(255, 255, 255);
    tako_core::presentation::contrast(canvas_rgb, text_rgb)
        >= tako_core::presentation::contrast_rules::TEXT
}

/// Valida se o caminho de uma imagem é relativo e seguro (sem `..` nem `/` absoluto).
pub fn is_safe_background_path(path_str: &str) -> bool {
    let s = path_str.trim();
    if s.is_empty() || s.contains(':') || s.starts_with('/') || s.starts_with('\\') {
        return false;
    }
    let normalized = s.replace('\\', "/");
    let p = Path::new(&normalized);
    if p.is_absolute() {
        return false;
    }
    for comp in p.components() {
        match comp {
            std::path::Component::Normal(_) => {}
            _ => return false,
        }
    }
    true
}

/// Modo de fundo escolhido na personalização.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundMode {
    Default,
    Solid,
    Image,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CopyImageError {
    #[error("arquivo não encontrado: {0}")]
    NotFound(PathBuf),
    #[error("extensão de imagem não permitida")]
    InvalidExtension,
    #[error("conteúdo de imagem inválido ou corrompido")]
    InvalidContent,
    #[error("imagem excede os limites permitidos")]
    TooLarge,
    #[error("erro de E/S: {0}")]
    Io(String),
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

/// Grava a sobreposição em `<themes_root>/.overrides/<id>/tokens.toml` de forma atômica.
pub fn save_override(
    themes_root: &Path,
    theme_id: Option<&str>,
    toml_content: &str,
) -> io::Result<PathBuf> {
    let dir = override_dir(themes_root, theme_id);
    fs::create_dir_all(&dir)?;
    let path = dir.join(TOKENS_FILE);
    tako_core::metadata::cache::write_atomic(&path, toml_content.as_bytes())?;
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

/// Limpa qualquer imagem de rascunho (`background.draft.*`) existente na pasta da sobreposição.
pub fn clean_background_draft(themes_root: &Path, theme_id: Option<&str>) -> io::Result<()> {
    let dir = override_dir(themes_root, theme_id);
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with(DRAFT_BACKGROUND_PREFIX) {
            let _ = fs::remove_file(entry.path());
        }
    }
    Ok(())
}

/// Remove arquivos órfãos de fundo (`background.*` e `background.draft.*`), mantendo apenas `keep` se fornecido.
pub fn clean_background_orphans(
    themes_root: &Path,
    theme_id: Option<&str>,
    keep: Option<&str>,
) -> io::Result<()> {
    let dir = override_dir(themes_root, theme_id);
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with(BACKGROUND_PREFIX) {
            if let Some(keep_name) = keep {
                if name_str == keep_name {
                    continue;
                }
            }
            let _ = fs::remove_file(entry.path());
        }
    }
    Ok(())
}

/// Valida cabeçalho e limites, e copia a imagem para um rascunho `background.draft.<ext>`.
pub fn copy_draft_background_image(
    themes_root: &Path,
    theme_id: Option<&str>,
    source: &Path,
) -> Result<String, CopyImageError> {
    if !source.is_file() {
        return Err(CopyImageError::NotFound(source.to_path_buf()));
    }
    let meta = fs::metadata(source).map_err(|e| CopyImageError::Io(e.to_string()))?;
    if meta.len() > MAX_IMAGE_BYTES {
        return Err(CopyImageError::TooLarge);
    }
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or(CopyImageError::InvalidExtension)?;

    if !ALLOWED_BACKGROUND_EXTENSIONS.contains(&ext.as_str()) {
        return Err(CopyImageError::InvalidExtension);
    }

    let reader = image::ImageReader::open(source)
        .map_err(|e| CopyImageError::Io(e.to_string()))?
        .with_guessed_format()
        .map_err(|e| CopyImageError::Io(e.to_string()))?;

    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(512 * 1024 * 1024);
    let mut reader = reader;
    reader.limits(limits);

    match reader.into_dimensions() {
        Ok((w, h)) => {
            if w == 0 || h == 0 || w > MAX_IMAGE_DIMENSION || h > MAX_IMAGE_DIMENSION {
                return Err(CopyImageError::TooLarge);
            }
        }
        Err(_) => return Err(CopyImageError::InvalidContent),
    }

    let dir = override_dir(themes_root, theme_id);
    fs::create_dir_all(&dir).map_err(|e| CopyImageError::Io(e.to_string()))?;

    clean_background_draft(themes_root, theme_id).map_err(|e| CopyImageError::Io(e.to_string()))?;

    let draft_name = format!("{DRAFT_BACKGROUND_PREFIX}{ext}");
    let dest = dir.join(&draft_name);
    fs::copy(source, &dest).map_err(|e| CopyImageError::Io(e.to_string()))?;
    tracing::info!(from = %source.display(), to = %dest.display(), "imagem copiada para rascunho");
    Ok(draft_name)
}

/// Promove a imagem de rascunho para `background.<ext>` definitivo e remove órfãos.
pub fn promote_background_draft(
    themes_root: &Path,
    theme_id: Option<&str>,
    draft_filename: &str,
) -> io::Result<String> {
    let dir = override_dir(themes_root, theme_id);
    if let Some(ext) = draft_filename.strip_prefix(DRAFT_BACKGROUND_PREFIX) {
        let final_name = format!("{BACKGROUND_PREFIX}{ext}");
        let draft_path = dir.join(draft_filename);
        let final_path = dir.join(&final_name);
        if draft_path.is_file() {
            fs::rename(&draft_path, &final_path)?;
        }
        clean_background_orphans(themes_root, theme_id, Some(&final_name))?;
        Ok(final_name)
    } else {
        clean_background_orphans(themes_root, theme_id, Some(draft_filename))?;
        Ok(draft_filename.to_string())
    }
}

/// Mantido por compatibilidade: copia rascunho da imagem.
pub fn copy_background_image(
    themes_root: &Path,
    theme_id: Option<&str>,
    source: &Path,
) -> io::Result<String> {
    copy_draft_background_image(themes_root, theme_id, source)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
}

/// Representação em memória dos tokens editáveis pelo Tako.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ThemeTokensOverride {
    pub accent: Option<String>,
    pub font: Option<String>,
    pub canvas: Option<String>,
    pub background_image: Option<String>,
    pub background_mode: Option<BackgroundMode>,
    pub extra: toml::Table,
}

impl ThemeTokensOverride {
    /// Carrega a partir do TOML salvo na sobreposição.
    pub fn from_toml_str(text: &str) -> Self {
        let Ok(mut table) = text.parse::<toml::Table>() else {
            return Self::default();
        };
        let get_str =
            |t: &toml::Table, key: &str| t.get(key).and_then(|v| v.as_str()).map(str::to_owned);

        let accent = get_str(&table, "accent").filter(|c| validate_color(c));
        let font = get_str(&table, "font")
            .or_else(|| get_str(&table, "font-text"))
            .or_else(|| get_str(&table, "font-title"));
        let canvas = get_str(&table, "canvas").filter(|c| validate_canvas(c));
        let background_image = get_str(&table, "custom-background")
            .or_else(|| get_str(&table, "background-image"))
            .filter(|p| is_safe_background_path(p));

        table.remove("accent");
        table.remove("font");
        table.remove("font-title");
        table.remove("font-text");
        table.remove("canvas");
        table.remove("custom-background");
        table.remove("background-image");

        let background_mode = if background_image.is_some() {
            Some(BackgroundMode::Image)
        } else if canvas.is_some() {
            Some(BackgroundMode::Solid)
        } else {
            None
        };

        Self {
            accent,
            font,
            canvas,
            background_image,
            background_mode,
            extra: table,
        }
    }

    /// Converte para texto TOML compatível com `tokens.toml` usando o crate `toml`.
    pub fn to_toml_string(&self) -> String {
        let mut table = self.extra.clone();
        if let Some(ref a) = self.accent {
            table.insert("accent".into(), toml::Value::String(a.clone()));
        } else {
            table.remove("accent");
        }
        if let Some(ref f) = self.font {
            table.insert("font".into(), toml::Value::String(f.clone()));
            table.insert("font-title".into(), toml::Value::String(f.clone()));
            table.insert("font-text".into(), toml::Value::String(f.clone()));
        } else {
            table.remove("font");
            table.remove("font-title");
            table.remove("font-text");
        }
        if let Some(ref c) = self.canvas {
            table.insert("canvas".into(), toml::Value::String(c.clone()));
        } else {
            table.remove("canvas");
        }
        if let Some(ref bg) = self.background_image {
            table.insert("custom-background".into(), toml::Value::String(bg.clone()));
        } else {
            table.remove("custom-background");
            table.remove("background-image");
        }
        toml::to_string_pretty(&table)
            .unwrap_or_else(|_| toml::to_string(&table).unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_colors() {
        assert!(validate_color("#112233"));
        assert!(validate_color("#aabbcc"));
        assert!(validate_color("#AABBCC"));

        // 8 dígitos rejeitado no override
        assert!(!validate_color("#11223344"));
        assert!(!validate_color("#AABBCCDD"));

        assert!(!validate_color("112233"));
        assert!(!validate_color("#123"));
        assert!(!validate_color("#12345"));
        assert!(!validate_color("#1122334455"));
        assert!(!validate_color("#zzxxcc"));
        assert!(!validate_color("red"));
    }

    #[test]
    fn validates_canvas_contrast() {
        // Escuros (alto contraste com branco): aceitos
        assert!(validate_canvas("#0E1116"));
        assert!(validate_canvas("#000000"));
        assert!(validate_canvas("#1A1D24"));

        // Claros (< 4.5:1 com branco): rejeitados
        assert!(!validate_canvas("#FFFFFF"));
        assert!(!validate_canvas("#EEEEEE"));
        assert!(!validate_canvas("#FFFF00"));

        // Inválidos: rejeitados
        assert!(!validate_canvas("invalid"));
        assert!(!validate_canvas("#12345678"));
    }

    #[test]
    fn safe_background_paths() {
        assert!(is_safe_background_path("background.png"));
        assert!(is_safe_background_path("sub/background.png"));
        assert!(is_safe_background_path("wallpaper.webp"));

        assert!(!is_safe_background_path("/etc/passwd"));
        assert!(!is_safe_background_path("C:\\windows\\system32"));
        assert!(!is_safe_background_path("../secret.png"));
        assert!(!is_safe_background_path("sub/../../secret.png"));
        assert!(!is_safe_background_path(""));
    }

    #[test]
    fn draft_promote_and_clean_background_image() {
        let tmp = tempfile::tempdir().unwrap();
        let themes_root = tmp.path().join("themes");
        let src_dir = tmp.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();

        // Cria uma imagem PNG válida pequena
        let img = image::RgbaImage::new(10, 10);
        let img_file = src_dir.join("fundo.png");
        img.save(&img_file).unwrap();

        // 1. Cópia vai para rascunho (background.draft.png)
        let draft_name = copy_draft_background_image(&themes_root, None, &img_file).unwrap();
        assert_eq!(draft_name, "background.draft.png");
        assert!(
            override_dir(&themes_root, None)
                .join("background.draft.png")
                .is_file()
        );
        assert!(
            !override_dir(&themes_root, None)
                .join("background.png")
                .exists()
        );

        // 2. Limpeza de rascunho (ex.: cancelar)
        clean_background_draft(&themes_root, None).unwrap();
        assert!(
            !override_dir(&themes_root, None)
                .join("background.draft.png")
                .exists()
        );

        // 3. Copia de novo e promove para definitivo
        let draft_name = copy_draft_background_image(&themes_root, None, &img_file).unwrap();
        let final_name = promote_background_draft(&themes_root, None, &draft_name).unwrap();
        assert_eq!(final_name, "background.png");
        assert!(
            override_dir(&themes_root, None)
                .join("background.png")
                .is_file()
        );
        assert!(
            !override_dir(&themes_root, None)
                .join("background.draft.png")
                .exists()
        );

        // 4. Copia um novo arquivo jpg e promove: o png anterior órfão deve ser removido
        let jpg_img = image::RgbImage::new(20, 20);
        let jpg_file = src_dir.join("fundo.jpg");
        jpg_img.save(&jpg_file).unwrap();

        let draft_jpg = copy_draft_background_image(&themes_root, None, &jpg_file).unwrap();
        assert_eq!(draft_jpg, "background.draft.jpg");
        // O background.png anterior AINDA existe enquanto não salvar!
        assert!(
            override_dir(&themes_root, None)
                .join("background.png")
                .is_file()
        );

        // Promover o jpg: background.png órfão é removido
        let final_jpg = promote_background_draft(&themes_root, None, &draft_jpg).unwrap();
        assert_eq!(final_jpg, "background.jpg");
        assert!(
            override_dir(&themes_root, None)
                .join("background.jpg")
                .is_file()
        );
        assert!(
            !override_dir(&themes_root, None)
                .join("background.png")
                .exists()
        );
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
        assert!(loaded.contains("#E3161E"));
        assert!(loaded.contains("Poppins"));

        let custom_id = "autor.meu-tema";
        save_override(themes_root, Some(custom_id), toml_sample).unwrap();
        assert!(has_override(themes_root, Some(custom_id)));

        assert!(reset_override(themes_root, None).unwrap());
        assert!(!has_override(themes_root, None));
        assert_eq!(load_override(themes_root, None), None);

        assert!(!reset_override(themes_root, None).unwrap());
        assert!(has_override(themes_root, Some(custom_id)));
    }

    #[test]
    fn theme_tokens_override_serialization_and_extra_preservation() {
        let mut extra = toml::Table::new();
        extra.insert("custom-token".into(), toml::Value::Integer(42));
        extra.insert("outro".into(), toml::Value::String("valor".into()));

        let tokens = ThemeTokensOverride {
            accent: Some("#6C5CE7".into()),
            font: Some("Plus Jakarta Sans".into()),
            canvas: Some("#0B1A33".into()),
            background_image: Some("background.jpg".into()),
            background_mode: Some(BackgroundMode::Image),
            extra,
        };
        let toml_str = tokens.to_toml_string();
        let parsed = ThemeTokensOverride::from_toml_str(&toml_str);
        assert_eq!(parsed.accent, tokens.accent);
        assert_eq!(parsed.font, tokens.font);
        assert_eq!(parsed.canvas, tokens.canvas);
        assert_eq!(parsed.background_image, tokens.background_image);
        assert_eq!(
            parsed
                .extra
                .get("custom-token")
                .and_then(|v| v.as_integer()),
            Some(42)
        );
        assert_eq!(
            parsed.extra.get("outro").and_then(|v| v.as_str()),
            Some("valor")
        );
    }
}
