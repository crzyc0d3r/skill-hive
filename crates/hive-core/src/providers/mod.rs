//! Model providers: CLI agents and HTTP APIs used to run evaluations, grade results and
//! re-rank recommendations.
//!
//! A [`Provider`] has two capabilities:
//!
//! * [`Provider::complete`]: system prompt + user prompt in, text out, with token usage and
//!   latency. The grader, the trigger evaluation and recommendation re-ranking use it.
//! * [`Provider::execute`]: run an agentic task inside a sandbox directory (`cwd`), optionally
//!   with a skill directory made available, within a timeout and honouring a cancellation
//!   token. It reports the final output, a transcript written inside the sandbox, tool-call
//!   and token counts, duration, cost when known and the files the run created.
//!
//! Implementations live one per file: [`claude_code`] (`claude -p`), [`codex`] (`codex exec`),
//! [`anthropic`] (Messages API), [`openai_compat`] (any chat-completions endpoint) and the
//! offline [`scripted`] provider that tests and demos use. [`registry::build_provider`] turns a
//! stored [`ProviderConfig`] into a provider and [`registry::probe`] checks that it answers.
//!
//! The `impl Hive` block at the bottom is the facade the CLI and the desktop app call.

use std::path::PathBuf;
use std::process::Command as StdCommand;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::error::{HiveError, IoContext, Result};
use crate::hive::Hive;
use crate::model::{
    DetectedCli, ProviderConfig, ProviderInput, ProviderKind, ProviderProbe, ProviderRole,
};
use crate::settings;

pub mod anthropic;
pub mod claude_code;
pub mod codex;
pub mod openai_compat;
pub mod process;
pub mod registry;
pub mod sandbox;
pub mod scripted;
pub mod trace;

pub use anthropic::AnthropicProvider;
pub use claude_code::ClaudeCodeProvider;
pub use codex::CodexProvider;
pub use openai_compat::OpenAiCompatProvider;
pub use registry::{build_provider, probe, resolve_api_key, secret_name};
pub use scripted::{Script, ScriptRule, ScriptedProvider};

/// Timeout of a single completion when the request does not set one.
pub const DEFAULT_COMPLETION_TIMEOUT: Duration = Duration::from_secs(300);
/// Per-task timeout stored on a new provider.
pub const DEFAULT_PROVIDER_TIMEOUT_SECS: u32 = 600;
/// Parallel executions a new provider allows.
pub const DEFAULT_PROVIDER_CONCURRENCY: u32 = 2;
/// Upper bound for a provider's `max_concurrency`.
pub const MAX_PROVIDER_CONCURRENCY: u32 = 16;

/// One text completion: an optional system prompt and a user prompt.
#[derive(Debug, Clone, Default)]
pub struct CompletionRequest {
    pub system: Option<String>,
    pub prompt: String,
    /// Maximum output tokens; providers fall back to their own default.
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    /// Wall-clock limit; [`DEFAULT_COMPLETION_TIMEOUT`] when `None`.
    pub timeout: Option<Duration>,
    /// Cancelling the token aborts the request with [`HiveError::Cancelled`].
    pub cancel: CancellationToken,
}

impl CompletionRequest {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            ..Default::default()
        }
    }

    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    pub fn with_cancel(mut self, cancel: CancellationToken) -> Self {
        self.cancel = cancel;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// The effective timeout.
    pub fn timeout_or_default(&self) -> Duration {
        self.timeout.unwrap_or(DEFAULT_COMPLETION_TIMEOUT)
    }
}

/// Result of a completion.
#[derive(Debug, Clone, Default)]
pub struct CompletionResponse {
    pub text: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub latency_ms: u64,
    /// Model that answered, when the provider reports it.
    pub model: Option<String>,
    pub cost_usd: Option<f64>,
}

/// An agentic task to run inside a sandbox directory.
#[derive(Debug, Clone)]
pub struct ExecutionTask {
    /// The task prompt (the eval case prompt).
    pub prompt: String,
    /// Sandbox working directory; created when missing. Input files are already in place.
    pub cwd: PathBuf,
    /// Skill directory to make available to the agent; `None` for the baseline arm.
    pub skill_dir: Option<PathBuf>,
    /// Wall-clock limit for the whole task.
    pub timeout: Duration,
    /// Cancelling the token kills the agent and returns [`HiveError::Cancelled`].
    pub cancel: CancellationToken,
    /// Short label for logs and transcripts, e.g. `case-1/with_skill/2`.
    pub label: String,
}

