//! Backup e restauração local da biblioteca e das configurações (CLOUD-001).
//!
//! Gera e restaura um arquivo único `.takobackup` (formato ZIP com manifesto `backup.toml`).
//!
//! - **O que entra no backup:**
//!   - Banco SQLite (`tako.db`) copiado de forma consistente via `VACUUM INTO` (biblioteca,
//!     jogos manuais, estado, favoritos, tempo jogado, status, sessões, coleções, tags,
//!     configurações por jogo e configurações gerais do usuário).
//!   - Permissões (`approved_commands`), tokens e segredos da tabela `settings` são zerados
//!     no backup antes de compactar.
//!   - Configurações da máquina (`updates.json`, se existir).
//!   - Temas instalados pelo usuário (`themes/`, incluindo `.selected`).
//!   - Manifesto versionado (`backup.toml`).
//!
//! - **O que fica de fora:**
//!   - Cache de capas e de metadados (`cache/art`, `cache/metadata`, `cache/steam-owned.json`).
//!   - Registros de logs (`logs/`).
//!   - Segredos e tokens (chaves de API no cofre do SO, arquivos legados `.dat`, tokens da API local).
//!   - Arquivos temporários de sessão e processo (`session.json`, `session-environment.json`,
//!     `cursor-hidden`, `tako.lock`, `activate.json`, backups automáticos anteriores).
//!
//! - **Garantias de segurança na restauração:**
//!   - Validação do manifesto e rejeição de versões futuras com mensagem informativa.
//!   - Verificação do esquema real do SQLite (`PRAGMA user_version;` e `schema_migrations`)
//!     para rejeitar versões futuras sem confiar cegamente no manifesto.
//!   - Backup automático do estado atual antes de substituir qualquer arquivo; se o banco
//!     atual estiver corrompido, salva uma cópia bruta dos arquivos para não impedir o restauro.
//!   - Mantém apenas os últimos 5 backups automáticos.
//!   - Na restauração, mantém permissões (`approved_commands`) e consentimentos locais em vez
//!     de importar dados de terceiros.
//!   - Proteção contra Zip-Slip (rejeita `..`, caminhos absolutos, letras de unidade, nomes de dispositivo).
//!   - Rejeição de links simbólicos dentro do arquivo compactado.
//!   - Limite real na descompressão com `entry.take(MAX_FILE_BYTES + 1)`, teto no manifesto e
//!     limite no total descompactado.
//!   - Substituição atômica via `rename` (com rollback em caso de falha), garantindo que
//!     arquivos `-wal`, `-shm` e `-journal` só sejam removidos após o sucesso da troca do banco.

use super::{PersistenceError, TakoDatabase, format_date, migrator};
use crate::i18n;
use crate::paths::DataPaths;
use chrono::Utc;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fmt;
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
pub const MAX_MANIFEST_BYTES: u64 = 1024 * 1024; // 1 MB para o manifesto
pub const MAX_ENTRIES: usize = 10_000;
pub const MAX_AUTO_BACKUPS: usize = 5;
const MAX_PATH_LEN: usize = 1024;

const DEVICE_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

#[derive(Debug)]
pub enum BackupError {
    NotFound(String),
    TooLarge { size: u64, max: u64 },
    CorruptZip(String),
    UnsafePath { entry: String, reason: String },
    InvalidManifest(String),
    FutureVersion { version: u32, max_supported: u32 },
    FutureDatabaseVersion { version: u32, max_supported: u32 },
    CorruptDatabase(String),
    Persistence(PersistenceError),
    Io(io::Error),
    Sqlite(rusqlite::Error),
    Zip(zip::result::ZipError),
    TomlDe(toml::de::Error),
    TomlSer(toml::ser::Error),
}

impl std::error::Error for BackupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BackupError::Persistence(e) => Some(e),
            BackupError::Io(e) => Some(e),
            BackupError::Sqlite(e) => Some(e),
            BackupError::Zip(e) => Some(e),
            BackupError::TomlDe(e) => Some(e),
            BackupError::TomlSer(e) => Some(e),
            _ => None,
        }
    }
}

impl From<PersistenceError> for BackupError {
    fn from(err: PersistenceError) -> Self {
        BackupError::Persistence(err)
    }
}

impl From<io::Error> for BackupError {
    fn from(err: io::Error) -> Self {
        BackupError::Io(err)
    }
}

impl From<rusqlite::Error> for BackupError {
    fn from(err: rusqlite::Error) -> Self {
        BackupError::Sqlite(err)
    }
}

impl From<zip::result::ZipError> for BackupError {
    fn from(err: zip::result::ZipError) -> Self {
        BackupError::Zip(err)
    }
}

impl From<toml::de::Error> for BackupError {
    fn from(err: toml::de::Error) -> Self {
        BackupError::TomlDe(err)
    }
}

impl From<toml::ser::Error> for BackupError {
    fn from(err: toml::ser::Error) -> Self {
        BackupError::TomlSer(err)
    }
}

