//! Project workspace commands. Scanning and copy use the ported engines.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use hive_core::core::installer::{self, InstallResult};
use hive_core::core::project_scanner::{self, AgentSkillConfig, ProjectSkillInfo};
use hive_core::core::skill_metadata::sanitize_skill_name;
use hive_core::core::sync_engine::{self, ReplacePolicy, SyncMode};
use hive_core::db::skill_store::{ProjectRecord, SkillRecord, SkillStore};
use hive_core::deploy::tool_adapters::all_tool_adapters;
use hive_core::library::skill_metadata::read_skill_document;
use hive_core::util::error::AppError;
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

fn configs(store: &SkillStore) -> Vec<AgentSkillConfig> {
    all_tool_adapters(store)
        .into_iter()
        .map(|adapter| {
            let relative_skills_dir = adapter.project_relative_skills_dir().to_string();
            AgentSkillConfig {
                key: adapter.key,
                display_name: adapter.display_name,
                relative_skills_dir,
            }
        })
        .collect()
}

fn project(store: &SkillStore, id: &str) -> Result<ProjectRecord, AppError> {
    store
        .get_project_by_id(id)
        .map_err(db)?
        .ok_or_else(|| AppError::not_found("project not found"))
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

fn skill_in_project<'a>(
    skills: &'a [ProjectSkillInfo],
    name: &str,
) -> Result<&'a ProjectSkillInfo, AppError> {
    skills
        .iter()
        .find(|skill| skill.name == name || skill.dir_name == name || skill.path == name)
        .ok_or_else(|| AppError::not_found("project skill not found"))
}

fn open_path(path: &Path) -> Result<(), AppError> {
    #[cfg(target_os = "windows")]
    let mut command = std::process::Command::new("explorer");
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = std::process::Command::new("xdg-open");
    command.arg(path).spawn().map_err(AppError::io)?;
    Ok(())
}

#[tauri::command]
pub async fn get_projects(
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<ProjectRecord>, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || store.as_ref().get_all_projects().map_err(db)).await
}

#[tauri::command]
pub async fn add_linked_workspace(
    name: String,
    path: String,
    agent: Option<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let path = PathBuf::from(path.trim());
        if !path.is_dir() {
            return Err(AppError::invalid_input("Choose a workspace folder"));
        }
        let stamp = now();
        let id = format!("project-{stamp:x}");
        let label = if name.trim().is_empty() {
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| "workspace".into())
        } else {
            name.trim().to_string()
        };
        store
            .insert_project(&ProjectRecord {
                id: id.clone(),
                name: label,
                path: path.to_string_lossy().to_string(),
                workspace_type: "linked".into(),
                linked_agent_key: agent.clone(),
                linked_agent_name: agent,
                disabled_path: None,
                sort_order: 0,
                created_at: stamp,
                updated_at: stamp,
            })
            .map_err(db)?;
        Ok(id)
    })
    .await
}

#[tauri::command]
pub async fn remove_project(id: String, store: State<'_, Arc<SkillStore>>) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || store.as_ref().delete_project(id.trim()).map_err(db)).await
}

#[tauri::command]
pub async fn reveal_project_folder(
    id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let row = project(store.as_ref(), id.trim())?;
        open_path(Path::new(&row.path))
    })
    .await
}

#[tauri::command]
pub async fn reorder_projects(
    ids: Vec<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || store.as_ref().reorder_projects(&ids).map_err(db)).await
}

#[tauri::command]
pub async fn scan_projects(
    path: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<String>, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let root = PathBuf::from(path.trim());
        if !root.is_dir() {
            return Err(AppError::invalid_input("Choose a folder to scan"));
        }
        Ok(project_scanner::scan_projects_in_dir(
            &root,
            3,
            &configs(store.as_ref()),
        ))
    })
    .await
}

#[tauri::command]
pub async fn get_project_agent_targets(
    id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<String>, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let row = project(store.as_ref(), id.trim())?;
        let root = PathBuf::from(&row.path);
        Ok(configs(store.as_ref())
            .into_iter()
            .map(|config| root.join(&config.relative_skills_dir))
            .filter(|path| path.is_dir())
            .map(|path| path.to_string_lossy().to_string())
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn get_project_skills(
    id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<ProjectSkillInfo>, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
        let row = project(store, id.trim())?;
        Ok(project_scanner::read_project_skills(
            Path::new(&row.path),
            &configs(store),
        ))
    })
    .await
}

#[tauri::command]
pub async fn get_project_skill_document(
    id: String,
    name: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
        let row = project(store, id.trim())?;
        let skills = project_scanner::read_project_skills(Path::new(&row.path), &configs(store));
        let skill = skill_in_project(&skills, name.trim())?;
        let doc = read_skill_document(Path::new(&skill.path)).map_err(AppError::io)?;
        Ok(doc.body)
    })
    .await
}

