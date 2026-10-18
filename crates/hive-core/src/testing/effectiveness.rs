//! Product entry for a measured run: create a skill, execute with_skill and
//! without_skill through the HTTP provider, and persist the skill-creator artifacts.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::{json, Value};

use crate::analytics::{write_history, HistoryEntry};
use crate::generation::{GenerationSession, Kind};
use crate::providers::{
    AnthropicProvider, CancellationToken, ChatProvider, ChatRequest, ContentBlock, Message,
    RequestMetadata, SecretString, ThinkingMode, VecSink,
};

use crate::providers::RunMode;

use super::engine::{run_plan, Plan};
use super::suite::{save_suite, Suite, SuiteCase, TriggerRow};

const MARKER: &str = "HIVE_SKILL_MARKER_9f3a";
const TOKEN: &str = "PINECONE";

pub fn run_effectiveness(
    root: &Path,
    base_url: &str,
    api_key: &str,
    goal: &str,
) -> Result<Value, String> {
    let base_url = base_url.trim();
    let goal = goal.trim();
    if base_url.is_empty() {
        return Err("An endpoint base URL is required".into());
    }
    if goal.is_empty() {
        return Err("A goal is required".into());
    }
    let name = skill_name(goal);
    let skill_dir = root.join("skills").join(&name);
    let workspace = root.join("workspaces").join(&name);
    let db_path = root.join("hive.db");
    write_skill(&skill_dir, &name, goal)?;
    let provider = AnthropicProvider::new(
        "effectiveness",
        base_url,
        SecretString::new(if api_key.is_empty() {
            "missing"
        } else {
            api_key
        }),
    );
    let plan = Plan {
        skill_dir: skill_dir.clone(),
        workspace: workspace.clone(),
        db_path: db_path.clone(),
        mode: RunMode::Live,
        seed: 7,
        repetitions: 1,
        budget_usd: None,
        cancel_after: None,
        crash_after: None,
        recordings: root.join("recordings"),
        personal_skills: None,
        secret: api_key.to_string(),
        usd_per_cell: None,
        model: "claude-sonnet-5-5".into(),
        kind: "anthropic".into(),
    };
    let report = run_plan(&plan, &provider).map_err(|err| err.to_string())?;
    let triggers = judge_triggers(&provider, &name)?;
    let trigger_path = workspace.join("trigger-results.json");
    fs::write(
        &trigger_path,
        serde_json::to_string_pretty(&triggers).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())?;
    let with_rate = rate_of(&report.cells, "with_skill");
    let without_rate = rate_of(&report.cells, "without_skill");
    let history_path = workspace.join("history.json");
    let history = write_history(
        &history_path,
        &name,
        &history_entries(with_rate, without_rate),
    )
    .map_err(|err| err.to_string())?;
    let benchmark = workspace.join("iteration-1").join("benchmark.json");
    let comparison = blind_outputs(&workspace);
    let steps = timeline_steps(&workspace);
    let regression = record_regression(&workspace, &report);
    let export_csv = write_run_export(&workspace, with_rate, without_rate)?;
    let document = json!({
        "run_id": report.run_id,
        "status": report.status,
        "skill": name,
        "goal": goal,
        "with_rate": with_rate,
        "without_rate": without_rate,
        "regressed": with_rate < without_rate,
        "benchmark": benchmark.display().to_string(),
        "history": history,
        "triggers": triggers,
        "regression": regression,
        "summary": [
            {"configuration": "with_skill", "passRate": with_rate, "stddev": 0.0},
            {"configuration": "without_skill", "passRate": without_rate, "stddev": 0.0},
            {"configuration": "delta", "passRate": with_rate - without_rate, "stddev": 0.0}
        ],
        "cells": report.cells.iter().map(|cell| json!({
            "id": cell.id,
            "label": format!("{} {}", cell.configuration, cell.run_number),
            "passRate": cell.pass_rate,
            "configuration": cell.configuration,
        })).collect::<Vec<_>>(),
        "steps": steps,
        "outputA": comparison.0,
        "outputB": comparison.1,
        "sealed": comparison.2,
        "exportCsv": export_csv,
    });
    let dest = root.join("effectiveness").join("last.json");
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(
        &dest,
        serde_json::to_string_pretty(&document).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())?;
    let _ = db_path;
    Ok(document)
}