impl fmt::Display for BackupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackupError::NotFound(path) => {
                write!(
                    f,
                    "{}",
                    i18n::tf("backup.error.not_found", &[("path", path)])
                )
            }
            BackupError::TooLarge { size, max } => {
                let s = size.to_string();
                let m = max.to_string();
                write!(
                    f,
                    "{}",
                    i18n::tf("backup.error.too_large", &[("size", &s), ("max", &m)])
                )
            }
            BackupError::CorruptZip(err) => {
                write!(
                    f,
                    "{}",
                    i18n::tf("backup.error.corrupt_zip", &[("error", err)])
                )
            }
            BackupError::UnsafePath { entry, reason } => {
                write!(
                    f,
                    "{}",
                    i18n::tf(
                        "backup.error.unsafe_path",
                        &[("entry", entry), ("reason", reason)]
                    )
                )
            }
            BackupError::InvalidManifest(err) => {
                write!(
                    f,
                    "{}",
                    i18n::tf("backup.error.invalid_manifest", &[("error", err)])
                )
            }
            BackupError::FutureVersion {
                version,
                max_supported,
            } => {
                let v = version.to_string();
                let m = max_supported.to_string();
                write!(
                    f,
                    "{}",
                    i18n::tf(
                        "backup.error.future_version",
                        &[("version", &v), ("max_supported", &m)]
                    )
                )
            }
            BackupError::FutureDatabaseVersion {
                version,
                max_supported,
            } => {
                let v = version.to_string();
                let m = max_supported.to_string();
                write!(
                    f,
                    "{}",
                    i18n::tf(
                        "backup.error.future_database_version",
                        &[("version", &v), ("max_supported", &m)]
                    )
                )
            }
            BackupError::CorruptDatabase(err) => {
                write!(
                    f,
                    "{}",
                    i18n::tf("backup.error.corrupt_database", &[("error", err)])
                )
            }
            BackupError::Persistence(e) => {
                let err_str = e.to_string();
                write!(
                    f,
                    "{}",
                    i18n::tf("backup.error.persistence", &[("error", &err_str)])
                )
            }
            BackupError::Io(e) => {
                let err_str = e.to_string();
                write!(f, "{}", i18n::tf("backup.error.io", &[("error", &err_str)]))
            }
            BackupError::Sqlite(e) => {
                let err_str = e.to_string();
                write!(
                    f,
                    "{}",
                    i18n::tf("backup.error.sqlite", &[("error", &err_str)])
                )
            }
            BackupError::Zip(e) => {
                let err_str = e.to_string();
                write!(
                    f,
                    "{}",
                    i18n::tf("backup.error.zip", &[("error", &err_str)])
                )
            }
            BackupError::TomlDe(e) => {
                let err_str = e.to_string();
                write!(
                    f,
                    "{}",
                    i18n::tf("backup.error.toml_de", &[("error", &err_str)])
                )
            }
            BackupError::TomlSer(e) => {
                let err_str = e.to_string();
                write!(
                    f,
                    "{}",
                    i18n::tf("backup.error.toml_ser", &[("error", &err_str)])
                )
            }
        }
    }
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

/// Zera permissões (`approved_commands`) e segredos/consentimentos no banco fornecido.
fn sanitize_backup_database(conn: &Connection) -> Result<(), rusqlite::Error> {
    let has_game_settings: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='game_settings';",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if has_game_settings {
        conn.execute("UPDATE game_settings SET approved_commands = NULL;", [])?;
    }

    let has_settings: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='settings';",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if has_settings {
        conn.execute(
            "DELETE FROM settings
             WHERE key IN ('LocalApiEnabled', 'LocalApiToken')
                OR LOWER(key) LIKE '%token%'
                OR LOWER(key) LIKE '%secret%'
                OR LOWER(key) LIKE '%password%'
                OR LOWER(key) LIKE '%credential%'
                OR LOWER(key) LIKE '%auth%'
                OR LOWER(key) LIKE '%consent%'
                OR LOWER(key) LIKE '%permission%'
                OR LOWER(key) LIKE '%approved%'
                OR LOWER(key) LIKE '%api_key%'
                OR LOWER(key) LIKE '%apikey%'
                OR LOWER(key) LIKE '%api-key%';",
            [],
        )?;
    }
    Ok(())
}

/// Permissões e consentimentos locais preservados durante a restauração.
struct LocalPermissions {
    approved_commands: Vec<(String, String)>,
    sensitive_settings: Vec<(String, String)>,
}

