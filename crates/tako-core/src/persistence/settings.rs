//! Port de `SettingsStore.cs`: preferências como pares chave/valor em texto invariante, como o C# grava
//! (`"true"`/`"false"`, números sem separador de milhar).

use super::{Result, TakoDatabase, require};
use rusqlite::OptionalExtension;
use std::collections::HashMap;

#[derive(Clone)]
pub struct SettingsStore {
    db: TakoDatabase,
}

impl SettingsStore {
    pub fn new(db: TakoDatabase) -> SettingsStore {
        SettingsStore { db }
    }

    pub fn load_all(&self) -> Result<HashMap<String, String>> {
        let conn = self.db.open()?;
        let mut stmt = conn.prepare("SELECT key, value FROM settings;")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn get(&self, key: &str) -> Result<Option<String>> {
        let conn = self.db.open()?;
        Ok(conn
            .query_row("SELECT value FROM settings WHERE key = ?1;", [key], |r| {
                r.get(0)
            })
            .optional()?)
    }

    pub fn set(&self, key: &str, value: &str) -> Result<()> {
        require(key, "key")?;
        self.db.open()?.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value;",
            [key, value],
        )?;
        Ok(())
    }

    /// `None` quando ausente ou ilegível (quem chama usa o padrão).
    pub fn get_bool(&self, key: &str) -> Result<Option<bool>> {
        Ok(self.get(key)?.as_deref().and_then(parse_bool))
    }

    pub fn set_bool(&self, key: &str, value: bool) -> Result<()> {
        self.set(key, if value { "true" } else { "false" })
    }

    pub fn get_i64(&self, key: &str) -> Result<Option<i64>> {
        Ok(self.get(key)?.and_then(|v| v.trim().parse().ok()))
    }

    pub fn set_i64(&self, key: &str, value: i64) -> Result<()> {
        self.set(key, &value.to_string())
    }
}

/// Como `bool.Parse` do .NET: sem diferença de caixa.
pub fn parse_bool(value: &str) -> Option<bool> {
    let v = value.trim();
    if v.eq_ignore_ascii_case("true") {
        Some(true)
    } else if v.eq_ignore_ascii_case("false") {
        Some(false)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_store_upserts_and_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tako.db");
        let settings = SettingsStore::new(TakoDatabase::new(&path));
        assert!(settings.load_all().unwrap().is_empty());

        settings.set("DynamicColor", "false").unwrap();
        settings.set("DynamicColor", "true").unwrap();
        settings.set("ReducedMotion", "true").unwrap();

        let reopened = SettingsStore::new(TakoDatabase::new(&path))
            .load_all()
            .unwrap();
        assert_eq!(reopened["DynamicColor"], "true");
        assert_eq!(reopened["ReducedMotion"], "true");
        assert_eq!(reopened.len(), 2);
    }

    #[test]
    fn typed_helpers_use_the_csharp_text() {
        let dir = tempfile::tempdir().unwrap();
        let s = SettingsStore::new(TakoDatabase::new(dir.path().join("tako.db")));
        s.set_bool("GameScreenEnabled", false).unwrap();
        s.set_i64("ControlCenterChord", 12288).unwrap();
        s.set("Velho", "True").unwrap();
        s.set("Ruim", "talvez").unwrap();
        assert_eq!(
            s.get("GameScreenEnabled").unwrap().as_deref(),
            Some("false")
        );
        assert_eq!(s.get_bool("GameScreenEnabled").unwrap(), Some(false));
        assert_eq!(s.get_bool("Velho").unwrap(), Some(true));
        assert_eq!(s.get_bool("Ruim").unwrap(), None);
        assert_eq!(s.get_bool("Ausente").unwrap(), None);
        assert_eq!(s.get_i64("ControlCenterChord").unwrap(), Some(12288));
        assert!(s.set("", "x").is_err());
    }
}
