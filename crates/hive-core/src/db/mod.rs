//! SQLite persistence. One connection behind a mutex: every call is short, and long work
//! (cloning, running evaluations) happens outside the lock.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use rusqlite::{Connection, OpenFlags, OptionalExtension, Transaction, params};

use crate::error::{HiveError, IoContext, Result};
use crate::model::ProjectRef;

pub mod deployments;
pub mod items;
pub mod loadouts;
pub mod providers;
pub mod runs;

const SCHEMA: &str = include_str!("schema.sql");
pub const SCHEMA_VERSION: i64 = 1;

/// Every table of [`SCHEMA`]. A file that has tables but lacks any of these was not written by
/// this program: an earlier incarnation of Skill Hive kept a different schema under the same
/// `hive.db` name and also set `user_version`, so the version number alone cannot be trusted.
const REQUIRED_TABLES: &[&str] = &[
    "meta",
    "items",
    "item_tags",
    "items_search",
    "deployments",
    "projects",
    "custom_targets",
    "target_overrides",
    "loadouts",
    "loadout_items",
    "providers",
    "eval_runs",
    "eval_case_results",
    "settings",
    "cache",
    "audit",
];

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
        if path.exists() {
            if let Some(reason) = foreign_database(path) {
                quarantine(path, &reason)?;
            }
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
        // The schema is written with IF NOT EXISTS, so applying it to a database that already
        // has every table is harmless; the table check covers files whose version number was
        // set by something else.
        if version < SCHEMA_VERSION || !missing_tables(&conn)?.is_empty() {
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

/// Names of the [`REQUIRED_TABLES`] that `conn` does not have.
fn missing_tables(conn: &Connection) -> Result<Vec<&'static str>> {
    let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table'")?;
    let present: HashSet<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(REQUIRED_TABLES
        .iter()
        .copied()
        .filter(|t| !present.contains(*t))
        .collect())
}

/// Why an existing `hive.db` cannot be used as it is, or `None` when it is ours (or still empty).
/// A file that cannot be opened or read counts as foreign too: the index is rebuildable from the
/// library directory, so starting over beats refusing to start.
fn foreign_database(path: &Path) -> Option<String> {
    let conn = match Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(conn) => conn,
        Err(e) => return Some(format!("it cannot be opened: {e}")),
    };
    let tables: Vec<String> = match conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
        .and_then(|mut s| {
            s.query_map([], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()
        }) {
        Ok(tables) => tables,
        Err(e) => return Some(format!("it cannot be read: {e}")),
    };
    if tables.is_empty() {
        return None; // a fresh file: the schema goes in next
    }
    let present: HashSet<&str> = tables.iter().map(String::as_str).collect();
    let missing: Vec<&str> = REQUIRED_TABLES
        .iter()
        .copied()
        .filter(|t| !present.contains(t))
        .collect();
    if missing.is_empty() {
        return None;
    }
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap_or(-1);
    Some(format!(
        "it has user_version {version} and {} tables but none called {}",
        tables.len(),
        missing.join(", ")
    ))
}

/// Move a database that is not ours out of the way (with its WAL and shared-memory files, which
/// SQLite names after the main file) so a fresh one can be created in its place.
fn quarantine(path: &Path, reason: &str) -> Result<()> {
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let base = path.to_string_lossy();
    for suffix in ["", "-wal", "-shm"] {
        let from = PathBuf::from(format!("{base}{suffix}"));
        if from.exists() {
            let to = PathBuf::from(format!("{base}.old-{stamp}{suffix}"));
            std::fs::rename(&from, &to).at(&from)?;
        }
    }
    tracing::warn!(
        path = %path.display(),
        reason,
        "moved aside a database that was not written by this version of Skill Hive (kept as \
         {}.old-{stamp}); a fresh one is created, run `hive doctor --reindex` to re-index the library",
        base
    );
    Ok(())
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

    fn old_files(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.contains(".old-"))
            .collect();
        names.sort();
        names
    }

    #[test]
    fn database_from_another_program_is_moved_aside() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("hive.db");
        {
            // What an earlier incarnation of the app left behind: its own tables and a
            // user_version that happens to be higher than ours.
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE skills (id TEXT PRIMARY KEY, name TEXT); \
                 INSERT INTO skills VALUES ('a', 'old'); \
                 PRAGMA user_version = 7;",
            )
            .unwrap();
        }
        let db = Database::open(&path).unwrap();
        assert!(db.list_custom_targets().unwrap().is_empty());
        db.with(|c| {
            let v: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0))?;
            assert_eq!(v, SCHEMA_VERSION);
            let old: i64 = c.query_row(
                "SELECT count(*) FROM sqlite_master WHERE name = 'skills'",
                [],
                |r| r.get(0),
            )?;
            assert_eq!(
                old, 0,
                "the foreign table must not survive into the new file"
            );
            Ok(())
        })
        .unwrap();
        let moved = old_files(tmp.path());
        assert_eq!(moved.len(), 1, "{moved:?}");
        assert!(moved[0].starts_with("hive.db.old-"));
    }

    #[test]
    fn own_database_is_reused_and_a_damaged_file_is_replaced() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("hive.db");
        {
            let db = Database::open(&path).unwrap();
            db.set_setting("theme", &"dark").unwrap();
        }
        let db = Database::open(&path).unwrap();
        assert_eq!(
            db.get_setting::<String>("theme").unwrap().as_deref(),
            Some("dark")
        );
        assert!(old_files(tmp.path()).is_empty());
        drop(db);

        std::fs::write(&path, b"this is not a database").unwrap();
        let db = Database::open(&path).unwrap();
        assert!(db.get_setting::<String>("theme").unwrap().is_none());
        assert_eq!(old_files(tmp.path()).len(), 1);
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