fn extract_local_permissions(paths: &DataPaths) -> LocalPermissions {
    let db_path = paths.database();
    if !db_path.exists() {
        return LocalPermissions {
            approved_commands: Vec::new(),
            sensitive_settings: Vec::new(),
        };
    }
    let Ok(conn) = Connection::open(&db_path) else {
        return LocalPermissions {
            approved_commands: Vec::new(),
            sensitive_settings: Vec::new(),
        };
    };

    let mut approved_commands = Vec::new();
    let has_game_settings: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='game_settings';",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if has_game_settings
        && let Ok(mut stmt) = conn.prepare(
            "SELECT game_id, approved_commands FROM game_settings WHERE approved_commands IS NOT NULL;",
        )
        && let Ok(rows) =
            stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
    {
        for row in rows.flatten() {
            approved_commands.push(row);
        }
    }

    let mut sensitive_settings = Vec::new();
    let has_settings: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='settings';",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if has_settings
        && let Ok(mut stmt) = conn.prepare(
            "SELECT key, value FROM settings
             WHERE key IN ('LocalApiEnabled', 'LocalApiToken')
                OR LOWER(key) LIKE '%token%'
                OR LOWER(key) LIKE '%secret%'
                OR LOWER(key) LIKE '%password%'
                OR LOWER(key) LIKE '%credential%'
                OR LOWER(key) LIKE '%auth%'
                OR LOWER(key) LIKE '%consent%'
                OR LOWER(key) LIKE '%permission%'
                OR LOWER(key) LIKE '%approved%'
                OR LOWER(key) LIKE '%api_key%'
                OR LOWER(key) LIKE '%apikey%'
                OR LOWER(key) LIKE '%api-key%';",
        )
        && let Ok(rows) =
            stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
    {
        for row in rows.flatten() {
            sensitive_settings.push(row);
        }
    }

    LocalPermissions {
        approved_commands,
        sensitive_settings,
    }
}

/// Gera um arquivo `.takobackup` no destino especificado.
///
/// O banco SQLite é copiado consistentemente usando `VACUUM INTO`, garantindo
/// que transações ativas e arquivos WAL não gerem dados corrompidos.
/// No banco temporário antes de zipar, zera `approved_commands` e remove segredos.
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

    // No banco temporário, antes de zipar, zera approved_commands e segredos/consentimentos
    {
        let temp_conn = Connection::open(&temp_db_path)?;
        sanitize_backup_database(&temp_conn)?;
    }

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

        // 2. Banco SQLite íntegro e saneado
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

/// Cria um backup automático bruto salvando os arquivos diretamente em caso de corrupção do SQLite.
fn create_raw_auto_backup(paths: &DataPaths, dest_file: &Path) -> Result<(), BackupError> {
    if let Some(parent) = dest_file.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }

    let manifest = BackupManifest {
        version: CURRENT_BACKUP_VERSION,
        tako_version: env!("CARGO_PKG_VERSION").into(),
        database_version: 0,
        created_at: format_date(Utc::now()),
        os: format!("{:?}", crate::paths::Os::current()),
        description: Some("Backup automático bruto (banco anterior com falha)".into()),
    };
    let manifest_toml = toml::to_string(&manifest)?;

    let temp_dest = tempfile::Builder::new()
        .prefix(".backup-raw-")
        .suffix(".tmp")
        .tempfile_in(dest_file.parent().unwrap_or_else(|| Path::new(".")))?;

    {
        let mut zip = ZipWriter::new(temp_dest.as_file());
        let file_options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        // 1. Manifesto
        zip.start_file(MANIFEST_FILE, file_options)?;
        zip.write_all(manifest_toml.as_bytes())?;

        // 2. Cópia bruta do banco tako.db (e wal/shm/journal se existirem)
        let db_path = paths.database();
        if db_path.is_file()
            && let Ok(mut f) = File::open(&db_path)
        {
            zip.start_file("tako.db", file_options)?;
            let _ = io::copy(&mut f, &mut zip);
        }
        for suffix in ["-wal", "-shm", "-journal"] {
            let mut aux = db_path.as_os_str().to_owned();
            aux.push(suffix);
            let aux_path = PathBuf::from(aux);
            if aux_path.is_file() {
                let name = format!("tako.db{suffix}");
                if let Ok(mut f) = File::open(&aux_path) {
                    let _ = zip.start_file(name, file_options);
                    let _ = io::copy(&mut f, &mut zip);
                }
            }
        }

        // 3. Temas
        let themes_root = paths.themes();
        if themes_root.is_dir() {
            let _ = add_themes_to_zip(&themes_root, &mut zip, file_options);
        }

        // 4. updates.json
        let updates_file = paths.updates();
        if updates_file.is_file()
            && let Ok(mut f) = File::open(&updates_file)
        {
            let _ = zip.start_file("updates.json", file_options);
            let _ = io::copy(&mut f, &mut zip);
        }

        zip.finish()?;
    }

    temp_dest
        .persist(dest_file)
        .map_err(|e| io::Error::other(e.error))?;

    Ok(())
}

