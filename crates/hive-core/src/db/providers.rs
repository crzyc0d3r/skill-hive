//! Provider configuration repository. Secrets never live here.

use rusqlite::{OptionalExtension, params};

use crate::error::{HiveError, Result};
use crate::model::ProviderConfig;

use super::Database;

impl Database {
    pub fn list_providers(&self) -> Result<Vec<ProviderConfig>> {
        self.with(|c| {
            let mut stmt = c.prepare("SELECT config_json FROM providers ORDER BY created_at")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            let mut out = Vec::new();
            for row in rows {
                out.push(serde_json::from_str(&row?)?);
            }
            Ok(out)
        })
    }

    pub fn get_provider(&self, id: &str) -> Result<Option<ProviderConfig>> {
        self.with(|c| {
            let raw: Option<String> = c
                .query_row(
                    "SELECT config_json FROM providers WHERE id = ?1",
                    params![id],
                    |r| r.get(0),
                )
                .optional()?;
            Ok(raw.map(|t| serde_json::from_str(&t)).transpose()?)
        })
    }

    pub fn save_provider(&self, config: &ProviderConfig) -> Result<()> {
        let text = serde_json::to_string(config)?;
        self.with(|c| {
            c.execute(
                "INSERT INTO providers(id, config_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET config_json = excluded.config_json, updated_at = excluded.updated_at",
                params![config.id, text, config.created_at, config.updated_at],
            )?;
            Ok(())
        })
    }

    pub fn delete_provider(&self, id: &str) -> Result<bool> {
        self.with(|c| Ok(c.execute("DELETE FROM providers WHERE id = ?1", params![id])? > 0))
    }

    pub fn require_provider(&self, id: &str) -> Result<ProviderConfig> {
        self.get_provider(id)?
            .ok_or_else(|| HiveError::not_found(format!("provider {id}")))
    }
}
