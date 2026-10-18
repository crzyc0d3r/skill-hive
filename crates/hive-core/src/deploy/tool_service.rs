// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

use crate::db::skill_store::SkillStore;
use crate::deploy::tool_adapters::{
    all_tool_adapters, custom_tool_paths, custom_tool_project_paths, custom_tools,
    default_tool_adapters, CustomToolDef, ToolAdapter, ToolCategory,
};

pub const BUILTIN_ADAPTER_COUNT: usize = 54;

#[derive(Debug, Clone, Serialize)]
pub struct ToolStatus {
    pub key: String,
    pub display_name: String,
    pub installed: bool,
    pub enabled: bool,
    pub skills_dir: String,
    pub project_dir: String,
    pub category: String,
    pub is_custom: bool,
    pub has_path_override: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomAdd {
    Added,
    SkippedCollision,
}

pub fn builtin_catalog() -> Vec<ToolAdapter> {
    default_tool_adapters()
}

pub fn adapter_installed(adapter: &ToolAdapter, home: &Path) -> bool {
    if adapter.is_custom || adapter.override_skills_dir.is_some() {
        return true;
    }
    let detect = home.join(&adapter.relative_detect_dir);
    if detect.exists() {
        return true;
    }
    if let Some(suffix) = adapter.relative_detect_dir.strip_prefix(".config/") {
        if let Some(config) = dirs::config_dir() {
            if config.join(suffix).exists() {
                return true;
            }
        }
    }
    false
}

pub fn tool_statuses(store: &SkillStore, home: &Path) -> Result<Vec<ToolStatus>> {
    let disabled = disabled_keys(store)?;
    let order = tool_order(store)?;
    let mut rows = Vec::new();
    for adapter in all_tool_adapters(store) {
        let installed = adapter_installed(&adapter, home);
        let sort_order = order
            .iter()
            .position(|key| key == &adapter.key)
            .map(|index| index as i32)
            .unwrap_or(10_000);
        rows.push(ToolStatus {
            key: adapter.key.clone(),
            display_name: adapter.display_name.clone(),
            installed,
            enabled: !disabled.contains(&adapter.key),
            skills_dir: adapter.skills_dir().to_string_lossy().to_string(),
            project_dir: adapter.project_relative_skills_dir().to_string(),
            category: match adapter.category {
                ToolCategory::Coding => "coding".into(),
                ToolCategory::Lobster => "lobster".into(),
            },
            is_custom: adapter.is_custom,
            has_path_override: adapter.has_path_override(),
            sort_order,
        });
    }
    rows.sort_by_key(|row| (row.sort_order, row.display_name.clone()));
    Ok(rows)
}

pub fn installed_keys(store: &SkillStore, home: &Path) -> Result<Vec<String>> {
    let mut keys: Vec<String> = tool_statuses(store, home)?
        .into_iter()
        .filter(|row| row.installed)
        .map(|row| row.key)
        .collect();
    keys.sort();
    Ok(keys)
}

pub fn set_tool_enabled(store: &SkillStore, key: &str, enabled: bool) -> Result<()> {
    let mut disabled = disabled_keys(store)?;
    if enabled {
        disabled.retain(|item| item != key);
    } else if !disabled.iter().any(|item| item == key) {
        disabled.push(key.to_string());
    }
    store.set_setting("disabled_tools", &serde_json::to_string(&disabled)?)?;
    Ok(())
}

pub fn enable_all(store: &SkillStore) -> Result<()> {
    store.set_setting("disabled_tools", "[]")?;
    Ok(())
}

pub fn disable_all(store: &SkillStore) -> Result<()> {
    let keys: Vec<String> = all_tool_adapters(store)
        .into_iter()
        .map(|a| a.key)
        .collect();
    store.set_setting("disabled_tools", &serde_json::to_string(&keys)?)?;
    Ok(())
}

pub fn reorder_tools(store: &SkillStore, keys: &[String]) -> Result<()> {
    crate::core::tool_service::set_tool_order(store, keys)
        .map_err(|err| anyhow::anyhow!(err.to_string()))
}

pub fn set_tool_path(store: &SkillStore, key: &str, path: &str) -> Result<()> {
    let path = crate::core::tool_service::normalize_skills_dir_input(path)
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    let mut paths = custom_tool_paths(store);
    paths.insert(key.to_string(), path);
    crate::core::tool_service::set_custom_tool_paths(store, &paths)
        .map_err(|err| anyhow::anyhow!(err.to_string()))
}

pub fn reset_tool_path(store: &SkillStore, key: &str) -> Result<()> {
    let mut paths = custom_tool_paths(store);
    paths.remove(key);
    store.set_setting("custom_tool_paths", &serde_json::to_string(&paths)?)?;
    Ok(())
}

pub fn set_project_path(store: &SkillStore, key: &str, path: &str) -> Result<()> {
    let path = crate::core::tool_service::normalize_project_relative_skills_dir_input(path)
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    let mut paths = custom_tool_project_paths(store);
    match path {
        Some(path) => {
            paths.insert(key.to_string(), path);
        }
        None => {
            paths.remove(key);
        }
    }
    crate::core::tool_service::set_custom_tool_project_paths(store, &paths)
        .map_err(|err| anyhow::anyhow!(err.to_string()))
}

pub fn add_custom_tool(store: &SkillStore, def: CustomToolDef) -> Result<CustomAdd> {
    if default_tool_adapters()
        .iter()
        .any(|adapter| adapter.key == def.key)
    {
        return Ok(CustomAdd::SkippedCollision);
    }
    let mut customs = custom_tools(store);
    if customs.iter().any(|tool| tool.key == def.key) {
        return Ok(CustomAdd::SkippedCollision);
    }
    customs.push(def);
    crate::core::tool_service::set_custom_tools(store, &customs)
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    Ok(CustomAdd::Added)
}

pub fn remove_custom_tool(store: &SkillStore, key: &str) -> Result<()> {
    let customs: Vec<CustomToolDef> = custom_tools(store)
        .into_iter()
        .filter(|tool| tool.key != key)
        .collect();
    store.set_setting("custom_tools", &serde_json::to_string(&customs)?)?;
    Ok(())
}

fn disabled_keys(store: &SkillStore) -> Result<Vec<String>> {
    Ok(store
        .get_setting("disabled_tools")?
        .and_then(|value| serde_json::from_str(&value).ok())
        .unwrap_or_default())
}

fn tool_order(store: &SkillStore) -> Result<Vec<String>> {
    Ok(store
        .get_setting("tool_order")?
        .and_then(|value| serde_json::from_str(&value).ok())
        .unwrap_or_default())
}

pub fn global_and_project_dirs() -> HashMap<String, (String, String)> {
    default_tool_adapters()
        .into_iter()
        .map(|adapter| {
            (
                adapter.key.clone(),
                (
                    adapter.relative_skills_dir.clone(),
                    adapter.project_relative_skills_dir().to_string(),
                ),
            )
        })
        .collect()
}