/// Mantém apenas os N backups automáticos mais recentes.
fn prune_auto_backups(backups_dir: &Path, keep: usize) {
    let Ok(entries) = fs::read_dir(backups_dir) else {
        return;
    };
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("auto-backup-") && name.ends_with(".takobackup") {
                files.push(path);
            }
        }
    }
    // Ordenação lexicográfica cronológica graças ao formato %Y%m%d-%H%M%S-%3f
    files.sort();
    if files.len() > keep {
        let to_remove = files.len() - keep;
        for f in &files[..to_remove] {
            let _ = fs::remove_file(f);
        }
    }
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
        .ok_or_else(|| BackupError::InvalidManifest(i18n::t("backup.error.manifest_missing")))?;

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

        println!(
            "DEBUG ENTRY {i}: name={}, is_symlink={}, mode={:?}",
            raw_name,
            entry.is_symlink(),
            entry.unix_mode()
        );
        if entry.is_symlink() {
            return Err(BackupError::UnsafePath {
                entry: raw_name,
                reason: i18n::t("backup.error.symlink_not_allowed"),
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
            reason,
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
        return Err(BackupError::InvalidManifest(i18n::t(
            "backup.error.manifest_missing",
        )));
    }
    if !has_db {
        return Err(BackupError::InvalidManifest(i18n::t(
            "backup.error.db_missing",
        )));
    }

    // 2. Validação do manifesto e versão
    let manifest_bytes = read_entry_bytes(&mut zip, MANIFEST_FILE)?
        .ok_or_else(|| BackupError::InvalidManifest(i18n::t("backup.error.manifest_missing")))?;
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

    // Preserva permissões e consentimentos locais atuais antes de tocar nos arquivos
    let local_perms = extract_local_permissions(paths);

    // 3. Guarda backup automático do estado atual antes de substituir qualquer arquivo
    let auto_backup_path = if paths.database().exists() {
        let backups_dir = paths.root.join("backups");
        fs::create_dir_all(&backups_dir)?;
        let timestamp = Utc::now().format("%Y%m%d-%H%M%S-%3f");
        let path = backups_dir.join(format!("auto-backup-{timestamp}.takobackup"));

        let created = match create_backup(paths, &path) {
            Ok(_) => true,
            Err(e) => {
                tracing::warn!(error = %e, "create_backup falhou no auto-backup; gerando cópia bruta dos arquivos");
                create_raw_auto_backup(paths, &path).is_ok()
            }
        };

        if created {
            prune_auto_backups(&backups_dir, MAX_AUTO_BACKUPS);
            tracing::info!(auto = %path.display(), "backup automático salvo antes da restauração");
            Some(path)
        } else {
            None
        }
    } else {
        None
    };

    // 4. Extração segura para pasta temporária de preparação (staging)
    fs::create_dir_all(&paths.root)?;
    let staging_dir = tempfile::Builder::new()
        .prefix(".restore-")
        .tempdir_in(&paths.root)?;

    let mut actual_uncompressed: u64 = 0;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| BackupError::CorruptZip(e.to_string()))?;
        let raw_name = entry
            .name()
            .map_err(|e| BackupError::CorruptZip(e.to_string()))?
            .to_string();

        if entry.is_symlink() {
            return Err(BackupError::UnsafePath {
                entry: raw_name,
                reason: i18n::t("backup.error.symlink_not_allowed"),
            });
        }

        let Some(normalized) =
            normalize_entry(&raw_name).map_err(|reason| BackupError::UnsafePath {
                entry: raw_name,
                reason,
            })?
        else {
            continue; // Entrada de pasta pura
        };

        let target_path = staging_dir.path().join(&normalized);
        // Garantia de Zip-Slip: o caminho deve permanecer dentro de staging
        if !target_path.starts_with(staging_dir.path()) {
            return Err(BackupError::UnsafePath {
                entry: normalized,
                reason: i18n::t("backup.error.escapes_target_dir"),
            });
        }

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut outfile = File::create(&target_path)?;
        let mut limited = (&mut entry).take(MAX_FILE_BYTES + 1);
        let written = io::copy(&mut limited, &mut outfile)?;
        if written > MAX_FILE_BYTES {
            return Err(BackupError::TooLarge {
                size: written,
                max: MAX_FILE_BYTES,
            });
        }
        actual_uncompressed = actual_uncompressed.saturating_add(written);
        if actual_uncompressed > MAX_UNCOMPRESSED_BYTES {
            return Err(BackupError::TooLarge {
                size: actual_uncompressed,
                max: MAX_UNCOMPRESSED_BYTES,
            });
        }
    }

    // 5. Verificação da integridade e versão do SQLite descompactado
    let extracted_db = staging_dir.path().join("tako.db");
    {
        let conn = Connection::open(&extracted_db)?;
        let check: String = conn.query_row("PRAGMA quick_check;", [], |r| r.get(0))?;
        if check != "ok" {
            return Err(BackupError::CorruptDatabase(format!(
                "quick_check: {check}"
            )));
        }

        // Lê a versão real do banco do SQLite (PRAGMA user_version e schema_migrations)
        let pragma_version: u32 = conn
            .query_row("PRAGMA user_version;", [], |r| r.get(0))
            .unwrap_or(0);
        let migration_version = migrator::current_version(&conn)?;
        let real_db_version = pragma_version.max(migration_version);

        if real_db_version > latest_supported_db {
            return Err(BackupError::FutureDatabaseVersion {
                version: real_db_version,
                max_supported: latest_supported_db,
            });
        }

        // Aplica migrações pendentes se o backup for de versão anterior
        migrator::migrate(&conn, None)?;

        // Garante que o banco do backup não injete approved_commands ou segredos
        sanitize_backup_database(&conn)?;

        // Restaura permissões e consentimentos locais da máquina atual
        for (game_id, approved) in &local_perms.approved_commands {
            conn.execute(
                "INSERT INTO game_settings (game_id, approved_commands) VALUES (?1, ?2)
                 ON CONFLICT(game_id) DO UPDATE SET approved_commands = excluded.approved_commands;",
                rusqlite::params![game_id, approved],
            )?;
        }
        for (key, value) in &local_perms.sensitive_settings {
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2);",
                rusqlite::params![key, value],
            )?;
        }
    }

    // 6. Substituição definitiva dos arquivos no destino (atômica, com rollback em erro)
    // 6.1 Banco de dados
    let db_path = paths.database();
    let db_old = db_path.with_extension("db.old");
    let had_old_db = db_path.exists();
    if had_old_db {
        if db_old.exists() {
            let _ = fs::remove_file(&db_old);
        }
        fs::rename(&db_path, &db_old)?;
    }
    if let Err(e) = fs::rename(&extracted_db, &db_path) {
        if had_old_db {
            let _ = fs::rename(&db_old, &db_path);
        }
        return Err(e.into());
    }
    // Sucesso no rename do banco: agora remove db.old e os logs WAL anteriores
    if had_old_db {
        let _ = fs::remove_file(&db_old);
    }
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut aux = db_path.as_os_str().to_owned();
        aux.push(suffix);
        let _ = fs::remove_file(aux);
    }

    // 6.2 Temas instalados: renomeia pasta inteira para não mesclar
    let extracted_themes = staging_dir.path().join("themes");
    if extracted_themes.is_dir() {
        let dest_themes = paths.themes();
        let themes_old = dest_themes.with_extension("themes.old");
        let had_old_themes = dest_themes.exists();
        if had_old_themes {
            if themes_old.exists() {
                let _ = fs::remove_dir_all(&themes_old);
            }
            fs::rename(&dest_themes, &themes_old)?;
        }
        if let Err(e) = fs::rename(&extracted_themes, &dest_themes) {
            if had_old_themes {
                let _ = fs::rename(&themes_old, &dest_themes);
            }
            return Err(e.into());
        }
        if had_old_themes {
            let _ = fs::remove_dir_all(&themes_old);
        }
    }

    // 6.3 updates.json se veio no backup
    let extracted_updates = staging_dir.path().join("updates.json");
    if extracted_updates.is_file() {
        let dest_updates = paths.updates();
        let updates_old = dest_updates.with_extension("json.old");
        let had_old_updates = dest_updates.exists();
        if had_old_updates {
            if updates_old.exists() {
                let _ = fs::remove_file(&updates_old);
            }
            fs::rename(&dest_updates, &updates_old)?;
        }
        if let Err(e) = fs::rename(&extracted_updates, &dest_updates) {
            if had_old_updates {
                let _ = fs::rename(&updates_old, &dest_updates);
            }
            return Err(e.into());
        }
        if had_old_updates {
            let _ = fs::remove_file(&updates_old);
        }
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

fn is_known_secret_file(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".dat")
        || lower.ends_with(".dat.migrado")
        || lower.ends_with(".log")
        || lower.ends_with(".key")
        || lower.ends_with(".token")
        || lower.ends_with(".pem")
        || lower == "steam-web-api-key"
        || lower == "igdb.dat"
        || lower == "screenscraper.dat"
        || lower == "steam-owned.json"
        || lower == "token.json"
        || lower == "secret.json"
        || lower == "credentials.json"
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
            // Defesa em profundidade: filtra apenas por arquivos de segredo conhecidos
            if is_known_secret_file(&file_name) {
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
            let mut limited = (&mut entry).take(MAX_MANIFEST_BYTES + 1);
            let mut bytes = Vec::new();
            limited.read_to_end(&mut bytes)?;
            if bytes.len() as u64 > MAX_MANIFEST_BYTES {
                return Err(BackupError::TooLarge {
                    size: bytes.len() as u64,
                    max: MAX_MANIFEST_BYTES,
                });
            }
            return Ok(Some(bytes));
        }
    }
    Ok(None)
}

