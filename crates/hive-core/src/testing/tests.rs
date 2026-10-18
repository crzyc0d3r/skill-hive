//! Engine acceptance tests. Replay and the local HTTP fixture never call a vendor API.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;

use crate::providers::{
    AnthropicProvider, CancellationToken, ChatProvider, ChatRequest, ChatResponse, ContentBlock,
    ErrorClass, ModelInfo, ProviderError, RunMode, ScriptedProvider, SecretString, StopReason,
    StreamEvent, StreamSink, ToolUse, Usage,
};

use super::*;

fn suite() -> Suite {
    Suite {
        skill_name: "fixture-skill".into(),
        evals: vec![SuiteCase {
            id: 1,
            prompt: "Write the fixture answer. token sk-ant-endpoint-fixture-secret".into(),
            expected_output: "PINECONE".into(),
            files: Vec::new(),
            expectations: vec![
                "answer file exists".into(),
                "The answer file contains PINECONE".into(),
            ],
        }],
        triggers: Vec::new(),
        checks: vec![super::suite::CheckSpec {
            id: "file".into(),
            expectation: "answer file exists".into(),
            kind: "file_exists".into(),
            path: "outputs/answer.txt".into(),
            pattern: None,
        }],
    }
}

fn write_skill(dir: &Path) {
    fs::create_dir_all(dir.join("evals")).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nname: fixture-skill\n---\nHIVE_SKILL_MARKER_9f3a\n",
    )
    .unwrap();
    save_suite(dir, &suite()).unwrap();
    fs::write(
        dir.join("evals").join("trigger_streams.json"),
        fs::read_to_string(queries_path()).unwrap(),
    )
    .unwrap();
}

fn queries_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/testing/trigger_queries.json")
}

fn plan(root: &Path, mode: RunMode) -> Plan {
    Plan {
        skill_dir: root.join("skill"),
        workspace: root.join("ws"),
        db_path: root.join("hive.db"),
        mode,
        seed: 7,
        repetitions: 3,
        budget_usd: None,
        cancel_after: None,
        crash_after: None,
        recordings: root.join("recordings"),
        personal_skills: None,
        secret: "sk-ant-endpoint-fixture-secret".into(),
        usd_per_cell: None,
        model: "fixture-model".into(),
        kind: "anthropic".into(),
    }
}

fn files_under(root: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(root, &path, out);
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push((
                rel.to_string_lossy().replace('\\', "/"),
                fs::read_to_string(&path).unwrap_or_default(),
            ));
        }
    }
}

struct Bomb {
    hits: AtomicUsize,
}

impl ChatProvider for Bomb {
    fn id(&self) -> &str {
        "bomb"
    }
    fn kind_name(&self) -> &'static str {
        "bomb"
    }
    fn capabilities(&self) -> crate::providers::Capabilities {
        crate::providers::Capabilities {
            tools: true,
            streaming: true,
            system_prompt: true,
        }
    }
    fn complete(
        &self,
        _: &ChatRequest,
        _: &mut dyn StreamSink,
        _: &CancellationToken,
    ) -> Result<ChatResponse, ProviderError> {
        self.hits.fetch_add(1, Ordering::SeqCst);
        Err(ProviderError::new(ErrorClass::Other, "bomb"))
    }
    fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(Vec::new())
    }
    fn count_tokens(&self, _: &ChatRequest) -> Result<Option<u64>, ProviderError> {
        Ok(None)
    }
}

struct Escape;

