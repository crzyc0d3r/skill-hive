//! Skill Hive core. No Tauri dependency: the app and the CLI both link this crate.
pub mod analytics;
pub mod authoring;
pub mod backup;
pub mod core;
pub mod db;
pub mod deploy;
pub mod generation;
pub mod library;
pub mod providers;
pub mod testing;
pub mod updater;
pub mod util;

#[cfg(test)]
mod agent_catalog_tests;
#[cfg(test)]
mod db_layout_tests;
#[cfg(test)]
mod deploy_tests;
#[cfg(test)]
mod install_tests;
#[cfg(test)]
mod library_import_tests;
#[cfg(test)]
mod library_sidecar_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod watcher_tests;
#[cfg(test)]
mod workspace_tests;
