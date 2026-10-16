//! Port de `ScreenScraper/RomHasher.cs`: identidade da ROM para o `jeuInfos.php`, em streaming:
//! até 64 MB CRC32+MD5+SHA1; acima, só CRC32; discos compactados e `.7z`, só nome e tamanho; `.zip`
//! com um arquivo só, o de dentro (exceto arcade, em que o `.zip` é a ROM). Cache por caminho,
//! tamanho e data em `rom-hashes.json`.

use super::screenscraper_parse::{COMPRESSED_DISC_EXTENSIONS, EmulatedSystem, extension_of};
use md5::Md5;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::fs::{File, Metadata};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const FULL_HASH_LIMIT: u64 = 64 * 1024 * 1024;
const BUFFER: usize = 1024 * 1024;

/// Hashes em hexadecimal maiúsculo, como o ScreenScraper usa; `None` quando não calculados.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct RomIdentity {
    pub file_name: String,
    pub size: u64,
    pub crc32: Option<String>,
    pub md5: Option<String>,
    pub sha1: Option<String>,
}

fn upper_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

/// Hash de um fluxo (puro; testado com bytes em memória).
pub fn hash(
    mut reader: impl Read,
    file_name: &str,
    size: u64,
    full: bool,
) -> io::Result<RomIdentity> {
    let mut crc = crc32fast::Hasher::new();
    let mut md5 = full.then(Md5::new);
    let mut sha1 = full.then(Sha1::new);
    let mut buf = vec![0u8; BUFFER];
    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        let chunk = &buf[..n];
        crc.update(chunk);
        if let Some(h) = md5.as_mut() {
            h.update(chunk);
        }
        if let Some(h) = sha1.as_mut() {
            h.update(chunk);
        }
    }
    Ok(RomIdentity {
        file_name: file_name.into(),
        size,
        crc32: Some(format!("{:08X}", crc.finalize())),
        md5: md5.map(|h| upper_hex(&h.finalize())),
        sha1: sha1.map(|h| upper_hex(&h.finalize())),
    })
}

/// Identidade da ROM no disco. Bloqueante: chamar fora da thread da UI.
pub fn identify(path: &Path, system: Option<&EmulatedSystem>) -> io::Result<RomIdentity> {
    let meta = std::fs::metadata(path)?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = extension_of(&name);
    if COMPRESSED_DISC_EXTENSIONS.contains(&ext.as_str()) || ext == ".7z" {
        return Ok(RomIdentity {
            file_name: name,
            size: meta.len(),
            ..Default::default()
        });
    }
    if ext == ".zip"
        && !system.is_some_and(|s| s.archive_is_rom)
        && let Some(inner) = identify_inside(path)
    {
        return Ok(inner);
    }
    let file = File::open(path)?;
    hash(file, &name, meta.len(), meta.len() <= FULL_HASH_LIMIT)
}

/// `.zip` com um arquivo só: hashes dele (é o que os dats registram). Vários ou corrompido: `None`.
fn identify_inside(path: &Path) -> Option<RomIdentity> {
    let mut archive = zip::ZipArchive::new(File::open(path).ok()?).ok()?;
    let files: Vec<usize> = (0..archive.len())
        .filter(|&i| {
            archive
                .by_index_raw(i)
                .is_ok_and(|e| e.is_file() && e.size() > 0)
        })
        .collect();
    let [only] = files.as_slice() else {
        return None;
    };
    let entry = archive.by_index(*only).ok()?;
    let size = entry.size();
    let name = entry
        .name()
        .ok()
        .and_then(|n| {
            Path::new(n.as_ref())
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
        })
        .unwrap_or_default();
    // `take`: um .zip mentiroso não faz ler além do tamanho declarado.
    hash(entry.take(size), &name, size, size <= FULL_HASH_LIMIT).ok()
}

/// Cache das identidades por caminho + tamanho + data: uma ISO de 4 GB não é lida de novo.
pub struct RomHashCache {
    path: Option<PathBuf>,
    entries: Mutex<Option<HashMap<String, RomIdentity>>>,
}

impl RomHashCache {
    /// `path` = `cache/rom-hashes.json`; `None` = só na memória.
    pub fn new(path: Option<PathBuf>) -> Self {
        RomHashCache {
            path,
            entries: Mutex::new(None),
        }
    }