impl ChatProvider for Escape {
    fn id(&self) -> &str {
        "escape"
    }
    fn kind_name(&self) -> &'static str {
        "escape"
    }
    fn capabilities(&self) -> crate::providers::Capabilities {
        crate::providers::Capabilities {
            tools: true,
            streaming: false,
            system_prompt: true,
        }
    }
    fn complete(
        &self,
        request: &ChatRequest,
        sink: &mut dyn StreamSink,
        _: &CancellationToken,
    ) -> Result<ChatResponse, ProviderError> {
        let has_result = request.messages.iter().any(|message| {
            message
                .content
                .iter()
                .any(|block| matches!(block, ContentBlock::ToolResult { .. }))
        });
        let usage = Usage {
            input_tokens: 2,
            output_tokens: 1,
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
        };
        if has_result {
            sink.on_event(&StreamEvent::TextDelta("done".into()));
            return Ok(ChatResponse {
                id: "e".into(),
                stop: StopReason::EndTurn,
                text: "done".into(),
                tool_uses: Vec::new(),
                usage,
                incomplete_tool: false,
            });
        }
        let input = serde_json::json!({"path": "../escape.txt", "content": "nope"});
        sink.on_event(&StreamEvent::MessageEnd {
            stop_reason: StopReason::ToolUse,
        });
        Ok(ChatResponse {
            id: "e".into(),
            stop: StopReason::ToolUse,
            text: String::new(),
            tool_uses: vec![ToolUse {
                id: "t".into(),
                name: "write_file".into(),
                input,
                raw_input: r#"{"path":"../escape.txt"}"#.into(),
                incomplete: false,
            }],
            usage,
            incomplete_tool: false,
        })
    }
    fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(Vec::new())
    }
    fn count_tokens(&self, _: &ChatRequest) -> Result<Option<u64>, ProviderError> {
        Ok(None)
    }
}

#[test]
fn suite_round_trip_rejects_empty_names_and_duplicate_ids() {
    let dir = tempfile::tempdir().unwrap();
    let skill = dir.path().join("skill");
    write_skill(&skill);
    let loaded = load_suite(&skill).unwrap();
    assert_eq!(loaded, suite());
    let raw = fs::read_to_string(skill.join("evals").join("evals.json")).unwrap();
    let parsed: Value = serde_json::from_str(&raw).unwrap();
    assert!(parsed.get("skill_name").is_some());
    assert!(parsed["evals"][0].get("id").is_some());
    assert!(parsed["evals"][0].get("files").is_some());
    fix_skill_name(&skill, "renamed-skill").unwrap();
    assert_eq!(load_suite(&skill).unwrap().skill_name, "renamed-skill");
    let mut broken = suite();
    broken.skill_name.clear();
    assert!(save_suite(dir.path(), &broken).is_err());
    broken.skill_name = "fixture-skill".into();
    broken.evals.push(broken.evals[0].clone());
    assert!(save_suite(dir.path(), &broken).is_err());
}

