//! Comparative evaluation of skills: with-skill vs baseline arms, repeats, deterministic
//! checks, LLM grading, trigger accuracy and analytics.
//!
//! * [`suite`] reads and writes the eval suite stored inside the skill (`<skill>/evals/`).
//! * [`checks`] evaluates deterministic [`crate::model::CheckKind`]s against a sandbox.
//! * [`grader`] is the LLM-as-judge for expectations that have no check.
//! * [`trigger`] measures whether the skill's description makes an agent pick it.
//! * [`analytics`] turns case results into a [`crate::model::Benchmark`] (statistics, verdict,
//!   expectation patterns, notes, markdown report).
//! * [`runner`] is the engine: it executes every case × arm × repeat through a provider and
//!   persists the results; its `impl Hive` block is the facade.

use std::collections::HashMap;
use std::sync::Mutex;

use tokio_util::sync::CancellationToken;

pub mod analytics;
pub mod checks;
pub mod grader;
pub mod runner;
pub mod suite;
pub mod trigger;

#[cfg(test)]
mod tests;

pub use analytics::{aggregate, benchmark_markdown};
pub use runner::{WITH_SKILL_LABEL, WITHOUT_SKILL_LABEL, validate_arms};
pub use suite::{case_name_from_prompt, draft_suite, read_suite, write_suite};

/// Tracks in-flight runs so they can be cancelled.
#[derive(Debug, Default)]
pub struct RunManager {
    cancels: Mutex<HashMap<String, CancellationToken>>,
}

impl RunManager {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, CancellationToken>> {
        self.cancels.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Register a run and return its cancellation token.
    pub fn register(&self, run_id: &str) -> CancellationToken {
        let token = CancellationToken::new();
        self.lock().insert(run_id.to_string(), token.clone());
        token
    }

    /// The token of an in-flight run.
    pub fn token(&self, run_id: &str) -> Option<CancellationToken> {
        self.lock().get(run_id).cloned()
    }

    /// Cancel a run. Returns false when the run is not in flight in this process.
    pub fn cancel(&self, run_id: &str) -> bool {
        match self.lock().get(run_id) {
            Some(token) => {
                token.cancel();
                true
            }
            None => false,
        }
    }

    /// Forget a run (it reached a terminal state).
    pub fn remove(&self, run_id: &str) {
        self.lock().remove(run_id);
    }

    pub fn is_active(&self, run_id: &str) -> bool {
        self.lock().contains_key(run_id)
    }

    /// Ids of runs in flight.
    pub fn active(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.lock().keys().cloned().collect();
        ids.sort();
        ids
    }
}
