//! Backup commands that call the full git backup engine.

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use hive_core::backup::github::{
    begin_device_flow, find_or_create_private_repo, poll_device_flow, validate_pat, DevicePoll,
    GitHubHttp,
};
use hive_core::core::git_backup::{self, BackupSizeReport};
use hive_core::core::git_credentials::{self, RemoteCredential};
use hive_core::core::merge::resolve::{self, ResolveAction};
use hive_core::core::repo_lock::RepoLock;
use hive_core::db::skill_store::{PendingConflictRow, SkillStore};
use hive_core::library::central_repo;
use hive_core::util::error::AppError;
use serde::Serialize;
use tauri::State;

const GITHUB_CLIENT_ID: &str = "Ov23li4a3SMdhIiKo7IE";

fn db(err: impl std::fmt::Display) -> AppError {
    AppError::db(err.to_string())
}

fn git(err: impl std::fmt::Display) -> AppError {
    AppError::git(err.to_string())
}

fn skills() -> PathBuf {
    central_repo::skills_dir()
}

fn device_name(store: &SkillStore) -> String {
    store
        .get_setting("backup_device_name")
        .ok()
        .flatten()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(git_backup::default_device_name)
}

fn ensure_repo(store: &SkillStore) -> Result<PathBuf, AppError> {
    let path = skills();
    if !path.join(".git").exists() {
        std::fs::create_dir_all(&path)?;
        git_backup::init_repo(&path, &device_name(store)).map_err(git)?;
    }
    Ok(path)
}

struct CurlHttp;

impl GitHubHttp for CurlHttp {
    fn post_form(&self, url: &str, body: &str) -> anyhow::Result<(u16, String)> {
        curl("POST", url, Some(body), None)
    }

    fn get(&self, url: &str, bearer: Option<&str>) -> anyhow::Result<(u16, String)> {
        curl("GET", url, None, bearer)
    }
}

