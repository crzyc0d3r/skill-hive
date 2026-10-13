// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::db::skill_store::{SkillRecord, SkillStore};
use crate::library::skill_metadata::EmptySkillMetadata;
use crate::library::sync_metadata::{ScenarioMetaFile, ScenarioSkillMetaFile};

const SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetaFile {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub source_type: String,
    pub source_ref: Option<String>,
    pub central_dirname: String,
    pub content_hash: Option<String>,
    pub tags: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct ImportReport {
    pub skills: usize,
    pub presets: usize,
    pub tag_assignments: usize,
    pub membership_order: Vec<(String, Vec<String>)>,
}

pub fn sidecar_dir_for(skills_root: &Path) -> PathBuf {
    let modern = skills_root.join(".skill-studio");
    let legacy = skills_root.join(".skills-manager");
    if json_dir_has_files(&modern.join("skills")) {
        return modern;
    }
    if legacy.exists() {
        return legacy;
    }
    modern
}

fn json_dir_has_files(dir: &Path) -> bool {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .any(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
        })
        .unwrap_or(false)
}

/// Read sidecar metadata into the database.
///
/// A library that has skill directories but no skill sidecar files is refused
/// with [`EmptySkillMetadata`]. Call [`write_all_from_db`] afterwards to rebuild
/// the sidecars from the database; the database itself is left unchanged.
pub fn reindex_from_metadata(store: &SkillStore, skills_root: &Path) -> Result<()> {
    let sidecar = sidecar_dir_for(skills_root);
    let skill_files = list_json(&sidecar.join("skills"))?;
    if skill_files.is_empty() && has_real_directories(skills_root)? {
        return Err(anyhow::Error::new(EmptySkillMetadata));
    }

    let mut metas = Vec::new();
    for path in skill_files {
        let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let meta: SkillMetaFile =
            serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        metas.push(meta);
    }

    let incoming: std::collections::HashSet<String> = metas.iter().map(|m| m.id.clone()).collect();
    for existing in store.get_all_skills()? {
        if !incoming.contains(&existing.id) {
            store.delete_skill(&existing.id)?;
        }
    }
    for meta in &metas {
        let central = skills_root.join(&meta.central_dirname);
        let record = skill_from_meta(meta, &central);
        if store.get_skill_by_id(&meta.id)?.is_some() {
            store.delete_skill(&meta.id)?;
        }
        store.insert_skill(&record)?;
        store.set_tags_for_skill(&meta.id, &meta.tags)?;
    }

    let scenarios = read_scenarios(&sidecar.join("scenarios"))?;
    let memberships = read_memberships(&sidecar.join("memberships"))?;
    store.replace_scenarios_from_metadata(&scenarios)?;
    store.replace_scenario_memberships_from_metadata(&memberships)?;
    Ok(())
}

pub fn write_all_from_db(store: &SkillStore, skills_root: &Path) -> Result<()> {
    let sidecar = skills_root.join(".skill-studio");
    let skills_dir = sidecar.join("skills");
    let scenarios_dir = sidecar.join("scenarios");
    let memberships_dir = sidecar.join("memberships");
    fs::create_dir_all(&skills_dir)?;
    fs::create_dir_all(&scenarios_dir)?;
    fs::create_dir_all(&memberships_dir)?;

    let tags = store.get_tags_map()?;
    for skill in store.get_all_skills()? {
        let dirname = Path::new(&skill.central_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| skill.name.clone());
        let meta = SkillMetaFile {
            schema_version: SCHEMA,
            id: skill.id.clone(),
            name: skill.name.clone(),
            description: skill.description.clone(),
            source_type: skill.source_type.clone(),
            source_ref: skill.source_ref.clone(),
            central_dirname: dirname,
            content_hash: skill.content_hash.clone(),
            tags: tags.get(&skill.id).cloned().unwrap_or_default(),
            enabled: skill.enabled,
        };
        fs::write(
            skills_dir.join(format!("{}.json", skill.id)),
            serde_json::to_vec_pretty(&meta)?,
        )?;
    }

    for scenario in store.get_all_scenarios()? {
        let meta = ScenarioMetaFile {
            schema_version: SCHEMA,
            scenario_id: scenario.id.clone(),
            name: scenario.name.clone(),
            description: scenario.description.clone(),
            icon: scenario.icon.clone(),
            sort_order: scenario.sort_order,
        };
        fs::write(
            scenarios_dir.join(format!("{}.json", scenario.id)),
            serde_json::to_vec_pretty(&meta)?,
        )?;
        let ids = store.get_skill_ids_for_scenario(&scenario.id)?;
        for (index, skill_id) in ids.iter().enumerate() {
            let toggles = store.get_scenario_skill_tool_toggles(&scenario.id, skill_id)?;
            let mut tools = std::collections::BTreeMap::new();
            for toggle in toggles {
                tools.insert(toggle.tool, toggle.enabled);
            }
            let member = ScenarioSkillMetaFile {
                schema_version: SCHEMA,
                scenario_id: scenario.id.clone(),
                skill_id: skill_id.clone(),
                sort_order: index as i32,
                tools,
            };
            let dir = memberships_dir.join(&scenario.id);
            fs::create_dir_all(&dir)?;
            fs::write(
                dir.join(format!("{skill_id}.json")),
                serde_json::to_vec_pretty(&member)?,
            )?;
        }
    }
    Ok(())
}

