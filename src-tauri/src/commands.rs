use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use hive_core::analytics::{apply_optimized, store_verdict, write_history, HistoryEntry};
use hive_core::authoring::skill::{publish_draft, write_skill_file, Deployment};
use hive_core::authoring::validator::{Finding, Severity};
use hive_core::authoring::versions::VersionStore;
use hive_core::core::git_backup;
use hive_core::core::git_fetcher;
use hive_core::core::installer::{self, InstallResult};
use hive_core::core::scenario_service::{self, BatchApplyMode};
use hive_core::core::sync_engine;
use hive_core::db::skill_store::{
    DiscoveredSkillRecord, ProjectRecord, ScenarioRecord, SkillRecord, SkillStore,
    SkillTargetRecord,
};
use hive_core::deploy::scenario_service as membership;
use hive_core::deploy::tool_adapters::{all_tool_adapters, CustomToolDef, ToolCategory};
use hive_core::deploy::tool_service::{self, CustomAdd};
use hive_core::library::central_repo;
use hive_core::library::marketplace::{self, fetch_https};
use hive_core::library::scanner::{self, ScannedSkill};
use hive_core::library::skill_check;
use hive_core::library::skill_metadata::{read_skill_document, sanitize_skill_name};
use hive_core::providers::{
    clear_git_token, clear_provider_secret, has_git_token, store_git_token, store_provider_secret,
};
use hive_core::testing::{save_suite, CheckSpec, Suite, SuiteCase, TriggerRow};
use hive_core::util::error::AppError;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

fn db(err: impl std::fmt::Display) -> AppError {
    AppError::db(err.to_string())
}

fn root() -> PathBuf {
    central_repo::skills_dir()
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

fn report(job: &str, current: u32, total: u32, message: &str) -> Result<(), AppError> {
    if crate::progress::step(job, current, total, message) {
        Ok(())
    } else {
        Err(AppError::invalid_input("cancelled"))
    }
}

#[derive(Clone, Serialize, Deserialize)]
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
    pub updated_at: i64,
    pub update_status: String,
    pub locations: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
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

#[derive(Clone, Serialize)]
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
    pub differs: bool,
    pub fingerprint: String,
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

struct CachedLibrary {
    path: String,
    rows: Vec<ManagedSkill>,
}

static LIBRARY_INDEX: Mutex<Option<CachedLibrary>> = Mutex::new(None);
static LIBRARY_GENERATION: AtomicU64 = AtomicU64::new(0);

struct CachedTools {
    at_ms: u64,
    rows: Vec<AgentDto>,
}

static TOOL_STATUS: Mutex<Option<CachedTools>> = Mutex::new(None);
const TOOL_STATUS_TTL_MS: u64 = 60_000;

fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

fn cached_tool_status() -> Option<Vec<AgentDto>> {
    let guard = TOOL_STATUS.lock().ok()?;
    let cached = guard.as_ref()?;
    if unix_ms().saturating_sub(cached.at_ms) > TOOL_STATUS_TTL_MS {
        return None;
    }
    Some(cached.rows.clone())
}

fn remember_tool_status(rows: &[AgentDto]) {
    if let Ok(mut guard) = TOOL_STATUS.lock() {
        *guard = Some(CachedTools {
            at_ms: unix_ms(),
            rows: rows.to_vec(),
        });
    }
}

fn invalidate_tool_status() {
    if let Ok(mut guard) = TOOL_STATUS.lock() {
        *guard = None;
    }
}

fn library_index_path() -> PathBuf {
    root().join(".skill-hive").join("library-index.json")
}

pub fn invalidate_library_index() {
    LIBRARY_GENERATION.fetch_add(1, Ordering::AcqRel);
    if let Ok(mut guard) = LIBRARY_INDEX.lock() {
        *guard = None;
    }
    let _ = std::fs::remove_file(library_index_path());
}

fn memory_index(path: &str) -> Option<Vec<ManagedSkill>> {
    let guard = LIBRARY_INDEX.lock().ok()?;
    let cached = guard.as_ref()?;
    if cached.path != path {
        return None;
    }
    Some(cached.rows.clone())
}

fn remember_index(path: &str, skills: &[ManagedSkill]) {
    if let Ok(mut guard) = LIBRARY_INDEX.lock() {
        *guard = Some(CachedLibrary {
            path: path.to_string(),
            rows: skills.to_vec(),
        });
    }
}

fn read_disk_index() -> Option<Vec<ManagedSkill>> {
    let text = std::fs::read_to_string(library_index_path()).ok()?;
    serde_json::from_str(&text).ok()
}

fn write_library_index(store: &SkillStore, skills: &[ManagedSkill]) {
    remember_index(&store.db_file(), skills);
    let path = library_index_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    hive_core::util::file_watcher::mute_self_writes(&path);
    if let Ok(text) = serde_json::to_string(skills) {
        let _ = std::fs::write(path, text);
    }
}

fn managed(store: &SkillStore) -> Result<Vec<ManagedSkill>, AppError> {
    if let Some(cached) = memory_index(&store.db_file()) {
        return Ok(cached);
    }
    rebuild_library(store)
}

fn rebuild_library(store: &SkillStore) -> Result<Vec<ManagedSkill>, AppError> {
    let generation = LIBRARY_GENERATION.load(Ordering::Acquire);
    crate::progress::emit("library", 0, 1, "Reading the library", false);
    let tags = store.get_tags_map().map_err(db)?;
    let targets = store.get_all_targets().map_err(db)?;
    let mut skills = Vec::new();
    for skill in store.get_all_skills().map_err(db)? {
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
            source_body: String::new(),
            files: Vec::new(),
            body: String::new(),
            agents,
            updated_at: skill.updated_at,
            update_status: skill.update_status.clone(),
            locations: vec![skill.central_path.clone()],
        });
    }
    let mut skills = collapse_managed(skills);
    skills.sort_by(|left, right| left.name.cmp(&right.name));
    for skill in &mut skills {
        skill.body.clear();
        skill.source_body.clear();
        skill.files.clear();
        skill.description = skill
            .description
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if skill.description.chars().count() > 180 {
            skill.description = skill.description.chars().take(180).collect();
        }
    }
    if LIBRARY_GENERATION.load(Ordering::Acquire) == generation {
        write_library_index(store, &skills);
    }
    crate::progress::emit("library", 1, 1, "Library ready", true);
    Ok(skills)
}

fn collapse_managed(skills: Vec<ManagedSkill>) -> Vec<ManagedSkill> {
    let mut order = Vec::new();
    let mut grouped = std::collections::HashMap::<String, ManagedSkill>::new();
    for skill in skills {
        let key = skill.name.to_ascii_lowercase();
        if let Some(existing) = grouped.get_mut(&key) {
            for agent in skill.agents {
                if !existing.agents.contains(&agent) {
                    existing.agents.push(agent);
                }
            }
            existing.agents.sort();
            for path in skill.locations {
                if !path.is_empty() && !existing.locations.contains(&path) {
                    existing.locations.push(path);
                }
            }
            continue;
        }
        order.push(key.clone());
        grouped.insert(key, skill);
    }
    order
        .into_iter()
        .filter_map(|key| grouped.remove(&key))
        .collect()
}

fn skill_tree_files(central: &Path, source: Option<&Path>) -> Vec<SkillFile> {
    let mut files = Vec::new();
    collect_skill_files(central, central, source, &mut files);
    if let Some(source) = source {
        let mut remote_only = Vec::new();
        collect_skill_files(source, source, Some(source), &mut remote_only);
        for file in remote_only {
            if files.iter().any(|existing| existing.path == file.path) {
                continue;
            }
            files.push(SkillFile {
                path: file.path,
                local: String::new(),
                remote: file.remote,
            });
        }
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    files
}

fn collect_skill_files(root: &Path, dir: &Path, source: Option<&Path>, files: &mut Vec<SkillFile>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name == ".git" || name == ".skill-hive" || name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            collect_skill_files(root, &path, source, files);
            continue;
        }
        if !path.is_file() {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let local = std::fs::read_to_string(&path).unwrap_or_default();
        let remote = source
            .map(|root| root.join(&relative))
            .filter(|path| path.is_file())
            .and_then(|path| std::fs::read_to_string(path).ok())
            .unwrap_or_else(|| local.clone());
        files.push(SkillFile {
            path: relative,
            local,
            remote,
        });
    }
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
    let found = scanner::scan_roots_with_central(store, &refs, Some(&central_repo::skills_dir()))
        .map_err(|err| AppError::io(err.to_string()))?;
    store.clear_discovered().map_err(db)?;
    let total = found.len() as u32;
    report("scan", 0, total.max(1), "Scanning local skills")?;
    for (index, skill) in found.iter().enumerate() {
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
        report("scan", index as u32 + 1, total.max(1), "Saving discoveries")?;
    }
    crate::progress::finish("scan", "Scan finished");
    Ok(found)
}

