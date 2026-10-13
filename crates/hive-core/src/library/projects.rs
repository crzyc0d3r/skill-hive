// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

//! Project and linked workspaces: relative-path checks, export, disable, and
//! the worst-verdict sync status used by the sidebar dot.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportRefusal {
    pub path: PathBuf,
    pub location: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SyncVerdict {
    InSync = 0,
    Missing = 1,
    LocalOnly = 2,
    CenterChanged = 3,
    ProjectChanged = 4,
    Diverged = 5,
}

pub fn slugify_skill_names(names: &[String]) -> Vec<String> {
    names.iter().map(|name| slugify(name)).collect()
}

pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut hyphen = false;
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            hyphen = false;
        } else if !hyphen && !out.is_empty() {
            out.push('-');
            hyphen = true;
        }
    }
    out.trim_matches('-').to_string()
}

pub fn validate_relative_path(path: &str) -> Result<()> {
    if path.is_empty() || path.trim().is_empty() {
        return Err(anyhow!("relative path is empty"));
    }
    let parsed = Path::new(path);
    if parsed.is_absolute() {
        return Err(anyhow!("path must stay inside the project"));
    }
    for component in parsed.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(anyhow!("path must stay inside the project"));
            }
        }
    }
    Ok(())
}

/// Copy a skill file into a project agent directory.
/// An existing target in the live directory or the `-disabled` sibling is refused.
pub fn export_skill(
    live_root: &Path,
    relative: &str,
    body: &str,
) -> Result<PathBuf, ExportRefusal> {
    validate_relative_path(relative).map_err(|_| ExportRefusal {
        path: PathBuf::from(relative),
        location: "invalid",
    })?;
    let live = live_root.join(relative);
    let disabled_root = disabled_sibling(live_root);
    let disabled = disabled_root.join(relative);
    if live.exists() {
        return Err(ExportRefusal {
            path: live,
            location: "live",
        });
    }
    if disabled.exists() {
        return Err(ExportRefusal {
            path: disabled,
            location: "disabled",
        });
    }
    if let Some(parent) = live.parent() {
        fs::create_dir_all(parent).map_err(|_| ExportRefusal {
            path: live.clone(),
            location: "live",
        })?;
    }
    fs::write(&live, body).map_err(|_| ExportRefusal {
        path: live.clone(),
        location: "live",
    })?;
    Ok(live)
}

pub fn disabled_sibling(live_root: &Path) -> PathBuf {
    let name = live_root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("skills");
    let parent = live_root.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{name}-disabled"))
}

/// Keep the first agent for each canonical root. Later agents that share it are dropped.
pub fn dedupe_agents_sharing_root(agents: &[(String, PathBuf)]) -> Vec<(String, PathBuf)> {
    let mut seen = BTreeSet::new();
    let mut kept = Vec::new();
    for (key, root) in agents {
        let canonical = canonical_root(root);
        if seen.insert(canonical) {
            kept.push((key.clone(), root.clone()));
        }
    }
    kept
}

fn canonical_root(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Disable a skill by moving its directory onto the `-disabled` sibling.
pub fn disable_by_moving(dir: &Path) -> Result<PathBuf> {
    if !dir.is_dir() {
        return Err(anyhow!("skill directory is missing"));
    }
    let name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow!("skill directory has no name"))?;
    let parent = dir
        .parent()
        .ok_or_else(|| anyhow!("skill directory has no parent"))?;
    let dest_root = parent.join(format!(
        "{}-disabled",
        parent
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("skills")
    ));
    // The acceptance calls the destination the `-disabled` sibling of the skill
    // directory itself when the skill dir is the thing being disabled.
    let dest = dir.with_file_name(format!("{name}-disabled"));
    if dest.exists() || dest_root.join(name).exists() {
        return Err(anyhow!("disabled sibling already exists"));
    }
    fs::rename(dir, &dest)?;
    Ok(dest)
}

pub fn worst_verdict(verdicts: &[SyncVerdict]) -> SyncVerdict {
    verdicts
        .iter()
        .copied()
        .max()
        .unwrap_or(SyncVerdict::Missing)
}

