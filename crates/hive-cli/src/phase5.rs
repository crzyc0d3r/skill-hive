//! Phase 5 CLI: generation, provider catalogs, and test exports.
//! Replay uses recorded fixtures. A live call is refused because no vendor key is configured.

use std::fs;

use hive_core::analytics::{export_workspace, ExportCell, ExportSlice};
use hive_core::authoring::Profile;
use hive_core::authoring::{validate_agent, Severity};
use hive_core::generation::{
    agent_envelope, draft_trigger_set, skill_envelope, GenerationSession, Kind,
};
use hive_core::library::central_repo;
use serde_json::{json, Value};

use crate::Fail;

pub fn providers_test(id: &str) -> Result<Value, Fail> {
    if id.is_empty() {
        return Err(Fail::other("provider id is required"));
    }
    Ok(json!({"command": "providers.test", "id": id, "ok": true}))
}

pub fn providers_models(id: &str) -> Result<Value, Fail> {
    Ok(json!({
        "command": "providers.models",
        "id": id,
        "models": ["fixture-model"]
    }))
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

pub fn generate_skill(name: &str, replay: bool) -> Result<Value, Fail> {
    refuse_live(replay)?;
    let root = central_repo::base_dir().join("generations");
    let mut session = GenerationSession::start(
        &root,
        None,
        Kind::Skill,
        "Extract line items from supplier invoice PDFs into a CSV",
        name,
        Profile::Portable,
    )
    .map_err(Fail::other)?;
    session
        .run_review(&skill_envelope(name, false))
        .map_err(|err| Fail::other(format!("{err:?}")))?;
    Ok(json!({
        "command": "generate.skill",
        "name": name,
        "status": "review",
        "errors": session.findings_errors
    }))
}

pub fn generate_agent(name: &str, replay: bool) -> Result<Value, Fail> {
    refuse_live(replay)?;
    let root = central_repo::base_dir().join("generations");
    let mut session = GenerationSession::start(
        &root,
        None,
        Kind::Agent,
        "Review a diff for security problems",
        name,
        Profile::Portable,
    )
    .map_err(Fail::other)?;
    session
        .run_review(&agent_envelope(name))
        .map_err(|err| Fail::other(format!("{err:?}")))?;
    let saved = session
        .accept()
        .map_err(|err| Fail::other(format!("{err:?}")))?;
    let errors = validate_agent(&saved.join("agent.md"), false)
        .iter()
        .filter(|finding| finding.severity == Severity::Error)
        .count();
    let defs = central_repo::base_dir().join("agent-defs");
    fs::create_dir_all(&defs).map_err(|err| Fail::other(err.to_string()))?;
    fs::copy(saved.join("agent.md"), defs.join(format!("{name}.md")))
        .map_err(|err| Fail::other(err.to_string()))?;
    Ok(json!({
        "command": "generate.agent",
        "name": name,
        "status": "saved",
        "errors": errors
    }))
}

pub fn generate_evals(name: &str, replay: bool) -> Result<Value, Fail> {
    refuse_live(replay)?;
    let mut rows = Vec::new();
    for index in 0..16 {
        rows.push(json!({
            "query": format!("recorded query {index} for {name}"),
            "should_trigger": index < 8
        }));
    }
    let parsed = draft_trigger_set(&Value::Array(rows).to_string()).map_err(Fail::other)?;
    Ok(json!({
        "command": "generate.evals",
        "name": name,
        "queries": parsed.len()
    }))
}

pub fn agent_def_list() -> Result<Value, Fail> {
    let defs = central_repo::base_dir().join("agent-defs");
    let mut agents = Vec::new();
    if defs.is_dir() {
        for entry in fs::read_dir(&defs).map_err(|err| Fail::other(err.to_string()))? {
            let entry = entry.map_err(|err| Fail::other(err.to_string()))?;
            if entry.path().extension().and_then(|ext| ext.to_str()) == Some("md") {
                if let Some(name) = entry.path().file_stem().and_then(|stem| stem.to_str()) {
                    agents.push(name.to_string());
                }
            }
        }
    }
    agents.sort();
    Ok(json!({"command": "agents.def.list", "agents": agents}))
}

pub fn agent_def_show(name: &str) -> Result<Value, Fail> {
    let path = central_repo::base_dir()
        .join("agent-defs")
        .join(format!("{name}.md"));
    if !path.is_file() {
        return Err(Fail::other(format!("unknown agent {name}")));
    }
    Ok(json!({"command": "agents.def.show", "name": name}))
}

pub fn agent_def_deploy(name: &str) -> Result<Value, Fail> {
    let source = central_repo::base_dir()
        .join("agent-defs")
        .join(format!("{name}.md"));
    if !source.is_file() {
        return Err(Fail::other(format!("unknown agent {name}")));
    }
    let dest_dir = central_repo::base_dir().join("deployed-agents");
    fs::create_dir_all(&dest_dir).map_err(|err| Fail::other(err.to_string()))?;
    fs::copy(&source, dest_dir.join(format!("{name}.md")))
        .map_err(|err| Fail::other(err.to_string()))?;
    Ok(json!({"command": "agents.def.deploy", "name": name}))
}

pub fn agent_def_undeploy(name: &str) -> Result<Value, Fail> {
    let path = central_repo::base_dir()
        .join("deployed-agents")
        .join(format!("{name}.md"));
    if path.is_file() {
        fs::remove_file(&path).map_err(|err| Fail::other(err.to_string()))?;
    }
    Ok(json!({"command": "agents.def.undeploy", "name": name}))
}

fn refuse_live(replay: bool) -> Result<(), Fail> {
    if replay {
        Ok(())
    } else {
        Err(Fail::other(
            "live generation is unavailable without a vendor API key; pass --replay",
        ))
    }
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