fn to_scan(rows: Vec<ScannedSkill>) -> Vec<ScanDto> {
    rows.into_iter()
        .map(|row| ScanDto {
            name: row.name,
            tool: row.tool,
            path: row.path,
            already_managed: row.already_managed,
            differs: row.differs,
            fingerprint: row.fingerprint,
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

struct HeldPreview {
    id: String,
    dir: PathBuf,
    owned: bool,
}

static PREVIEWS: Mutex<Vec<HeldPreview>> = Mutex::new(Vec::new());

fn persist_install(
    store: &SkillStore,
    installed: &InstallResult,
    source_type: &str,
    source_ref: Option<String>,
) -> Result<InstalledDto, AppError> {
    if let Some(existing) = store
        .get_all_skills()
        .map_err(db)?
        .into_iter()
        .find(|skill| {
            skill.name.eq_ignore_ascii_case(&installed.name)
                || skill.content_hash.as_deref() == Some(installed.content_hash.as_str())
        })
    {
        if Path::new(&existing.central_path) != installed.central_path.as_path()
            && installed.central_path.is_dir()
        {
            let _ = std::fs::remove_dir_all(&installed.central_path);
        }
        return Ok(installed_dto(existing.name, existing.id));
    }
    let stamp = now();
    let id = format!("{}-{stamp:x}", installed.name);
    store
        .insert_skill(&SkillRecord {
            id: id.clone(),
            name: installed.name.clone(),
            description: installed.description.clone(),
            source_type: source_type.to_string(),
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
    invalidate_library_index();
    Ok(installed_dto(installed.name.clone(), id))
}

fn sync_skill(store: &SkillStore, skill_id: &str, tool: &str, add: bool) -> Result<(), AppError> {
    let result = sync_skill_now(store, skill_id, tool, add);
    if result.is_ok() {
        invalidate_library_index();
    }
    result
}

fn sync_skill_now(
    store: &SkillStore,
    skill_id: &str,
    tool: &str,
    add: bool,
) -> Result<(), AppError> {
    let skill = store
        .get_skill_by_id(skill_id)
        .map_err(db)?
        .ok_or_else(|| AppError::not_found("Skill not found"))?;
    let adapter = all_tool_adapters(store)
        .into_iter()
        .find(|adapter| adapter.key == tool)
        .ok_or_else(|| AppError::not_found(format!("unknown agent {tool}")))?;
    let source = PathBuf::from(&skill.central_path);
    let target = adapter
        .skills_dir()
        .join(sync_engine::target_dir_name(&source, &skill.name));
    if !add {
        if let Some(row) = store
            .get_targets_for_skill(skill_id)
            .map_err(db)?
            .into_iter()
            .find(|row| row.tool == tool)
        {
            sync_engine::remove_recorded_target(Path::new(&row.target_path), &row.mode)
                .map_err(|err| AppError::io(err.to_string()))?;
            store.delete_target(skill_id, tool).map_err(db)?;
        }
        return Ok(());
    }
    let configured = store.get_setting("sync_mode").map_err(db)?;
    let actual = sync_engine::sync_skill(
        &source,
        &target,
        sync_engine::sync_mode_for_tool(tool, configured.as_deref()),
        sync_engine::ReplacePolicy::NoClobber,
    )
    .map_err(|err| AppError::io(err.to_string()))?;
    store
        .insert_target(&SkillTargetRecord {
            id: format!("target-{skill_id}-{tool}-{}", now()),
            skill_id: skill_id.to_string(),
            tool: tool.to_string(),
            target_path: target.to_string_lossy().to_string(),
            mode: actual.as_str().to_string(),
            status: "ok".into(),
            synced_at: Some(now()),
            last_error: None,
            source_hash: skill.content_hash.clone(),
        })
        .map_err(db)?;
    Ok(())
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

fn backup_repo() -> PathBuf {
    root()
}

fn ensure_backup_repo(path: &Path) -> Result<(), AppError> {
    if path.join(".git").exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir_all(path)?;
    git_backup::init_repo(path, &git_backup::default_device_name())
        .map_err(|err| AppError::git(err.to_string()))
}

fn is_bare(path: &Path) -> bool {
    matches!(
        git_ok(path, &["rev-parse", "--is-bare-repository"]).as_deref(),
        Ok("true")
    )
}

pub fn preview_repository(url: &str) -> Result<GitPreviewDto, AppError> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(AppError::invalid_input("Enter a repository URL or path"));
    }
    let path = PathBuf::from(trimmed);
    let owned;
    let source = if path.is_dir() && !is_bare(&path) {
        owned = false;
        path
    } else {
        let cancel = Arc::new(AtomicBool::new(false));
        owned = true;
        git_fetcher::clone_repo_ref(trimmed, None, Some(&cancel), None)
            .map_err(|err| AppError::classify_git_error(err.to_string()))?
    };
    let dirs = hive_core::library::skill_metadata::find_skill_dirs(&source, 4);
    let id = format!("preview-{}", now());
    let skills = dirs
        .iter()
        .map(|dir| {
            let relative = dir
                .strip_prefix(&source)
                .unwrap_or(dir)
                .to_string_lossy()
                .replace('\\', "/");
            let name = read_skill_document(dir)
                .map(|doc| doc.name)
                .unwrap_or_else(|_| sanitize_skill_name(&relative));
            PreviewSkillDto {
                relative: if relative.is_empty() {
                    ".".into()
                } else {
                    relative
                },
                name,
            }
        })
        .collect();
    PREVIEWS.lock().expect("previews").push(HeldPreview {
        id: id.clone(),
        dir: source,
        owned,
    });
    Ok(GitPreviewDto { id, skills })
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
pub async fn get_managed_skills(
    app: AppHandle,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<ManagedSkill>, AppError> {
    let store = store.inner().clone();
    let path = store.db_file();
    if let Some(cached) = memory_index(&path) {
        return Ok(cached);
    }
    if let Some(cached) = read_disk_index() {
        let generation = LIBRARY_GENERATION.load(Ordering::Acquire);
        remember_index(&path, &cached);
        let refreshing = store.clone();
        tauri::async_runtime::spawn(async move {
            let rebuilt =
                crate::offload::offload(move || rebuild_library(refreshing.as_ref())).await;
            if let Ok(rows) = rebuilt {
                if LIBRARY_GENERATION.load(Ordering::Acquire) == generation {
                    let _ = app.emit("library-index", &rows);
                }
            }
        });
        return Ok(cached);
    }
    crate::offload::offload(move || managed(store.as_ref())).await
}

#[tauri::command]
pub async fn get_skill_files(
    skill_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<SkillFile>, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
        let skill = store
            .get_skill_by_id(skill_id.trim())
            .map_err(db)?
            .ok_or_else(|| AppError::not_found("skill"))?;
        report("library", 0, 1, "Reading skill files")?;
        let source = source_dir(&skill);
        let files = skill_tree_files(Path::new(&skill.central_path), source.as_deref());
        crate::progress::finish("library", "Skill files ready");
        Ok(files)
    })
    .await
}

#[tauri::command]
pub async fn cancel_job() -> bool {
    crate::progress::request_cancel();
    crate::progress::finish("job", "Cancelled");
    true
}

#[tauri::command]
pub async fn list_presets(store: State<'_, Arc<SkillStore>>) -> Result<Vec<PresetDto>, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
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
    })
    .await
}

#[tauri::command]
pub async fn get_tool_status(store: State<'_, Arc<SkillStore>>) -> Result<Vec<AgentDto>, AppError> {
    if let Some(cached) = cached_tool_status() {
        return Ok(cached);
    }
    let store = store.inner().clone();
    let rows = crate::offload::offload(move || -> Result<Vec<AgentDto>, AppError> {
        let home = home_dir().ok_or_else(|| AppError::io("home directory is not set"))?;
        let rows = tool_service::tool_statuses(store.as_ref(), &home).map_err(db)?;
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
            .collect::<Vec<_>>())
    })
    .await?;
    remember_tool_status(&rows);
    Ok(rows)
}

#[tauri::command]
pub async fn scan_local_skills(
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<ScanDto>, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || Ok(to_scan(scan(store.as_ref())?))).await
}

#[tauri::command]
pub async fn install_local_skill(
    path: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<InstalledDto, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        crate::progress::reset_cancel();
        let source = PathBuf::from(path.trim());
        if !source.is_dir() {
            return Err(AppError::invalid_input("Choose a skill folder"));
        }
        report("install", 0, 2, "Installing from a folder")?;
        let installed = installer::install_from_local(&source, None)
            .map_err(|err| AppError::io(err.to_string()))?;
        report("install", 1, 2, "Saving the skill")?;
        let saved = persist_install(
            store.as_ref(),
            &installed,
            "local",
            Some(source.to_string_lossy().to_string()),
        );
        crate::progress::finish("install", "Installed");
        saved
    })
    .await
}

#[tauri::command]
pub async fn install_archive_skill(
    path: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<InstalledDto, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        crate::progress::reset_cancel();
        let archive = PathBuf::from(path.trim());
        if !archive.is_file() {
            return Err(AppError::invalid_input("Choose a .zip or .skill archive"));
        }
        report("install", 0, 2, "Installing from an archive")?;
        let installed = installer::install_from_local(&archive, None)
            .map_err(|err| AppError::io(err.to_string()))?;
        report("install", 1, 2, "Saving the skill")?;
        let saved = persist_install(
            store.as_ref(),
            &installed,
            "local",
            Some(archive.to_string_lossy().to_string()),
        );
        crate::progress::finish("install", "Installed");
        saved
    })
    .await
}

#[tauri::command]
pub async fn import_existing_skill(
    path: String,
    rename: Option<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<InstalledDto, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
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
    })
    .await
}

#[tauri::command]
pub async fn import_all_discovered(
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<String>, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        crate::progress::reset_cancel();
        let store = store.as_ref();
        report("import", 0, 2, "Finding skills to import")?;
        let found = scan(store)?;
        report("import", 1, 2, "Importing discovered skills")?;
        let names = scanner::import_all(store, &root(), &found)
            .map_err(|err| AppError::io(err.to_string()))?;
        crate::progress::finish("import", "Import finished");
        Ok(names)
    })
    .await
}

pub fn normalize_install_source(url: &str) -> (String, Option<String>) {
    let trimmed = url.trim();
    let bare = trimmed
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let Some(rest) = bare.strip_prefix("skills.sh/") else {
        return (trimmed.to_string(), None);
    };
    let parts: Vec<&str> = rest.split('/').filter(|part| !part.is_empty()).collect();
    if parts.len() < 2 {
        return (trimmed.to_string(), None);
    }
    let repo = format!("https://github.com/{}/{}", parts[0], parts[1]);
    let subpath = if parts.len() > 2 {
        Some(parts[2..].join("/"))
    } else {
        None
    };
    (repo, subpath)
}

#[tauri::command]
pub async fn preview_git_install(url: String) -> Result<GitPreviewDto, AppError> {
    crate::offload::offload(move || {
        let (source, subpath) = normalize_install_source(&url);
        let mut preview = preview_repository(&source)?;
        if let Some(subpath) = subpath {
            preview.skills.retain(|skill| {
                skill.relative == subpath
                    || skill.relative.ends_with(&format!("/{subpath}"))
                    || skill.name == subpath
            });
        }
        Ok(preview)
    })
    .await
}

#[tauri::command]
pub async fn confirm_git_install_skill(
    preview_id: String,
    relative: String,
    rename: Option<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<String>, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || confirm_preview(store.as_ref(), &preview_id, &relative, rename))
        .await
}