#[test]
fn fixture_run_writes_the_skill_creator_layout_live_and_replay() {
    let root = tempfile::tempdir().unwrap();
    write_skill(&root.path().join("skill"));
    let mut recorded = plan(root.path(), RunMode::Record);
    recorded.workspace = root.path().join("record");
    let scripted = ScriptedProvider::new("scripted");
    let live = run_plan(&recorded, &scripted).unwrap();
    assert_eq!(live.status, "done");
    let cell = root
        .path()
        .join("record")
        .join("iteration-1")
        .join("eval-1")
        .join("with_skill");
    for name in [
        "transcript.jsonl",
        "transcript.md",
        "timing.json",
        "grading.json",
        "checks.json",
        "env.json",
        "command.txt",
        "stdout.log",
        "stderr.log",
    ] {
        assert!(cell.join(name).is_file(), "{name}");
    }
    assert!(cell.join("outputs").join("metrics.json").is_file());
    let timing: Value =
        serde_json::from_str(&fs::read_to_string(cell.join("timing.json")).unwrap()).unwrap();
    assert!(timing["total_tokens"].as_u64().unwrap_or(0) > 0);
    assert!(timing["duration_ms"].as_u64().unwrap_or(0) > 0);
    let grading: Value =
        serde_json::from_str(&fs::read_to_string(cell.join("grading.json")).unwrap()).unwrap();
    let expectations = grading["expectations"].as_array().unwrap();
    assert!(expectations.iter().all(|row| row.get("text").is_some()
        && row.get("passed").is_some()
        && row.get("evidence").is_some()));
    assert_ne!(
        grading["execution_metrics"]["total_tool_calls"].as_u64(),
        Some(999)
    );
    let transcript = fs::read_to_string(cell.join("transcript.jsonl")).unwrap();
    assert!(!transcript.contains("sk-ant-endpoint-fixture-secret"));
    assert!(transcript.contains("<token>") || transcript.contains("<redacted>"));
    assert!(!transcript.contains("HIVE_SKILL_MARKER_9f3a"));
    let plain = fs::read_to_string(
        root.path()
            .join("record")
            .join("iteration-1")
            .join("eval-1")
            .join("without_skill")
            .join("transcript.jsonl"),
    )
    .unwrap();
    assert!(!plain.contains("HIVE_SKILL_MARKER_9f3a"));
    let listings = files_under(&root.path().join("record").join("iteration-1"))
        .into_iter()
        .filter(|(path, _)| path.ends_with("sandbox-listing.txt"))
        .map(|(_, body)| body)
        .collect::<Vec<_>>();
    assert!(
        listings.iter().any(|body| body.trim().is_empty()),
        "without_skill skills dir was not empty: {listings:?}"
    );
    let sandboxes = root.path().join("record").join("sandboxes");
    assert!(
        fs::read_dir(&sandboxes)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(true),
        "sandbox directories were kept after the cells"
    );
    assert!(root
        .path()
        .join("record")
        .join("iteration-1")
        .join("benchmark.json")
        .is_file());
    assert!(root
        .path()
        .join("record")
        .join("iteration-1")
        .join("benchmark.md")
        .is_file());
    assert!(root
        .path()
        .join("record")
        .join("trigger")
        .join(&live.run_id)
        .join("results.json")
        .is_file());
    let prompts = scripted.prompts.lock().unwrap().clone();
    assert!(prompts
        .iter()
        .all(|prompt| !prompt.contains("answer file exists")));
    assert!(prompts.iter().any(|prompt| prompt.contains("PINECONE")));

    let bomb = Bomb {
        hits: AtomicUsize::new(0),
    };
    let mut replay = recorded.clone();
    replay.mode = RunMode::Replay;
    replay.workspace = root.path().join("replay-a");
    replay.db_path = root.path().join("replay-a.db");
    let first = run_plan(&replay, &bomb).unwrap();
    assert_eq!(first.status, "done");
    assert_eq!(bomb.hits.load(Ordering::SeqCst), 0);
    let mut second_plan = replay.clone();
    second_plan.workspace = root.path().join("replay-b");
    second_plan.db_path = root.path().join("replay-b.db");
    run_plan(&second_plan, &bomb).unwrap();
    assert_eq!(bomb.hits.load(Ordering::SeqCst), 0);
    let left = files_under(&root.path().join("replay-a").join("iteration-1"));
    let right = files_under(&root.path().join("replay-b").join("iteration-1"));
    assert_eq!(left, right);
    let env = fs::read_to_string(
        root.path()
            .join("replay-a")
            .join("iteration-1")
            .join("eval-1")
            .join("with_skill")
            .join("env.json"),
    )
    .unwrap();
    assert!(!env.contains(root.path().to_string_lossy().as_ref()));
}

#[test]
fn trigger_verdicts_agree_with_python_across_three_runs() {
    let path = queries_path();
    for _ in 0..3 {
        let (agree, total) = agree_with_python(&path, "fixture-skill", 0.5).unwrap();
        assert!(total >= 20, "{total}");
        assert!(agree >= 18, "{agree}/{total}");
    }
    let dir = tempfile::tempdir().unwrap();
    let removed = cleanup_command_file(dir.path(), "fixture-skill", "timeout").unwrap();
    assert!(!removed.exists());
    let cancelled = cleanup_command_file(dir.path(), "fixture-skill", "cancel").unwrap();
    assert!(!cancelled.exists());
}

