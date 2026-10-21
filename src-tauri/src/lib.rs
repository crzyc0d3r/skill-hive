use std::path::PathBuf;
use std::sync::Arc;

use hive_core::db::skill_store::SkillStore;
use hive_core::library::central_repo;
use hive_core::util::error::AppError;
use hive_core::util::file_watcher::start_file_watcher;
use hive_core::util::notifier::Notifier;
use tauri::{Emitter, Manager, WebviewWindowBuilder};

mod commands;
mod offload;
mod parity_backup;
mod parity_presets;
mod parity_projects;
mod parity_shell;
mod parity_skills;
mod parity_workspace;
mod progress;
mod tray;
mod webview_watch;

struct TauriNotifier(tauri::AppHandle);

impl Notifier for TauriNotifier {
    fn emit(&self, event: &str, payload: &str) {
        if let Err(err) = self.0.emit(event, payload) {
            log::debug!("emit {event} failed: {err}");
        }
    }
}

#[tauri::command]
async fn get_settings(
    key: String,
    store: tauri::State<'_, Arc<SkillStore>>,
) -> Result<Option<String>, AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || store.get_setting(&key).map_err(AppError::db)).await
}

#[tauri::command]
async fn set_settings(
    key: String,
    value: String,
    store: tauri::State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    crate::offload::offload(move || {
        store.set_setting(&key, &value).map_err(AppError::db)?;
        mirror_setting(store.as_ref(), &key, &value)
    })
    .await
}

