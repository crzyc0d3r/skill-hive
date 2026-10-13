use std::path::PathBuf;
use std::sync::Arc;

use hive_core::db::skill_store::SkillStore;
use hive_core::library::central_repo;
use hive_core::util::error::AppError;
use hive_core::util::file_watcher::start_file_watcher;
use hive_core::util::notifier::Notifier;
use tauri::{Emitter, Manager, WebviewWindowBuilder};

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

#[cfg(any(windows, test))]
struct WebviewDriverOverrides {
    browser_args: String,
    data_directory: Option<PathBuf>,
}

/// msedgedriver launches the executable with a debugging port and a user-data
/// directory, then waits for `DevToolsActivePort` in that directory. WebView2
/// only honors those switches when they are applied before the webview exists.
#[cfg(any(windows, test))]
fn webview_driver_overrides<I, S>(args: I) -> Option<WebviewDriverOverrides>
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
    if forwarded.is_empty() && data_directory.is_none() {
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
            let window = if let Some(overrides) = webview_driver_overrides(std::env::args().skip(1))
            {
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

#[cfg(test)]
mod tests {
    use super::webview_driver_overrides;

    #[test]
    fn driver_switches_become_webview_arguments() {
        let overrides = webview_driver_overrides([
            "--remote-debugging-port=9222",
            "--user-data-dir=C:\\temp\\edge-profile",
            "--unrelated",
        ])
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
        assert!(webview_driver_overrides(["--help"]).is_none());
    }
}
