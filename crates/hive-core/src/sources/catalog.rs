//! Curated catalog of well-known skill collections, with an optional remote refresh.

use crate::model::CatalogEntry;

const BUILTIN: &str = include_str!("catalog.json");

/// Where a newer catalog can be fetched from (the file in this repository's main branch).
pub const REMOTE_URL: &str = "https://raw.githubusercontent.com/crzyc0d3r/skill-hive/main/crates/hive-core/src/sources/catalog.json";

pub const CACHE_KEY: &str = "catalog";
pub const CACHE_MAX_AGE_MS: i64 = 24 * 60 * 60 * 1000;

pub fn builtin() -> Vec<CatalogEntry> {
    serde_json::from_str(BUILTIN).expect("built-in catalog is valid JSON")
}

/// Parse a remote catalog payload; entries missing required fields are dropped.
pub fn parse_remote(text: &str) -> Option<Vec<CatalogEntry>> {
    let entries: Vec<CatalogEntry> = serde_json::from_str(text).ok()?;
    let valid: Vec<CatalogEntry> = entries
        .into_iter()
        .filter(|e| !e.id.is_empty() && !e.source.is_empty() && !e.name.is_empty())
        .collect();
    (!valid.is_empty()).then_some(valid)
}

/// Simple text filter over name, description, tags and publisher.
pub fn filter(entries: &[CatalogEntry], query: &str) -> Vec<CatalogEntry> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return entries.to_vec();
    }
    let terms: Vec<&str> = q.split_whitespace().collect();
    entries
        .iter()
        .filter(|e| {
            let hay = format!(
                "{} {} {} {} {}",
                e.name.to_lowercase(),
                e.description.to_lowercase(),
                e.tags.join(" ").to_lowercase(),
                e.publisher.to_lowercase(),
                e.source.to_lowercase()
            );
            terms.iter().all(|t| hay.contains(t))
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_catalog_is_valid_and_unique() {
        let entries = builtin();
        assert!(entries.len() >= 10);
        let mut ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n);
        for e in &entries {
            assert!(
                crate::sources::parse_spec(&e.source).is_ok(),
                "{}",
                e.source
            );
            assert!(!e.description.is_empty());
        }
        assert_eq!(filter(&entries, "anthropic").len(), 1);
        assert!(
            filter(&entries, "security audit")
                .iter()
                .any(|e| e.id == "trailofbits-skills")
        );
        assert!(parse_remote("[]").is_none());
        assert!(parse_remote("nonsense").is_none());
    }
}
