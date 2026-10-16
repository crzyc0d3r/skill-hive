//! Preset commands that call the full scenario service.

use std::sync::Arc;

use hive_core::core::scenario_service::{self, BatchApplyMode};
use hive_core::db::skill_store::SkillStore;
use hive_core::deploy::tool_adapters::{all_tool_adapters, ToolCategory};
use hive_core::util::error::AppError;
use serde::Serialize;
use tauri::State;

fn db(err: impl std::fmt::Display) -> AppError {
    AppError::db(err.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetRow {
    pub id: String,
    pub name: String,
    pub skill_ids: Vec<String>,
}

#[tauri::command]
pub fn get_presets(store: State<'_, Arc<SkillStore>>) -> Result<Vec<PresetRow>, AppError> {
    let store = store.inner();
    let rows = store.get_all_scenarios().map_err(db)?;
    let mut presets = Vec::new();
    for row in rows {
        presets.push(PresetRow {
            id: row.id.clone(),
            name: row.name,
            skill_ids: store.get_skill_ids_for_scenario(&row.id).map_err(db)?,
        });
    }
    Ok(presets)
}

#[tauri::command]
pub fn get_active_preset(store: State<'_, Arc<SkillStore>>) -> Result<Option<String>, AppError> {
    store.inner().get_active_scenario_id().map_err(db)
}

#[tauri::command]
pub fn apply_preset_to_default(
    preset_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<usize, AppError> {
    let store = store.inner();
    scenario_service::apply_scenario_to_default(store, preset_id.trim())?;
    Ok(store
        .get_skill_ids_for_scenario(preset_id.trim())
        .map_err(db)?
        .len())
}

#[tauri::command]
pub fn switch_preset(
    preset_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<usize, AppError> {
    apply_preset_to_default(preset_id, store)
}

#[tauri::command]
pub fn get_preset_skill_order(
    preset_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<String>, AppError> {
    store
        .inner()
        .get_skill_ids_for_scenario(preset_id.trim())
        .map_err(db)
}

#[tauri::command]
pub fn apply_preset_to_coding_agents(
    preset_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<usize, AppError> {
    let store = store.inner();
    let skills = store
        .get_skill_ids_for_scenario(preset_id.trim())
        .map_err(db)?;
    let tools: Vec<String> = all_tool_adapters(store)
        .into_iter()
        .filter(|adapter| adapter.category == ToolCategory::Coding)
        .filter(|adapter| adapter.is_custom || adapter.is_installed())
        .map(|adapter| adapter.key)
        .collect();
    scenario_service::apply_skills_to_tools(store, &skills, &tools, BatchApplyMode::Add)?;
    Ok(skills.len())
}
