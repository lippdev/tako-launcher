//! Persistência no `tako.db` (port de `Launcher.Core/Persistence`, MIG-011) e migração das alphas C# (MIG-030).
//!
//! O esquema é o mesmo das alphas (v1..v6) mais as versões novas do Rust: um banco da alpha abre sem mudanças.
//! Datas são texto ISO-8601 no formato "O" do .NET (`2026-10-15T12:00:00.0000000+00:00`).

pub mod alpha_migration;
mod choices;
mod collections;
mod database;
mod game_settings;
mod library;
pub mod migrator;
mod settings;

#[cfg(test)]
mod alpha_compat_tests;

pub use choices::GameChoicesStore;
pub use collections::{CollectionRow, CollectionStore};
pub use database::TakoDatabase;
pub use game_settings::GameSettingsStore;
pub use library::{GameStateRow, IMPORTED_SUFFIX, LibraryData, LibraryStore, ManualGameRow};
pub use settings::SettingsStore;

use chrono::{DateTime, NaiveDateTime, Utc};

#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("banco de dados: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("arquivo: {0}")]
    Io(#[from] std::io::Error),
    #[error("argumento vazio: {0}")]
    EmptyArgument(&'static str),
}

pub type Result<T> = std::result::Result<T, PersistenceError>;

fn require(value: &str, name: &'static str) -> Result<()> {
    if value.is_empty() {
        Err(PersistenceError::EmptyArgument(name))
    } else {
        Ok(())
    }
}

/// Formato "O" do .NET para `DateTimeOffset` em UTC: 7 casas e `+00:00`.
pub(crate) fn format_date(value: DateTime<Utc>) -> String {
    // chrono não tem `%.7f`: os ticks de 100 ns vão à mão.
    format!(
        "{}.{:07}+00:00",
        value.format("%Y-%m-%dT%H:%M:%S"),
        value.timestamp_subsec_nanos() / 100
    )
}

/// Lê o que o C# gravou (`+00:00`, `Z`, `-03:00`, de 0 a 7 casas) e o que não tem fuso (UTC).
pub(crate) fn parse_date(value: &str) -> Option<DateTime<Utc>> {
    let value = value.trim();
    DateTime::parse_from_rfc3339(value)
        .map(|d| d.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f")
                .ok()
                .map(|n| n.and_utc())
        })
}

/// Data obrigatória ilegível não derruba a leitura nem apaga a linha: vira a época Unix.
pub(crate) fn parse_date_or_epoch(value: &str) -> DateTime<Utc> {
    parse_date(value).unwrap_or(DateTime::UNIX_EPOCH)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn dates_use_the_dotnet_round_trip_format() {
        let d = Utc.with_ymd_and_hms(2026, 10, 15, 15, 0, 0).unwrap();
        assert_eq!(format_date(d), "2026-10-15T15:00:00.0000000+00:00");
        for text in [
            "2026-10-15T15:00:00.0000000+00:00",
            "2026-10-15T15:00:00Z",
            "2026-10-15T12:00:00-03:00",
            "2026-10-15T12:00:00.0000000-03:00",
            "2026-10-15T15:00:00",
        ] {
            assert_eq!(parse_date(text), Some(d), "{text}");
        }
        assert_eq!(parse_date("ontem"), None);
        assert_eq!(parse_date_or_epoch(""), DateTime::UNIX_EPOCH);
    }
}
