//! Migração dos dados das alphas C# (MIG-030, D-06).
//!
//! No Windows o Rust usa a mesma pasta de dados do C# (instalado: `%LOCALAPPDATA%\Tako`; portable:
//! `<exe>\Tako_Data`, ver [`crate::paths`]), então `tako.db`, `updates.json` e as escolhas ficam onde estão:
//! - o banco ganha uma cópia `tako.db.alpha-backup` antes de subir da v6 para a v7 (feito ao abrir o banco);
//! - os segredos dos provedores (`igdb.dat`, `screenscraper.dat`, `steamgriddb.dat`, cifrados com DPAPI) vão
//!   para o cofre do SO e os arquivos viram `.dat.migrado`. A decifragem fica em `tako-platform`.

use super::database::with_suffix;
use super::{Result, migrator};
use crate::metadata::SecretStore;
use crate::metadata::igdb::IgdbSettings;
use crate::metadata::screenscraper::ScreenScraperSettings;
use crate::metadata::steamgriddb::SteamGridDbSettings;
use crate::paths::DataPaths;
use rusqlite::Connection;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::fmt;
use std::path::{Path, PathBuf};

pub const ALPHA_BACKUP_SUFFIX: &str = ".alpha-backup";
pub const MIGRATED_SUFFIX: &str = ".migrado";
/// Prefixo do `DpapiSecretProtector` do C#; texto sem ele é JSON puro.
pub const DPAPI_PREFIX: &str = "dpapi:";

pub const IGDB_FILE: &str = "igdb.dat";
pub const SCREENSCRAPER_FILE: &str = "screenscraper.dat";
pub const STEAMGRIDDB_FILE: &str = "steamgriddb.dat";

/// Arquivos de dados das alphas são pequenos; acima disto é lixo.
const MAX_SMALL_FILE: u64 = 64 * 1024;

/// Chaves no cofre do SO (`tako_platform::secrets`).
pub mod secret_keys {
    pub const IGDB_CLIENT_ID: &str = "igdb.client_id";
    pub const IGDB_CLIENT_SECRET: &str = "igdb.client_secret";
    pub const SCREENSCRAPER_USER: &str = "screenscraper.user";
    pub const SCREENSCRAPER_PASSWORD: &str = "screenscraper.password";
    pub const STEAMGRIDDB_API_KEY: &str = "steamgriddb.api_key";
}

/// Chaves na tabela `settings` para o que não é segredo.
pub mod setting_keys {
    pub const IGDB_ENABLED: &str = "Igdb.Enabled";
    pub const SCREENSCRAPER_ENABLED: &str = "ScreenScraper.Enabled";
    pub const STEAMGRIDDB_ENABLED: &str = "SteamGridDb.Enabled";
    pub const STEAMGRIDDB_AUTO_FILL: &str = "SteamGridDb.AutoFill";
    pub const STEAMGRIDDB_ANIMATED: &str = "SteamGridDb.Animated";
}

/// O que sobrou de uma alpha na pasta de dados.
#[derive(Debug, Clone, PartialEq)]
pub struct AlphaData {
    pub root: PathBuf,
    /// `tako.db` numa versão das alphas (v1..v6).
    pub database: bool,
    /// `.dat` ainda não migrados.
    pub secret_files: Vec<PathBuf>,
}

impl AlphaData {
    /// `None` quando não há nada das alphas (instalação nova, ou já migrada).
    pub fn detect(paths: &DataPaths) -> Option<AlphaData> {
        let root = &paths.root;
        let secret_files: Vec<PathBuf> = [IGDB_FILE, SCREENSCRAPER_FILE, STEAMGRIDDB_FILE]
            .into_iter()
            .map(|f| root.join(f))
            .filter(|p| p.is_file())
            .collect();
        let database = database_version(&paths.database())
            .is_some_and(|v| (1..=migrator::LAST_ALPHA_VERSION).contains(&v));
        (database || !secret_files.is_empty()).then(|| AlphaData {
            root: root.clone(),
            database,
            secret_files,
        })
    }
}

/// Lê a versão sem migrar nem criar nada (abre só para leitura).
fn database_version(path: &Path) -> Option<u32> {
    if !path.is_file() {
        return None;
    }
    let conn =
        Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
    conn.query_row("SELECT MAX(version) FROM schema_migrations;", [], |r| {
        r.get::<_, Option<u32>>(0)
    })
    .ok()
    .flatten()
}