    /// Chave: caminho, tamanho, data e se o .zip é a ROM (muda o que é calculado).
    pub fn key_for(path: &Path, meta: &Metadata, system: Option<&EmulatedSystem>) -> String {
        let full = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        let modified = meta
            .modified()
            .ok()
            .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos());
        let kind = if system.is_some_and(|s| s.archive_is_rom) {
            "a"
        } else {
            "r"
        };
        format!(
            "{}|{}|{modified}|{kind}",
            full.to_string_lossy().to_uppercase(),
            meta.len()
        )
    }

    pub fn get(&self, rom: &Path, system: Option<&EmulatedSystem>) -> io::Result<RomIdentity> {
        let key = Self::key_for(rom, &std::fs::metadata(rom)?, system);
        if let Some(known) = self.lock().get_or_insert_with(|| self.load()).get(&key) {
            return Ok(known.clone());
        }
        let identity = identify(rom, system)?;
        let mut guard = self.lock();
        let entries = guard.get_or_insert_with(|| self.load());
        entries.insert(key, identity.clone());
        self.save(entries);
        Ok(identity)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<HashMap<String, RomIdentity>>> {
        self.entries.lock().unwrap_or_else(|e| e.into_inner())
    }

    // Ilegível ou grande demais: as ROMs são lidas de novo.
    fn load(&self) -> HashMap<String, RomIdentity> {
        self.path
            .as_ref()
            .filter(|p| std::fs::metadata(p).is_ok_and(|m| m.len() < 32 * 1024 * 1024))
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    fn save(&self, entries: &HashMap<String, RomIdentity>) {
        let Some(path) = &self.path else { return };
        let Ok(json) = serde_json::to_vec(entries) else {
            return;
        };
        // Sem cache em disco, o hash é recalculado na próxima abertura.
        let _ = super::cache::write_atomic(path, &json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::screenscraper_parse::system_by_key;
    use std::io::Write;

    #[test]
    fn hashes_match_the_standard_test_vectors() {
        let data = b"123456789";
        let full = hash(&data[..], "a.sfc", 9, true).unwrap();
        assert_eq!(
            full,
            RomIdentity {
                file_name: "a.sfc".into(),
                size: 9,
                crc32: Some("CBF43926".into()),
                md5: Some("25F9E794323B453885F5181F1B624D0B".into()),
                sha1: Some("F7C3BC1D808E04732ADF679965CCC34CA7AE3441".into()),
            }
        );
        let crc_only = hash(&data[..], "a.iso", 9, false).unwrap();
        assert_eq!(crc_only.crc32.as_deref(), Some("CBF43926"));
        assert_eq!((crc_only.md5, crc_only.sha1), (None, None));
    }

    #[test]
    fn zip_hashes_the_inner_rom_except_for_arcade_sets() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("lanterna.zip");
        {
            let mut w = zip::ZipWriter::new(File::create(&zip_path).unwrap());
            w.start_file(
                "Lanterna (USA).sfc",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            w.write_all(b"123456789").unwrap();
            w.finish().unwrap();
        }
        let inner = identify(&zip_path, system_by_key(Some("snes"))).unwrap();
        assert_eq!(inner.file_name, "Lanterna (USA).sfc");
        assert_eq!(inner.size, 9);
        assert_eq!(inner.crc32.as_deref(), Some("CBF43926"));

        let set = identify(&zip_path, system_by_key(Some("arcade"))).unwrap();
        assert_eq!(set.file_name, "lanterna.zip");
        assert_eq!(set.size, std::fs::metadata(&zip_path).unwrap().len());
        assert_ne!(set.crc32.as_deref(), Some("CBF43926"));
        assert!(set.sha1.is_some());

        let chd = dir.path().join("lanterna.chd");
        std::fs::write(&chd, "não importa").unwrap();
        assert_eq!(
            identify(&chd, system_by_key(Some("ps1"))).unwrap(),
            RomIdentity {
                file_name: "lanterna.chd".into(),
                size: 12,
                ..Default::default()
            }
        );

        // .zip corrompido: hash do arquivo como está.
        let broken = dir.path().join("quebrado.zip");
        std::fs::write(&broken, "PK\x03\x04lixo").unwrap();
        assert_eq!(
            identify(&broken, system_by_key(Some("snes")))
                .unwrap()
                .file_name,
            "quebrado.zip"
        );
    }

    #[test]
    fn hash_cache_survives_restarts_and_notices_changed_files() {
        let dir = tempfile::tempdir().unwrap();
        let rom = dir.path().join("lanterna.sfc");
        std::fs::write(&rom, "123456789").unwrap();
        let cache_file = dir.path().join("cache").join("rom-hashes.json");
        let snes = system_by_key(Some("snes"));

        let first = RomHashCache::new(Some(cache_file.clone()))
            .get(&rom, snes)
            .unwrap();
        assert_eq!(first.crc32.as_deref(), Some("CBF43926"));
        assert!(cache_file.exists());
        let key = RomHashCache::key_for(&rom, &std::fs::metadata(&rom).unwrap(), snes);
        assert!(
            std::fs::read_to_string(&cache_file)
                .unwrap()
                .contains(&serde_json::to_string(&key).unwrap())
        );
        assert_eq!(
            RomHashCache::new(Some(cache_file.clone()))
                .get(&rom, snes)
                .unwrap(),
            first
        );

        std::fs::write(&rom, "1234567890").unwrap();
        let f = File::options().write(true).open(&rom).unwrap();
        f.set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(60))
            .unwrap();
        let changed = RomHashCache::new(Some(cache_file)).get(&rom, snes).unwrap();
        assert_eq!(changed.size, 10);
        assert_ne!(changed.crc32, first.crc32);
    }
}
