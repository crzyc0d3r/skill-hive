// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

use anyhow::Result;
use serde::Serialize;
use std::path::Path;

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
    let managed: std::collections::HashSet<String> = store
        .get_all_skills()?
        .into_iter()
        .filter_map(|s| s.content_hash)
        .collect();
    let mut found = Vec::new();
    for (tool, root) in roots {
        if !root.exists() {
            continue;
        }
        for dir in find_skill_dirs(root, 4) {
            let doc = read_skill_document(&dir)?;
            let fingerprint = hash_skill_dir(&dir).unwrap_or_default();
            found.push(ScannedSkill {
                tool: (*tool).to_string(),
                path: dir.to_string_lossy().to_string(),
                name: doc.name,
                fingerprint: fingerprint.clone(),
                already_managed: managed.contains(&fingerprint),
            });
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(found)
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
    for skill in scanned.iter().filter(|s| !s.already_managed) {
        names.push(import_scanned(store, skills_root, skill, None)?);
    }
    Ok(names)
}
