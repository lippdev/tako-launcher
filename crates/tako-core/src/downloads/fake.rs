//! Motor de download em memória para os testes: nada sai da máquina.

use super::{DownloadError, Downloader, EngineId, EngineStatus};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct State {
    next: EngineId,
    started: Vec<(String, PathBuf)>,
    resumes: Vec<bool>,
    status: HashMap<EngineId, EngineStatus>,
    paused: HashMap<EngineId, bool>,
    removed: Vec<(EngineId, bool)>,
}

/// Clones dividem o mesmo estado: o teste guarda um e entrega o outro ao [`super::Downloads`].
#[derive(Clone, Default)]
pub struct MemoryDownloader(Arc<Mutex<State>>);

impl MemoryDownloader {
    fn with<R>(&self, f: impl FnOnce(&mut State) -> R) -> R {
        f(&mut self.0.lock().unwrap_or_else(|e| e.into_inner()))
    }
    pub fn set(&self, id: EngineId, done: u64, total: Option<u64>, finished: bool) {
        self.with(|s| {
            s.status.insert(
                id,
                EngineStatus {
                    done,
                    total,
                    down_bps: 1000,
                    finished,
                    error: None,
                },
            )
        });
    }
    pub fn fail(&self, id: EngineId, error: DownloadError) {
        self.with(|s| {
            s.status.insert(
                id,
                EngineStatus {
                    error: Some(error),
                    ..Default::default()
                },
            )
        });
    }
    pub fn started(&self) -> Vec<(String, PathBuf)> {
        self.with(|s| s.started.clone())
    }
    /// O `resume` de cada `start`, em ordem.
    pub fn resumes(&self) -> Vec<bool> {
        self.with(|s| s.resumes.clone())
    }
    pub fn removed(&self) -> Vec<(EngineId, bool)> {
        self.with(|s| s.removed.clone())
    }
    pub fn paused(&self, id: EngineId) -> bool {
        self.with(|s| s.paused.get(&id).copied().unwrap_or(false))
    }
}

impl Downloader for MemoryDownloader {
    fn start(&mut self, uri: &str, folder: &Path, resume: bool) -> Result<EngineId, DownloadError> {
        Ok(self.with(|s| {
            s.next += 1;
            s.resumes.push(resume);
            s.started.push((uri.to_owned(), folder.to_owned()));
            s.status.insert(s.next, EngineStatus::default());
            s.next
        }))
    }
    fn pause(&mut self, id: EngineId) {
        self.with(|s| s.paused.insert(id, true));
    }
    fn resume(&mut self, id: EngineId) {
        self.with(|s| s.paused.insert(id, false));
    }
    fn remove(&mut self, id: EngineId, delete_files: bool) {
        self.with(|s| {
            s.status.remove(&id);
            s.removed.push((id, delete_files));
        });
    }
    fn status(&self, id: EngineId) -> Option<EngineStatus> {
        self.with(|s| s.status.get(&id).cloned())
    }
}
