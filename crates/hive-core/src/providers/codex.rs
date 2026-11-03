//! OpenAI Codex CLI provider (`codex exec`).
//!
//! `execute()` runs `codex exec --json --skip-git-repo-check --sandbox workspace-write -C <cwd>
//! [--model m] [extra args] <prompt>` and parses the JSONL events: the last `agent_message` is
//! the answer, `turn.completed` carries token usage, and command executions, file changes, MCP
//! tool calls and web searches count as tool calls. Both the current event schema
//! (`{"type":"item.completed","item":{...}}`) and the older one (`{"msg":{"type":...}}`) are
//! understood. For the with-skill arm the skill is copied to `<cwd>/.codex/skills/<name>` and
//! the prompt is prefixed with a notice that points at it.
//!
//! `complete()` runs `codex exec --json --sandbox read-only` in a scratch directory with the
//! system prompt prepended to the user prompt (Codex has no separate system-prompt flag).

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;

use super::{
    CompletionRequest, CompletionResponse, ExecutionResult, ExecutionTask, Provider, process,
    sandbox,
};
use crate::error::{HiveError, IoContext, Result};
use crate::model::{ProviderConfig, ProviderProbe};

/// Executable used when the config does not name one.
pub const DEFAULT_COMMAND: &str = "codex";
/// Where the skill is installed inside the sandbox.
pub const SKILLS_DIR: &str = ".codex/skills";

/// What the `codex exec --json` events add up to.
#[derive(Debug, Clone, Default)]
pub struct CodexSummary {
    /// Text of the last agent message.
    pub last_message: Option<String>,
    pub tool_calls: u32,
    /// Input tokens including cached input.
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub error: Option<String>,
    pub model: Option<String>,
    pub num_events: u32,
}

const TOOL_ITEM_TYPES: [&str; 6] = [
    "command_execution",
    "file_change",
    "patch_apply",
    "mcp_tool_call",
    "web_search",
    "tool_call",
];
const LEGACY_TOOL_EVENTS: [&str; 5] = [
    "exec_command_begin",
    "patch_apply_begin",
    "mcp_tool_call_begin",
    "web_search_begin",
    "custom_tool_call_begin",
];

fn u64_at(v: &Value, key: &str) -> u64 {
    v.get(key).and_then(Value::as_u64).unwrap_or(0)
}

impl CodexSummary {
    fn absorb_usage(&mut self, usage: &Value) {
        let input = u64_at(usage, "input_tokens");
        let cached = u64_at(usage, "cached_input_tokens");
        let output = u64_at(usage, "output_tokens");
        if input + cached + output > 0 {
            // Older builds report cached tokens as a subset of input tokens; newer ones report
            // them separately. Taking the max keeps both readings sane.
            self.input_tokens = (input + cached).max(self.input_tokens);
            self.output_tokens = output.max(self.output_tokens);
        }
    }

    /// Fold one parsed event into the summary.
    pub fn absorb(&mut self, event: &Value) {
        self.num_events += 1;
        if let Some(kind) = event.get("type").and_then(Value::as_str) {
            match kind {
                "item.completed" => {
                    let item = &event["item"];
                    match item.get("type").and_then(Value::as_str) {
                        Some("agent_message") => {
                            if let Some(t) = item.get("text").and_then(Value::as_str) {
                                self.last_message = Some(t.to_string());
                            }
                        }
                        Some(t) if TOOL_ITEM_TYPES.contains(&t) => self.tool_calls += 1,
                        _ => {}
                    }
                }
                "turn.completed" => {
                    if let Some(u) = event.get("usage") {
                        self.absorb_usage(u);
                    }
                }
                "turn.failed" | "error" => {
                    let message = event
                        .get("error")
                        .and_then(|e| e.get("message").or(Some(e)))
                        .and_then(Value::as_str)
                        .or_else(|| event.get("message").and_then(Value::as_str))
                        .unwrap_or("codex reported an error");
                    self.error = Some(message.to_string());
                }
                "thread.started" | "session_configured" => {
                    if let Some(m) = event.get("model").and_then(Value::as_str) {
                        self.model = Some(m.to_string());
                    }
                }
                _ => {}
            }
            return;
        }
        if let Some(msg) = event.get("msg") {
            match msg.get("type").and_then(Value::as_str) {
                Some("agent_message") => {
                    if let Some(t) = msg.get("message").and_then(Value::as_str) {
                        self.last_message = Some(t.to_string());
                    }
                }
                Some("token_count") => {
                    let usage = msg
                        .get("info")
                        .and_then(|i| i.get("total_token_usage"))
                        .unwrap_or(msg);
                    self.absorb_usage(usage);
                }
                Some("error") => {
                    self.error = msg
                        .get("message")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                        .or(Some("codex reported an error".into()));
                }
                Some("session_configured") => {
                    if let Some(m) = msg.get("model").and_then(Value::as_str) {
                        self.model = Some(m.to_string());
                    }
                }
                Some(t) if LEGACY_TOOL_EVENTS.contains(&t) => self.tool_calls += 1,
                _ => {}
            }
        }
    }

