//! Run lifecycle. Cells interleave configurations. Replay never calls the live provider.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use rusqlite::Connection;
use serde_json::{json, Value};

use crate::providers::{
    scrub, CancellationToken, ChatProvider, ChatRequest, ChatResponse, ContentBlock, Limiter,
    Message, Middleware, NoopSleeper, Pricing, RequestMetadata, RunMode, StopReason, ThinkingMode,
    ToolDef, ToolUse, Usage, VecSink,
};
use crate::util::path_guard::is_path_safe;

use super::aggregate::aggregate_benchmark;
use super::checks::{run_check, DeterministicCheck};
use super::suite::{load_suite, CheckSpec, Suite};
use super::trigger::{load_queries, run_trigger_set, write_trigger_results};

#[derive(Clone)]
pub struct Plan {
    pub skill_dir: PathBuf,
    pub workspace: PathBuf,
    pub db_path: PathBuf,
    pub mode: RunMode,
    pub seed: u64,
    pub repetitions: u32,
    pub budget_usd: Option<f64>,
    pub cancel_after: Option<usize>,
    pub crash_after: Option<usize>,
    pub recordings: PathBuf,
    pub personal_skills: Option<PathBuf>,
    pub secret: String,
    pub usd_per_cell: Option<f64>,
    pub model: String,
    pub kind: String,
}

#[derive(Debug, Clone)]
pub struct CellReport {
    pub id: String,
    pub configuration: String,
    pub run_number: u32,
    pub status: String,
    pub dir: PathBuf,
    pub pass_rate: f64,
}

#[derive(Debug, Clone)]
pub struct RunReport {
    pub run_id: String,
    pub status: String,
    pub cells: Vec<CellReport>,
    pub alias: Option<String>,
    pub paused: bool,
    pub progress: Vec<String>,
}

struct CellSpec {
    eval_id: i64,
    name: String,
    prompt: String,
    configuration: String,
    run_number: u32,
    expectations: Vec<String>,
}

struct Clock {
    origin: i64,
    tick: AtomicU64,
}

impl Clock {
    fn new(seed: u64) -> Self {
        Self {
            origin: 1_736_000_000 + i64::try_from(seed % 10_000).unwrap_or(0),
            tick: AtomicU64::new(0),
        }
    }

    fn now(&self) -> i64 {
        self.origin + i64::try_from(self.tick.fetch_add(1, Ordering::Relaxed)).unwrap_or(0)
    }
}

pub fn run_plan(plan: &Plan, provider: &dyn ChatProvider) -> Result<RunReport> {
    let suite = load_suite(&plan.skill_dir)?;
    let conn = open_db(&plan.db_path)?;
    ensure_skill(&conn, &suite.skill_name, &plan.skill_dir)?;
    let run_id = format!("run-{:016x}", plan.seed);
    persist_suite(&conn, &suite, &run_id, plan)?;
    let mut cells = expand(&suite, plan.repetitions);
    order_cells(&mut cells, plan.seed);
    let alias = collision_alias(&suite.skill_name, &run_id, plan.personal_skills.as_deref());
    let mut report = RunReport {
        run_id: run_id.clone(),
        status: "running".into(),
        cells: Vec::new(),
        alias: alias.clone(),
        paused: false,
        progress: vec!["preparing".into(), "running".into()],
    };
    fs::create_dir_all(&plan.workspace)?;
    fs::write(
        plan.workspace.join("prepare.json"),
        json!({"alias": alias, "seed": plan.seed}).to_string(),
    )?;
    execute_cells(plan, provider, &conn, &suite, &cells, &mut report, false)?;
    Ok(report)
}

pub fn resume_run(plan: &Plan, provider: &dyn ChatProvider) -> Result<RunReport> {
    let suite = load_suite(&plan.skill_dir)?;
    let conn = open_db(&plan.db_path)?;
    let run_id = format!("run-{:016x}", plan.seed);
    let mut cells = expand(&suite, plan.repetitions);
    order_cells(&mut cells, plan.seed);
    let done = completed_ids(&conn, &run_id)?;
    cells.retain(|cell| !done.contains(&cell_id(&run_id, cell)));
    let alias = collision_alias(&suite.skill_name, &run_id, plan.personal_skills.as_deref());
    let mut report = RunReport {
        run_id,
        status: "running".into(),
        cells: Vec::new(),
        alias,
        paused: false,
        progress: vec!["running".into()],
    };
    let mut resumed = plan.clone();
    resumed.crash_after = None;
    resumed.cancel_after = None;
    execute_cells(&resumed, provider, &conn, &suite, &cells, &mut report, true)?;
    Ok(report)
}

