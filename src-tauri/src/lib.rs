//! Skill Hive desktop shell.

mod commands;
mod error;

use std::sync::Arc;

use hive_core::Hive;
use tauri::{Emitter, Manager};

/// Event name every [`hive_core::model::HiveEvent`] is forwarded under.
pub const EVENT_NAME: &str = "hive://event";

fn init_logging(hive: &Hive) {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,hive_core=debug"));
    let log_dir = hive.paths.logs_dir();
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_dir.join("skill-hive.log"))
        .ok();
    match file {
        Some(file) => {
            let _ = tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_ansi(false)
                .with_writer(std::sync::Mutex::new(file))
                .try_init();
        }
        None => {
            let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
        }
    }
}

pub fn run() {
    let hive = Hive::open_default().expect("failed to open the Skill Hive home directory");
    init_logging(&hive);
    tracing::info!(home = %hive.paths.home.display(), version = hive_core::version(), "starting Skill Hive");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(hive.clone())
        .setup(move |app| {
            // Forward core events to the webview.
            let handle = app.handle().clone();
            let mut rx = hive.subscribe();
            tauri::async_runtime::spawn(async move {
                loop {
                    match rx.recv().await {
                        Ok(event) => {
                            let _ = handle.emit(EVENT_NAME, &event);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(_) => break,
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::dashboard,
            commands::settings_get,
            commands::settings_update,
            commands::doctor,
            commands::library_list,
            commands::library_stats,
            commands::library_get,
            commands::library_read_file,
            commands::library_import,
            commands::library_remove,
            commands::library_set_tags,
            commands::library_set_favorite,
            commands::library_set_notes,
            commands::library_create_skill,
            commands::library_write_document,
            commands::library_refresh,
            commands::library_analyze,
            commands::scan_local,
            commands::scan_paths,
            commands::source_parse,
            commands::source_preview,
            commands::source_import,
            commands::source_check_updates,
            commands::source_update_item,
            commands::catalog_list,
            commands::github_search,
            commands::source_clear_cache,
            commands::targets_list,
            commands::targets_installed,
            commands::target_status,
            commands::target_add_custom,
            commands::target_remove_custom,
            commands::target_set_skills_dir,
            commands::projects_list,
            commands::project_add,
            commands::project_remove,
            commands::deploy,
            commands::undeploy,
            commands::undeploy_item,
            commands::resync,
            commands::deployments_list,
            commands::remove_installed_path,
            commands::loadouts_list,
            commands::loadout_get,
            commands::loadout_create,
            commands::loadout_update,
            commands::loadout_delete,
            commands::loadout_plan,
            commands::loadout_apply,
            commands::recommend,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Skill Hive");
}

#[allow(dead_code)]
fn _assert_send() {
    fn is_send<T: Send + Sync>() {}
    is_send::<Arc<Hive>>();
}