    /// Parse one JSONL line (non-JSON lines are ignored).
    pub fn absorb_line(&mut self, line: &str) {
        if let Ok(v) = serde_json::from_str::<Value>(line.trim()) {
            self.absorb(&v);
        }
    }
}

/// Codex CLI provider.
pub struct CodexProvider {
    config: ProviderConfig,
}

impl CodexProvider {
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

    fn exec_command(&self, cwd: &Path, sandbox_mode: &str) -> tokio::process::Command {
        let mut cmd = process::command(&self.program());
        cmd.current_dir(cwd);
        cmd.envs(&self.config.env);
        cmd.arg("exec")
            .arg("--json")
            .arg("--skip-git-repo-check")
            .arg("--sandbox")
            .arg(sandbox_mode)
            .arg("-C")
            .arg(cwd);
        if let Some(model) = self
            .config
            .model
            .as_deref()
            .filter(|m| !m.trim().is_empty())
        {
            cmd.arg("--model").arg(model.trim());
        }
        cmd.args(&self.config.args);
        cmd
    }

    fn fail(&self, message: impl Into<String>) -> HiveError {
        HiveError::provider(&self.config.name, message)
    }
}

#[async_trait]
impl Provider for CodexProvider {
    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse> {
        let scratch = tempfile::tempdir()?;
        let prompt = match req.system.as_deref().filter(|s| !s.trim().is_empty()) {
            Some(system) => format!(
                "System instructions:\n{system}\n\nUser request:\n{}",
                req.prompt
            ),
            None => req.prompt.clone(),
        };
        let mut cmd = self.exec_command(scratch.path(), "read-only");
        cmd.arg(&prompt);
        let mut summary = CodexSummary::default();
        let out = process::run_streaming(cmd, req.timeout_or_default(), &req.cancel, |line| {
            summary.absorb_line(line);
        })
        .await?;
        if let Some(err) = &summary.error {
            if summary.last_message.is_none() {
                return Err(self.fail(err.clone()));
            }
        }
        let Some(text) = summary.last_message.clone() else {
            return Err(self.fail(format!(
                "codex exited with status {} without an agent message: {}",
                out.status.unwrap_or(-1),
                process::tail(&out.stderr, 500)
            )));
        };
        Ok(CompletionResponse {
            text,
            input_tokens: summary.input_tokens,
            output_tokens: summary.output_tokens,
            latency_ms: out.duration_ms,
            model: summary.model.or_else(|| self.config.model.clone()),
            cost_usd: None,
        })
    }

