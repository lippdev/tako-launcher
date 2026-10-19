//! Backup e restauração local da biblioteca e das configurações (CLOUD-001).
//!
//! Gera e restaura um arquivo único `.takobackup` (formato ZIP com manifesto `backup.toml`).
//!
//! - **O que entra no backup:**
//!   - Banco SQLite (`tako.db`) copiado de forma consistente via `VACUUM INTO` (biblioteca,
//!     jogos manuais, estado, favoritos, tempo jogado, status, sessões, coleções, tags,
//!     configurações por jogo e configurações gerais do usuário).
//!   - Configurações da máquina (`updates.json`, se existir).
//!   - Temas instalados pelo usuário (`themes/`, incluindo `.selected`).
//!   - Manifesto versionado (`backup.toml`).
//!
//! - **O que fica de fora:**
//!   - Cache de capas e de metadados (`cache/art`, `cache/metadata`, `cache/steam-owned.json`).
//!   - Registros de logs (`logs/`).
//!   - Segredos e tokens (chaves de API da Steam Web API, IGDB, ScreenScraper, SteamGridDB
//!     guardados no cofre do sistema operacional; arquivos legados `.dat` e `.dat.migrado`).
//!   - Arquivos temporários de sessão e processo (`session.json`, `session-environment.json`,
//!     `cursor-hidden`, `tako.lock`, `activate.json`, backups automáticos anteriores).
//!
//! - **Garantias de segurança na restauração:**
//!   - Validação do manifesto e rejeição de versões futuras com mensagem informativa.
//!   - Backup automático do estado atual antes de substituir qualquer arquivo.
//!   - Proteção contra Zip-Slip (rejeita `..`, caminhos absolutos, letras de unidade, nomes de dispositivo).
//!   - Limite de tamanho total e por arquivo descompactado.
//!   - Verificação de integridade do banco SQLite (`PRAGMA quick_check;`) antes de aplicar.

use super::{PersistenceError, TakoDatabase, format_date, migrator};
use crate::paths::DataPaths;
use chrono::Utc;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

pub const BACKUP_EXTENSION: &str = "takobackup";
pub const MANIFEST_FILE: &str = "backup.toml";
pub const CURRENT_BACKUP_VERSION: u32 = 1;

/// Limites razoáveis de proteção contra bombas de descompressão e arquivos maliciosos.
pub const MAX_BACKUP_BYTES: u64 = 500 * 1024 * 1024; // 500 MB
pub const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024; // 256 MB por arquivo
pub const MAX_UNCOMPRESSED_BYTES: u64 = 1024 * 1024 * 1024; // 1 GB no total
pub const MAX_ENTRIES: usize = 10_000;
const MAX_PATH_LEN: usize = 1024;

