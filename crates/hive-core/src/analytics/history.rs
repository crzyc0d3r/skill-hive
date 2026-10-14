//! `history.json` in the skill-creator shape, with one current best.

use std::fs;
use std::path::Path;

use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub version: String,
    pub parent: Option<String>,
    pub expectation_pass_rate: f64,
    pub grading_result: String,
    pub is_current_best: bool,
}

pub fn write_history(
    path: &Path,
    skill_name: &str,
    entries: &[HistoryEntry],
) -> Result<Value, String> {
    let best = entries.iter().filter(|entry| entry.is_current_best).count();
    if entries.is_empty() || best != 1 {
        return Err("history.json must have exactly one is_current_best".into());
    }
    for entry in entries {
        if !matches!(
            entry.grading_result.as_str(),
            "baseline" | "won" | "lost" | "tie"
        ) {
            return Err(format!(
                "grading_result {} is not allowed",
                entry.grading_result
            ));
        }
    }
    let current = entries
        .iter()
        .find(|entry| entry.is_current_best)
        .map(|entry| entry.version.clone())
        .unwrap_or_default();
    let iterations: Vec<Value> = entries
        .iter()
        .map(|entry| {
            json!({
                "version": entry.version,
                "parent": entry.parent,
                "expectation_pass_rate": entry.expectation_pass_rate,
                "grading_result": entry.grading_result,
                "is_current_best": entry.is_current_best,
            })
        })
        .collect();
    let value = json!({
        "started_at": "2026-01-15T10:30:00Z",
        "skill_name": skill_name,
        "current_best": current,
        "iterations": iterations,
    });
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(
        path,
        serde_json::to_string_pretty(&value).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())?;
    Ok(value)
}
