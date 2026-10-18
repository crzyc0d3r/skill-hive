//! Seven-stage connection probe. Each stage is recorded even when a later stage fails.

use std::time::Instant;

use rusqlite::Connection;

use super::cli_agent::{self, LocalCli};
use super::http::{AnthropicProvider, OpenAiProvider};
use super::store::{self, ProviderConfig};
use super::types::*;

pub use super::types::ProbeStage;

pub fn probe_offline(config: &ProviderConfig) -> ProbeReport {
    let started = Instant::now();
    let secret = SecretString::new(if config.auth_mode == "none" {
        ""
    } else {
        "offline-secret"
    });
    let mut stages = Vec::new();
    stages.push(stage(
        1,
        "credential",
        true,
        credential_detail(config, &secret),
    ));
    let (proxy_ok, proxy_detail) = proxy_stage(&config.proxy_mode);
    stages.push(stage(2, "proxy", proxy_ok, proxy_detail));
    let mut provider = super::scripted::ScriptedProvider::new(config.id.clone());
    if let Some(model) = &config.default_model {
        provider.model = model.clone();
    }
    let models = provider.list_models().unwrap_or_default();
    let names: Vec<String> = models.iter().map(|model| model.id.clone()).collect();
    stages.push(stage(
        3,
        "models",
        !names.is_empty(),
        format!("{} models", names.len()),
    ));
    let cancel = CancellationToken::new();
    let mut sink = VecSink::default();
    let completion = provider
        .complete(&tiny_request(config, false), &mut sink, &cancel)
        .is_ok();
    stages.push(stage(4, "completion", completion, "offline completion"));
    let mut tool_sink = VecSink::default();
    let tools = provider.complete(&tiny_request(config, true), &mut tool_sink, &cancel);
    let supports_tools = tools
        .as_ref()
        .map(|response| !response.tool_uses.is_empty())
        .unwrap_or(false);
    stages.push(stage(5, "tools", supports_tools, "offline tools"));
    let cli_ok = config.kind != ProviderKind::LocalCli || config.cli_program.is_some();
    stages.push(stage(6, "cli", cli_ok, "offline cli"));
    stages.push(stage(7, "persist", true, "offline persist"));
    finish(stages, started, None, names, supports_tools)
}

pub fn probe_provider(
    config: &ProviderConfig,
    secret: &SecretString,
    cancel: &CancellationToken,
) -> ProbeReport {
    probe_inner(config, secret, cancel, None)
}

pub fn probe_and_persist(
    config: &ProviderConfig,
    secret: &SecretString,
    cancel: &CancellationToken,
    conn: &Connection,
) -> ProbeReport {
    probe_inner(config, secret, cancel, Some(conn))
}