fn mirror_setting(store: &SkillStore, key: &str, value: &str) -> Result<(), AppError> {
    match key {
        "close_behavior" | "closeBehaviour" | "close_action" => {
            for alias in ["close_behavior", "closeBehaviour", "close_action"] {
                if alias != key {
                    store.set_setting(alias, value).map_err(AppError::db)?;
                }
            }
        }
        "tray" | "trayIcon" | "show_tray_icon" => {
            let shown = !matches!(value, "0" | "off" | "false");
            let pairs = [
                ("tray", if shown { "1" } else { "0" }),
                ("trayIcon", if shown { "on" } else { "off" }),
                ("show_tray_icon", if shown { "1" } else { "0" }),
            ];
            for (alias, stored) in pairs {
                if alias != key {
                    store.set_setting(alias, stored).map_err(AppError::db)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
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
fn log_frontend_error(message: String, stack: String, route: String) {
    let clean = |value: String, limit: usize| {
        value
            .chars()
            .filter(|c| !c.is_control() || *c == '\n')
            .take(limit)
            .collect::<String>()
    };
    log::error!(
        "frontend error route={} message={} stack={}",
        clean(route, 160),
        clean(message, 500),
        clean(stack, 2000)
    );
}

#[tauri::command]
async fn check_app_update() -> String {
    crate::offload::offload(move || {
        let local = env!("CARGO_PKG_VERSION");
        let from_env = std::env::var("SKILL_HIVE_RELEASE_TAG").ok();
        let from_file =
            std::fs::read_to_string(central_repo::base_dir().join("app-release.txt")).ok();
        let remote = from_env.or(from_file).or_else(|| {
            if std::env::var_os("SKILL_HIVE_SKIP_RELEASE_CHECK").is_some() {
                return None;
            }
            let url = std::env::var("SKILL_HIVE_RELEASES_URL").unwrap_or_else(|_| {
                "https://api.github.com/repos/crzyc0d3r/skill-hive/releases/latest".into()
            });
            hive_core::updater::fetch_latest_release_tag(&url).ok()
        });
        hive_core::updater::classify_release(local, remote.as_deref())
    })
    .await
}

#[tauri::command]
async fn update_install_blocker(active_runs: u32) -> Option<String> {
    crate::offload::offload(move || {
        hive_core::updater::update_install_blocker(active_runs).map(str::to_string)
    })
    .await
}

#[tauri::command]
async fn updater_proxy(url: String) -> Option<String> {
    crate::offload::offload(move || hive_core::updater::updater_proxy(&url)).await
}

#[tauri::command]
async fn download_and_install_update(app: tauri::AppHandle) -> Result<String, String> {
    use tauri_plugin_updater::UpdaterExt;
    let outcome = match app.updater() {
        Ok(updater) => match updater.check().await {
            Ok(Some(update)) => {
                let version = update.version.clone();
                match update.download_and_install(|_, _| {}, || {}).await {
                    Ok(()) => format!("installed:{version}"),
                    Err(err) => format!("download-failed:{err}"),
                }
            }
            Ok(None) => "current".into(),
            Err(err) => format!("unavailable:{err}"),
        },
        Err(err) => format!("unavailable:{err}"),
    };
    if let Some(store) = app.try_state::<Arc<SkillStore>>() {
        let _ = store.set_setting("updater_last_result", &outcome);
    }
    Ok(outcome)
}

#[tauri::command]
fn rebuild_tray_menu(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    tray::refresh(&app).map_err(|err| err.to_string())
}

#[tauri::command]
async fn list_base_entries() -> Result<Vec<String>, AppError> {
    crate::offload::offload(move || {
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
    })
    .await
}

fn box_err(err: impl std::fmt::Display) -> Box<dyn std::error::Error> {
    err.to_string().into()
}

#[cfg(any(windows, test))]
struct WebviewDriverOverrides {
    browser_args: String,
    data_directory: Option<PathBuf>,
}

/// msedgedriver launches the executable with a debugging port and a user-data
/// directory, then waits for `DevToolsActivePort` in that directory. WebView2
/// only honors those switches when they are applied before the webview exists.
#[cfg(any(windows, test))]
fn webview_driver_overrides<I, S>(
    args: I,
    extra_arguments: Option<&str>,
    configured_directory: Option<&str>,
) -> Option<WebviewDriverOverrides>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut forwarded = Vec::new();
    let mut data_directory = None;
    let mut argv = args.into_iter().peekable();
    while let Some(arg) = argv.next() {
        let arg = arg.as_ref();
        if let Some(path) = arg.strip_prefix("--user-data-dir=") {
            data_directory = Some(PathBuf::from(path));
            continue;
        }
        if arg == "--user-data-dir" {
            if let Some(path) = argv.next() {
                data_directory = Some(PathBuf::from(path.as_ref()));
            }
            continue;
        }
        if arg.starts_with("--remote-debugging-port") || arg.starts_with("--remote-allow-origins") {
            forwarded.push(arg.to_string());
        }
    }
    let extra = extra_arguments
        .map(str::trim)
        .filter(|extra| !extra.is_empty());
    if data_directory.is_none() {
        if let Some(dir) = configured_directory
            .map(str::trim)
            .filter(|dir| !dir.is_empty())
        {
            data_directory = Some(PathBuf::from(dir));
        }
    }
    if forwarded.is_empty() && extra.is_none() && data_directory.is_none() {
        return None;
    }
    let mut browser_args =
        String::from("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection");
    for arg in forwarded {
        browser_args.push(' ');
        if arg.contains(' ') {
            if let Some((flag, value)) = arg.split_once('=') {
                browser_args.push_str(flag);
                browser_args.push_str("=\"");
                browser_args.push_str(value);
                browser_args.push('"');
                continue;
            }
        }
        browser_args.push_str(&arg);
    }
    // WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS is already a browser-argument
    // string. Appending it as one token would quote the first flag's value
    // and swallow every flag after the first space.
    if let Some(extra) = extra {
        browser_args.push(' ');
        browser_args.push_str(extra);
    }
    Some(WebviewDriverOverrides {
        browser_args,
        data_directory,
    })
}

fn driver_log(line: &str) {
    if std::env::var("SKILL_HIVE_DRIVER_LOG").ok().as_deref() != Some("1") {
        return;
    }
    let path = std::env::temp_dir().join("skill-hive-driver.log");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        use std::io::Write;
        let _ = writeln!(file, "{line}");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let args = std::env::args().collect::<Vec<_>>().join(" | ");
    driver_log(&format!("args {args}"));
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
            let window_config = app
                .config()
                .app
                .windows
                .first()
                .cloned()
                .ok_or_else(|| box_err("missing the main window config"))?;
            let window = WebviewWindowBuilder::from_config(app.handle(), &window_config)?;
            #[cfg(windows)]
            let window = if let Some(overrides) = webview_driver_overrides(
                std::env::args().skip(1),
                std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS")
                    .ok()
                    .as_deref(),
                std::env::var("SKILL_HIVE_WEBVIEW_DATA_DIR").ok().as_deref(),
            ) {
                driver_log(&format!(
                    "webview overrides {} {:?}",
                    overrides.browser_args, overrides.data_directory
                ));
                let window = window.additional_browser_args(&overrides.browser_args);
                if let Some(dir) = overrides.data_directory {
                    window.data_directory(dir)
                } else {
                    window
                }
            } else {
                window
            };
            if let Err(err) = window.build() {
                driver_log(&format!("window build failed: {err}"));
                return Err(box_err(err));
            }
            driver_log("window built");
            if let Some(main) = app.get_webview_window("main") {
                webview_watch::watch_process_failures(&main);
            }
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
            let (store, timings) =
                hive_core::core::app_state::initialize_store().map_err(box_err)?;
            timings.log();
            app.manage(store.clone());
            let watched = store.clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|elapsed| elapsed.as_millis() as i64)
                    .unwrap_or(0);
                let _ = commands::maybe_run_auto_update(&watched, now);
                let _ = hive_core::backup::auto_backup::maybe_run_auto_backup(
                    &watched,
                    &central_repo::skills_dir(),
                    (now.max(0) as u64) / 1000,
                );
            });
            progress::bind(app.handle().clone());
            let startup = store.clone();
            tauri::async_runtime::spawn(async move {
                offload::offload(move || {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|elapsed| elapsed.as_millis() as i64)
                        .unwrap_or(0);
                    let _ = commands::maybe_run_auto_update(&startup, now);
                    let _ = hive_core::backup::auto_backup::maybe_run_auto_backup(
                        &startup,
                        &central_repo::skills_dir(),
                        (now.max(0) as u64) / 1000,
                    );
                })
                .await;
            });
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
            log_frontend_error,
            list_base_entries,
            check_app_update,
            update_install_blocker,
            updater_proxy,
            commands::get_managed_skills,
            commands::get_skill_files,
            commands::cancel_job,
            commands::list_presets,
            commands::get_tool_status,
            commands::scan_local_skills,
            commands::install_local_skill,
            commands::install_archive_skill,
            commands::import_existing_skill,
            commands::import_all_discovered,
            commands::preview_git_install,
            commands::confirm_git_install_skill,
            commands::cancel_install,
            commands::delete_managed_skills,
            commands::set_skills_enabled,
            commands::set_skill_tags,
            commands::rename_tag,
            commands::delete_tag,
            commands::sync_skill_to_tool,
            commands::unsync_skill_from_tool,
            commands::add_skill_to_preset,
            commands::remove_skill_from_preset,
            commands::reorder_preset_skills,
            commands::create_preset,
            commands::update_preset,
            commands::delete_preset,
            commands::reorder_presets,
            commands::apply_preset,
            commands::add_custom_tool,
            commands::remove_custom_tool,
            commands::set_tool_enabled,
            commands::set_all_tools_enabled,
            commands::set_tool_order_cmd,
            commands::set_custom_tool_path,
            commands::reset_custom_tool_path,
            commands::set_custom_tool_project_path,
            commands::get_central_repo_path,
            commands::set_central_repo_path,
            commands::check_all_skill_updates,
            commands::check_skill_update,
            commands::apply_available_skill_updates,
            commands::install_manage_skills,
            commands::git_backup_status,
            commands::git_backup_list_versions,
            commands::git_backup_sync,
            commands::git_backup_restore_version,
            commands::git_backup_set_remote,
            commands::git_backup_remove_remote,
            commands::backup_set_device_name,
            commands::add_project,
            commands::fetch_leaderboard,
            commands::search_skillssh,
            commands::install_from_skillssh,
            commands::begin_market_install,
            commands::market_install_result,
            commands::cancel_market_install,
            commands::save_skill_draft,
            commands::snapshot_skill_draft,
            commands::restore_skill_draft,
            commands::publish_skill_draft,
            commands::fix_skill_draft,
            commands::save_agent_draft,
            commands::export_agent_draft,
            commands::interview_for_goal,
            commands::endpoint_prices,
            commands::save_generated_draft,
            commands::apply_optimized_description,
            commands::get_effectiveness_report,
            commands::run_effectiveness_suite,
            commands::optimize_skill_description,
            commands::blind_compare_last,
            commands::control_measured_run,
            commands::export_measured_run,
            commands::save_suite_draft,
            commands::probe_endpoint,
            commands::record_compare_verdict,
            commands::write_history_baseline,
            commands::set_provider_secret,
            commands::clear_endpoint_secret,
            commands::store_backup_pat,
            commands::revoke_backup_token,
            commands::set_aside_backup,
            commands::pull_project_center,
            commands::export_diagnostics,
            commands::git_backup_restore_remote,
            commands::skip_first_run_restore,
            parity_workspace::get_global_local_skills,
            parity_workspace::get_global_local_skill_document,
            parity_workspace::import_global_local_skill_to_center,
            parity_workspace::update_global_local_skill_from_center,
            parity_workspace::delete_global_local_skill,
            parity_backup::git_backup_fetch,
            parity_backup::git_backup_init,
            parity_backup::github_backup_connect,
            parity_backup::github_device_flow_start,
            parity_backup::github_device_flow_poll,
            parity_backup::git_backup_sanitize_remote_url,
            parity_backup::git_backup_commit,
            parity_backup::git_backup_push,
            parity_backup::git_backup_pull,
            parity_backup::git_backup_pending_conflicts,
            parity_backup::git_backup_resolve_conflict,
            parity_backup::git_backup_clone,
            parity_backup::git_backup_reclone,
            parity_backup::git_backup_create_snapshot,
            parity_backup::backup_device_name,
            parity_backup::git_backup_size_report,
            parity_backup::git_backup_migrate_credentials,
            parity_presets::get_presets,
            parity_presets::get_active_preset,
            parity_presets::apply_preset_to_default,
            parity_presets::switch_preset,
            parity_presets::get_preset_skill_order,
            parity_presets::apply_preset_to_coding_agents,
            parity_projects::get_projects,
            parity_projects::add_linked_workspace,
            parity_projects::remove_project,
            parity_projects::reveal_project_folder,
            parity_projects::reorder_projects,
            parity_projects::scan_projects,
            parity_projects::get_project_agent_targets,
            parity_projects::get_project_skills,
            parity_projects::get_project_skill_document,
            parity_projects::import_project_skill_to_center,
            parity_projects::update_project_skill_to_center,
            parity_projects::slugify_skill_names,
            parity_projects::export_skill_to_project,
            parity_projects::update_project_skill_from_center,
            parity_projects::toggle_project_skill,
            parity_projects::delete_project_skill,
            parity_shell::get_central_repo_path_override,
            parity_shell::get_central_repo_pending_path,
            parity_shell::get_central_repo_warnings,
            parity_shell::open_central_repo_folder,
            parity_shell::get_diagnostic_info,
            parity_shell::check_last_panic,
            parity_shell::clear_last_panic,
            parity_shell::get_recent_log_excerpt,
            parity_shell::export_logs_zip,
            parity_shell::app_exit,
            parity_shell::restart_app,
            parity_shell::hide_to_tray,
            parity_skills::get_skills_for_preset,
            parity_skills::get_skill_document,
            parity_skills::get_source_skill_document,
            parity_skills::get_skill_source_diff,
            parity_skills::delete_managed_skill,
            parity_skills::install_local,
            parity_skills::install_git,
            parity_skills::confirm_git_install,
            parity_skills::cancel_git_preview,
            parity_skills::update_skill,
            parity_skills::reimport_local_skill,
            parity_skills::batch_update_skills,
            parity_skills::relink_local_skill_source,
            parity_skills::detach_local_skill_source,
            parity_skills::get_all_tags,
            parity_skills::batch_import_folder,
            parity_skills::get_skill_tool_toggles,
            parity_skills::set_skill_tool_toggle,
            parity_skills::get_tool_order_cmd,
            parity_skills::reset_custom_tool_project_path,
            download_and_install_update,
            rebuild_tray_menu
        ])
        .run(tauri::generate_context!())
        .expect("error while running Skill Hive");
}

