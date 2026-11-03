//! Comparative evaluation of skills (with-skill vs baseline arms, repeats, grading, analytics).

/// Tracks in-flight runs so they can be cancelled.
#[derive(Debug, Default)]
pub struct RunManager {
    pub(crate) cancels:
        std::sync::Mutex<std::collections::HashMap<String, tokio_util::sync::CancellationToken>>,
}

impl RunManager {
    pub fn new() -> Self {
        Self::default()
    }
}
