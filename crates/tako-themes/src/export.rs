//! Exportação de temas em formato de pacote `.takotheme` (THEME-006).

use std::fs;
use std::io;
use std::path::Path;

use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::Problem;
use crate::overrides::{BackgroundMode, ThemeTokensOverride};
use crate::package::validate_package;

#[derive(Debug, thiserror::Error)]
pub enum ExportThemeError {
    #[error("pasta do tema não encontrada: {0}")]
    NotFound(String),
    #[error("falha de E/S: {0}")]
    Io(#[from] io::Error),
    #[error("erro ao compactar tema: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("o pacote do tema é inválido")]
    InvalidPackage(Vec<Problem>),
}

/// Compacta os arquivos de um tema instalado em um arquivo `.takotheme` usando escrita atômica,
/// opcionalmente aplicando uma sobreposição de tokens e imagem de fundo (THEME-006).
pub fn export_theme_with_override(
    theme_dir: &Path,
    output_path: &Path,
    override_tokens: Option<&ThemeTokensOverride>,
    override_dir: Option<&Path>,
) -> Result<(), ExportThemeError> {
    if !theme_dir.is_dir() {
        return Err(ExportThemeError::NotFound(theme_dir.display().to_string()));
    }

    // Confere que o tema de origem é válido
    let _manifest = validate_package(theme_dir).map_err(ExportThemeError::InvalidPackage)?;

    let parent_dir = output_path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent_dir)?;

    let temp_file = tempfile::Builder::new()
        .prefix(".export-")
        .suffix(".takotheme")
        .tempfile_in(parent_dir)?;

    if let Some(ov) = override_tokens {
        let staging = tempfile::Builder::new()
            .prefix(".export-staging-")
            .tempdir_in(parent_dir)?;

        for entry in fs::read_dir(theme_dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') {
                continue;
            }
            if path.is_file() {
                fs::copy(&path, staging.path().join(&name))?;
            } else if path.is_dir() {
                copy_dir_recursive(&path, &staging.path().join(&name))?;
            }
        }

        let base_tokens_path = staging.path().join("tokens.toml");
        let mut tokens = if base_tokens_path.is_file() {
            let text = fs::read_to_string(&base_tokens_path).unwrap_or_default();
            ThemeTokensOverride::from_toml_str(&text)
        } else {
            ThemeTokensOverride::default()
        };

        if let Some(ref a) = ov.accent {
            tokens.accent = Some(a.clone());
        }
        if let Some(ref f) = ov.font {
            tokens.font = Some(f.clone());
        }
        if let Some(ref c) = ov.canvas {
            tokens.canvas = Some(c.clone());
        }
        if let Some(ref bm) = ov.background_mode {
            tokens.background_mode = Some(*bm);
        }
        for (k, v) in &ov.extra {
            tokens.extra.insert(k.clone(), v.clone());
        }

        if let Some(ref bg_file) = ov.background_image {
            let mut found_src = None;
            if let Some(ref d) = ov.draft_dir {
                let candidate = d.join(bg_file);
                if candidate.is_file() {
                    found_src = Some(candidate);
                }
            }
            if let (None, Some(od)) = (&found_src, override_dir) {
                let candidate = od.join(bg_file);
                if candidate.is_file() {
                    found_src = Some(candidate);
                }
            }
            if found_src.is_none() {
                let candidate = theme_dir.join(bg_file);
                if candidate.is_file() {
                    found_src = Some(candidate);
                }
            }

            if let Some(src_img) = found_src {
                let ext = crate::overrides::validate_background_image(&src_img).map_err(|_| {
                    ExportThemeError::Io(io::Error::other("invalid background image"))
                })?;
                let dest_name = format!("background.{ext}");
                fs::copy(&src_img, staging.path().join(&dest_name))?;
                tokens.background_image = Some(dest_name);
                tokens.background_mode = Some(BackgroundMode::Image);
                tokens.canvas = None;
            }
        } else if ov.background_mode == Some(BackgroundMode::Solid) || ov.canvas.is_some() {
            tokens.background_image = None;
            for ext in ["png", "jpg", "jpeg", "webp"] {
                let old_bg = staging.path().join(format!("background.{ext}"));
                if old_bg.exists() {
                    let _ = fs::remove_file(old_bg);
                }
            }
        }

        tokens.background_source_name = None;
        tokens.draft_dir = None;

        fs::write(staging.path().join("tokens.toml"), tokens.to_toml_string())?;

        validate_package(staging.path()).map_err(ExportThemeError::InvalidPackage)?;

        {
            let mut zip = ZipWriter::new(temp_file.as_file());
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            write_dir_to_zip(&mut zip, staging.path(), "", options)?;
            zip.finish()?;
        }
    } else {
        {
            let mut zip = ZipWriter::new(temp_file.as_file());
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

            write_dir_to_zip(&mut zip, theme_dir, "", options)?;
            zip.finish()?;
        }
    }

    validate_package(temp_file.path()).map_err(ExportThemeError::InvalidPackage)?;

    temp_file
        .persist(output_path)
        .map_err(|e| ExportThemeError::Io(e.error))?;

    tracing::info!(
        from = %theme_dir.display(),
        to = %output_path.display(),
        "tema exportado com sucesso"
    );

    Ok(())
}

/// Compacta os arquivos de um tema instalado em um arquivo `.takotheme` usando escrita atômica.
pub fn export_theme(theme_dir: &Path, output_path: &Path) -> Result<(), ExportThemeError> {
    export_theme_with_override(theme_dir, output_path, None, None)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        if path.is_file() {
            fs::copy(&path, dst.join(&name))?;
        } else if path.is_dir() {
            copy_dir_recursive(&path, &dst.join(&name))?;
        }
    }
    Ok(())
}

fn write_dir_to_zip<W: io::Write + io::Seek>(
    zip: &mut ZipWriter<W>,
    dir: &Path,
    prefix: &str,
    options: SimpleFileOptions,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();

        if name_str.starts_with('.') {
            continue;
        }

        let rel_path = if prefix.is_empty() {
            name_str.into_owned()
        } else {
            format!("{prefix}/{name_str}")
        };

        if path.is_file() {
            zip.start_file(&rel_path, options)
                .map_err(io::Error::other)?;
            let mut f = fs::File::open(&path)?;
            io::copy(&mut f, zip)?;
        } else if path.is_dir() {
            write_dir_to_zip(zip, &path, &rel_path, options)?;
        }
    }
    Ok(())
}
