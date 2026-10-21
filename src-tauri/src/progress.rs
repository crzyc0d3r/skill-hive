use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

static APP: Mutex<Option<AppHandle>> = Mutex::new(None);
static CANCELLED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobProgress {
    pub job: String,
    pub current: u32,
    pub total: u32,
    pub message: String,
    pub done: bool,
}

pub fn bind(app: AppHandle) {
    if let Ok(mut slot) = APP.lock() {
        *slot = Some(app);
    }
}

pub fn reset_cancel() {
    CANCELLED.store(false, Ordering::Relaxed);
}

pub fn request_cancel() {
    CANCELLED.store(true, Ordering::Relaxed);
}

pub fn cancelled() -> bool {
    CANCELLED.load(Ordering::Relaxed)
}

/// Returns false when the user cancelled. The cancel event is already emitted.
pub fn step(job: &str, current: u32, total: u32, message: impl Into<String>) -> bool {
    if cancelled() {
        emit(job, current, total.max(1), "Cancelled", true);
        return false;
    }
    emit(job, current, total.max(1), message, false);
    true
}

pub fn finish(job: &str, message: impl Into<String>) {
    emit(job, 1, 1, message, true);
}

pub fn emit(job: &str, current: u32, total: u32, message: impl Into<String>, done: bool) {
    let Ok(slot) = APP.lock() else {
        return;
    };
    let Some(app) = slot.as_ref() else {
        return;
    };
    let _ = app.emit(
        "job-progress",
        JobProgress {
            job: job.to_string(),
            current,
            total,
            message: message.into(),
            done,
        },
    );
}