/// Antes de subir um banco das alphas, guarda uma cópia íntegra ao lado (`VACUUM INTO`). Nunca troca uma
/// cópia que já existe: ela é o estado da alpha. Devolve se fez a cópia.
pub(crate) fn backup_alpha_database(conn: &Connection, db_path: &Path) -> Result<bool> {
    let version = migrator::current_version(conn)?;
    let backup = with_suffix(db_path, ALPHA_BACKUP_SUFFIX);
    if !(1..=migrator::LAST_ALPHA_VERSION).contains(&version) || backup.exists() {
        return Ok(false);
    }
    conn.execute("VACUUM INTO ?1;", [backup.to_string_lossy()])?;
    tracing::info!(version, backup = %backup.display(), "cópia do banco da alpha antes de migrar");
    Ok(true)
}

/// `updates.json` do C# (`UpdatePreferences`, System.Text.Json com nomes em PascalCase).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct UpdatePreferences {
    /// Procurar e baixar ao abrir.
    pub auto_update: bool,
    /// Receber versões de teste (alpha/beta) numa versão estável.
    pub prereleases: bool,
}

impl Default for UpdatePreferences {
    fn default() -> Self {
        UpdatePreferences {
            auto_update: true,
            prereleases: false,
        }
    }
}

impl UpdatePreferences {
    /// Arquivo ausente ou ilegível = padrão, como no C#.
    pub fn read(path: &Path) -> UpdatePreferences {
        read_small(path)
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }
}

/// Credenciais e opções dos provedores lidas dos `.dat`. `None` = arquivo ausente ou ilegível.
/// O `Debug` nunca mostra os segredos.
#[derive(Clone, Default, PartialEq)]
pub struct MigratedSecrets {
    pub igdb_client_id: Option<String>,
    pub igdb_client_secret: Option<String>,
    pub screenscraper_user: Option<String>,
    pub screenscraper_password: Option<String>,
    pub steamgriddb_api_key: Option<String>,
    pub igdb_enabled: Option<bool>,
    pub screenscraper_enabled: Option<bool>,
    pub steamgriddb_enabled: Option<bool>,
    pub steamgriddb_auto_fill: Option<bool>,
    pub steamgriddb_animated: Option<bool>,
    /// Arquivos lidos: só estes viram `.migrado`.
    pub read_files: Vec<PathBuf>,
}

impl fmt::Debug for MigratedSecrets {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MigratedSecrets")
            .field("secrets", &self.secret_entries().len())
            .field("flags", &self.flag_entries())
            .field("read_files", &self.read_files)
            .finish()
    }
}

impl MigratedSecrets {
    /// `(chave no cofre, valor)`, sem os vazios.
    pub fn secret_entries(&self) -> Vec<(&'static str, &str)> {
        use secret_keys::*;
        [
            (IGDB_CLIENT_ID, &self.igdb_client_id),
            (IGDB_CLIENT_SECRET, &self.igdb_client_secret),
            (SCREENSCRAPER_USER, &self.screenscraper_user),
            (SCREENSCRAPER_PASSWORD, &self.screenscraper_password),
            (STEAMGRIDDB_API_KEY, &self.steamgriddb_api_key),
        ]
        .into_iter()
        .filter_map(|(k, v)| Some((k, v.as_deref().filter(|v| !v.is_empty())?)))
        .collect()
    }

    /// `(chave em settings, valor)`.
    pub fn flag_entries(&self) -> Vec<(&'static str, bool)> {
        use setting_keys::*;
        [
            (IGDB_ENABLED, self.igdb_enabled),
            (SCREENSCRAPER_ENABLED, self.screenscraper_enabled),
            (STEAMGRIDDB_ENABLED, self.steamgriddb_enabled),
            (STEAMGRIDDB_AUTO_FILL, self.steamgriddb_auto_fill),
            (STEAMGRIDDB_ANIMATED, self.steamgriddb_animated),
        ]
        .into_iter()
        .filter_map(|(k, v)| Some((k, v?)))
        .collect()
    }
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "PascalCase", default)]
struct IgdbStored {
    enabled: bool,
    client_id: Option<String>,
    client_secret: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "PascalCase", default)]
struct ScreenScraperStored {
    enabled: bool,
    user: Option<String>,
    password: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "PascalCase", default)]
struct SteamGridDbStored {
    enabled: bool,
    api_key: Option<String>,
    auto_fill: bool,
    animated: bool,
}