#[test]
fn budget_pause_cancel_and_crash_resume() {
    let root = tempfile::tempdir().unwrap();
    write_skill(&root.path().join("skill"));
    let scripted = ScriptedProvider::new("budget");
    let mut paused = plan(root.path(), RunMode::Live);
    paused.repetitions = 1;
    paused.budget_usd = Some(1.0);
    paused.usd_per_cell = Some(1.0);
    paused.workspace = root.path().join("budget");
    paused.db_path = root.path().join("budget.db");
    let report = run_plan(&paused, &scripted).unwrap();
    assert_eq!(report.status, "paused");
    assert!(report.paused);
    assert_eq!(report.cells.len(), 1);
    assert_eq!(report.cells[0].status, "executed");
    let grading = fs::read_to_string(report.cells[0].dir.join("grading.json")).unwrap();
    assert_eq!(grading, "{}\n");
    assert!(scripted.prompts.lock().unwrap().is_empty());

    let mut cancelled = plan(root.path(), RunMode::Live);
    cancelled.repetitions = 3;
    cancelled.cancel_after = Some(4);
    cancelled.workspace = root.path().join("cancel");
    cancelled.db_path = root.path().join("cancel.db");
    let mut extra = suite();
    extra.evals.push(SuiteCase {
        id: 2,
        prompt: "Second eval".into(),
        expected_output: "PINECONE".into(),
        files: Vec::new(),
        expectations: vec!["The answer file contains PINECONE".into()],
    });
    save_suite(&root.path().join("skill"), &extra).unwrap();
    let report = run_plan(&cancelled, &ScriptedProvider::new("cancel")).unwrap();
    assert_eq!(report.cells.len(), 12);
    assert_eq!(
        report
            .cells
            .iter()
            .filter(|cell| cell.status == "graded")
            .count(),
        4
    );
    assert_eq!(
        report
            .cells
            .iter()
            .filter(|cell| cell.status == "cancelled")
            .count(),
        8
    );
    assert_eq!(report.status, "cancelled");

    let mut crashed = plan(root.path(), RunMode::Live);
    crashed.repetitions = 1;
    crashed.crash_after = Some(1);
    crashed.workspace = root.path().join("crash");
    crashed.db_path = root.path().join("crash.db");
    save_suite(&root.path().join("skill"), &suite()).unwrap();
    let partial = run_plan(&crashed, &ScriptedProvider::new("crash")).unwrap();
    assert_eq!(partial.status, "running");
    assert_eq!(partial.cells.len(), 1);
    let resumed = resume_run(&crashed, &ScriptedProvider::new("resume")).unwrap();
    assert_eq!(resumed.status, "done");
    assert!(!resumed.cells.is_empty());
}

#[test]
fn sandbox_refuses_parent_paths_and_aliases_personal_collisions() {
    let root = tempfile::tempdir().unwrap();
    write_skill(&root.path().join("skill"));
    let mut spec = plan(root.path(), RunMode::Live);
    spec.repetitions = 1;
    spec.workspace = root.path().join("escape");
    spec.db_path = root.path().join("escape.db");
    let report = run_plan(&spec, &Escape).unwrap();
    assert!(!root.path().join("escape.txt").exists());
    let transcript = fs::read_to_string(report.cells[0].dir.join("transcript.jsonl")).unwrap();
    assert!(transcript.contains("path refused"));

    let personal = root.path().join("personal");
    fs::create_dir_all(personal.join("fixture-skill")).unwrap();
    let mut aliased = plan(root.path(), RunMode::Live);
    aliased.repetitions = 1;
    aliased.personal_skills = Some(personal);
    aliased.workspace = root.path().join("alias");
    aliased.db_path = root.path().join("alias.db");
    let report = run_plan(&aliased, &ScriptedProvider::new("alias")).unwrap();
    let alias = report.alias.expect("alias");
    assert!(alias.starts_with("fixture-skill-studio-"));
    let prepare = fs::read_to_string(root.path().join("alias").join("prepare.json")).unwrap();
    assert!(prepare.contains(&alias));
}

