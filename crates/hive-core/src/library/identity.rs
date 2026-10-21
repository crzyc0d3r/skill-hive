use std::fs;
use std::path::{Path, PathBuf};

use super::skill_metadata::read_skill_document;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillInstance {
    pub name: String,
    pub path: PathBuf,
    pub canonical: PathBuf,
}

/// Resolve a directory symlink or Windows junction to the directory it names.
pub fn canonical_dir(path: &Path) -> PathBuf {
    let mut current = path.to_path_buf();
    for _ in 0..16 {
        let Ok(meta) = fs::symlink_metadata(&current) else {
            break;
        };
        if !meta.file_type().is_symlink() {
            break;
        }
        let Ok(target) = fs::read_link(&current) else {
            break;
        };
        current = if target.is_absolute() {
            target
        } else {
            current
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .join(target)
        };
    }
    fs::canonicalize(&current).unwrap_or(current)
}

fn looks_like_skill(path: &Path) -> bool {
    path.join("SKILL.md").is_file() || canonical_dir(path).join("SKILL.md").is_file()
}

/// Skill folders directly under `root`, plus one category level, including links.
pub fn discover_skill_dirs(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        if looks_like_skill(&path) {
            found.push(path);
            continue;
        }
        if !path.is_dir() {
            continue;
        }
        let Ok(children) = fs::read_dir(&path) else {
            continue;
        };
        for child in children.flatten() {
            let child_path = child.path();
            if child.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            if looks_like_skill(&child_path) {
                found.push(child_path);
            }
        }
    }
    found
}

pub fn identify(path: &Path) -> Option<SkillInstance> {
    let canonical = canonical_dir(path);
    let doc = read_skill_document(&canonical)
        .or_else(|_| read_skill_document(path))
        .ok()?;
    if doc.name.is_empty() {
        return None;
    }
    Some(SkillInstance {
        name: doc.name,
        path: path.to_path_buf(),
        canonical,
    })
}

/// One group per skill name. Copies and links to the same skill share a group.
pub fn group_instances(roots: &[PathBuf]) -> Vec<Vec<SkillInstance>> {
    let mut hits = Vec::new();
    for root in roots {
        for path in discover_skill_dirs(root) {
            let Some(hit) = identify(&path) else {
                continue;
            };
            if hits.iter().any(|existing: &SkillInstance| existing.path == hit.path) {
                continue;
            }
            hits.push(hit);
        }
    }
    let mut groups: Vec<Vec<SkillInstance>> = Vec::new();
    for hit in hits {
        if let Some(group) = groups
            .iter_mut()
            .find(|group| group[0].name.eq_ignore_ascii_case(&hit.name))
        {
            group.push(hit);
        } else {
            groups.push(vec![hit]);
        }
    }
    groups
}