fn curl(
    method: &str,
    url: &str,
    body: Option<&str>,
    bearer: Option<&str>,
) -> anyhow::Result<(u16, String)> {
    let mut command = Command::new("curl");
    command.args([
        "-sS",
        "-X",
        method,
        "-w",
        "\n%{http_code}",
        "-H",
        "Accept: application/json",
        "-H",
        "User-Agent: skill-hive",
    ]);
    let header = bearer.map(|token| format!("Authorization: Bearer {token}"));
    if let Some(header) = header.as_deref() {
        command.args(["-H", header]);
    }
    if let Some(body) = body {
        command.args(["--data", body]);
    }
    let output = command.arg(url).output()?;
    if !output.status.success() {
        anyhow::bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
    }
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    let (payload, code) = text.rsplit_once('\n').unwrap_or((text.as_str(), "0"));
    Ok((code.trim().parse().unwrap_or(0), payload.to_string()))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubBackupConnectResult {
    pub url: String,
    pub login: String,
    pub remote_has_content: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceFlowStart {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceFlowPoll {
    pub status: String,
    pub result: Option<GithubBackupConnectResult>,
}

fn connect_token(
    store: &SkillStore,
    token: &str,
    repo_name: &str,
    method: &str,
) -> Result<GithubBackupConnectResult, AppError> {
    if token.is_empty() {
        return Err(AppError::invalid_input("Token is empty"));
    }
    let http = CurlHttp;
    let login = validate_pat(&http, token).map_err(|err| AppError::network(err.to_string()))?;
    let url = find_or_create_private_repo(&http, token, repo_name)
        .map_err(|err| AppError::network(err.to_string()))?;
    git_credentials::store_credential(
        "github.com",
        &RemoteCredential {
            username: login.clone(),
            password: token.to_string(),
        },
    )
    .map_err(|err| AppError::internal(format!("KEYCHAIN_UNAVAILABLE: {err}")))?;
    store
        .set_setting("git_backup_remote_url", &url)
        .map_err(db)?;
    store
        .set_setting("github_auth_method", method)
        .map_err(db)?;
    let path = ensure_repo(store)?;
    git_backup::set_remote(&path, &url).map_err(git)?;
    let remote_has_content = git_backup::remote_has_heads(&url).unwrap_or(false);
    Ok(GithubBackupConnectResult {
        url,
        login,
        remote_has_content,
    })
}

#[tauri::command]
pub fn git_backup_fetch(store: State<'_, Arc<SkillStore>>) -> Result<(), AppError> {
    let path = ensure_repo(store.inner())?;
    git_backup::fetch_remote(&path).map_err(git)
}

#[tauri::command]
pub fn git_backup_init(store: State<'_, Arc<SkillStore>>) -> Result<(), AppError> {
    ensure_repo(store.inner()).map(|_| ())
}

#[tauri::command]
pub fn github_backup_connect(
    token: String,
    repo_name: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<GithubBackupConnectResult, AppError> {
    connect_token(store.inner(), token.trim(), repo_name.trim(), "pat")
}

#[tauri::command]
pub fn github_device_flow_start() -> Result<DeviceFlowStart, AppError> {
    let codes = begin_device_flow(&CurlHttp, GITHUB_CLIENT_ID)
        .map_err(|err| AppError::network(err.to_string()))?;
    Ok(DeviceFlowStart {
        device_code: codes.device_code,
        user_code: codes.user_code,
        verification_uri: codes.verification_uri,
        interval: codes.interval,
    })
}

#[tauri::command]
pub fn github_device_flow_poll(
    device_code: String,
    repo_name: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<DeviceFlowPoll, AppError> {
    match poll_device_flow(&CurlHttp, GITHUB_CLIENT_ID, device_code.trim())
        .map_err(|err| AppError::network(err.to_string()))?
    {
        DevicePoll::Pending => Ok(DeviceFlowPoll {
            status: "pending".into(),
            result: None,
        }),
        DevicePoll::Denied => Ok(DeviceFlowPoll {
            status: "denied".into(),
            result: None,
        }),
        DevicePoll::Approved { access_token } => Ok(DeviceFlowPoll {
            status: "connected".into(),
            result: Some(connect_token(
                store.inner(),
                &access_token,
                repo_name.trim(),
                "oauth",
            )?),
        }),
    }
}

#[tauri::command]
pub fn git_backup_sanitize_remote_url(url: String) -> Result<String, AppError> {
    Ok(git_credentials::split_credentials_from_url(url.trim())
        .map(|(_, clean)| clean)
        .unwrap_or_else(|| url.trim().to_string()))
}

#[tauri::command]
pub fn git_backup_commit(
    message: Option<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let path = ensure_repo(store.inner())?;
    let note = message.unwrap_or_else(|| "Backup".into());
    git_backup::commit_all(&path, &note).map_err(git)
}

#[tauri::command]
pub fn git_backup_push(store: State<'_, Arc<SkillStore>>) -> Result<(), AppError> {
    let path = ensure_repo(store.inner())?;
    git_backup::push(&path).map_err(git)
}

#[tauri::command]
pub fn git_backup_pull(store: State<'_, Arc<SkillStore>>) -> Result<(), AppError> {
    let path = ensure_repo(store.inner())?;
    git_backup::pull(&path).map_err(git)
}

#[tauri::command]
pub fn git_backup_pending_conflicts(
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<PendingConflictRow>, AppError> {
    store.inner().list_pending_conflicts().map_err(db)
}

#[tauri::command]
pub fn git_backup_resolve_conflict(
    skill_id: String,
    action: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let chosen = ResolveAction::parse(action.trim()).ok_or_else(|| {
        AppError::invalid_input("action must be keep_local, use_remote, or keep_both")
    })?;
    let path = skills();
    let _lock = RepoLock::acquire_foreground("resolve backup conflict").map_err(git)?;
    resolve::resolve_conflict_unlocked(store.inner(), &path, skill_id.trim(), chosen).map_err(git)
}

#[tauri::command]
pub fn git_backup_clone(
    url: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<String, AppError> {
    let path = skills();
    std::fs::create_dir_all(&path)?;
    git_backup::clone_into_strict(&path, url.trim()).map_err(git)?;
    store
        .inner()
        .set_setting("git_backup_remote_url", url.trim())
        .map_err(db)?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn git_backup_reclone(store: State<'_, Arc<SkillStore>>) -> Result<String, AppError> {
    let path = skills();
    let status = git_backup::get_status(&path).map_err(git)?;
    let url = status
        .remote_url
        .or_else(|| {
            store
                .inner()
                .get_setting("git_backup_remote_url")
                .ok()
                .flatten()
        })
        .filter(|url| !url.is_empty())
        .ok_or_else(|| AppError::invalid_input("No backup remote is configured"))?;
    let aside = path.with_extension(format!("aside-{}", std::process::id()));
    if path.exists() {
        let _ = std::fs::remove_dir_all(&aside);
        std::fs::rename(&path, &aside).map_err(AppError::io)?;
    }
    match git_backup::clone_into(&path, &url) {
        Ok(()) => {
            let _ = std::fs::remove_dir_all(&aside);
            Ok(path.to_string_lossy().to_string())
        }
        Err(err) => {
            let _ = std::fs::remove_dir_all(&path);
            if aside.exists() {
                let _ = std::fs::rename(&aside, &path);
            }
            Err(git(err))
        }
    }
}

#[tauri::command]
pub fn git_backup_create_snapshot(store: State<'_, Arc<SkillStore>>) -> Result<String, AppError> {
    let path = ensure_repo(store.inner())?;
    git_backup::create_snapshot_tag(&path).map_err(git)
}

#[tauri::command]
pub fn backup_device_name(store: State<'_, Arc<SkillStore>>) -> Result<String, AppError> {
    Ok(device_name(store.inner()))
}

#[tauri::command]
pub fn git_backup_size_report() -> Result<BackupSizeReport, AppError> {
    git_backup::size_report(&skills()).map_err(git)
}

#[tauri::command]
pub fn git_backup_migrate_credentials(
    store: State<'_, Arc<SkillStore>>,
) -> Result<Option<String>, AppError> {
    let store = store.inner();
    let url = store
        .get_setting("git_backup_remote_url")
        .map_err(db)?
        .unwrap_or_default();
    let Some((cred, clean)) = git_credentials::split_credentials_from_url(&url) else {
        return Ok(None);
    };
    let host = git_credentials::https_host(&clean).unwrap_or_else(|| "github.com".into());
    git_credentials::store_credential(&host, &cred)
        .map_err(|err| AppError::internal(err.to_string()))?;
    store
        .set_setting("git_backup_remote_url", &clean)
        .map_err(db)?;
    if skills().join(".git").exists() {
        git_backup::set_remote(&skills(), &clean).map_err(git)?;
    }
    Ok(Some(clean))
}