#[test]
fn malformed_grader_output_is_a_failed_expectation() {
    let root = tempfile::tempdir().unwrap();
    write_skill(&root.path().join("skill"));
    let mut broken = suite();
    broken.checks.clear();
    broken.evals[0].expectations = vec!["MALFORMED reply".into()];
    save_suite(&root.path().join("skill"), &broken).unwrap();
    let mut spec = plan(root.path(), RunMode::Live);
    spec.repetitions = 1;
    let report = run_plan(&spec, &ScriptedProvider::new("grader")).unwrap();
    let grading: Value = serde_json::from_str(
        &fs::read_to_string(report.cells[0].dir.join("grading.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(grading["expectations"][0]["passed"], false);
    assert_eq!(
        grading["expectations"][0]["evidence"],
        "grader reply was not JSON"
    );
}

#[test]
fn aggregate_matches_the_python_fixture() {
    let dir = tempfile::tempdir().unwrap();
    let iteration = dir.path().join("iteration-1");
    for (config, pass_rate, seconds, tokens) in
        [("with_skill", 1.0, 2.0, 10), ("without_skill", 0.0, 1.0, 4)]
    {
        let folder = iteration.join("eval-1").join(config);
        fs::create_dir_all(folder.join("outputs")).unwrap();
        fs::write(folder.join("grading.json"), format!(r#"{{"expectations":[],"summary":{{"pass_rate":{pass_rate},"passed":1,"failed":0,"total":1}}}}"#)).unwrap();
        fs::write(
            folder.join("timing.json"),
            format!(r#"{{"total_duration_seconds":{seconds},"total_tokens":{tokens}}}"#),
        )
        .unwrap();
        fs::write(
            folder.join("outputs").join("metrics.json"),
            r#"{"total_tool_calls":1,"errors_encountered":0}"#,
        )
        .unwrap();
    }
    let body = aggregate_benchmark(&iteration, "fixture-skill", "fixture-model").unwrap();
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/testing/aggregate_benchmark.py");
    let output = python_bin().arg(script).arg(&iteration).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let python: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        body["run_summary"]["delta"]["pass_rate"],
        python["delta"]["pass_rate"]
    );
    assert_eq!(
        body["run_summary"]["delta"]["time_seconds"],
        python["delta"]["time_seconds"]
    );
    assert_eq!(
        body["run_summary"]["delta"]["tokens"],
        python["delta"]["tokens"]
    );
    assert_eq!(body["runs"][0]["configuration"], "with_skill");
}

#[test]
fn local_http_endpoint_can_drive_one_live_cell() {
    let hits = std::sync::Arc::new(AtomicUsize::new(0));
    let seen = hits.clone();
    let server = std::thread::spawn(move || {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let port_flag = (port, flag.clone());
        std::thread::spawn(move || {
            while flag.load(Ordering::SeqCst) {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                if !flag.load(Ordering::SeqCst) {
                    break;
                }
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(500)));
                let mut buf = Vec::new();
                let mut tmp = [0_u8; 4096];
                loop {
                    match std::io::Read::read(&mut stream, &mut tmp) {
                        Ok(0) | Err(_) => break,
                        Ok(count) => {
                            buf.extend_from_slice(&tmp[..count]);
                            if buf.windows(4).any(|window| window == b"\r\n\r\n") {
                                break;
                            }
                        }
                    }
                }
                let header_end = buf
                    .windows(4)
                    .position(|window| window == b"\r\n\r\n")
                    .unwrap_or(buf.len());
                let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
                let need = head
                    .lines()
                    .find_map(|line| {
                        line.split_once(':').and_then(|(name, value)| {
                            if name.eq_ignore_ascii_case("content-length") {
                                value.trim().parse::<usize>().ok()
                            } else {
                                None
                            }
                        })
                    })
                    .unwrap_or(0);
                let mut body = if buf.len() > header_end + 4 {
                    buf[header_end + 4..].to_vec()
                } else {
                    Vec::new()
                };
                while body.len() < need {
                    match std::io::Read::read(&mut stream, &mut tmp) {
                        Ok(0) | Err(_) => break,
                        Ok(count) => body.extend_from_slice(&tmp[..count]),
                    }
                }
                let text = String::from_utf8_lossy(&body).to_string();
                seen.fetch_add(1, Ordering::SeqCst);
                let payload = http_body(&text);
                let packet = format!(
                    "HTTP/1.1 200 TEST\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                    payload.len()
                );
                let _ = std::io::Write::write_all(&mut stream, packet.as_bytes());
            }
        });
        port_flag
    });
    let (port, flag) = server.join().unwrap();
    let root = tempfile::tempdir().unwrap();
    write_skill(&root.path().join("skill"));
    let mut spec = plan(root.path(), RunMode::Live);
    spec.repetitions = 1;
    let provider = AnthropicProvider::new(
        "local",
        format!("http://127.0.0.1:{port}"),
        SecretString::new("sk-ant-endpoint-fixture-secret"),
    );
    let report = run_plan(&spec, &provider).unwrap();
    assert_eq!(report.status, "done");
    assert!(hits.load(Ordering::SeqCst) > 0);
    flag.store(false, Ordering::SeqCst);
    let _ = std::net::TcpStream::connect(format!("127.0.0.1:{port}"));
}

fn http_body(request: &str) -> String {
    if request.contains("GRADE") {
        let passed = request.contains("PINECONE");
        let body = format!(
            r#"{{"expectations":[{{"text":"The answer file contains PINECONE","passed":{passed},"evidence":"step 2"}}]}}"#
        );
        return sse_text(&body);
    }
    if request.contains("tool_result") {
        let text = if request.contains("HIVE_SKILL_MARKER_9f3a") {
            "Wrote PINECONE."
        } else {
            "plain"
        };
        return sse_text(text);
    }
    let content = if request.contains("HIVE_SKILL_MARKER_9f3a") {
        "PINECONE"
    } else {
        "plain"
    };
    sse_tool(content)
}

fn sse_text(text: &str) -> String {
    format!(
        "data: {{\"type\":\"message_start\",\"message\":{{\"id\":\"m\",\"usage\":{{\"input_tokens\":4}}}}}}\n\ndata: {{\"type\":\"content_block_delta\",\"delta\":{{\"type\":\"text_delta\",\"text\":{}}}}}\n\ndata: {{\"type\":\"message_delta\",\"delta\":{{\"stop_reason\":\"end_turn\"}},\"usage\":{{\"output_tokens\":2}}}}\n\ndata: {{\"type\":\"message_stop\"}}\n\n",
        serde_json::to_string(text).unwrap()
    )
}

struct SandboxReader {
    workspace: PathBuf,
    marker: PathBuf,
}

impl ChatProvider for SandboxReader {
    fn id(&self) -> &str {
        "sandbox-reader"
    }
    fn kind_name(&self) -> &'static str {
        "sandbox-reader"
    }
    fn capabilities(&self) -> crate::providers::Capabilities {
        crate::providers::Capabilities {
            tools: false,
            streaming: false,
            system_prompt: true,
        }
    }
    fn complete(
        &self,
        request: &ChatRequest,
        sink: &mut dyn StreamSink,
        _: &CancellationToken,
    ) -> Result<ChatResponse, ProviderError> {
        if request.metadata.purpose != "grader" {
            if let Ok(entries) = fs::read_dir(self.workspace.join("sandboxes")) {
                for entry in entries.flatten() {
                    let root = entry.path();
                    let case = fs::read_to_string(root.join("case-note.txt")).unwrap_or_default();
                    let old = fs::read_to_string(
                        root.join(".claude")
                            .join("skills")
                            .join("fixture-skill")
                            .join("OLD.txt"),
                    )
                    .unwrap_or_default();
                    let fixture = fs::read_to_string(root.join("fixture.dat")).unwrap_or_default();
                    let mut file = fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&self.marker)
                        .unwrap();
                    let _ = writeln!(
                        file,
                        "PATH:{}\nCASE:{case}OLD:{old}FIXTURE:{fixture}",
                        root.display()
                    );
                }
            }
        }
        let text = if request.metadata.purpose == "grader" {
            r#"{"expectations":[{"text":"The answer file contains PINECONE","passed":true,"evidence":"marker"}],"claims":[{"claim":"claim-from-provider","type":"process","verified":true,"evidence":"step 2"}],"eval_feedback":{"suggestions":[{"assertion":"The answer file contains PINECONE","reason":"eval-feedback-from-provider"}],"overall":"eval-feedback-from-provider"}}"#.to_string()
        } else {
            "done".into()
        };
        sink.on_event(&StreamEvent::TextDelta(text.clone()));
        Ok(ChatResponse {
            id: "reader".into(),
            stop: StopReason::EndTurn,
            text,
            tool_uses: Vec::new(),
            usage: Usage {
                input_tokens: 3,
                output_tokens: 1,
                cache_read_input_tokens: 0,
                cache_creation_input_tokens: 0,
            },
            incomplete_tool: false,
        })
    }
    fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(Vec::new())
    }
    fn count_tokens(&self, _: &ChatRequest) -> Result<Option<u64>, ProviderError> {
        Ok(None)
    }
}