fn execute_cells(
    plan: &Plan,
    provider: &dyn ChatProvider,
    conn: &Connection,
    suite: &Suite,
    cells: &[CellSpec],
    report: &mut RunReport,
    resuming: bool,
) -> Result<()> {
    let clock = Clock::new(plan.seed);
    let mut spent = if resuming {
        spent_usd(conn, &report.run_id)?
    } else {
        0.0
    };
    let mut completed = if resuming {
        completed_ids(conn, &report.run_id)?.len()
    } else {
        0
    };
    let iteration = plan.workspace.join("iteration-1");
    fs::create_dir_all(&iteration)?;
    for cell in cells {
        if let Some(after) = plan.cancel_after {
            if completed >= after {
                mark_cancelled(conn, &report.run_id, &suite.skill_name, cell)?;
                report.cells.push(cell_report(
                    &report.run_id,
                    cell,
                    "cancelled",
                    0.0,
                    &cell_dir(&iteration, cell),
                ));
                continue;
            }
        }
        if let Some(limit) = plan.budget_usd {
            if spent >= limit {
                report.paused = true;
                report.status = "paused".into();
                report.progress.push("paused".into());
                set_run_status(conn, &report.run_id, "paused")?;
                return Ok(());
            }
        }
        if let Some(after) = plan.crash_after {
            if completed >= after {
                report.status = "running".into();
                set_run_status(conn, &report.run_id, "running")?;
                report.progress.push("crashed".into());
                return Ok(());
            }
        }
        let dir = cell_dir(&iteration, cell);
        fs::create_dir_all(dir.join("outputs"))?;
        let sandbox = plan
            .workspace
            .join("sandboxes")
            .join(cell_id(&report.run_id, cell));
        fs::create_dir_all(&sandbox)?;
        inject_skill(
            &plan.skill_dir,
            &sandbox,
            &suite.skill_name,
            &cell.configuration,
            report.alias.as_deref(),
        )?;
        let outcome = execute_cell(plan, provider, cell, &sandbox, &dir, &clock, suite)?;
        let cost = plan.usd_per_cell.unwrap_or(outcome.cost);
        spent += cost;
        let over_budget = plan.budget_usd.is_some_and(|limit| spent >= limit);
        let (graded, pass_rate) = if over_budget {
            write_pending_grade(&dir)?;
            ("executed", 0.0)
        } else {
            let transcript_text =
                fs::read_to_string(dir.join("transcript.jsonl")).unwrap_or_default();
            let metrics: Value = fs::read_to_string(dir.join("outputs").join("metrics.json"))
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or(json!({}));
            let timing: Value = fs::read_to_string(dir.join("timing.json"))
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or(json!({}));
            let (grading, pass_rate) = grade_cell(&GradeInput {
                plan,
                provider,
                cell,
                dir: &dir,
                suite,
                transcript: &transcript_text,
                metrics: &metrics,
                timing: &timing,
                secret: &plan.secret,
            })?;
            fs::write(
                dir.join("grading.json"),
                serde_json::to_string_pretty(&grading)?,
            )?;
            ("graded", pass_rate)
        };
        write_ledger(conn, plan, cell, cost, &outcome.usage)?;
        save_case(conn, &report.run_id, &suite.skill_name, cell, graded, &dir)?;
        report
            .cells
            .push(cell_report(&report.run_id, cell, graded, pass_rate, &dir));
        completed += 1;
        if over_budget {
            report.paused = true;
            report.status = "paused".into();
            report.progress.push("paused".into());
            set_run_status(conn, &report.run_id, "paused")?;
            return Ok(());
        }
    }
    if report.cells.iter().any(|cell| cell.status == "cancelled")
        && report
            .cells
            .iter()
            .all(|cell| cell.status == "cancelled" || cell.status == "graded")
    {
        report.status = "cancelled".into();
        set_run_status(conn, &report.run_id, "cancelled")?;
        report.progress.push("cancelled".into());
        return Ok(());
    }
    let triggers = trigger_path(plan);
    if triggers.exists() {
        let queries = load_queries(&triggers)?;
        let verdicts = run_trigger_set(&queries, &format!("{}-skill", suite.skill_name), 0.5);
        write_trigger_results(
            &plan
                .workspace
                .join("trigger")
                .join(&report.run_id)
                .join("results.json"),
            &verdicts,
        )?;
    }
    aggregate_benchmark(&iteration, &suite.skill_name, &plan.model)?;
    report.status = "done".into();
    report.progress.push("done".into());
    set_run_status(conn, &report.run_id, "done")?;
    Ok(())
}

