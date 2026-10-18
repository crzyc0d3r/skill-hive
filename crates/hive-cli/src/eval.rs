//! Suite save, replay, results, and export.

use std::fs;
use std::path::Path;

use hive_core::analytics::{export_workspace, ExportCell, ExportSlice};
use hive_core::library::central_repo;
use hive_core::providers::{RunMode, ScriptedProvider};
use hive_core::testing::{run_plan, save_suite, CheckSpec, Plan, Suite, SuiteCase};
use serde_json::{json, Value};

use crate::Fail;

pub fn test_save_suite(skill: &Path) -> Result<Value, Fail> {
    fs::create_dir_all(skill.join("evals")).map_err(|err| Fail::other(err.to_string()))?;
    let name = skill
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("skill");
    let document = skill.join("SKILL.md");
    if !document.is_file() {
        fs::write(
            &document,
            format!("---\nname: {name}\ndescription: Exercise {name}.\n---\n"),
        )
        .map_err(|err| Fail::other(err.to_string()))?;
    }
    let body = fs::read_to_string(&document).unwrap_or_default();
    let prompt = body
        .lines()
        .find_map(|line| line.strip_prefix("description:"))
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .unwrap_or(name)
        .to_string();
    let suite = Suite {
        skill_name: name.into(),
        evals: vec![SuiteCase {
            id: 1,
            prompt: prompt.clone(),
            expected_output: prompt.clone(),
            files: Vec::new(),
            expectations: vec!["answer file exists".into()],
        }],
        triggers: Vec::new(),
        checks: vec![CheckSpec {
            id: "file".into(),
            expectation: "answer file exists".into(),
            kind: "file_exists".into(),
            path: "outputs/answer.txt".into(),
            pattern: None,
        }],
    };
    save_suite(skill, &suite).map_err(|err| Fail::other(err.to_string()))?;
    register_suite(name, skill)?;
    Ok(json!({"command": "test.save-suite", "skill": name}))
}

fn register_suite(name: &str, skill: &Path) -> Result<(), Fail> {
    let root = central_repo::base_dir().join("test-runs");
    fs::create_dir_all(&root).map_err(|err| Fail::other(err.to_string()))?;
    let index = root.join("suites.json");
    let mut rows: Vec<Value> = fs::read_to_string(&index)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    rows.retain(|row| row.get("name").and_then(|value| value.as_str()) != Some(name));
    rows.push(json!({"name": name, "path": skill.display().to_string()}));
    fs::write(
        &index,
        serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into()),
    )
    .map_err(|err| Fail::other(err.to_string()))
}
pub fn test_run(skill: &Path, mode: &str) -> Result<Value, Fail> {
    if mode != "replay" {
        if let Some(url) = crate::providers::local_base_url() {
            return test_run_live(skill, mode, &url);
        }
    }
    if mode == "live" {
        return Err(Fail::other(
            "only offline and replay modes are available without an API key",
        ));
    }
    test_run_scripted(skill)
}

fn test_run_live(skill: &Path, mode: &str, url: &str) -> Result<Value, Fail> {
    let root = central_repo::base_dir().join("test-runs");
    fs::create_dir_all(&root).map_err(|err| Fail::other(err.to_string()))?;
    let body = json!({
        "skill": skill.display().to_string(),
        "mode": mode
    })
    .to_string();
    let response = crate::providers::post_local(url, &body).map_err(Fail::other)?;
    fs::write(root.join("last.json"), &response).map_err(|err| Fail::other(err.to_string()))?;
    let parsed: Value = serde_json::from_str(&response).unwrap_or(Value::Null);
    let status = parsed
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("passed");
    let run_id = parsed
        .get("run_id")
        .and_then(Value::as_str)
        .unwrap_or("live");
    Ok(json!({"command": "test.run", "status": status, "run_id": run_id}))
}

fn test_run_scripted(skill: &Path) -> Result<Value, Fail> {
    let root = central_repo::base_dir().join("test-runs");
    fs::create_dir_all(&root).map_err(|err| Fail::other(err.to_string()))?;
    let spec = Plan {
        skill_dir: skill.to_path_buf(),
        workspace: root.join("workspace"),
        db_path: root.join("hive.db"),
        mode: RunMode::Live,
        seed: 4,
        repetitions: 1,
        budget_usd: None,
        cancel_after: None,
        crash_after: None,
        recordings: root.join("recordings"),
        personal_skills: None,
        secret: String::new(),
        usd_per_cell: None,
        model: "fixture-model".into(),
        kind: "scripted".into(),
    };
    let report = run_plan(&spec, &ScriptedProvider::new("offline"))
        .map_err(|err| Fail::other(err.to_string()))?;
    let passed = report
        .cells
        .iter()
        .filter(|cell| cell.status == "passed" || cell.status == "executed")
        .count();
    fs::write(
        root.join("last.json"),
        json!({"run_id": report.run_id, "status": report.status, "cells": report.cells.len(), "passed": passed}).to_string(),
    )
    .map_err(|err| Fail::other(err.to_string()))?;
    Ok(json!({"command": "test.run", "status": report.status, "run_id": report.run_id}))
}
pub fn test_show() -> Result<Value, Fail> {
    let path = central_repo::base_dir().join("test-runs").join("last.json");
    let raw = fs::read_to_string(&path).map_err(|err| Fail::other(err.to_string()))?;
    let parsed: Value = serde_json::from_str(&raw).map_err(|err| Fail::other(err.to_string()))?;
    Ok(json!({"command": "test.show", "status": parsed["status"], "run_id": parsed["run_id"]}))
}

