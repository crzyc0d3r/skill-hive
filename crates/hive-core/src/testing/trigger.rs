//! Trigger judge. The first tool decides: `Skill` or `Read` naming the command file is a hit.

use std::fs;
use std::path::Path;

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use super::python_bin;

#[derive(Debug, Clone)]
pub struct TriggerQuery {
    pub query: String,
    pub should_trigger: bool,
    pub streams: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TriggerVerdict {
    pub query: String,
    pub should_trigger: bool,
    pub pass: bool,
    pub trigger_rate: f64,
}

pub fn judge_stream(stream: &str, command_name: &str) -> bool {
    let mut tool_name: Option<String> = None;
    let mut buffer = String::new();
    let mut decided: Option<bool> = None;
    for line in stream.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let event = if value.get("type").and_then(Value::as_str) == Some("stream_event") {
            value.get("event").cloned().unwrap_or(Value::Null)
        } else {
            value.clone()
        };
        let event_type = event.get("type").and_then(Value::as_str).unwrap_or("");
        if event_type == "content_block_start" {
            let block = &event["content_block"];
            if block.get("type").and_then(Value::as_str) == Some("tool_use") && tool_name.is_none()
            {
                let name = block
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                if name != "Skill" && name != "Read" {
                    decided = Some(false);
                    break;
                }
                tool_name = Some(name);
            }
        } else if event_type == "content_block_delta" {
            let delta = &event["delta"];
            if delta.get("type").and_then(Value::as_str) == Some("input_json_delta") {
                if let Some(partial) = delta.get("partial_json").and_then(Value::as_str) {
                    buffer.push_str(partial);
                    if tool_name.is_some() && buffer.contains(command_name) {
                        decided = Some(true);
                        break;
                    }
                }
            }
        } else if (event_type == "content_block_stop" || event_type == "message_stop")
            && tool_name.is_some()
        {
            decided = Some(buffer.contains(command_name));
            break;
        }
        if value.get("type").and_then(Value::as_str) == Some("assistant") {
            let content = value.pointer("/message/content").and_then(Value::as_array);
            if let Some(blocks) = content {
                for block in blocks {
                    if block.get("type").and_then(Value::as_str) == Some("tool_use") {
                        let name = block.get("name").and_then(Value::as_str).unwrap_or("");
                        let blob = block.to_string();
                        if tool_name.is_none() {
                            if name != "Skill" && name != "Read" {
                                return false;
                            }
                            return blob.contains(command_name);
                        }
                    }
                }
            }
        }
    }
    decided.unwrap_or(false)
}

pub fn run_trigger_set(
    queries: &[TriggerQuery],
    command_name: &str,
    threshold: f64,
) -> Vec<TriggerVerdict> {
    queries
        .iter()
        .map(|query| {
            let hits = query
                .streams
                .iter()
                .filter(|stream| judge_stream(stream, command_name))
                .count();
            let runs = query.streams.len().max(1) as f64;
            let rate = hits as f64 / runs;
            let pass = if query.should_trigger {
                rate >= threshold
            } else {
                rate < threshold
            };
            TriggerVerdict {
                query: query.query.clone(),
                should_trigger: query.should_trigger,
                pass,
                trigger_rate: rate,
            }
        })
        .collect()
}

pub fn cleanup_command_file(
    project: &Path,
    skill_name: &str,
    token: &str,
) -> Result<std::path::PathBuf> {
    let commands = project.join(".claude").join("commands");
    fs::create_dir_all(&commands)?;
    let path = commands.join(format!("{skill_name}-skill-{token}.md"));
    let body = format!("---\ndescription: |\n  candidate\n---\n# {skill_name}\n");
    fs::write(&path, body)?;
    let timed_out = token == "timeout" || token == "cancel";
    if path.exists() {
        fs::remove_file(&path)?;
    }
    if path.exists() {
        return Err(anyhow!("command file was not deleted"));
    }
    if timed_out {
        return Ok(path);
    }
    Ok(path)
}

pub fn agree_with_python(
    fixture: &Path,
    command_name: &str,
    threshold: f64,
) -> Result<(usize, usize)> {
    let queries = load_queries(fixture)?;
    let rust = run_trigger_set(&queries, command_name, threshold);
    let mut command = python_bin();
    let script = fixture_script("run_eval.py");
    let output = command
        .arg(&script)
        .arg(fixture)
        .arg(command_name)
        .arg(threshold.to_string())
        .output()?;
    if !output.status.success() {
        return Err(anyhow!(String::from_utf8_lossy(&output.stderr).to_string()));
    }
    let parsed: Value = serde_json::from_slice(&output.stdout)?;
    let python = parsed
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut agree = 0_usize;
    for (index, verdict) in rust.iter().enumerate() {
        let python_pass = python
            .get(index)
            .and_then(|row| row.get("pass"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if python_pass == verdict.pass {
            agree += 1;
        }
    }
    Ok((agree, rust.len()))
}

pub fn spawn_trigger_queries(program: &Path, queries: &[String], root: &Path) -> Result<usize> {
    fs::create_dir_all(root)?;
    for (index, query) in queries.iter().enumerate() {
        let dir = root.join(format!("query-{index}"));
        fs::create_dir_all(&dir)?;
        let output = std::process::Command::new(program)
            .arg("-p")
            .arg(query)
            .current_dir(&dir)
            .output()
            .map_err(|err| anyhow!(err))?;
        if !output.status.success() {
            return Err(anyhow!(
                "trigger query {index} exited {}",
                output.status.code().unwrap_or(-1)
            ));
        }
    }
    Ok(queries.len())
}

pub fn load_queries(path: &Path) -> Result<Vec<TriggerQuery>> {
    let value: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
    let rows = value
        .get("queries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(rows
        .iter()
        .map(|row| TriggerQuery {
            query: row
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            should_trigger: row
                .get("should_trigger")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            streams: row
                .get("streams")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect())
}

fn fixture_script(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/testing")
        .join(name)
}

pub fn write_trigger_results(path: &Path, verdicts: &[TriggerVerdict]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let passed = verdicts.iter().filter(|row| row.pass).count();
    let body = json!({
        "results": verdicts.iter().map(|row| json!({
            "query": row.query,
            "should_trigger": row.should_trigger,
            "trigger_rate": row.trigger_rate,
            "pass": row.pass
        })).collect::<Vec<_>>(),
        "summary": {"total": verdicts.len(), "passed": passed, "failed": verdicts.len() - passed}
    });
    fs::write(path, serde_json::to_string_pretty(&body)?)?;
    Ok(())
}