struct CellOutcome {
    cost: f64,
    usage: Usage,
}

fn execute_cell(
    plan: &Plan,
    provider: &dyn ChatProvider,
    cell: &CellSpec,
    sandbox: &Path,
    dir: &Path,
    clock: &Clock,
    suite: &Suite,
) -> Result<CellOutcome> {
    let skilled = cell.configuration == "with_skill";
    let marker = fs::read_to_string(plan.skill_dir.join("SKILL.md")).unwrap_or_default();
    let system = if skilled { Some(marker) } else { None };
    let secret = plan.secret.clone();
    let mut transcript = Transcript::new(dir.join("transcript.jsonl"), &secret);
    let tools = sandbox_tools();
    transcript.push(clock, "user", None, false, &cell.prompt, &secret)?;
    let mut messages = vec![Message {
        role: "user".into(),
        content: vec![ContentBlock::Text(cell.prompt.clone())],
    }];
    let mut usage = Usage::zero();
    let mut tool_counts =
        json!({"Read": 0, "Write": 0, "Edit": 0, "Glob": 0, "Grep": 0, "Bash": 0});
    let before = list_files(sandbox);
    for _turn in 0..6 {
        let request = ChatRequest {
            model: plan.model.clone(),
            system: system.clone(),
            messages: messages.clone(),
            tools: tools.clone(),
            max_tokens: 256,
            effort: None,
            thinking: ThinkingMode::Off,
            stream: true,
            metadata: RequestMetadata {
                purpose: "executor".into(),
                ref_id: Some(cell_id("cell", cell)),
                provider_id: None,
            },
        };
        let response = call_provider(plan, provider, &request, &mut transcript, clock)?;
        add_usage(&mut usage, &response.usage);
        if matches!(response.stop, StopReason::Refusal { .. }) {
            transcript.push(clock, "error", None, true, &response.text, &secret)?;
            break;
        }
        if response.tool_uses.is_empty() || response.tool_uses.iter().any(|tool| tool.incomplete) {
            if response.incomplete_tool {
                transcript.push(
                    clock,
                    "error",
                    Some("tool"),
                    true,
                    "incomplete tool input",
                    &secret,
                )?;
            }
            transcript.push(
                clock,
                "assistant_text",
                None,
                false,
                &response.text,
                &secret,
            )?;
            break;
        }
        let mut results = Vec::new();
        for tool in &response.tool_uses {
            let (mapped, body, is_error) = run_tool(sandbox, tool);
            let current = tool_counts
                .get(&mapped)
                .and_then(Value::as_u64)
                .unwrap_or(0);
            if let Some(object) = tool_counts.as_object_mut() {
                object.insert(mapped.clone(), json!(current + 1));
            }
            transcript.push(
                clock,
                "tool_use",
                Some(&mapped),
                is_error,
                &tool.raw_input,
                &secret,
            )?;
            transcript.push(
                clock,
                "tool_result",
                Some(&mapped),
                is_error,
                &body,
                &secret,
            )?;
            results.push(ContentBlock::ToolResult {
                tool_use_id: tool.id.clone(),
                content: body,
                is_error,
            });
        }
        messages.push(Message {
            role: "assistant".into(),
            content: response
                .tool_uses
                .iter()
                .map(|tool| ContentBlock::ToolUse {
                    id: tool.id.clone(),
                    name: tool.name.clone(),
                    input: tool.input.clone(),
                })
                .collect(),
        });
        messages.push(Message {
            role: "user".into(),
            content: results,
        });
    }
    copy_outputs(sandbox, &dir.join("outputs"))?;
    let after = list_files(sandbox);
    let created: Vec<String> = after
        .into_iter()
        .filter(|path| !before.contains(path))
        .collect();
    let total_tools: u64 = ["Read", "Write", "Edit", "Glob", "Grep", "Bash"]
        .iter()
        .map(|name| tool_counts.get(*name).and_then(Value::as_u64).unwrap_or(0))
        .sum();
    let metrics = json!({
        "tool_calls": {"Read": count_of(&tool_counts, "Read"), "Write": count_of(&tool_counts, "Write"), "Bash": count_of(&tool_counts, "Bash"), "Edit": count_of(&tool_counts, "Edit"), "Glob": count_of(&tool_counts, "Glob"), "Grep": count_of(&tool_counts, "Grep")},
        "total_tool_calls": total_tools,
        "total_steps": 2,
        "files_created": created,
        "errors_encountered": 0,
        "output_chars": fs::read_to_string(dir.join("outputs").join("answer.txt")).map(|text| text.len()).unwrap_or(0),
        "transcript_chars": fs::read_to_string(dir.join("transcript.jsonl")).map(|text| text.len()).unwrap_or(0)
    });
    fs::write(
        dir.join("outputs").join("metrics.json"),
        serde_json::to_string_pretty(&metrics)?,
    )?;
    let duration_ms = usage.total().max(1);
    let timing = json!({
        "total_tokens": usage.total(),
        "duration_ms": duration_ms,
        "total_duration_seconds": (duration_ms as f64) / 1000.0,
        "executor_start": "2026-01-15T10:30:00Z",
        "executor_end": "2026-01-15T10:30:01Z",
        "executor_duration_seconds": (duration_ms as f64) / 1000.0
    });
    fs::write(
        dir.join("timing.json"),
        serde_json::to_string_pretty(&timing)?,
    )?;
    fs::write(
        dir.join("env.json"),
        "{\n  \"PATH\": \"<redacted>\",\n  \"HOME\": \"<redacted>\",\n  \"CLAUDECODE\": null\n}\n",
    )?;
    fs::write(
        dir.join("command.txt"),
        format!("api {} model {}\n", plan.kind, plan.model),
    )?;
    fs::write(dir.join("stdout.log"), "")?;
    fs::write(dir.join("stderr.log"), "")?;
    transcript.render_markdown(&dir.join("transcript.md"))?;
    let _ = (sandbox, suite);
    Ok(CellOutcome {
        cost: cost_of(plan, &usage),
        usage,
    })
}

