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
    let mut rows = Vec::new();
    let managed = store.get_all_skills()?;
    if agent_skills.exists() {
        for entry in fs::read_dir(agent_skills)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() && fs::symlink_metadata(&path)?.file_type().is_symlink() {
                // A symlink to a skill directory still counts.
            } else if !path.join("SKILL.md").is_file()
                && !fs::symlink_metadata(&path)?.file_type().is_symlink()
            {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let matched = managed.iter().find(|skill| {
                skill.name == name || link_points_at(&path, Path::new(&skill.central_path))
            });
            let status = match matched {
                None => WorkspaceStatus::LocalOnly,
                Some(skill) => {
                    if link_points_at(&path, Path::new(&skill.central_path)) {
                        WorkspaceStatus::InSync
                    } else {
                        let center = hash_skill_dir(Path::new(&skill.central_path)).ok();
                        let agent = hash_skill_dir(&path).ok();
                        if center.is_some() && center == agent {
                            WorkspaceStatus::InSync
                        } else {
                            WorkspaceStatus::CenterChanged
                        }
                    }
                }
            };
            rows.push(WorkspaceSkill {
                name,
                path: path.to_string_lossy().to_string(),
                skill_id: matched.map(|skill| skill.id.clone()),
                status,
            });
        }
    }
    for skill in &managed {
        let deployed = store
            .get_targets_for_skill(&skill.id)?
            .into_iter()
            .any(|target| target.tool == tool);
        if deployed
            && !rows
                .iter()
                .any(|row| row.skill_id.as_deref() == Some(skill.id.as_str()))
        {
            rows.push(WorkspaceSkill {
                name: skill.name.clone(),
                path: agent_skills.join(&skill.name).to_string_lossy().to_string(),
                skill_id: Some(skill.id.clone()),
                status: WorkspaceStatus::Missing,
            });
        }
    }
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(rows)
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