fn probe_inner(
    config: &ProviderConfig,
    secret: &SecretString,
    cancel: &CancellationToken,
    conn: Option<&Connection>,
) -> ProbeReport {
    let started = Instant::now();
    let mut stages = Vec::new();
    stages.push(stage(
        1,
        "credential",
        true,
        credential_detail(config, secret),
    ));
    let (proxy_ok, proxy_detail) = proxy_stage(&config.proxy_mode);
    stages.push(stage(2, "proxy", proxy_ok, proxy_detail));
    if !proxy_ok {
        return persist(
            config,
            conn,
            finish(stages, started, Some("proxy".into()), Vec::new(), false),
        );
    }

    let provider = build(config, secret);
    let models = match provider.list_models() {
        Ok(models) => models,
        Err(err) => {
            stages.push(stage(3, "models", false, err.message.clone()));
            return persist(
                config,
                conn,
                finish(
                    stages,
                    started,
                    Some(err.class.as_str().to_string()),
                    Vec::new(),
                    false,
                ),
            );
        }
    };
    let names: Vec<String> = models.iter().map(|model| model.id.clone()).collect();
    let model_ok = config
        .default_model
        .as_ref()
        .map(|model| names.iter().any(|name| name == model))
        .unwrap_or(true);
    let model_detail = if model_ok {
        format!("{} models", names.len())
    } else {
        format!(
            "default model {} is missing",
            config.default_model.clone().unwrap_or_default()
        )
    };
    stages.push(stage(3, "models", model_ok, model_detail));
    if !model_ok {
        return persist(
            config,
            conn,
            finish(stages, started, Some("unknown_model".into()), names, false),
        );
    }

    let mut sink = VecSink::default();
    let completion = provider.complete(&tiny_request(config, false), &mut sink, cancel);
    match &completion {
        Ok(response) => stages.push(stage(
            4,
            "completion",
            true,
            format!("stop {}", stop_label(&response.stop)),
        )),
        Err(err) => stages.push(stage(4, "completion", false, err.message.clone())),
    }
    if completion.is_err() {
        return persist(
            config,
            conn,
            finish(stages, started, Some("network".into()), names, false),
        );
    }

    let mut tool_sink = VecSink::default();
    let tool_request = tiny_request(config, true);
    let tools = provider.complete(&tool_request, &mut tool_sink, cancel);
    let supports_tools = tools
        .as_ref()
        .map(|response| {
            !response.tool_uses.is_empty() || matches!(response.stop, StopReason::ToolUse)
        })
        .unwrap_or(false);
    match &tools {
        Ok(_) => {
            let detail = if supports_tools {
                "tool call round-tripped"
            } else {
                "no tool call returned"
            };
            stages.push(stage(
                5,
                "tools",
                supports_tools || !config.supports_tools,
                detail,
            ));
        }
        Err(err) => stages.push(stage(5, "tools", false, err.message.clone())),
    }

    let (cli_ok, cli_detail) = cli_stage(config, cancel);
    stages.push(stage(6, "cli", cli_ok, cli_detail));
    let wrote = conn.is_some();
    stages.push(stage(
        7,
        "persist",
        true,
        if wrote {
            "wrote last_test_at, last_test_ok, latency"
        } else {
            "last_test fields ready"
        },
    ));
    let ok = stages.iter().all(|item| item.ok);
    persist(
        config,
        conn,
        finish(
            stages,
            started,
            if ok { None } else { Some("probe".into()) },
            names,
            supports_tools,
        ),
    )
}

fn proxy_stage(mode: &str) -> (bool, String) {
    match mode {
        "none" => (true, "proxy disabled".into()),
        "inherit" => (true, "proxy inherited from settings".into()),
        other => (false, format!("proxy mode {other} could not be resolved")),
    }
}

fn cli_stage(config: &ProviderConfig, cancel: &CancellationToken) -> (bool, String) {
    if config.kind != ProviderKind::LocalCli {
        return (
            true,
            "CLI probe does not apply to this HTTP endpoint".into(),
        );
    }
    let program = config
        .cli_program
        .clone()
        .unwrap_or_else(|| "claude".into());
    let codex = config.flavor == Some(Flavor::CodexCli)
        || program.rsplit(['/', '\\']).next().unwrap_or(&program) == "codex";
    if codex {
        return match cli_agent::command_output(&program, &["--help"]) {
            Ok(help) => (
                true,
                format!("flags {}", help.chars().take(80).collect::<String>()),
            ),
            Err(err) => (false, err.message),
        };
    }
    let sandbox = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(err) => return (false, err.to_string()),
    };
    match cli_agent::run_claude(&program, "Reply OK", None, sandbox.path(), cancel, false) {
        Ok(output) => {
            let joined = output.lines.join("\n");
            if joined.contains("OK") {
                (
                    true,
                    format!(
                        "claude -p reply {}",
                        joined.chars().take(80).collect::<String>()
                    ),
                )
            } else {
                (false, "claude -p produced no Reply OK".into())
            }
        }
        Err(err) => (false, err.message),
    }
}

