//! Global agent-workspace commands. Each one reads or writes a real skill directory.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use hive_core::core::installer::{self, InstallResult};
use hive_core::core::sync_engine::{self, ReplacePolicy, SyncMode};
use hive_core::db::skill_store::{SkillRecord, SkillStore};
use hive_core::deploy::tool_adapters::all_tool_adapters;
use hive_core::library::skill_metadata::read_skill_document;
use hive_core::library::workspace::{self, WorkspaceSkill};
use hive_core::util::error::AppError;
use serde::Serialize;
use tauri::State;

fn db(err: impl std::fmt::Display) -> AppError {
    AppError::db(err.to_string())
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or(0)
}

fn agent_dir(store: &SkillStore, tool: &str) -> Result<PathBuf, AppError> {
    all_tool_adapters(store)
        .into_iter()
        .find(|adapter| adapter.key == tool)
        .map(|adapter| adapter.skills_dir())
        .ok_or_else(|| AppError::not_found(format!("unknown agent {tool}")))
}

fn persist(
    store: &SkillStore,
    installed: &InstallResult,
    source: &Path,
) -> Result<String, AppError> {
    let stamp = now();
    let id = format!("{}-{stamp:x}", installed.name);
    store
        .insert_skill(&SkillRecord {
            id: id.clone(),
            name: installed.name.clone(),
            description: installed.description.clone(),
            source_type: "local".into(),
            source_ref: Some(source.to_string_lossy().to_string()),
            source_ref_resolved: None,
            source_subpath: None,
            source_branch: None,
            source_revision: None,
            remote_revision: None,
            central_path: installed.central_path.to_string_lossy().to_string(),
            content_hash: Some(installed.content_hash.clone()),
            enabled: true,
            created_at: stamp,
            updated_at: stamp,
            status: "managed".into(),
            update_status: "current".into(),
            last_checked_at: None,
            last_check_error: None,
        })
        .map_err(db)?;
    Ok(id)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDocumentDto {
    pub name: String,
    pub description: String,
    pub body: String,
    pub path: String,
}

fn document_at(path: &Path) -> Result<SkillDocumentDto, AppError> {
    let doc = read_skill_document(path).map_err(AppError::io)?;
    Ok(SkillDocumentDto {
        name: doc.name,
        description: doc.description.unwrap_or_default(),
        body: doc.body,
        path: path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn get_global_local_skills(
    tool: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<WorkspaceSkill>, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let dir = agent_dir(store.as_ref(), tool.trim())?;
        workspace::list_agent_skills(store.as_ref(), tool.trim(), &dir)
            .map_err(|err| AppError::io(err.to_string()))
    })
    .await
}

#[tauri::command]
pub async fn get_global_local_skill_document(
    tool: String,
    name: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<SkillDocumentDto, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let path = agent_dir(store.as_ref(), tool.trim())?.join(name.trim());
        document_at(&path)
    })
    .await
}

#[tauri::command]
pub async fn import_global_local_skill_to_center(
    tool: String,
    name: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let source = agent_dir(store.as_ref(), tool.trim())?.join(name.trim());
        if !source.join("SKILL.md").is_file() {
            return Err(AppError::not_found("skill folder has no SKILL.md"));
        }
        let installed = installer::install_from_local(&source, None)
            .map_err(|err| AppError::io(err.to_string()))?;
        persist(store.as_ref(), &installed, &source)
    })
    .await
}

#[tauri::command]
pub async fn update_global_local_skill_from_center(
    tool: String,
    skill_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
        let skill = store
            .get_skill_by_id(skill_id.trim())
            .map_err(db)?
            .ok_or_else(|| AppError::not_found("Skill not found"))?;
        let agent = agent_dir(store, tool.trim())?;
        let target = agent.join(&skill.name);
        let source = PathBuf::from(&skill.central_path);
        sync_engine::sync_skill(
            &source,
            &target,
            SyncMode::Copy,
            ReplacePolicy::UserConfirmed,
        )
        .map_err(|err| AppError::io(err.to_string()))?;
        Ok(target.to_string_lossy().to_string())
    })
    .await
}

#[tauri::command]
pub async fn delete_global_local_skill(
    tool: String,
    name: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let path = agent_dir(store.as_ref(), tool.trim())?.join(name.trim());
        if path.exists() {
            if path.is_dir() {
                std::fs::remove_dir_all(&path).map_err(AppError::io)?;
            } else {
                std::fs::remove_file(&path).map_err(AppError::io)?;
            }
        }
        Ok(())
    })
    .await
}
