//! Local SQLite storage. Records are stored as JSON (the same shape as the export) with a few
//! indexed columns; the volume of a single person's writing is small enough to aggregate in memory.

use std::path::Path;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Serialize, de::DeserializeOwned};

use crate::error::{Error, Result};
use crate::model::Entry;
use crate::settings::Settings;
use crate::tone::{Tone, builtin_tones};

const MIGRATIONS: &[&str] = &[r#"
    CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
    CREATE TABLE tones (id TEXT PRIMARY KEY, position INTEGER NOT NULL, json TEXT NOT NULL);
    CREATE TABLE entries (
        id TEXT PRIMARY KEY,
        created_at TEXT NOT NULL,
        language TEXT NOT NULL,
        json TEXT NOT NULL
    );
    CREATE INDEX entries_created_at ON entries (created_at DESC);
"#];

pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    pub fn open(path: &Path) -> Result<Store> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Store::init(Connection::open(path)?)
    }

    pub fn in_memory() -> Result<Store> {
        Store::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Store> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", (i + 1) as i64)?;
            tx.commit()?;
        }
        let store = Store { conn: Mutex::new(conn) };
        store.seed_tones()?;
        Ok(store)
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|p| p.into_inner())
    }

    // ---- key/value -----------------------------------------------------------------------

    fn get_meta_raw(&self, key: &str) -> Result<Option<String>> {
        Ok(self.conn().query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0)).optional()?)
    }

    /// A stored value, or an error if it exists but can't be read (never silently `None`, so a
    /// later save can't overwrite data we failed to parse).
    pub fn get_meta<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        match self.get_meta_raw(key)? {
            Some(v) => Ok(Some(serde_json::from_str(&v)?)),
            None => Ok(None),
        }
    }

    pub fn set_meta<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        self.conn().execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, serde_json::to_string(value)?],
        )?;
        Ok(())
    }

    pub fn settings(&self) -> Result<Settings> {
        match self.get_meta("settings") {
            Ok(s) => Ok(s.unwrap_or_default()),
            Err(e) => {
                // Keep the unreadable copy before anything can overwrite it, then start fresh.
                tracing::error!(error = %e, "settings unreadable; backing them up and using defaults");
                if let Some(raw) = self.get_meta_raw("settings")? {
                    self.conn().execute(
                        "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
                        params![format!("settings.unreadable.{}", chrono::Utc::now().timestamp()), raw],
                    )?;
                }
                Ok(Settings::default())
            }
        }
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<()> {
        settings.validate().map_err(Error::Invalid)?;
        self.set_meta("settings", settings)?;
        self.bump_version()
    }

    /// Bumped on every change that affects stats (entries, settings); cached stats compare it.
    pub fn data_version(&self) -> Result<u64> {
        Ok(self.get_meta::<u64>("data_version")?.unwrap_or(0))
    }

    fn bump_version(&self) -> Result<()> {
        self.conn().execute(
            "INSERT INTO meta (key, value) VALUES ('data_version', '1')
             ON CONFLICT(key) DO UPDATE SET value = CAST(value AS INTEGER) + 1",
            [],
        )?;
        Ok(())
    }

    // ---- tones ---------------------------------------------------------------------------

    fn seed_tones(&self) -> Result<()> {
        if self.get_meta::<bool>("tones_seeded")?.unwrap_or(false) {
            return Ok(());
        }
        for tone in builtin_tones() {
            self.save_tone(&tone)?;
        }
        self.set_meta("tones_seeded", &true)
    }

    pub fn tones(&self) -> Result<Vec<Tone>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT json FROM tones ORDER BY position, rowid")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        let mut tones = Vec::new();
        for json in rows {
            if let Ok(t) = serde_json::from_str(&json?) {
                tones.push(t);
            }
        }
        Ok(tones)
    }

    pub fn tone(&self, id: &str) -> Result<Option<Tone>> {
        Ok(self.tones()?.into_iter().find(|t| t.id == id))
    }

    pub fn save_tone(&self, tone: &Tone) -> Result<()> {
        if tone.name.trim().is_empty() {
            return Err(Error::Invalid("Give the tone a name.".into()));
        }
        let conn = self.conn();
        let position: i64 = conn
            .query_row("SELECT position FROM tones WHERE id = ?1", [&tone.id], |r| r.get(0))
            .optional()?
            .map_or_else(|| conn.query_row("SELECT COALESCE(MAX(position) + 1, 0) FROM tones", [], |r| r.get(0)), Ok)?;
        conn.execute(
            "INSERT INTO tones (id, position, json) VALUES (?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET json = excluded.json",
            params![tone.id, position, serde_json::to_string(tone)?],
        )?;
        Ok(())
    }

    pub fn delete_tone(&self, id: &str) -> Result<()> {
        if self.tones()?.len() <= 1 {
            return Err(Error::Invalid("Keep at least one tone.".into()));
        }
        self.conn().execute("DELETE FROM tones WHERE id = ?1", [id])?;
        Ok(())
    }

    // ---- entries -------------------------------------------------------------------------

    pub fn insert_entry(&self, entry: &Entry) -> Result<()> {
        self.conn().execute(
            "INSERT INTO entries (id, created_at, language, json) VALUES (?1, ?2, ?3, ?4)",
            params![entry.id, entry.created_at.to_rfc3339(), entry.language, serde_json::to_string(entry)?],
        )?;
        self.bump_version()
    }

    pub fn entry(&self, id: &str) -> Result<Entry> {
        let json: Option<String> =
            self.conn().query_row("SELECT json FROM entries WHERE id = ?1", [id], |r| r.get(0)).optional()?;
        let json = json.ok_or_else(|| Error::NotFound("entry".into()))?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn delete_entry(&self, id: &str) -> Result<()> {
        self.conn().execute("DELETE FROM entries WHERE id = ?1", [id])?;
        self.bump_version()
    }

    /// Newest first. `before`: only entries created before this time (for paging).
    pub fn entries(&self, before: Option<DateTime<Utc>>, limit: usize) -> Result<Vec<Entry>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT json FROM entries WHERE (?1 IS NULL OR created_at < ?1) ORDER BY created_at DESC, id DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![before.map(|b| b.to_rfc3339()), limit as i64], |r| r.get::<_, String>(0))?;
        let mut out = Vec::new();
        for json in rows {
            match serde_json::from_str(&json?) {
                Ok(e) => out.push(e),
                Err(e) => tracing::warn!(error = %e, "skipping unreadable entry"),
            }
        }
        Ok(out)
    }

    /// Every entry, oldest first (for stats and export).
    pub fn all_entries(&self) -> Result<Vec<Entry>> {
        let mut all = self.entries(None, usize::MAX >> 1)?;
        all.reverse();
        Ok(all)
    }

    pub fn entry_count(&self) -> Result<u64> {
        Ok(self.conn().query_row("SELECT COUNT(*) FROM entries", [], |r| r.get::<_, i64>(0))? as u64)
    }

    pub fn clear_history(&self) -> Result<()> {
        self.conn().execute("DELETE FROM entries", [])?;
        self.bump_version()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeds_tones_once_and_keeps_order() {
        let store = Store::in_memory().unwrap();
        let ids: Vec<String> = store.tones().unwrap().into_iter().map(|t| t.id).collect();
        assert_eq!(ids, ["neutral", "slack", "github", "email", "friends"]);
        store.delete_tone("friends").unwrap();
        store.seed_tones().unwrap();
        assert_eq!(store.tones().unwrap().len(), 4);
    }

    #[test]
    fn unreadable_settings_are_backed_up_not_lost() {
        let store = Store::in_memory().unwrap();
        store.conn().execute("INSERT INTO meta (key, value) VALUES ('settings', '{\"broken\": true')", []).unwrap();
        assert_eq!(store.settings().unwrap(), Settings::default());
        let backups: i64 = store
            .conn()
            .query_row("SELECT COUNT(*) FROM meta WHERE key LIKE 'settings.unreadable.%'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(backups, 1);
    }

    #[test]
    fn data_version_moves_on_changes() {
        let store = Store::in_memory().unwrap();
        let v0 = store.data_version().unwrap();
        store.save_settings(&Settings::default()).unwrap();
        let v1 = store.data_version().unwrap();
        store.delete_entry("nope").unwrap();
        assert!(v1 > v0 && store.data_version().unwrap() > v1);
    }

    #[test]
    fn settings_round_trip_and_validation() {
        let store = Store::in_memory().unwrap();
        let mut s = store.settings().unwrap();
        s.providers[0].model = "openai/gpt-x".into();
        store.save_settings(&s).unwrap();
        assert_eq!(store.settings().unwrap().providers[0].model, "openai/gpt-x");
        s.languages.clear();
        assert!(store.save_settings(&s).is_err());
    }
}
