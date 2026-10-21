//! Skill, tag, and tool commands that were absent from the desktop shell.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use hive_core::core::git_fetcher::{self, ParsedGitSource};
use hive_core::core::installer::{self, InstallResult};
use hive_core::db::skill_store::{ScenarioSkillToolToggleRecord, SkillRecord, SkillStore};
use hive_core::deploy::tool_adapters::all_tool_adapters;
use hive_core::library::skill_metadata::{find_skill_dirs, read_skill_document};
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

fn persist(
    store: &SkillStore,
    installed: &InstallResult,
    source_type: &str,
    source_ref: Option<String>,
) -> Result<String, AppError> {
    let stamp = now();
    let id = format!("{}-{stamp:x}", installed.name);
    store
        .insert_skill(&SkillRecord {
            id: id.clone(),
            name: installed.name.clone(),
            description: installed.description.clone(),
            source_type: source_type.into(),
            source_ref,
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

fn require(
    store: &SkillStore,
    id: &str,
) -> Result<hive_core::db::skill_store::SkillRecord, AppError> {
    store
        .get_skill_by_id(id)
        .map_err(db)?
        .ok_or_else(|| AppError::not_found("Skill not found"))
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

fn replace_central(store: &SkillStore, id: &str, source: &Path) -> Result<(), AppError> {
    let skill = require(store, id)?;
    let installed = installer::install_from_local_to_destination(
        source,
        Some(&skill.name),
        Path::new(&skill.central_path),
    )
    .map_err(|err| AppError::io(err.to_string()))?;
    store
        .update_skill_after_install(
            &skill.id,
            &installed.name,
            installed.description.as_deref(),
            None,
            None,
            Some(&installed.content_hash),
            "current",
        )
        .map_err(db)
}

fn refresh_git(store: &SkillStore, id: &str) -> Result<(), AppError> {
    let skill = require(store, id)?;
    let source = skill
        .source_ref
        .clone()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::invalid_input("skill has no source"))?;
    if Path::new(&source).is_dir() {
        return replace_central(store, id, Path::new(&source));
    }
    let parsed: ParsedGitSource = git_fetcher::parse_git_source(&source);
    git_fetcher::validate_git_url(&parsed.clone_url)
        .map_err(|err| AppError::invalid_input(err.to_string()))?;
    let repo = git_fetcher::clone_repo_ref(&parsed.clone_url, parsed.branch.as_deref(), None, None)
        .map_err(|err| AppError::git(err.to_string()))?;
    let skill_dir = match parsed.subpath.as_deref() {
        Some(sub) => repo.join(sub),
        None => git_fetcher::find_skill_dir(&repo, None).unwrap_or_else(|_| repo.clone()),
    };
    let result = replace_central(store, id, &skill_dir);
    git_fetcher::cleanup_temp(&repo);
    result
}

#[tauri::command]
pub async fn get_skills_for_preset(
    preset_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<String>, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        store
            .get_skill_ids_for_scenario(preset_id.trim())
            .map_err(db)
    })
    .await
}

#[tauri::command]
pub async fn get_skill_document(
    skill_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<SkillDocumentDto, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let skill = require(store.as_ref(), skill_id.trim())?;
        document_at(Path::new(&skill.central_path))
    })
    .await
}

#[tauri::command]
pub async fn get_source_skill_document(
    skill_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<SkillDocumentDto, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let skill = require(store.as_ref(), skill_id.trim())?;
        let source = skill
            .source_ref
            .filter(|value| Path::new(value).is_dir())
            .ok_or_else(|| AppError::not_found("source folder is not on disk"))?;
        document_at(Path::new(&source))
    })
    .await
}