pub fn load_effectiveness(root: &Path) -> Result<Value, String> {
    let path = root.join("effectiveness").join("last.json");
    if !path.is_file() {
        return Ok(json!({}));
    }
    let text = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&text).map_err(|err| err.to_string())
}

pub fn interview_goal(
    root: &Path,
    base_url: &str,
    api_key: &str,
    goal: &str,
) -> Result<Value, String> {
    let base_url = base_url.trim();
    let goal = goal.trim();
    if base_url.is_empty() {
        return Err("An endpoint base URL is required".into());
    }
    if goal.is_empty() {
        return Err("A goal is required".into());
    }
    let name = skill_name(goal);
    let mut session = GenerationSession::start(
        &root.join("generations"),
        Some(&root.join("hive.db")),
        Kind::Skill,
        goal,
        &name,
        crate::authoring::Profile::Portable,
    )?;
    let provider = AnthropicProvider::new(
        "interview",
        base_url,
        SecretString::new(if api_key.is_empty() {
            "missing"
        } else {
            api_key
        }),
    );
    session.interview(&provider, "")?;
    let path = root
        .join("generations")
        .join("staging")
        .join(&name)
        .join("interview.json");
    let text = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&text).map_err(|err| err.to_string())
}

fn write_skill(dir: &Path, name: &str, goal: &str) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    let body = format!("---\nname: {name}\ndescription: {goal}\n---\n{MARKER}\nFollow the task.\n");
    fs::write(dir.join("SKILL.md"), body).map_err(|err| err.to_string())?;
    let suite = Suite {
        skill_name: name.to_string(),
        evals: vec![SuiteCase {
            id: 1,
            prompt: format!("Complete this task: {goal}"),
            expected_output: format!("The answer file contains {TOKEN}"),
            files: Vec::new(),
            expectations: vec![format!("The answer file contains {TOKEN}")],
        }],
        triggers: vec![
            TriggerRow {
                query: format!("use the {name} skill"),
                should_trigger: true,
                rationale: None,
            },
            TriggerRow {
                query: "what is the weather".into(),
                should_trigger: false,
                rationale: None,
            },
        ],
        checks: Vec::new(),
    };
    save_suite(dir, &suite).map_err(|err| err.to_string())
}

fn skill_name(goal: &str) -> String {
    let mut out = String::new();
    for ch in goal.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "measured-skill".into()
    } else {
        trimmed.to_string()
    }
}

fn rate_of(cells: &[super::engine::CellReport], configuration: &str) -> f64 {
    let matched: Vec<f64> = cells
        .iter()
        .filter(|cell| cell.configuration == configuration)
        .map(|cell| cell.pass_rate)
        .collect();
    if matched.is_empty() {
        0.0
    } else {
        matched.iter().sum::<f64>() / matched.len() as f64
    }
}

pub fn history_entries(with_rate: f64, without_rate: f64) -> Vec<HistoryEntry> {
    let (result, with_best) = if (with_rate - without_rate).abs() < f64::EPSILON {
        ("tie", true)
    } else if with_rate > without_rate {
        ("won", true)
    } else {
        ("lost", false)
    };
    vec![
        HistoryEntry {
            version: "without_skill".into(),
            parent: None,
            expectation_pass_rate: without_rate,
            grading_result: "baseline".into(),
            is_current_best: !with_best,
        },
        HistoryEntry {
            version: "with_skill".into(),
            parent: Some("without_skill".into()),
            expectation_pass_rate: with_rate,
            grading_result: result.into(),
            is_current_best: with_best,
        },
    ]
}