impl ExecutionTask {
    pub fn new(prompt: impl Into<String>, cwd: impl Into<PathBuf>) -> Self {
        Self {
            prompt: prompt.into(),
            cwd: cwd.into(),
            skill_dir: None,
            timeout: Duration::from_secs(DEFAULT_PROVIDER_TIMEOUT_SECS as u64),
            cancel: CancellationToken::new(),
            label: String::new(),
        }
    }
}

/// What an execution produced.
#[derive(Debug, Clone, Default)]
pub struct ExecutionResult {
    /// Final assistant output.
    pub output: String,
    /// Transcript written inside the sandbox (`.hive/transcript.jsonl`), when available.
    pub transcript_path: Option<PathBuf>,
    pub tool_calls: u32,
    /// Input tokens including cache reads/writes when the provider reports them.
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub duration_ms: u64,
    pub cost_usd: Option<f64>,
    /// Files created under the sandbox, relative paths with `/` separators, excluding
    /// directories Hive or the agent tooling manage (`.hive`, `.claude`, `.codex`, …).
    pub files_created: Vec<String>,
    pub model: Option<String>,
    /// What the agent did, when the provider's event stream allows reconstructing it.
    pub trace: Option<crate::model::ExecutionTrace>,
}

/// A model provider. Implementations are cheap to build and safe to share across tasks.
#[async_trait]
pub trait Provider: Send + Sync {
    /// The configuration this provider was built from.
    fn config(&self) -> &ProviderConfig;

    fn id(&self) -> &str {
        &self.config().id
    }

    fn name(&self) -> &str {
        &self.config().name
    }

    fn kind(&self) -> ProviderKind {
        self.config().kind
    }

    /// True when [`Provider::execute`] runs with real tools; false when it is a single
    /// completion (HTTP API providers).
    fn is_agentic(&self) -> bool {
        self.kind().is_agentic()
    }

    /// System + user prompt to text.
    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse>;

    /// Run an agentic task in the sandbox. Returns [`HiveError::Timeout`] when the task
    /// exceeded `task.timeout` and [`HiveError::Cancelled`] when the token was cancelled.
    async fn execute(&self, task: ExecutionTask) -> Result<ExecutionResult>;

    /// Check that the provider answers (CLI version, model list, script file).
    async fn probe(&self) -> ProviderProbe;
}

/// System prompt used by API providers when no skill is available.
pub const NO_SKILL_SYSTEM_PROMPT: &str = "You are a capable assistant completing a task for the user. Do the work as completely as you can in a single reply and end with the final result.";

/// Execute a task as one completion: the skill's `SKILL.md` becomes the system prompt and the
/// reply is the output. No tools run and no files are created, so file-based checks fail for
/// these providers; output-based checks and the grader still work. HTTP API providers use this.
pub async fn execute_as_completion(
    provider: &dyn Provider,
    task: ExecutionTask,
) -> Result<ExecutionResult> {
    std::fs::create_dir_all(&task.cwd).at(&task.cwd)?;
    let started = Instant::now();
    let system = match &task.skill_dir {
        Some(dir) => sandbox::skill_system_prompt(dir)?,
        None => NO_SKILL_SYSTEM_PROMPT.to_string(),
    };
    let req = CompletionRequest {
        system: Some(system.clone()),
        prompt: task.prompt.clone(),
        max_tokens: None,
        temperature: None,
        timeout: Some(task.timeout),
        cancel: task.cancel.clone(),
    };
    let resp = provider.complete(req).await?;
    let mut trace = trace::TraceBuilder::new(trace::Dialect::ClaudeCode, &task.cwd, None);
    trace.turn();
    trace.text(&resp.text, Some(resp.latency_ms));
    trace.set_model_time(resp.latency_ms);
    trace.set_skill_in_context(task.skill_dir.is_some());
    let transcript = sandbox::write_transcript(
        &task.cwd,
        &[
            serde_json::json!({"type": "system", "text": system}),
            serde_json::json!({"type": "user", "text": task.prompt}),
            serde_json::json!({
                "type": "assistant",
                "text": resp.text,
                "model": resp.model,
                "usage": {"input_tokens": resp.input_tokens, "output_tokens": resp.output_tokens},
                "latency_ms": resp.latency_ms,
            }),
        ],
    )?;
    Ok(ExecutionResult {
        output: resp.text,
        transcript_path: Some(transcript),
        tool_calls: 0,
        input_tokens: resp.input_tokens,
        output_tokens: resp.output_tokens,
        duration_ms: started.elapsed().as_millis() as u64,
        cost_usd: resp.cost_usd,
        files_created: Vec::new(),
        model: resp.model,
        trace: Some(trace.finish(resp.output_tokens)),
    })
}