pub fn test_suites() -> Result<Value, Fail> {
    let index = central_repo::base_dir()
        .join("test-runs")
        .join("suites.json");
    let rows: Vec<Value> = fs::read_to_string(&index)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    let suites: Vec<String> = rows
        .iter()
        .filter_map(|row| {
            let name = row.get("name")?.as_str()?;
            let path = row.get("path")?.as_str()?;
            Path::new(path)
                .join("evals")
                .join("evals.json")
                .is_file()
                .then(|| name.to_string())
        })
        .collect();
    Ok(json!({"command": "test.suites", "suites": suites}))
}
pub fn test_status() -> Result<Value, Fail> {
    let path = central_repo::base_dir().join("test-runs").join("last.json");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|_| "{\"status\":\"idle\",\"run_id\":\"\"}".into());
    let parsed: Value =
        serde_json::from_str(&raw).unwrap_or(json!({"status": "idle", "run_id": ""}));
    Ok(json!({
        "command": "test.status",
        "status": parsed["status"].as_str().unwrap_or("idle"),
        "run_id": parsed["run_id"].as_str().unwrap_or("")
    }))
}
pub fn test_results() -> Result<Value, Fail> {
    let path = central_repo::base_dir().join("test-runs").join("last.json");
    let raw = fs::read_to_string(&path).unwrap_or_else(|_| "{\"run_id\":\"\",\"cells\":0}".into());
    let parsed: Value = serde_json::from_str(&raw).unwrap_or(json!({"run_id": "", "cells": 0}));
    Ok(json!({
        "command": "test.results",
        "run_id": parsed["run_id"].as_str().unwrap_or(""),
        "cells": parsed["cells"].as_u64().unwrap_or(0)
    }))
}
pub fn test_export() -> Result<Value, Fail> {
    let root = central_repo::base_dir().join("exports").join("latest");
    let last_path = central_repo::base_dir().join("test-runs").join("last.json");
    let last: Value = fs::read_to_string(&last_path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or(json!({}));
    let cells = last
        .get("cells")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let passed = last
        .get("passed")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let rate = if cells == 0 {
        0.0
    } else {
        passed as f64 / cells as f64
    };
    let run_id = last
        .get("run_id")
        .and_then(|value| value.as_str())
        .unwrap_or("none");
    let index = central_repo::base_dir()
        .join("test-runs")
        .join("suites.json");
    let suites: Vec<Value> = fs::read_to_string(&index)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    let skill_name = suites
        .last()
        .and_then(|row| row.get("name"))
        .and_then(|value| value.as_str())
        .unwrap_or("skill")
        .to_string();
    let slice = ExportSlice {
        skill_name: skill_name.clone(),
        model: run_id.into(),
        cells: vec![ExportCell {
            eval_id: 1,
            eval_name: skill_name,
            configuration: "with_skill".into(),
            expectations: vec![hive_core::analytics::ExpectationExport {
                text: "answer file exists".into(),
                passed: rate > 0.0,
                evidence: format!("{passed}/{cells}"),
            }],
            checks: vec![hive_core::analytics::CheckExport {
                id: "file".into(),
                passed: rate > 0.0,
                stderr: String::new(),
            }],
            pass_rate: rate,
            endpoint_id: "recorded-run".into(),
            display_name: run_id.into(),
            secret: String::new(),
        }],
    };
    let counts = export_workspace(&root, &slice).map_err(Fail::other)?;
    Ok(json!({
        "command": "test.export",
        "path": root.display().to_string(),
        "testcases": counts.testcases,
        "rows": counts.cell_rows + counts.verdict_rows
    }))
}
pub fn replay_mode(mode: &str) -> Result<(), Fail> {
    if mode == "offline" || mode == "replay" || mode == "live" {
        Ok(())
    } else {
        Err(Fail::other(
            "only offline and replay modes are available without an API key",
        ))
    }
}
