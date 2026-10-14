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
    fs::write(
        skill.join("SKILL.md"),
        "---\nname: fixture-skill\n---\nHIVE_SKILL_MARKER_9f3a\n",
    )
    .map_err(|err| Fail::other(err.to_string()))?;
    let suite = Suite {
        skill_name: "fixture-skill".into(),
        evals: vec![SuiteCase {
            id: 1,
            prompt: "Write the fixture answer.".into(),
            expected_output: "PINECONE".into(),
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
    Ok(json!({"command": "test.save-suite", "skill": "fixture-skill"}))
}
pub fn test_run(skill: &Path) -> Result<Value, Fail> {
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
    fs::write(
        root.join("last.json"),
        json!({"run_id": report.run_id, "status": report.status}).to_string(),
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
    Ok(json!({"command": "test.suites", "suites": ["fixture-skill"]}))
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
    let raw = fs::read_to_string(&path).unwrap_or_else(|_| "{\"run_id\":\"\"}".into());
    let parsed: Value = serde_json::from_str(&raw).unwrap_or(json!({"run_id": ""}));
    Ok(json!({
        "command": "test.results",
        "run_id": parsed["run_id"].as_str().unwrap_or(""),
        "cells": 2
    }))
}
pub fn test_export() -> Result<Value, Fail> {
    let root = central_repo::base_dir().join("exports").join("latest");
    let slice = ExportSlice {
        skill_name: "fixture-skill".into(),
        model: "fixture-model".into(),
        cells: vec![ExportCell {
            eval_id: 1,
            eval_name: "eval-1".into(),
            configuration: "with_skill".into(),
            expectations: vec![hive_core::analytics::ExpectationExport {
                text: "answer file exists".into(),
                passed: true,
                evidence: String::new(),
            }],
            checks: vec![hive_core::analytics::CheckExport {
                id: "file".into(),
                passed: true,
                stderr: String::new(),
            }],
            pass_rate: 1.0,
            endpoint_id: "endpoint-secret-id".into(),
            display_name: "Fixture model".into(),
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
    if mode == "offline" || mode == "replay" {
        Ok(())
    } else {
        Err(Fail::other(
            "only offline and replay modes are available without an API key",
        ))
    }
}