/// A reqwest client for an HTTP provider: long timeout (LLM replies are slow), Hive user agent.
pub(crate) fn build_http_client(timeout_secs: u32) -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(format!("skill-hive/{}", crate::version()))
        .timeout(Duration::from_secs(timeout_secs.max(30) as u64))
        .connect_timeout(Duration::from_secs(20))
        .build()?)
}

/// Find agent CLIs on PATH and read their versions.
pub fn detect_clis() -> Vec<DetectedCli> {
    let mut out = Vec::new();
    for (kind, command) in [
        (ProviderKind::ClaudeCode, "claude"),
        (ProviderKind::Codex, "codex"),
    ] {
        if let Ok(path) = which::which(command) {
            let version = probe_version(&path);
            out.push(DetectedCli {
                kind,
                command: command.to_string(),
                path: path.to_string_lossy().to_string(),
                version,
            });
        }
    }
    out
}

/// How long `<cli> --version` may take before it is killed and the version left unknown.
const VERSION_PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// Run `<path> --version` with a bounded wait. The dashboard calls this on every load, so a CLI
/// that hangs without a terminal (first-run checks, a broken npm shim) must not hang the app,
/// and on Windows the child must not open a console window over it.
fn probe_version(path: &std::path::Path) -> Option<String> {
    let mut cmd = StdCommand::new(path);
    cmd.arg("--version")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    crate::util::hide_console_std(&mut cmd);
    let mut child = cmd.spawn().ok()?;
    let deadline = Instant::now() + VERSION_PROBE_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child.wait_with_output().ok()?;
                if !status.success() {
                    return None;
                }
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return (!version.is_empty()).then_some(version);
            }
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(50));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

fn all_roles() -> Vec<ProviderRole> {
    vec![
        ProviderRole::Executor,
        ProviderRole::Grader,
        ProviderRole::Recommender,
    ]
}