/// Lê os três `.dat` de `root`. `decrypt` é o `CryptUnprotectData` do usuário atual (só existe no Windows);
/// devolve `None` quando o arquivo é de outro usuário ou PC.
pub fn read_secrets(root: &Path, decrypt: impl Fn(&[u8]) -> Option<Vec<u8>>) -> MigratedSecrets {
    let mut out = MigratedSecrets::default();
    let path = root.join(IGDB_FILE);
    if let Some(s) = read_provider::<IgdbStored>(&path, &decrypt) {
        out.igdb_enabled = Some(s.enabled);
        out.igdb_client_id = s.client_id.map(|v| v.trim().to_string());
        out.igdb_client_secret = s.client_secret.map(|v| v.trim().to_string());
        out.read_files.push(path);
    }
    let path = root.join(SCREENSCRAPER_FILE);
    if let Some(s) = read_provider::<ScreenScraperStored>(&path, &decrypt) {
        out.screenscraper_enabled = Some(s.enabled);
        out.screenscraper_user = s.user.map(|v| v.trim().to_string());
        out.screenscraper_password = s.password;
        out.read_files.push(path);
    }
    let path = root.join(STEAMGRIDDB_FILE);
    if let Some(s) = read_provider::<SteamGridDbStored>(&path, &decrypt) {
        out.steamgriddb_enabled = Some(s.enabled);
        out.steamgriddb_api_key = s.api_key.map(|v| v.trim().to_string());
        out.steamgriddb_auto_fill = Some(s.auto_fill);
        out.steamgriddb_animated = Some(s.animated);
        out.read_files.push(path);
    }
    out
}

/// Renomeia os `.dat` lidos para `.dat.migrado` (o cofre já tem os segredos).
pub fn mark_migrated(secrets: &MigratedSecrets) -> std::io::Result<()> {
    for path in &secrets.read_files {
        std::fs::rename(path, with_suffix(path, MIGRATED_SUFFIX))?;
    }
    Ok(())
}

/// Fluxo completo: lê os `.dat`, grava cada provedor lido no cofre do SO no formato que as Configurações e a
/// busca de metadados leem (`IgdbSettings::save` etc.) e só então renomeia os arquivos. Qualquer falha deixa os
/// `.dat` no lugar para tentar de novo.
pub fn migrate_secrets(
    root: &Path,
    decrypt: impl Fn(&[u8]) -> Option<Vec<u8>>,
    store: &dyn SecretStore,
) -> std::result::Result<MigratedSecrets, String> {
    let s = read_secrets(root, decrypt);
    let read = |name: &str| s.read_files.iter().any(|p| p.ends_with(name));
    if read(IGDB_FILE) {
        IgdbSettings {
            enabled: s.igdb_enabled.unwrap_or(false),
            client_id: s.igdb_client_id.clone().unwrap_or_default(),
            client_secret: s.igdb_client_secret.clone().unwrap_or_default(),
        }
        .save(store)?;
    }
    if read(SCREENSCRAPER_FILE) {
        ScreenScraperSettings {
            enabled: s.screenscraper_enabled.unwrap_or(false),
            user: s.screenscraper_user.clone().unwrap_or_default(),
            password: s.screenscraper_password.clone().unwrap_or_default(),
        }
        .save(store)?;
    }
    if read(STEAMGRIDDB_FILE) {
        SteamGridDbSettings {
            enabled: s.steamgriddb_enabled.unwrap_or(false),
            api_key: s.steamgriddb_api_key.clone().unwrap_or_default(),
            auto_fill: s.steamgriddb_auto_fill.unwrap_or(false),
            animated: s.steamgriddb_animated.unwrap_or(false),
        }
        .save(store)?;
    }
    let secrets = s;
    if secrets.read_files.is_empty() {
        return Ok(secrets);
    }
    mark_migrated(&secrets).map_err(|e| e.to_string())?;
    tracing::info!(
        files = secrets.read_files.len(),
        "segredos das alphas migrados para o cofre"
    );
    Ok(secrets)
}

fn read_provider<T: DeserializeOwned>(
    path: &Path,
    decrypt: &impl Fn(&[u8]) -> Option<Vec<u8>>,
) -> Option<T> {
    let text = read_small(path)?;
    let json = match text.strip_prefix(DPAPI_PREFIX) {
        Some(b64) => String::from_utf8(decrypt(&base64_decode(b64)?)?).ok()?,
        None => text,
    };
    let parsed = serde_json::from_str(json.trim_start_matches('\u{feff}'));
    if parsed.is_err() {
        tracing::warn!(path = %path.display(), "segredo da alpha ilegível; fica para o usuário informar de novo");
    }
    parsed.ok()
}