#[tauri::command]
pub async fn get_skill_source_diff(
    skill_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<String>, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let kept = store.as_ref();
        let current = {
            let skill = require(kept, skill_id.trim())?;
            document_at(Path::new(&skill.central_path))?
        };
        let source = (|| {
            let skill = require(kept, skill_id.trim())?;
            let source = skill
                .source_ref
                .filter(|value| Path::new(value).is_dir())
                .ok_or_else(|| AppError::not_found("source folder is not on disk"))?;
            document_at(Path::new(&source))
        })()
        .unwrap_or(SkillDocumentDto {
            name: current.name.clone(),
            description: String::new(),
            body: String::new(),
            path: String::new(),
        });
        let mut lines = Vec::new();
        if current.body != source.body {
            lines.push(format!(
                "body {} -> {}",
                source.body.len(),
                current.body.len()
            ));
        }
        if current.description != source.description {
            lines.push("description differs".into());
        }
        Ok(lines)
    })
    .await
}

#[tauri::command]
pub async fn delete_managed_skill(
    id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
        if let Some(skill) = store.get_skill_by_id(id.trim()).map_err(db)? {
            for target in store.get_targets_for_skill(&skill.id).map_err(db)? {
                let _ = std::fs::remove_dir_all(Path::new(&target.target_path));
            }
            let central = PathBuf::from(&skill.central_path);
            if central.exists() {
                std::fs::remove_dir_all(&central).map_err(AppError::io)?;
            }
        }
        store.delete_skill(id.trim()).map_err(db)
    })
    .await
}

#[tauri::command]
pub async fn install_local(
    path: String,
    name: Option<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let source = PathBuf::from(path.trim());
        if !source.is_dir() {
            return Err(AppError::invalid_input("Choose a skill folder"));
        }
        let installed = installer::install_from_local(&source, name.as_deref())
            .map_err(|err| AppError::io(err.to_string()))?;
        persist(
            store.as_ref(),
            &installed,
            "local",
            Some(source.to_string_lossy().to_string()),
        )
    })
    .await
}

#[tauri::command]
pub async fn install_git(
    url: String,
    name: Option<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let parsed = git_fetcher::parse_git_source(url.trim());
        git_fetcher::validate_git_url(&parsed.clone_url)
            .map_err(|err| AppError::invalid_input(err.to_string()))?;
        let repo =
            git_fetcher::clone_repo_ref(&parsed.clone_url, parsed.branch.as_deref(), None, None)
                .map_err(|err| AppError::git(err.to_string()))?;
        let skill_dir = match parsed.subpath.as_deref() {
            Some(sub) => repo.join(sub),
            None => git_fetcher::find_skill_dir(&repo, None).unwrap_or_else(|_| repo.clone()),
        };
        let installed = installer::install_from_git_dir(&skill_dir, name.as_deref())
            .map_err(|err| AppError::git(err.to_string()))?;
        let id = persist(store.as_ref(), &installed, "git", Some(parsed.original_url))?;
        git_fetcher::cleanup_temp(&repo);
        Ok(id)
    })
    .await
}

#[tauri::command]
pub async fn confirm_git_install(
    preview_id: String,
    relative: String,
    rename: Option<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<String>, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        crate::commands::confirm_preview(store.as_ref(), &preview_id, &relative, rename)
    })
    .await
}

#[tauri::command]
pub async fn cancel_git_preview(temp_dir: String) -> Result<(), AppError> {
    crate::offload::offload(move || {
        crate::commands::cancel_install_now(Some(temp_dir))?;
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn update_skill(
    skill_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || refresh_git(store.as_ref(), skill_id.trim())).await
}

#[tauri::command]
pub async fn reimport_local_skill(
    skill_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
        let skill = require(store, skill_id.trim())?;
        let source = skill
            .source_ref
            .filter(|value| Path::new(value).exists())
            .ok_or_else(|| AppError::not_found("local source is missing"))?;
        replace_central(store, &skill.id, Path::new(&source))
    })
    .await
}

#[tauri::command]
pub async fn batch_update_skills(
    ids: Vec<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<usize, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
        let mut updated = 0;
        for id in ids {
            refresh_git(store, &id)?;
            updated += 1;
        }
        Ok(updated)
    })
    .await
}