fn clean(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

impl Hive {
    /// Every configured provider, with `has_secret` reflecting the secret store.
    pub fn list_providers(&self) -> Result<Vec<ProviderConfig>> {
        let mut out = self.db.list_providers()?;
        for p in &mut out {
            p.has_secret = self.secrets.has(&secret_name(&p.id));
        }
        Ok(out)
    }

    /// One provider by id, with `has_secret` filled in.
    pub fn get_provider(&self, id: &str) -> Result<ProviderConfig> {
        let mut config = self.db.require_provider(id)?;
        config.has_secret = self.secrets.has(&secret_name(id));
        Ok(config)
    }

    /// Create (`id == None`) or update a provider. `input.api_key` is stored in the secret
    /// store when `Some(non-empty)`, cleared when `Some("")` and left alone when `None`.
    pub fn save_provider(&self, id: Option<&str>, input: &ProviderInput) -> Result<ProviderConfig> {
        let now = crate::util::now_ms();
        let name = input.name.trim();
        if name.is_empty() {
            return Err(HiveError::invalid("provider name is required"));
        }
        let mut config = match id {
            Some(id) => self.db.require_provider(id)?,
            None => ProviderConfig {
                id: crate::util::new_id(),
                name: String::new(),
                kind: input.kind,
                model: None,
                base_url: None,
                command: None,
                args: Vec::new(),
                env: Default::default(),
                api_key_env: None,
                has_secret: false,
                roles: Vec::new(),
                timeout_secs: DEFAULT_PROVIDER_TIMEOUT_SECS,
                max_concurrency: DEFAULT_PROVIDER_CONCURRENCY,
                last_probe: None,
                created_at: now,
                updated_at: now,
            },
        };
        config.name = name.to_string();
        config.kind = input.kind;
        config.model = clean(&input.model);
        config.base_url = clean(&input.base_url);
        config.command = clean(&input.command);
        config.args = input
            .args
            .iter()
            .map(|a| a.trim().to_string())
            .filter(|a| !a.is_empty())
            .collect();
        config.env = input
            .env
            .iter()
            .filter(|(k, _)| !k.trim().is_empty())
            .map(|(k, v)| (k.trim().to_string(), v.clone()))
            .collect();
        config.api_key_env = clean(&input.api_key_env);
        let mut roles = if input.roles.is_empty() {
            all_roles()
        } else {
            input.roles.clone()
        };
        roles.dedup();
        config.roles = roles;
        if let Some(t) = input.timeout_secs {
            config.timeout_secs = t.max(10);
        }
        if let Some(c) = input.max_concurrency {
            config.max_concurrency = c.clamp(1, MAX_PROVIDER_CONCURRENCY);
        }
        if config.kind == ProviderKind::Scripted && config.command.is_none() {
            return Err(HiveError::invalid(
                "scripted providers need the script path in `command`",
            ));
        }
        if let Some(url) = &config.base_url {
            url::Url::parse(url)
                .map_err(|e| HiveError::invalid(format!("base URL `{url}` is not valid: {e}")))?;
        }
        config.updated_at = now;
        if let Some(key) = &input.api_key {
            self.secrets.set(&secret_name(&config.id), key.trim())?;
        }
        config.has_secret = self.secrets.has(&secret_name(&config.id));
        // Kind or endpoint changes invalidate the last probe.
        if id.is_some() {
            config.last_probe = None;
        }
        self.db.save_provider(&config)?;
        self.db.audit(
            "provider.save",
            &serde_json::json!({"id": config.id, "name": config.name, "kind": config.kind}),
        )?;
        Ok(config)
    }

    /// Delete a provider, its secret and any settings default that pointed at it.
    pub fn delete_provider(&self, id: &str) -> Result<()> {
        self.db.require_provider(id)?;
        self.db.delete_provider(id)?;
        self.secrets.delete(&secret_name(id))?;
        let s = self.settings()?;
        if s.default_executor_id.as_deref() == Some(id) {
            self.db.delete_setting(settings::keys::EXECUTOR)?;
        }
        if s.default_grader_id.as_deref() == Some(id) {
            self.db.delete_setting(settings::keys::GRADER)?;
        }
        self.db
            .audit("provider.delete", &serde_json::json!({"id": id}))?;
        Ok(())
    }

    /// Probe a provider and persist the result as `last_probe`.
    pub async fn probe_provider(&self, id: &str) -> Result<ProviderProbe> {
        let mut config = self.db.require_provider(id)?;
        let probe = registry::probe(&config, &self.secrets).await;
        config.last_probe = Some(probe.clone());
        config.updated_at = crate::util::now_ms();
        self.db.save_provider(&config)?;
        Ok(probe)
    }

    /// First run: when no provider is configured, create one per detected CLI
    /// (Claude Code → executor, grader and recommender; Codex → executor). Returns the list.
    pub fn ensure_default_providers(&self) -> Result<Vec<ProviderConfig>> {
        let existing = self.list_providers()?;
        if !existing.is_empty() {
            return Ok(existing);
        }
        for cli in detect_clis() {
            let (name, roles) = match cli.kind {
                ProviderKind::ClaudeCode => ("Claude Code", all_roles()),
                ProviderKind::Codex => ("Codex", vec![ProviderRole::Executor]),
                _ => continue,
            };
            self.save_provider(
                None,
                &ProviderInput {
                    name: name.to_string(),
                    kind: cli.kind,
                    model: None,
                    base_url: None,
                    command: Some(cli.command.clone()),
                    args: Vec::new(),
                    env: Default::default(),
                    api_key_env: None,
                    api_key: None,
                    roles,
                    timeout_secs: None,
                    max_concurrency: None,
                },
            )?;
        }
        self.list_providers()
    }

    /// Build a ready-to-use provider from its stored configuration.
    pub fn provider(&self, id: &str) -> Result<Arc<dyn Provider>> {
        let config = self.db.require_provider(id)?;
        registry::build_provider(&config, &self.secrets)
    }

    /// The provider to use for a role: the settings default (`default_executor_id`,
    /// `default_grader_id`) when it exists, else the first provider that has the role.
    pub fn default_provider_for(&self, role: ProviderRole) -> Result<Option<ProviderConfig>> {
        let s = self.settings()?;
        let preferred = match role {
            ProviderRole::Executor => s.default_executor_id,
            ProviderRole::Grader => s.default_grader_id,
            ProviderRole::Recommender => None,
        };
        let providers = self.list_providers()?;
        if let Some(pid) = preferred {
            if let Some(p) = providers.iter().find(|p| p.id == pid) {
                return Ok(Some(p.clone()));
            }
        }
        Ok(providers.into_iter().find(|p| p.roles.contains(&role)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SettingsPatch;

    fn input(name: &str, kind: ProviderKind) -> ProviderInput {
        ProviderInput {
            name: name.into(),
            kind,
            model: None,
            base_url: None,
            command: Some("/tmp/script.json".into()),
            args: vec![],
            env: Default::default(),
            api_key_env: None,
            api_key: None,
            roles: vec![],
            timeout_secs: None,
            max_concurrency: None,
        }
    }

    #[test]
    fn save_list_secret_default_delete() {
        let tmp = tempfile::tempdir().unwrap();
        let hive = Hive::open_temp(tmp.path()).unwrap();
        assert!(hive.list_providers().unwrap().is_empty());

        let mut i = input("Judge", ProviderKind::Anthropic);
        i.api_key = Some("sk-test".into());
        i.roles = vec![ProviderRole::Grader];
        let judge = hive.save_provider(None, &i).unwrap();
        assert!(judge.has_secret);
        assert_eq!(judge.roles, vec![ProviderRole::Grader]);
        assert_eq!(judge.timeout_secs, DEFAULT_PROVIDER_TIMEOUT_SECS);
        assert_eq!(
            hive.secrets
                .get(&secret_name(&judge.id))
                .unwrap()
                .as_deref(),
            Some("sk-test")
        );

        let runner = hive
            .save_provider(None, &input("Runner", ProviderKind::Scripted))
            .unwrap();
        assert_eq!(runner.roles.len(), 3);
        assert!(!runner.has_secret);

        // No default set: first provider with the role wins.
        let exec = hive
            .default_provider_for(ProviderRole::Executor)
            .unwrap()
            .unwrap();
        assert_eq!(exec.id, runner.id);
        let grader = hive
            .default_provider_for(ProviderRole::Grader)
            .unwrap()
            .unwrap();
        assert_eq!(grader.id, judge.id);
        // Settings default takes precedence.
        hive.update_settings(&SettingsPatch {
            default_grader_id: Some(Some(runner.id.clone())),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(
            hive.default_provider_for(ProviderRole::Grader)
                .unwrap()
                .unwrap()
                .id,
            runner.id
        );

        // Update: clear the secret with "", keep it with None.
        let mut upd = input("Judge 2", ProviderKind::Anthropic);
        upd.api_key = None;
        upd.timeout_secs = Some(5);
        let judge = hive.save_provider(Some(&judge.id), &upd).unwrap();
        assert!(judge.has_secret);
        assert_eq!(judge.name, "Judge 2");
        assert_eq!(judge.timeout_secs, 10);
        upd.api_key = Some(String::new());
        let judge = hive.save_provider(Some(&judge.id), &upd).unwrap();
        assert!(!judge.has_secret);

        // Scripted without a command is refused; bad URLs are refused.
        let mut bad = input("x", ProviderKind::Scripted);
        bad.command = None;
        assert!(hive.save_provider(None, &bad).is_err());
        let mut bad = input("x", ProviderKind::OpenaiCompatible);
        bad.base_url = Some("not a url".into());
        assert!(hive.save_provider(None, &bad).is_err());

        hive.delete_provider(&runner.id).unwrap();
        assert!(hive.get_provider(&runner.id).is_err());
        assert!(hive.settings().unwrap().default_grader_id.is_none());
        assert_eq!(hive.list_providers().unwrap().len(), 1);
        assert!(hive.delete_provider("missing").is_err());
    }

    #[test]
    fn ensure_defaults_is_idempotent_once_configured() {
        let tmp = tempfile::tempdir().unwrap();
        let hive = Hive::open_temp(tmp.path()).unwrap();
        hive.save_provider(None, &input("Runner", ProviderKind::Scripted))
            .unwrap();
        let list = hive.ensure_default_providers().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Runner");
    }

    #[test]
    fn build_provider_for_every_kind() {
        let tmp = tempfile::tempdir().unwrap();
        let hive = Hive::open_temp(tmp.path()).unwrap();
        let script = tmp.path().join("script.json");
        std::fs::write(&script, r#"{"default": {"output": "hi"}}"#).unwrap();
        for kind in [
            ProviderKind::ClaudeCode,
            ProviderKind::Codex,
            ProviderKind::Anthropic,
            ProviderKind::OpenaiCompatible,
            ProviderKind::Scripted,
        ] {
            let mut i = input(&format!("{kind:?}"), kind);
            i.command = Some(script.to_string_lossy().to_string());
            let cfg = hive.save_provider(None, &i).unwrap();
            let provider = hive.provider(&cfg.id).unwrap();
            assert_eq!(provider.kind(), kind);
            assert_eq!(provider.id(), cfg.id);
            assert_eq!(provider.is_agentic(), kind.is_agentic());
        }
    }
}