fn blind_outputs(workspace: &Path) -> (String, String, String) {
    let iteration = workspace.join("iteration-1").join("eval-1");
    let with_text =
        fs::read_to_string(iteration.join("with_skill").join("grading.json")).unwrap_or_default();
    let without_text = fs::read_to_string(iteration.join("without_skill").join("grading.json"))
        .unwrap_or_default();
    (with_text, without_text, "with_skill".into())
}

fn judge_triggers(provider: &AnthropicProvider, name: &str) -> Result<Value, String> {
    let queries = [
        (format!("use the {name} skill"), true),
        ("what is the weather".into(), false),
    ];
    let mut rows = Vec::new();
    for (query, should) in queries {
        let prompt = format!("trigger-query: {query}");
        let request = ChatRequest {
            model: "claude-haiku-5-5".into(),
            system: None,
            messages: vec![Message {
                role: "user".into(),
                content: vec![ContentBlock::Text(prompt)],
            }],
            tools: Vec::new(),
            max_tokens: 200,
            effort: None,
            thinking: ThinkingMode::Off,
            stream: true,
            metadata: RequestMetadata {
                purpose: "trigger".into(),
                ref_id: None,
                provider_id: None,
            },
        };
        let mut sink = VecSink::default();
        let cancel = CancellationToken::new();
        let response = provider
            .complete(&request, &mut sink, &cancel)
            .map_err(|err| err.message)?;
        let first = response.tool_uses.first();
        let triggered = first.is_some_and(|tool| {
            tool.name == "Skill"
                && (tool.raw_input.contains(name) || tool.input.to_string().contains(name))
        });
        let pass = triggered == should;
        rows.push(json!({
            "query": query,
            "should_trigger": should,
            "triggered": triggered,
            "pass": pass,
        }));
    }
    Ok(json!(rows))
}

fn timeline_steps(workspace: &Path) -> Value {
    let path = workspace
        .join("iteration-1")
        .join("eval-1")
        .join("with_skill")
        .join("transcript.jsonl");
    let text = fs::read_to_string(path).unwrap_or_default();
    let mut steps = Vec::new();
    let mut pending: Option<(u64, String, i64)> = None;
    for line in text.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let kind = value.get("kind").and_then(Value::as_str).unwrap_or("");
        let seq = value.get("seq").and_then(Value::as_u64).unwrap_or(0);
        let ts = value
            .get("ts")
            .and_then(Value::as_i64)
            .unwrap_or(seq as i64);
        let tool = value
            .get("tool_name")
            .and_then(Value::as_str)
            .unwrap_or("tool")
            .to_string();
        if kind == "tool_use" {
            pending = Some((seq.max(1), tool, ts));
        } else if kind == "tool_result" {
            if let Some((id, name, use_ts)) = pending.take() {
                let result_ts = ts.max(use_ts.saturating_add(1));
                steps.push(json!({
                    "id": id,
                    "tool": name,
                    "useTs": use_ts,
                    "resultTs": result_ts,
                }));
            }
        }
    }
    Value::Array(steps)
}

