//! Claude Code CLI provider (`claude -p`).
//!
//! `execute()` runs `claude -p <prompt> --output-format stream-json --verbose --permission-mode
//! acceptEdits` (plus `--model` when set, then the configured extra args) with the sandbox as
//! working directory and parses the stream-json events: every `tool_use` content block counts
//! as a tool call and the final `result` event carries the answer, token usage, cost and
//! duration. For the with-skill arm the skill is copied to `<cwd>/.claude/skills/<name>` so
//! Claude Code discovers it as a project skill, and the prompt is prefixed with a notice that
//! points at it (mirroring Anthropic's skill-creator evaluation loop).
//!
//! `complete()` runs `claude -p <prompt> --output-format json` in a scratch directory, passing
//! the system prompt through `--append-system-prompt`.

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;

use super::trace::{Dialect, TraceBuilder};
use super::{
    CompletionRequest, CompletionResponse, ExecutionResult, ExecutionTask, Provider, process,
    sandbox,
};
use crate::error::{HiveError, IoContext, Result};
use crate::model::{ProviderConfig, ProviderProbe};

/// Executable used when the config does not name one.
pub const DEFAULT_COMMAND: &str = "claude";
/// Where the skill is installed inside the sandbox.
pub const SKILLS_DIR: &str = ".claude/skills";

/// What the stream-json (or `--output-format json`) events add up to.
#[derive(Debug, Clone, Default)]
pub struct StreamSummary {
    /// Text of the final `result` event.
    pub result: Option<String>,
    /// Concatenated assistant text blocks (fallback when no result event arrives).
    pub assistant_text: String,
    pub tool_calls: u32,
    /// Input tokens including cache creation and cache reads.
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: Option<f64>,
    pub duration_ms: Option<u64>,
    pub is_error: bool,
    pub model: Option<String>,
    pub num_events: u32,
}