struct GradeInput<'a> {
    plan: &'a Plan,
    provider: &'a dyn ChatProvider,
    cell: &'a CellSpec,
    dir: &'a Path,
    suite: &'a Suite,
    transcript: &'a str,
    metrics: &'a Value,
    timing: &'a Value,
    secret: &'a str,
}

fn grade_cell(input: &GradeInput<'_>) -> Result<(Value, f64)> {
    let GradeInput {
        plan,
        provider,
        cell,
        dir,
        suite,
        transcript,
        metrics,
        timing,
        secret,
    } = *input;
    let mut expectations = Vec::new();
    let mut check_rows = Vec::new();
    let tool_total = json!({"total": metrics.get("total_tool_calls").cloned().unwrap_or(json!(0))});
    for (index, text) in cell.expectations.iter().enumerate() {
        if let Some(check) = suite.checks.iter().find(|check| &check.expectation == text) {
            let result = run_check(&to_check(check), dir, transcript, &tool_total);
            check_rows.push(json!({"id": result.id, "passed": result.passed, "stdout": result.stdout, "stderr": result.stderr}));
            expectations
                .push(json!({"text": text, "passed": result.passed, "evidence": result.stdout}));
            let _ = index;
        } else {
            let prompt = format!("GRADE\nexpectation: {text}\ntranscript:\n{transcript}");
            let request = ChatRequest {
                model: plan.model.clone(),
                system: Some("You are a grader. Reply with JSON.".into()),
                messages: vec![Message {
                    role: "user".into(),
                    content: vec![ContentBlock::Text(scrub(&prompt, &[secret.to_string()]))],
                }],
                tools: Vec::new(),
                max_tokens: 400,
                effort: None,
                thinking: ThinkingMode::Off,
                stream: false,
                metadata: RequestMetadata {
                    purpose: "grader".into(),
                    ref_id: None,
                    provider_id: None,
                },
            };
            let response = call_provider(
                plan,
                provider,
                &request,
                &mut Transcript::new(dir.join("grader.jsonl"), secret),
                &Clock::new(plan.seed),
            )?;
            let parsed: Value = serde_json::from_str(&response.text).unwrap_or(Value::Null);
            let row = parsed
                .get("expectations")
                .and_then(Value::as_array)
                .and_then(|rows| rows.first())
                .cloned()
                .unwrap_or(
                    json!({"text": text, "passed": false, "evidence": "grader reply was not JSON"}),
                );
            expectations.push(json!({"text": row.get("text").and_then(Value::as_str).unwrap_or(text), "passed": row.get("passed").and_then(Value::as_bool).unwrap_or(false), "evidence": row.get("evidence").and_then(Value::as_str).unwrap_or("")}));
        }
    }
    fs::write(
        dir.join("checks.json"),
        serde_json::to_string_pretty(&json!({"checks": check_rows}))?,
    )?;
    let passed = expectations
        .iter()
        .filter(|row| row.get("passed").and_then(Value::as_bool).unwrap_or(false))
        .count();
    let total = expectations.len().max(1);
    let pass_rate = passed as f64 / total as f64;
    let grading = json!({
        "expectations": expectations,
        "summary": {"passed": passed, "failed": total - passed, "total": total, "pass_rate": pass_rate},
        "execution_metrics": metrics,
        "timing": {"executor_duration_seconds": timing.get("executor_duration_seconds").and_then(Value::as_f64).unwrap_or(0.0), "grader_duration_seconds": 0.0, "total_duration_seconds": timing.get("total_duration_seconds").and_then(Value::as_f64).unwrap_or(0.0)},
        "claims": [],
        "user_notes_summary": {"uncertainties": [], "needs_review": [], "workarounds": []},
        "eval_feedback": {"suggestions": [], "overall": ""}
    });
    Ok((grading, pass_rate))
}