const DEVICE_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("arquivo não encontrado: {0}")]
    NotFound(String),
    #[error("arquivo excede o limite de tamanho ({size} bytes, máximo {max} bytes)")]
    TooLarge { size: u64, max: u64 },
    #[error("arquivo zip inválido ou corrompido: {0}")]
    CorruptZip(String),
    #[error("caminho inseguro no arquivo ({entry}): {reason}")]
    UnsafePath { entry: String, reason: String },
    #[error("manifesto ausente ou inválido: {0}")]
    InvalidManifest(String),
    #[error(
        "o arquivo de backup usa a versão {version}, mas esta versão do Tako só suporta até a versão {max_supported}"
    )]
    FutureVersion { version: u32, max_supported: u32 },
    #[error(
        "o banco de dados do backup usa a versão do esquema {version}, mais recente do que a suportada ({max_supported})"
    )]
    FutureDatabaseVersion { version: u32, max_supported: u32 },
    #[error("banco de dados corrompido no backup: {0}")]
    CorruptDatabase(String),
    #[error("erro de persistência: {0}")]
    Persistence(#[from] PersistenceError),
    #[error("falha de E/S: {0}")]
    Io(#[from] io::Error),
    #[error("falha no banco de dados: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("falha no arquivo zip: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("falha ao interpretar toml: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("falha ao gerar toml: {0}")]
    TomlSer(#[from] toml::ser::Error),
}

/// Manifesto descritivo incluído em todo pacote `.takobackup`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupManifest {
    pub version: u32,
    pub tako_version: String,
    pub database_version: u32,
    pub created_at: String,
    pub os: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Resultado da operação de restauração.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreResult {
    pub manifest: BackupManifest,
    /// Caminho do backup automático gerado antes de substituir o estado.
    pub auto_backup_path: Option<PathBuf>,
}

/// Gera um arquivo `.takobackup` no destino especificado.
///
/// O banco SQLite é copiado consistentemente usando `VACUUM INTO`, garantindo
/// que transações ativas e arquivos WAL não gerem dados corrompidos.
pub fn create_backup(paths: &DataPaths, dest_file: &Path) -> Result<BackupManifest, BackupError> {
    if let Some(parent) = dest_file.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }

    // Garante que o banco exista e esteja pronto
    let db_path = paths.database();
    let conn = if db_path.exists() {
        Connection::open(&db_path)?
    } else {
        TakoDatabase::new(&db_path).open()?
    };

    // Verificação rápida de integridade do banco antes do backup
    let check: String = conn.query_row("PRAGMA quick_check;", [], |r| r.get(0))?;
    if check != "ok" {
        return Err(BackupError::CorruptDatabase(format!(
            "quick_check: {check}"
        )));
    }

    let database_version = migrator::current_version(&conn)?;

    // Snapshot atômico e consistente do SQLite com `VACUUM INTO`
    let temp_dir = tempfile::tempdir()?;
    let temp_db_path = temp_dir.path().join("tako.db");
    conn.execute("VACUUM INTO ?1;", [temp_db_path.to_string_lossy()])?;

    let manifest = BackupManifest {
        version: CURRENT_BACKUP_VERSION,
        tako_version: env!("CARGO_PKG_VERSION").into(),
        database_version,
        created_at: format_date(Utc::now()),
        os: format!("{:?}", crate::paths::Os::current()),
        description: Some("Backup da biblioteca e configurações do Tako".into()),
    };
    let manifest_toml = toml::to_string(&manifest)?;

    // Escreve primeiro em arquivo temporário no mesmo diretório para atomicidade
    let temp_dest = tempfile::Builder::new()
        .prefix(".backup-")
        .suffix(".tmp")
        .tempfile_in(dest_file.parent().unwrap_or_else(|| Path::new(".")))?;

    {
        let mut zip = ZipWriter::new(temp_dest.as_file());
        let file_options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        // 1. Manifesto
        zip.start_file(MANIFEST_FILE, file_options)?;
        zip.write_all(manifest_toml.as_bytes())?;

        // 2. Banco SQLite íntegro
        zip.start_file("tako.db", file_options)?;
        let mut db_file = File::open(&temp_db_path)?;
        io::copy(&mut db_file, &mut zip)?;

        // 3. Temas instalados pelo usuário
        let themes_root = paths.themes();
        if themes_root.is_dir() {
            add_themes_to_zip(&themes_root, &mut zip, file_options)?;
        }

        // 4. Configurações da máquina (updates.json se existir)
        let updates_file = paths.updates();
        if updates_file.is_file()
            && let Ok(mut f) = File::open(&updates_file)
        {
            zip.start_file("updates.json", file_options)?;
            io::copy(&mut f, &mut zip)?;
        }

        zip.finish()?;
    }

    temp_dest
        .persist(dest_file)
        .map_err(|e| io::Error::other(e.error))?;

    tracing::info!(
        dest = %dest_file.display(),
        version = manifest.version,
        "backup criado com sucesso"
    );

    Ok(manifest)
}

/// Lê e valida o manifesto de um arquivo `.takobackup` sem alterar o disco.
pub fn inspect_backup(backup_file: &Path) -> Result<BackupManifest, BackupError> {
    if !backup_file.is_file() {
        return Err(BackupError::NotFound(backup_file.display().to_string()));
    }
    let metadata = fs::metadata(backup_file)?;
    if metadata.len() > MAX_BACKUP_BYTES {
        return Err(BackupError::TooLarge {
            size: metadata.len(),
            max: MAX_BACKUP_BYTES,
        });
    }

    let file = File::open(backup_file)?;
    let mut zip = ZipArchive::new(file).map_err(|e| BackupError::CorruptZip(e.to_string()))?;

    let manifest_bytes = read_entry_bytes(&mut zip, MANIFEST_FILE)?
        .ok_or_else(|| BackupError::InvalidManifest("backup.toml ausente".into()))?;

    let manifest: BackupManifest = toml::from_slice(&manifest_bytes)?;
    Ok(manifest)
}

/// Restaura o estado a partir de um `.takobackup`.
///
/// Antes de aplicar as mudanças, cria um backup automático do estado atual em
/// `paths.root/backups/auto-backup-{timestamp}.takobackup`.
pub fn restore_backup(paths: &DataPaths, backup_file: &Path) -> Result<RestoreResult, BackupError> {
    if !backup_file.is_file() {
        return Err(BackupError::NotFound(backup_file.display().to_string()));
    }
    let metadata = fs::metadata(backup_file)?;
    if metadata.len() > MAX_BACKUP_BYTES {
        return Err(BackupError::TooLarge {
            size: metadata.len(),
            max: MAX_BACKUP_BYTES,
        });
    }

    let file = File::open(backup_file)?;
    let mut zip = ZipArchive::new(file).map_err(|e| BackupError::CorruptZip(e.to_string()))?;

    if zip.len() > MAX_ENTRIES {
        return Err(BackupError::TooLarge {
            size: zip.len() as u64,
            max: MAX_ENTRIES as u64,
        });
    }

    // 1. Verificação prévia de todas as entradas (Zip-Slip, limites e symlinks)
    let mut total_uncompressed: u64 = 0;
    let mut has_manifest = false;
    let mut has_db = false;

    for i in 0..zip.len() {
        let entry = zip
            .by_index_raw(i)
            .map_err(|e| BackupError::CorruptZip(e.to_string()))?;
        let raw_name = entry
            .name()
            .map_err(|e| BackupError::CorruptZip(e.to_string()))?
            .into_owned();

        if entry.is_symlink() {
            return Err(BackupError::UnsafePath {
                entry: raw_name,
                reason: "links simbólicos não são permitidos".into(),
            });
        }

        let size = entry.size();
        if size > MAX_FILE_BYTES {
            return Err(BackupError::TooLarge {
                size,
                max: MAX_FILE_BYTES,
            });
        }
        total_uncompressed = total_uncompressed.saturating_add(size);
        if total_uncompressed > MAX_UNCOMPRESSED_BYTES {
            return Err(BackupError::TooLarge {
                size: total_uncompressed,
                max: MAX_UNCOMPRESSED_BYTES,
            });
        }

        let normalized = normalize_entry(&raw_name).map_err(|reason| BackupError::UnsafePath {
            entry: raw_name.clone(),
            reason: reason.into(),
        })?;

        if let Some(norm) = normalized {
            if norm == MANIFEST_FILE {
                has_manifest = true;
            } else if norm == "tako.db" {
                has_db = true;
            }
        }
    }

    if !has_manifest {
        return Err(BackupError::InvalidManifest("backup.toml ausente".into()));
    }
    if !has_db {
        return Err(BackupError::InvalidManifest(
            "banco de dados tako.db ausente no backup".into(),
        ));
    }

    // 2. Validação do manifesto e versão
    let manifest_bytes = read_entry_bytes(&mut zip, MANIFEST_FILE)?
        .ok_or_else(|| BackupError::InvalidManifest("backup.toml ausente".into()))?;
    let manifest: BackupManifest = toml::from_slice(&manifest_bytes)?;

    if manifest.version > CURRENT_BACKUP_VERSION {
        return Err(BackupError::FutureVersion {
            version: manifest.version,
            max_supported: CURRENT_BACKUP_VERSION,
        });
    }

    let latest_supported_db = migrator::latest_version();
    if manifest.database_version > latest_supported_db {
        return Err(BackupError::FutureDatabaseVersion {
            version: manifest.database_version,
            max_supported: latest_supported_db,
        });
    }

    // 3. Guarda backup automático do estado atual antes de substituir qualquer arquivo
    let auto_backup_path = if paths.database().exists() {
        let backups_dir = paths.root.join("backups");
        fs::create_dir_all(&backups_dir)?;
        let path = backups_dir.join(format!(
            "auto-backup-{}.takobackup",
            Utc::now().format("%Y%m%d-%H%M%S")
        ));
        create_backup(paths, &path)?;
        tracing::info!(auto = %path.display(), "backup automático salvo antes da restauração");
        Some(path)
    } else {
        None
    };

    // 4. Extração segura para pasta temporária de preparação (staging)
    let staging_dir = if paths.root.exists() {
        tempfile::Builder::new()
            .prefix(".restore-")
            .tempdir_in(&paths.root)?
    } else {
        tempfile::tempdir()?
    };

    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| BackupError::CorruptZip(e.to_string()))?;
        let raw_name = entry
            .name()
            .map_err(|e| BackupError::CorruptZip(e.to_string()))?
            .to_string();
        let Some(normalized) =
            normalize_entry(&raw_name).map_err(|reason| BackupError::UnsafePath {
                entry: raw_name,
                reason: reason.into(),
            })?
        else {
            continue; // Entrada de pasta pura
        };

        let target_path = staging_dir.path().join(&normalized);
        // Garantia de Zip-Slip: o caminho deve permanecer dentro de staging
        if !target_path.starts_with(staging_dir.path()) {
            return Err(BackupError::UnsafePath {
                entry: normalized,
                reason: "caminho escapa do diretório de destino".into(),
            });
        }

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut outfile = File::create(&target_path)?;
        io::copy(&mut entry, &mut outfile)?;
    }

    // 5. Verificação da integridade do SQLite descompactado
    let extracted_db = staging_dir.path().join("tako.db");
    {
        let conn = Connection::open(&extracted_db)?;
        let check: String = conn.query_row("PRAGMA quick_check;", [], |r| r.get(0))?;
        if check != "ok" {
            return Err(BackupError::CorruptDatabase(format!(
                "quick_check: {check}"
            )));
        }
        // Aplica migrações pendentes se o backup for de versão anterior
        migrator::migrate(&conn, None)?;
    }

    // 6. Substituição definitiva dos arquivos no destino
    fs::create_dir_all(&paths.root)?;

    // Remove arquivos de log do WAL anteriores para evitar corrupção cruzada
    let db_path = paths.database();
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut aux = db_path.as_os_str().to_owned();
        aux.push(suffix);
        let _ = fs::remove_file(aux);
    }

    // Substitui o banco de dados principal
    replace_file_or_copy(&extracted_db, &db_path)?;

    // Substitui os temas instalados
    let extracted_themes = staging_dir.path().join("themes");
    if extracted_themes.is_dir() {
        let dest_themes = paths.themes();
        fs::create_dir_all(&dest_themes)?;
        copy_dir_recursive(&extracted_themes, &dest_themes)?;
    }

    // Substitui updates.json se veio no backup
    let extracted_updates = staging_dir.path().join("updates.json");
    if extracted_updates.is_file() {
        replace_file_or_copy(&extracted_updates, &paths.updates())?;
    }

    tracing::info!(
        backup = %backup_file.display(),
        version = manifest.version,
        "restauração concluída com sucesso"
    );

    Ok(RestoreResult {
        manifest,
        auto_backup_path,
    })
}

