//! Deterministic offline provider driven by a JSON script. Tests and the offline demo use it to
//! simulate "with the skill the agent writes the right file; without it, it does not".
//!
//! # Script format
//!
//! The script is a JSON file whose path is the provider's `command`:
//!
//! ```json
//! {
//!   "rules": [
//!     {
//!       "match": "(?i)skill is available",
//!       "with_skill": true,
//!       "output": "Done. I followed the skill and wrote report.md.",
//!       "files": { "report.md": "# Report\n\nTotal: 42\n" },
//!       "tool_calls": 3,
//!       "input_tokens": 1200,
//!       "output_tokens": 300,
//!       "duration_ms": 20,
//!       "cost_usd": 0.004
//!     },
//!     { "system": "(?i)available skills", "match": "(?i)pdf", "output": "{\"skill\": \"pdf-tools\"}" },
//!     { "match": "crash", "fail": true, "error": "simulated provider failure" }
//!   ],
//!   "default": { "output": "I could not find a procedure for this, so I answered from memory." }
//! }
//! ```
//!
//! Rules are tried in order; the first whose conditions all hold wins, else `default`, else the
//! call fails. Conditions:
//!
//! * `match`: regular expression over the task prompt (for `complete()`, the user prompt). For
//!   `execute()` the prompt already carries the "A skill is available at …" notice when a
//!   skill is present, so rules can match on it.
//! * `system`: regular expression over the system prompt (`complete()` only; never matches in
//!   `execute()`, which has no system prompt).
//! * `with_skill`: `true` requires a skill directory in the task, `false` requires none.
//!   `complete()` counts as "no skill".
//!
//! A rule with no conditions matches everything. Effects: `output` is the final text; `files`
//! are written into the sandbox (`path` → `content`, paths relative to the sandbox, no escapes);
//! `tool_calls`, `input_tokens`, `output_tokens` (defaults: token estimates of the prompt and
//! the output), `duration_ms` (the provider really waits this long, honouring cancellation and
//! the timeout) and `cost_usd` feed the metrics; `fail: true` makes the call return an error
//! whose message is `error`.
//!
//! The execution trace is simulated too: every file written is a write; `reads`, `searches`
//! and `commands` add that many tool calls of each kind (when a skill is installed the first
//! read opens its `SKILL.md`); `thinking` is recorded as one thinking block. `tool_calls` is
//! raised to the total of these when it is lower.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use super::trace::{Dialect, TraceBuilder};
use super::{
    CompletionRequest, CompletionResponse, ExecutionResult, ExecutionTask, Provider, sandbox,
};
use crate::error::{HiveError, IoContext, Result};
use crate::model::{ProviderConfig, ProviderProbe, ToolCategory};

/// Directory (inside the sandbox) where the scripted provider installs the skill.
pub const SKILLS_DIR: &str = ".skills";

/// One scripted behaviour. See the module docs for the semantics of each field.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ScriptRule {
    #[serde(rename = "match", skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_skill: Option<bool>,
    pub output: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub files: BTreeMap<String, String>,
    pub tool_calls: u32,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    pub fail: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Simulated file reads (the first one opens the skill's `SKILL.md` when one is installed).
    pub reads: u32,
    /// Simulated searches (`Glob`/`Grep`-style scans).
    pub searches: u32,
    /// Simulated shell commands.
    pub commands: u32,
    /// Simulated chain of thought, recorded as one thinking block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
}

