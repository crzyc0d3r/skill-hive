//! Skill Hive desktop shell.

mod commands;
mod error;
mod update;

use std::path::PathBuf;
use std::sync::Arc;

use hive_core::Hive;
use hive_core::paths::HivePaths;
use tauri::{Emitter, Manager};

/// Event name every [`hive_core::model::HiveEvent`] is forwarded under.
pub const EVENT_NAME: &str = "hive://event";

/// When set, fatal errors are only logged and written to the crash file; no native dialog is
/// shown. The CI smoke test sets it so a failure cannot block the runner.
const HEADLESS_ENV: &str = "SKILL_HIVE_HEADLESS";

/// Start logging before anything else can fail. Logs go to `<home>/logs/skill-hive.log`; when
/// that file cannot be opened they go to stderr instead.
fn init_logging(paths: &HivePaths) {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,hive_core=debug"));
    let log_dir = paths.logs_dir();
    let _ = std::fs::create_dir_all(&log_dir);
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

/// Where a fatal error is written in full: next to the log when the home directory is usable,
/// otherwise in the temporary directory.
fn crash_file_path() -> PathBuf {
    let logs = HivePaths::resolve().logs_dir();
    if std::fs::create_dir_all(&logs).is_ok() {
        logs.join("skill-hive-crash.log")
    } else {
        std::env::temp_dir().join("skill-hive-crash.log")
    }
}

/// Record a fatal error where a user can find it. The release binary is a windowed process
/// (no console on Windows) built with `panic = "abort"`, so without this a failure before the
/// window appears would be completely silent.
fn report_fatal(title: &str, detail: &str) {
    tracing::error!(detail, "{title}");
    let crash_file = crash_file_path();
    let _ = std::fs::write(&crash_file, format!("{title}\n\n{detail}\n"));
    let on_main_thread = std::thread::current().name() == Some("main");
    if on_main_thread && std::env::var_os(HEADLESS_ENV).is_none() {
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title(title)
            .set_description(format!(
                "{detail}\n\nThis message was also written to {}",
                crash_file.display()
            ))
            .show();
    }
}

fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let message = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("unknown panic");
        let location = info
            .location()
            .map(|l| format!(" at {}:{}", l.file(), l.line()))
            .unwrap_or_default();
        report_fatal(
            "Skill Hive stopped unexpectedly",
            &format!("{message}{location}"),
        );
        previous(info);
    }));
}

fn fail(title: &str, detail: &str) -> ! {
    report_fatal(title, detail);
    std::process::exit(1)
}

/// WebKitGTK's DMA-BUF renderer misbehaves on a number of GPU, driver and compositor
/// combinations (partially painted windows, wrong colours in composited layers). Prefer the
/// older rendering path unless the user has chosen explicitly through either WebKit variable.
#[cfg(target_os = "linux")]
fn configure_webkit() {
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none()
        && std::env::var_os("WEBKIT_DISABLE_COMPOSITING_MODE").is_none()
    {
        // SAFETY: runs first thing in `run()`, before any other thread exists.
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }
}

pub fn run() {
    #[cfg(target_os = "linux")]
    configure_webkit();
    install_panic_hook();

    let paths = HivePaths::resolve();
    init_logging(&paths);
    tracing::info!(
        home = %paths.home.display(),
        version = hive_core::version(),
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
        pid = std::process::id(),
        webview = %tauri::webview_version().unwrap_or_else(|e| format!("unavailable ({e})")),
        "starting Skill Hive"
    );

    let app = tauri::Builder::default()
        // First, so that a second launch hands over to the running instance before this process
        // touches the database or the keychain.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tracing::info!("another instance started; focusing this window");
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(update::PendingUpdate::default())
        .on_page_load(|webview, payload| {
            tracing::info!(
                window = webview.label(),
                url = %payload.url(),
                event = ?payload.event(),
                "page load"
            );
        })
        .setup(move |app| {
            let hive = match Hive::open(paths) {
                Ok(hive) => hive,
                Err(err) => fail(
                    "Skill Hive could not open its data directory",
                    &err.to_string(),
                ),
            };
            tracing::info!(
                secrets = hive.secrets.backend_name(),
                windows = ?app.webview_windows().keys().collect::<Vec<_>>(),
                webview_data = ?app.path().app_local_data_dir().ok(),
                "core ready"
            );
            app.manage(hive.clone());

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
            commands::ui_report,
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
            commands::merge_propose,
            commands::merge_apply,
            commands::instructions_list,
            commands::instruction_read,
            commands::instruction_write,
            commands::instruction_delete,
            commands::instruction_backups,
            commands::providers_list,
            commands::provider_save,
            commands::provider_delete,
            commands::provider_probe,
            commands::providers_detect,
            commands::providers_ensure_defaults,
            commands::eval_suite_get,
            commands::eval_suite_save,
            commands::eval_suite_draft,
            commands::eval_default_config,
            commands::eval_run_start,
            commands::eval_run_cancel,
            commands::eval_runs_list,
            commands::eval_run_detail,
            commands::eval_run_delete,
            commands::eval_history,
            commands::eval_benchmark_markdown,
            commands::eval_case_transcript,
            update::update_check,
            update::update_install,
        ])
        .build(tauri::generate_context!());

    match app {
        Ok(app) => app.run(|_, _| {}),
        Err(err) => fail("Skill Hive could not start its window", &err.to_string()),
    }
}

#[allow(dead_code)]
fn _assert_send() {
    fn is_send<T: Send + Sync>() {}
    is_send::<Arc<Hive>>();
}