pub fn confirm_preview(
    store: &SkillStore,
    preview_id: &str,
    relative: &str,
    rename: Option<String>,
) -> Result<Vec<String>, AppError> {
    let held = {
        let guard = PREVIEWS.lock().expect("previews");
        guard
            .iter()
            .find(|preview| preview.id == preview_id)
            .map(|preview| preview.dir.clone())
            .ok_or_else(|| AppError::not_found("git preview expired"))?
    };
    let relative = relative.trim().trim_start_matches("./");
    let skill_dir = if relative.is_empty() || relative == "." {
        held
    } else {
        held.join(relative)
    };
    let rename = rename.filter(|name| !name.trim().is_empty());
    let installed = installer::install_from_git_dir(&skill_dir, rename.as_deref())
        .map_err(|err| AppError::classify_git_error(err.to_string()))?;
    let dto = persist_install(store, &installed, "git", Some(preview_id.to_string()))?;
    mark_source(store, &dto.name, "git")?;
    Ok(vec![dto.name])
}

pub fn cancel_install_now(preview_id: Option<String>) -> Result<bool, AppError> {
    if let Some(id) = preview_id.filter(|id| !id.is_empty()) {
        let mut guard = PREVIEWS.lock().expect("previews");
        if let Some(index) = guard.iter().position(|preview| preview.id == id) {
            let preview = guard.remove(index);
            if preview.owned {
                git_fetcher::cleanup_temp(&preview.dir);
            }
            return Ok(true);
        }
        drop(guard);
        if cancel_registered_install(&id) {
            if id == "market" || id.starts_with("market:") {
                let _ = cancel_market_install_now();
            }
            return Ok(true);
        }
    }
    Ok(false)
}

#[tauri::command]
pub async fn cancel_install(preview_id: Option<String>) -> Result<bool, AppError> {
    crate::offload::offload(move || cancel_install_now(preview_id)).await
}

static INSTALLS: Mutex<BTreeMap<String, bool>> = Mutex::new(BTreeMap::new());

pub fn register_install(key: &str) {
    if let Ok(mut installs) = INSTALLS.lock() {
        installs.insert(key.to_string(), false);
    }
}

pub fn install_is_cancelled(key: &str) -> bool {
    INSTALLS
        .lock()
        .ok()
        .and_then(|installs| installs.get(key).copied())
        .unwrap_or(false)
}

fn cancel_registered_install(key: &str) -> bool {
    let Ok(mut installs) = INSTALLS.lock() else {
        return false;
    };
    if !installs.contains_key(key) {
        return false;
    }
    installs.insert(key.to_string(), true);
    true
}

#[tauri::command]
pub async fn delete_managed_skills(
    ids: Vec<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
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
        invalidate_library_index();
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn set_skills_enabled(
    ids: Vec<String>,
    enabled: bool,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        for id in ids {
            store.update_skill_enabled(&id, enabled).map_err(db)?;
        }
        invalidate_library_index();
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn set_skill_tags(
    skill_id: String,
    tags: Vec<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        store.set_tags_for_skill(&skill_id, &tags).map_err(db)?;
        invalidate_library_index();
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn rename_tag(
    from: String,
    to: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<String>, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let names = store.as_ref().rename_tag(&from, &to).map_err(db)?;
        invalidate_library_index();
        Ok(names)
    })
    .await
}

#[tauri::command]
pub async fn delete_tag(
    tag: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<String>, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let names = store.as_ref().delete_tag(&tag).map_err(db)?;
        invalidate_library_index();
        Ok(names)
    })
    .await
}

#[tauri::command]
pub async fn sync_skill_to_tool(
    skill_id: String,
    tool: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || sync_skill(store.as_ref(), &skill_id, &tool, true)).await
}

#[tauri::command]
pub async fn unsync_skill_from_tool(
    skill_id: String,
    tool: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || sync_skill(store.as_ref(), &skill_id, &tool, false)).await
}

#[tauri::command]
pub async fn add_skill_to_preset(
    preset_id: String,
    skill_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let tools = tool_service::installed_keys(
            store.as_ref(),
            &home_dir().unwrap_or_else(|| PathBuf::from(".")),
        )
        .map_err(db)?;
        membership::add_membership(store.as_ref(), &preset_id, &skill_id, &tools).map_err(db)
    })
    .await
}

#[tauri::command]
pub async fn remove_skill_from_preset(
    preset_id: String,
    skill_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        store
            .remove_skill_from_scenario(&preset_id, &skill_id)
            .map_err(db)
    })
    .await
}

#[tauri::command]
pub async fn reorder_preset_skills(
    preset_id: String,
    skill_ids: Vec<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        store
            .reorder_scenario_skills(&preset_id, &skill_ids)
            .map_err(db)
    })
    .await
}

#[tauri::command]
pub async fn create_preset(
    name: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::invalid_input("Preset name is required"));
        }
        let id = format!("preset-{}", now());
        let stamp = now();
        store
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
    })
    .await
}

#[tauri::command]
pub async fn update_preset(
    id: String,
    name: String,
    description: Option<String>,
    icon: Option<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        update_preset_record(store.as_ref(), &id, &name, description, icon)
    })
    .await
}

pub fn update_preset_record(
    store: &SkillStore,
    id: &str,
    name: &str,
    description: Option<String>,
    icon: Option<String>,
) -> Result<(), AppError> {
    let existing = store
        .get_all_scenarios()
        .map_err(db)?
        .into_iter()
        .find(|row| row.id == id);
    let description =
        description.or_else(|| existing.as_ref().and_then(|row| row.description.clone()));
    let icon = icon.or_else(|| existing.as_ref().and_then(|row| row.icon.clone()));
    store
        .update_scenario(id, name.trim(), description.as_deref(), icon.as_deref())
        .map_err(db)
}

#[tauri::command]
pub async fn delete_preset(id: String, store: State<'_, Arc<SkillStore>>) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || store.as_ref().delete_scenario(&id).map_err(db)).await
}

#[tauri::command]
pub async fn reorder_presets(
    ids: Vec<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || store.as_ref().reorder_scenarios(&ids).map_err(db)).await
}

#[tauri::command]
pub async fn apply_preset(
    preset_id: String,
    tool: Option<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<usize, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
        if let Some(tool) = tool.filter(|key| !key.is_empty()) {
            let skills = store.get_skill_ids_for_scenario(&preset_id).map_err(db)?;
            scenario_service::apply_skills_to_tools(store, &skills, &[tool], BatchApplyMode::Add)?;
            return Ok(skills.len());
        }
        apply_preset_by_id(store, &preset_id)
    })
    .await
}

#[tauri::command]
pub async fn add_custom_tool(
    name: String,
    path: String,
    project_path: Option<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<AgentDto, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let name = name.trim();
        let path = path.trim();
        if name.is_empty() || path.is_empty() {
            return Err(AppError::invalid_input(
                "Agent name and skills path are required",
            ));
        }
        let key = slug(name);
        let added = tool_service::add_custom_tool(
            store.as_ref(),
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
        invalidate_tool_status();
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
    })
    .await
}

#[tauri::command]
pub async fn remove_custom_tool(
    key: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        tool_service::remove_custom_tool(store.as_ref(), &key).map_err(db)?;
        invalidate_tool_status();
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn set_tool_enabled(
    key: String,
    enabled: bool,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        tool_service::set_tool_enabled(store.as_ref(), &key, enabled).map_err(db)?;
        invalidate_tool_status();
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn set_all_tools_enabled(
    enabled: bool,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        if enabled {
            tool_service::enable_all(store.as_ref()).map_err(db)?;
        } else {
            tool_service::disable_all(store.as_ref()).map_err(db)?;
        }
        invalidate_tool_status();
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn set_tool_order_cmd(
    keys: Vec<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        tool_service::reorder_tools(store.as_ref(), &keys).map_err(db)?;
        invalidate_tool_status();
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn set_custom_tool_path(
    key: String,
    path: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        tool_service::set_tool_path(store.as_ref(), &key, path.trim()).map_err(db)?;
        invalidate_tool_status();
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn reset_custom_tool_path(
    key: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
        tool_service::reset_tool_path(store, &key).map_err(db)?;
        invalidate_tool_status();
        tool_dir(store, &key).map(|path| path.to_string_lossy().to_string())
    })
    .await
}

#[tauri::command]
pub async fn set_custom_tool_project_path(
    key: String,
    path: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        tool_service::set_project_path(store.as_ref(), &key, path.trim()).map_err(db)?;
        invalidate_tool_status();
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn get_central_repo_path() -> String {
    crate::offload::offload(move || central_repo::base_dir().to_string_lossy().to_string()).await
}

#[tauri::command]
pub async fn set_central_repo_path(path: String) -> Result<String, AppError> {
    crate::offload::offload(move || {
        let trimmed = path.trim();
        let override_path = if trimmed.is_empty() || trimmed == "~/.skill-hive" {
            None
        } else {
            Some(trimmed.to_string())
        };
        central_repo::set_base_dir_override(override_path)
            .map(|path| path.to_string_lossy().to_string())
            .map_err(|err| AppError::io(err.to_string()))
    })
    .await
}

fn source_dir(skill: &SkillRecord) -> Option<PathBuf> {
    let raw = skill
        .source_ref_resolved
        .as_deref()
        .or(skill.source_ref.as_deref())
        .filter(|value| !value.trim().is_empty())?;
    let path = PathBuf::from(raw.trim());
    if path.is_file() {
        path.parent().map(Path::to_path_buf)
    } else {
        Some(path)
    }
}

fn check_ttl_ms(store: &SkillStore) -> i64 {
    match store
        .get_setting("update_check_ttl_minutes")
        .ok()
        .flatten()
        .and_then(|value| value.trim().parse::<i64>().ok())
    {
        Some(minutes) if minutes <= 0 => 0,
        Some(minutes) => minutes.saturating_mul(60_000),
        None => 60 * 60_000,
    }
}

fn remember_check(store: &SkillStore, skill: &SkillRecord) -> Result<String, AppError> {
    if let Some(status) = fresh_enough(skill, check_ttl_ms(store)) {
        return Ok(status);
    }
    let (status, detail) =
        skill_check::classify_dirs(Path::new(&skill.central_path), source_dir(skill).as_deref());
    store
        .update_skill_check_state(&skill.id, None, &status, detail.as_deref())
        .map_err(db)?;
    Ok(status)
}

fn apply_engine_preference(store: &SkillStore) {
    let engine = store
        .get_setting("git_backup_engine")
        .ok()
        .flatten()
        .unwrap_or_default();
    let proxy = store.get_setting("proxy_url").ok().flatten();
    hive_core::core::git2_engine::set_preference(engine == "git2", proxy);
}

pub fn maybe_run_auto_update(store: &SkillStore, now_ms: i64) -> Result<u32, AppError> {
    let interval = store
        .get_setting("auto_update_interval")
        .ok()
        .flatten()
        .unwrap_or_default();
    if interval != "daily" {
        return Ok(0);
    }
    let last = store
        .get_setting("auto_update_last_run_ms")
        .ok()
        .flatten()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0);
    if last > 0 && now_ms.saturating_sub(last) < 86_400_000 {
        return Ok(0);
    }
    store
        .set_setting("auto_update_last_run_ms", &now_ms.to_string())
        .map_err(db)?;
    let found = check_updates_in(store)?;
    let apply = store.get_setting("auto_update_apply").ok().flatten();
    if matches!(apply.as_deref(), Some("1") | Some("true")) {
        apply_available_in(store)?;
    }
    Ok(found)
}

pub fn check_updates_in(store: &SkillStore) -> Result<u32, AppError> {
    let skills = store.get_all_skills().map_err(db)?;
    let total = skills.len() as u32;
    let ttl = check_ttl_ms(store);
    let mut available = 0u32;
    let mut pending = Vec::new();
    report("scan", 0, total.max(1), "Checking for updates")?;
    for skill in &skills {
        if crate::progress::cancelled() {
            return Err(AppError::invalid_input("cancelled"));
        }
        if let Some(status) = fresh_enough(skill, ttl) {
            if status == "update_available" || status == "pending_removals" {
                available += 1;
            }
            continue;
        }
        let (status, detail) = skill_check::classify_dirs(
            Path::new(&skill.central_path),
            source_dir(skill).as_deref(),
        );
        if status == "update_available" || status == "pending_removals" {
            available += 1;
        }
        pending.push((skill.id.clone(), status, detail));
    }
    if !pending.is_empty() {
        store.update_skill_check_states(&pending).map_err(db)?;
    }
    crate::progress::finish("scan", "Update check finished");
    Ok(available)
}

fn fresh_enough(skill: &SkillRecord, ttl: i64) -> Option<String> {
    if ttl <= 0 {
        return None;
    }
    let checked = skill.last_checked_at?;
    let status = skill.update_status.trim();
    if now().saturating_sub(checked) < ttl && !status.is_empty() && status != "unknown" {
        Some(status.to_string())
    } else {
        None
    }
}

#[tauri::command]
pub async fn check_all_skill_updates(store: State<'_, Arc<SkillStore>>) -> Result<u32, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || check_updates_in(store.as_ref())).await
}

#[tauri::command]
pub async fn check_skill_update(
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
        remember_check(store, &skill)
    })
    .await
}

#[tauri::command]
pub async fn apply_available_skill_updates(
    store: State<'_, Arc<SkillStore>>,
) -> Result<u32, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || apply_available_in(store.as_ref())).await
}

