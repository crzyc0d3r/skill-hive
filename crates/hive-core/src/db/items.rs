//! Library item repository.

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::error::{HiveError, Result};
use crate::model::{
    AssetKind, EffectivenessSummary, LibraryItem, LibraryQuery, LibrarySort, LibraryStats,
    SourceRef, TagCount, ValidationReport, ValidationStatus,
};

use super::{Database, fts_query};

/// Everything stored for an item (the public [`LibraryItem`] plus the body and validation report).
#[derive(Debug, Clone)]
pub struct ItemRecord {
    pub item: LibraryItem,
    pub body: String,
    pub validation: ValidationReport,
}

const SELECT_COLUMNS: &str = "i.id, i.kind, i.name, i.slug, i.description, i.path, i.content_hash, i.size_bytes, i.token_estimate, i.description_tokens, i.frontmatter_json, i.body, i.source_json, i.validation, i.validation_json, i.effectiveness_json, i.favorite, i.notes, i.created_at, i.updated_at, (SELECT count(*) FROM deployments d WHERE d.item_id = i.id) AS deployment_count, (SELECT group_concat(t.tag, '\u{1f}') FROM item_tags t WHERE t.item_id = i.id) AS tags";

fn row_to_record(row: &Row<'_>) -> rusqlite::Result<ItemRecord> {
    let kind: String = row.get(1)?;
    let frontmatter: String = row.get(10)?;
    let source: String = row.get(12)?;
    let validation: String = row.get(13)?;
    let validation_json: String = row.get(14)?;
    let effectiveness_json: Option<String> = row.get(15)?;
    let tags: Option<String> = row.get(21)?;
    let item = LibraryItem {
        id: row.get(0)?,
        kind: AssetKind::parse(&kind).unwrap_or(AssetKind::Skill),
        name: row.get(2)?,
        slug: row.get(3)?,
        description: row.get(4)?,
        path: row.get(5)?,
        content_hash: row.get(6)?,
        size_bytes: row.get::<_, i64>(7)? as u64,
        token_estimate: row.get::<_, i64>(8)? as u32,
        description_tokens: row.get::<_, i64>(9)? as u32,
        tags: tags
            .map(|t| t.split('\u{1f}').map(|s| s.to_string()).collect())
            .unwrap_or_default(),
        source: serde_json::from_str::<SourceRef>(&source)
            .unwrap_or_else(|_| SourceRef::authored()),
        frontmatter: serde_json::from_str(&frontmatter).unwrap_or(serde_json::Value::Null),
        validation: ValidationStatus::parse(&validation),
        effectiveness: effectiveness_json
            .and_then(|t| serde_json::from_str::<EffectivenessSummary>(&t).ok()),
        favorite: row.get::<_, i64>(16)? != 0,
        notes: row.get(17)?,
        deployment_count: row.get::<_, i64>(20)? as u32,
        created_at: row.get(18)?,
        updated_at: row.get(19)?,
    };
    Ok(ItemRecord {
        item,
        body: row.get(11)?,
        validation: serde_json::from_str(&validation_json).unwrap_or_default(),
    })
}

/// Fields needed to insert or replace an item.
#[derive(Debug, Clone)]
pub struct NewItem {
    pub id: String,
    pub kind: AssetKind,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub path: String,
    pub content_hash: String,
    pub size_bytes: u64,
    pub token_estimate: u32,
    pub description_tokens: u32,
    pub frontmatter: serde_json::Value,
    pub body: String,
    pub source: SourceRef,
    pub validation: ValidationReport,
    pub tags: Vec<String>,
}

fn write_search_row(
    conn: &Connection,
    id: &str,
    name: &str,
    description: &str,
    body: &str,
    tags: &[String],
) -> Result<()> {
    let tag_text = tags.join(" ");
    conn.execute(
        "INSERT INTO items_search(item_id, name, description, body, tags) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(item_id) DO UPDATE SET name = excluded.name, description = excluded.description, body = excluded.body, tags = excluded.tags",
        params![id, name, description, body, tag_text],
    )?;
    Ok(())
}

