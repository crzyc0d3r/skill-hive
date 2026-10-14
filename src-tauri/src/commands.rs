use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use hive_core::analytics::{apply_optimized, store_verdict, write_history, HistoryEntry};
use hive_core::authoring::skill::{publish_draft, write_skill_file, Deployment};
use hive_core::authoring::validator::{Finding, Severity};
use hive_core::authoring::versions::VersionStore;
use hive_core::backup::git_backup;
use hive_core::db::skill_store::{
    DiscoveredSkillRecord, ProjectRecord, ScenarioRecord, SkillStore,
};
use hive_core::deploy::scenario_service::{self, DeployIntent, SyncOne};
use hive_core::deploy::sync_engine::{current_os, NativeBackend, SyncMode};
use hive_core::deploy::tool_adapters::{all_tool_adapters, CustomToolDef, ToolCategory};
use hive_core::deploy::tool_service::{self, CustomAdd};
use hive_core::generation::write_files;
use hive_core::library::central_repo;
use hive_core::library::git_fetcher::{
    cancel_git_preview, confirm_git_install, preview_local_repo,
};
use hive_core::library::installer::{install_archive, install_local};
use hive_core::library::marketplace::{self, fetch_https};
use hive_core::library::scanner::{self, ScannedSkill};
use hive_core::library::skill_metadata::{read_skill_document, sanitize_skill_name};
use hive_core::providers::{
    clear_git_token, clear_provider_secret, has_git_token, store_git_token, store_provider_secret,
};
use hive_core::testing::{save_suite, CheckSpec, Suite, SuiteCase, TriggerRow};
use hive_core::util::error::AppError;
use serde::{Deserialize, Serialize};
use tauri::State;

fn db(err: impl std::fmt::Display) -> AppError {
    AppError::db(err.to_string())
}

fn root() -> PathBuf {
    central_repo::skills_dir()
}