pub fn apply_available_in(store: &SkillStore) -> Result<u32, AppError> {
    let skills = store.get_all_skills().map_err(db)?;
    let total = skills.len() as u32;
    let mut applied = 0u32;
    for (index, skill) in skills.into_iter().enumerate() {
        report("sync", index as u32, total.max(1), "Applying updates")?;
        let status = remember_check(store, &skill)?;
        if status != "update_available" {
            continue;
        }
        let Some(source) = source_dir(&skill) else {
            continue;
        };
        if !source.join("SKILL.md").is_file() && !source.is_dir() {
            continue;
        }
        let held =
            hive_core::core::removals::removed_paths(Path::new(&skill.central_path), &source)
                .map_err(|err| AppError::io(err.to_string()))?;
        if !held.is_empty() {
            store
                .update_skill_check_state(
                    &skill.id,
                    None,
                    "pending_removals",
                    Some(&held.join("\n")),
                )
                .map_err(db)?;
            continue;
        }
        copy_source_tree(&source, Path::new(&skill.central_path))?;
        let _ = remember_check(store, &skill);
        applied += 1;
    }
    crate::progress::finish("sync", "Updates applied");
    Ok(applied)
}

fn copy_source_tree(source: &Path, dest: &Path) -> Result<(), AppError> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(source).map_err(AppError::io)? {
        let entry = entry.map_err(AppError::io)?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == ".git" || name == ".skill-hive" {
            continue;
        }
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if from.is_dir() {
            copy_source_tree(&from, &to)?;
        } else if from.is_file() {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn install_manage_skills(
    store: State<'_, Arc<SkillStore>>,
) -> Result<InstalledDto, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
        if let Some(existing) = store
            .get_all_skills()
            .map_err(db)?
            .into_iter()
            .find(|skill| skill.name == "manage-skills")
        {
            return Ok(installed_dto(existing.name, existing.id));
        }
        let dir = central_repo::base_dir()
            .join("sources")
            .join("manage-skills");
        std::fs::create_dir_all(&dir)?;
        let document = dir.join("SKILL.md");
        if !document.is_file() {
            std::fs::write(
                &document,
                "---\nname: manage-skills\ndescription: Install, update, and remove skills from the Skill Hive library.\n---\n# manage-skills\nUse the library to install and update skills for the connected agents.\n",
            )?;
        }
        let installed =
            installer::install_from_local(&dir, None).map_err(|err| AppError::io(err.to_string()))?;
        persist_install(
            store,
            &installed,
            "local",
            Some(dir.to_string_lossy().to_string()),
        )
    }).await
}

#[tauri::command]
pub async fn git_backup_status(
    store: State<'_, Arc<SkillStore>>,
) -> Result<BackupStatusDto, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        apply_engine_preference(store.as_ref());
        let status =
            git_backup::get_status(&backup_repo()).map_err(|err| AppError::git(err.to_string()))?;
        Ok(BackupStatusDto {
            branch: status.branch.unwrap_or_default(),
            clean: !status.has_changes,
            remote: status.remote_url,
            head: status.last_commit.unwrap_or_default(),
        })
    })
    .await
}

#[tauri::command]
pub async fn git_backup_list_versions(limit: Option<u32>) -> Result<Vec<SnapshotDto>, AppError> {
    crate::offload::offload(move || {
        let path = backup_repo();
        if !path.join(".git").exists() {
            return Ok(Vec::new());
        }
        let versions = git_backup::list_snapshot_versions(&path, limit.map(|value| value as usize))
            .map_err(|err| AppError::git(err.to_string()))?;
        Ok(versions
            .into_iter()
            .map(|version| SnapshotDto {
                id: version.tag.clone(),
                label: format!("{} {}", version.tag, version.message),
            })
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn git_backup_sync(message: Option<String>) -> Result<SnapshotDto, AppError> {
    crate::offload::offload(move || {
        crate::progress::reset_cancel();
        let path = backup_repo();
        report("sync", 0, 3, "Preparing the backup")?;
        ensure_backup_repo(&path)?;
        let note = message.unwrap_or_else(|| "Backup".to_string());
        match git_backup::commit_all(&path, &note) {
            Ok(()) => {}
            Err(err) if err.to_string().contains("Nothing to commit") => {}
            Err(err) => return Err(AppError::git(err.to_string())),
        }
        report("sync", 1, 3, "Syncing the backup")?;
        if git_backup::get_status(&path)
            .ok()
            .and_then(|status| status.remote_url)
            .is_some()
        {
            let _ = git_backup::pull(&path);
            let _ = git_backup::push(&path);
        }
        report("sync", 2, 3, "Tagging the snapshot")?;
        let tag =
            git_backup::create_snapshot_tag(&path).map_err(|err| AppError::git(err.to_string()))?;
        crate::progress::finish("sync", "Backup synced");
        Ok(SnapshotDto {
            id: tag.clone(),
            label: tag,
        })
    })
    .await
}

#[tauri::command]
pub async fn git_backup_restore_version(tag: String) -> Result<String, AppError> {
    crate::offload::offload(move || {
        git_backup::restore_snapshot_version(&backup_repo(), tag.trim())
            .map_err(|err| AppError::git(err.to_string()))
    })
    .await
}

#[tauri::command]
pub async fn git_backup_set_remote(
    url: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let path = backup_repo();
        ensure_backup_repo(&path)?;
        git_backup::set_remote(&path, url.trim()).map_err(|err| AppError::git(err.to_string()))?;
        store
            .set_setting("git_backup_remote_url", url.trim())
            .map_err(db)
    })
    .await
}

#[tauri::command]
pub async fn git_backup_remove_remote(store: State<'_, Arc<SkillStore>>) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let path = backup_repo();
        if path.join(".git").exists() {
            git_backup::remove_remote(&path).map_err(|err| AppError::git(err.to_string()))?;
        }
        store.set_setting("git_backup_remote_url", "").map_err(db)
    })
    .await
}

#[tauri::command]
pub async fn backup_set_device_name(
    name: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let store = store.as_ref();
        let trimmed = name.trim();
        store
            .set_setting("backup_device_name", trimmed)
            .map_err(db)?;
        let path = backup_repo();
        if path.join(".git").exists() {
            git_backup::configure_device_identity(&path, trimmed)
                .map_err(|err| AppError::git(err.to_string()))?;
        }
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn add_project(
    name: String,
    path: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
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
    })
    .await
}

