// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

use anyhow::{anyhow, Result};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::db::skill_store::SkillStore;
use crate::deploy::sync_engine::{
    deploy_skill, link_points_at, remove_deployed_link, NativeBackend, OsKind, SyncMode,
};
use crate::library::content_hash::hash_skill_dir;
use crate::library::identity::{self, SkillInstance};
use crate::library::installer::install_local;
use crate::library::skill_metadata::read_skill_document;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceStatus {
    LocalOnly,
    InSync,
    CenterChanged,
    Missing,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceSkill {
    pub name: String,
    pub path: String,
    pub locations: Vec<String>,
    pub skill_id: Option<String>,
    pub status: WorkspaceStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Adopt {
    UserConfirmed,
}

pub fn list_agent_skills(
    store: &SkillStore,
    tool: &str,
    agent_skills: &Path,
) -> Result<Vec<WorkspaceSkill>> {
    list_agent_skills_in(store, tool, &[agent_skills.to_path_buf()])
}

/// One card per skill name across every root. Copies and symlinks/junctions group together.
pub fn list_agent_skills_in(
    store: &SkillStore,
    tool: &str,
    roots: &[PathBuf],
) -> Result<Vec<WorkspaceSkill>> {
    let managed = store.get_all_skills()?;
    let mut rows = Vec::new();
    let mut seen_names = Vec::new();
    for group in identity::group_instances(roots) {
        let name = group[0].name.clone();
        seen_names.push(name.to_ascii_lowercase());
        let locations: Vec<String> = group
            .iter()
            .map(|hit| hit.path.to_string_lossy().to_string())
            .collect();
        let matched = managed.iter().find(|skill| {
            skill.name.eq_ignore_ascii_case(&name)
                || group.iter().any(|hit| {
                    link_points_at(&hit.path, Path::new(&skill.central_path))
                        || identity::canonical_dir(&hit.path)
                            == identity::canonical_dir(Path::new(&skill.central_path))
                })
        });
        let status = status_for_group(matched.map(|skill| skill.central_path.as_str()), &group);
        rows.push(WorkspaceSkill {
            name,
            path: locations
                .first()
                .cloned()
                .unwrap_or_default(),
            locations,
            skill_id: matched.map(|skill| skill.id.clone()),
            status,
        });
    }
    let anchor = roots.first().cloned().unwrap_or_default();
    for skill in &managed {
        if seen_names.iter().any(|name| name == &skill.name.to_ascii_lowercase()) {
            continue;
        }
        let deployed = store
            .get_targets_for_skill(&skill.id)?
            .into_iter()
            .any(|target| target.tool == tool);
        if !deployed {
            continue;
        }
        let path = anchor.join(&skill.name).to_string_lossy().to_string();
        rows.push(WorkspaceSkill {
            name: skill.name.clone(),
            path: path.clone(),
            locations: vec![path],
            skill_id: Some(skill.id.clone()),
            status: WorkspaceStatus::Missing,
        });
    }
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(rows)
}

fn status_for_group(central: Option<&str>, group: &[SkillInstance]) -> WorkspaceStatus {
    let Some(central) = central else {
        return WorkspaceStatus::LocalOnly;
    };
    let center_path = Path::new(central);
    let mut saw_sync = false;
    for hit in group {
        if link_points_at(&hit.path, center_path)
            || identity::canonical_dir(&hit.path) == identity::canonical_dir(center_path)
        {
            saw_sync = true;
            continue;
        }
        let center = hash_skill_dir(center_path).ok();
        let agent = hash_skill_dir(&hit.path).ok();
        if center.is_some() && center == agent {
            saw_sync = true;
        } else {
            return WorkspaceStatus::CenterChanged;
        }
    }
    if saw_sync {
        WorkspaceStatus::InSync
    } else {
        WorkspaceStatus::LocalOnly
    }
}

pub fn import_agent_local_skill_to_center(
    store: &SkillStore,
    skills_root: &Path,
    disk_path: &Path,
    confirm: Option<Adopt>,
) -> Result<String> {
    if confirm != Some(Adopt::UserConfirmed) {
        return Err(anyhow!(
            "UserConfirmed required to adopt an existing directory"
        ));
    }
    let doc = read_skill_document(disk_path)?;
    let installed = install_local(store, skills_root, disk_path)?;
    if disk_path.exists() {
        let meta = fs::symlink_metadata(disk_path)?;
        if meta.file_type().is_symlink() {
            remove_deployed_link(disk_path)?;
        } else if meta.is_dir() {
            fs::remove_dir_all(disk_path)?;
        }
    }
    deploy_skill(
        Path::new(&installed.record.central_path),
        disk_path,
        SyncMode::Symlink,
        if cfg!(windows) {
            OsKind::Windows
        } else {
            OsKind::Unix
        },
        &NativeBackend,
    )
    .map_err(|err| anyhow!(err.to_string()))?;
    let _ = doc;
    Ok(installed.record.id)
}

/// Recreate a recorded agent target whose path is missing or whose symlink is broken.
pub fn repair_stranded_targets(
    store: &SkillStore,
    tool: &str,
    agent_skills: &Path,
) -> Result<usize> {
    let _ = agent_skills;
    let mut repaired = 0usize;
    let os = if cfg!(windows) {
        OsKind::Windows
    } else {
        OsKind::Unix
    };
    for skill in store.get_all_skills()? {
        let central = PathBuf::from(&skill.central_path);
        if !central.is_dir() {
            continue;
        }
        for target in store.get_targets_for_skill(&skill.id)? {
            if target.tool != tool || !target_is_stranded(Path::new(&target.target_path)) {
                continue;
            }
            let path = PathBuf::from(&target.target_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            deploy_skill(&central, &path, SyncMode::Symlink, os, &NativeBackend)
                .map_err(|err| anyhow!(err.to_string()))?;
            repaired += 1;
        }
    }
    Ok(repaired)
}

fn target_is_stranded(path: &Path) -> bool {
    match fs::symlink_metadata(path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => true,
        Ok(meta) if meta.file_type().is_symlink() => fs::read_link(path)
            .ok()
            .map(|target| {
                let absolute = if target.is_absolute() {
                    target
                } else {
                    path.parent().unwrap_or(path).join(target)
                };
                !absolute.exists()
            })
            .unwrap_or(true),
        _ => false,
    }
}

pub fn pull_center_to_agent(
    store: &SkillStore,
    skill_id: &str,
    tool: &str,
    agent_skills: &Path,
    mode: SyncMode,
) -> Result<WorkspaceStatus> {
    let skill = store
        .get_skill_by_id(skill_id)?
        .ok_or_else(|| anyhow!("skill not found"))?;
    let target = agent_skills.join(&skill.name);
    if mode == SyncMode::Copy {
        let owned = store
            .get_targets_for_skill(skill_id)?
            .into_iter()
            .any(|row| {
                row.tool == tool && row.mode == "copy" && Path::new(&row.target_path) == target
            });
        if owned && target.exists() {
            fs::remove_dir_all(&target)?;
        }
    }
    deploy_skill(
        Path::new(&skill.central_path),
        &target,
        mode,
        if cfg!(windows) {
            OsKind::Windows
        } else {
            OsKind::Unix
        },
        &NativeBackend,
    )
    .map_err(|err| anyhow!(err.to_string()))?;
    let fresh = hash_skill_dir(Path::new(&skill.central_path))?;
    store.update_skill_after_install(
        skill_id,
        &skill.name,
        skill.description.as_deref(),
        None,
        None,
        Some(&fresh),
        "current",
    )?;
    Ok(WorkspaceStatus::InSync)
}

/// Deleting a managed library skill from the agent page is refused. The
/// directory at the target is left in place.
pub fn delete_managed_target(store: &SkillStore, skill_id: &str, target: &Path) -> Result<()> {
    if store.get_skill_by_id(skill_id)?.is_some() {
        let before = fs::read_dir(target).ok().map(|iter| iter.count());
        return Err(anyhow!(
            "refusing to delete managed skill {skill_id} at {}; content was left untouched ({before:?})",
            target.display()
        ));
    }
    Ok(())
}

pub fn center_changed_after_edit(
    store: &SkillStore,
    skill_id: &str,
    agent_copy: &Path,
) -> Result<bool> {
    let skill = store
        .get_skill_by_id(skill_id)?
        .ok_or_else(|| anyhow!("skill not found"))?;
    let center = hash_skill_dir(Path::new(&skill.central_path))?;
    let agent = hash_skill_dir(agent_copy)?;
    Ok(center != agent)
}

pub fn ensure_dir(path: &Path) -> std::io::Result<PathBuf> {
    fs::create_dir_all(path)?;
    Ok(path.to_path_buf())
}
