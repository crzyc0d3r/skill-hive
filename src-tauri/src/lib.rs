use std::path::PathBuf;
use std::sync::Arc;

use hive_core::db::skill_store::SkillStore;
use hive_core::library::central_repo;
use hive_core::util::error::AppError;
use hive_core::util::file_watcher::start_file_watcher;
use hive_core::util::notifier::Notifier;
use tauri::{Emitter, Manager};

mod tray;

struct TauriNotifier(tauri::AppHandle);

impl Notifier for TauriNotifier {
    fn emit(&self, event: &str, payload: &str) {
        if let Err(err) = self.0.emit(event, payload) {
            log::debug!("emit {event} failed: {err}");
        }
    }
}

#[tauri::command]
fn get_settings(
    key: String,
    store: tauri::State<'_, Arc<SkillStore>>,
) -> Result<Option<String>, AppError> {
    store.get_setting(&key).map_err(AppError::db)
}

#[tauri::command]
fn set_settings(
    key: String,
    value: String,
    store: tauri::State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    store.set_setting(&key, &value).map_err(AppError::db)
}

#[tauri::command]
fn log_startup_event(label: String, elapsed_ms: u64) {
    let sanitized: String = label.chars().filter(|c| !c.is_control()).take(64).collect();
    let display = if sanitized.is_empty() {
        "(empty)".to_string()
    } else {
        sanitized
    };
    log::info!("frontend startup: {display} {elapsed_ms} ms");
}

#[tauri::command]
fn list_base_entries() -> Result<Vec<String>, AppError> {
    let dir = central_repo::skills_dir();
    let mut names = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(AppError::db)?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        names.push(name);
    }
    names.sort();
    Ok(names)
}

fn box_err(err: impl std::fmt::Display) -> Box<dyn std::error::Error> {
    err.to_string().into()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    .build(),
            )?;
            if let Ok(base) = std::env::var("SKILL_HIVE_BASE") {
                if !base.is_empty() {
                    central_repo::set_runtime_base_dir_override(Some(PathBuf::from(base)));
                }
            }
            central_repo::ensure_central_repo(true).map_err(box_err)?;
            let db = central_repo::db_path();
            if let Some(parent) = db.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let store = Arc::new(SkillStore::new(&db).map_err(box_err)?);
            app.manage(store.clone());
            start_file_watcher(Arc::new(TauriNotifier(app.handle().clone())), store);
            if let Ok(dir) = app.path().app_log_dir() {
                hive_core::util::panic_log::install_panic_hook(dir);
            }
            if let Err(err) = tray::install(app.handle()) {
                log::warn!("tray icon unavailable: {err}");
            }
            Ok(())
        })
        .on_window_event(tray::on_window)
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_settings,
            log_startup_event,
            list_base_entries
        ])
        .run(tauri::generate_context!())
        .expect("error while running Skill Hive");
}
