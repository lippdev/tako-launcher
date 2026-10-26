//! Quando o jogo chegou ao PC (port de `InstallDates`): data de criação da pasta.

use chrono::{DateTime, Utc};
use std::path::Path;

/// Criação da pasta (ou do arquivo); `None` se não existe ou o sistema não informa.
pub fn of(path: impl AsRef<Path>) -> Option<DateTime<Utc>> {
    let meta = std::fs::metadata(path).ok()?;
    meta.created()
        .or_else(|_| meta.modified())
        .ok()
        .map(DateTime::<Utc>::from)
}