#[test]
fn clock_argument_stamps_timing_and_progress_events() {
    let root = tempfile::tempdir().unwrap();
    write_skill(&root.path().join("skill"));
    let mut spec = plan(root.path(), RunMode::Live);
    spec.repetitions = 1;
    let clock = EngineClock::at(1_700_000_111);
    let report = run_plan_with_clock(&spec, &ScriptedProvider::new("clock"), &clock).unwrap();
    let timing: Value =
        serde_json::from_str(&fs::read_to_string(report.cells[0].dir.join("timing.json")).unwrap())
            .unwrap();
    assert_eq!(timing["executor_start"], clock.start_stamp());
    assert_eq!(timing["executor_end"], clock.end_stamp());
    let started = report
        .progress
        .iter()
        .position(|event| event == "started")
        .unwrap();
    let cell = report
        .progress
        .iter()
        .position(|event| event == "cell")
        .unwrap();
    let finished = report
        .progress
        .iter()
        .position(|event| event == "finished")
        .unwrap();
    assert!(started < cell && cell < finished);
}

#[test]
fn sandbox_copies_case_files_old_skill_and_deletes_the_directory() {
    let root = tempfile::tempdir().unwrap();
    let skill = root.path().join("skill");
    write_skill(&skill);
    let mut custom = suite();
    custom.evals[0].files = vec!["case-note.txt".into()];
    save_suite(&skill, &custom).unwrap();
    fs::write(
        skill.join("evals").join("files").join("case-note.txt"),
        "CASE-NOTE-TOKEN\n",
    )
    .unwrap();
    fs::create_dir_all(skill.join("skill-snapshot")).unwrap();
    fs::write(
        skill.join("skill-snapshot").join("OLD.txt"),
        "OLD-SKILL-TOKEN\n",
    )
    .unwrap();
    let set = skill.join("fixtures").join("default");
    fs::create_dir_all(&set).unwrap();
    fs::write(set.join("alpha.txt"), "FIXTURE-ALPHA\n").unwrap();
    fs::write(set.join("beta.txt"), "FIXTURE-BETA\n").unwrap();
    let mut spec = plan(root.path(), RunMode::Live);
    spec.repetitions = 1;
    spec.workspace = root.path().join("sandbox-ws");
    spec.db_path = root.path().join("sandbox.db");
    let marker = root.path().join("marker.txt");
    let reader = SandboxReader {
        workspace: spec.workspace.clone(),
        marker: marker.clone(),
    };
    let report = run_plan(&spec, &reader).unwrap();
    assert_eq!(report.status, "done");
    let seen = fs::read_to_string(&marker).unwrap();
    assert!(seen.contains("CASE-NOTE-TOKEN"), "{seen}");
    assert!(seen.contains("OLD-SKILL-TOKEN"), "{seen}");
    let chosen = select_fixture_file(&set, spec.seed).unwrap();
    let fixture = fs::read_to_string(&chosen).unwrap();
    assert!(seen.contains(fixture.trim()), "{seen}");
    for line in seen.lines() {
        if let Some(path) = line.strip_prefix("PATH:") {
            assert!(!Path::new(path).exists(), "{path}");
        }
    }
    let sandboxes = spec.workspace.join("sandboxes");
    assert!(fs::read_dir(&sandboxes)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(true));
}