#[tauri::command]
pub async fn fetch_leaderboard(board: Option<String>) -> Result<Vec<MarketDto>, AppError> {
    crate::offload::offload(move || {
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
        let skills = marketplace::parse_leaderboard(&html)
            .map_err(|err| AppError::network(err.to_string()))?;
        Ok(skills
            .into_iter()
            .map(|skill| MarketDto {
                owner: skill.owner,
                name: skill.name,
                installs: skill.installs,
                source: skill.source,
            })
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn search_skillssh(
    query: String,
    limit: Option<u32>,
) -> Result<Vec<MarketDto>, AppError> {
    crate::offload::offload(move || {
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
    })
    .await
}

fn install_market_work(
    source: &str,
    name: &str,
    subpath: Option<&str>,
    store: &SkillStore,
) -> Result<InstalledDto, AppError> {
    MARKET_CANCEL.store(false, Ordering::SeqCst);
    let raw = source.trim().to_string();
    if market_cancelled_during_pause() {
        return Err(AppError::invalid_input("marketplace install cancelled"));
    }
    if Path::new(&raw).is_dir() {
        let folder = match subpath.filter(|value| !value.is_empty()) {
            Some(subpath) => Path::new(&raw).join(subpath),
            None => PathBuf::from(&raw),
        };
        let installed = installer::install_from_local(&folder, None)
            .map_err(|err| AppError::io(err.to_string()))?;
        let dto = persist_install(store, &installed, "skillssh", Some(raw.clone()))?;
        store
            .set_skill_source_type(&dto.id, "skillssh")
            .map_err(db)?;
        return Ok(dto);
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
            if let Some(subpath) = subpath.filter(|value| !value.is_empty()) {
                let relative = dir
                    .strip_prefix(&dest)
                    .unwrap_or(dir)
                    .to_string_lossy()
                    .replace('\\', "/");
                return relative == subpath || relative.ends_with(&format!("/{subpath}"));
            }
            read_skill_document(dir)
                .map(|doc| doc.name == sanitize_skill_name(name))
                .unwrap_or(false)
        })
        .cloned()
        .or_else(|| dirs.into_iter().next())
        .ok_or_else(|| AppError::not_found("repository has no SKILL.md"))?;
    let installed = installer::install_from_git_dir(&folder, Some(name))
        .map_err(|err| AppError::io(err.to_string()))?;
    let dto = persist_install(store, &installed, "skillssh", Some(url.clone()))?;
    store
        .set_skill_source_type(&dto.id, "skillssh")
        .map_err(db)?;
    let _ = std::fs::remove_dir_all(dest);
    Ok(dto)
}

fn set_market_status(value: &str) {
    if let Ok(mut slot) = MARKET_STATUS.lock() {
        *slot = value.to_string();
    }
}

#[tauri::command]
pub async fn install_from_skillssh(
    source: String,
    name: String,
    subpath: Option<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<InstalledDto, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let key = format!("market:{source}");
        register_install(&key);
        if install_is_cancelled(&key) {
            return Err(AppError::invalid_input("marketplace install cancelled"));
        }
        install_market_work(&source, &name, subpath.as_deref(), store.as_ref())
    })
    .await
}

#[tauri::command]
pub async fn begin_market_install(
    source: String,
    name: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<bool, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        set_market_status("running");
        let store = store.clone();
        std::thread::spawn(move || {
            let status = match install_market_work(&source, &name, None, &store) {
                Ok(installed) => format!("installed:{}:{}", installed.id, installed.name),
                Err(err) => format!("error:{err}"),
            };
            set_market_status(&status);
        });
        Ok(true)
    })
    .await
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

fn cancel_market_install_now() -> Result<bool, AppError> {
    MARKET_CANCEL.store(true, Ordering::SeqCst);
    if let Some(pid) = MARKET_PID.lock().ok().and_then(|mut slot| slot.take()) {
        kill_pid(pid);
    }
    Ok(true)
}

#[tauri::command]
pub async fn cancel_market_install() -> Result<bool, AppError> {
    crate::offload::offload(cancel_market_install_now).await
}

#[tauri::command]
pub async fn save_skill_draft(id: String, files: Vec<DraftFileArg>) -> Result<String, AppError> {
    crate::offload::offload(move || {
        let dir = authoring_root("skill", &id);
        std::fs::create_dir_all(&dir)?;
        for file in files {
            write_skill_file(&dir, &file.path, &file.content)?;
        }
        let version = VersionStore::open(&dir)
            .save_autosave()
            .map_err(|err| AppError::io(err.to_string()))?;
        Ok(version.id)
    })
    .await
}

#[tauri::command]
pub async fn snapshot_skill_draft(id: String, label: String) -> Result<String, AppError> {
    crate::offload::offload(move || {
        let dir = authoring_root("skill", &id);
        std::fs::create_dir_all(&dir)?;
        let version = VersionStore::open(&dir)
            .snapshot(Some(label.trim()))
            .map_err(|err| AppError::io(err.to_string()))?;
        Ok(version.id)
    })
    .await
}

#[tauri::command]
pub async fn restore_skill_draft(id: String, version_id: String) -> Result<String, AppError> {
    crate::offload::offload(move || {
        let dir = authoring_root("skill", &id);
        let version = VersionStore::open(&dir)
            .restore(version_id.trim())
            .map_err(|err| AppError::io(err.to_string()))?;
        Ok(version.id)
    })
    .await
}

#[tauri::command]
pub async fn publish_skill_draft(
    id: String,
    confirmed: bool,
    deployments: Vec<DeploymentArg>,
) -> Result<String, AppError> {
    crate::offload::offload(move || {
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
    })
    .await
}

#[tauri::command]
pub async fn fix_skill_draft(
    id: String,
    fix_id: String,
    path: Option<String>,
    message: Option<String>,
) -> Result<bool, AppError> {
    crate::offload::offload(move || {
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
    })
    .await
}

fn apply_fix(dir: &Path, finding: &Finding) -> Result<bool, AppError> {
    hive_core::authoring::skill::apply_skill_fix(dir, finding)
}

fn save_agent_draft_now(id: &str, text: &str) -> Result<String, AppError> {
    let dir = authoring_root("agent", id);
    std::fs::create_dir_all(&dir)?;
    write_skill_file(&dir, "AGENT.md", text)?;
    Ok(dir.join("AGENT.md").to_string_lossy().to_string())
}

#[tauri::command]
pub async fn save_agent_draft(id: String, text: String) -> Result<String, AppError> {
    crate::offload::offload(move || save_agent_draft_now(&id, &text)).await
}

#[tauri::command]
pub async fn export_agent_draft(id: String, text: String) -> Result<String, AppError> {
    crate::offload::offload(move || {
        let saved = save_agent_draft_now(&id, &text)?;
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
    })
    .await
}

#[tauri::command]
pub async fn interview_for_goal(goal: String) -> Result<String, AppError> {
    crate::offload::offload(move || {
        let goal = goal.trim();
        if goal.is_empty() {
            return Err(AppError::invalid_input("A goal is required"));
        }
        let (provider, model, secret) = generation_provider()?;
        let parsed = hive_core::testing::interview_goal(provider.as_ref(), &model, goal, &secret)
            .map_err(AppError::io)?;
        let questions = parsed
            .get("questions")
            .cloned()
            .unwrap_or(serde_json::json!([]));
        serde_json::to_string(&questions).map_err(AppError::io)
    })
    .await
}

#[tauri::command]
pub fn endpoint_prices() -> Result<String, AppError> {
    let rows: Vec<serde_json::Value> = hive_core::providers::seed_models()
        .iter()
        .filter_map(|model| {
            hive_core::providers::Pricing::anthropic_defaults(model).map(|price| {
                serde_json::json!({
                    "model": model,
                    "input_per_mtok": price.input_per_mtok,
                    "output_per_mtok": price.output_per_mtok,
                })
            })
        })
        .collect();
    serde_json::to_string(&rows).map_err(AppError::io)
}

fn save_generated_draft_now(goal: &str) -> Result<String, AppError> {
    let goal = goal.trim();
    if goal.is_empty() {
        return Err(AppError::invalid_input("A goal is required"));
    }
    let name = sanitize_skill_name(goal);
    let name = if name.is_empty() {
        "generated".to_string()
    } else {
        name
    };
    let dir = authoring_root("generate", "draft");
    std::fs::create_dir_all(&dir)?;
    let dest = dir.join("SKILL.md");
    let drafted = match generation_provider() {
        Ok((provider, model, secret)) => hive_core::testing::send_templates(
            provider.as_ref(),
            &model,
            &name,
            goal,
            "skill",
            &secret,
        )
        .map_err(AppError::io)?,
        Err(err) => return Err(err),
    };
    let body = format!("---\nname: {name}\ndescription: {goal}\n---\n\n{goal}\n\n{drafted}\n");
    std::fs::write(&dest, body)?;
    let text = std::fs::read_to_string(&dest)?;
    if !text.contains(goal) && !text.contains(&name) {
        return Err(AppError::io("generated draft did not keep the goal"));
    }
    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn save_generated_draft(goal: String) -> Result<String, AppError> {
    crate::offload::offload(move || save_generated_draft_now(&goal)).await
}

fn generation_provider(
) -> Result<(Box<dyn hive_core::providers::ChatProvider>, String, String), AppError> {
    let rows = hive_core::providers::list_named_providers(&central_repo::db_path())
        .map_err(AppError::db)?;
    let Some(row) = rows
        .into_iter()
        .find(|row| row.base_url.as_ref().is_some_and(|url| !url.is_empty()))
    else {
        return Err(AppError::invalid_input(
            "live generation is unavailable without a vendor API key",
        ));
    };
    let api_key = std::env::var("SKILL_HIVE_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            hive_core::providers::load_provider_secret(
                &row.id,
                &central_repo::base_dir().join("provider-secrets"),
            )
            .ok()
        });
    let Some(api_key) = api_key else {
        return Err(AppError::invalid_input(
            "live generation is unavailable without a vendor API key",
        ));
    };
    let secret = hive_core::providers::SecretString::new(api_key.clone());
    let model = row
        .default_model
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "claude-sonnet-5-5".to_string());
    let url = row.base_url.unwrap_or_default();
    let provider: Box<dyn hive_core::providers::ChatProvider> = match row.kind {
        hive_core::providers::ProviderKind::OpenaiCompat => {
            Box::new(hive_core::providers::OpenAiProvider::new(
                "generator",
                url,
                hive_core::providers::Flavor::Openai,
                secret,
            ))
        }
        hive_core::providers::ProviderKind::Anthropic => Box::new(
            hive_core::providers::AnthropicProvider::new("generator", url, secret),
        ),
        hive_core::providers::ProviderKind::LocalCli => {
            return Err(AppError::invalid_input(
                "live generation is unavailable without a vendor API key",
            ));
        }
    };
    Ok((provider, model, api_key))
}