#[cfg(test)]
mod tests {
    use super::webview_driver_overrides;

    #[test]
    fn driver_switches_become_webview_arguments() {
        let overrides = webview_driver_overrides(
            [
                "--remote-debugging-port=9222",
                "--user-data-dir=C:\\temp\\edge-profile",
                "--unrelated",
            ],
            None,
            None,
        )
        .expect("driver switches");
        assert!(overrides
            .browser_args
            .contains("--remote-debugging-port=9222"));
        assert!(overrides.browser_args.contains("msSmartScreenProtection"));
        assert!(!overrides.browser_args.contains("unrelated"));
        assert_eq!(
            overrides.data_directory.unwrap(),
            std::path::PathBuf::from("C:\\temp\\edge-profile")
        );
    }

    #[test]
    fn a_normal_launch_does_not_override_the_webview() {
        assert!(webview_driver_overrides(["--help"], None, None).is_none());
    }

    #[test]
    fn an_environment_port_is_forwarded_without_driver_arguments() {
        let overrides = webview_driver_overrides(
            std::iter::empty::<&str>(),
            Some("--remote-debugging-port=9222"),
            Some("C:\\temp\\webview"),
        )
        .expect("environment overrides");
        assert!(overrides
            .browser_args
            .contains("--remote-debugging-port=9222"));
        assert_eq!(
            overrides.data_directory.unwrap(),
            std::path::PathBuf::from("C:\\temp\\webview")
        );
    }

    #[test]
    fn an_environment_argument_string_keeps_every_flag() {
        let overrides = webview_driver_overrides(
            std::iter::empty::<&str>(),
            Some("--remote-debugging-port=9222 --remote-allow-origins=*"),
            None,
        )
        .expect("environment arguments");
        assert!(overrides
            .browser_args
            .contains("--remote-debugging-port=9222 --remote-allow-origins=*"));
        assert!(!overrides.browser_args.contains('"'));
    }
}