fn call_provider(
    plan: &Plan,
    provider: &dyn ChatProvider,
    request: &ChatRequest,
    transcript: &mut Transcript,
    clock: &Clock,
) -> Result<ChatResponse> {
    let middleware = Middleware {
        mode: plan.mode,
        recordings_dir: plan.recordings.clone(),
        kind: plan.kind.clone(),
        max_retries: 2,
        limiter: Limiter::new(4),
        sleeper: Box::new(NoopSleeper::new()),
        secrets: vec![plan.secret.clone()],
        pricing: Pricing {
            input_per_mtok: 4.0,
            output_per_mtok: 20.0,
            cache_read_per_mtok: 0.4,
            cache_write_per_mtok: 5.0,
        },
        provider_id: "provider".into(),
        hits: std::sync::Mutex::new(0),
    };
    let mut sink = VecSink::default();
    let cancel = CancellationToken::new();
    let (response, _row) = middleware
        .complete(provider, request, &mut sink, &cancel)
        .map_err(|err| anyhow!(err.message))?;
    transcript.push(
        clock,
        "usage",
        None,
        false,
        &format!(
            "{} {}",
            response.usage.input_tokens, response.usage.output_tokens
        ),
        &plan.secret,
    )?;
    Ok(response)
}

fn run_tool(sandbox: &Path, tool: &ToolUse) -> (String, String, bool) {
    let path = tool.input.get("path").and_then(Value::as_str).unwrap_or("");
    match tool.name.as_str() {
        "write_file" => match inside(sandbox, path) {
            Ok(target) => {
                if let Some(parent) = target.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let content = tool
                    .input
                    .get("content")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                match fs::write(&target, content) {
                    Ok(()) => ("Write".into(), format!("wrote {path}"), false),
                    Err(err) => ("Write".into(), err.to_string(), true),
                }
            }
            Err(err) => ("Write".into(), err, true),
        },
        "read_file" => match inside(sandbox, path) {
            Ok(target) => match fs::read_to_string(&target) {
                Ok(text) => ("Read".into(), text, false),
                Err(err) => ("Read".into(), err.to_string(), true),
            },
            Err(err) => ("Read".into(), err, true),
        },
        "bash" => {
            let command = tool
                .input
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or("");
            if command.contains("..") {
                return ("Bash".into(), "path refused".into(), true);
            }
            let output = std::process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .current_dir(sandbox)
                .output();
            match output {
                Ok(output) => (
                    "Bash".into(),
                    String::from_utf8_lossy(&output.stdout).into_owned(),
                    !output.status.success(),
                ),
                Err(err) => ("Bash".into(), err.to_string(), true),
            }
        }
        "list_dir" => ("Glob".into(), list_files(sandbox).join("\n"), false),
        other => (other.into(), "unsupported tool".into(), true),
    }
}

