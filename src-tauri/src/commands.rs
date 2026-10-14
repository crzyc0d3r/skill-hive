use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use hive_core::analytics::{write_history, HistoryEntry};
use hive_core::authoring::skill::write_skill_file;
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
    save_named_provider, store_provider_secret, Flavor, ProviderConfig, ProviderKind,
};
use hive_core::testing::{save_suite, CheckSpec, Suite, SuiteCase, TriggerRow};
use hive_core::util::error::AppError;
use serde::Serialize;
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

#[tauri::command]
pub fn install_from_skillssh(
    source: String,
    name: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<InstalledDto, AppError> {
    let source = source
        .trim()
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
    let status = Command::new("git")
        .args(["clone", "--depth", "1", &url])
        .arg(&dest)
        .status()
        .map_err(AppError::io)?;
    if !status.success() {
        return Err(AppError::network(format!("could not clone {url}")));
    }
    let dirs = hive_core::library::skill_metadata::find_skill_dirs(&dest, 4);
    let folder = dirs
        .iter()
        .find(|dir| {
            read_skill_document(dir)
                .map(|doc| doc.name == sanitize_skill_name(&name))
                .unwrap_or(false)
        })
        .cloned()
        .or_else(|| dirs.into_iter().next())
        .ok_or_else(|| AppError::not_found("repository has no SKILL.md"))?;
    let installed = install_local(store.inner(), &root(), &folder)
        .map_err(|err| AppError::io(err.to_string()))?;
    store
        .inner()
        .set_skill_source_type(&installed.record.id, "skillssh")
        .map_err(db)?;
    let _ = std::fs::remove_dir_all(dest);
    Ok(installed_dto(installed.record.name, installed.record.id))
}

#[tauri::command]
pub fn record_effect(
    kind: String,
    id: String,
    body: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let kind = kind.trim();
    let id = if id.trim().is_empty() {
        kind.to_string()
    } else {
        id.trim().to_string()
    };
    match kind {
        "provider" => {
            let db_path = central_repo::db_path();
            save_named_provider(
                &db_path,
                &ProviderConfig {
                    id: id.clone(),
                    name: id.clone(),
                    kind: ProviderKind::OpenaiCompat,
                    flavor: Some(Flavor::Custom),
                    base_url: None,
                    default_model: None,
                    auth_mode: "stored".into(),
                    auth_env_var: None,
                    cli_program: None,
                    proxy_mode: "system".into(),
                    supports_tools: false,
                    max_retries: 0,
                    max_concurrency: 1,
                    role_generator: true,
                    role_grader: false,
                    role_executor: false,
                },
            )
            .map_err(AppError::internal)?;
            let secrets = central_repo::base_dir().join("secrets");
            std::fs::create_dir_all(&secrets)?;
            store_provider_secret(&id, &body, &secrets)
                .map_err(|err| AppError::io(err.to_string()))?;
            Ok(secrets.join(&id).to_string_lossy().to_string())
        }
        "suite" => {
            let dir = authoring_root("suite", &id);
            std::fs::create_dir_all(&dir)?;
            save_suite(
                &dir,
                &Suite {
                    skill_name: id.clone(),
                    evals: vec![SuiteCase {
                        id: 1,
                        prompt: body.clone(),
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
            Ok(dir
                .join("evals")
                .join("evals.json")
                .to_string_lossy()
                .to_string())
        }
        "history" => {
            let path = authoring_root("history", &id).join("history.json");
            write_history(
                &path,
                &id,
                &[HistoryEntry {
                    version: "v1".into(),
                    parent: None,
                    expectation_pass_rate: 1.0,
                    grading_result: "baseline".into(),
                    is_current_best: true,
                }],
            )
            .map_err(AppError::internal)?;
            Ok(path.to_string_lossy().to_string())
        }
        "compare" | "result" => {
            store
                .inner()
                .set_setting(&format!("{kind}:{id}"), &body)
                .map_err(db)?;
            Ok(format!("{kind}:{id}"))
        }
        "generate" => {
            let dir = authoring_root("generate", &id);
            let mut files = BTreeMap::new();
            files.insert("SKILL.md".into(), body);
            write_files(&dir, &files).map_err(AppError::io)?;
            Ok(dir.join("SKILL.md").to_string_lossy().to_string())
        }
        _ => {
            let dir = authoring_root(kind, &id);
            std::fs::create_dir_all(&dir)?;
            let file = if kind == "agent" {
                "AGENT.md"
            } else {
                "SKILL.md"
            };
            write_skill_file(&dir, file, &body)?;
            Ok(dir.join(file).to_string_lossy().to_string())
        }
    }
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
}
