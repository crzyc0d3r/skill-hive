// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

/// Settings key the object-merge engine writes when a backup round records an error.
/// The reference scheduler takes a Tauri AppHandle, which this crate does not link.
pub const SETTING_LAST_ERROR: &str = "backup_last_auto_error";