#[tauri::command]
pub async fn apply_optimized_description(description: String) -> Result<String, AppError> {
    crate::offload::offload(move || {
        let dir = authoring_root("generate", "draft");
        if !dir.join("SKILL.md").is_file() {
            save_generated_draft_now(&description)?;
        }
        let text = description.trim();
        let description = if text.is_empty() {
            "Optimized description"
        } else {
            text
        };
        let optimization = dir.join("optimization.json");
        if optimization.is_file() {
            let saved: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&optimization).unwrap_or_default())
                    .unwrap_or(serde_json::Value::Null);
            if saved.get("restored").and_then(|value| value.as_bool()) == Some(false) {
                return Err(AppError::invalid_input(
                    "held-out score regressed, so the description stays unapplied",
                ));
            }
        }
        apply_optimized(&dir, description).map_err(AppError::io)?;
        let suite_dir = authoring_root("suite", "optimized");
        std::fs::create_dir_all(&suite_dir)?;
        save_suite(
            &suite_dir,
            &Suite {
                skill_name: "optimized".into(),
                evals: vec![SuiteCase {
                    id: 1,
                    prompt: description.to_string(),
                    expected_output: description.to_string(),
                    files: Vec::new(),
                    expectations: vec![description.to_string()],
                }],
                triggers: vec![TriggerRow {
                    query: description.to_string(),
                    should_trigger: true,
                    rationale: None,
                }],
                checks: vec![CheckSpec {
                    id: "description".into(),
                    expectation: description.to_string(),
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
    })
    .await
}

#[tauri::command]
pub async fn get_effectiveness_report() -> Result<String, AppError> {
    crate::offload::offload(move || {
        let path = central_repo::base_dir()
            .join("effectiveness")
            .join("last.json");
        if !path.is_file() {
            return Ok(String::new());
        }
        std::fs::read_to_string(path).map_err(AppError::io)
    })
    .await
}

#[tauri::command]
pub async fn run_effectiveness_suite(
    skill_path: String,
    adapter: Option<String>,
    repetitions: Option<u32>,
) -> Result<String, AppError> {
    crate::offload::offload(move || {
        crate::progress::reset_cancel();
        let skill = PathBuf::from(skill_path.trim());
        if !skill.join("SKILL.md").is_file() {
            return Err(AppError::invalid_input(
                "the skill directory has no SKILL.md",
            ));
        }
        report("effectiveness", 0, 4, "Reading the provider")?;
        let rows = hive_core::providers::list_named_providers(&central_repo::db_path())
            .map_err(AppError::db)?;
        let Some(row) = rows
            .into_iter()
            .find(|row| row.base_url.as_ref().is_some_and(|url| !url.is_empty()))
        else {
            return Err(AppError::invalid_input(
                "add a provider with a base URL before measuring",
            ));
        };
        let kind = match row.kind {
            hive_core::providers::ProviderKind::OpenaiCompat => "openai",
            hive_core::providers::ProviderKind::Anthropic => "anthropic",
            hive_core::providers::ProviderKind::LocalCli => {
                return Err(AppError::invalid_input(
                    "measuring needs an Anthropic or OpenAI adapter",
                ));
            }
        };
        let api_key = std::env::var("SKILL_HIVE_API_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .or_else(|| {
                hive_core::providers::load_provider_secret(
                    &row.id,
                    &central_repo::base_dir().join("provider-secrets"),
                )
                .ok()
            });
        let Some(api_key) = api_key else {
            return Err(AppError::invalid_input(
                "measuring needs SKILL_HIVE_API_KEY or a stored provider secret",
            ));
        };
        if !skill.join("evals").join("evals.json").is_file() {
            let name = skill
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("skill");
            hive_core::testing::write_default_suite(
                &skill,
                name,
                &format!("Complete this task: {name}"),
            )
            .map_err(AppError::io)?;
        }
        let model = row
            .default_model
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "claude-sonnet-5-5".to_string());
        let root = central_repo::base_dir().join("test-runs");
        std::fs::create_dir_all(&root)?;
        report("effectiveness", 1, 4, "Running the measured suite")?;
        if crate::progress::cancelled() {
            return Err(AppError::invalid_input("cancelled"));
        }
        let report = hive_core::testing::run_measured(&hive_core::testing::MeasureRequest {
            skill_dir: skill,
            workspace: root.join("workspace"),
            db_path: root.join("hive.db"),
            base_url: row.base_url.unwrap_or_default(),
            api_key,
            kind: kind.to_string(),
            flavor: None,
            models: vec![hive_core::testing::ModelChoice {
                id: model,
                strong: true,
            }],
            adapter: adapter.unwrap_or_else(|| "emulated".to_string()),
            repetitions: repetitions.unwrap_or(3),
            threshold: 0.5,
            pass_bar: 0.5,
            distractors: Vec::new(),
            seed: 7,
            cli_program: None,
            cli_prefix: Vec::new(),
        })
        .map_err(AppError::io)?;
        let text = serde_json::to_string(&report).map_err(AppError::io)?;
        let saved = central_repo::base_dir().join("effectiveness");
        std::fs::create_dir_all(&saved)?;
        std::fs::write(saved.join("last.json"), &text)?;
        crate::progress::finish("effectiveness", "Measurement saved");
        Ok(text)
    })
    .await
}

#[tauri::command]
pub async fn optimize_skill_description(
    skill_path: String,
    rounds: Option<u32>,
) -> Result<String, AppError> {
    crate::offload::offload(move || {
        let skill = PathBuf::from(skill_path.trim());
        if !skill.join("SKILL.md").is_file() {
            return Err(AppError::invalid_input(
                "the skill directory has no SKILL.md",
            ));
        }
        let endpoint = measured_endpoint()?;
        let model = endpoint.model;
        let secret = endpoint.secret;
        let url = endpoint.url;
        let kind = endpoint.kind;
        let root = central_repo::base_dir().join("test-runs");
        std::fs::create_dir_all(&root)?;
        let saved = hive_core::testing::optimize_descriptions(
            &hive_core::testing::MeasureRequest {
                skill_dir: skill,
                workspace: root.join("workspace"),
                db_path: root.join("hive.db"),
                base_url: url,
                api_key: secret,
                kind,
                flavor: None,
                models: vec![hive_core::testing::ModelChoice {
                    id: model,
                    strong: true,
                }],
                adapter: "emulated".into(),
                repetitions: 1,
                threshold: 0.5,
                pass_bar: 0.5,
                distractors: Vec::new(),
                seed: 7,
                cli_program: None,
                cli_prefix: Vec::new(),
            },
            rounds.unwrap_or(5),
        )
        .map_err(AppError::io)?;
        Ok(saved.to_string())
    })
    .await
}

#[tauri::command]
pub async fn blind_compare_last(
    prompt: String,
    output_a: String,
    output_b: String,
) -> Result<String, AppError> {
    crate::offload::offload(move || {
        let endpoint = measured_endpoint()?;
        let provider = endpoint.provider;
        let model = endpoint.model;
        let secret = endpoint.secret;
        let compared = hive_core::testing::blind_compare(&hive_core::testing::BlindRequest {
            provider: provider.as_ref(),
            model: &model,
            prompt: &prompt,
            output_a: &output_a,
            output_b: &output_b,
            sealed_a: "with_skill",
            sealed_b: "without_skill",
            secret: &secret,
        })
        .map_err(AppError::io)?;
        let root = central_repo::base_dir().join("test-runs");
        std::fs::create_dir_all(&root)?;
        std::fs::write(
            root.join("blind.json"),
            serde_json::to_string(&compared).unwrap_or_default(),
        )?;
        Ok(compared.to_string())
    })
    .await
}

#[tauri::command]
pub async fn control_measured_run(action: String) -> Result<String, AppError> {
    crate::offload::offload(move || {
        let action = match action.as_str() {
            "continue" | "stop" | "cancel" => action,
            _ => {
                return Err(AppError::invalid_input(
                    "action must be continue, stop, or cancel",
                ))
            }
        };
        let root = central_repo::base_dir().join("test-runs");
        std::fs::create_dir_all(&root)?;
        let path = root.join("control.json");
        let body = serde_json::json!({"action": action}).to_string();
        std::fs::write(&path, &body)?;
        let workspace = root.join("workspace");
        if workspace.is_dir() {
            std::fs::write(workspace.join("control.json"), &body)?;
        }
        Ok(path.to_string_lossy().to_string())
    })
    .await
}

#[tauri::command]
pub async fn export_measured_run() -> Result<String, AppError> {
    crate::offload::offload(move || {
        let workspace = central_repo::base_dir().join("test-runs").join("workspace");
        let names = ["benchmark.json", "cells.csv", "junit.xml", "review.html"];
        let present: Vec<&str> = names
            .into_iter()
            .filter(|name| workspace.join(name).is_file())
            .collect();
        if present.is_empty() {
            return Err(AppError::invalid_input("export needs a measured run"));
        }
        Ok(serde_json::json!({"files": present}).to_string())
    })
    .await
}

#[tauri::command]
pub async fn save_suite_draft(
    skill_path: String,
    prompt: String,
    query: String,
    check: String,
) -> Result<String, AppError> {
    crate::offload::offload(move || {
        let skill = PathBuf::from(skill_path.trim());
        std::fs::create_dir_all(&skill)?;
        if !skill.join("SKILL.md").is_file() {
            let name = skill
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("skill");
            std::fs::write(
                skill.join("SKILL.md"),
                format!("---\nname: {name}\ndescription: {prompt}\n---\n"),
            )?;
        }
        let name = skill
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("skill");
        save_suite(
            &skill,
            &Suite {
                skill_name: name.into(),
                evals: vec![SuiteCase {
                    id: 1,
                    prompt: prompt.clone(),
                    expected_output: prompt.clone(),
                    files: Vec::new(),
                    expectations: vec![check.clone()],
                }],
                triggers: vec![TriggerRow {
                    query,
                    should_trigger: true,
                    rationale: None,
                }],
                checks: vec![CheckSpec {
                    id: "case".into(),
                    expectation: check.clone(),
                    kind: "text".into(),
                    path: "SKILL.md".into(),
                    pattern: None,
                }],
            },
        )
        .map_err(|err| AppError::io(err.to_string()))?;
        Ok(skill
            .join("evals")
            .join("evals.json")
            .to_string_lossy()
            .to_string())
    })
    .await
}