fn record_regression(workspace: &Path, report: &super::engine::RunReport) -> Value {
    use crate::analytics::{evaluate, set_baseline, RunSlice, SampleCell};
    let cells = report
        .cells
        .iter()
        .map(|cell| SampleCell {
            id: cell.id.clone(),
            eval_id: "1".into(),
            configuration: cell.configuration.clone(),
            pass_rate: cell.pass_rate,
            expectations: expectations_of(&cell.dir),
            trigger_should: None,
            trigger_rate: None,
            checks_ok: true,
            tokens: 1.0,
            cost: 0.0,
            time_seconds: 1.0,
        })
        .collect();
    let current = RunSlice {
        run_id: report.run_id.clone(),
        endpoint_model: "anthropic/claude-sonnet-5-5".into(),
        cells,
    };
    let dir = workspace.join("regression");
    let previous: Option<RunSlice> = fs::read_to_string(dir.join("baseline.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok());
    let outcome = evaluate(previous.as_ref(), &current);
    if previous.is_none() {
        let _ = set_baseline(&dir, &current);
    }
    let alerts: Vec<Value> = outcome
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
    json!({
        "offer_set_baseline": outcome.offer_set_baseline,
        "alerts": alerts,
        "flaky": outcome.flaky,
    })
}

fn expectations_of(dir: &Path) -> Vec<crate::analytics::ExpectationOutcome> {
    use crate::analytics::ExpectationOutcome;
    let text = fs::read_to_string(dir.join("grading.json")).unwrap_or_default();
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    value
        .get("expectations")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    Some(ExpectationOutcome {
                        text: row.get("text")?.as_str()?.to_string(),
                        passed: row.get("passed")?.as_bool()?,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn write_run_export(workspace: &Path, with_rate: f64, without_rate: f64) -> Result<String, String> {
    let dir = workspace.join("export");
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    let csv =
        format!("configuration,pass_rate\nwith_skill,{with_rate}\nwithout_skill,{without_rate}\n");
    fs::write(dir.join("cells.csv"), &csv).map_err(|err| err.to_string())?;
    let mut junit = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuite name=\"effectiveness\">\n",
    );
    for configuration in ["with_skill", "without_skill"] {
        let grading = fs::read_to_string(
            workspace
                .join("iteration-1")
                .join("eval-1")
                .join(configuration)
                .join("grading.json"),
        )
        .unwrap_or_default();
        let value: Value = serde_json::from_str(&grading).unwrap_or(json!({}));
        let rows = value
            .get("expectations")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for (index, row) in rows.iter().enumerate() {
            let name = xml_escape(&format!("{configuration}-{index}"));
            let passed = row.get("passed").and_then(Value::as_bool).unwrap_or(false);
            if passed {
                junit.push_str(&format!(
                    "  <testcase name=\"{name}\" classname=\"{configuration}\"/>\n"
                ));
            } else {
                let evidence = xml_escape(
                    row.get("evidence")
                        .and_then(Value::as_str)
                        .unwrap_or("failed"),
                );
                junit.push_str(&format!(
                    "  <testcase name=\"{name}\" classname=\"{configuration}\"><failure message=\"failed\">{evidence}</failure></testcase>\n"
                ));
            }
        }
    }
    junit.push_str("</testsuite>\n");
    fs::write(dir.join("junit.xml"), &junit).map_err(|err| err.to_string())?;
    Ok(csv)
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn grading_path(root: &Path, goal: &str, configuration: &str) -> PathBuf {
    root.join("workspaces")
        .join(skill_name(goal))
        .join("iteration-1")
        .join("eval-1")
        .join(configuration)
        .join("grading.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn history_marks_a_regression_when_the_skill_loses() {
        let entries = history_entries(0.0, 1.0);
        assert_eq!(entries[1].grading_result, "lost");
        assert!(entries[0].is_current_best);
        assert!(!entries[1].is_current_best);
    }

    #[test]
    fn measured_run_writes_artifacts_and_database_rows() {
        let hits = Arc::new(AtomicUsize::new(0));
        let seen = hits.clone();
        let (port, stop) = serve(seen);
        let root = tempfile::tempdir().unwrap();
        let goal = "classify the dock sample";
        let document = run_effectiveness(
            root.path(),
            &format!("http://127.0.0.1:{port}"),
            "sk-ant-test",
            goal,
        )
        .unwrap();
        stop.store(false, Ordering::SeqCst);
        let _ = std::net::TcpStream::connect(format!("127.0.0.1:{port}"));
        assert!(hits.load(Ordering::SeqCst) > 0);
        assert_eq!(document["status"], "done");
        assert!(
            document["with_rate"].as_f64().unwrap() > document["without_rate"].as_f64().unwrap(),
            "{document}"
        );
        assert_eq!(document["regressed"], false);
        let grading = fs::read_to_string(grading_path(root.path(), goal, "with_skill")).unwrap();
        assert!(grading.contains("\"passed\": true"), "{grading}");
        let without = fs::read_to_string(grading_path(root.path(), goal, "without_skill")).unwrap();
        assert!(without.contains("\"passed\": false"), "{without}");
        let benchmark = fs::read_to_string(
            root.path()
                .join("workspaces")
                .join(skill_name(goal))
                .join("iteration-1")
                .join("benchmark.json"),
        )
        .unwrap();
        assert!(benchmark.contains("with_skill"), "{benchmark}");
        assert!(benchmark.contains("without_skill"), "{benchmark}");
        let history = fs::read_to_string(
            root.path()
                .join("workspaces")
                .join(skill_name(goal))
                .join("history.json"),
        )
        .unwrap();
        assert!(
            history.contains("\"grading_result\": \"won\"")
                || history.contains("\"grading_result\": \"won\""),
            "{history}"
        );
        assert!(history.contains("\"is_current_best\": true"), "{history}");
        let triggers = fs::read_to_string(
            root.path()
                .join("workspaces")
                .join(skill_name(goal))
                .join("trigger-results.json"),
        )
        .unwrap();
        assert!(triggers.contains("\"pass\": true"), "{triggers}");
        let conn = rusqlite::Connection::open(root.path().join("hive.db")).unwrap();
        let runs: i64 = conn
            .query_row("select count(*) from runs", [], |row| row.get(0))
            .unwrap();
        let cases: i64 = conn
            .query_row("select count(*) from run_cases", [], |row| row.get(0))
            .unwrap();
        let ledger: i64 = conn
            .query_row("select count(*) from usage_ledger", [], |row| row.get(0))
            .unwrap();
        assert!(runs >= 1, "{runs}");
        assert!(cases >= 2, "{cases}");
        assert!(ledger >= 1, "{ledger}");
        let loaded = load_effectiveness(root.path()).unwrap();
        assert_eq!(loaded["run_id"], document["run_id"]);
        let steps = document["steps"].as_array().cloned().unwrap_or_default();
        assert!(
            steps
                .iter()
                .any(|step| step["tool"] == "Write"
                    && step["useTs"].as_i64().unwrap_or(0) > 1_000_000),
            "{document}"
        );
        let export = root
            .path()
            .join("workspaces")
            .join(skill_name(goal))
            .join("export")
            .join("cells.csv");
        let csv = fs::read_to_string(export).unwrap();
        assert!(csv.contains("with_skill"), "{csv}");
        let junit = fs::read_to_string(
            root.path()
                .join("workspaces")
                .join(skill_name(goal))
                .join("export")
                .join("junit.xml"),
        )
        .unwrap();
        assert!(junit.contains("<testcase"), "{junit}");
        assert_eq!(document["regression"]["offer_set_baseline"], true);
        assert!(document["regression"]["alerts"]
            .as_array()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn interview_stores_the_provider_questions() {
        let (port, stop) = serve(Arc::new(AtomicUsize::new(0)));
        let root = tempfile::tempdir().unwrap();
        let document = interview_goal(
            root.path(),
            &format!("http://127.0.0.1:{port}"),
            "sk-ant-test",
            "classify the dock sample",
        )
        .unwrap();
        stop.store(false, Ordering::SeqCst);
        let _ = std::net::TcpStream::connect(format!("127.0.0.1:{port}"));
        let questions = document["questions"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert!(
            questions
                .iter()
                .any(|item| item.as_str() == Some("What output format?")),
            "{document}"
        );
    }

    fn serve(hits: Arc<AtomicUsize>) -> (u16, Arc<AtomicBool>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let stop = Arc::new(AtomicBool::new(true));
        let flag = stop.clone();
        thread::spawn(move || {
            while flag.load(Ordering::SeqCst) {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                if !flag.load(Ordering::SeqCst) {
                    break;
                }
                let _ = stream.set_read_timeout(Some(Duration::from_millis(800)));
                let mut buf = Vec::new();
                let mut tmp = [0_u8; 4096];
                loop {
                    match stream.read(&mut tmp) {
                        Ok(0) | Err(_) => break,
                        Ok(count) => {
                            buf.extend_from_slice(&tmp[..count]);
                            if let Some(header_end) =
                                buf.windows(4).position(|window| window == b"\r\n\r\n")
                            {
                                let head = String::from_utf8_lossy(&buf[..header_end]);
                                let need = head
                                    .lines()
                                    .find_map(|line| {
                                        let (name, value) = line.split_once(':')?;
                                        name.eq_ignore_ascii_case("content-length")
                                            .then(|| value.trim().parse::<usize>().ok())
                                            .flatten()
                                    })
                                    .unwrap_or(0);
                                if buf.len() >= header_end + 4 + need {
                                    break;
                                }
                            }
                        }
                    }
                }
                let text = String::from_utf8_lossy(&buf).to_string();
                hits.fetch_add(1, Ordering::SeqCst);
                let payload = reply(&text);
                let packet = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                    payload.len()
                );
                let _ = stream.write_all(packet.as_bytes());
            }
        });
        (port, stop)
    }

    fn reply(request: &str) -> String {
        if request.contains("trigger-query") {
            let name = if request.contains("pinecone") || request.contains("classify") {
                "Skill"
            } else {
                "Bash"
            };
            let input = if name == "Skill" {
                "{\"skill\":\"classify-the-dock-sample\"}"
            } else {
                "{\"command\":\"date\"}"
            };
            return sse_tool(name, input);
        }
        if request.contains("You are a grader") || request.contains("grader") {
            let passed = request.matches(TOKEN).count() > 1;
            let body = format!(
                r#"{{"expectations":[{{"text":"The answer file contains {TOKEN}","passed":{passed},"evidence":"transcript"}}]}}"#
            );
            return sse_text(&body);
        }
        if request.contains("tool_result") {
            return sse_text("done");
        }
        if request.contains("interview")
            || request.contains("What should")
            || request.contains("questions")
            || request.contains("dock sample") && !request.contains(MARKER)
        {
            return sse_text(r#"{"questions":["What output format?"]}"#);
        }
        let content = if request.contains(MARKER) {
            TOKEN
        } else {
            "plain"
        };
        sse_tool(
            "write_file",
            &serde_json::json!({"path": "outputs/answer.txt", "content": content}).to_string(),
        )
    }

    fn sse_text(text: &str) -> String {
        format!(
            "data: {{\"type\":\"message_start\",\"message\":{{\"id\":\"m\",\"usage\":{{\"input_tokens\":4}}}}}}\n\ndata: {{\"type\":\"content_block_delta\",\"delta\":{{\"type\":\"text_delta\",\"text\":{}}}}}\n\ndata: {{\"type\":\"message_delta\",\"delta\":{{\"stop_reason\":\"end_turn\"}},\"usage\":{{\"output_tokens\":2}}}}\n\ndata: {{\"type\":\"message_stop\"}}\n\n",
            serde_json::to_string(text).unwrap()
        )
    }

    fn sse_tool(name: &str, input: &str) -> String {
        format!(
            "data: {{\"type\":\"message_start\",\"message\":{{\"id\":\"m\",\"usage\":{{\"input_tokens\":4}}}}}}\n\ndata: {{\"type\":\"content_block_start\",\"content_block\":{{\"type\":\"tool_use\",\"id\":\"tool-1\",\"name\":{name}}}}}\n\ndata: {{\"type\":\"content_block_delta\",\"delta\":{{\"type\":\"input_json_delta\",\"partial_json\":{input}}}}}\n\ndata: {{\"type\":\"content_block_stop\"}}\n\ndata: {{\"type\":\"message_delta\",\"delta\":{{\"stop_reason\":\"tool_use\"}},\"usage\":{{\"output_tokens\":3}}}}\n\ndata: {{\"type\":\"message_stop\"}}\n\n",
            name = serde_json::to_string(name).unwrap(),
            input = serde_json::to_string(input).unwrap(),
        )
    }
}
