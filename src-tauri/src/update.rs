//! In-app updates through the Tauri updater.
//!
//! Every GitHub release carries a `latest.json` that lists one signed bundle per platform. The
//! check fetches it, compares the announced version with this build and keeps the pending
//! update; the install downloads the bundle, verifies its signature against the public key in
//! `tauri.conf.json`, replaces the installed app and restarts it. The interface drives both
//! through commands and follows the download on the `hive://update` event.

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::error::{CmdResult, CommandError};

/// Event the install progress is emitted under.
pub const PROGRESS_EVENT: &str = "hive://update";

/// Emit a download event at most every this many bytes.
const PROGRESS_STEP: u64 = 256 * 1024;

/// The update the last check found, kept so the install needs no second lookup.
#[derive(Default)]
pub struct PendingUpdate(Mutex<Option<Update>>);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    pub current_version: String,
    pub available: bool,
    pub version: Option<String>,
    /// Release notes, Markdown.
    pub notes: Option<String>,
    /// Release date, `YYYY-MM-DD`.
    pub date: Option<String>,
    /// The platform key looked up in `latest.json`, for example `linux-x86_64`.
    pub target: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "phase")]
pub enum UpdateProgress {
    Downloading { downloaded: u64, total: Option<u64> },
    Installing,
    Installed,
}

fn updater_error(err: tauri_plugin_updater::Error) -> CommandError {
    use tauri_plugin_updater::Error;
    let message = match &err {
        Error::Network(_) | Error::Reqwest(_) => {
            format!("could not reach the release server ({err})")
        }
        Error::Minisign(_) | Error::SignatureUtf8(_) | Error::Base64(_) => {
            format!("the downloaded update is not signed with the Skill Hive key ({err})")
        }
        _ => err.to_string(),
    };
    CommandError {
        code: "update".into(),
        message,
    }
}

/// Compare this build with the latest release. Not an error when the release has no bundle
/// for this platform: that is an architecture the release workflow does not build.
#[tauri::command]
pub async fn update_check(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
) -> CmdResult<UpdateCheck> {
    let current_version = app.package_info().version.to_string();
    let target = tauri_plugin_updater::target().unwrap_or_else(|| "unknown".to_string());
    let updater = app
        .updater_builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(updater_error)?;
    let none = UpdateCheck {
        current_version: current_version.clone(),
        available: false,
        version: None,
        notes: None,
        date: None,
        target: target.clone(),
    };
    match updater.check().await {
        Ok(Some(update)) => {
            tracing::info!(
                current = %current_version,
                available = %update.version,
                target = %update.target,
                "an update is available"
            );
            let check = UpdateCheck {
                current_version,
                available: true,
                version: Some(update.version.clone()),
                notes: update.body.clone(),
                date: update
                    .date
                    .map(|d| format!("{:04}-{:02}-{:02}", d.year(), d.month() as u8, d.day())),
                target: update.target.clone(),
            };
            *pending.0.lock().unwrap_or_else(|p| p.into_inner()) = Some(update);
            Ok(check)
        }
        Ok(None) => {
            tracing::info!(current = %current_version, "up to date");
            *pending.0.lock().unwrap_or_else(|p| p.into_inner()) = None;
            Ok(none)
        }
        Err(tauri_plugin_updater::Error::TargetNotFound(t)) => {
            tracing::info!(target = %t, "the latest release has no bundle for this platform");
            Ok(none)
        }
        Err(tauri_plugin_updater::Error::TargetsNotFound(t)) => {
            tracing::info!(targets = ?t, "the latest release has no bundle for this platform");
            Ok(none)
        }
        Err(err) => {
            tracing::warn!(error = %err, "update check failed");
            Err(updater_error(err))
        }
    }
}

/// Download, verify and install the update the last check found, then restart into it.
#[tauri::command]
pub async fn update_install(app: AppHandle, pending: State<'_, PendingUpdate>) -> CmdResult<()> {
    let update = pending
        .0
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
        .ok_or_else(|| {
            CommandError::from("no update is pending; check for updates first".to_string())
        })?;
    tracing::info!(version = %update.version, url = %update.download_url, "installing update");
    let progress = app.clone();
    let mut downloaded = 0u64;
    let mut reported = 0u64;
    let finished = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                if downloaded - reported >= PROGRESS_STEP || Some(downloaded) == total {
                    reported = downloaded;
                    let _ = progress.emit(
                        PROGRESS_EVENT,
                        UpdateProgress::Downloading { downloaded, total },
                    );
                }
            },
            move || {
                let _ = finished.emit(PROGRESS_EVENT, UpdateProgress::Installing);
            },
        )
        .await
        .map_err(updater_error)?;
    let _ = app.emit(PROGRESS_EVENT, UpdateProgress::Installed);
    tracing::info!("update installed; restarting");
    // On Windows the installer has already closed this process by now. Elsewhere the new
    // bundle is in place and only needs a relaunch.
    app.restart()
}