#[tauri::command]
pub async fn probe_endpoint(id: String) -> Result<String, AppError> {
    crate::offload::offload(move || {
        let rows = hive_core::providers::list_named_providers(&central_repo::db_path())
            .map_err(AppError::db)?;
        let Some(config) = rows.into_iter().find(|row| row.id == id.trim()) else {
            return Err(AppError::invalid_input("unknown provider"));
        };
        let secret = std::env::var("SKILL_HIVE_API_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .or_else(|| {
                hive_core::providers::load_provider_secret(
                    &config.id,
                    &central_repo::base_dir().join("provider-secrets"),
                )
                .ok()
            })
            .ok_or_else(|| AppError::invalid_input("probe needs a stored provider secret"))?;
        let report = hive_core::providers::probe_provider(
            &config,
            &hive_core::providers::SecretString::new(secret),
            &hive_core::providers::CancellationToken::new(),
        );
        serde_json::to_string(&report).map_err(AppError::io)
    })
    .await
}

struct MeasuredEndpoint {
    provider: Box<dyn hive_core::providers::ChatProvider>,
    model: String,
    secret: String,
    url: String,
    kind: String,
}

fn measured_endpoint() -> Result<MeasuredEndpoint, AppError> {
    let rows = hive_core::providers::list_named_providers(&central_repo::db_path())
        .map_err(AppError::db)?;
    let Some(row) = rows
        .into_iter()
        .find(|row| row.base_url.as_ref().is_some_and(|url| !url.is_empty()))
    else {
        return Err(AppError::invalid_input(
            "add a provider with a base URL before measuring",
        ));
    };
    let kind = match row.kind {
        hive_core::providers::ProviderKind::OpenaiCompat => "openai",
        hive_core::providers::ProviderKind::Anthropic => "anthropic",
        hive_core::providers::ProviderKind::LocalCli => {
            return Err(AppError::invalid_input(
                "measuring needs an Anthropic or OpenAI adapter",
            ));
        }
    };
    let api_key = std::env::var("SKILL_HIVE_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            hive_core::providers::load_provider_secret(
                &row.id,
                &central_repo::base_dir().join("provider-secrets"),
            )
            .ok()
        })
        .ok_or_else(|| {
            AppError::invalid_input(
                "measuring needs SKILL_HIVE_API_KEY or a stored provider secret",
            )
        })?;
    let secret = hive_core::providers::SecretString::new(api_key.clone());
    let model = row
        .default_model
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "claude-sonnet-5-5".to_string());
    let url = row.base_url.clone().unwrap_or_default();
    let provider: Box<dyn hive_core::providers::ChatProvider> = if kind == "openai" {
        Box::new(hive_core::providers::OpenAiProvider::new(
            "measured",
            url.clone(),
            hive_core::providers::Flavor::Openai,
            secret,
        ))
    } else {
        Box::new(hive_core::providers::AnthropicProvider::new(
            "measured",
            url.clone(),
            secret,
        ))
    };
    Ok(MeasuredEndpoint {
        provider,
        model,
        secret: api_key,
        url,
        kind: kind.to_string(),
    })
}

#[tauri::command]
pub async fn record_compare_verdict(id: String, verdict: String) -> Result<String, AppError> {
    crate::offload::offload(move || {
        store_verdict(&central_repo::db_path(), id.trim(), verdict.trim()).map_err(AppError::db)
    })
    .await
}

#[tauri::command]
pub async fn write_history_baseline(skill_name: String) -> Result<String, AppError> {
    crate::offload::offload(move || {
        let name = if skill_name.trim().is_empty() {
            "baseline"
        } else {
            skill_name.trim()
        };
        let last = central_repo::base_dir()
            .join("effectiveness")
            .join("last.json");
        let measured = std::fs::read_to_string(&last)
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .filter(|value| value.get("run_id").and_then(|item| item.as_str()).is_some());
        let Some(measured) = measured else {
            return Err(AppError::invalid_input(
                "baseline needs a measured run. A canned history chain is not written.",
            ));
        };
        let path = authoring_root("history", name).join("history.json");
        let entries = history_entries(&measured);
        let value = write_history(&path, name, &entries).map_err(AppError::internal)?;
        Ok(value.to_string())
    })
    .await
}

fn history_entries(measured: &serde_json::Value) -> Vec<HistoryEntry> {
    let rows = measured
        .get("history")
        .and_then(|value| value.get("iterations"))
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    if !rows.is_empty() {
        return rows
            .iter()
            .filter_map(|row| {
                Some(HistoryEntry {
                    version: row.get("version")?.as_str()?.to_string(),
                    parent: row
                        .get("parent")
                        .and_then(|value| value.as_str())
                        .map(str::to_string),
                    expectation_pass_rate: row
                        .get("expectation_pass_rate")
                        .and_then(|value| value.as_f64())
                        .unwrap_or(0.0),
                    grading_result: row
                        .get("grading_result")
                        .and_then(|value| value.as_str())
                        .unwrap_or("baseline")
                        .to_string(),
                    is_current_best: row
                        .get("is_current_best")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false),
                })
            })
            .collect();
    }
    let rate = measured
        .get("with_rate")
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0);
    vec![HistoryEntry {
        version: measured
            .get("run_id")
            .and_then(|value| value.as_str())
            .unwrap_or("measured")
            .to_string(),
        parent: None,
        expectation_pass_rate: rate,
        grading_result: "baseline".into(),
        is_current_best: true,
    }]
}

#[tauri::command]
pub async fn set_provider_secret(
    id: String,
    secret: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let account = if id.trim().is_empty() {
            "endpoint"
        } else {
            id.trim()
        };
        let kind = store_provider_secret(account, &secret, &secrets_dir())
            .map_err(|err| AppError::io(err.to_string()))?;
        let label = format!("{kind:?}").to_ascii_lowercase();
        store
            .set_setting("endpoint_secret_backend", &label)
            .map_err(db)?;
        Ok(label)
    })
    .await
}

#[tauri::command]
pub async fn clear_endpoint_secret(id: String) -> Result<bool, AppError> {
    crate::offload::offload(move || {
        let account = if id.trim().is_empty() {
            "endpoint"
        } else {
            id.trim()
        };
        clear_provider_secret(account, &secrets_dir())
            .map_err(|err| AppError::io(err.to_string()))?;
        Ok(true)
    })
    .await
}

#[tauri::command]
pub async fn store_backup_pat(
    token: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let kind = store_git_token(token.trim(), &secrets_dir())
            .map_err(|err| AppError::io(err.to_string()))?;
        let label = format!("{kind:?}").to_ascii_lowercase();
        store
            .set_setting("backup_pat_backend", &label)
            .map_err(db)?;
        Ok(label)
    })
    .await
}

#[tauri::command]
pub async fn revoke_backup_token(store: State<'_, Arc<SkillStore>>) -> Result<bool, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        clear_git_token(&secrets_dir()).map_err(|err| AppError::io(err.to_string()))?;
        let gone = !has_git_token(&secrets_dir());
        store
            .set_setting(
                "backup_pat_backend",
                if gone { "cleared" } else { "present" },
            )
            .map_err(db)?;
        Ok(gone)
    })
    .await
}

#[tauri::command]
pub async fn set_aside_backup(
    note: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        let dest = central_repo::base_dir()
            .join("aside")
            .join(format!("aside-{}", now()));
        copy_children(&root(), &dest.join("skills"))?;
        std::fs::write(dest.join("note.txt"), note.trim()).map_err(AppError::io)?;
        store
            .set_setting("backup_aside", &dest.to_string_lossy())
            .map_err(db)?;
        Ok(dest.to_string_lossy().to_string())
    })
    .await
}

#[tauri::command]
pub async fn pull_project_center(name: String, body: String) -> Result<String, AppError> {
    crate::offload::offload(move || {
        let dir = root().join("projects").join(sanitize_skill_name(&name));
        let document = format!("---\nname: {name}\n---\n{body}\n");
        write_skill_file(&dir, "SKILL.md", &document)?;
        Ok(dir.join("SKILL.md").to_string_lossy().to_string())
    })
    .await
}

#[tauri::command]
pub async fn export_diagnostics(path: String) -> Result<String, AppError> {
    crate::offload::offload(move || {
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
    })
    .await
}

#[tauri::command]
pub async fn git_backup_restore_remote(url: String) -> Result<String, AppError> {
    crate::offload::offload(move || {
        let url = url.trim();
        if url.is_empty() {
            return Err(AppError::invalid_input("Enter a remote to restore"));
        }
        let staging = std::env::temp_dir().join(format!("skill-hive-restore-{}", now()));
        let _ = std::fs::remove_dir_all(&staging);
        git_backup::clone_into(&staging, url).map_err(|err| AppError::git(err.to_string()))?;
        copy_children(&staging, &root())?;
        let _ = std::fs::remove_dir_all(&staging);
        Ok(root().to_string_lossy().to_string())
    })
    .await
}