fn inside(sandbox: &Path, relative: &str) -> Result<PathBuf, String> {
    if relative.is_empty() || relative.contains('\0') {
        return Err("path refused".into());
    }
    let target = sandbox.join(relative);
    if !is_path_safe(sandbox, &target) {
        return Err("path refused".into());
    }
    Ok(target)
}

fn inject_skill(
    skill: &Path,
    sandbox: &Path,
    name: &str,
    configuration: &str,
    alias: Option<&str>,
) -> Result<()> {
    let skills = sandbox.join(".claude").join("skills");
    if configuration == "without_skill" {
        fs::create_dir_all(&skills)?;
        return Ok(());
    }
    let folder = alias.unwrap_or(name);
    copy_dir(skill, &skills.join(folder))?;
    Ok(())
}

fn collision_alias(name: &str, run_id: &str, personal: Option<&Path>) -> Option<String> {
    let personal = personal?;
    if personal.join(name).exists() {
        Some(format!("{name}-studio-{run_id}"))
    } else {
        None
    }
}

struct Transcript {
    path: PathBuf,
    offset: u64,
}

impl Transcript {
    fn new(path: PathBuf, _secret: &str) -> Self {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::File::create(&path);
        Self { path, offset: 0 }
    }

    fn push(
        &mut self,
        clock: &Clock,
        kind: &str,
        tool: Option<&str>,
        is_error: bool,
        payload: &str,
        secret: &str,
    ) -> Result<()> {
        let record = json!({
            "seq": self.offset,
            "ts": clock.now(),
            "kind": kind,
            "tool_name": tool,
            "is_error": is_error,
            "payload": scrub(payload, &[secret.to_string()])
        });
        let line = format!("{record}\n");
        use std::io::Write;
        let mut file = fs::OpenOptions::new().append(true).open(&self.path)?;
        file.write_all(line.as_bytes())?;
        file.flush()?;
        self.offset += 1;
        Ok(())
    }

    fn render_markdown(&self, path: &Path) -> Result<()> {
        let raw = fs::read_to_string(&self.path).unwrap_or_default();
        let mut md = String::from("# Transcript\n\n");
        for line in raw.lines() {
            let value: Value = serde_json::from_str(line).unwrap_or(Value::Null);
            let kind = value.get("kind").and_then(Value::as_str).unwrap_or("");
            let payload = value.get("payload").and_then(Value::as_str).unwrap_or("");
            let shown = if payload.lines().count() > 40 {
                format!(
                    "{}\n…",
                    payload.lines().take(40).collect::<Vec<_>>().join("\n")
                )
            } else {
                payload.to_string()
            };
            md.push_str(&format!("## {kind}\n\n{shown}\n\n"));
        }
        fs::write(path, md)?;
        Ok(())
    }
}

fn sandbox_tools() -> Vec<ToolDef> {
    vec![ToolDef {
        name: "write_file".into(),
        description: "Write a file inside the sandbox".into(),
        input_schema: json!({"type": "object", "properties": {"path": {"type": "string"}, "content": {"type": "string"}}}),
    }]
}

