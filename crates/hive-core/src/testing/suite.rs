//! `evals.json`, `trigger.json`, and `checks.json` with schema-exact writes.

use std::fs;
use std::path::Path;

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expectation {
    pub text: String,
    pub check_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuiteCase {
    pub id: i64,
    pub prompt: String,
    pub expected_output: String,
    pub files: Vec<String>,
    pub expectations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerRow {
    pub query: String,
    pub should_trigger: bool,
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckSpec {
    pub id: String,
    pub expectation: String,
    pub kind: String,
    pub path: String,
    pub pattern: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suite {
    pub skill_name: String,
    pub evals: Vec<SuiteCase>,
    pub triggers: Vec<TriggerRow>,
    pub checks: Vec<CheckSpec>,
}

pub fn save_suite(skill_dir: &Path, suite: &Suite) -> Result<()> {
    validate_suite(suite)?;
    let evals_dir = skill_dir.join("evals");
    fs::create_dir_all(evals_dir.join("files"))?;
    let evals = json!({
        "skill_name": suite.skill_name,
        "evals": suite.evals.iter().map(|case| json!({
            "id": case.id,
            "prompt": case.prompt,
            "expected_output": case.expected_output,
            "files": case.files,
            "expectations": case.expectations
        })).collect::<Vec<_>>()
    });
    write_exact(&evals_dir.join("evals.json"), &evals)?;
    let triggers = Value::Array(
        suite
            .triggers
            .iter()
            .map(|row| {
                let mut value = json!({"query": row.query, "should_trigger": row.should_trigger});
                if let Some(rationale) = &row.rationale {
                    value["rationale"] = json!(rationale);
                }
                value
            })
            .collect(),
    );
    write_exact(&evals_dir.join("trigger.json"), &triggers)?;
    let checks = json!({
        "checks": suite.checks.iter().map(|check| {
            let mut value = json!({
                "id": check.id,
                "expectation": check.expectation,
                "kind": check.kind,
                "path": check.path
            });
            if let Some(pattern) = &check.pattern {
                value["pattern"] = json!(pattern);
            }
            value
        }).collect::<Vec<_>>()
    });
    write_exact(&evals_dir.join("checks.json"), &checks)?;
    Ok(())
}

pub fn load_suite(skill_dir: &Path) -> Result<Suite> {
    let evals_dir = skill_dir.join("evals");
    let evals: Value = read_json(&evals_dir.join("evals.json"))?;
    let triggers: Value = read_json(&evals_dir.join("trigger.json"))?;
    let checks: Value = read_json(&evals_dir.join("checks.json"))?;
    let suite = Suite {
        skill_name: evals
            .get("skill_name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        evals: evals
            .get("evals")
            .and_then(Value::as_array)
            .unwrap_or(&Vec::new())
            .iter()
            .map(|row| SuiteCase {
                id: row.get("id").and_then(Value::as_i64).unwrap_or(0),
                prompt: row
                    .get("prompt")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                expected_output: row
                    .get("expected_output")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                files: string_array(row.get("files")),
                expectations: string_array(row.get("expectations")),
            })
            .collect(),
        triggers: triggers
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|row| TriggerRow {
                query: row
                    .get("query")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                should_trigger: row
                    .get("should_trigger")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                rationale: row
                    .get("rationale")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            })
            .collect(),
        checks: checks
            .get("checks")
            .and_then(Value::as_array)
            .unwrap_or(&Vec::new())
            .iter()
            .map(|row| CheckSpec {
                id: row
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                expectation: row
                    .get("expectation")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                kind: row
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                path: row
                    .get("path")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                pattern: row
                    .get("pattern")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            })
            .collect(),
    };
    validate_suite(&suite)?;
    Ok(suite)
}

pub fn fix_skill_name(skill_dir: &Path, name: &str) -> Result<()> {
    let mut suite = load_suite(skill_dir)?;
    suite.skill_name = name.to_string();
    save_suite(skill_dir, &suite)
}

fn validate_suite(suite: &Suite) -> Result<()> {
    if suite.skill_name.is_empty() {
        return Err(anyhow!("S-33 skill_name is empty"));
    }
    let mut seen = Vec::new();
    for case in &suite.evals {
        if seen.contains(&case.id) {
            return Err(anyhow!("S-33 duplicate eval id {}", case.id));
        }
        seen.push(case.id);
    }
    Ok(())
}

fn write_exact(path: &Path, value: &Value) -> Result<()> {
    let text = serde_json::to_string_pretty(value)?;
    fs::write(path, format!("{text}\n"))?;
    Ok(())
}

fn read_json(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}
