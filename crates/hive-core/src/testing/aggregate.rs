//! Benchmark aggregation. Numbers match `fixtures/testing/aggregate_benchmark.py`.

use std::fs;
use std::path::Path;

use anyhow::Result;
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy)]
pub struct Stat {
    pub mean: f64,
    pub stddev: f64,
    pub min: f64,
    pub max: f64,
}

pub fn population_stddev(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / values.len() as f64;
    variance.sqrt()
}

pub fn stat(values: &[f64]) -> Stat {
    if values.is_empty() {
        return Stat {
            mean: 0.0,
            stddev: 0.0,
            min: 0.0,
            max: 0.0,
        };
    }
    Stat {
        mean: values.iter().sum::<f64>() / values.len() as f64,
        stddev: population_stddev(values),
        min: values.iter().copied().fold(f64::INFINITY, f64::min),
        max: values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    }
}

pub fn delta_rate(with_skill: f64, without_skill: f64) -> String {
    format!("{:+.2}", with_skill - without_skill)
}

pub fn delta_time(with_skill: f64, without_skill: f64) -> String {
    format!("{:+.1}", with_skill - without_skill)
}

pub fn delta_tokens(with_skill: f64, without_skill: f64) -> String {
    format!("{:+}", (with_skill - without_skill).round() as i64)
}

pub fn delta_string(kind: &str, with_skill: f64, without_skill: f64) -> String {
    match kind {
        "time_seconds" => delta_time(with_skill, without_skill),
        "tokens" => delta_tokens(with_skill, without_skill),
        _ => delta_rate(with_skill, without_skill),
    }
}

pub fn aggregate_benchmark(iteration: &Path, skill_name: &str, model: &str) -> Result<Value> {
    let mut runs = Vec::new();
    if iteration.is_dir() {
        let mut dirs: Vec<_> = fs::read_dir(iteration)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        dirs.sort();
        for eval_dir in dirs {
            for config in ["with_skill", "without_skill"] {
                for run_number in 1..=3 {
                    let folder = if run_number == 1 {
                        eval_dir.join(config)
                    } else {
                        eval_dir.join(format!("{config}-r{run_number}"))
                    };
                    if !folder.is_dir() {
                        continue;
                    }
                    if let Some(row) = read_run(&eval_dir, &folder, config, run_number) {
                        runs.push(row);
                    }
                }
            }
        }
    }
    let summary = summary_for(&runs);
    let notes = analyst_notes(&runs);
    let evals_run: Vec<i64> = {
        let mut ids: Vec<i64> = runs
            .iter()
            .filter_map(|row| row.get("eval_id").and_then(Value::as_i64))
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    };
    let body = json!({
        "metadata": {
            "skill_name": skill_name,
            "skill_path": skill_name,
            "executor_model": model,
            "analyzer_model": "deterministic",
            "timestamp": "2026-01-15T10:30:00Z",
            "evals_run": evals_run,
            "runs_per_configuration": runs.iter().filter(|row| row.get("configuration").and_then(Value::as_str) == Some("with_skill")).count()
        },
        "runs": runs,
        "run_summary": summary,
        "notes": notes
    });
    fs::write(
        iteration.join("benchmark.json"),
        serde_json::to_string_pretty(&body)?,
    )?;
    fs::write(iteration.join("benchmark.md"), render_markdown(&body))?;
    Ok(body)
}

fn read_run(eval_dir: &Path, folder: &Path, configuration: &str, run_number: i64) -> Option<Value> {
    let grading: Value = fs::read_to_string(folder.join("grading.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())?;
    let timing: Value = fs::read_to_string(folder.join("timing.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(json!({}));
    let metrics: Value = fs::read_to_string(folder.join("outputs").join("metrics.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(json!({}));
    let name = eval_dir.file_name()?.to_string_lossy();
    let eval_id = name
        .split('-')
        .nth(1)
        .and_then(|text| text.parse::<i64>().ok())
        .unwrap_or(0);
    let expectations = grading.get("expectations").cloned().unwrap_or(json!([]));
    let summary = grading.get("summary").cloned().unwrap_or(json!({}));
    Some(json!({
        "eval_id": eval_id,
        "eval_name": name,
        "configuration": configuration,
        "run_number": run_number,
        "result": {
            "pass_rate": summary.get("pass_rate").and_then(Value::as_f64).unwrap_or(0.0),
            "passed": summary.get("passed").and_then(Value::as_i64).unwrap_or(0),
            "failed": summary.get("failed").and_then(Value::as_i64).unwrap_or(0),
            "total": summary.get("total").and_then(Value::as_i64).unwrap_or(0),
            "time_seconds": timing.get("total_duration_seconds").and_then(Value::as_f64).unwrap_or(0.0),
            "tokens": timing.get("total_tokens").and_then(Value::as_i64).unwrap_or(0),
            "tool_calls": metrics.get("total_tool_calls").and_then(Value::as_i64).unwrap_or(0),
            "errors": metrics.get("errors_encountered").and_then(Value::as_i64).unwrap_or(0)
        },
        "expectations": expectations,
        "notes": []
    }))
}

fn summary_for(runs: &[Value]) -> Value {
    let mut body = json!({});
    for config in ["with_skill", "without_skill"] {
        let rows: Vec<&Value> = runs
            .iter()
            .filter(|row| row.get("configuration").and_then(Value::as_str) == Some(config))
            .collect();
        let pass = numbers(&rows, "pass_rate");
        let time = numbers(&rows, "time_seconds");
        let tokens = numbers(&rows, "tokens");
        body[config] = json!({
            "pass_rate": stat_json(&stat(&pass)),
            "time_seconds": stat_json(&stat(&time)),
            "tokens": stat_json(&stat(&tokens))
        });
    }
    let with_pass = body
        .pointer("/with_skill/pass_rate/mean")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let without_pass = body
        .pointer("/without_skill/pass_rate/mean")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let with_time = body
        .pointer("/with_skill/time_seconds/mean")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let without_time = body
        .pointer("/without_skill/time_seconds/mean")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let with_tokens = body
        .pointer("/with_skill/tokens/mean")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let without_tokens = body
        .pointer("/without_skill/tokens/mean")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    body["delta"] = json!({
        "pass_rate": delta_rate(with_pass, without_pass),
        "time_seconds": delta_time(with_time, without_time),
        "tokens": delta_tokens(with_tokens, without_tokens)
    });
    body
}

fn numbers(rows: &[&Value], field: &str) -> Vec<f64> {
    rows.iter()
        .filter_map(|row| {
            row.pointer(&format!("/result/{field}"))
                .and_then(Value::as_f64)
                .or_else(|| {
                    row.pointer(&format!("/result/{field}"))
                        .and_then(Value::as_i64)
                        .map(|value| value as f64)
                })
        })
        .collect()
}

fn stat_json(stat: &Stat) -> Value {
    json!({"mean": round4(stat.mean), "stddev": round4(stat.stddev), "min": round4(stat.min), "max": round4(stat.max)})
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn analyst_notes(runs: &[Value]) -> Vec<String> {
    if runs.is_empty() {
        return vec!["No completed cells to analyze.".into()];
    }
    vec!["Deterministic analyst notes. No live model was called.".into()]
}

fn render_markdown(body: &Value) -> String {
    let delta = body
        .pointer("/run_summary/delta/pass_rate")
        .and_then(Value::as_str)
        .unwrap_or("");
    format!("# Benchmark\n\npass_rate delta {delta}\n")
}
