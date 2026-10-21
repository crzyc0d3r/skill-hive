// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

use anyhow::Result;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::db::skill_store::SkillStore;
use crate::library::content_hash::hash_skill_dir;
use crate::library::installer::install_local;
use crate::library::skill_metadata::{find_skill_dirs, read_skill_document};

#[derive(Debug, Clone, Serialize)]
pub struct ScannedSkill {
    pub tool: String,
    pub path: String,
    pub name: String,
    pub fingerprint: String,
    pub already_managed: bool,
}

pub fn scan_roots(store: &SkillStore, roots: &[(&str, &Path)]) -> Result<Vec<ScannedSkill>> {
    scan_roots_with_central(store, roots, None)
}

/// Flat children, nested `SKILL.md` trees, and symlinks that already point into
/// the central library. A central symlink is managed even when its hash is new.
pub fn scan_roots_with_central(
    store: &SkillStore,
    roots: &[(&str, &Path)],
    central: Option<&Path>,
) -> Result<Vec<ScannedSkill>> {
    let managed: std::collections::HashSet<String> = store
        .get_all_skills()?
        .into_iter()
        .filter_map(|s| s.content_hash)
        .collect();
    let home = central.and_then(|path| path.canonicalize().ok());
    let mut found = Vec::new();
    for (tool, root) in roots {
        if !root.exists() {
            continue;
        }
        let mut dirs = collect_flat(root);
        dirs.extend(collect_recursive(root));
        dirs.extend(collect_central_symlinks(root, home.as_deref()));
        dirs.sort();
        dirs.dedup();
        for dir in dirs {
            let doc = read_skill_document(&dir)?;
            let fingerprint = hash_skill_dir(&dir).unwrap_or_default();
            let linked = home
                .as_ref()
                .is_some_and(|central_dir| symlink_points_inside(&dir, central_dir));
            found.push(ScannedSkill {
                tool: (*tool).to_string(),
                path: dir.to_string_lossy().to_string(),
                name: doc.name,
                fingerprint: fingerprint.clone(),
                already_managed: managed.contains(&fingerprint) || linked,
            });
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    let mut grouped = Vec::new();
    for skill in found {
        if let Some(existing) = grouped
            .iter_mut()
            .find(|row: &&mut ScannedSkill| row.name.eq_ignore_ascii_case(&skill.name))
        {
            if skill.already_managed {
                existing.already_managed = true;
            }
            continue;
        }
        grouped.push(skill);
    }
    Ok(grouped)
}

pub fn collect_flat(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.join("SKILL.md").is_file() {
            found.push(path);
        }
    }
    found.sort();
    found
}

pub fn collect_recursive(root: &Path) -> Vec<PathBuf> {
    find_skill_dirs(root, 6)
}

pub fn collect_central_symlinks(root: &Path, central: Option<&Path>) -> Vec<PathBuf> {
    let Some(central) = central else {
        return Vec::new();
    };
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if symlink_points_inside(&path, central) {
            found.push(path);
        }
    }
    found.sort();
    found
}

fn symlink_points_inside(path: &Path, central: &Path) -> bool {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return false;
    };
    if !meta.file_type().is_symlink() {
        return false;
    }
    let Ok(target) = fs::read_link(path) else {
        return false;
    };
    let absolute = if target.is_absolute() {
        target
    } else {
        path.parent().unwrap_or(path).join(target)
    };
    absolute
        .canonicalize()
        .ok()
        .is_some_and(|real| real.starts_with(central))
}

pub fn import_scanned(
    store: &SkillStore,
    skills_root: &Path,
    scanned: &ScannedSkill,
    rename: Option<&str>,
) -> Result<String> {
    let installed = install_local(store, skills_root, Path::new(&scanned.path))?;
    if let Some(name) = rename {
        let name = crate::library::skill_metadata::sanitize_skill_name(name);
        store.update_skill_after_install(
            &installed.record.id,
            &name,
            installed.record.description.as_deref(),
            None,
            None,
            installed.record.content_hash.as_deref(),
            "unknown",
        )?;
        return Ok(name);
    }
    Ok(installed.record.name)
}

pub fn import_all(
    store: &SkillStore,
    skills_root: &Path,
    scanned: &[ScannedSkill],
) -> Result<Vec<String>> {
    let mut names = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for skill in scanned.iter().filter(|s| !s.already_managed) {
        if !seen.insert(skill.name.to_ascii_lowercase()) {
            continue;
        }
        names.push(import_scanned(store, skills_root, skill, None)?);
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::skill_store::SkillStore;

    fn write_skill(dir: &Path, name: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: d\n---\n{name}\n"),
        )
        .unwrap();
    }

    #[test]
    fn flat_recursive_and_central_symlink_collectors_split_the_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("agent");
        write_skill(&root.join("top"), "top");
        write_skill(&root.join("nested").join("deep"), "deep");
        let central = tmp.path().join("central");
        write_skill(&central.join("linked"), "linked");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(central.join("linked"), root.join("linked")).unwrap();
        }
        assert_eq!(collect_flat(&root).len(), if cfg!(unix) { 2 } else { 1 });
        assert!(collect_recursive(&root)
            .iter()
            .any(|path| path.ends_with("deep")));
        let store = SkillStore::new(&tmp.path().join("hive.db")).unwrap();
        let rows = scan_roots_with_central(
            &store,
            &[("codex", root.as_path())],
            Some(central.as_path()),
        )
        .unwrap();
        assert!(rows
            .iter()
            .any(|row| row.name == "top" && !row.already_managed));
        assert!(rows.iter().any(|row| row.name == "deep"));
        #[cfg(unix)]
        assert!(rows
            .iter()
            .any(|row| row.name == "linked" && row.already_managed));
        let imported = import_all(&store, &tmp.path().join("library"), &rows).unwrap();
        assert!(imported.iter().any(|name| name == "top"));
        #[cfg(unix)]
        assert!(!imported.iter().any(|name| name == "linked"));
        assert!(tmp
            .path()
            .join("library")
            .join("top")
            .join("SKILL.md")
            .is_file());
    }
}