#[tauri::command]
pub async fn import_project_skill_to_center(
    id: String,
    name: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
        let row = project(store, id.trim())?;
        let skills = project_scanner::read_project_skills(Path::new(&row.path), &configs(store));
        let skill = skill_in_project(&skills, name.trim())?;
        let source = PathBuf::from(&skill.path);
        let installed = installer::install_from_local(&source, Some(&skill.name))
            .map_err(|err| AppError::io(err.to_string()))?;
        persist(store, &installed, &source)
    })
    .await
}

#[tauri::command]
pub async fn update_project_skill_to_center(
    id: String,
    name: String,
    skill_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
        let row = project(store, id.trim())?;
        let skills = project_scanner::read_project_skills(Path::new(&row.path), &configs(store));
        let found = skill_in_project(&skills, name.trim())?;
        let managed = store
            .get_skill_by_id(skill_id.trim())
            .map_err(db)?
            .ok_or_else(|| AppError::not_found("Skill not found"))?;
        let installed = installer::install_from_local_to_destination(
            Path::new(&found.path),
            Some(&managed.name),
            Path::new(&managed.central_path),
        )
        .map_err(|err| AppError::io(err.to_string()))?;
        store
            .update_skill_after_install(
                &managed.id,
                &installed.name,
                installed.description.as_deref(),
                None,
                None,
                Some(&installed.content_hash),
                "current",
            )
            .map_err(db)
    })
    .await
}

#[tauri::command]
pub fn slugify_skill_names(names: Vec<String>) -> Vec<String> {
    names
        .into_iter()
        .map(|name| sanitize_skill_name(&name).unwrap_or_else(|| "skill".into()))
        .collect()
}

#[tauri::command]
pub async fn export_skill_to_project(
    id: String,
    skill_id: String,
    agent: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
        let row = project(store, id.trim())?;
        let skill = store
            .get_skill_by_id(skill_id.trim())
            .map_err(db)?
            .ok_or_else(|| AppError::not_found("Skill not found"))?;
        let config = configs(store)
            .into_iter()
            .find(|config| config.key == agent.trim())
            .ok_or_else(|| AppError::not_found("unknown agent"))?;
        let dest = PathBuf::from(&row.path)
            .join(&config.relative_skills_dir)
            .join(&skill.name);
        sync_engine::sync_skill(
            Path::new(&skill.central_path),
            &dest,
            SyncMode::Copy,
            ReplacePolicy::UserConfirmed,
        )
        .map_err(|err| AppError::io(err.to_string()))?;
        Ok(dest.to_string_lossy().to_string())
    })
    .await
}

#[tauri::command]
pub async fn update_project_skill_from_center(
    id: String,
    skill_id: String,
    agent: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
        let row = project(store, id.trim())?;
        let skill = store
            .get_skill_by_id(skill_id.trim())
            .map_err(db)?
            .ok_or_else(|| AppError::not_found("Skill not found"))?;
        let config = configs(store)
            .into_iter()
            .find(|config| config.key == agent.trim())
            .ok_or_else(|| AppError::not_found("unknown agent"))?;
        let dest = PathBuf::from(&row.path)
            .join(&config.relative_skills_dir)
            .join(&skill.name);
        sync_engine::sync_skill(
            Path::new(&skill.central_path),
            &dest,
            SyncMode::Copy,
            ReplacePolicy::UserConfirmed,
        )
        .map_err(|err| AppError::io(err.to_string()))?;
        Ok(dest.to_string_lossy().to_string())
    })
    .await
}

#[tauri::command]
pub async fn toggle_project_skill(
    id: String,
    name: String,
    enabled: bool,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
        let row = project(store, id.trim())?;
        let skills = project_scanner::read_project_skills(Path::new(&row.path), &configs(store));
        let skill = skill_in_project(&skills, name.trim())?;
        let current = PathBuf::from(&skill.path);
        let parent = current
            .parent()
            .ok_or_else(|| AppError::invalid_input("skill path has no parent"))?;
        let folder_name = parent
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        let project_root = parent
            .parent()
            .ok_or_else(|| AppError::invalid_input("skill path is too short"))?;
        let enabled_name = folder_name.trim_end_matches("-disabled").to_string();
        let target_parent = if enabled {
            project_root.join(&enabled_name)
        } else {
            project_root.join(format!("{enabled_name}-disabled"))
        };
        std::fs::create_dir_all(&target_parent)?;
        let dest = target_parent.join(&skill.dir_name);
        if current != dest {
            if dest.exists() {
                std::fs::remove_dir_all(&dest)?;
            }
            std::fs::rename(&current, &dest).map_err(AppError::io)?;
        }
        Ok(dest.to_string_lossy().to_string())
    })
    .await
}

#[tauri::command]
pub async fn delete_project_skill(
    id: String,
    name: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
        let row = project(store, id.trim())?;
        let skills = project_scanner::read_project_skills(Path::new(&row.path), &configs(store));
        let skill = skill_in_project(&skills, name.trim())?;
        let path = PathBuf::from(&skill.path);
        let root = PathBuf::from(&row.path);
        if !path.starts_with(&root) {
            return Err(AppError::invalid_input("skill path is outside the project"));
        }
        if path.exists() {
            std::fs::remove_dir_all(&path).map_err(AppError::io)?;
        }
        Ok(())
    })
    .await
}