#[tauri::command]
pub async fn relink_local_skill_source(
    skill_id: String,
    path: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let source = PathBuf::from(path.trim());
        if !source.join("SKILL.md").is_file() && !source.is_dir() {
            return Err(AppError::invalid_input("Choose the skill's source folder"));
        }
        let store = store.as_ref();
        let _ = require(store, skill_id.trim())?;
        store
            .update_skill_source_ref(skill_id.trim(), &source.to_string_lossy())
            .map_err(db)?;
        store
            .set_skill_source_type(skill_id.trim(), "local")
            .map_err(db)?;
        replace_central(store, skill_id.trim(), &source)
    })
    .await
}

#[tauri::command]
pub async fn detach_local_skill_source(
    skill_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
        let skill = require(store, skill_id.trim())?;
        store
            .update_skill_source_ref(&skill.id, &skill.central_path)
            .map_err(db)?;
        store
            .set_skill_source_type(&skill.id, "managed")
            .map_err(db)
    })
    .await
}

#[tauri::command]
pub async fn get_all_tags(store: State<'_, Arc<SkillStore>>) -> Result<Vec<String>, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || store.as_ref().get_all_tags().map_err(db)).await
}

#[tauri::command]
pub async fn batch_import_folder(
    path: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<String>, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let folder = PathBuf::from(path.trim());
        if !folder.is_dir() {
            return Err(AppError::invalid_input("Choose a folder of skills"));
        }
        let mut dirs = find_skill_dirs(&folder, 4);
        if dirs.is_empty() && folder.join("SKILL.md").is_file() {
            dirs.push(folder.clone());
        }
        let mut ids = Vec::new();
        for dir in dirs {
            let installed = installer::install_from_local(&dir, None)
                .map_err(|err| AppError::io(err.to_string()))?;
            ids.push(persist(
                store.as_ref(),
                &installed,
                "local",
                Some(dir.to_string_lossy().to_string()),
            )?);
        }
        Ok(ids)
    })
    .await
}

#[tauri::command]
pub async fn get_skill_tool_toggles(
    skill_id: String,
    preset_id: Option<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<ScenarioSkillToolToggleRecord>, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
        let scenario = match preset_id.filter(|id| !id.is_empty()) {
            Some(id) => id,
            None => store
                .get_active_scenario_id()
                .map_err(db)?
                .unwrap_or_default(),
        };
        if scenario.is_empty() {
            return Ok(Vec::new());
        }
        store
            .get_scenario_skill_tool_toggles(&scenario, skill_id.trim())
            .map_err(db)
    })
    .await
}

#[tauri::command]
pub async fn set_skill_tool_toggle(
    skill_id: String,
    tool: String,
    enabled: bool,
    preset_id: Option<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
        let scenario = match preset_id.filter(|id| !id.is_empty()) {
            Some(id) => id,
            None => store
                .get_active_scenario_id()
                .map_err(db)?
                .ok_or_else(|| AppError::invalid_input("No active preset"))?,
        };
        store
            .set_scenario_skill_tool_enabled(&scenario, skill_id.trim(), tool.trim(), enabled)
            .map_err(db)
    })
    .await
}

#[tauri::command]
pub async fn get_tool_order_cmd(
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<String>, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let raw = store
            .get_setting("tool_order")
            .map_err(db)?
            .unwrap_or_else(|| "[]".into());
        let saved: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
        if !saved.is_empty() {
            return Ok(saved);
        }
        Ok(all_tool_adapters(store.as_ref())
            .into_iter()
            .map(|adapter| adapter.key)
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn reset_custom_tool_project_path(
    key: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
        let raw = store
            .get_setting("custom_tool_project_paths")
            .map_err(db)?
            .unwrap_or_else(|| "{}".into());
        let mut paths: BTreeMap<String, String> = serde_json::from_str(&raw).unwrap_or_default();
        paths.remove(key.trim());
        store
            .set_setting(
                "custom_tool_project_paths",
                &serde_json::to_string(&paths).map_err(|err| AppError::io(err.to_string()))?,
            )
            .map_err(db)
    })
    .await
}
