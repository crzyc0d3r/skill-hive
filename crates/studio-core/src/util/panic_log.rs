// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

use std::backtrace::Backtrace;
use std::fs;
use std::io::Write;
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn last_panic_path(log_dir: &Path) -> PathBuf {
    log_dir.join("last_panic.log")
}

pub fn cached_last_panic_path() -> Option<PathBuf> {
    LOG_DIR.get().map(|dir| dir.join("last_panic.log"))
}

/// Install a panic hook that writes `last_panic.log` under `log_dir`.
///
/// The reference hooked this through `tauri::AppHandle`. Studio-core has no
/// Tauri dependency, so the app crate passes the log directory in.
pub fn install_panic_hook(log_dir: PathBuf) {
    let _ = fs::create_dir_all(&log_dir);
    let _ = LOG_DIR.set(log_dir);

    let prev = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let backtrace = Backtrace::capture();
        let timestamp = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%:z");
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<unknown>".into());
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "<non-string panic payload>".to_string()
        };

        let body =
            format!("[{timestamp}] PANIC at {location}\n{payload}\n\nBacktrace:\n{backtrace}\n");

        log::error!("panic: {payload} at {location}");

        if let Some(dir) = LOG_DIR.get() {
            let path = dir.join("last_panic.log");
            if let Ok(mut f) = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&path)
            {
                let _ = f.write_all(body.as_bytes());
            }
        }

        prev(info);
    }));
}
