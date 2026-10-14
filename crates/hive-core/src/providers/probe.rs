//! Seven-stage connection probe. Each stage is recorded even when a later stage fails.

use std::time::Instant;

use super::cli_agent::{self, LocalCli};
use super::http::{AnthropicProvider, OpenAiProvider};
use super::store::ProviderConfig;
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
    stages.push(stage(2, "proxy", true, "offline fixture"));
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
    let started = Instant::now();
    let mut stages = Vec::new();
    stages.push(stage(
        1,
        "credential",
        true,
        credential_detail(config, secret),
    ));
    let proxy_detail = if config.proxy_mode == "none" {
        "proxy disabled"
    } else {
        "proxy inherited from settings"
    };
    stages.push(stage(2, "proxy", true, proxy_detail));

    let provider = build(config, secret);
    let models = match provider.list_models() {
        Ok(models) => models,
        Err(err) => {
            stages.push(stage(3, "models", false, err.message.clone()));
            return finish(
                stages,
                started,
                Some(err.class.as_str().to_string()),
                Vec::new(),
                false,
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
        return finish(stages, started, Some("unknown_model".into()), names, false);
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
        return finish(stages, started, Some("network".into()), names, false);
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

    let cli_detail = if config.kind == ProviderKind::LocalCli {
        match cli_agent::command_output(
            config.cli_program.as_deref().unwrap_or("claude"),
            &["--help"],
        ) {
            Ok(help) => format!("flags {}", help.chars().take(80).collect::<String>()),
            Err(err) => err.message,
        }
    } else {
        "CLI probe does not apply to this HTTP endpoint".into()
    };
    let cli_ok = config.kind != ProviderKind::LocalCli || cli_detail.starts_with("flags");
    stages.push(stage(6, "cli", cli_ok, cli_detail));
    stages.push(stage(7, "persist", true, "last_test fields ready"));
    let ok = stages.iter().all(|item| item.ok);
    finish(
        stages,
        started,
        if ok { None } else { Some("probe".into()) },
        names,
        supports_tools,
    )
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
