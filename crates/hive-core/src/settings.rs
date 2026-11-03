//! Typed settings stored as JSON rows in the `settings` table.

use crate::db::Database;
use crate::error::Result;
use crate::model::{LinkMode, Settings, SettingsPatch};
use crate::paths::HivePaths;
use crate::targets::CLAUDE_CODE_DESCRIPTION_BUDGET_CHARS;

pub const DEFAULT_OVERLAP_THRESHOLD: f64 = 0.6;

pub mod keys {
    pub const LINK_MODE: &str = "default_link_mode";
    pub const BUDGET_CHARS: &str = "default_description_budget_chars";
    pub const EXECUTOR: &str = "default_executor_id";
    pub const GRADER: &str = "default_grader_id";
    pub const EXTRA_SCAN_DIRS: &str = "extra_scan_dirs";
    pub const THEME: &str = "theme";
    pub const OVERLAP_THRESHOLD: &str = "overlap_threshold";
}

/// Name of the GitHub token entry in the secret store.
pub const GITHUB_TOKEN_SECRET: &str = "github-token";

pub fn load(db: &Database, paths: &HivePaths, github_token_set: bool) -> Result<Settings> {
    Ok(Settings {
        home: paths.home.to_string_lossy().to_string(),
        default_link_mode: db.get_setting(keys::LINK_MODE)?.unwrap_or(LinkMode::Link),
        default_description_budget_chars: db
            .get_setting(keys::BUDGET_CHARS)?
            .unwrap_or(CLAUDE_CODE_DESCRIPTION_BUDGET_CHARS),
        default_executor_id: db.get_setting(keys::EXECUTOR)?,
        default_grader_id: db.get_setting(keys::GRADER)?,
        extra_scan_dirs: db.get_setting(keys::EXTRA_SCAN_DIRS)?.unwrap_or_default(),
        github_token_set,
        theme: db
            .get_setting(keys::THEME)?
            .unwrap_or_else(|| "system".to_string()),
        overlap_threshold: db
            .get_setting(keys::OVERLAP_THRESHOLD)?
            .unwrap_or(DEFAULT_OVERLAP_THRESHOLD),
    })
}

/// Apply a patch. The GitHub token is handled by the caller (it goes to the secret store).
pub fn apply(db: &Database, patch: &SettingsPatch) -> Result<()> {
    if let Some(v) = patch.default_link_mode {
        db.set_setting(keys::LINK_MODE, &v)?;
    }
    if let Some(v) = patch.default_description_budget_chars {
        db.set_setting(keys::BUDGET_CHARS, &v.max(1000))?;
    }
    if let Some(v) = &patch.default_executor_id {
        match v {
            Some(id) => db.set_setting(keys::EXECUTOR, id)?,
            None => db.delete_setting(keys::EXECUTOR)?,
        }
    }
    if let Some(v) = &patch.default_grader_id {
        match v {
            Some(id) => db.set_setting(keys::GRADER, id)?,
            None => db.delete_setting(keys::GRADER)?,
        }
    }
    if let Some(v) = &patch.extra_scan_dirs {
        db.set_setting(keys::EXTRA_SCAN_DIRS, v)?;
    }
    if let Some(v) = &patch.theme {
        db.set_setting(keys::THEME, v)?;
    }
    if let Some(v) = patch.overlap_threshold {
        db.set_setting(keys::OVERLAP_THRESHOLD, &v.clamp(0.2, 0.95))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_patch() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = HivePaths::new(tmp.path());
        let db = Database::open_memory().unwrap();
        let s = load(&db, &paths, false).unwrap();
        assert_eq!(s.default_link_mode, LinkMode::Link);
        assert_eq!(
            s.default_description_budget_chars,
            CLAUDE_CODE_DESCRIPTION_BUDGET_CHARS
        );
        assert_eq!(s.theme, "system");
        apply(
            &db,
            &SettingsPatch {
                default_link_mode: Some(LinkMode::Copy),
                default_executor_id: Some(Some("p1".into())),
                theme: Some("dark".into()),
                overlap_threshold: Some(2.0),
                ..Default::default()
            },
        )
        .unwrap();
        let s = load(&db, &paths, true).unwrap();
        assert_eq!(s.default_link_mode, LinkMode::Copy);
        assert_eq!(s.default_executor_id.as_deref(), Some("p1"));
        assert_eq!(s.theme, "dark");
        assert_eq!(s.overlap_threshold, 0.95);
        assert!(s.github_token_set);
        apply(
            &db,
            &SettingsPatch {
                default_executor_id: Some(None),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            load(&db, &paths, false)
                .unwrap()
                .default_executor_id
                .is_none()
        );
    }
}