#[test]
fn step_metrics_come_from_three_steps_and_one_error() {
    let root = tempfile::tempdir().unwrap();
    let transcript = "\
{\"kind\":\"user\",\"is_error\":false,\"payload\":\"one\"}\n\
{\"kind\":\"tool_use\",\"is_error\":false,\"payload\":\"two\"}\n\
{\"kind\":\"error\",\"is_error\":true,\"payload\":\"three\"}\n\
{\"kind\":\"usage\",\"is_error\":false,\"payload\":\"skip\"}\n";
    persist_step_metrics(root.path(), transcript).unwrap();
    let metrics: Value =
        serde_json::from_str(&fs::read_to_string(root.path().join("metrics.json")).unwrap())
            .unwrap();
    assert_eq!(metrics["total_steps"], 3);
    assert_eq!(metrics["errors_encountered"], 1);
}

#[test]
fn grader_prompt_uses_eight_steps_and_fills_claims() {
    let root = tempfile::tempdir().unwrap();
    write_skill(&root.path().join("skill"));
    let mut spec = plan(root.path(), RunMode::Live);
    spec.repetitions = 1;
    let scripted = ScriptedProvider::new("grade");
    let report = run_plan(&spec, &scripted).unwrap();
    let prompts = scripted.prompts.lock().unwrap().clone();
    for phrase in [
        "Read the transcript completely",
        "Examine every output file",
        "burden of proof is on the expectation",
        "No partial credit",
        "Extract implicit claims",
        "Summarize the executor notes",
        "Critique the evals",
        "Write grading.json",
        "Fold in metrics.json and timing.json",
    ] {
        assert!(
            prompts.iter().any(|prompt| prompt.contains(phrase)),
            "{phrase}"
        );
    }
    let grading = fs::read_to_string(report.cells[0].dir.join("grading.json")).unwrap();
    assert!(grading.contains("claim-from-provider"), "{grading}");
    assert!(grading.contains("eval-feedback-from-provider"), "{grading}");
}

