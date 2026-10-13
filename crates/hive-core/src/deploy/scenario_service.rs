// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::db::skill_store::{SkillStore, SkillTargetRecord};
use crate::deploy::sync_engine::{
    deploy_skill, remove_copy_target, undeploy_target, LinkBackend, NativeBackend, OsKind, SyncMode,
};
use crate::util::error::AppError;

#[derive(Debug, Clone)]
pub struct DesiredTarget {
    pub skill_id: String,
    pub tool: String,
    pub source: PathBuf,
    pub target: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeployIntent {
    Add,
    Remove,
}

pub fn desired_targets(
    store: &SkillStore,
    scenario_id: &str,
    roots: &HashMap<String, PathBuf>,
) -> Result<Vec<DesiredTarget>> {
    let mut desired = Vec::new();
    for skill_id in store.get_skill_ids_for_scenario(scenario_id)? {
        let Some(skill) = store.get_skill_by_id(&skill_id)? else {
            continue;
        };
        let tools = store.get_enabled_tools_for_scenario_skill(scenario_id, &skill_id)?;
        for tool in tools {
            let Some(root) = roots.get(&tool) else {
                continue;
            };
            desired.push(DesiredTarget {
                skill_id: skill_id.clone(),
                tool: tool.clone(),
                source: PathBuf::from(&skill.central_path),
                target: root.join(&skill.name),
            });
        }
    }
    Ok(desired)
}

/// Exclusive apply: the preset's desired set replaces what is deployed.
/// Targets still claimed by another desired row (shared amp/replit or
/// cline/warp directories) are kept on disk.
pub fn apply_preset(
    store: &SkillStore,
    scenario_id: &str,
    roots: &HashMap<String, PathBuf>,
    mode: SyncMode,
    os: OsKind,
) -> Result<usize> {
    let desired = desired_targets(store, scenario_id, roots)?;
    let claimed = claimed_paths(&desired);
    let current = store.get_all_targets()?;
    for target in &current {
        let still = desired
            .iter()
            .any(|d| d.skill_id == target.skill_id && d.tool == target.tool);
        if still {
            continue;
        }
        let path = PathBuf::from(&target.target_path);
        let keep = claimed.contains(&normalise(&path));
        if target.mode == "copy" && !keep {
            remove_copy_target(&path).map_err(|e| anyhow!(e.to_string()))?;
        } else {
            undeploy_target(&path, keep).map_err(|e| anyhow!(e.to_string()))?;
        }
        store.delete_target(&target.skill_id, &target.tool)?;
    }
    let backend = NativeBackend;
    let mut applied = 0;
    for item in &desired {
        match deploy_skill(&item.source, &item.target, mode, os, &backend) {
            Ok(outcome) => {
                upsert_target(store, item, outcome.mode)?;
                applied += 1;
            }
            Err(err) => return Err(anyhow!(err.to_string())),
        }
    }
    store.set_active_scenario(scenario_id)?;
    Ok(applied)
}

pub struct SyncOne {
    pub skill_id: String,
    pub tool: String,
    pub root: PathBuf,
    pub intent: DeployIntent,
    pub mode: SyncMode,
    pub os: OsKind,
}

pub fn sync_one(
    store: &SkillStore,
    request: &SyncOne,
    backend: &dyn LinkBackend,
) -> Result<(), AppError> {
    let skill = store
        .get_skill_by_id(&request.skill_id)
        .map_err(AppError::db)?
        .ok_or_else(|| AppError::not_found("skill not found"))?;
    let target = request.root.join(&skill.name);
    match request.intent {
        DeployIntent::Add => {
            let outcome = deploy_skill(
                Path::new(&skill.central_path),
                &target,
                request.mode,
                request.os,
                backend,
            )?;
            let item = DesiredTarget {
                skill_id: request.skill_id.clone(),
                tool: request.tool.clone(),
                source: PathBuf::from(&skill.central_path),
                target: target.clone(),
            };
            upsert_target(store, &item, outcome.mode).map_err(AppError::db)?;
        }
        DeployIntent::Remove => {
            let others = store.get_all_targets().map_err(AppError::db)?;
            let keep = others.iter().any(|row| {
                row.tool != request.tool
                    && normalise(&PathBuf::from(&row.target_path)) == normalise(&target)
            });
            if request.mode == SyncMode::Copy && !keep {
                remove_copy_target(&target)?;
            } else {
                undeploy_target(&target, keep)?;
            }
            store
                .delete_target(&request.skill_id, &request.tool)
                .map_err(AppError::db)?;
        }
    }
    Ok(())
}

fn claimed_paths(desired: &[DesiredTarget]) -> HashSet<String> {
    desired.iter().map(|d| normalise(&d.target)).collect()
}

fn normalise(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn upsert_target(store: &SkillStore, item: &DesiredTarget, mode: &str) -> Result<()> {
    if store
        .get_targets_for_skill(&item.skill_id)?
        .iter()
        .any(|t| t.tool == item.tool)
    {
        store.delete_target(&item.skill_id, &item.tool)?;
    }
    let now = chrono::Utc::now().timestamp_millis();
    store.insert_target(&SkillTargetRecord {
        id: format!("{}:{}", item.skill_id, item.tool),
        skill_id: item.skill_id.clone(),
        tool: item.tool.clone(),
        target_path: item.target.to_string_lossy().to_string(),
        mode: mode.to_string(),
        status: "ok".into(),
        synced_at: Some(now),
        last_error: None,
        source_hash: None,
    })?;
    Ok(())
}

/// Membership edits record the preset only. They do not deploy.
pub fn add_membership(
    store: &SkillStore,
    scenario_id: &str,
    skill_id: &str,
    tools: &[String],
) -> Result<()> {
    store.add_skill_to_scenario(scenario_id, skill_id)?;
    store.ensure_scenario_skill_tool_defaults(scenario_id, skill_id, tools)?;
    Ok(())
}