fn copy_outputs(sandbox: &Path, outputs: &Path) -> Result<()> {
    let produced = sandbox.join("outputs");
    if produced.is_dir() {
        copy_dir(&produced, outputs)?;
    }
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from).with_context(|| format!("read {}", from.display()))? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn list_files(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(root, &path, out);
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

fn add_usage(total: &mut Usage, extra: &Usage) {
    total.input_tokens += extra.input_tokens;
    total.output_tokens += extra.output_tokens;
    total.cache_read_input_tokens += extra.cache_read_input_tokens;
    total.cache_creation_input_tokens += extra.cache_creation_input_tokens;
}

fn cost_of(plan: &Plan, usage: &Usage) -> f64 {
    if let Some(fixed) = plan.usd_per_cell {
        return fixed;
    }
    (usage.input_tokens as f64 * 4.0
        + usage.output_tokens as f64 * 20.0
        + usage.cache_read_input_tokens as f64 * 0.4
        + usage.cache_creation_input_tokens as f64 * 5.0)
        / 1_000_000.0
}

fn count_of(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn to_check(check: &CheckSpec) -> DeterministicCheck {
    DeterministicCheck {
        id: check.id.clone(),
        expectation: check.expectation.clone(),
        kind: check.kind.clone(),
        path: check.path.clone(),
        pattern: check.pattern.clone(),
        minimum: None,
    }
}

fn write_pending_grade(dir: &Path) -> Result<()> {
    fs::write(dir.join("grading.json"), "{}\n")?;
    fs::write(dir.join("checks.json"), "{\"checks\":[]}\n")?;
    Ok(())
}

fn expand(suite: &Suite, repetitions: u32) -> Vec<CellSpec> {
    let mut cells = Vec::new();
    for case in &suite.evals {
        for run_number in 1..=repetitions {
            for configuration in ["with_skill", "without_skill"] {
                cells.push(CellSpec {
                    eval_id: case.id,
                    name: format!("eval-{}", case.id),
                    prompt: case.prompt.clone(),
                    configuration: configuration.into(),
                    run_number,
                    expectations: case.expectations.clone(),
                });
            }
        }
    }
    cells
}

fn order_cells(cells: &mut [CellSpec], seed: u64) {
    cells.sort_by_key(|cell| {
        seed ^ (cell.eval_id as u64).wrapping_mul(0x9E37_79B9)
            ^ u64::from(cell.run_number) << 8
            ^ if cell.configuration == "with_skill" {
                1
            } else {
                2
            }
    });
}

fn cell_id(run_id: &str, cell: &CellSpec) -> String {
    format!(
        "{run_id}-{}-{}-r{}",
        cell.eval_id, cell.configuration, cell.run_number
    )
}

fn cell_dir(iteration: &Path, cell: &CellSpec) -> PathBuf {
    let config = if cell.run_number == 1 {
        cell.configuration.clone()
    } else {
        format!("{}-r{}", cell.configuration, cell.run_number)
    };
    iteration.join(&cell.name).join(config)
}

fn cell_report(
    run_id: &str,
    cell: &CellSpec,
    status: &str,
    pass_rate: f64,
    dir: &Path,
) -> CellReport {
    CellReport {
        id: cell_id(run_id, cell),
        configuration: cell.configuration.clone(),
        run_number: cell.run_number,
        status: status.into(),
        dir: dir.to_path_buf(),
        pass_rate,
    }
}

fn trigger_path(plan: &Plan) -> PathBuf {
    plan.skill_dir.join("evals").join("trigger_streams.json")
}

fn open_db(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    crate::db::migrations::run_migrations(&conn)?;
    Ok(conn)
}

fn ensure_skill(conn: &Connection, name: &str, dir: &Path) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO skills (id, name, source_type, central_path, created_at, updated_at) VALUES (?1, ?2, 'local', ?3, 0, 0)",
        (name, name, dir.to_string_lossy().as_ref()),
    )?;
    Ok(())
}