fn add_themes_to_zip(
    themes_root: &Path,
    zip: &mut ZipWriter<&File>,
    options: SimpleFileOptions,
) -> Result<(), BackupError> {
    let mut stack = vec![(themes_root.to_path_buf(), String::from("themes"))];
    while let Some((dir, prefix)) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(std::result::Result::ok) {
            let file_type = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            if file_type.is_symlink() {
                continue;
            }

            let file_name = entry.file_name().to_string_lossy().into_owned();
            // Ignora pastas/arquivos ocultos e temporários, exceto o arquivo de seleção (.selected)
            if file_name.starts_with('.') && file_name != ".selected" {
                continue;
            }
            // Defesa em profundidade: não inclui segredos nem tokens
            let lower = file_name.to_ascii_lowercase();
            if lower.ends_with(".dat")
                || lower.ends_with(".log")
                || lower.contains("secret")
                || lower.contains("token")
            {
                continue;
            }

            let path = entry.path();
            let entry_name = format!("{prefix}/{file_name}");

            if file_type.is_dir() {
                stack.push((path, entry_name));
            } else if file_type.is_file()
                && let Ok(mut f) = File::open(&path)
            {
                zip.start_file(entry_name, options)?;
                io::copy(&mut f, zip)?;
            }
        }
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        if ft.is_symlink() {
            continue;
        }
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ft.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if ft.is_file() {
            replace_file_or_copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn replace_file_or_copy(src: &Path, dst: &Path) -> io::Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(src, dst)?;
            let _ = fs::remove_file(src);
            Ok(())
        }
    }
}