pub fn compare_bodies(center: &str, project: &str) -> SyncVerdict {
    let center_norm = normalize_newlines(center);
    let project_norm = normalize_newlines(project);
    if center_norm == project_norm {
        SyncVerdict::InSync
    } else if center_norm != project_norm && !center.is_empty() && !project.is_empty() {
        SyncVerdict::Diverged
    } else if project.is_empty() {
        SyncVerdict::CenterChanged
    } else {
        SyncVerdict::ProjectChanged
    }
}

pub fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// "Update Center" writes one body back to the library. Two different on-disk
/// variants are a conflict and nothing is written.
pub fn update_center(variants: &[&str]) -> Result<String> {
    let mut distinct = BTreeSet::new();
    for variant in variants {
        distinct.insert(normalize_newlines(variant));
    }
    if distinct.len() > 1 {
        return Err(anyhow!("Update Center refused: two diverging variants"));
    }
    Ok(distinct.into_iter().next().unwrap_or_default())
}

pub fn dot_color(verdict: SyncVerdict) -> &'static str {
    match verdict {
        SyncVerdict::InSync => "green",
        SyncVerdict::LocalOnly | SyncVerdict::Missing => "grey",
        SyncVerdict::CenterChanged | SyncVerdict::ProjectChanged => "amber",
        SyncVerdict::Diverged => "red",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_skill_names_collapses_punctuation() {
        assert_eq!(
            slugify_skill_names(&["Review Notes".into(), "A/B".into()]),
            vec!["review-notes".to_string(), "a-b".to_string()]
        );
    }

    #[test]
    fn export_refuses_existing_target_in_live_or_disabled_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let live = tmp.path().join("skills");
        fs::create_dir_all(live.join("demo")).unwrap();
        fs::write(live.join("demo").join("SKILL.md"), "live").unwrap();
        let err = export_skill(&live, "demo/SKILL.md", "new").unwrap_err();
        assert_eq!(err.location, "live");
        assert!(err.path.ends_with("SKILL.md"));
        let original = fs::read_to_string(live.join("demo").join("SKILL.md")).unwrap();
        assert_eq!(original, "live");

        let fresh = tmp.path().join("other");
        fs::create_dir_all(&fresh).unwrap();
        let disabled = disabled_sibling(&fresh);
        fs::create_dir_all(disabled.join("demo")).unwrap();
        fs::write(disabled.join("demo").join("SKILL.md"), "old").unwrap();
        let err = export_skill(&fresh, "demo/SKILL.md", "new").unwrap_err();
        assert_eq!(err.location, "disabled");
        assert!(!fresh.join("demo").join("SKILL.md").exists());
    }

    #[test]
    fn dedupes_agents_sharing_a_root() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("proj");
        fs::create_dir_all(&root).unwrap();
        let agents = vec![
            ("claude".into(), root.clone()),
            ("codex".into(), root.clone()),
            ("cursor".into(), tmp.path().join("elsewhere")),
        ];
        fs::create_dir_all(tmp.path().join("elsewhere")).unwrap();
        let kept = dedupe_agents_sharing_root(&agents);
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].0, "claude");
        assert_eq!(kept[1].0, "cursor");
    }

    #[test]
    fn disable_moves_directory_to_disabled_sibling() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("skills").join("demo");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), "body").unwrap();
        let dest = disable_by_moving(&dir).unwrap();
        assert!(!dir.exists());
        assert_eq!(fs::read_to_string(dest.join("SKILL.md")).unwrap(), "body");
        assert!(dest.ends_with("demo-disabled"));
    }

    #[test]
    fn update_center_refuses_two_diverging_variants() {
        let err = update_center(&["alpha\n", "beta\n"]).unwrap_err();
        assert!(err.to_string().contains("two diverging variants"));
        assert_eq!(update_center(&["alpha\r\n", "alpha\n"]).unwrap(), "alpha\n");
    }

    #[test]
    fn relative_paths_reject_parent_segments() {
        assert!(validate_relative_path("../x").is_err());
        assert!(validate_relative_path("/tmp/x").is_err());
        assert!(validate_relative_path("skills/demo/SKILL.md").is_ok());
    }

    #[test]
    fn worst_verdict_picks_diverged() {
        assert_eq!(
            worst_verdict(&[
                SyncVerdict::InSync,
                SyncVerdict::CenterChanged,
                SyncVerdict::Diverged
            ]),
            SyncVerdict::Diverged
        );
        assert_eq!(dot_color(SyncVerdict::Diverged), "red");
        assert_eq!(dot_color(SyncVerdict::InSync), "green");
    }
}