impl Database {
    pub fn insert_item(&self, new: &NewItem) -> Result<LibraryItem> {
        let now = crate::util::now_ms();
        self.transaction(|tx| {
            tx.execute(
                "INSERT INTO items(id, kind, name, slug, description, path, content_hash, size_bytes, token_estimate, description_tokens, frontmatter_json, body, source_json, validation, validation_json, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16)",
                params![
                    new.id,
                    new.kind.as_str(),
                    new.name,
                    new.slug,
                    new.description,
                    new.path,
                    new.content_hash,
                    new.size_bytes as i64,
                    new.token_estimate as i64,
                    new.description_tokens as i64,
                    new.frontmatter.to_string(),
                    new.body,
                    serde_json::to_string(&new.source)?,
                    new.validation.status().as_str(),
                    serde_json::to_string(&new.validation)?,
                    now,
                ],
            )?;
            for tag in &new.tags {
                tx.execute("INSERT OR IGNORE INTO item_tags(item_id, tag) VALUES (?1, ?2)", params![new.id, tag])?;
            }
            write_search_row(tx, &new.id, &new.name, &new.description, &new.body, &new.tags)?;
            Ok(())
        })?;
        self.get_item(&new.id)?
            .ok_or_else(|| HiveError::not_found(&new.id))
    }

    /// Refresh content-derived fields after the files changed (re-import, edit, update from source).
    pub fn update_item_content(&self, new: &NewItem) -> Result<LibraryItem> {
        let now = crate::util::now_ms();
        self.transaction(|tx| {
            let changed = tx.execute(
                "UPDATE items SET name = ?2, slug = ?3, description = ?4, path = ?5, content_hash = ?6, size_bytes = ?7, token_estimate = ?8, description_tokens = ?9, frontmatter_json = ?10, body = ?11, source_json = ?12, validation = ?13, validation_json = ?14, updated_at = ?15 WHERE id = ?1",
                params![
                    new.id,
                    new.name,
                    new.slug,
                    new.description,
                    new.path,
                    new.content_hash,
                    new.size_bytes as i64,
                    new.token_estimate as i64,
                    new.description_tokens as i64,
                    new.frontmatter.to_string(),
                    new.body,
                    serde_json::to_string(&new.source)?,
                    new.validation.status().as_str(),
                    serde_json::to_string(&new.validation)?,
                    now,
                ],
            )?;
            if changed == 0 {
                return Err(HiveError::not_found(&new.id));
            }
            if !new.tags.is_empty() {
                for tag in &new.tags {
                    tx.execute("INSERT OR IGNORE INTO item_tags(item_id, tag) VALUES (?1, ?2)", params![new.id, tag])?;
                }
            }
            let tags = tags_for(tx, &new.id)?;
            write_search_row(tx, &new.id, &new.name, &new.description, &new.body, &tags)?;
            Ok(())
        })?;
        self.get_item(&new.id)?
            .ok_or_else(|| HiveError::not_found(&new.id))
    }

    pub fn get_item(&self, id: &str) -> Result<Option<LibraryItem>> {
        Ok(self.get_item_record(id)?.map(|r| r.item))
    }

    pub fn get_item_record(&self, id: &str) -> Result<Option<ItemRecord>> {
        let conn = self.lock();
        let sql = format!("SELECT {SELECT_COLUMNS} FROM items i WHERE i.id = ?1");
        Ok(conn
            .query_row(&sql, params![id], row_to_record)
            .optional()?)
    }

    pub fn find_item_by_name(&self, kind: AssetKind, name: &str) -> Result<Option<LibraryItem>> {
        let conn = self.lock();
        let sql = format!(
            "SELECT {SELECT_COLUMNS} FROM items i WHERE i.kind = ?1 AND i.name = ?2 COLLATE NOCASE LIMIT 1"
        );
        Ok(conn
            .query_row(&sql, params![kind.as_str(), name], row_to_record)
            .optional()?
            .map(|r| r.item))
    }

    pub fn find_item_by_path(&self, path: &str) -> Result<Option<LibraryItem>> {
        let conn = self.lock();
        let sql = format!("SELECT {SELECT_COLUMNS} FROM items i WHERE i.path = ?1");
        Ok(conn
            .query_row(&sql, params![path], row_to_record)
            .optional()?
            .map(|r| r.item))
    }