fn read_entry_bytes(
    zip: &mut ZipArchive<File>,
    name: &str,
) -> Result<Option<Vec<u8>>, BackupError> {
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| BackupError::CorruptZip(e.to_string()))?;
        let entry_name = entry
            .name()
            .map_err(|e| BackupError::CorruptZip(e.to_string()))?;
        if entry_name == name {
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            return Ok(Some(bytes));
        }
    }
    Ok(None)
}

/// Normalização e validação de caminhos para proteção contra Zip-Slip.
///
/// Rejeita caminhos absolutos, segmentos `..`, separadores de unidade e caracteres ilegais.
pub fn normalize_entry(name: &str) -> Result<Option<String>, &'static str> {
    if name.is_empty() {
        return Err("caminho vazio");
    }
    let path = name.replace('\\', "/");
    if path.starts_with('/') {
        return Err("caminho absoluto não permitido");
    }
    if path.chars().count() > MAX_PATH_LEN {
        return Err("caminho longo demais");
    }

    let mut segments = Vec::new();
    for s in path.split('/') {
        if s.is_empty() || s == "." {
            continue;
        }
        if s == ".." {
            return Err("navegação de diretório pai (..) não permitida");
        }
        if s.chars().any(|c| c.is_control() || "<>:\"|?*".contains(c)) {
            return Err(if s.contains(':') {
                "dois-pontos de unidade não permitidos"
            } else {
                "caracteres especiais proibidos no caminho"
            });
        }
        if s.ends_with('.') || s.ends_with(' ') {
            return Err("ponto ou espaço final não permitido");
        }
        if is_device_name(s.split('.').next().unwrap_or(s)) {
            return Err("nome de dispositivo do sistema proibido");
        }
        segments.push(s);
    }

    if segments.is_empty() || path.ends_with('/') {
        return Ok(None);
    }
    Ok(Some(segments.join("/")))
}

