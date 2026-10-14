//! Generation sessions: prompt templates, file envelopes, and recorded replay.
//! Live vendor calls are not made from this module.

mod envelope;
mod safety;
mod session;
mod templates;

pub use envelope::{parse_envelope, write_files, EnvelopeError, RequiredFile};
pub use safety::{scan_scripts, SafetyFlag};
pub use session::{
    agent_envelope, draft_expectations, draft_trigger_set, named_files, replay_goal_attempts,
    skill_envelope, skill_files, GenerationSession, Kind, SessionError, Status,
};
pub use templates::{render, Template};

#[cfg(test)]
mod tests;
