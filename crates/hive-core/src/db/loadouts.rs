//! Loadout repository.

use rusqlite::{OptionalExtension, params};

use crate::error::{HiveError, Result};
use crate::model::{Loadout, LoadoutInput, LoadoutRef};

use super::Database;

impl Database {
    fn load_loadout(conn: &rusqlite::Connection, id: &str) -> Result<Option<Loadout>> {
        let base = conn
            .query_row(
                "SELECT id, name, description, color, created_at, updated_at FROM loadouts WHERE id = ?1",
                params![id],
                |r| {
                    Ok(Loadout {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        description: r.get(2)?,
                        item_ids: Vec::new(),
                        color: r.get(3)?,
                        created_at: r.get(4)?,
                        updated_at: r.get(5)?,
                    })
                },
            )
            .optional()?;
        let Some(mut loadout) = base else {
            return Ok(None);
        };
        let mut stmt = conn.prepare(
            "SELECT item_id FROM loadout_items WHERE loadout_id = ?1 ORDER BY position, item_id",
        )?;
        loadout.item_ids = stmt
            .query_map(params![id], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<_, _>>()?;
        Ok(Some(loadout))
    }

    pub fn list_loadouts(&self) -> Result<Vec<Loadout>> {
        self.with(|c| {
            let mut stmt = c.prepare("SELECT id FROM loadouts ORDER BY name COLLATE NOCASE")?;
            let ids: Vec<String> = stmt
                .query_map([], |r| r.get(0))?
                .collect::<std::result::Result<_, _>>()?;
            let mut out = Vec::new();
            for id in ids {
                if let Some(l) = Self::load_loadout(c, &id)? {
                    out.push(l);
                }
            }
            Ok(out)
        })
    }

    pub fn get_loadout(&self, id: &str) -> Result<Option<Loadout>> {
        self.with(|c| Self::load_loadout(c, id))
    }

    pub fn create_loadout(&self, input: &LoadoutInput) -> Result<Loadout> {
        let name = input.name.trim();
        if name.is_empty() {
            return Err(HiveError::invalid("loadout name is required"));
        }
        let id = crate::util::new_id();
        let now = crate::util::now_ms();
        self.transaction(|tx| {
            tx.execute(
                "INSERT INTO loadouts(id, name, description, color, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![id, name, input.description.trim(), input.color, now],
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(err, _) if err.code == rusqlite::ErrorCode::ConstraintViolation => {
                    HiveError::AlreadyExists(format!("loadout '{name}'"))
                }
                other => HiveError::Db(other),
            })?;
            for (pos, item_id) in input.item_ids.iter().enumerate() {
                tx.execute(
                    "INSERT OR IGNORE INTO loadout_items(loadout_id, item_id, position) VALUES (?1, ?2, ?3)",
                    params![id, item_id, pos as i64],
                )?;
            }
            Ok(())
        })?;
        self.get_loadout(&id)?
            .ok_or_else(|| HiveError::not_found("loadout"))
    }

    pub fn update_loadout(&self, id: &str, input: &LoadoutInput) -> Result<Loadout> {
        let name = input.name.trim();
        if name.is_empty() {
            return Err(HiveError::invalid("loadout name is required"));
        }
        self.transaction(|tx| {
            let changed = tx.execute(
                "UPDATE loadouts SET name = ?2, description = ?3, color = ?4, updated_at = ?5 WHERE id = ?1",
                params![id, name, input.description.trim(), input.color, crate::util::now_ms()],
            )?;
            if changed == 0 {
                return Err(HiveError::not_found(format!("loadout {id}")));
            }
            tx.execute("DELETE FROM loadout_items WHERE loadout_id = ?1", params![id])?;
            for (pos, item_id) in input.item_ids.iter().enumerate() {
                tx.execute(
                    "INSERT OR IGNORE INTO loadout_items(loadout_id, item_id, position) VALUES (?1, ?2, ?3)",
                    params![id, item_id, pos as i64],
                )?;
            }
            Ok(())
        })?;
        self.get_loadout(id)?
            .ok_or_else(|| HiveError::not_found("loadout"))
    }

    pub fn delete_loadout(&self, id: &str) -> Result<bool> {
        self.with(|c| Ok(c.execute("DELETE FROM loadouts WHERE id = ?1", params![id])? > 0))
    }

    pub fn loadouts_for_item(&self, item_id: &str) -> Result<Vec<LoadoutRef>> {
        self.with(|c| {
            let mut stmt = c.prepare(
                "SELECT l.id, l.name FROM loadouts l JOIN loadout_items li ON li.loadout_id = l.id WHERE li.item_id = ?1 ORDER BY l.name",
            )?;
            let rows = stmt.query_map(params![item_id], |r| {
                Ok(LoadoutRef {
                    id: r.get(0)?,
                    name: r.get(1)?,
                })
            })?;
            Ok(rows.collect::<std::result::Result<_, _>>()?)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::items::sample_item;

    #[test]
    fn loadout_crud() {
        let db = Database::open_memory().unwrap();
        db.insert_item(&sample_item(
            "1",
            "a",
            "alpha skill for testing things",
            "body",
            &[],
        ))
        .unwrap();
        db.insert_item(&sample_item(
            "2",
            "b",
            "beta skill for testing things",
            "body",
            &[],
        ))
        .unwrap();
        let l = db
            .create_loadout(&LoadoutInput {
                name: "Data".into(),
                description: "d".into(),
                item_ids: vec!["2".into(), "1".into()],
                color: None,
            })
            .unwrap();
        assert_eq!(l.item_ids, vec!["2", "1"]);
        assert!(matches!(
            db.create_loadout(&LoadoutInput {
                name: "Data".into(),
                description: String::new(),
                item_ids: vec![],
                color: None
            }),
            Err(HiveError::AlreadyExists(_))
        ));
        let l = db
            .update_loadout(
                &l.id,
                &LoadoutInput {
                    name: "Data 2".into(),
                    description: String::new(),
                    item_ids: vec!["1".into()],
                    color: Some("#fff".into()),
                },
            )
            .unwrap();
        assert_eq!(l.item_ids, vec!["1"]);
        assert_eq!(db.loadouts_for_item("1").unwrap()[0].name, "Data 2");
        assert!(db.loadouts_for_item("2").unwrap().is_empty());
        // deleting an item removes it from loadouts
        db.delete_item("1").unwrap();
        assert!(db.get_loadout(&l.id).unwrap().unwrap().item_ids.is_empty());
        assert!(db.delete_loadout(&l.id).unwrap());
        assert!(db.list_loadouts().unwrap().is_empty());
    }
}
