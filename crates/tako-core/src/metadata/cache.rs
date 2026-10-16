//! Cache de metadados por jogo (`cache/metadata/<id>.json`), port da parte de arquivos de
//! `MetadataService.cs`. Arquivo ilegível conta como "sem cache".

use super::GameMetadata;
use std::io;
use std::path::{Path, PathBuf};

/// Cache de um jogo acima disso é ignorado (consulta de novo).
const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;

pub struct MetadataCache {
    dir: PathBuf,
}

impl MetadataCache {
    /// `dir` = `DataPaths::metadata_cache()`.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        MetadataCache { dir: dir.into() }
    }

    /// Nome do arquivo: o id com os caracteres proibidos em nomes de arquivo trocados por `_`.
    pub fn path_for(&self, game_id: &str) -> PathBuf {
        let name: String = game_id
            .chars()
            .map(|c| {
                if c.is_control() || "<>:\"/\\|?*".contains(c) {
                    '_'
                } else {
                    c
                }
            })
            .collect();
        self.dir.join(format!("{name}.json"))
    }

    pub fn read(&self, game_id: &str) -> Option<GameMetadata> {
        let path = self.path_for(game_id);
        if std::fs::metadata(&path).ok()?.len() > MAX_FILE_BYTES {
            return None;
        }
        serde_json::from_slice(&std::fs::read(path).ok()?).ok()
    }

    pub fn write(&self, game_id: &str, metadata: &GameMetadata) -> io::Result<()> {
        let json = serde_json::to_vec_pretty(metadata).map_err(io::Error::other)?;
        write_atomic(&self.path_for(game_id), &json)
    }
}

/// Grava num `.tmp` e renomeia: queda no meio não deixa arquivo pela metade.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_survives_broken_files() {
        let dir = tempfile::tempdir().unwrap();
        let cache = MetadataCache::new(dir.path());
        assert!(cache.read("steam:570").is_none());
        let m = GameMetadata {
            steam_app_id: Some(570),
            ..GameMetadata::new("Steam")
        };
        cache.write("steam:570", &m).unwrap();
        assert_eq!(cache.read("steam:570"), Some(m));
        assert!(
            cache
                .path_for("heroic:legendary:Fun/Game?")
                .ends_with("heroic_legendary_Fun_Game_.json")
        );
        std::fs::write(cache.path_for("x"), "{ quebrado").unwrap();
        assert!(cache.read("x").is_none());
    }
}