/// Texto pequeno, sem BOM e sem espaços nas pontas; vazio = nada.
fn read_small(path: &Path) -> Option<String> {
    if std::fs::metadata(path).ok()?.len() > MAX_SMALL_FILE {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    let text = text.trim_start_matches('\u{feff}').trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// Base64 padrão (`Convert.ToBase64String`). Sem crate para 15 linhas.
fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut buf, mut bits) = (0u32, 0u32);
    for c in s.trim().trim_end_matches('=').bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        buf = (buf << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::TakoDatabase;
    use std::collections::HashMap;

    /// "Cifra" de teste: inverte os bytes. Nada a ver com DPAPI, só prova que a decifragem é chamada.
    fn fake_decrypt(data: &[u8]) -> Option<Vec<u8>> {
        data.strip_prefix(b"OK:")
            .map(|d| d.iter().rev().copied().collect())
    }

    fn fake_protect(json: &str) -> String {
        let mut data = b"OK:".to_vec();
        data.extend(json.bytes().rev());
        format!("{DPAPI_PREFIX}{}", base64_encode(&data))
    }

    fn base64_encode(data: &[u8]) -> String {
        const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut s = String::new();
        for chunk in data.chunks(3) {
            let n = chunk
                .iter()
                .enumerate()
                .fold(0u32, |n, (i, b)| n | (u32::from(*b) << (16 - 8 * i)));
            for i in 0..4 {
                s.push(if i <= chunk.len() {
                    A[(n >> (18 - 6 * i) & 63) as usize] as char
                } else {
                    '='
                });
            }
        }
        s
    }

    #[test]
    fn base64_decodes_dotnet_output() {
        assert_eq!(base64_decode("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(base64_decode("aGk=").unwrap(), b"hi");
        assert_eq!(base64_decode("").unwrap(), b"");
        assert_eq!(base64_decode("não!"), None);
        let bytes: Vec<u8> = (0..=255).collect();
        assert_eq!(base64_decode(&base64_encode(&bytes)).unwrap(), bytes);
    }

    fn write_alpha_files(root: &Path) {
        // IGDB cifrado, ScreenScraper em JSON puro, SteamGridDB cifrado: como o C# grava.
        std::fs::write(
            root.join(IGDB_FILE),
            fake_protect(r#"{"Enabled":true,"ClientId":" abc123 ","ClientSecret":"s3cr3t"}"#),
        )
        .unwrap();
        std::fs::write(
            root.join(SCREENSCRAPER_FILE),
            "\u{feff}{\"Enabled\":false,\"User\":\"fulano\",\"Password\":\"senha\"}\r\n",
        )
        .unwrap();
        std::fs::write(
            root.join(STEAMGRIDDB_FILE),
            fake_protect(r#"{"Enabled":true,"ApiKey":"0123456789abcdef0123456789abcdef","AutoFill":true,"Animated":false}"#),
        )
        .unwrap();
    }

    #[test]
    fn reads_the_three_provider_files() {
        let dir = tempfile::tempdir().unwrap();
        write_alpha_files(dir.path());

        let s = read_secrets(dir.path(), fake_decrypt);

        assert_eq!(s.igdb_client_id.as_deref(), Some("abc123"));
        assert_eq!(s.igdb_client_secret.as_deref(), Some("s3cr3t"));
        assert_eq!(s.igdb_enabled, Some(true));
        assert_eq!(s.screenscraper_user.as_deref(), Some("fulano"));
        assert_eq!(s.screenscraper_password.as_deref(), Some("senha"));
        assert_eq!(s.screenscraper_enabled, Some(false));
        assert_eq!(
            s.steamgriddb_api_key.as_deref(),
            Some("0123456789abcdef0123456789abcdef")
        );
        assert_eq!(s.steamgriddb_auto_fill, Some(true));
        assert_eq!(s.steamgriddb_animated, Some(false));
        assert_eq!(s.read_files.len(), 3);
        let debug = format!("{s:?}");
        assert!(!debug.contains("s3cr3t") && !debug.contains("senha"));
    }

    #[test]
    fn unreadable_files_are_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // De outro usuário (a decifragem falha), base64 quebrado, JSON quebrado, vazio.
        std::fs::write(root.join(IGDB_FILE), format!("{DPAPI_PREFIX}AAAA")).unwrap();
        std::fs::write(root.join(SCREENSCRAPER_FILE), "dpapi:%%%").unwrap();
        std::fs::write(root.join(STEAMGRIDDB_FILE), "{ não é json").unwrap();

        let s = read_secrets(root, fake_decrypt);
        assert_eq!(s, MigratedSecrets::default());

        std::fs::write(root.join(STEAMGRIDDB_FILE), "  ").unwrap();
        assert!(read_secrets(root, fake_decrypt).read_files.is_empty());
        std::fs::write(root.join(STEAMGRIDDB_FILE), vec![b'x'; 70 * 1024]).unwrap();
        assert!(read_secrets(root, fake_decrypt).read_files.is_empty());
    }

    #[derive(Default)]
    struct TestVault(std::cell::RefCell<HashMap<String, String>>);
    impl SecretStore for TestVault {
        fn get(&self, key: &str) -> Option<String> {
            self.0.borrow().get(key).cloned()
        }
        fn set(&self, key: &str, value: &str) -> std::result::Result<(), String> {
            self.0.borrow_mut().insert(key.into(), value.into());
            Ok(())
        }
    }

    #[test]
    fn migration_stores_secrets_flags_and_renames_the_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_alpha_files(root);
        let vault = TestVault::default();

        let s = migrate_secrets(root, fake_decrypt, &vault).unwrap();

        assert_eq!(s.read_files.len(), 3);
        let igdb = IgdbSettings::load(&vault);
        assert!(igdb.enabled);
        assert_eq!(igdb.client_secret, "s3cr3t");
        let ss = ScreenScraperSettings::load(&vault);
        assert!(!ss.enabled);
        assert_eq!(ss.password, "senha");
        assert!(SteamGridDbSettings::load(&vault).auto_fill);
        for f in [IGDB_FILE, SCREENSCRAPER_FILE, STEAMGRIDDB_FILE] {
            assert!(!root.join(f).exists());
            assert!(root.join(format!("{f}{MIGRATED_SUFFIX}")).exists());
        }
        assert_eq!(AlphaData::detect(&DataPaths::at(root)), None);
    }

    #[test]
    fn failed_vault_write_keeps_the_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_alpha_files(root);
        struct Closed;
        impl SecretStore for Closed {
            fn get(&self, _: &str) -> Option<String> {
                None
            }
            fn set(&self, _: &str, _: &str) -> std::result::Result<(), String> {
                Err("cofre fechado".into())
            }
        }

        let r = migrate_secrets(root, fake_decrypt, &Closed);

        assert!(r.is_err());
        assert!(root.join(IGDB_FILE).exists());
    }

    #[test]
    fn update_preferences_follow_the_csharp_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("updates.json");
        assert_eq!(UpdatePreferences::read(&path), UpdatePreferences::default());
        std::fs::write(
            &path,
            "{\n  \"AutoUpdate\": false,\n  \"Prereleases\": true\n}",
        )
        .unwrap();
        assert_eq!(
            UpdatePreferences::read(&path),
            UpdatePreferences {
                auto_update: false,
                prereleases: true
            }
        );
        std::fs::write(&path, "{\"Prereleases\": true}").unwrap();
        assert!(UpdatePreferences::read(&path).auto_update);
        std::fs::write(&path, "lixo").unwrap();
        assert_eq!(UpdatePreferences::read(&path), UpdatePreferences::default());
    }

    #[test]
    fn detects_alpha_data_and_backs_up_the_database_once() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::at(dir.path());
        assert_eq!(AlphaData::detect(&paths), None);
        {
            let conn = Connection::open(paths.database()).unwrap();
            migrator::migrate(&conn, Some(migrator::LAST_ALPHA_VERSION)).unwrap();
            conn.execute_batch("INSERT INTO settings VALUES ('DynamicColor', 'false');")
                .unwrap();
        }
        let alpha = AlphaData::detect(&paths).unwrap();
        assert!(alpha.database);
        assert!(alpha.secret_files.is_empty());

        TakoDatabase::new(paths.database()).open().unwrap();

        let backup = with_suffix(&paths.database(), ALPHA_BACKUP_SUFFIX);
        let conn = Connection::open(&backup).unwrap();
        assert_eq!(migrator::current_version(&conn).unwrap(), 6);
        let v: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'DynamicColor';",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(v, "false");
        // Já na v7: nada mais a fazer.
        assert_eq!(AlphaData::detect(&paths), None);
    }
}