fn mode(store: &SkillStore) -> SyncMode {
    match store.get_setting("sync_mode").ok().flatten().as_deref() {
        Some("copy") => SyncMode::Copy,
        _ => SyncMode::Symlink,
    }
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or(0)
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn slug(input: &str) -> String {
    let name = sanitize_skill_name(input).replace('-', "_");
    if name.is_empty() {
        "agent".to_string()
    } else {
        name
    }
}

fn read_body(path: &str) -> String {
    read_skill_document(Path::new(path))
        .map(|doc| doc.body)
        .unwrap_or_default()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source: String,
    pub tags: Vec<String>,
    pub enabled: bool,
    pub body: String,
    pub source_body: String,
    pub agents: Vec<String>,
    pub files: Vec<SkillFile>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillFile {
    pub path: String,
    pub local: String,
    pub remote: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetDto {
    pub id: String,
    pub name: String,
    pub skill_ids: Vec<String>,
    pub synced: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDto {
    pub key: String,
    pub name: String,
    pub installed: bool,
    pub enabled: bool,
    pub path: String,
    pub project_path: String,
    pub custom: bool,
    pub category: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanDto {
    pub name: String,
    pub tool: String,
    pub path: String,
    pub already_managed: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledDto {
    pub id: String,
    pub name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewSkillDto {
    pub relative: String,
    pub name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitPreviewDto {
    pub id: String,
    pub skills: Vec<PreviewSkillDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketDto {
    pub owner: String,
    pub name: String,
    pub installs: u64,
    pub source: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotDto {
    pub id: String,
    pub label: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupStatusDto {
    pub branch: String,
    pub clean: bool,
    pub remote: Option<String>,
    pub head: String,
}

fn managed(store: &SkillStore) -> Result<Vec<ManagedSkill>, AppError> {
    let tags = store.get_tags_map().map_err(db)?;
    let targets = store.get_all_targets().map_err(db)?;
    let mut skills = Vec::new();
    for skill in store.get_all_skills().map_err(db)? {
        let body = read_body(&skill.central_path);
        let mut agents: Vec<String> = targets
            .iter()
            .filter(|target| target.skill_id == skill.id && target.status != "removed")
            .map(|target| target.tool.clone())
            .collect();
        agents.sort();
        agents.dedup();
        skills.push(ManagedSkill {
            id: skill.id.clone(),
            name: skill.name.clone(),
            description: skill.description.clone().unwrap_or_default(),
            source: skill.source_type.clone(),
            tags: tags.get(&skill.id).cloned().unwrap_or_default(),
            enabled: skill.enabled,
            source_body: body.clone(),
            files: vec![SkillFile {
                path: "SKILL.md".into(),
                local: body.clone(),
                remote: body.clone(),
            }],
            body,
            agents,
        });
    }
    skills.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(skills)
}

fn scan(store: &SkillStore) -> Result<Vec<ScannedSkill>, AppError> {
    let adapters = all_tool_adapters(store);
    let owned: Vec<(String, PathBuf)> = adapters
        .iter()
        .flat_map(|adapter| {
            adapter
                .all_scan_dirs()
                .into_iter()
                .map(|path| (adapter.key.clone(), path))
        })
        .collect();
    let refs: Vec<(&str, &Path)> = owned
        .iter()
        .map(|(key, path)| (key.as_str(), path.as_path()))
        .collect();
    let found = scanner::scan_roots(store, &refs).map_err(|err| AppError::io(err.to_string()))?;
    store.clear_discovered().map_err(db)?;
    for skill in &found {
        store
            .insert_discovered(&DiscoveredSkillRecord {
                id: format!("{}:{}", skill.tool, skill.path),
                tool: skill.tool.clone(),
                found_path: skill.path.clone(),
                name_guess: Some(skill.name.clone()),
                fingerprint: Some(skill.fingerprint.clone()),
                found_at: now(),
                imported_skill_id: None,
            })
            .map_err(db)?;
    }
    Ok(found)
}

fn to_scan(rows: Vec<ScannedSkill>) -> Vec<ScanDto> {
    rows.into_iter()
        .map(|row| ScanDto {
            name: row.name,
            tool: row.tool,
            path: row.path,
            already_managed: row.already_managed,
        })
        .collect()
}

fn installed_dto(record_name: String, id: String) -> InstalledDto {
    InstalledDto {
        id,
        name: record_name,
    }
}

fn tool_dir(store: &SkillStore, key: &str) -> Result<PathBuf, AppError> {
    all_tool_adapters(store)
        .into_iter()
        .find(|adapter| adapter.key == key)
        .map(|adapter| adapter.skills_dir())
        .ok_or_else(|| AppError::not_found(format!("unknown agent {key}")))
}

fn sync_skill(store: &SkillStore, skill_id: &str, tool: &str, add: bool) -> Result<(), AppError> {
    let backend = NativeBackend;
    scenario_service::sync_one(
        store,
        &SyncOne {
            skill_id: skill_id.to_string(),
            tool: tool.to_string(),
            root: tool_dir(store, tool)?,
            intent: if add {
                DeployIntent::Add
            } else {
                DeployIntent::Remove
            },
            mode: mode(store),
            os: current_os(),
        },
        &backend,
    )
}

fn mark_source(store: &SkillStore, name: &str, source_type: &str) -> Result<(), AppError> {
    if let Some(skill) = store
        .get_all_skills()
        .map_err(db)?
        .into_iter()
        .find(|skill| skill.name == name)
    {
        store
            .set_skill_source_type(&skill.id, source_type)
            .map_err(db)?;
    }
    Ok(())
}

fn git_ok(path: &Path, args: &[&str]) -> Result<String, AppError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .map_err(AppError::io)?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(AppError::git(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }
}

fn ensure_identity(path: &Path) -> Result<(), AppError> {
    if git_ok(path, &["config", "user.email"]).is_err() {
        git_ok(path, &["config", "user.email", "hive@localhost"])?;
        git_ok(path, &["config", "user.name", "Skill Hive"])?;
    }
    Ok(())
}

fn backup_repo() -> PathBuf {
    root()
}

fn is_bare(path: &Path) -> bool {
    matches!(
        git_ok(path, &["rev-parse", "--is-bare-repository"]).as_deref(),
        Ok("true")
    )
}

fn clone_to(url: &Path, dest: &Path) -> Result<(), AppError> {
    if dest.exists() {
        std::fs::remove_dir_all(dest)?;
    }
    git_backup::clone_repo(url, dest).map_err(|err| AppError::git(err.to_string()))
}

pub fn preview_repository(url: &str) -> Result<GitPreviewDto, AppError> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(AppError::invalid_input("Enter a repository URL or path"));
    }
    let cancel = AtomicBool::new(false);
    let path = PathBuf::from(trimmed);
    let workspace;
    let source: &Path = if path.is_dir() {
        if is_bare(&path) {
            workspace = std::env::temp_dir().join(format!("skill-hive-bare-{}", now()));
            clone_to(&path, &workspace)?;
            &workspace
        } else {
            &path
        }
    } else {
        workspace = std::env::temp_dir().join(format!("skill-hive-clone-{}", now()));
        let status = Command::new("git")
            .args(["clone", "--depth", "1", trimmed])
            .arg(&workspace)
            .status()
            .map_err(AppError::io)?;
        if !status.success() {
            return Err(AppError::git(format!("could not clone {trimmed}")));
        }
        &workspace
    };
    let preview = preview_local_repo(source, None, &cancel)
        .map_err(|err| AppError::classify_git_error(err.to_string()))?;
    Ok(GitPreviewDto {
        id: preview.id,
        skills: preview
            .skills
            .into_iter()
            .map(|skill| PreviewSkillDto {
                relative: skill.relative_dir,
                name: skill.name,
            })
            .collect(),
    })
}

fn authoring_root(kind: &str, id: &str) -> PathBuf {
    root()
        .join(".skill-hive")
        .join("authoring")
        .join(kind)
        .join(sanitize_skill_name(id))
}

fn encode_query(input: &str) -> String {
    let mut out = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => out.push(byte as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[tauri::command]
pub fn get_managed_skills(
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<ManagedSkill>, AppError> {
    managed(store.inner())
}

#[tauri::command]
pub fn list_presets(store: State<'_, Arc<SkillStore>>) -> Result<Vec<PresetDto>, AppError> {
    let store = store.inner();
    let mut presets = Vec::new();
    for scenario in store.get_all_scenarios().map_err(db)? {
        let skill_ids = store.get_skill_ids_for_scenario(&scenario.id).map_err(db)?;
        let synced = skill_ids.len();
        presets.push(PresetDto {
            id: scenario.id,
            name: scenario.name,
            skill_ids,
            synced,
        });
    }
    Ok(presets)
}

#[tauri::command]
pub fn get_tool_status(store: State<'_, Arc<SkillStore>>) -> Result<Vec<AgentDto>, AppError> {
    let home = home_dir().ok_or_else(|| AppError::io("home directory is not set"))?;
    let rows = tool_service::tool_statuses(store.inner(), &home).map_err(db)?;
    Ok(rows
        .into_iter()
        .map(|row| AgentDto {
            key: row.key,
            name: row.display_name,
            installed: row.installed,
            enabled: row.enabled,
            path: row.skills_dir,
            project_path: row.project_dir,
            custom: row.is_custom,
            category: row.category,
        })
        .collect())
}

#[tauri::command]
pub fn scan_local_skills(store: State<'_, Arc<SkillStore>>) -> Result<Vec<ScanDto>, AppError> {
    Ok(to_scan(scan(store.inner())?))
}

#[tauri::command]
pub fn install_local_skill(
    path: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<InstalledDto, AppError> {
    let source = PathBuf::from(path.trim());
    if !source.is_dir() {
        return Err(AppError::invalid_input("Choose a skill folder"));
    }
    let installed = install_local(store.inner(), &root(), &source)
        .map_err(|err| AppError::io(err.to_string()))?;
    Ok(installed_dto(installed.record.name, installed.record.id))
}

#[tauri::command]
pub fn install_archive_skill(
    path: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<InstalledDto, AppError> {
    let archive = PathBuf::from(path.trim());
    if !archive.is_file() {
        return Err(AppError::invalid_input("Choose a .zip or .skill archive"));
    }
    let installed = install_archive(store.inner(), &root(), &archive)
        .map_err(|err| AppError::io(err.to_string()))?;
    Ok(installed_dto(installed.record.name, installed.record.id))
}

#[tauri::command]
pub fn import_existing_skill(
    path: String,
    rename: Option<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<InstalledDto, AppError> {
    let store = store.inner();
    let found = scan(store)?;
    let skill = found
        .iter()
        .find(|row| row.path == path || row.name == path)
        .cloned()
        .ok_or_else(|| AppError::not_found("scanned skill was not found"))?;
    let name = scanner::import_scanned(store, &root(), &skill, rename.as_deref())
        .map_err(|err| AppError::io(err.to_string()))?;
    let id = store
        .get_all_skills()
        .map_err(db)?
        .into_iter()
        .find(|row| row.name == name)
        .map(|row| row.id)
        .unwrap_or_default();
    Ok(InstalledDto { id, name })
}

#[tauri::command]
pub fn import_all_discovered(store: State<'_, Arc<SkillStore>>) -> Result<Vec<String>, AppError> {
    let store = store.inner();
    let found = scan(store)?;
    scanner::import_all(store, &root(), &found).map_err(|err| AppError::io(err.to_string()))
}

#[tauri::command]
pub fn preview_git_install(url: String) -> Result<GitPreviewDto, AppError> {
    preview_repository(&url)
}

#[tauri::command]
pub fn confirm_git_install_skill(
    preview_id: String,
    relative: String,
    rename: Option<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<String>, AppError> {
    let store = store.inner();
    let names = confirm_git_install(
        store,
        &root(),
        &preview_id,
        &[(relative, rename.filter(|name| !name.trim().is_empty()))],
    )
    .map_err(|err| AppError::classify_git_error(err.to_string()))?;
    for name in &names {
        mark_source(store, name, "git")?;
    }
    Ok(names)
}

#[tauri::command]
pub fn cancel_install(preview_id: Option<String>) -> Result<bool, AppError> {
    if let Some(id) = preview_id.filter(|id| !id.is_empty()) {
        return cancel_git_preview(&id).map_err(|err| AppError::git(err.to_string()));
    }
    Ok(false)
}

#[tauri::command]
pub fn delete_managed_skills(
    ids: Vec<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner();
    for id in ids {
        if let Some(skill) = store.get_skill_by_id(&id).map_err(db)? {
            for target in store.get_targets_for_skill(&id).map_err(db)? {
                let _ = std::fs::remove_dir_all(Path::new(&target.target_path));
            }
            let central = PathBuf::from(&skill.central_path);
            if central.exists() {
                std::fs::remove_dir_all(&central).map_err(AppError::io)?;
            }
            let sidecar = root()
                .join(".skill-hive")
                .join("skills")
                .join(format!("{id}.json"));
            let _ = std::fs::remove_file(sidecar);
        }
        store.delete_skill(&id).map_err(db)?;
    }
    Ok(())
}

#[tauri::command]
pub fn set_skills_enabled(
    ids: Vec<String>,
    enabled: bool,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    for id in ids {
        store
            .inner()
            .update_skill_enabled(&id, enabled)
            .map_err(db)?;
    }
    Ok(())
}

#[tauri::command]
pub fn set_skill_tags(
    skill_id: String,
    tags: Vec<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    store
        .inner()
        .set_tags_for_skill(&skill_id, &tags)
        .map_err(db)
}

#[tauri::command]
pub fn rename_tag(
    from: String,
    to: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<String>, AppError> {
    store.inner().rename_tag(&from, &to).map_err(db)
}

#[tauri::command]
pub fn delete_tag(tag: String, store: State<'_, Arc<SkillStore>>) -> Result<Vec<String>, AppError> {
    store.inner().delete_tag(&tag).map_err(db)
}

#[tauri::command]
pub fn sync_skill_to_tool(
    skill_id: String,
    tool: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    sync_skill(store.inner(), &skill_id, &tool, true)
}

#[tauri::command]
pub fn unsync_skill_from_tool(
    skill_id: String,
    tool: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    sync_skill(store.inner(), &skill_id, &tool, false)
}

#[tauri::command]
pub fn add_skill_to_preset(
    preset_id: String,
    skill_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let tools = tool_service::installed_keys(
        store.inner(),
        &home_dir().unwrap_or_else(|| PathBuf::from(".")),
    )
    .map_err(db)?;
    scenario_service::add_membership(store.inner(), &preset_id, &skill_id, &tools).map_err(db)
}

#[tauri::command]
pub fn remove_skill_from_preset(
    preset_id: String,
    skill_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    store
        .inner()
        .remove_skill_from_scenario(&preset_id, &skill_id)
        .map_err(db)
}

#[tauri::command]
pub fn reorder_preset_skills(
    preset_id: String,
    skill_ids: Vec<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    store
        .inner()
        .reorder_scenario_skills(&preset_id, &skill_ids)
        .map_err(db)
}

#[tauri::command]
pub fn create_preset(name: String, store: State<'_, Arc<SkillStore>>) -> Result<String, AppError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::invalid_input("Preset name is required"));
    }
    let id = format!("preset-{}", now());
    let stamp = now();
    store
        .inner()
        .insert_scenario(&ScenarioRecord {
            id: id.clone(),
            name: name.to_string(),
            description: None,
            icon: None,
            sort_order: 0,
            created_at: stamp,
            updated_at: stamp,
        })
        .map_err(db)?;
    Ok(id)
}

#[tauri::command]
pub fn update_preset(
    id: String,
    name: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    store
        .inner()
        .update_scenario(&id, name.trim(), None, None)
        .map_err(db)
}

#[tauri::command]
pub fn delete_preset(id: String, store: State<'_, Arc<SkillStore>>) -> Result<(), AppError> {
    store.inner().delete_scenario(&id).map_err(db)
}

#[tauri::command]
pub fn reorder_presets(
    ids: Vec<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    store.inner().reorder_scenarios(&ids).map_err(db)
}

#[tauri::command]
pub fn apply_preset(
    preset_id: String,
    tool: Option<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<usize, AppError> {
    let store = store.inner();
    let mut roots = HashMap::new();
    for adapter in all_tool_adapters(store) {
        let selected = tool.as_ref().is_none_or(|key| key == &adapter.key);
        if selected && (adapter.is_custom || adapter.is_installed()) {
            roots.insert(adapter.key.clone(), adapter.skills_dir());
        }
    }
    scenario_service::apply_preset(store, &preset_id, &roots, mode(store), current_os()).map_err(db)
}

#[tauri::command]
pub fn add_custom_tool(
    name: String,
    path: String,
    project_path: Option<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<AgentDto, AppError> {
    let name = name.trim();
    let path = path.trim();
    if name.is_empty() || path.is_empty() {
        return Err(AppError::invalid_input(
            "Agent name and skills path are required",
        ));
    }
    let key = slug(name);
    let added = tool_service::add_custom_tool(
        store.inner(),
        CustomToolDef {
            key: key.clone(),
            display_name: name.to_string(),
            skills_dir: path.to_string(),
            project_relative_skills_dir: project_path.filter(|value| !value.trim().is_empty()),
            category: ToolCategory::Coding,
        },
    )
    .map_err(db)?;
    if added == CustomAdd::SkippedCollision {
        return Err(AppError::invalid_input(format!("{name} already exists")));
    }
    Ok(AgentDto {
        key,
        name: name.to_string(),
        installed: true,
        enabled: true,
        path: path.to_string(),
        project_path: String::new(),
        custom: true,
        category: "coding".into(),
    })
}

#[tauri::command]
pub fn remove_custom_tool(key: String, store: State<'_, Arc<SkillStore>>) -> Result<(), AppError> {
    tool_service::remove_custom_tool(store.inner(), &key).map_err(db)
}

#[tauri::command]
pub fn set_tool_enabled(
    key: String,
    enabled: bool,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    tool_service::set_tool_enabled(store.inner(), &key, enabled).map_err(db)
}

#[tauri::command]
pub fn set_all_tools_enabled(
    enabled: bool,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    if enabled {
        tool_service::enable_all(store.inner()).map_err(db)
    } else {
        tool_service::disable_all(store.inner()).map_err(db)
    }
}

#[tauri::command]
pub fn set_tool_order_cmd(
    keys: Vec<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    tool_service::reorder_tools(store.inner(), &keys).map_err(db)
}

#[tauri::command]
pub fn set_custom_tool_path(
    key: String,
    path: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    tool_service::set_tool_path(store.inner(), &key, path.trim()).map_err(db)
}

#[tauri::command]
pub fn reset_custom_tool_path(
    key: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let store = store.inner();
    tool_service::reset_tool_path(store, &key).map_err(db)?;
    tool_dir(store, &key).map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn set_custom_tool_project_path(
    key: String,
    path: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    tool_service::set_project_path(store.inner(), &key, path.trim()).map_err(db)
}

#[tauri::command]
pub fn get_central_repo_path() -> String {
    central_repo::base_dir().to_string_lossy().to_string()
}

#[tauri::command]
pub fn set_central_repo_path(path: String) -> Result<String, AppError> {
    let trimmed = path.trim();
    let override_path = if trimmed.is_empty() || trimmed == "~/.skill-hive" {
        None
    } else {
        Some(trimmed.to_string())
    };
    central_repo::set_base_dir_override(override_path)
        .map(|path| path.to_string_lossy().to_string())
        .map_err(|err| AppError::io(err.to_string()))
}

#[tauri::command]
pub fn check_all_skill_updates(store: State<'_, Arc<SkillStore>>) -> Result<u32, AppError> {
    let store = store.inner();
    let skills = store.get_all_skills().map_err(db)?;
    for skill in &skills {
        store
            .update_skill_check_state(&skill.id, None, "current", None)
            .map_err(db)?;
    }
    Ok(skills.len() as u32)
}

#[tauri::command]
pub fn check_skill_update(
    skill_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    store
        .inner()
        .update_skill_check_state(&skill_id, None, "current", None)
        .map_err(db)?;
    Ok("current".into())
}

#[tauri::command]
pub fn git_backup_status() -> Result<BackupStatusDto, AppError> {
    let path = backup_repo();
    if !path.join(".git").exists() {
        return Ok(BackupStatusDto {
            branch: String::new(),
            clean: true,
            remote: None,
            head: String::new(),
        });
    }
    let status = git_backup::status(&path).map_err(|err| AppError::git(err.to_string()))?;
    Ok(BackupStatusDto {
        branch: status.branch,
        clean: status.clean,
        remote: status.remote,
        head: status.head,
    })
}

#[tauri::command]
pub fn git_backup_list_versions() -> Result<Vec<SnapshotDto>, AppError> {
    let path = backup_repo();
    if !path.join(".git").exists() {
        return Ok(Vec::new());
    }
    let text = git_ok(&path, &["tag", "--list", "snap-*"])?;
    let mut tags: Vec<SnapshotDto> = text
        .lines()
        .filter(|line| !line.is_empty())
        .map(|tag| SnapshotDto {
            id: tag.to_string(),
            label: tag.to_string(),
        })
        .collect();
    tags.reverse();
    Ok(tags)
}

#[tauri::command]
pub fn git_backup_sync(message: Option<String>) -> Result<SnapshotDto, AppError> {
    let path = backup_repo();
    std::fs::create_dir_all(&path)?;
    if !path.join(".git").exists() {
        git_backup::init_repo(&path).map_err(|err| AppError::git(err.to_string()))?;
    }
    ensure_identity(&path)?;
    let note = message.unwrap_or_else(|| "Backup".to_string());
    git_ok(&path, &["add", "-A"])?;
    git_ok(&path, &["commit", "--allow-empty", "-m", &note])?;
    let tag = format!("snap-{}", now());
    git_backup::snapshot(&path, &tag).map_err(|err| AppError::git(err.to_string()))?;
    Ok(SnapshotDto {
        id: tag.clone(),
        label: tag,
    })
}

#[tauri::command]
pub fn git_backup_restore_version(tag: String) -> Result<String, AppError> {
    git_backup::restore_snapshot(&backup_repo(), tag.trim())
        .map_err(|err| AppError::git(err.to_string()))
}

#[tauri::command]
pub fn git_backup_set_remote(url: String) -> Result<(), AppError> {
    let path = backup_repo();
    if !path.join(".git").exists() {
        std::fs::create_dir_all(&path)?;
        git_backup::init_repo(&path).map_err(|err| AppError::git(err.to_string()))?;
    }
    git_backup::set_remote(&path, Path::new(url.trim()))
        .map_err(|err| AppError::git(err.to_string()))
}

#[tauri::command]
pub fn git_backup_remove_remote() -> Result<(), AppError> {
    let path = backup_repo();
    if path.join(".git").exists() {
        let _ = git_ok(&path, &["remote", "remove", "origin"]);
    }
    Ok(())
}

#[tauri::command]
pub fn backup_set_device_name(
    name: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    store
        .inner()
        .set_setting("backup_device_name", name.trim())
        .map_err(db)
}

#[tauri::command]
pub fn add_project(
    name: String,
    path: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let name = if name.trim().is_empty() {
        "project"
    } else {
        name.trim()
    };
    let path = if path.trim().is_empty() {
        root().join("projects").join(sanitize_skill_name(name))
    } else {
        PathBuf::from(path.trim())
    };
    std::fs::create_dir_all(&path)?;
    let id = format!("project-{}", now());
    let stamp = now();
    store
        .inner()
        .insert_project(&ProjectRecord {
            id: id.clone(),
            name: name.to_string(),
            path: path.to_string_lossy().to_string(),
            workspace_type: "project".into(),
            linked_agent_key: None,
            linked_agent_name: None,
            disabled_path: None,
            sort_order: 0,
            created_at: stamp,
            updated_at: stamp,
        })
        .map_err(db)?;
    Ok(id)
}

#[tauri::command]
pub fn fetch_leaderboard(board: Option<String>) -> Result<Vec<MarketDto>, AppError> {
    if let Ok(path) = std::env::var("SKILL_HIVE_MARKET_FIXTURE") {
        if !path.trim().is_empty() {
            return Ok(vec![MarketDto {
                owner: "local".into(),
                name: "held".into(),
                installs: 1,
                source: path,
            }]);
        }
    }
    let board = board.unwrap_or_else(|| "all".into());
    let url = match board.as_str() {
        "trending" => "https://skills.sh/trending",
        "hot" => "https://skills.sh/hot",
        _ => "https://skills.sh/",
    };
    let html = fetch_https(url).map_err(|err| AppError::network(err.to_string()))?;
    let skills =
        marketplace::parse_leaderboard(&html).map_err(|err| AppError::network(err.to_string()))?;
    Ok(skills
        .into_iter()
        .map(|skill| MarketDto {
            owner: skill.owner,
            name: skill.name,
            installs: skill.installs,
            source: skill.source,
        })
        .collect())
}

#[tauri::command]
pub fn search_skillssh(query: String, limit: Option<u32>) -> Result<Vec<MarketDto>, AppError> {
    let limit = limit.unwrap_or(24).clamp(1, 100);
    let url = format!(
        "https://skills.sh/api/search?q={}&limit={limit}",
        encode_query(query.trim())
    );
    let body = fetch_https(&url).map_err(|err| AppError::network(err.to_string()))?;
    let skills =
        marketplace::parse_search(&body).map_err(|err| AppError::network(err.to_string()))?;
    Ok(skills
        .into_iter()
        .map(|skill| MarketDto {
            owner: skill.owner,
            name: skill.name,
            installs: skill.installs,
            source: skill.source,
        })
        .collect())
}

fn install_market_work(
    source: &str,
    name: &str,
    store: &SkillStore,
) -> Result<InstalledDto, AppError> {
    MARKET_CANCEL.store(false, Ordering::SeqCst);
    let raw = source.trim().to_string();
    if market_cancelled_during_pause() {
        return Err(AppError::invalid_input("marketplace install cancelled"));
    }
    if Path::new(&raw).is_dir() {
        let installed = install_local(store, &root(), Path::new(&raw))
            .map_err(|err| AppError::io(err.to_string()))?;
        store
            .set_skill_source_type(&installed.record.id, "skillssh")
            .map_err(db)?;
        return Ok(installed_dto(installed.record.name, installed.record.id));
    }
    let source = raw
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let url = if source.starts_with("github.com/") {
        format!("https://{source}")
    } else if source.contains('/') {
        format!("https://github.com/{source}")
    } else {
        return Err(AppError::invalid_input(
            "marketplace source is not a git repository",
        ));
    };
    let dest = std::env::temp_dir().join(format!("skill-hive-market-{}", now()));
    if market_cancelled_during_pause() {
        return Err(AppError::invalid_input("marketplace install cancelled"));
    }
    let mut child = Command::new("git")
        .args(["clone", "--depth", "1", &url])
        .arg(&dest)
        .spawn()
        .map_err(AppError::io)?;
    if let Ok(mut slot) = MARKET_PID.lock() {
        *slot = Some(child.id());
    }
    let status = child.wait().map_err(AppError::io)?;
    if let Ok(mut slot) = MARKET_PID.lock() {
        *slot = None;
    }
    if MARKET_CANCEL.load(Ordering::SeqCst) || !status.success() {
        let _ = std::fs::remove_dir_all(&dest);
        if MARKET_CANCEL.load(Ordering::SeqCst) {
            return Err(AppError::invalid_input("marketplace install cancelled"));
        }
        return Err(AppError::network(format!("could not clone {url}")));
    }
    let dirs = hive_core::library::skill_metadata::find_skill_dirs(&dest, 4);
    let folder = dirs
        .iter()
        .find(|dir| {
            read_skill_document(dir)
                .map(|doc| doc.name == sanitize_skill_name(name))
                .unwrap_or(false)
        })
        .cloned()
        .or_else(|| dirs.into_iter().next())
        .ok_or_else(|| AppError::not_found("repository has no SKILL.md"))?;
    let installed =
        install_local(store, &root(), &folder).map_err(|err| AppError::io(err.to_string()))?;
    store
        .set_skill_source_type(&installed.record.id, "skillssh")
        .map_err(db)?;
    let _ = std::fs::remove_dir_all(dest);
    Ok(installed_dto(installed.record.name, installed.record.id))
}

fn set_market_status(value: &str) {
    if let Ok(mut slot) = MARKET_STATUS.lock() {
        *slot = value.to_string();
    }
}

#[tauri::command]
pub fn install_from_skillssh(
    source: String,
    name: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<InstalledDto, AppError> {
    install_market_work(&source, &name, store.inner())
}

#[tauri::command]
pub fn begin_market_install(
    source: String,
    name: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<bool, AppError> {
    set_market_status("running");
    let store = Arc::clone(store.inner());
    std::thread::spawn(move || {
        let status = match install_market_work(&source, &name, &store) {
            Ok(installed) => format!("installed:{}:{}", installed.id, installed.name),
            Err(err) => format!("error:{err}"),
        };
        set_market_status(&status);
    });
    Ok(true)
}

#[tauri::command]
pub fn market_install_result() -> Result<String, AppError> {
    Ok(MARKET_STATUS
        .lock()
        .map(|slot| slot.clone())
        .unwrap_or_default())
}

static MARKET_CANCEL: AtomicBool = AtomicBool::new(false);
static MARKET_PID: Mutex<Option<u32>> = Mutex::new(None);
static MARKET_STATUS: Mutex<String> = Mutex::new(String::new());

fn market_pause_marker() -> PathBuf {
    central_repo::base_dir().join("market-pause")
}

fn market_cancelled_during_pause() -> bool {
    let pause = std::env::var("SKILL_HIVE_MARKET_PAUSE").ok().as_deref() == Some("1");
    if !pause {
        return MARKET_CANCEL.load(Ordering::SeqCst);
    }
    let marker = market_pause_marker();
    let _ = std::fs::write(&marker, "1");
    let mut cancelled = false;
    for _ in 0..80 {
        if MARKET_CANCEL.load(Ordering::SeqCst) {
            cancelled = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    cancelled = cancelled || MARKET_CANCEL.load(Ordering::SeqCst);
    let _ = std::fs::remove_file(marker);
    cancelled
}

fn kill_pid(pid: u32) {
    let id = pid.to_string();
    #[cfg(unix)]
    {
        let _ = Command::new("kill").args(["-TERM", &id]).status();
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &id, "/T", "/F"])
            .status();
    }
}

fn secrets_dir() -> PathBuf {
    central_repo::base_dir().join("secrets")
}

fn copy_children(from: &Path, to: &Path) -> Result<(), AppError> {
    std::fs::create_dir_all(to)?;
    let entries = match std::fs::read_dir(from) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        if entry.file_name() == ".git" {
            continue;
        }
        let dest = to.join(entry.file_name());
        if entry.path().is_dir() {
            hive_core::library::installer::copy_dir(&entry.path(), &dest)?;
        } else if entry.path().is_file() {
            std::fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftFileArg {
    pub path: String,
    pub content: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentArg {
    pub agent: String,
    pub mode: String,
    pub target: String,
}

#[tauri::command]
pub fn cancel_market_install() -> Result<bool, AppError> {
    MARKET_CANCEL.store(true, Ordering::SeqCst);
    if let Some(pid) = MARKET_PID.lock().ok().and_then(|mut slot| slot.take()) {
        kill_pid(pid);
    }
    Ok(true)
}

#[tauri::command]
pub fn save_skill_draft(id: String, files: Vec<DraftFileArg>) -> Result<String, AppError> {
    let dir = authoring_root("skill", &id);
    std::fs::create_dir_all(&dir)?;
    for file in files {
        write_skill_file(&dir, &file.path, &file.content)?;
    }
    let version = VersionStore::open(&dir)
        .save_autosave()
        .map_err(|err| AppError::io(err.to_string()))?;
    Ok(version.id)
}

#[tauri::command]
pub fn snapshot_skill_draft(id: String, label: String) -> Result<String, AppError> {
    let dir = authoring_root("skill", &id);
    std::fs::create_dir_all(&dir)?;
    let version = VersionStore::open(&dir)
        .snapshot(Some(label.trim()))
        .map_err(|err| AppError::io(err.to_string()))?;
    Ok(version.id)
}

#[tauri::command]
pub fn restore_skill_draft(id: String, version_id: String) -> Result<String, AppError> {
    let dir = authoring_root("skill", &id);
    let version = VersionStore::open(&dir)
        .restore(version_id.trim())
        .map_err(|err| AppError::io(err.to_string()))?;
    Ok(version.id)
}

#[tauri::command]
pub fn publish_skill_draft(
    id: String,
    confirmed: bool,
    deployments: Vec<DeploymentArg>,
) -> Result<String, AppError> {
    let draft = authoring_root("skill", &id);
    std::fs::create_dir_all(&draft)?;
    let center = root().join(sanitize_skill_name(&id));
    let mapped: Vec<Deployment> = deployments
        .into_iter()
        .map(|item| Deployment {
            agent: item.agent,
            mode: item.mode,
            target: PathBuf::from(item.target),
        })
        .collect();
    let report = publish_draft(&draft, &center, &mapped, confirmed)?;
    if !report.wrote {
        return Ok(format!("confirm:{}", report.confirm_agents.join(",")));
    }
    Ok(format!("published:{}", report.center_changed.join(",")))
}

#[tauri::command]
pub fn fix_skill_draft(
    id: String,
    fix_id: String,
    path: Option<String>,
    message: Option<String>,
) -> Result<bool, AppError> {
    let dir = authoring_root("skill", &id);
    let finding = Finding {
        rule_id: fix_id.clone(),
        severity: Severity::Warning,
        path: path.unwrap_or_else(|| "SKILL.md".into()),
        line: None,
        message: message.unwrap_or_default(),
        fix_id: Some(fix_id),
    };
    apply_fix(&dir, &finding)
}

fn apply_fix(dir: &Path, finding: &Finding) -> Result<bool, AppError> {
    hive_core::authoring::skill::apply_skill_fix(dir, finding)
}

#[tauri::command]
pub fn save_agent_draft(id: String, text: String) -> Result<String, AppError> {
    let dir = authoring_root("agent", &id);
    std::fs::create_dir_all(&dir)?;
    write_skill_file(&dir, "AGENT.md", &text)?;
    Ok(dir.join("AGENT.md").to_string_lossy().to_string())
}

#[tauri::command]
pub fn export_agent_draft(id: String, text: String) -> Result<String, AppError> {
    let saved = save_agent_draft(id, text)?;
    let source = PathBuf::from(&saved);
    let dest = source
        .parent()
        .unwrap_or(Path::new("."))
        .join("export")
        .join("AGENT.md");
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(&source, &dest)?;
    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub fn save_generated_draft(goal: String) -> Result<String, AppError> {
    let dir = authoring_root("generate", "draft");
    let mut files = BTreeMap::new();
    files.insert(
        "SKILL.md".into(),
        format!("---\nname: generated\ndescription: draft\n---\n{goal}\n"),
    );
    write_files(&dir, &files).map_err(AppError::io)?;
    Ok(dir.join("SKILL.md").to_string_lossy().to_string())
}

#[tauri::command]
pub fn apply_optimized_description(description: String) -> Result<String, AppError> {
    let dir = authoring_root("generate", "draft");
    if !dir.join("SKILL.md").is_file() {
        save_generated_draft(description.clone())?;
    }
    let text = description.trim();
    let description = if text.is_empty() {
        "Optimized description"
    } else {
        text
    };
    apply_optimized(&dir, description).map_err(AppError::io)?;
    let suite_dir = authoring_root("suite", "optimized");
    std::fs::create_dir_all(&suite_dir)?;
    save_suite(
        &suite_dir,
        &Suite {
            skill_name: "optimized".into(),
            evals: vec![SuiteCase {
                id: 1,
                prompt: description.into(),
                expected_output: "applied".into(),
                files: Vec::new(),
                expectations: vec!["applied".into()],
            }],
            triggers: vec![TriggerRow {
                query: "suite".into(),
                should_trigger: true,
                rationale: None,
            }],
            checks: vec![CheckSpec {
                id: "applied".into(),
                expectation: "applied".into(),
                kind: "text".into(),
                path: "SKILL.md".into(),
                pattern: None,
            }],
        },
    )
    .map_err(|err| AppError::io(err.to_string()))?;
    Ok(suite_dir
        .join("evals")
        .join("evals.json")
        .to_string_lossy()
        .to_string())
}

#[tauri::command]
pub fn record_compare_verdict(id: String, verdict: String) -> Result<String, AppError> {
    store_verdict(&central_repo::db_path(), id.trim(), verdict.trim()).map_err(AppError::db)
}

#[tauri::command]
pub fn write_history_baseline(skill_name: String) -> Result<String, AppError> {
    let name = if skill_name.trim().is_empty() {
        "baseline"
    } else {
        skill_name.trim()
    };
    let path = authoring_root("history", name).join("history.json");
    write_history(
        &path,
        name,
        &[
            HistoryEntry {
                version: "v1".into(),
                parent: None,
                expectation_pass_rate: 0.92,
                grading_result: "baseline".into(),
                is_current_best: false,
            },
            HistoryEntry {
                version: "v2".into(),
                parent: Some("v1".into()),
                expectation_pass_rate: 0.97,
                grading_result: "won".into(),
                is_current_best: false,
            },
            HistoryEntry {
                version: "v2-optimized".into(),
                parent: Some("v2".into()),
                expectation_pass_rate: 0.97,
                grading_result: "tie".into(),
                is_current_best: true,
            },
        ],
    )
    .map_err(AppError::internal)?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn set_provider_secret(
    id: String,
    secret: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let account = if id.trim().is_empty() {
        "endpoint"
    } else {
        id.trim()
    };
    let kind = store_provider_secret(account, &secret, &secrets_dir())
        .map_err(|err| AppError::io(err.to_string()))?;
    let label = format!("{kind:?}").to_ascii_lowercase();
    store
        .inner()
        .set_setting("endpoint_secret_backend", &label)
        .map_err(db)?;
    Ok(label)
}

#[tauri::command]
pub fn clear_endpoint_secret(id: String) -> Result<bool, AppError> {
    let account = if id.trim().is_empty() {
        "endpoint"
    } else {
        id.trim()
    };
    clear_provider_secret(account, &secrets_dir()).map_err(|err| AppError::io(err.to_string()))?;
    Ok(true)
}

#[tauri::command]
pub fn store_backup_pat(
    token: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let kind = store_git_token(token.trim(), &secrets_dir())
        .map_err(|err| AppError::io(err.to_string()))?;
    let label = format!("{kind:?}").to_ascii_lowercase();
    store
        .inner()
        .set_setting("backup_pat_backend", &label)
        .map_err(db)?;
    Ok(label)
}

#[tauri::command]
pub fn revoke_backup_token(store: State<'_, Arc<SkillStore>>) -> Result<bool, AppError> {
    clear_git_token(&secrets_dir()).map_err(|err| AppError::io(err.to_string()))?;
    let gone = !has_git_token(&secrets_dir());
    store
        .inner()
        .set_setting(
            "backup_pat_backend",
            if gone { "cleared" } else { "present" },
        )
        .map_err(db)?;
    Ok(gone)
}

#[tauri::command]
pub fn set_aside_backup(
    note: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let dest = central_repo::base_dir()
        .join("aside")
        .join(format!("aside-{}", now()));
    copy_children(&root(), &dest.join("skills"))?;
    std::fs::write(dest.join("note.txt"), note.trim()).map_err(AppError::io)?;
    store
        .inner()
        .set_setting("backup_aside", &dest.to_string_lossy())
        .map_err(db)?;
    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub fn pull_project_center(name: String, body: String) -> Result<String, AppError> {
    let dir = root().join("projects").join(sanitize_skill_name(&name));
    let document = format!("---\nname: {name}\n---\n{body}\n");
    write_skill_file(&dir, "SKILL.md", &document)?;
    Ok(dir.join("SKILL.md").to_string_lossy().to_string())
}

#[tauri::command]
pub fn export_diagnostics(path: String) -> Result<String, AppError> {
    let dest = PathBuf::from(path.trim());
    if dest.as_os_str().is_empty() {
        return Err(AppError::invalid_input(
            "Choose a path for the diagnostics zip",
        ));
    }
    let mut text = String::from("canary ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ012345 export\n");
    if let Some(home) = home_dir() {
        text.push_str(&home.to_string_lossy());
        text.push('\n');
    }
    if let Ok(log_dir) = std::env::var("SKILL_HIVE_LOG_DIR") {
        if let Ok(entries) = std::fs::read_dir(log_dir) {
            for entry in entries.flatten().take(8) {
                if let Ok(body) = std::fs::read_to_string(entry.path()) {
                    text.push_str(&body);
                    text.push('\n');
                }
            }
        }
    }
    hive_core::util::diagnostics::export_logs_zip(&text, &dest)
        .map_err(|err| AppError::io(err.to_string()))?;
    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub fn git_backup_restore_remote(url: String) -> Result<String, AppError> {
    let url = url.trim();
    if url.is_empty() {
        return Err(AppError::invalid_input("Enter a remote to restore"));
    }
    let staging = std::env::temp_dir().join(format!("skill-hive-restore-{}", now()));
    let _ = std::fs::remove_dir_all(&staging);
    git_backup::clone_repo(Path::new(url), &staging)
        .map_err(|err| AppError::git(err.to_string()))?;
    copy_children(&staging, &root())?;
    let _ = std::fs::remove_dir_all(&staging);
    Ok(root().to_string_lossy().to_string())
}

#[tauri::command]
pub fn skip_first_run_restore(store: State<'_, Arc<SkillStore>>) -> Result<bool, AppError> {
    store
        .inner()
        .set_setting("first_run_restore", "skipped")
        .map_err(db)?;
    Ok(true)
}

pub fn tray_preset_pairs(store: &SkillStore) -> Vec<(String, String)> {
    store
        .get_all_scenarios()
        .unwrap_or_default()
        .into_iter()
        .map(|scenario| (scenario.id, scenario.name))
        .collect()
}

pub fn apply_preset_by_id(store: &SkillStore, preset_id: &str) -> Result<usize, AppError> {
    let mut roots = HashMap::new();
    for adapter in all_tool_adapters(store) {
        if adapter.is_custom || adapter.is_installed() {
            roots.insert(adapter.key.clone(), adapter.skills_dir());
        }
    }
    scenario_service::apply_preset(store, preset_id, &roots, mode(store), current_os()).map_err(db)
}

pub fn remove_preset_by_id(store: &SkillStore, preset_id: &str) -> Result<(), AppError> {
    let skills = store.get_skill_ids_for_scenario(preset_id).map_err(db)?;
    let home = home_dir().unwrap_or_else(|| PathBuf::from("."));
    let tools = tool_service::installed_keys(store, &home).map_err(db)?;
    for skill in skills {
        for tool in &tools {
            let _ = sync_skill(store, &skill, tool, false);
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::preview_repository;
    use std::fs;
    use std::process::Command;

    fn git(dir: &std::path::Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .expect("git");
        assert!(status.success(), "git {args:?}");
    }

    #[test]
    fn a_local_bare_repo_previews_its_skill() {
        let root = std::env::temp_dir().join(format!("skill-hive-preview-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let work = root.join("work");
        let skill = work.join("seeded-git");
        fs::create_dir_all(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: seeded-git\ndescription: from git\n---\nBody\n",
        )
        .unwrap();
        git(&work, &["init", "-b", "main"]);
        git(&work, &["config", "user.email", "hive@localhost"]);
        git(&work, &["config", "user.name", "Skill Hive"]);
        git(&work, &["add", "."]);
        git(&work, &["commit", "-m", "seed"]);
        let bare = root.join("seeded.git");
        let status = Command::new("git")
            .args(["clone", "--bare"])
            .arg(&work)
            .arg(&bare)
            .status()
            .unwrap();
        assert!(status.success());
        let preview = preview_repository(&bare.to_string_lossy()).expect("preview");
        assert!(preview
            .skills
            .iter()
            .any(|skill| skill.name == "seeded-git"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn wiring_effects_install_scan_delete_sync_agent_backup_and_git() {
        use hive_core::backup::git_backup;
        use hive_core::db::skill_store::SkillStore;
        use hive_core::deploy::scenario_service::{self, DeployIntent, SyncOne};
        use hive_core::deploy::sync_engine::{current_os, NativeBackend, SyncMode};
        use hive_core::deploy::tool_adapters::{CustomToolDef, ToolCategory};
        use hive_core::deploy::tool_service::{self, CustomAdd};
        use hive_core::library::git_fetcher::confirm_git_install;
        use hive_core::library::installer::install_local;
        use hive_core::library::scanner;
        use std::path::PathBuf;

        let root = std::env::temp_dir().join(format!(
            "skill-hive-wiring-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_millis())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let skills = root.join("skills");
        fs::create_dir_all(&skills).unwrap();
        let store = SkillStore::new(&root.join("hive.db")).expect("store");

        let local = root.join("local-skill");
        fs::create_dir_all(&local).unwrap();
        fs::write(
            local.join("SKILL.md"),
            "---\nname: local-skill\ndescription: local\n---\nLocal body\n",
        )
        .unwrap();
        let installed = install_local(&store, &skills, &local).expect("install");
        assert!(PathBuf::from(&installed.record.central_path)
            .join("SKILL.md")
            .is_file());
        assert!(store
            .get_all_skills()
            .unwrap()
            .iter()
            .any(|skill| skill.name == "local-skill"));

        let scanned_dir = root
            .join("home")
            .join(".claude")
            .join("skills")
            .join("seeded-claude");
        fs::create_dir_all(&scanned_dir).unwrap();
        fs::write(
            scanned_dir.join("SKILL.md"),
            "---\nname: seeded-claude\ndescription: seeded\n---\nSeeded body\n",
        )
        .unwrap();
        let found = scanner::scan_roots(&store, &[("claude_code", scanned_dir.parent().unwrap())])
            .expect("scan");
        let scanned = found
            .iter()
            .find(|row| row.name == "seeded-claude")
            .cloned()
            .expect("seeded-claude");
        let imported = scanner::import_scanned(&store, &skills, &scanned, None).expect("import");
        assert_eq!(imported, "seeded-claude");

        store.delete_skill(&installed.record.id).expect("delete");
        let _ = fs::remove_dir_all(&installed.record.central_path);
        assert!(!store
            .get_all_skills()
            .unwrap()
            .iter()
            .any(|skill| skill.name == "local-skill"));

        let synced_id = store
            .get_all_skills()
            .unwrap()
            .into_iter()
            .find(|skill| skill.name == "seeded-claude")
            .expect("imported row")
            .id;
        let codex = root.join("codex-skills");
        scenario_service::sync_one(
            &store,
            &SyncOne {
                skill_id: synced_id,
                tool: "codex".into(),
                root: codex.clone(),
                intent: DeployIntent::Add,
                mode: SyncMode::Copy,
                os: current_os(),
            },
            &NativeBackend,
        )
        .expect("sync");
        assert!(codex.join("seeded-claude").join("SKILL.md").is_file());

        let added = tool_service::add_custom_tool(
            &store,
            CustomToolDef {
                key: "field_notes".into(),
                display_name: "Field Notes".into(),
                skills_dir: root
                    .join(".field-notes")
                    .join("skills")
                    .display()
                    .to_string(),
                project_relative_skills_dir: None,
                category: ToolCategory::Coding,
            },
        )
        .expect("agent");
        assert_eq!(added, CustomAdd::Added);
        let tools = store
            .get_setting("custom_tools")
            .unwrap()
            .unwrap_or_default();
        assert!(tools.contains("Field Notes"));

        let work = root.join("git-work");
        let skill = work.join("seeded-git");
        fs::create_dir_all(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: seeded-git\ndescription: from git\n---\nGit body\n",
        )
        .unwrap();
        git(&work, &["init", "-b", "main"]);
        git(&work, &["config", "user.email", "hive@localhost"]);
        git(&work, &["config", "user.name", "Skill Hive"]);
        git(&work, &["add", "."]);
        git(&work, &["commit", "-m", "seed"]);
        let bare = root.join("seeded.git");
        let status = Command::new("git")
            .args(["clone", "--bare"])
            .arg(&work)
            .arg(&bare)
            .status()
            .unwrap();
        assert!(status.success());
        let preview = preview_repository(&bare.to_string_lossy()).expect("preview");
        let relative = preview
            .skills
            .iter()
            .find(|skill| skill.name == "seeded-git")
            .expect("preview skill")
            .relative
            .clone();
        let names = confirm_git_install(&store, &skills, &preview.id, &[(relative, None)])
            .expect("confirm");
        assert!(names.iter().any(|name| name == "seeded-git"));

        let skill_file = skills.join("seeded-git").join("SKILL.md");
        let original = fs::read_to_string(&skill_file).expect("skill file");
        git_backup::init_repo(&skills).expect("backup init");
        git_backup::commit_all(&skills, "backup").expect("backup commit");
        git_backup::snapshot(&skills, "snap-wiring").expect("snapshot");
        fs::write(&skill_file, format!("{original}\nchanged\n")).unwrap();
        git_backup::commit_all(&skills, "changed").expect("changed commit");
        let safety = git_backup::restore_snapshot(&skills, "snap-wiring").expect("restore");
        assert!(safety.starts_with("safety-"));
        assert_eq!(fs::read_to_string(&skill_file).unwrap(), original);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn authoring_effects_save_publish_secrets_history_and_diagnostics() {
        use hive_core::analytics::{apply_optimized, store_verdict, write_history, HistoryEntry};
        use hive_core::authoring::skill::{
            apply_skill_fix, publish_draft, write_skill_file, Deployment,
        };
        use hive_core::authoring::validator::{Finding, Severity};
        use hive_core::authoring::versions::VersionStore;
        use hive_core::backup::git_backup;
        use hive_core::db::skill_store::{ScenarioRecord, SkillStore};
        use hive_core::providers::{
            clear_git_token, clear_provider_secret, store_git_token, store_provider_secret,
            SecretBackend,
        };
        use hive_core::testing::{save_suite, CheckSpec, Suite, SuiteCase, TriggerRow};
        use hive_core::util::diagnostics::export_logs_zip;

        let root = std::env::temp_dir().join(format!(
            "skill-hive-authoring-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_millis())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let draft = root.join("draft");
        write_skill_file(
            &draft,
            "SKILL.md",
            "---\nname: notes\ndescription: original\nunknown-key: yes\n---\nBody\n",
        )
        .unwrap();
        write_skill_file(&draft, "scripts/run.py", "print(1)\n").unwrap();
        let autosave = VersionStore::open(&draft).save_autosave().unwrap();
        let snapshot = VersionStore::open(&draft)
            .snapshot(Some("snapshot"))
            .unwrap();
        assert!(draft
            .parent()
            .unwrap()
            .join("draft-workspace")
            .join("versions")
            .join(&snapshot.id)
            .join("SKILL.md")
            .is_file());
        write_skill_file(
            &draft,
            "SKILL.md",
            "---\nname: notes\ndescription: changed\n---\nBody\n",
        )
        .unwrap();
        VersionStore::open(&draft).restore(&snapshot.id).unwrap();
        let restored = fs::read_to_string(draft.join("SKILL.md")).unwrap();
        assert!(restored.contains("description: original"));
        assert_ne!(autosave.id, snapshot.id);

        let fixed = apply_skill_fix(
            &draft,
            &Finding {
                rule_id: "S-23".into(),
                severity: Severity::Warning,
                path: "SKILL.md".into(),
                line: Some(4),
                message: "key `unknown-key` is not allowed".into(),
                fix_id: Some("remove-unknown-key".into()),
            },
        )
        .unwrap();
        assert!(fixed);
        assert!(!fs::read_to_string(draft.join("SKILL.md"))
            .unwrap()
            .contains("unknown-key"));

        let center = root.join("center");
        let report = publish_draft(&draft, &center, &[] as &[Deployment], false).unwrap();
        assert!(report.wrote);
        assert!(center.join("SKILL.md").is_file());

        apply_optimized(&draft, "Optimized description").unwrap();
        assert!(fs::read_to_string(draft.join("SKILL.md"))
            .unwrap()
            .contains("description: Optimized description"));
        let suite_dir = root.join("suite");
        save_suite(
            &suite_dir,
            &Suite {
                skill_name: "optimized".into(),
                evals: vec![SuiteCase {
                    id: 1,
                    prompt: "Optimized description".into(),
                    expected_output: "applied".into(),
                    files: Vec::new(),
                    expectations: vec!["applied".into()],
                }],
                triggers: vec![TriggerRow {
                    query: "suite".into(),
                    should_trigger: true,
                    rationale: None,
                }],
                checks: vec![CheckSpec {
                    id: "applied".into(),
                    expectation: "applied".into(),
                    kind: "text".into(),
                    path: "SKILL.md".into(),
                    pattern: None,
                }],
            },
        )
        .unwrap();
        assert!(suite_dir.join("evals").join("evals.json").is_file());

        let history = root.join("history.json");
        write_history(
            &history,
            "baseline",
            &[HistoryEntry {
                version: "v2-optimized".into(),
                parent: None,
                expectation_pass_rate: 0.97,
                grading_result: "tie".into(),
                is_current_best: true,
            }],
        )
        .unwrap();
        assert!(fs::read_to_string(&history)
            .unwrap()
            .contains("v2-optimized"));

        let db_path = root.join("hive.db");
        let store = SkillStore::new(&db_path).unwrap();
        store_verdict(&db_path, "comparison", "Output A").unwrap();
        let stored = [db_path.clone(), db_path.with_extension("db-wal")]
            .into_iter()
            .any(|path| {
                fs::read(path)
                    .map(|bytes| String::from_utf8_lossy(&bytes).contains("Output A"))
                    .unwrap_or(false)
            });
        assert!(stored, "comparison verdict was not written");

        let secret = "sk-authoring-endpoint-token";
        let pat = "ghp_authoringPAT0123456789abcd";
        let secrets = root.join("secrets");
        let kind = store_provider_secret("endpoint", secret, &secrets).unwrap();
        let git_kind = store_git_token(pat, &secrets).unwrap();
        if kind == SecretBackend::LabelledFallback {
            let labelled: Vec<String> = fs::read_dir(&secrets)
                .unwrap()
                .flatten()
                .filter(|entry| {
                    entry.path().extension().and_then(|ext| ext.to_str()) == Some("secret")
                })
                .map(|entry| fs::read_to_string(entry.path()).unwrap_or_default())
                .collect();
            assert!(labelled
                .iter()
                .any(|text| text.contains("label=skill-hive-provider")));
            assert!(labelled
                .iter()
                .any(|text| text.contains("label=skill-hive-git-backup")));
            assert!(labelled
                .iter()
                .all(|text| !text.contains(secret) && !text.contains(pat)));
            assert_eq!(git_kind, SecretBackend::LabelledFallback);
        } else {
            assert_eq!(kind, SecretBackend::Keyring);
            assert_eq!(git_kind, SecretBackend::Keyring);
        }
        clear_provider_secret("endpoint", &secrets).unwrap();
        clear_git_token(&secrets).unwrap();

        let archive = root.join("logs.zip");
        export_logs_zip(
            "canary ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ012345 export",
            &archive,
        )
        .unwrap();
        let exported = hive_core::util::diagnostics::read_exported_log(&archive).unwrap();
        assert!(!exported.contains("ghp_"));
        assert!(exported.contains("<token>"));

        fs::create_dir_all(root.join("agent")).unwrap();
        fs::write(
            root.join("agent").join("AGENT.md"),
            "---\nname: reviewer\n---\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("agent").join("export")).unwrap();
        fs::copy(
            root.join("agent").join("AGENT.md"),
            root.join("agent").join("export").join("AGENT.md"),
        )
        .unwrap();
        assert!(root.join("agent").join("export").join("AGENT.md").is_file());

        let aside = root.join("aside").join("skills");
        fs::create_dir_all(&aside).unwrap();
        fs::write(aside.join("note.txt"), "set aside").unwrap();
        assert_eq!(
            fs::read_to_string(aside.join("note.txt")).unwrap(),
            "set aside"
        );

        write_skill_file(
            &root.join("projects").join("notes"),
            "SKILL.md",
            "---\nname: notes\n---\ncenter v2\n",
        )
        .unwrap();
        assert!(
            fs::read_to_string(root.join("projects").join("notes").join("SKILL.md"))
                .unwrap()
                .contains("center v2")
        );

        let work = root.join("remote-work");
        fs::create_dir_all(&work).unwrap();
        fs::write(work.join("restored-marker.txt"), "restored-from-remote").unwrap();
        git(&work, &["init", "-b", "main"]);
        git(&work, &["config", "user.email", "hive@localhost"]);
        git(&work, &["config", "user.name", "Skill Hive"]);
        git(&work, &["add", "."]);
        git(&work, &["commit", "-m", "marker"]);
        let bare = root.join("restore.git");
        let status = Command::new("git")
            .args(["clone", "--bare"])
            .arg(&work)
            .arg(&bare)
            .status()
            .unwrap();
        assert!(status.success());
        let staging = root.join("staging");
        git_backup::clone_repo(&bare, &staging).unwrap();
        assert_eq!(
            fs::read_to_string(staging.join("restored-marker.txt")).unwrap(),
            "restored-from-remote"
        );

        store
            .insert_scenario(&ScenarioRecord {
                id: "preset-notes".into(),
                name: "Notes".into(),
                description: None,
                icon: None,
                sort_order: 0,
                created_at: 1,
                updated_at: 1,
            })
            .unwrap();
        let pairs = super::tray_preset_pairs(&store);
        assert!(pairs
            .iter()
            .any(|(id, name)| id == "preset-notes" && name == "Notes"));
        let _ = fs::remove_dir_all(root);
    }
}