fn persist(config: &ProviderConfig, conn: Option<&Connection>, report: ProbeReport) -> ProbeReport {
    if let Some(conn) = conn {
        let _ = store::set_last_test(
            conn,
            &config.id,
            report.ok,
            report.latency_ms,
            report.error_class.as_deref(),
        );
    }
    report
}

fn credential_detail(config: &ProviderConfig, secret: &SecretString) -> String {
    match config.auth_mode.as_str() {
        "env" => format!(
            "environment {}",
            config.auth_env_var.as_deref().unwrap_or("")
        ),
        "none" => "no credential".into(),
        _ => {
            if secret.is_empty() {
                "keyring credential missing".into()
            } else {
                "keyring credential resolved".into()
            }
        }
    }
}

fn build(config: &ProviderConfig, secret: &SecretString) -> Box<dyn ChatProvider> {
    match config.kind {
        ProviderKind::Anthropic => Box::new(AnthropicProvider::new(
            config.id.clone(),
            config
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.anthropic.com".into()),
            secret.clone(),
        )),
        ProviderKind::OpenaiCompat => {
            let flavor = config.flavor.unwrap_or(Flavor::Custom);
            let base = config
                .base_url
                .clone()
                .or_else(|| flavor.default_base_url().map(str::to_string))
                .unwrap_or_else(|| "http://127.0.0.1:9/v1".into());
            Box::new(OpenAiProvider::new(
                config.id.clone(),
                base,
                flavor,
                secret.clone(),
            ))
        }
        ProviderKind::LocalCli => Box::new(LocalCli {
            id: config.id.clone(),
            program: config
                .cli_program
                .clone()
                .unwrap_or_else(|| "claude".into()),
            flavor: config.flavor.unwrap_or(Flavor::ClaudeCli),
            model: config.default_model.clone(),
            sandbox: None,
        }),
    }
}

fn tiny_request(config: &ProviderConfig, with_tool: bool) -> ChatRequest {
    let mut request = ChatRequest {
        model: config
            .default_model
            .clone()
            .unwrap_or_else(|| "fixture-model".into()),
        system: None,
        messages: vec![Message {
            role: "user".into(),
            content: vec![ContentBlock::Text("Reply with the single word OK".into())],
        }],
        tools: Vec::new(),
        max_tokens: 16,
        effort: None,
        thinking: ThinkingMode::Off,
        stream: true,
        metadata: RequestMetadata {
            purpose: "test_connection".into(),
            ref_id: Some(config.id.clone()),
            provider_id: Some(config.id.clone()),
        },
    };
    if with_tool {
        request.tools.push(ToolDef {
            name: "echo".into(),
            description: "Echo a word".into(),
            input_schema: serde_json::json!({"type": "object", "properties": {"word": {"type": "string"}}}),
        });
    }
    request
}

fn stage(index: u8, name: &str, ok: bool, detail: impl Into<String>) -> ProbeStage {
    ProbeStage {
        index,
        name: name.into(),
        ok,
        detail: detail.into(),
    }
}

fn finish(
    stages: Vec<ProbeStage>,
    started: Instant,
    error_class: Option<String>,
    models: Vec<String>,
    supports_tools: bool,
) -> ProbeReport {
    let ok = stages.len() == 7 && stages.iter().all(|item| item.ok);
    ProbeReport {
        ok,
        stages,
        latency_ms: u64::try_from(started.elapsed().as_millis())
            .unwrap_or(1)
            .max(1),
        error_class,
        models,
        supports_tools,
    }
}

fn stop_label(stop: &StopReason) -> &'static str {
    match stop {
        StopReason::EndTurn => "end_turn",
        StopReason::ToolUse => "tool_use",
        StopReason::MaxTokens => "max_tokens",
        StopReason::StopSequence => "stop_sequence",
        StopReason::Refusal { .. } => "refusal",
        StopReason::PauseTurn => "pause_turn",
        StopReason::Other(_) => "other",
    }
}
