// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang
//
// Reference engines live in this module. Infrastructure that already matched
// the reference is re-exported so the ported `crate::core::` paths share one type.

#![allow(
    clippy::all,
    dead_code,
    unused,
    unused_imports,
    unused_variables,
    unused_mut
)]

pub mod app_state;
pub mod auto_backup;
pub use crate::library::central_repo;
pub mod content_hash;
pub use crate::util::error;
pub use crate::util::file_watcher;
pub mod git2_engine;
pub mod git_backup;
pub mod git_credentials;
pub mod git_fetcher;
pub mod installer;
pub mod merge;
pub mod project_scanner;
pub mod removals;
pub use crate::util::repo_lock;
pub mod scenario_service;
pub mod skill_metadata;
pub use crate::db::skill_store;
pub mod sync_engine;
pub mod sync_metadata;
pub use crate::deploy::tool_adapters;
pub mod tool_service;
pub mod well_known;
