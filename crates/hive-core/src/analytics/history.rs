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

/// Read a persisted run document and write the results view from that document.
pub fn load_persisted_results(run_path: &Path, out: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(run_path).map_err(|err| err.to_string())?;
    let value: Value = serde_json::from_str(&raw).map_err(|err| err.to_string())?;
    let projected = json!({
        "run_id": value.get("run_id").cloned().unwrap_or(Value::Null),
        "prompt": value.get("prompt").cloned().unwrap_or(Value::Null),
        "cells": value.get("cells").cloned().unwrap_or(json!([])),
        "notes": value.get("notes").cloned().unwrap_or(json!([])),
    });
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(
        out,
        serde_json::to_string_pretty(&projected).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())?;
    Ok(projected)
}

/// Regression alerts for a history comparison. Rules come from `regression::evaluate` (R1–R6).
pub fn write_history_alerts(
    baseline_path: &Path,
    current_path: &Path,
    out: &Path,
) -> Result<Vec<super::regression::Alert>, String> {
    let baseline = read_slice(baseline_path)?;
    let current = read_slice(current_path)?;
    let report = super::regression::evaluate(Some(&baseline), &current);
    let alerts: Vec<Value> = report
        .alerts
        .iter()
        .map(|alert| {
            json!({
                "rule": alert.rule,
                "severity": alert.severity,
                "message": alert.message,
                "eval_id": alert.eval_id,
                "cells": alert.cells,
            })
        })
        .collect();
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(
        out,
        serde_json::to_string_pretty(&json!({"alerts": alerts, "flaky": report.flaky}))
            .map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())?;
    Ok(report.alerts)
}

fn read_slice(path: &Path) -> Result<super::regression::RunSlice, String> {
    let text = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&text).map_err(|err| err.to_string())
}
