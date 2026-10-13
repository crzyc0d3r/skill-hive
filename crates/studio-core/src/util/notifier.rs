/// Replaces `tauri::AppHandle::emit` so studio-core never links Tauri.
///
/// The app crate implements this with `AppHandle::emit`. The CLI implements
/// it by writing a JSON line to stderr.
pub trait Notifier: Send + Sync {
    fn emit(&self, event: &str, payload: &str);
}

/// Test double that records events.
#[derive(Default)]
pub struct RecordingNotifier {
    pub events: std::sync::Mutex<Vec<(String, String)>>,
}

impl Notifier for RecordingNotifier {
    fn emit(&self, event: &str, payload: &str) {
        self.events
            .lock()
            .expect("notifier lock")
            .push((event.to_string(), payload.to_string()));
    }
}

/// CLI implementation: one JSON object per line on stderr.
pub struct StderrNotifier;

impl Notifier for StderrNotifier {
    fn emit(&self, event: &str, payload: &str) {
        eprintln!(
            "{}",
            serde_json::json!({"event": event, "payload": payload})
        );
    }
}