pub fn import_skills_manager_library(
    store: &SkillStore,
    source_skills: &Path,
    dest_skills: &Path,
) -> Result<ImportReport> {
    fs::create_dir_all(dest_skills)?;
    for entry in fs::read_dir(source_skills)? {
        let entry = entry?;
        let name = entry.file_name();
        copy_path(&entry.path(), &dest_skills.join(name))?;
    }
    reindex_from_metadata(store, dest_skills)?;
    report(store)
}

pub fn report(store: &SkillStore) -> Result<ImportReport> {
    let skills = store.get_all_skills()?;
    let presets = store.get_all_scenarios()?;
    let tags = store.get_tags_map()?;
    let tag_assignments = tags.values().map(|v| v.len()).sum();
    let mut membership_order = Vec::new();
    let mut ordered = presets.clone();
    ordered.sort_by_key(|s| s.sort_order);
    for preset in ordered {
        let ids = store.get_skill_ids_for_scenario(&preset.id)?;
        let names = ids
            .iter()
            .filter_map(|id| skills.iter().find(|s| s.id == *id).map(|s| s.name.clone()))
            .collect();
        membership_order.push((preset.name.clone(), names));
    }
    Ok(ImportReport {
        skills: skills.len(),
        presets: presets.len(),
        tag_assignments,
        membership_order,
    })
}

fn skill_from_meta(meta: &SkillMetaFile, central: &Path) -> SkillRecord {
    let now = chrono::Utc::now().timestamp_millis();
    SkillRecord {
        id: meta.id.clone(),
        name: meta.name.clone(),
        description: meta.description.clone(),
        source_type: meta.source_type.clone(),
        source_ref: meta.source_ref.clone(),
        source_ref_resolved: None,
        source_subpath: None,
        source_branch: None,
        source_revision: None,
        remote_revision: None,
        central_path: central.to_string_lossy().to_string(),
        content_hash: meta.content_hash.clone(),
        enabled: meta.enabled,
        created_at: now,
        updated_at: now,
        status: "ok".into(),
        update_status: "unknown".into(),
        last_checked_at: None,
        last_check_error: None,
    }
}

fn has_real_directories(skills_root: &Path) -> Result<bool> {
    if !skills_root.exists() {
        return Ok(false);
    }
    for entry in fs::read_dir(skills_root)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        if entry.path().is_dir() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn list_json(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

fn read_scenarios(dir: &Path) -> Result<Vec<ScenarioMetaFile>> {
    let mut out = Vec::new();
    for path in list_json(dir)? {
        let text = fs::read_to_string(&path)?;
        out.push(serde_json::from_str(&text)?);
    }
    Ok(out)
}

fn read_memberships(dir: &Path) -> Result<Vec<ScenarioSkillMetaFile>> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            for file in list_json(&path)? {
                let text = fs::read_to_string(&file)?;
                out.push(serde_json::from_str(&text)?);
            }
        }
    }
    Ok(out)
}

fn copy_path(from: &Path, to: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(from)?;
    if meta.is_dir() {
        fs::create_dir_all(to)?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            copy_path(&entry.path(), &to.join(entry.file_name()))?;
        }
        return Ok(());
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(from, to)?;
    Ok(())
}