impl StreamSummary {
    /// Fold one parsed event into the summary.
    pub fn absorb(&mut self, event: &Value) {
        self.num_events += 1;
        match event.get("type").and_then(Value::as_str) {
            Some("assistant") => {
                let message = &event["message"];
                if let Some(model) = message.get("model").and_then(Value::as_str) {
                    self.model = Some(model.to_string());
                }
                if let Some(blocks) = message.get("content").and_then(Value::as_array) {
                    for block in blocks {
                        match block.get("type").and_then(Value::as_str) {
                            Some("tool_use") => self.tool_calls += 1,
                            Some("text") => {
                                if let Some(t) = block.get("text").and_then(Value::as_str) {
                                    if !self.assistant_text.is_empty() {
                                        self.assistant_text.push('\n');
                                    }
                                    self.assistant_text.push_str(t);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            Some("result") => {
                if let Some(r) = event.get("result").and_then(Value::as_str) {
                    self.result = Some(r.to_string());
                }
                self.is_error = event
                    .get("is_error")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                if let Some(u) = event.get("usage") {
                    self.input_tokens = u64_at(u, "input_tokens")
                        + u64_at(u, "cache_creation_input_tokens")
                        + u64_at(u, "cache_read_input_tokens");
                    self.output_tokens = u64_at(u, "output_tokens");
                }
                if let Some(c) = event.get("total_cost_usd").and_then(Value::as_f64) {
                    self.cost_usd = Some(c);
                }
                if let Some(d) = event.get("duration_ms").and_then(Value::as_u64) {
                    self.duration_ms = Some(d);
                }
                if self.model.is_none() {
                    if let Some(usage) = event.get("modelUsage").and_then(Value::as_object) {
                        self.model = usage.keys().next().cloned();
                    }
                }
            }
            _ => {}
        }
    }

    /// Parse one stream-json line (non-JSON lines are ignored).
    pub fn absorb_line(&mut self, line: &str) {
        if let Ok(v) = serde_json::from_str::<Value>(line.trim()) {
            self.absorb(&v);
        }
    }

    /// The best available answer text.
    pub fn output(&self) -> String {
        self.result
            .clone()
            .unwrap_or_else(|| self.assistant_text.clone())
    }
}

fn u64_at(v: &Value, key: &str) -> u64 {
    v.get(key).and_then(Value::as_u64).unwrap_or(0)
}

/// Parse the stdout of `claude -p --output-format json`: a single result object, or (with
/// `--verbose`) an array of events, or one JSON object per line.
pub fn parse_json_output(text: &str) -> Result<StreamSummary> {
    let mut summary = StreamSummary::default();
    let trimmed = text.trim();
    if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
        match v {
            Value::Array(items) => {
                for item in &items {
                    summary.absorb(item);
                }
            }
            other => summary.absorb(&other),
        }
    } else {
        for line in trimmed.lines() {
            summary.absorb_line(line);
        }
    }
    if summary.num_events == 0 {
        return Err(HiveError::msg(format!(
            "claude returned no JSON: {}",
            process::tail(trimmed, 300)
        )));
    }
    Ok(summary)
}

/// Claude Code CLI provider.
pub struct ClaudeCodeProvider {
    config: ProviderConfig,
}

impl ClaudeCodeProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }

    fn program(&self) -> String {
        self.config
            .command
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .unwrap_or(DEFAULT_COMMAND)
            .to_string()
    }

    fn base_command(&self, cwd: &Path) -> tokio::process::Command {
        let mut cmd = process::command(&self.program());
        cmd.current_dir(cwd);
        cmd.envs(&self.config.env);
        cmd
    }

    fn push_model_and_args(&self, cmd: &mut tokio::process::Command) {
        if let Some(model) = self
            .config
            .model
            .as_deref()
            .filter(|m| !m.trim().is_empty())
        {
            cmd.arg("--model").arg(model.trim());
        }
        cmd.args(&self.config.args);
    }

    fn fail(&self, message: impl Into<String>) -> HiveError {
        HiveError::provider(&self.config.name, message)
    }
}

#[async_trait]
impl Provider for ClaudeCodeProvider {
    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse> {
        let scratch = tempfile::tempdir()?;
        let mut cmd = self.base_command(scratch.path());
        cmd.arg("-p")
            .arg(&req.prompt)
            .args(["--output-format", "json"]);
        if let Some(system) = req.system.as_deref().filter(|s| !s.trim().is_empty()) {
            cmd.arg("--append-system-prompt").arg(system);
        }
        self.push_model_and_args(&mut cmd);
        let out = process::run_capture(cmd, req.timeout_or_default(), &req.cancel).await?;
        if !out.success && out.stdout.trim().is_empty() {
            return Err(self.fail(format!(
                "claude exited with status {}: {}",
                out.status.unwrap_or(-1),
                process::tail(&out.stderr, 500)
            )));
        }
        let summary = parse_json_output(&out.stdout)?;
        if summary.is_error {
            return Err(self.fail(format!("claude reported an error: {}", summary.output())));
        }
        Ok(CompletionResponse {
            text: summary.output(),
            input_tokens: summary.input_tokens,
            output_tokens: summary.output_tokens,
            latency_ms: summary.duration_ms.unwrap_or(out.duration_ms),
            model: summary.model.or_else(|| self.config.model.clone()),
            cost_usd: summary.cost_usd,
        })
    }

    async fn execute(&self, task: ExecutionTask) -> Result<ExecutionResult> {
        std::fs::create_dir_all(&task.cwd).at(&task.cwd)?;
        let mut prompt = task.prompt.clone();
        let mut skill_marker = None;
        if let Some(skill) = &task.skill_dir {
            let (name, _) = sandbox::install_skill(skill, &task.cwd.join(SKILLS_DIR))?;
            prompt = format!(
                "{}{}",
                sandbox::skill_notice(&format!("./{SKILLS_DIR}/{name}")),
                prompt
            );
            skill_marker = Some(format!("{SKILLS_DIR}/{name}"));
        }
        let mut cmd = self.base_command(&task.cwd);
        cmd.arg("-p").arg(&prompt).args([
            "--output-format",
            "stream-json",
            "--verbose",
            "--permission-mode",
            "acceptEdits",
        ]);
        self.push_model_and_args(&mut cmd);

        let (transcript, mut file) = sandbox::open_transcript(&task.cwd)?;
        let before = sandbox::snapshot(&task.cwd);
        let mut summary = StreamSummary::default();
        let mut trace = TraceBuilder::new(Dialect::ClaudeCode, &task.cwd, skill_marker.as_deref());
        let started = std::time::Instant::now();
        let out = process::run_streaming(cmd, task.timeout, &task.cancel, |line| {
            let _ = writeln!(file, "{line}");
            summary.absorb_line(line);
            trace.absorb_line(line, Some(started.elapsed().as_millis() as u64));
        })
        .await?;
        drop(file);
        if summary.result.is_none() && !out.success {
            return Err(self.fail(format!(
                "claude exited with status {} before producing a result: {}",
                out.status.unwrap_or(-1),
                process::tail(&out.stderr, 500)
            )));
        }
        if summary.is_error {
            return Err(self.fail(format!("claude reported an error: {}", summary.output())));
        }
        let files_created = sandbox::files_created(&task.cwd, &before);
        Ok(ExecutionResult {
            output: summary.output(),
            transcript_path: Some(transcript),
            tool_calls: summary.tool_calls,
            input_tokens: summary.input_tokens,
            output_tokens: summary.output_tokens,
            duration_ms: summary.duration_ms.unwrap_or(out.duration_ms),
            cost_usd: summary.cost_usd,
            files_created,
            model: summary.model.or_else(|| self.config.model.clone()),
            trace: Some(trace.finish(summary.output_tokens)),
        })
    }

    async fn probe(&self) -> ProviderProbe {
        let started = std::time::Instant::now();
        let program = self.program();
        let (ok, message, version) =
            match process::version_of(&program, Duration::from_secs(20)).await {
                Ok(v) => (true, format!("{program} {v}"), Some(v)),
                Err(e) => (false, e.to_string(), None),
            };
        ProviderProbe {
            ok,
            latency_ms: started.elapsed().as_millis() as u64,
            message,
            version,
            checked_at: crate::util::now_ms(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STREAM: &str = r#"{"type":"system","subtype":"init","model":"claude-sonnet-4-5"}
{"type":"assistant","message":{"model":"claude-sonnet-4-5","content":[{"type":"text","text":"Let me look."},{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"x"}}]}}
{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1","content":"..."}]}}
{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t2","name":"Write","input":{}}]}}
not json at all
{"type":"result","subtype":"success","is_error":false,"duration_ms":4321,"num_turns":3,"result":"Wrote report.md","total_cost_usd":0.0123,"usage":{"input_tokens":10,"cache_creation_input_tokens":20,"cache_read_input_tokens":30,"output_tokens":7}}"#;

    #[test]
    fn stream_summary_counts_tools_and_reads_result() {
        let mut s = StreamSummary::default();
        for line in STREAM.lines() {
            s.absorb_line(line);
        }
        assert_eq!(s.tool_calls, 2);
        assert_eq!(s.result.as_deref(), Some("Wrote report.md"));
        assert_eq!(s.input_tokens, 60);
        assert_eq!(s.output_tokens, 7);
        assert_eq!(s.cost_usd, Some(0.0123));
        assert_eq!(s.duration_ms, Some(4321));
        assert_eq!(s.model.as_deref(), Some("claude-sonnet-4-5"));
        assert!(!s.is_error);
        assert_eq!(s.output(), "Wrote report.md");
    }

    #[test]
    fn json_output_accepts_object_array_and_lines() {
        let obj = r#"{"type":"result","result":"hi","usage":{"input_tokens":3,"output_tokens":1},"is_error":false}"#;
        assert_eq!(parse_json_output(obj).unwrap().output(), "hi");
        let arr = format!("[{obj}]");
        assert_eq!(parse_json_output(&arr).unwrap().input_tokens, 3);
        assert_eq!(parse_json_output(STREAM).unwrap().tool_calls, 2);
        assert!(parse_json_output("nothing here").is_err());
        let fallback =
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"partial"}]}}"#;
        assert_eq!(parse_json_output(fallback).unwrap().output(), "partial");
    }

    #[test]
    fn program_defaults_to_claude() {
        let mut cfg = super::super::scripted::test_support::config("c", Path::new("/x"));
        cfg.command = None;
        let p = ClaudeCodeProvider::new(cfg);
        assert_eq!(p.program(), "claude");
    }
}
