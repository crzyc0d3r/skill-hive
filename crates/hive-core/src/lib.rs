//! Skill Hive core.
//!
//! Collect skills, subagents and commands from the local machine or online sources, keep them in
//! one library, deploy curated loadouts to agent tools without conflicts, and measure whether a
//! skill actually helps with comparative evaluations.
//!
//! Entry point: [`Hive`].

pub mod conflicts;
pub mod db;
pub mod deploy;
pub mod discovery;
pub mod error;
pub mod evals;
pub mod hash;
pub mod hive;
pub mod library;
pub mod loadouts;
pub mod model;
pub mod paths;
pub mod providers;
pub mod recommend;
pub mod secrets;
pub mod settings;
pub mod skill;
pub mod sources;
pub mod targets;
pub mod util;

pub use error::{HiveError, Result};
pub use hive::Hive;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
