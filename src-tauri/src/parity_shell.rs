//! Diagnostics, library path, and window commands.

use std::path::PathBuf;

use hive_core::library::central_repo;
use hive_core::util::error::AppError;
use hive_core::util::panic_log;
use serde::Serialize;
use tauri::{AppHandle, Manager};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticInfo {
    pub version: String,
    pub os: String,
    pub arch: String,
    pub library_path: String,
    pub skills_path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanicInfo {
    pub path: String,
    pub excerpt: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogExcerpt {
    pub path: String,
    pub text: String,
}

fn open_path(path: &std::path::Path) -> Result<(), AppError> {
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
pub fn get_central_repo_path_override() -> Option<String> {
    central_repo::configured_base_dir().map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_central_repo_pending_path() -> Option<String> {
    central_repo::pending_base_dir().map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_central_repo_warnings() -> Vec<String> {
    central_repo::startup_warnings()
}

#[tauri::command]
pub fn open_central_repo_folder() -> Result<(), AppError> {
    let path = central_repo::base_dir();
    std::fs::create_dir_all(&path).map_err(AppError::io)?;
    open_path(&path)
}

#[tauri::command]
pub fn get_diagnostic_info(app: AppHandle) -> Result<DiagnosticInfo, AppError> {
    Ok(DiagnosticInfo {
        version: app.package_info().version.to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        library_path: central_repo::base_dir().to_string_lossy().to_string(),
        skills_path: central_repo::skills_dir().to_string_lossy().to_string(),
    })
}

fn panic_file() -> PathBuf {
    panic_log::cached_last_panic_path()
        .unwrap_or_else(|| central_repo::logs_dir().join("last_panic.log"))
}

#[tauri::command]
pub fn check_last_panic() -> Result<Option<PanicInfo>, AppError> {
    let path = panic_file();
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(AppError::io)?;
    let excerpt: String = text.chars().take(4000).collect();
    Ok(Some(PanicInfo {
        path: path.to_string_lossy().to_string(),
        excerpt,
    }))
}

#[tauri::command]
pub fn clear_last_panic() -> Result<(), AppError> {
    let path = panic_file();
    if path.is_file() {
        std::fs::remove_file(&path).map_err(AppError::io)?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_recent_log_excerpt() -> Result<LogExcerpt, AppError> {
    let dir = central_repo::logs_dir();
    let mut newest: Option<PathBuf> = None;
    if dir.is_dir() {
        for entry in std::fs::read_dir(&dir).map_err(AppError::io)? {
            let entry = entry.map_err(AppError::io)?;
            if entry.path().is_file() {
                newest = Some(entry.path());
            }
        }
    }
    let Some(path) = newest else {
        return Ok(LogExcerpt {
            path: dir.to_string_lossy().to_string(),
            text: String::new(),
        });
    };
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let start = text.len().saturating_sub(8000);
    Ok(LogExcerpt {
        path: path.to_string_lossy().to_string(),
        text: text[start..].to_string(),
    })
}

#[tauri::command]
pub fn export_logs_zip(path: String) -> Result<String, AppError> {
    let dest = PathBuf::from(path.trim());
    if dest.as_os_str().is_empty() {
        return Err(AppError::invalid_input("Choose a path for the log archive"));
    }
    let excerpt = get_recent_log_excerpt()?;
    hive_core::util::diagnostics::export_logs_zip(&excerpt.text, &dest)
        .map_err(|err| AppError::io(err.to_string()))?;
    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub fn app_exit(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn restart_app(app: AppHandle) {
    app.restart();
}

#[tauri::command]
pub fn hide_to_tray(app: AppHandle) -> Result<(), AppError> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|err| AppError::io(err.to_string()))?;
    }
    Ok(())
}