fn persist_suite(conn: &Connection, suite: &Suite, run_id: &str, plan: &Plan) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO test_suites (id, name, kind, skill_id, source, baseline, workspace_dir, created_at, updated_at) VALUES (?1, ?2, 'task_eval', ?2, 'evals_json', 'without_skill', ?3, 0, 0)",
        (suite.skill_name.as_str(), suite.skill_name.as_str(), plan.workspace.to_string_lossy().as_ref()),
    )?;
    for case in &suite.evals {
        conn.execute(
            "INSERT OR IGNORE INTO test_cases (id, suite_id, eval_id, name, prompt, expected_output, expectations_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 0)",
            (
                format!("{}-{}", suite.skill_name, case.id),
                suite.skill_name.as_str(),
                case.id,
                format!("eval-{}", case.id),
                case.prompt.as_str(),
                case.expected_output.as_str(),
                serde_json::to_string(&case.expectations)?,
            ),
        )?;
    }
    conn.execute(
        "INSERT OR REPLACE INTO runs (id, suite_id, iteration, configurations_json, runs_per_case, mode, seed, budget_usd, status, workspace_dir, queued_at) VALUES (?1, ?2, 1, ?3, ?4, ?5, ?6, ?7, 'running', ?8, 0)",
        (
            run_id,
            suite.skill_name.as_str(),
            "[\"with_skill\",\"without_skill\"]",
            plan.repetitions,
            mode_name(plan.mode),
            plan.seed as i64,
            plan.budget_usd,
            plan.workspace.to_string_lossy().as_ref(),
        ),
    )?;
    Ok(())
}

fn save_case(
    conn: &Connection,
    run_id: &str,
    skill: &str,
    cell: &CellSpec,
    status: &str,
    dir: &Path,
) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO run_cases (id, run_id, test_case_id, configuration, run_number, status, outputs_dir, transcript_path) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (
            cell_id(run_id, cell),
            run_id,
            format!("{skill}-{}", cell.eval_id),
            cell.configuration.as_str(),
            cell.run_number,
            status,
            dir.to_string_lossy().as_ref(),
            dir.join("transcript.jsonl").to_string_lossy().as_ref(),
        ),
    )?;
    Ok(())
}

fn mark_cancelled(conn: &Connection, run_id: &str, skill: &str, cell: &CellSpec) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO run_cases (id, run_id, test_case_id, configuration, run_number, status) VALUES (?1, ?2, ?3, ?4, ?5, 'cancelled')",
        (cell_id(run_id, cell), run_id, format!("{skill}-{}", cell.eval_id), cell.configuration.as_str(), cell.run_number),
    )?;
    Ok(())
}

fn set_run_status(conn: &Connection, run_id: &str, status: &str) -> Result<()> {
    conn.execute(
        "UPDATE runs SET status = ?2 WHERE id = ?1",
        (run_id, status),
    )?;
    Ok(())
}

fn completed_ids(conn: &Connection, run_id: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT id FROM run_cases WHERE run_id = ?1 AND status IN ('graded', 'executed')",
    )?;
    let rows = stmt.query_map((run_id,), |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn spent_usd(conn: &Connection, run_id: &str) -> Result<f64> {
    let value: Option<f64> = conn
        .query_row(
            "SELECT SUM(cost_usd) FROM usage_ledger WHERE ref_id = ?1",
            (run_id,),
            |row| row.get(0),
        )
        .ok();
    Ok(value.unwrap_or(0.0))
}

fn write_ledger(
    conn: &Connection,
    plan: &Plan,
    cell: &CellSpec,
    cost: f64,
    usage: &Usage,
) -> Result<()> {
    let status = if plan.mode == RunMode::Replay {
        "replayed"
    } else {
        "ok"
    };
    let cost = if plan.mode == RunMode::Replay {
        0.0
    } else {
        cost
    };
    conn.execute(
        "INSERT INTO usage_ledger (ts, model, purpose, ref_kind, ref_id, input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, cost_usd, status) VALUES (0, ?1, 'executor', 'run_case', ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (
            plan.model.as_str(),
            format!("run-{:016x}", plan.seed),
            usage.input_tokens as i64,
            usage.output_tokens as i64,
            usage.cache_read_input_tokens as i64,
            usage.cache_creation_input_tokens as i64,
            cost,
            status,
        ),
    )?;
    let _ = cell;
    Ok(())
}

fn mode_name(mode: RunMode) -> &'static str {
    match mode {
        RunMode::Live => "live",
        RunMode::Record => "record",
        RunMode::Replay => "replay",
    }
}

#[allow(dead_code)]
fn _timeout() -> Duration {
    Duration::from_secs(30)
}