#[tauri::command]
pub async fn skip_first_run_restore(store: State<'_, Arc<SkillStore>>) -> Result<bool, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        store
            .set_setting("first_run_restore", "skipped")
            .map_err(db)?;
        Ok(true)
    })
    .await
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
    scenario_service::ensure_scenario_exists(store, preset_id)?;
    let before = store
        .get_skill_ids_for_scenario(preset_id)
        .map_err(db)?
        .len();
    scenario_service::apply_scenario_to_default(store, preset_id)?;
    Ok(before)
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
    use super::{confirm_preview, preview_repository};
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
        hive_core::library::central_repo::set_runtime_skills_dir_override(Some(skills.clone()));
        let names = confirm_preview(&store, &preview.id, &relative, None).expect("confirm");
        hive_core::library::central_repo::set_runtime_skills_dir_override(None);
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
        hive_core::backup::git_backup::clone_repo(&bare, &staging).unwrap();
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

    #[test]
    fn check_updates_records_a_changed_source_in_the_database() {
        use hive_core::db::skill_store::{SkillRecord, SkillStore};

        let root = std::env::temp_dir().join(format!("skill-hive-update-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let central = root.join("central");
        let source = root.join("source");
        fs::create_dir_all(&central).unwrap();
        fs::create_dir_all(&source).unwrap();
        fs::write(central.join("SKILL.md"), "---\nname: notes\n---\nold\n").unwrap();
        fs::write(source.join("SKILL.md"), "---\nname: notes\n---\nnew\n").unwrap();
        let store = SkillStore::new(&root.join("hive.db")).unwrap();
        store.set_setting("update_check_ttl_minutes", "0").unwrap();
        store
            .insert_skill(&SkillRecord {
                id: "notes".into(),
                name: "notes".into(),
                description: None,
                source_type: "local".into(),
                source_ref: Some(source.to_string_lossy().to_string()),
                source_ref_resolved: None,
                source_subpath: None,
                source_branch: None,
                source_revision: None,
                remote_revision: None,
                central_path: central.to_string_lossy().to_string(),
                content_hash: None,
                enabled: true,
                created_at: 1,
                updated_at: 1,
                status: "managed".into(),
                update_status: "unknown".into(),
                last_checked_at: None,
                last_check_error: None,
            })
            .unwrap();
        assert_eq!(super::check_updates_in(&store).unwrap(), 1);
        let row = store.get_skill_by_id("notes").unwrap().unwrap();
        assert_eq!(row.update_status, "update_available");
        fs::copy(source.join("SKILL.md"), central.join("SKILL.md")).unwrap();
        assert_eq!(super::check_updates_in(&store).unwrap(), 0);
        assert_eq!(
            store
                .get_skill_by_id("notes")
                .unwrap()
                .unwrap()
                .update_status,
            "current"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn update_checks_honor_ttl_and_daily_auto_apply_writes_the_library() {
        use hive_core::db::skill_store::{SkillRecord, SkillStore};

        let root = std::env::temp_dir().join(format!(
            "skill-hive-ttl-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_millis())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let central = root.join("central");
        let source = root.join("source");
        fs::create_dir_all(&central).unwrap();
        fs::create_dir_all(&source).unwrap();
        fs::write(central.join("SKILL.md"), "---\nname: notes\n---\nold\n").unwrap();
        fs::write(source.join("SKILL.md"), "---\nname: notes\n---\nnew\n").unwrap();
        let store = SkillStore::new(&root.join("hive.db")).unwrap();
        store
            .insert_skill(&SkillRecord {
                id: "notes".into(),
                name: "notes".into(),
                description: None,
                source_type: "local".into(),
                source_ref: Some(source.to_string_lossy().to_string()),
                source_ref_resolved: None,
                source_subpath: None,
                source_branch: None,
                source_revision: None,
                remote_revision: None,
                central_path: central.to_string_lossy().to_string(),
                content_hash: None,
                enabled: true,
                created_at: 1,
                updated_at: 1,
                status: "managed".into(),
                update_status: "unknown".into(),
                last_checked_at: None,
                last_check_error: None,
            })
            .unwrap();
        store.set_setting("update_check_ttl_minutes", "60").unwrap();
        assert_eq!(super::check_updates_in(&store).unwrap(), 1);
        fs::write(source.join("SKILL.md"), "---\nname: notes\n---\nold\n").unwrap();
        assert_eq!(super::check_updates_in(&store).unwrap(), 1);
        assert_eq!(
            store
                .get_skill_by_id("notes")
                .unwrap()
                .unwrap()
                .update_status,
            "update_available"
        );
        store.set_skill_checked_at("notes", 1).unwrap();
        assert_eq!(super::check_updates_in(&store).unwrap(), 0);
        assert_eq!(
            store
                .get_skill_by_id("notes")
                .unwrap()
                .unwrap()
                .update_status,
            "current"
        );
        fs::write(source.join("notes.md"), "remote notes\n").unwrap();
        let listed = super::managed(&store).unwrap();
        let skill = listed.iter().find(|row| row.id == "notes").unwrap();
        assert!(skill.files.is_empty(), "the list payload stays metadata");
        assert!(skill.body.is_empty());
        let files = super::skill_tree_files(&central, Some(source.as_path()));
        assert!(files.iter().any(|file| file.path == "notes.md"));
        assert_eq!(
            files
                .iter()
                .find(|file| file.path == "notes.md")
                .unwrap()
                .remote,
            "remote notes\n"
        );

        fs::write(source.join("SKILL.md"), "---\nname: notes\n---\napplied\n").unwrap();
        store.set_setting("auto_update_interval", "daily").unwrap();
        store.set_setting("auto_update_apply", "1").unwrap();
        store.set_setting("update_check_ttl_minutes", "0").unwrap();
        store.set_skill_checked_at("notes", 1).unwrap();
        assert_eq!(super::maybe_run_auto_update(&store, 10_000).unwrap(), 1);
        assert!(fs::read_to_string(central.join("SKILL.md"))
            .unwrap()
            .contains("applied"));
        fs::write(source.join("SKILL.md"), "---\nname: notes\n---\nlater\n").unwrap();
        assert_eq!(super::maybe_run_auto_update(&store, 20_000).unwrap(), 0);
        assert!(!fs::read_to_string(central.join("SKILL.md"))
            .unwrap()
            .contains("later"));
        store.set_skill_checked_at("notes", 1).unwrap();
        assert_eq!(
            super::maybe_run_auto_update(&store, 20_000 + 86_400_000).unwrap(),
            1
        );
        assert!(fs::read_to_string(central.join("SKILL.md"))
            .unwrap()
            .contains("later"));
        fs::write(central.join("keep.txt"), "mine\n").unwrap();
        fs::write(
            source.join("SKILL.md"),
            "---\nname: notes\n---\nremoved-file\n",
        )
        .unwrap();
        store.set_skill_checked_at("notes", 1).unwrap();
        assert_eq!(
            super::maybe_run_auto_update(&store, 20_000 + 86_400_000 * 2).unwrap(),
            1
        );
        assert_eq!(
            fs::read_to_string(central.join("keep.txt")).unwrap(),
            "mine\n"
        );
        assert!(!fs::read_to_string(central.join("SKILL.md"))
            .unwrap()
            .contains("removed-file"));
        assert_eq!(
            store
                .get_skill_by_id("notes")
                .unwrap()
                .unwrap()
                .update_status,
            "pending_removals"
        );

        let (repo, subpath) =
            super::normalize_install_source("https://skills.sh/acme/widgets/helper");
        assert_eq!(repo, "https://github.com/acme/widgets");
        assert_eq!(subpath.as_deref(), Some("helper"));
        super::register_install("market:acme/widgets");
        assert!(super::cancel_install_now(Some("market:acme/widgets".into())).unwrap());
        assert!(super::install_is_cancelled("market:acme/widgets"));

        store
            .insert_scenario(&hive_core::db::skill_store::ScenarioRecord {
                id: "preset-keep".into(),
                name: "Keep".into(),
                description: Some("kept".into()),
                icon: Some("book".into()),
                sort_order: 0,
                created_at: 1,
                updated_at: 1,
            })
            .unwrap();
        super::update_preset_record(&store, "preset-keep", "Renamed", None, None).unwrap();
        let preset = store
            .get_all_scenarios()
            .unwrap()
            .into_iter()
            .find(|row| row.id == "preset-keep")
            .unwrap();
        assert_eq!(preset.name, "Renamed");
        assert_eq!(preset.description.as_deref(), Some("kept"));
        assert_eq!(preset.icon.as_deref(), Some("book"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn profile_library_load() {
        use hive_core::db::skill_store::{ProjectRecord, ScenarioRecord, SkillRecord};
        use std::time::Instant;

        let root = std::env::temp_dir().join(format!("skill-hive-profile-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let store = hive_core::db::skill_store::SkillStore::new(&root.join("hive.db")).unwrap();
        let started = Instant::now();
        for i in 0..200 {
            let dir = root.join(format!("skills/skill-{i:03}"));
            fs::create_dir_all(dir.join("refs")).unwrap();
            fs::write(
                dir.join("SKILL.md"),
                format!("---\nname: skill-{i:03}\ndescription: A skill used in a crowded library.\n---\n# skill {i}\n{}", "line\n".repeat(40)),
            )
            .unwrap();
            fs::write(dir.join("notes.md"), "n".repeat(4000)).unwrap();
            fs::write(dir.join("refs/a.md"), "a".repeat(2000)).unwrap();
            store
                .insert_skill(&SkillRecord {
                    id: format!("s{i}"),
                    name: format!("skill-{i:03}"),
                    description: Some(format!("description {i}")),
                    source_type: "local".into(),
                    source_ref: None,
                    source_ref_resolved: None,
                    source_subpath: None,
                    source_branch: None,
                    source_revision: None,
                    remote_revision: None,
                    central_path: dir.to_string_lossy().into(),
                    content_hash: None,
                    enabled: i % 5 != 0,
                    created_at: 1,
                    updated_at: 1,
                    status: "active".into(),
                    update_status: "current".into(),
                    last_checked_at: None,
                    last_check_error: None,
                })
                .unwrap();
        }
        let seed_ms = started.elapsed().as_secs_f64() * 1000.0;
        for i in 0..4 {
            store
                .insert_project(&ProjectRecord {
                    id: format!("p{i}"),
                    name: format!("project-{i}"),
                    path: root.join(format!("projects/{i}")).to_string_lossy().into(),
                    workspace_type: "folder".into(),
                    linked_agent_key: None,
                    linked_agent_name: None,
                    disabled_path: None,
                    sort_order: i,
                    created_at: 1,
                    updated_at: 1,
                })
                .unwrap();
            store
                .insert_scenario(&ScenarioRecord {
                    id: format!("preset-{i}"),
                    name: format!("Preset {i}"),
                    description: None,
                    icon: None,
                    sort_order: i,
                    created_at: 1,
                    updated_at: 1,
                })
                .unwrap();
        }
        let list_started = Instant::now();
        let skills = super::managed(&store).unwrap();
        let list_ms = list_started.elapsed().as_secs_f64() * 1000.0;
        let project_started = Instant::now();
        let projects = store.get_all_projects().unwrap();
        let projects_ms = project_started.elapsed().as_secs_f64() * 1000.0;
        let preset_started = Instant::now();
        let presets = store.get_all_scenarios().unwrap();
        let presets_ms = preset_started.elapsed().as_secs_f64() * 1000.0;
        let bytes: usize = skills
            .iter()
            .map(|skill| {
                skill.body.len()
                    + skill.source_body.len()
                    + skill
                        .files
                        .iter()
                        .map(|file| file.local.len() + file.remote.len())
                        .sum::<usize>()
            })
            .sum();
        println!(
            "PROFILE seed_ms={seed_ms:.1} list_ms={list_ms:.1} skills={} payload_bytes={bytes} projects_ms={projects_ms:.1} projects={} presets_ms={presets_ms:.1} presets={}",
            skills.len(),
            projects.len(),
            presets.len()
        );
        assert_eq!(skills.len(), 200);
        assert_eq!(projects.len(), 4);
        let _ = fs::remove_dir_all(&root);
    }
}