    async fn execute(&self, task: ExecutionTask) -> Result<ExecutionResult> {
        std::fs::create_dir_all(&task.cwd).at(&task.cwd)?;
        let mut prompt = task.prompt.clone();
        if let Some(skill) = &task.skill_dir {
            let (name, _) = sandbox::install_skill(skill, &task.cwd.join(SKILLS_DIR))?;
            prompt = format!(
                "{}{}",
                sandbox::skill_notice(&format!("./{SKILLS_DIR}/{name}")),
                prompt
            );
        }
        let mut cmd = self.exec_command(&task.cwd, "workspace-write");
        cmd.arg(&prompt);
        let (transcript, mut file) = sandbox::open_transcript(&task.cwd)?;
        let before = sandbox::snapshot(&task.cwd);
        let mut summary = CodexSummary::default();
        let out = process::run_streaming(cmd, task.timeout, &task.cancel, |line| {
            let _ = writeln!(file, "{line}");
            summary.absorb_line(line);
        })
        .await?;
        drop(file);
        if summary.last_message.is_none() {
            let detail = summary
                .error
                .clone()
                .unwrap_or_else(|| process::tail(&out.stderr, 500));
            return Err(self.fail(format!(
                "codex exited with status {} without an agent message: {detail}",
                out.status.unwrap_or(-1)
            )));
        }
        let files_created = sandbox::files_created(&task.cwd, &before);
        Ok(ExecutionResult {
            output: summary.last_message.clone().unwrap_or_default(),
            transcript_path: Some(transcript),
            tool_calls: summary.tool_calls,
            input_tokens: summary.input_tokens,
            output_tokens: summary.output_tokens,
            duration_ms: out.duration_ms,
            cost_usd: None,
            files_created,
            model: summary.model.or_else(|| self.config.model.clone()),
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

    #[test]
    fn parses_current_event_schema() {
        let lines = [
            r#"{"type":"thread.started","thread_id":"t","model":"gpt-5-codex"}"#,
            r#"{"type":"turn.started"}"#,
            r#"{"type":"item.started","item":{"id":"i0","type":"command_execution","command":"ls"}}"#,
            r#"{"type":"item.completed","item":{"id":"i0","type":"command_execution","command":"ls","exit_code":0}}"#,
            r#"{"type":"item.completed","item":{"id":"i1","type":"file_change","changes":[{"path":"a.md","kind":"add"}]}}"#,
            r#"{"type":"item.completed","item":{"id":"i2","type":"agent_message","text":"first"}}"#,
            r#"{"type":"item.completed","item":{"id":"i3","type":"agent_message","text":"Done, wrote a.md"}}"#,
            r#"{"type":"turn.completed","usage":{"input_tokens":100,"cached_input_tokens":40,"output_tokens":25}}"#,
        ];
        let mut s = CodexSummary::default();
        for l in lines {
            s.absorb_line(l);
        }
        assert_eq!(s.tool_calls, 2);
        assert_eq!(s.last_message.as_deref(), Some("Done, wrote a.md"));
        assert_eq!((s.input_tokens, s.output_tokens), (140, 25));
        assert_eq!(s.model.as_deref(), Some("gpt-5-codex"));
        assert!(s.error.is_none());
    }

    #[test]
    fn parses_legacy_event_schema_and_errors() {
        let lines = [
            r#"{"id":"0","msg":{"type":"session_configured","model":"o4-mini"}}"#,
            r#"{"id":"1","msg":{"type":"exec_command_begin","command":["bash","-lc","ls"]}}"#,
            r#"{"id":"2","msg":{"type":"exec_command_end","exit_code":0}}"#,
            r#"{"id":"3","msg":{"type":"patch_apply_begin"}}"#,
            r#"{"id":"4","msg":{"type":"agent_message","message":"All set."}}"#,
            r#"{"id":"5","msg":{"type":"token_count","info":{"total_token_usage":{"input_tokens":50,"cached_input_tokens":10,"output_tokens":5}}}}"#,
            "garbage",
        ];
        let mut s = CodexSummary::default();
        for l in lines {
            s.absorb_line(l);
        }
        assert_eq!(s.tool_calls, 2);
        assert_eq!(s.last_message.as_deref(), Some("All set."));
        assert_eq!((s.input_tokens, s.output_tokens), (60, 5));
        assert_eq!(s.model.as_deref(), Some("o4-mini"));

        let mut s = CodexSummary::default();
        s.absorb_line(r#"{"type":"turn.failed","error":{"message":"rate limited"}}"#);
        assert_eq!(s.error.as_deref(), Some("rate limited"));
        let mut s = CodexSummary::default();
        s.absorb_line(r#"{"msg":{"type":"error","message":"bad key"}}"#);
        assert_eq!(s.error.as_deref(), Some("bad key"));
    }

    #[test]
    fn program_defaults_to_codex() {
        let mut cfg = super::super::scripted::test_support::config("c", Path::new("/x"));
        cfg.command = Some("  ".into());
        assert_eq!(CodexProvider::new(cfg).program(), "codex");
    }
}
