//! Deployment records: which library item is linked or copied where.

use rusqlite::{OptionalExtension, Row, params};

use crate::error::Result;
use crate::model::{Deployment, InstallState, LinkMode, Scope};

use super::Database;

const COLUMNS: &str = "d.id, d.item_id, i.name, d.target_id, d.scope, d.project_path, d.path, d.mode, d.deployed_hash, d.created_at, d.updated_at";

fn row_to_deployment(row: &Row<'_>) -> rusqlite::Result<Deployment> {
    let scope: String = row.get(4)?;
    let mode: String = row.get(7)?;
    Ok(Deployment {
        id: row.get(0)?,
        item_id: row.get(1)?,
        item_name: row.get(2)?,
        target_id: row.get(3)?,
        scope: Scope::parse(&scope).unwrap_or(Scope::User),
        project_path: row.get(5)?,
        path: row.get(6)?,
        mode: LinkMode::parse(&mode).unwrap_or(LinkMode::Link),
        deployed_hash: row.get(8)?,
        state: InstallState::Linked,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

#[derive(Debug, Clone)]
pub struct NewDeployment {
    pub item_id: String,
    pub target_id: String,
    pub scope: Scope,
    pub project_path: Option<String>,
    pub path: String,
    pub mode: LinkMode,
    pub deployed_hash: String,
}

impl Database {
    pub fn upsert_deployment(&self, new: &NewDeployment) -> Result<Deployment> {
        let now = crate::util::now_ms();
        let id = crate::util::new_id();
        self.with(|c| {
            c.execute(
                "INSERT INTO deployments(id, item_id, target_id, scope, project_path, path, mode, deployed_hash, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
                 ON CONFLICT(path) DO UPDATE SET item_id = excluded.item_id, target_id = excluded.target_id, scope = excluded.scope, project_path = excluded.project_path, mode = excluded.mode, deployed_hash = excluded.deployed_hash, updated_at = excluded.updated_at",
                params![id, new.item_id, new.target_id, new.scope.as_str(), new.project_path, new.path, new.mode.as_str(), new.deployed_hash, now],
            )?;
            Ok(())
        })?;
        self.deployment_by_path(&new.path)?
            .ok_or_else(|| crate::error::HiveError::not_found("deployment"))
    }

    pub fn deployment_by_path(&self, path: &str) -> Result<Option<Deployment>> {
        self.with(|c| {
            let sql = format!("SELECT {COLUMNS} FROM deployments d JOIN items i ON i.id = d.item_id WHERE d.path = ?1");
            Ok(c.query_row(&sql, params![path], row_to_deployment).optional()?)
        })
    }

    pub fn deployment(&self, id: &str) -> Result<Option<Deployment>> {
        self.with(|c| {
            let sql = format!("SELECT {COLUMNS} FROM deployments d JOIN items i ON i.id = d.item_id WHERE d.id = ?1");
            Ok(c.query_row(&sql, params![id], row_to_deployment).optional()?)
        })
    }

    pub fn deployments_for_item(&self, item_id: &str) -> Result<Vec<Deployment>> {
        self.with(|c| {
            let sql = format!("SELECT {COLUMNS} FROM deployments d JOIN items i ON i.id = d.item_id WHERE d.item_id = ?1 ORDER BY d.target_id, d.scope, d.project_path");
            let mut stmt = c.prepare(&sql)?;
            let rows = stmt.query_map(params![item_id], row_to_deployment)?;
            Ok(rows.collect::<std::result::Result<_, _>>()?)
        })
    }

    pub fn deployments_for_target(
        &self,
        target_id: &str,
        scope: Scope,
        project_path: Option<&str>,
    ) -> Result<Vec<Deployment>> {
        self.with(|c| {
            let sql = format!("SELECT {COLUMNS} FROM deployments d JOIN items i ON i.id = d.item_id WHERE d.target_id = ?1 AND d.scope = ?2 AND ((?3 IS NULL AND d.project_path IS NULL) OR d.project_path = ?3) ORDER BY i.name");
            let mut stmt = c.prepare(&sql)?;
            let rows = stmt.query_map(params![target_id, scope.as_str(), project_path], row_to_deployment)?;
            Ok(rows.collect::<std::result::Result<_, _>>()?)
        })
    }

    pub fn all_deployments(&self) -> Result<Vec<Deployment>> {
        self.with(|c| {
            let sql = format!("SELECT {COLUMNS} FROM deployments d JOIN items i ON i.id = d.item_id ORDER BY d.target_id, i.name");
            let mut stmt = c.prepare(&sql)?;
            let rows = stmt.query_map([], row_to_deployment)?;
            Ok(rows.collect::<std::result::Result<_, _>>()?)
        })
    }

    pub fn update_deployment_hash(&self, id: &str, hash: &str) -> Result<()> {
        self.with(|c| {
            c.execute(
                "UPDATE deployments SET deployed_hash = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, hash, crate::util::now_ms()],
            )?;
            Ok(())
        })
    }

    pub fn delete_deployment(&self, id: &str) -> Result<bool> {
        self.with(|c| Ok(c.execute("DELETE FROM deployments WHERE id = ?1", params![id])? > 0))
    }

    pub fn delete_deployment_by_path(&self, path: &str) -> Result<bool> {
        self.with(|c| Ok(c.execute("DELETE FROM deployments WHERE path = ?1", params![path])? > 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::items::sample_item;

    #[test]
    fn deployment_crud() {
        let db = Database::open_memory().unwrap();
        db.insert_item(&sample_item(
            "1",
            "a",
            "alpha skill for testing things",
            "body",
            &[],
        ))
        .unwrap();
        let dep = db
            .upsert_deployment(&NewDeployment {
                item_id: "1".into(),
                target_id: "claude-code".into(),
                scope: Scope::User,
                project_path: None,
                path: "/home/u/.claude/skills/a".into(),
                mode: LinkMode::Link,
                deployed_hash: "hash-1".into(),
            })
            .unwrap();
        assert_eq!(dep.item_name, "a");
        assert_eq!(db.deployments_for_item("1").unwrap().len(), 1);
        assert_eq!(
            db.deployments_for_target("claude-code", Scope::User, None)
                .unwrap()
                .len(),
            1
        );
        assert!(
            db.deployments_for_target("claude-code", Scope::Project, Some("/p"))
                .unwrap()
                .is_empty()
        );
        assert_eq!(db.get_item("1").unwrap().unwrap().deployment_count, 1);
        db.update_deployment_hash(&dep.id, "h2").unwrap();
        assert_eq!(db.deployment(&dep.id).unwrap().unwrap().deployed_hash, "h2");
        // Same path again replaces the row.
        db.upsert_deployment(&NewDeployment {
            item_id: "1".into(),
            target_id: "claude-code".into(),
            scope: Scope::User,
            project_path: None,
            path: "/home/u/.claude/skills/a".into(),
            mode: LinkMode::Copy,
            deployed_hash: "h3".into(),
        })
        .unwrap();
        assert_eq!(db.all_deployments().unwrap().len(), 1);
        assert!(
            db.delete_deployment_by_path("/home/u/.claude/skills/a")
                .unwrap()
        );
        assert!(db.all_deployments().unwrap().is_empty());
    }
}