impl ScriptRule {
    fn matches(&self, prompt: &str, system: Option<&str>, has_skill: bool) -> Result<bool> {
        if let Some(p) = &self.pattern {
            if !regex(p)?.is_match(prompt) {
                return Ok(false);
            }
        }
        if let Some(s) = &self.system {
            match system {
                Some(text) if regex(s)?.is_match(text) => {}
                _ => return Ok(false),
            }
        }
        if let Some(w) = self.with_skill {
            if w != has_skill {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

fn regex(pattern: &str) -> Result<regex::Regex> {
    regex::Regex::new(pattern)
        .map_err(|e| HiveError::invalid(format!("script regex `{pattern}` is invalid: {e}")))
}

/// A whole script: ordered rules plus an optional fallback.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Script {
    pub rules: Vec<ScriptRule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<ScriptRule>,
}

impl Script {
    /// Read and validate a script file (every regex must compile).
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).at(path)?;
        let script: Script = serde_json::from_str(&text).map_err(|e| {
            HiveError::invalid(format!("script {} is not valid: {e}", path.display()))
        })?;
        script.validate()?;
        Ok(script)
    }

    /// Check every regex compiles.
    pub fn validate(&self) -> Result<()> {
        for rule in self.rules.iter().chain(self.default.iter()) {
            if let Some(p) = &rule.pattern {
                regex(p)?;
            }
            if let Some(s) = &rule.system {
                regex(s)?;
            }
        }
        Ok(())
    }

    /// Pick the rule for a call.
    pub fn select(
        &self,
        prompt: &str,
        system: Option<&str>,
        has_skill: bool,
    ) -> Result<&ScriptRule> {
        for rule in &self.rules {
            if rule.matches(prompt, system, has_skill)? {
                return Ok(rule);
            }
        }
        self.default.as_ref().ok_or_else(|| {
            HiveError::invalid("no scripted rule matches the prompt and no default rule is set")
        })
    }
}

/// The scripted provider.
pub struct ScriptedProvider {
    config: ProviderConfig,
    script: Script,
    path: Option<PathBuf>,
}

impl ScriptedProvider {
    /// Load the script at `config.command`.
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let path = config
            .command
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .map(crate::paths::expand_tilde)
            .ok_or_else(|| {
                HiveError::invalid("scripted providers need the script path in `command`")
            })?;
        let script = Script::load(&path)?;
        Ok(Self {
            config,
            script,
            path: Some(path),
        })
    }

    /// Use an in-memory script (tests).
    pub fn from_script(config: ProviderConfig, script: Script) -> Self {
        Self {
            config,
            script,
            path: None,
        }
    }

    pub fn script(&self) -> &Script {
        &self.script
    }

    async fn wait(&self, ms: u64, timeout: Duration, cancel: &CancellationToken) -> Result<()> {
        if cancel.is_cancelled() {
            return Err(HiveError::Cancelled);
        }
        if ms == 0 {
            return Ok(());
        }
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(ms)) => Ok(()),
            _ = tokio::time::sleep(timeout) => Err(HiveError::Timeout(timeout.as_secs())),
            _ = cancel.cancelled() => Err(HiveError::Cancelled),
        }
    }

    fn failure(&self, rule: &ScriptRule) -> HiveError {
        HiveError::provider(
            &self.config.name,
            rule.error
                .clone()
                .unwrap_or_else(|| "scripted failure".to_string()),
        )
    }
}

#[async_trait]
impl Provider for ScriptedProvider {
    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse> {
        let started = Instant::now();
        let rule = self
            .script
            .select(&req.prompt, req.system.as_deref(), false)?
            .clone();
        self.wait(rule.duration_ms, req.timeout_or_default(), &req.cancel)
            .await?;
        if rule.fail {
            return Err(self.failure(&rule));
        }
        let prompt_len = req.prompt.len() + req.system.as_deref().map_or(0, str::len);
        Ok(CompletionResponse {
            input_tokens: if rule.input_tokens > 0 {
                rule.input_tokens
            } else {
                (prompt_len as u64).div_ceil(4)
            },
            output_tokens: if rule.output_tokens > 0 {
                rule.output_tokens
            } else {
                crate::util::estimate_tokens(&rule.output) as u64
            },
            text: rule.output,
            latency_ms: started.elapsed().as_millis() as u64,
            model: self.config.model.clone(),
            cost_usd: rule.cost_usd,
        })
    }