fn is_device_name(stem: &str) -> bool {
    let stem = stem.trim_end();
    DEVICE_NAMES.iter().any(|d| stem.eq_ignore_ascii_case(d))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::PlayStatus;
    use crate::persistence::{GameStateRow, LibraryStore, ManualGameRow, SettingsStore};

    fn make_test_paths(dir: &Path) -> DataPaths {
        DataPaths::at(dir)
    }

    #[test]
    fn backup_restore_roundtrip_preserves_all_data() {
        let source_dir = tempfile::tempdir().unwrap();
        let target_dir = tempfile::tempdir().unwrap();
        let paths_src = make_test_paths(source_dir.path());
        let paths_dst = make_test_paths(target_dir.path());

        // 1. Preenche dados de teste na origem
        let db = TakoDatabase::new(paths_src.database());
        let store = LibraryStore::new(db.clone(), None);
        let mut data = store.load().unwrap();
        data.manual.push(ManualGameRow {
            id: "manual:jogo1".into(),
            title: "Super Jogo".into(),
            path: "/games/jogo1.exe".into(),
            arguments: None,
            working_directory: None,
            platform: None,
            cover_path: None,
            hero_path: None,
            added_at: Utc::now(),
            track_by: None,
        });
        data.state.insert(
            "manual:jogo1".into(),
            GameStateRow {
                is_favorite: true,
                play_minutes: 120.0,
                last_played_at: Some(Utc::now()),
                play_status: Some(PlayStatus::Playing),
                sessions: Vec::new(),
                tags: std::collections::BTreeSet::new(),
            },
        );
        store.save(&data).unwrap();

        let settings = SettingsStore::new(db);
        settings.set("Language", "pt-BR").unwrap();
        settings.set("ReducedMotion", "true").unwrap();

        // Cria tema e seleção
        let theme_dir = paths_src.themes().join("meu-tema");
        fs::create_dir_all(&theme_dir).unwrap();
        fs::write(theme_dir.join("theme.toml"), "name = 'Meu Tema'\n").unwrap();
        fs::write(paths_src.themes().join(".selected"), "meu-tema").unwrap();

        // Configuração updates.json
        fs::create_dir_all(&paths_src.machine).unwrap();
        fs::write(paths_src.updates(), "{\"AutoUpdate\":true}").unwrap();

        // 2. Cria backup
        let backup_file = source_dir.path().join("backup.takobackup");
        let manifest = create_backup(&paths_src, &backup_file).unwrap();
        assert_eq!(manifest.version, CURRENT_BACKUP_VERSION);
        assert!(backup_file.exists());

        // 3. Destino tem dados diferentes antes da restauração
        let dst_db = TakoDatabase::new(paths_dst.database());
        let dst_store = LibraryStore::new(dst_db.clone(), None);
        let mut dst_data = dst_store.load().unwrap();
        dst_data.manual.push(ManualGameRow {
            id: "manual:antigo".into(),
            title: "Jogo Antigo".into(),
            path: "/old.exe".into(),
            arguments: None,
            working_directory: None,
            platform: None,
            cover_path: None,
            hero_path: None,
            added_at: Utc::now(),
            track_by: None,
        });
        dst_store.save(&dst_data).unwrap();

        // 4. Restaura no destino
        let result = restore_backup(&paths_dst, &backup_file).unwrap();
        assert_eq!(result.manifest.version, CURRENT_BACKUP_VERSION);
        assert!(result.auto_backup_path.is_some());
        assert!(result.auto_backup_path.unwrap().exists());

        // 5. Verifica se os dados restaurados conferem exatamente com a origem
        let restored_store = LibraryStore::new(TakoDatabase::new(paths_dst.database()), None);
        let restored_data = restored_store.load().unwrap();
        assert_eq!(restored_data.manual.len(), 1);
        assert_eq!(restored_data.manual[0].id, "manual:jogo1");
        assert_eq!(restored_data.manual[0].title, "Super Jogo");
        assert!(restored_data.state["manual:jogo1"].is_favorite);
        assert_eq!(
            restored_data.state["manual:jogo1"].play_status,
            Some(PlayStatus::Playing)
        );

        let restored_settings = SettingsStore::new(TakoDatabase::new(paths_dst.database()));
        assert_eq!(
            restored_settings.get("Language").unwrap().as_deref(),
            Some("pt-BR")
        );
        assert_eq!(
            restored_settings.get("ReducedMotion").unwrap().as_deref(),
            Some("true")
        );

        // Verifica tema e updates
        assert_eq!(
            fs::read_to_string(paths_dst.themes().join(".selected")).unwrap(),
            "meu-tema"
        );
        assert!(
            paths_dst
                .themes()
                .join("meu-tema")
                .join("theme.toml")
                .exists()
        );
        assert_eq!(
            fs::read_to_string(paths_dst.updates()).unwrap(),
            "{\"AutoUpdate\":true}"
        );
    }

    #[test]
    fn zip_slip_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let paths = make_test_paths(dir.path());
        let malicious_zip = dir.path().join("evil.takobackup");

        // Cria zip malicioso com caminho relativo escapando com `..`
        {
            let file = File::create(&malicious_zip).unwrap();
            let mut zip = ZipWriter::new(file);
            let opts = SimpleFileOptions::default();

            let manifest = BackupManifest {
                version: CURRENT_BACKUP_VERSION,
                tako_version: "0.2.0".into(),
                database_version: migrator::latest_version(),
                created_at: format_date(Utc::now()),
                os: "Linux".into(),
                description: None,
            };
            zip.start_file(MANIFEST_FILE, opts).unwrap();
            zip.write_all(toml::to_string(&manifest).unwrap().as_bytes())
                .unwrap();

            zip.start_file("tako.db", opts).unwrap();
            zip.write_all(b"fake sqlite").unwrap();

            // Entrada perigosa
            zip.start_file("../evil.txt", opts).unwrap();
            zip.write_all(b"perigo!").unwrap();

            zip.finish().unwrap();
        }

        let err = restore_backup(&paths, &malicious_zip).unwrap_err();
        match err {
            BackupError::UnsafePath { entry, .. } => {
                assert_eq!(entry, "../evil.txt");
            }
            other => panic!("esperava UnsafePath, veio {other:?}"),
        }

        // Certifica que nada foi criado fora
        assert!(!dir.path().parent().unwrap().join("evil.txt").exists());
    }

    #[test]
    fn future_version_is_rejected_with_message() {
        let dir = tempfile::tempdir().unwrap();
        let paths = make_test_paths(dir.path());
        let future_zip = dir.path().join("future.takobackup");

        {
            let file = File::create(&future_zip).unwrap();
            let mut zip = ZipWriter::new(file);
            let opts = SimpleFileOptions::default();

            let manifest = BackupManifest {
                version: 999, // Versão futura incompatível
                tako_version: "99.0.0".into(),
                database_version: migrator::latest_version(),
                created_at: format_date(Utc::now()),
                os: "Linux".into(),
                description: None,
            };
            zip.start_file(MANIFEST_FILE, opts).unwrap();
            zip.write_all(toml::to_string(&manifest).unwrap().as_bytes())
                .unwrap();

            zip.start_file("tako.db", opts).unwrap();
            zip.write_all(b"fake").unwrap();

            zip.finish().unwrap();
        }

        let err = restore_backup(&paths, &future_zip).unwrap_err();
        assert!(matches!(
            err,
            BackupError::FutureVersion { version: 999, .. }
        ));
        let msg = err.to_string();
        assert!(msg.contains("versão 999"));
    }

    #[test]
    fn secrets_and_caches_are_excluded_from_backup() {
        let dir = tempfile::tempdir().unwrap();
        let paths = make_test_paths(dir.path());

        // Inicializa banco
        let db = TakoDatabase::new(paths.database());
        db.open().unwrap();

        // Cria arquivos que DEVEM FICAR DE FORA
        fs::create_dir_all(paths.logs()).unwrap();
        fs::write(paths.logs().join("app.log"), "log secreto").unwrap();

        fs::create_dir_all(paths.art_cache()).unwrap();
        fs::write(paths.art_cache().join("capa.jpg"), "imagem cache").unwrap();

        fs::create_dir_all(paths.metadata_cache()).unwrap();
        fs::write(paths.metadata_cache().join("meta.json"), "metadata").unwrap();

        // Segredos e arquivos legados
        fs::write(paths.root.join("igdb.dat"), "segredo igdb").unwrap();
        fs::write(paths.root.join("screenscraper.dat"), "senha ss").unwrap();
        fs::write(paths.root.join("token.json"), "api token").unwrap();

        // Arquivos temporários de sessão
        fs::write(paths.session_journal(), "sessao ativa").unwrap();
        fs::write(paths.cursor_marker(), "marcador").unwrap();

        // Gera backup
        let backup_file = dir.path().join("teste.takobackup");
        create_backup(&paths, &backup_file).unwrap();

        // Inspeciona o zip gerado
        let mut zip = ZipArchive::new(File::open(&backup_file).unwrap()).unwrap();
        let entry_names: Vec<String> = (0..zip.len())
            .map(|i| zip.by_index(i).unwrap().name().unwrap().to_string())
            .collect();

        for name in &entry_names {
            let lower = name.to_ascii_lowercase();
            assert!(
                !lower.contains(".dat"),
                "arquivo de segredo .dat entrou: {name}"
            );
            assert!(!lower.contains(".log"), "arquivo de log entrou: {name}");
            assert!(!lower.contains("token"), "token entrou: {name}");
            assert!(!lower.contains("secret"), "segredo entrou: {name}");
            assert!(!lower.contains("cache"), "cache entrou: {name}");
            assert!(!lower.contains("session"), "sessão entrou: {name}");
            assert!(
                !lower.contains("cursor-hidden"),
                "marcador de cursor entrou: {name}"
            );
        }

        assert!(entry_names.contains(&"backup.toml".to_string()));
        assert!(entry_names.contains(&"tako.db".to_string()));
    }
}
