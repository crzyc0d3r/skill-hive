//! Skill Hive core. No Tauri dependency: the app and the CLI both link this crate.
pub mod analytics;
pub mod authoring;
pub mod backup;
pub mod db;
pub mod deploy;
pub mod generation;
pub mod library;
pub mod providers;
pub mod testing;
pub mod updater;
pub mod util;

#[cfg(test)]
mod phase0_tests;
#[cfg(test)]
mod phase1_tests;