    async fn execute(&self, task: ExecutionTask) -> Result<ExecutionResult> {
        let started = Instant::now();
        std::fs::create_dir_all(&task.cwd).at(&task.cwd)?;
        let mut prompt = task.prompt.clone();
        let mut skill_name = None;
        if let Some(skill) = &task.skill_dir {
            let (name, _) = sandbox::install_skill(skill, &task.cwd.join(SKILLS_DIR))?;
            prompt = format!(
                "{}{}",
                sandbox::skill_notice(&format!("./{SKILLS_DIR}/{name}")),
                prompt
            );
            skill_name = Some(name);
        }
        let has_skill = skill_name.is_some();
        let (rule_index, rule) = {
            let rule = self.script.select(&prompt, None, has_skill)?;
            let index = self.script.rules.iter().position(|r| std::ptr::eq(r, rule));
            (index, rule.clone())
        };
        let before = sandbox::snapshot(&task.cwd);
        self.wait(rule.duration_ms, task.timeout, &task.cancel)
            .await?;
        if rule.fail {
            return Err(self.failure(&rule));
        }
        let mut lines = vec![
            serde_json::json!({"type": "prompt", "text": prompt, "label": task.label, "skill": skill_name}),
            serde_json::json!({"type": "rule", "index": rule_index, "default": rule_index.is_none()}),
        ];
        let skill_marker = skill_name.as_ref().map(|n| format!("{SKILLS_DIR}/{n}"));
        let mut trace = TraceBuilder::new(Dialect::ClaudeCode, &task.cwd, skill_marker.as_deref());
        let mut at = 0u64;
        if let Some(thinking) = rule.thinking.as_deref().filter(|t| !t.trim().is_empty()) {
            trace.thinking(thinking, Some(at));
            lines.push(serde_json::json!({"type": "thinking", "text": thinking}));
        }
        for i in 0..rule.reads {
            let path = match (&skill_marker, i) {
                (Some(marker), 0) => format!("{marker}/SKILL.md"),
                _ => format!("input-{}.txt", i + 1),
            };
            trace.turn();
            at += 1;
            trace.tool_use(
                None,
                "read_file",
                ToolCategory::Read,
                &path,
                Some(&path),
                path.len() as u64,
                Some(at),
            );
            at += 1;
            trace.tool_result(None, 400, "(simulated file content)", false, Some(at));
            lines.push(serde_json::json!({"type": "tool_use", "name": "read_file", "path": path}));
        }
        for i in 0..rule.searches {
            let pattern = format!("pattern-{}", i + 1);
            trace.turn();
            at += 1;
            trace.tool_use(
                None,
                "search",
                ToolCategory::Search,
                &pattern,
                None,
                pattern.len() as u64,
                Some(at),
            );
            at += 1;
            trace.tool_result(None, 200, "(simulated matches)", false, Some(at));
            lines.push(
                serde_json::json!({"type": "tool_use", "name": "search", "pattern": pattern}),
            );
        }
        for (rel, content) in &rule.files {
            if !crate::util::is_safe_relative(rel) {
                return Err(HiveError::invalid(format!(
                    "scripted file path `{rel}` escapes the sandbox"
                )));
            }
            let path = task.cwd.join(rel);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).at(parent)?;
            }
            std::fs::write(&path, content).at(&path)?;
            trace.turn();
            at += 1;
            trace.tool_use(
                None,
                "write_file",
                ToolCategory::Write,
                rel,
                Some(rel),
                content.chars().count() as u64,
                Some(at),
            );
            at += 1;
            trace.tool_result(None, 24, "File written", false, Some(at));
            lines.push(serde_json::json!({"type": "tool_use", "name": "write_file", "path": rel}));
        }
        for i in 0..rule.commands {
            let command = format!("run-step-{}", i + 1);
            trace.turn();
            at += 1;
            trace.tool_use(
                None,
                "shell",
                ToolCategory::Command,
                &command,
                None,
                command.len() as u64,
                Some(at),
            );
            at += 1;
            trace.tool_result(None, 100, "(simulated command output)", false, Some(at));
            lines
                .push(serde_json::json!({"type": "tool_use", "name": "shell", "command": command}));
        }
        let simulated = rule.reads + rule.searches + rule.files.len() as u32 + rule.commands;
        for _ in simulated..rule.tool_calls {
            trace.tool_use(None, "tool", ToolCategory::Other, "", None, 0, Some(at));
            trace.tool_result(None, 0, "", false, Some(at));
        }
        trace.turn();
        trace.text(&rule.output, Some(at + 1));
        lines.push(serde_json::json!({"type": "result", "text": rule.output}));
        let transcript = sandbox::write_transcript(&task.cwd, &lines)?;
        let files_created = sandbox::files_created(&task.cwd, &before);
        let output_tokens = if rule.output_tokens > 0 {
            rule.output_tokens
        } else {
            crate::util::estimate_tokens(&rule.output) as u64
        };
        Ok(ExecutionResult {
            input_tokens: if rule.input_tokens > 0 {
                rule.input_tokens
            } else {
                crate::util::estimate_tokens(&prompt) as u64
            },
            output_tokens,
            output: rule.output,
            transcript_path: Some(transcript),
            tool_calls: rule.tool_calls.max(simulated),
            duration_ms: started.elapsed().as_millis().max(rule.duration_ms as u128) as u64,
            cost_usd: rule.cost_usd,
            files_created,
            model: self.config.model.clone(),
            trace: Some(trace.finish(output_tokens)),
        })
    }

    async fn probe(&self) -> ProviderProbe {
        let started = Instant::now();
        let (ok, message) = match &self.path {
            Some(path) => match Script::load(path) {
                Ok(script) => (
                    true,
                    format!(
                        "script with {} rule(s){}",
                        script.rules.len(),
                        if script.default.is_some() {
                            " and a default"
                        } else {
                            ""
                        }
                    ),
                ),
                Err(e) => (false, e.to_string()),
            },
            None => (
                true,
                format!("in-memory script with {} rule(s)", self.script.rules.len()),
            ),
        };
        ProviderProbe {
            ok,
            latency_ms: started.elapsed().as_millis() as u64,
            message,
            version: None,
            checked_at: crate::util::now_ms(),
        }
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use crate::model::{ProviderKind, ProviderRole};

    /// A provider config pointing at a script file.
    pub fn config(id: &str, script_path: &Path) -> ProviderConfig {
        ProviderConfig {
            id: id.into(),
            name: format!("scripted-{id}"),
            kind: ProviderKind::Scripted,
            model: Some("script-1".into()),
            base_url: None,
            command: Some(script_path.to_string_lossy().to_string()),
            args: vec![],
            env: Default::default(),
            api_key_env: None,
            has_secret: false,
            roles: vec![
                ProviderRole::Executor,
                ProviderRole::Grader,
                ProviderRole::Recommender,
            ],
            timeout_secs: 60,
            max_concurrency: 4,
            last_probe: None,
            created_at: 0,
            updated_at: 0,
        }
    }

    /// Write `script` as JSON into `dir/<name>.json` and return the path.
    pub fn write_script(dir: &Path, name: &str, script: &serde_json::Value) -> PathBuf {
        let path = dir.join(format!("{name}.json"));
        std::fs::write(&path, serde_json::to_string_pretty(script).unwrap()).unwrap();
        path
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{config, write_script};
    use super::*;
    use crate::library::test_support::write_skill;

    fn script() -> serde_json::Value {
        serde_json::json!({
            "rules": [
                {"match": "skill is available", "with_skill": true, "output": "Followed the skill. MARKER",
                 "files": {"out/report.md": "# Report\nok\n"}, "tool_calls": 2, "reads": 1, "thinking": "Read the skill, then write the report.",
                 "input_tokens": 100, "output_tokens": 20, "cost_usd": 0.01},
                {"system": "(?i)available skills", "match": "(?i)pdf", "output": "{\"skill\": \"pdf-tools\"}"},
                {"system": "(?i)available skills", "output": "{\"skill\": null}"},
                {"match": "crash", "fail": true, "error": "boom"},
                {"match": "slow", "duration_ms": 5000, "output": "late"}
            ],
            "default": {"output": "No idea.", "tool_calls": 1}
        })
    }

    #[tokio::test]
    async fn rules_files_and_complete() {
        let tmp = tempfile::tempdir().unwrap();
        let path = write_script(tmp.path(), "s", &script());
        let provider = ScriptedProvider::new(config("p1", &path)).unwrap();
        let skill = write_skill(
            tmp.path(),
            "report-writer",
            "Write reports when asked for a report",
            "# Do it",
        );

        // with skill: rule 0 fires, file written, skill installed under .skills
        let cwd = tmp.path().join("sb1");
        let mut task = ExecutionTask::new("Write the report", &cwd);
        task.skill_dir = Some(skill.clone());
        let res = provider.execute(task).await.unwrap();
        assert!(res.output.contains("MARKER"));
        assert_eq!(res.files_created, vec!["out/report.md"]);
        assert_eq!(res.tool_calls, 2);
        assert_eq!((res.input_tokens, res.output_tokens), (100, 20));
        assert_eq!(res.cost_usd, Some(0.01));
        assert!(cwd.join(".skills/report-writer/SKILL.md").is_file());
        assert!(res.transcript_path.unwrap().is_file());
        assert!(cwd.join("out/report.md").is_file());
        let trace = res.trace.unwrap();
        assert_eq!(trace.writes, 1);
        assert_eq!(trace.reads, 1);
        assert_eq!(
            trace.skill_reads, 1,
            "the first simulated read opens SKILL.md"
        );
        assert_eq!(trace.files_read, vec![".skills/report-writer/SKILL.md"]);
        assert_eq!(trace.files_written, vec!["out/report.md"]);
        assert_eq!(trace.tool_calls, 2);
        assert_eq!(trace.thinking_blocks, 1);
        assert_eq!(trace.text_blocks, 1);
        assert_eq!(
            trace.thinking_tokens + trace.text_tokens + trace.tool_input_tokens,
            20
        );
        assert!(trace.io_share > 0.0);

        // without skill: default
        let res = provider
            .execute(ExecutionTask::new(
                "Write the report",
                tmp.path().join("sb2"),
            ))
            .await
            .unwrap();
        assert_eq!(res.output, "No idea.");
        assert!(res.files_created.is_empty());
        assert!(res.input_tokens > 0);
        let trace = res.trace.unwrap();
        assert_eq!(
            trace.tool_calls, 1,
            "tool_calls beyond the simulated ones count as other"
        );
        assert_eq!(trace.other_tools, 1);
        assert_eq!(trace.skill_reads, 0);

        // failure rule
        let err = provider
            .execute(ExecutionTask::new("please crash", tmp.path().join("sb3")))
            .await
            .unwrap_err();
        assert!(matches!(err, HiveError::Provider { .. }), "{err}");
        assert!(err.to_string().contains("boom"));

        // complete(): system + prompt conditions
        let r = provider
            .complete(
                CompletionRequest::new("Can you read this PDF?")
                    .with_system("Available skills:\n- pdf-tools: x"),
            )
            .await
            .unwrap();
        assert_eq!(r.text, "{\"skill\": \"pdf-tools\"}");
        let r = provider
            .complete(CompletionRequest::new("weather?").with_system("Available skills:\n- a: b"))
            .await
            .unwrap();
        assert_eq!(r.text, "{\"skill\": null}");
        // with_skill rules never match complete()
        let r = provider
            .complete(CompletionRequest::new("A skill is available at x"))
            .await
            .unwrap();
        assert_eq!(r.text, "No idea.");
        assert!(r.latency_ms < 1000);

        let probe = provider.probe().await;
        assert!(probe.ok, "{}", probe.message);
        assert!(probe.message.contains("5 rule(s)"));
    }

    #[tokio::test]
    async fn slow_rule_honours_cancel_and_timeout() {
        let tmp = tempfile::tempdir().unwrap();
        let path = write_script(tmp.path(), "s", &script());
        let provider = ScriptedProvider::new(config("p1", &path)).unwrap();
        let mut task = ExecutionTask::new("slow", tmp.path().join("sb"));
        task.timeout = Duration::from_millis(100);
        let err = provider.execute(task).await.unwrap_err();
        assert!(matches!(err, HiveError::Timeout(_)), "{err}");

        let mut task = ExecutionTask::new("slow", tmp.path().join("sb"));
        let token = CancellationToken::new();
        task.cancel = token.clone();
        let handle = tokio::spawn(async move { provider.execute(task).await });
        tokio::time::sleep(Duration::from_millis(50)).await;
        token.cancel();
        let err = handle.await.unwrap().unwrap_err();
        assert!(matches!(err, HiveError::Cancelled), "{err}");
    }

    #[test]
    fn invalid_scripts_are_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let path = write_script(
            tmp.path(),
            "bad",
            &serde_json::json!({"rules": [{"match": "(unclosed", "output": "x"}]}),
        );
        assert!(ScriptedProvider::new(config("p", &path)).is_err());
        let path = write_script(tmp.path(), "nodefault", &serde_json::json!({"rules": []}));
        let p = ScriptedProvider::new(config("p", &path)).unwrap();
        assert!(p.script().select("x", None, false).is_err());
        let mut cfg = config("p", &path);
        cfg.command = None;
        assert!(ScriptedProvider::new(cfg).is_err());
    }

    #[tokio::test]
    async fn escaping_file_paths_are_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let path = write_script(
            tmp.path(),
            "esc",
            &serde_json::json!({"default": {"output": "x", "files": {"../evil.txt": "x"}}}),
        );
        let provider = ScriptedProvider::new(config("p", &path)).unwrap();
        let err = provider
            .execute(ExecutionTask::new("go", tmp.path().join("sb")))
            .await
            .unwrap_err();
        assert!(matches!(err, HiveError::Invalid(_)), "{err}");
        assert!(!tmp.path().join("evil.txt").exists());
    }
}
