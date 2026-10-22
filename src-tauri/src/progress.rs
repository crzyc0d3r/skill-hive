use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

#[cfg(test)]
mod capture {
    use std::cell::Cell;
    use std::sync::Mutex;

    thread_local! {
        static CAPTURE_JOBS: Cell<bool> = const { Cell::new(false) };
    }
    static CAPTURED_JOBS: Mutex<Vec<String>> = Mutex::new(Vec::new());

    pub fn capture_job_events() {
        CAPTURE_JOBS.with(|flag| flag.set(true));
        if let Ok(mut events) = CAPTURED_JOBS.lock() {
            events.clear();
        }
    }

    pub fn captured_job_events() -> Vec<String> {
        CAPTURE_JOBS.with(|flag| flag.set(false));
        CAPTURED_JOBS
            .lock()
            .map(|mut events| std::mem::take(&mut *events))
            .unwrap_or_default()
    }

    pub fn record_job_event(phase: &str, job: &str, current: u32, total: u32) {
        let capture = CAPTURE_JOBS.with(|flag| flag.get());
        if !capture {
            return;
        }
        if let Ok(mut events) = CAPTURED_JOBS.lock() {
            events.push(format!("{phase} {job} {current}/{total}"));
        }
    }
}

#[cfg(test)]
pub use capture::{capture_job_events, captured_job_events};

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
    let phase = if done { "stop" } else { "start" };
    log::info!("job-event {phase} job={job} current={current} total={total}");
    #[cfg(test)]
    capture::record_job_event(phase, job, current, total);
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
