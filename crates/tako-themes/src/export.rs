//! Exportação de temas em formato de pacote `.takotheme` (THEME-006).

use std::fs;
use std::io;
use std::path::Path;

use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::Problem;
use crate::package::validate_package;

#[derive(Debug, thiserror::Error)]
pub enum ExportThemeError {
    #[error("pasta do tema não encontrada: {0}")]
    NotFound(String),
    #[error("falha de E/S: {0}")]
    Io(#[from] io::Error),
    #[error("erro ao compactar tema: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("o pacote do tema é inválido: {0:?}")]
    InvalidPackage(Vec<Problem>),
}

/// Compacta os arquivos de um tema instalado em um arquivo `.takotheme` usando escrita atômica.
pub fn export_theme(theme_dir: &Path, output_path: &Path) -> Result<(), ExportThemeError> {
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

    {
        let mut zip = ZipWriter::new(temp_file.as_file());
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        write_dir_to_zip(&mut zip, theme_dir, "", options)?;
        zip.finish()?;
    }

    // Valida que o pacote temporário gerado passa no validate_package existente antes de persistir
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