#[test]
fn fixture_choice_depends_on_the_seed() {
    let root = tempfile::tempdir().unwrap();
    let set = root.path().join("fixtures");
    fs::create_dir_all(&set).unwrap();
    fs::write(set.join("alpha.txt"), "alpha-bytes").unwrap();
    fs::write(set.join("beta.txt"), "beta-bytes").unwrap();
    let first = materialize_fixture(&set, 0, &root.path().join("out-0")).unwrap();
    let second = materialize_fixture(&set, 1, &root.path().join("out-1")).unwrap();
    assert_ne!(first, second);
    assert_eq!(
        fs::read(root.path().join("out-0")).unwrap(),
        fs::read(&first).unwrap()
    );
    assert_eq!(
        fs::read(root.path().join("out-1")).unwrap(),
        fs::read(&second).unwrap()
    );
}

#[cfg(unix)]
#[test]
fn trigger_queries_spawn_once_each_and_versions_are_compared() {
    let root = tempfile::tempdir().unwrap();
    let bin = root.path().join("claude-fake");
    fs::write(
        &bin,
        "#!/bin/sh\nprintf '%s\\n' \"$*\" > \"$PWD/invocation.txt\"\nexit 0\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&bin).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&bin, permissions).unwrap();
    }
    let queries = vec![
        "query alpha".into(),
        "query beta".into(),
        "query gamma".into(),
    ];
    let work = root.path().join("triggers");
    let count = spawn_trigger_queries(&bin, &queries, &work).unwrap();
    assert_eq!(count, 3);
    for (index, query) in queries.iter().enumerate() {
        let invocation =
            fs::read_to_string(work.join(format!("query-{index}")).join("invocation.txt")).unwrap();
        assert!(invocation.contains("-p"), "{invocation}");
        assert!(invocation.contains(query), "{invocation}");
    }
    let left = root.path().join("skill-left");
    let right = root.path().join("skill-right");
    fs::create_dir_all(&left).unwrap();
    fs::create_dir_all(&right).unwrap();
    fs::write(left.join("SKILL.md"), "LEFT-SKILL-BODY\n").unwrap();
    fs::write(right.join("SKILL.md"), "RIGHT-SKILL-BODY\n").unwrap();
    let out = root.path().join("comparison.json");
    run_version_comparison(&left, &right, &out).unwrap();
    let body = fs::read_to_string(&out).unwrap();
    assert!(body.contains("version_versus_version"));
    assert!(body.contains("LEFT-SKILL-BODY"));
    assert!(body.contains("RIGHT-SKILL-BODY"));
}

fn sse_tool(content: &str) -> String {
    let partial = serde_json::json!({"path": "outputs/answer.txt", "content": content}).to_string();
    format!(
        "data: {{\"type\":\"message_start\",\"message\":{{\"id\":\"m\",\"usage\":{{\"input_tokens\":4}}}}}}\n\ndata: {{\"type\":\"content_block_start\",\"content_block\":{{\"type\":\"tool_use\",\"id\":\"tool-1\",\"name\":\"write_file\"}}}}\n\ndata: {{\"type\":\"content_block_delta\",\"delta\":{{\"type\":\"input_json_delta\",\"partial_json\":{}}}}}\n\ndata: {{\"type\":\"content_block_stop\"}}\n\ndata: {{\"type\":\"message_delta\",\"delta\":{{\"stop_reason\":\"tool_use\"}},\"usage\":{{\"output_tokens\":3}}}}\n\ndata: {{\"type\":\"message_stop\"}}\n\n",
        serde_json::to_string(&partial).unwrap()
    )
}