    pub fn find_items_by_hash(&self, hash: &str) -> Result<Vec<LibraryItem>> {
        let conn = self.lock();
        let sql = format!("SELECT {SELECT_COLUMNS} FROM items i WHERE i.content_hash = ?1");
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![hash], row_to_record)?;
        Ok(rows
            .map(|r| r.map(|rec| rec.item))
            .collect::<std::result::Result<_, _>>()?)
    }

    pub fn slug_exists(&self, kind: AssetKind, slug: &str) -> Result<bool> {
        let conn = self.lock();
        let n: i64 = conn.query_row(
            "SELECT count(*) FROM items WHERE kind = ?1 AND slug = ?2",
            params![kind.as_str(), slug],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    pub fn all_items(&self) -> Result<Vec<LibraryItem>> {
        self.query_items(&LibraryQuery::default())
    }

    pub fn items_by_ids(&self, ids: &[String]) -> Result<Vec<LibraryItem>> {
        let mut out = Vec::new();
        for id in ids {
            if let Some(item) = self.get_item(id)? {
                out.push(item);
            }
        }
        Ok(out)
    }

    /// Filtered, optionally full-text, listing.
    pub fn query_items(&self, q: &LibraryQuery) -> Result<Vec<LibraryItem>> {
        let conn = self.lock();
        let mut sql = format!("SELECT {SELECT_COLUMNS}");
        let mut where_clauses: Vec<String> = Vec::new();
        let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let text = q.text.as_deref().map(str::trim).filter(|t| !t.is_empty());
        let match_expr = text.and_then(|t| fts_query(t, true));
        if let Some(expr) = &match_expr {
            sql.push_str(", bm25(items_fts, 10.0, 6.0, 1.0, 4.0) AS rank FROM items i JOIN items_search s ON s.item_id = i.id JOIN items_fts ON items_fts.rowid = s.rowid");
            where_clauses.push("items_fts MATCH ?".into());
            args.push(Box::new(expr.clone()));
        } else {
            sql.push_str(", 0.0 AS rank FROM items i");
            if let Some(t) = text {
                // Too short for FTS: fall back to a LIKE on the name.
                where_clauses.push("(i.name LIKE ? OR i.description LIKE ?)".into());
                let like = format!("%{t}%");
                args.push(Box::new(like.clone()));
                args.push(Box::new(like));
            }
        }
        if !q.kinds.is_empty() {
            let placeholders = vec!["?"; q.kinds.len()].join(",");
            where_clauses.push(format!("i.kind IN ({placeholders})"));
            for k in &q.kinds {
                args.push(Box::new(k.as_str().to_string()));
            }
        }
        for tag in &q.tags {
            where_clauses.push(
                "EXISTS (SELECT 1 FROM item_tags t WHERE t.item_id = i.id AND t.tag = ?)".into(),
            );
            args.push(Box::new(tag.clone()));
        }
        if !q.source_kinds.is_empty() {
            let mut parts = Vec::new();
            for sk in &q.source_kinds {
                parts.push("json_extract(i.source_json, '$.kind') = ?".to_string());
                args.push(Box::new(
                    serde_json::to_value(sk)?
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                ));
            }
            where_clauses.push(format!("({})", parts.join(" OR ")));
        }
        if q.favorites_only {
            where_clauses.push("i.favorite = 1".into());
        }
        if let Some(target) = &q.deployed_to {
            where_clauses.push(
                "EXISTS (SELECT 1 FROM deployments d WHERE d.item_id = i.id AND d.target_id = ?)"
                    .into(),
            );
            args.push(Box::new(target.clone()));
        }
        if !where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_clauses.join(" AND "));
        }
        let order = match q.sort {
            LibrarySort::Relevance if match_expr.is_some() => "rank ASC, i.name COLLATE NOCASE ASC",
            LibrarySort::Relevance | LibrarySort::Name => "i.name COLLATE NOCASE ASC",
            LibrarySort::Updated => "i.updated_at DESC",
            LibrarySort::Effectiveness => {
                "json_extract(i.effectiveness_json, '$.delta') DESC NULLS LAST, i.name COLLATE NOCASE ASC"
            }
        };
        sql.push_str(" ORDER BY ");
        sql.push_str(order);
        if let Some(limit) = q.limit {
            sql.push_str(&format!(" LIMIT {}", limit.max(1)));
        }
        let mut stmt = conn.prepare(&sql)?;
        let params_ref: Vec<&dyn rusqlite::ToSql> = args.iter().map(|b| b.as_ref()).collect();
        let rows = stmt.query_map(params_ref.as_slice(), row_to_record)?;
        Ok(rows
            .map(|r| r.map(|rec| rec.item))
            .collect::<std::result::Result<_, _>>()?)
    }

    /// Full-text ranking for recommendations: `(item_id, bm25 score)` with OR semantics.
    /// Lower bm25 is better; the caller normalises.
    pub fn rank_items(&self, text: &str, limit: u32) -> Result<Vec<(String, f64)>> {
        let Some(expr) = fts_query(text, false) else {
            return Ok(Vec::new());
        };
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT s.item_id, bm25(items_fts, 10.0, 6.0, 1.0, 4.0) AS rank FROM items_fts JOIN items_search s ON s.rowid = items_fts.rowid WHERE items_fts MATCH ?1 ORDER BY rank ASC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![expr, limit as i64], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn delete_item(&self, id: &str) -> Result<bool> {
        self.transaction(|tx| {
            tx.execute("DELETE FROM items_search WHERE item_id = ?1", params![id])?;
            Ok(tx.execute("DELETE FROM items WHERE id = ?1", params![id])? > 0)
        })
    }

    pub fn set_item_tags(&self, id: &str, tags: &[String]) -> Result<()> {
        self.transaction(|tx| {
            tx.execute("DELETE FROM item_tags WHERE item_id = ?1", params![id])?;
            for tag in tags {
                let tag = tag.trim().to_lowercase();
                if !tag.is_empty() {
                    tx.execute(
                        "INSERT OR IGNORE INTO item_tags(item_id, tag) VALUES (?1, ?2)",
                        params![id, tag],
                    )?;
                }
            }
            tx.execute(
                "UPDATE items_search SET tags = ?2 WHERE item_id = ?1",
                params![id, tags.join(" ")],
            )?;
            tx.execute(
                "UPDATE items SET updated_at = ?2 WHERE id = ?1",
                params![id, crate::util::now_ms()],
            )?;
            Ok(())
        })
    }

    pub fn set_item_favorite(&self, id: &str, favorite: bool) -> Result<()> {
        let conn = self.lock();
        conn.execute(
            "UPDATE items SET favorite = ?2 WHERE id = ?1",
            params![id, favorite as i64],
        )?;
        Ok(())
    }

    pub fn set_item_notes(&self, id: &str, notes: Option<&str>) -> Result<()> {
        let conn = self.lock();
        conn.execute(
            "UPDATE items SET notes = ?2 WHERE id = ?1",
            params![id, notes],
        )?;
        Ok(())
    }

    pub fn set_item_effectiveness(
        &self,
        id: &str,
        summary: Option<&EffectivenessSummary>,
    ) -> Result<()> {
        let text = summary.map(serde_json::to_string).transpose()?;
        let conn = self.lock();
        conn.execute(
            "UPDATE items SET effectiveness_json = ?2 WHERE id = ?1",
            params![id, text],
        )?;
        Ok(())
    }

    pub fn library_stats(&self) -> Result<LibraryStats> {
        let conn = self.lock();
        let count = |sql: &str| -> Result<u32> {
            Ok(conn.query_row(sql, [], |r| r.get::<_, i64>(0))? as u32)
        };
        let total = count("SELECT count(*) FROM items")?;
        let skills = count("SELECT count(*) FROM items WHERE kind = 'skill'")?;
        let agents = count("SELECT count(*) FROM items WHERE kind = 'agent'")?;
        let commands = count("SELECT count(*) FROM items WHERE kind = 'command'")?;
        let deployed = count("SELECT count(DISTINCT item_id) FROM deployments")?;
        let tested = count("SELECT count(*) FROM items WHERE effectiveness_json IS NOT NULL")?;
        let with_errors = count("SELECT count(*) FROM items WHERE validation = 'errors'")?;
        let mut stmt = conn.prepare("SELECT tag, count(*) FROM item_tags GROUP BY tag ORDER BY count(*) DESC, tag ASC LIMIT 100")?;
        let tags = stmt
            .query_map([], |r| {
                Ok(TagCount {
                    tag: r.get(0)?,
                    count: r.get::<_, i64>(1)? as u32,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(LibraryStats {
            total,
            skills,
            agents,
            commands,
            deployed,
            tested,
            with_errors,
            tags,
        })
    }
}

fn tags_for(conn: &Connection, id: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT tag FROM item_tags WHERE item_id = ?1 ORDER BY tag")?;
    let rows = stmt.query_map(params![id], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

#[cfg(test)]
pub(crate) fn sample_item(
    id: &str,
    name: &str,
    description: &str,
    body: &str,
    tags: &[&str],
) -> NewItem {
    NewItem {
        id: id.to_string(),
        kind: AssetKind::Skill,
        name: name.to_string(),
        slug: name.to_string(),
        description: description.to_string(),
        path: format!("/lib/skills/{name}"),
        content_hash: format!("hash-{id}"),
        size_bytes: 10,
        token_estimate: crate::util::estimate_tokens(body),
        description_tokens: crate::util::estimate_tokens(description),
        frontmatter: serde_json::json!({"name": name, "description": description}),
        body: body.to_string(),
        source: SourceRef::local("/tmp/x"),
        validation: ValidationReport::default(),
        tags: tags.iter().map(|t| t.to_string()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_query_and_search() {
        let db = Database::open_memory().unwrap();
        db.insert_item(&sample_item(
            "1",
            "pdf-tools",
            "Extract text and tables from PDF files",
            "Run the script",
            &["docs", "pdf"],
        ))
        .unwrap();
        db.insert_item(&sample_item(
            "2",
            "git-commit",
            "Write conventional commit messages",
            "Use git log",
            &["git"],
        ))
        .unwrap();
        let all = db.all_items().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].name, "git-commit");
        assert_eq!(all[1].tags, vec!["docs", "pdf"]);

        let q = LibraryQuery {
            text: Some("pdf".into()),
            ..Default::default()
        };
        let found = db.query_items(&q).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "1");

        let q = LibraryQuery {
            text: Some("extract tables".into()),
            ..Default::default()
        };
        assert_eq!(db.query_items(&q).unwrap().len(), 1);

        let q = LibraryQuery {
            tags: vec!["git".into()],
            ..Default::default()
        };
        assert_eq!(db.query_items(&q).unwrap()[0].id, "2");

        let ranked = db.rank_items("commit message for git", 10).unwrap();
        assert_eq!(ranked[0].0, "2");

        assert!(
            db.find_item_by_name(AssetKind::Skill, "PDF-TOOLS")
                .unwrap()
                .is_some()
        );
        assert_eq!(db.find_items_by_hash("hash-1").unwrap().len(), 1);
        assert!(db.slug_exists(AssetKind::Skill, "pdf-tools").unwrap());

        let stats = db.library_stats().unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.skills, 2);
        assert_eq!(stats.tags[0].count, 1);
    }

    #[test]
    fn update_tags_favorite_delete() {
        let db = Database::open_memory().unwrap();
        db.insert_item(&sample_item(
            "1",
            "a",
            "alpha skill for testing things",
            "body",
            &[],
        ))
        .unwrap();
        db.set_item_tags("1", &["X".into(), "y".into()]).unwrap();
        let item = db.get_item("1").unwrap().unwrap();
        assert_eq!(item.tags, vec!["x", "y"]);
        db.set_item_favorite("1", true).unwrap();
        assert!(db.get_item("1").unwrap().unwrap().favorite);
        let q = LibraryQuery {
            favorites_only: true,
            ..Default::default()
        };
        assert_eq!(db.query_items(&q).unwrap().len(), 1);

        let mut updated = sample_item(
            "1",
            "a",
            "alpha skill rewritten for pandas",
            "new body",
            &[],
        );
        updated.content_hash = "h2".into();
        db.update_item_content(&updated).unwrap();
        let q = LibraryQuery {
            text: Some("pandas".into()),
            ..Default::default()
        };
        assert_eq!(db.query_items(&q).unwrap().len(), 1);

        assert!(db.delete_item("1").unwrap());
        assert!(db.get_item("1").unwrap().is_none());
        assert!(db.query_items(&q).unwrap().is_empty());
    }
}
