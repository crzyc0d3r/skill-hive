// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

pub mod central_repo;
pub mod content_hash;
pub mod git_fetcher;
pub mod identity;
pub mod index;
pub mod installer;
pub mod marketplace;
pub mod projects;
pub mod scanner;
pub mod skill_check;
pub mod skill_metadata;
pub mod sync_metadata;
pub mod updates;
pub mod workspace;

#[cfg(test)]
mod open_path;