/// Normalização e validação de caminhos para proteção contra Zip-Slip.
///
/// Rejeita caminhos absolutos, segmentos `..`, separadores de unidade e caracteres ilegais.
pub fn normalize_entry(name: &str) -> Result<Option<String>, String> {
    if name.is_empty() {
        return Err(i18n::t("backup.error.empty_path"));
    }
    let path = name.replace('\\', "/");
    if path.starts_with('/') {
        return Err(i18n::t("backup.error.absolute_path_not_allowed"));
    }
    if path.chars().count() > MAX_PATH_LEN {
        return Err(i18n::t("backup.error.path_too_long"));
    }

    let mut segments = Vec::new();
    for s in path.split('/') {
        if s.is_empty() || s == "." {
            continue;
        }
        if s == ".." {
            return Err(i18n::t("backup.error.parent_dir_not_allowed"));
        }
        if s.chars().any(|c| c.is_control() || "<>:\"|?*".contains(c)) {
            return Err(if s.contains(':') {
                i18n::t("backup.error.drive_colon_forbidden")
            } else {
                i18n::t("backup.error.special_chars_forbidden")
            });
        }
        if s.ends_with('.') || s.ends_with(' ') {
            return Err(i18n::t("backup.error.trailing_dot_or_space_forbidden"));
        }
        if is_device_name(s.split('.').next().unwrap_or(s)) {
            return Err(i18n::t("backup.error.device_name_forbidden"));
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
    use crate::persistence::{
        GameSettingsStore, GameStateRow, LibraryStore, ManualGameRow, SettingsStore,
    };
    use crate::session::settings::GameSettingsOverride;

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
        assert!(msg.contains("999"));
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
            assert!(!lower.ends_with(".key"), "chave entrou: {name}");
            assert!(!lower.ends_with(".token"), "token entrou: {name}");
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

    #[test]
    fn permissions_and_secrets_are_zeroed_in_backup_and_not_imported_on_restore() {
        let dir_src = tempfile::tempdir().unwrap();
        let dir_dst = tempfile::tempdir().unwrap();
        let paths_src = make_test_paths(dir_src.path());
        let paths_dst = make_test_paths(dir_dst.path());

        // 1. Configura banco de origem com comandos aprovados e tokens
        let db_src = TakoDatabase::new(paths_src.database());
        db_src.open().unwrap();
        let game_settings_src = GameSettingsStore::new(db_src.clone());
        let g_override = GameSettingsOverride {
            before_command: Some("sh -c 'evil.sh'".into()),
            approved_commands: Some("sha256_of_evil".into()),
            ..Default::default()
        };
        game_settings_src
            .save("manual:evil_game", &g_override)
            .unwrap();

        let settings_src = SettingsStore::new(db_src);
        settings_src.set("LocalApiEnabled", "true").unwrap();
        settings_src
            .set("LocalApiToken", "super-secret-token-1234")
            .unwrap();
        settings_src.set("my_api_key", "secret-key").unwrap();
        settings_src.set("Theme", "custom").unwrap();

        // 2. Cria o backup
        let backup_file = dir_src.path().join("test.takobackup");
        create_backup(&paths_src, &backup_file).unwrap();

        // 2.1 Verifica que o tako.db dentro do zip tem approved_commands = NULL e segredos apagados
        {
            let mut zip = ZipArchive::new(File::open(&backup_file).unwrap()).unwrap();
            let mut db_bytes = Vec::new();
            zip.by_name("tako.db")
                .unwrap()
                .read_to_end(&mut db_bytes)
                .unwrap();
            let tmp_db_file = dir_src.path().join("inspected.db");
            fs::write(&tmp_db_file, db_bytes).unwrap();

            let conn = Connection::open(&tmp_db_file).unwrap();
            let approved: Option<String> = conn
                .query_row(
                    "SELECT approved_commands FROM game_settings WHERE game_id = 'manual:evil_game';",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(
                approved, None,
                "approved_commands deveria ser NULL no backup"
            );

            let token: Option<String> = conn
                .query_row(
                    "SELECT value FROM settings WHERE key = 'LocalApiToken';",
                    [],
                    |r| r.get(0),
                )
                .ok();
            assert_eq!(token, None, "LocalApiToken não deveria entrar no backup");

            let enabled: Option<String> = conn
                .query_row(
                    "SELECT value FROM settings WHERE key = 'LocalApiEnabled';",
                    [],
                    |r| r.get(0),
                )
                .ok();
            assert_eq!(
                enabled, None,
                "LocalApiEnabled não deveria entrar no backup"
            );

            let key_val: Option<String> = conn
                .query_row(
                    "SELECT value FROM settings WHERE key = 'my_api_key';",
                    [],
                    |r| r.get(0),
                )
                .ok();
            assert_eq!(key_val, None, "my_api_key não deveria entrar no backup");

            // Configuração normal (Theme) deve permanecer
            let theme: Option<String> = conn
                .query_row("SELECT value FROM settings WHERE key = 'Theme';", [], |r| {
                    r.get(0)
                })
                .ok();
            assert_eq!(theme.as_deref(), Some("custom"));
        }

        // 3. No destino, configura banco com permissão local legítima e token local próprio
        let db_dst = TakoDatabase::new(paths_dst.database());
        db_dst.open().unwrap();
        let game_settings_dst = GameSettingsStore::new(db_dst.clone());
        let local_override = GameSettingsOverride {
            approved_commands: Some("legit_local_approved".into()),
            ..Default::default()
        };
        game_settings_dst
            .save("manual:local_game", &local_override)
            .unwrap();

        let settings_dst = SettingsStore::new(db_dst);
        settings_dst.set("LocalApiEnabled", "true").unwrap();
        settings_dst
            .set("LocalApiToken", "my-local-machine-token")
            .unwrap();

        // 4. Restaura o backup no destino
        restore_backup(&paths_dst, &backup_file).unwrap();

        // 5. Verifica que permissões locais foram mantidas e que o jogo importado NÃO tem permissão
        let conn_dst = Connection::open(paths_dst.database()).unwrap();
        let evil_approved: Option<String> = conn_dst
            .query_row(
                "SELECT approved_commands FROM game_settings WHERE game_id = 'manual:evil_game';",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            evil_approved, None,
            "jogo importado não pode ter comandos aprovados"
        );

        let local_approved: Option<String> = conn_dst
            .query_row(
                "SELECT approved_commands FROM game_settings WHERE game_id = 'manual:local_game';",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            local_approved.as_deref(),
            Some("legit_local_approved"),
            "permissão local deve ser mantida após restauração"
        );

        let local_token: String = conn_dst
            .query_row(
                "SELECT value FROM settings WHERE key = 'LocalApiToken';",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            local_token, "my-local-machine-token",
            "token local deve ser preservado"
        );
    }

    #[test]
    fn future_database_version_in_sqlite_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let paths = make_test_paths(dir.path());
        let future_db_zip = dir.path().join("future_db.takobackup");

        // Cria banco SQLite com user_version no futuro (999)
        let temp_db_path = dir.path().join("future.db");
        {
            let conn = Connection::open(&temp_db_path).unwrap();
            conn.execute_batch(
                "PRAGMA user_version = 999;
                 CREATE TABLE test (id INT);",
            )
            .unwrap();
        }

        {
            let file = File::create(&future_db_zip).unwrap();
            let mut zip = ZipWriter::new(file);
            let opts = SimpleFileOptions::default();

            // O manifesto mente dizendo que a versão é 1
            let manifest = BackupManifest {
                version: CURRENT_BACKUP_VERSION,
                tako_version: "0.2.0".into(),
                database_version: 1,
                created_at: format_date(Utc::now()),
                os: "Linux".into(),
                description: None,
            };
            zip.start_file(MANIFEST_FILE, opts).unwrap();
            zip.write_all(toml::to_string(&manifest).unwrap().as_bytes())
                .unwrap();

            zip.start_file("tako.db", opts).unwrap();
            let mut f = File::open(&temp_db_path).unwrap();
            io::copy(&mut f, &mut zip).unwrap();

            zip.finish().unwrap();
        }

        let err = restore_backup(&paths, &future_db_zip).unwrap_err();
        assert!(
            matches!(err, BackupError::FutureDatabaseVersion { version: 999, .. }),
            "deveria rejeitar versão futura lida do banco SQLite real"
        );
    }

    #[test]
    fn old_database_is_migrated_on_restore() {
        let dir = tempfile::tempdir().unwrap();
        let paths = make_test_paths(dir.path());
        let old_backup_file = dir.path().join("old.takobackup");

        // Cria banco SQLite antigo na versão 1 do migrador
        let temp_db_path = dir.path().join("old.db");
        {
            let conn = Connection::open(&temp_db_path).unwrap();
            migrator::migrate(&conn, Some(1)).unwrap();
            assert_eq!(migrator::current_version(&conn).unwrap(), 1);
        }

        {
            let file = File::create(&old_backup_file).unwrap();
            let mut zip = ZipWriter::new(file);
            let opts = SimpleFileOptions::default();

            let manifest = BackupManifest {
                version: CURRENT_BACKUP_VERSION,
                tako_version: "0.1.0".into(),
                database_version: 1,
                created_at: format_date(Utc::now()),
                os: "Linux".into(),
                description: None,
            };
            zip.start_file(MANIFEST_FILE, opts).unwrap();
            zip.write_all(toml::to_string(&manifest).unwrap().as_bytes())
                .unwrap();

            zip.start_file("tako.db", opts).unwrap();
            let mut f = File::open(&temp_db_path).unwrap();
            io::copy(&mut f, &mut zip).unwrap();

            zip.finish().unwrap();
        }

        // Restaura e verifica se o banco foi migrado para a versão mais recente
        restore_backup(&paths, &old_backup_file).unwrap();

        let restored_conn = Connection::open(paths.database()).unwrap();
        let version_after = migrator::current_version(&restored_conn).unwrap();
        assert_eq!(
            version_after,
            migrator::latest_version(),
            "o banco antigo deve ter sido migrado para a versão mais recente"
        );
    }

    #[test]
    fn corrupted_database_triggers_raw_auto_backup_and_prunes_older() {
        let dir = tempfile::tempdir().unwrap();
        let paths = make_test_paths(dir.path());

        // Coloca um banco CORROMPIDO no destino
        fs::create_dir_all(&paths.root).unwrap();
        fs::write(paths.database(), b"banco corrompido que falha quick_check").unwrap();

        // Cria 7 backups automáticos prévios
        let backups_dir = paths.root.join("backups");
        fs::create_dir_all(&backups_dir).unwrap();
        for i in 1..=7 {
            let name = format!("auto-backup-20261019-00000{i}-000.takobackup");
            fs::write(backups_dir.join(name), b"dummy backup").unwrap();
        }

        // Cria um backup válido para restaurar
        let valid_backup = dir.path().join("valid.takobackup");
        {
            let valid_db_path = dir.path().join("valid.db");
            let conn = Connection::open(&valid_db_path).unwrap();
            migrator::migrate(&conn, None).unwrap();

            let file = File::create(&valid_backup).unwrap();
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
            let mut f = File::open(&valid_db_path).unwrap();
            io::copy(&mut f, &mut zip).unwrap();

            zip.finish().unwrap();
        }

        // A restauração NÃO pode falhar pelo banco atual estar corrompido!
        let res = restore_backup(&paths, &valid_backup).unwrap();
        assert!(res.auto_backup_path.is_some());
        assert!(res.auto_backup_path.unwrap().exists());

        // Confere que foram mantidos apenas 5 backups automáticos
        let auto_backups: Vec<_> = fs::read_dir(&backups_dir)
            .unwrap()
            .flatten()
            .filter(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                name.starts_with("auto-backup-") && name.ends_with(".takobackup")
            })
            .collect();
        assert_eq!(
            auto_backups.len(),
            MAX_AUTO_BACKUPS,
            "deve manter exatamente os 5 últimos auto-backups"
        );
    }

    #[test]
    fn themes_with_token_or_secret_in_name_are_preserved() {
        let dir_src = tempfile::tempdir().unwrap();
        let dir_dst = tempfile::tempdir().unwrap();
        let paths_src = make_test_paths(dir_src.path());
        let paths_dst = make_test_paths(dir_dst.path());

        let db = TakoDatabase::new(paths_src.database());
        db.open().unwrap();

        // Tema com "token" e "secret" no nome da pasta e arquivos
        let theme_dir = paths_src.themes().join("theme-secret-token");
        fs::create_dir_all(&theme_dir).unwrap();
        fs::write(theme_dir.join("theme.toml"), "name = 'Secret Theme'\n").unwrap();
        fs::write(theme_dir.join("secret_token_asset.txt"), "asset content").unwrap();

        let backup_file = dir_src.path().join("themes.takobackup");
        create_backup(&paths_src, &backup_file).unwrap();

        restore_backup(&paths_dst, &backup_file).unwrap();

        let restored_theme = paths_dst.themes().join("theme-secret-token");
        assert!(restored_theme.join("theme.toml").exists());
        assert_eq!(
            fs::read_to_string(restored_theme.join("secret_token_asset.txt")).unwrap(),
            "asset content"
        );
    }

    #[test]
    fn symlink_in_backup_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let paths = make_test_paths(dir.path());
        let symlink_zip = dir.path().join("symlink.takobackup");

        let valid_db_path = dir.path().join("valid.db");
        {
            let conn = Connection::open(&valid_db_path).unwrap();
            migrator::migrate(&conn, None).unwrap();
            conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                .unwrap();
        }

        {
            let file = File::create(&symlink_zip).unwrap();
            let mut zip = ZipWriter::new(file);
            let opts = SimpleFileOptions::default().external_attributes(0o120777 << 16);

            let manifest = BackupManifest {
                version: CURRENT_BACKUP_VERSION,
                tako_version: "0.2.0".into(),
                database_version: migrator::latest_version(),
                created_at: format_date(Utc::now()),
                os: "Linux".into(),
                description: None,
            };
            zip.start_file(MANIFEST_FILE, SimpleFileOptions::default())
                .unwrap();
            zip.write_all(toml::to_string(&manifest).unwrap().as_bytes())
                .unwrap();

            zip.start_file("tako.db", SimpleFileOptions::default())
                .unwrap();
            let mut f = File::open(&valid_db_path).unwrap();
            io::copy(&mut f, &mut zip).unwrap();

            zip.start_file("themes/symlink", opts).unwrap();
            zip.write_all(b"/etc/passwd").unwrap();

            zip.finish().unwrap();
        }

        let err = restore_backup(&paths, &symlink_zip).unwrap_err();
        assert!(matches!(err, BackupError::UnsafePath { .. }));
        let msg = err.to_string();
        assert!(
            msg.contains("links simbólicos") || msg.contains("symbolic links"),
            "mensagem de erro: {msg}"
        );
    }

    #[test]
    fn file_exceeding_max_file_bytes_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let paths = make_test_paths(dir.path());
        let huge_zip = dir.path().join("huge.takobackup");

        // Testa limite do manifesto (MAX_MANIFEST_BYTES)
        {
            let file = File::create(&huge_zip).unwrap();
            let mut zip = ZipWriter::new(file);
            let opts = SimpleFileOptions::default();

            zip.start_file(MANIFEST_FILE, opts).unwrap();
            let padding = vec![b' '; (MAX_MANIFEST_BYTES + 1024) as usize];
            zip.write_all(&padding).unwrap();

            zip.start_file("tako.db", opts).unwrap();
            zip.write_all(b"sqlite").unwrap();

            zip.finish().unwrap();
        }

        let err = restore_backup(&paths, &huge_zip).unwrap_err();
        assert!(matches!(err, BackupError::TooLarge { .. }));
    }
}
