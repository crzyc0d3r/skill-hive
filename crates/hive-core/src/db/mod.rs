//! SQLite persistence. One connection behind a mutex: every call is short, and long work
//! (cloning, running evaluations) happens outside the lock.

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::{Connection, OptionalExtension, Transaction, params};

use crate::error::{HiveError, Result};
use crate::model::ProjectRef;

pub mod deployments;
pub mod items;
pub mod loadouts;
pub mod providers;
pub mod runs;

const SCHEMA: &str = include_str!("schema.sql");
pub const SCHEMA_VERSION: i64 = 1;

pub struct Database {
    conn: Mutex<Connection>,
}

impl std::fmt::Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Database")
    }
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| HiveError::io(parent, e))?;
        }
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    pub fn open_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "temp_store", "MEMORY")?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.lock();
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version < 1 {
            conn.execute_batch(SCHEMA)?;
            conn.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION};"))?;
        }
        // Future migrations go here, keyed on `version`.
        Ok(())
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, Connection> {
        self.conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Run `f` with the connection.
    pub fn with<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let conn = self.lock();
        f(&conn)
    }

    /// Run `f` inside a transaction; commits on `Ok`, rolls back on `Err`.
    pub fn transaction<T>(&self, f: impl FnOnce(&Transaction) -> Result<T>) -> Result<T> {
        let mut conn = self.lock();
        let tx = conn.transaction()?;
        let out = f(&tx)?;
        tx.commit()?;
        Ok(out)
    }

    // ---- settings (JSON key/value) -------------------------------------------------------

    pub fn get_setting<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let conn = self.lock();
        let raw: Option<String> = conn
            .query_row(
                "SELECT value_json FROM settings WHERE key = ?1",
                params![key],
                |r| r.get(0),
            )
            .optional()?;
        match raw {
            Some(text) => Ok(Some(serde_json::from_str(&text)?)),
            None => Ok(None),
        }
    }

    pub fn set_setting<T: serde::Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let text = serde_json::to_string(value)?;
        let conn = self.lock();
        conn.execute(
            "INSERT INTO settings(key, value_json) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json",
            params![key, text],
        )?;
        Ok(())
    }

    pub fn delete_setting(&self, key: &str) -> Result<()> {
        let conn = self.lock();
        conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
        Ok(())
    }

    // ---- cache ----------------------------------------------------------------------------

    pub fn cache_get<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
        max_age_ms: i64,
    ) -> Result<Option<T>> {
        let conn = self.lock();
        let row: Option<(String, i64)> = conn
            .query_row(
                "SELECT value_json, fetched_at FROM cache WHERE key = ?1",
                params![key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        match row {
            Some((text, fetched_at)) if crate::util::now_ms() - fetched_at <= max_age_ms => {
                Ok(Some(serde_json::from_str(&text)?))
            }
            _ => Ok(None),
        }
    }

    pub fn cache_put<T: serde::Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let text = serde_json::to_string(value)?;
        let conn = self.lock();
        conn.execute(
            "INSERT INTO cache(key, value_json, fetched_at) VALUES (?1, ?2, ?3) ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, fetched_at = excluded.fetched_at",
            params![key, text, crate::util::now_ms()],
        )?;
        Ok(())
    }

    // ---- audit ----------------------------------------------------------------------------

    pub fn audit(&self, action: &str, detail: &serde_json::Value) -> Result<()> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO audit(ts, action, detail_json) VALUES (?1, ?2, ?3)",
            params![crate::util::now_ms(), action, detail.to_string()],
        )?;
        conn.execute(
            "DELETE FROM audit WHERE id NOT IN (SELECT id FROM audit ORDER BY id DESC LIMIT 5000)",
            [],
        )?;
        Ok(())
    }

    // ---- projects -------------------------------------------------------------------------

    pub fn list_projects(&self) -> Result<Vec<ProjectRef>> {
        let conn = self.lock();
        let mut stmt = conn.prepare("SELECT path, name, added_at FROM projects ORDER BY name")?;
        let rows = stmt.query_map([], |r| {
            Ok(ProjectRef {
                path: r.get(0)?,
                name: r.get(1)?,
                added_at: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn add_project(&self, path: &str, name: &str) -> Result<ProjectRef> {
        let now = crate::util::now_ms();
        let conn = self.lock();
        conn.execute(
            "INSERT INTO projects(path, name, added_at) VALUES (?1, ?2, ?3) ON CONFLICT(path) DO UPDATE SET name = excluded.name",
            params![path, name, now],
        )?;
        Ok(ProjectRef {
            path: path.to_string(),
            name: name.to_string(),
            added_at: now,
        })
    }

    pub fn remove_project(&self, path: &str) -> Result<bool> {
        let conn = self.lock();
        Ok(conn.execute("DELETE FROM projects WHERE path = ?1", params![path])? > 0)
    }

    // ---- custom targets / overrides -------------------------------------------------------

    pub fn list_custom_targets(&self) -> Result<Vec<crate::model::TargetSpec>> {
        let conn = self.lock();
        let mut stmt = conn.prepare("SELECT spec_json FROM custom_targets ORDER BY id")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(serde_json::from_str(&row?)?);
        }
        Ok(out)
    }

    pub fn upsert_custom_target(&self, spec: &crate::model::TargetSpec) -> Result<()> {
        let text = serde_json::to_string(spec)?;
        let conn = self.lock();
        conn.execute(
            "INSERT INTO custom_targets(id, spec_json) VALUES (?1, ?2) ON CONFLICT(id) DO UPDATE SET spec_json = excluded.spec_json",
            params![spec.id, text],
        )?;
        Ok(())
    }

    pub fn remove_custom_target(&self, id: &str) -> Result<bool> {
        let conn = self.lock();
        Ok(conn.execute("DELETE FROM custom_targets WHERE id = ?1", params![id])? > 0)
    }

    pub fn target_overrides(&self) -> Result<std::collections::BTreeMap<String, String>> {
        let conn = self.lock();
        let mut stmt = conn.prepare("SELECT target_id, skills_dir FROM target_overrides")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn set_target_override(&self, target_id: &str, skills_dir: Option<&str>) -> Result<()> {
        let conn = self.lock();
        match skills_dir {
            Some(dir) => {
                conn.execute(
                    "INSERT INTO target_overrides(target_id, skills_dir) VALUES (?1, ?2) ON CONFLICT(target_id) DO UPDATE SET skills_dir = excluded.skills_dir",
                    params![target_id, dir],
                )?;
            }
            None => {
                conn.execute(
                    "DELETE FROM target_overrides WHERE target_id = ?1",
                    params![target_id],
                )?;
            }
        }
        Ok(())
    }
}

/// Build an FTS5 MATCH expression from free text. Each token becomes a quoted prefix term.
/// `require_all` joins with AND (library filtering); otherwise OR (ranking for recommendations).
pub fn fts_query(text: &str, require_all: bool) -> Option<String> {
    let tokens: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .filter(|t| t.len() >= 2)
        .map(|t| format!("\"{}\"*", t.replace('"', "")))
        .collect();
    if tokens.is_empty() {
        return None;
    }
    Some(tokens.join(if require_all { " AND " } else { " OR " }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_and_migrates_with_fts() {
        let db = Database::open_memory().unwrap();
        db.with(|c| {
            let v: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0))?;
            assert_eq!(v, SCHEMA_VERSION);
            c.execute(
                "INSERT INTO items(id, kind, name, slug, path, content_hash, source_json, created_at, updated_at) VALUES ('x','skill','pdf tools','pdf-tools','/x','h','{}',0,0)",
                [],
            )?;
            c.execute(
                "INSERT INTO items_search(item_id, name, description, body, tags) VALUES ('x','pdf tools','extract text from pdf files','body','docs')",
                [],
            )?;
            let n: i64 = c.query_row("SELECT count(*) FROM items_fts WHERE items_fts MATCH '\"pdf\"*'", [], |r| r.get(0))?;
            assert_eq!(n, 1);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn settings_round_trip() {
        let db = Database::open_memory().unwrap();
        assert!(db.get_setting::<String>("theme").unwrap().is_none());
        db.set_setting("theme", &"dark").unwrap();
        assert_eq!(
            db.get_setting::<String>("theme").unwrap().as_deref(),
            Some("dark")
        );
        db.delete_setting("theme").unwrap();
        assert!(db.get_setting::<String>("theme").unwrap().is_none());
    }

    #[test]
    fn fts_query_building() {
        assert_eq!(
            fts_query("pdf extract", true).unwrap(),
            "\"pdf\"* AND \"extract\"*"
        );
        assert_eq!(
            fts_query("pdf extract", false).unwrap(),
            "\"pdf\"* OR \"extract\"*"
        );
        assert!(fts_query("a", true).is_none());
        assert_eq!(
            fts_query("say \"hi\"", true).unwrap(),
            "\"say\"* AND \"hi\"*"
        );
    }

    #[test]
    fn projects_and_overrides() {
        let db = Database::open_memory().unwrap();
        db.add_project("/p", "P").unwrap();
        assert_eq!(db.list_projects().unwrap().len(), 1);
        db.set_target_override("codex", Some("/x")).unwrap();
        assert_eq!(db.target_overrides().unwrap().get("codex").unwrap(), "/x");
        db.set_target_override("codex", None).unwrap();
        assert!(db.target_overrides().unwrap().is_empty());
        assert!(db.remove_project("/p").unwrap());
    }
}
