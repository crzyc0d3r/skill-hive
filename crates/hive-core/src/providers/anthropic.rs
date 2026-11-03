//! Anthropic Messages API provider.
//!
//! `complete()` posts to `{base_url}/v1/messages` (default `https://api.anthropic.com`) with the
//! `x-api-key` and `anthropic-version: 2023-06-01` headers; the default model is
//! `claude-sonnet-4-5` and `max_tokens` is 4096 unless the request says otherwise.
//!
//! **Limitation:** this provider has no tools. `execute()` is a single completion with the
//! skill's `SKILL.md` as the system prompt (see [`super::execute_as_completion`]): the answer is
//! the output, no files are created and bundled skill files are not available to the model.
//! File-based checks therefore fail for this provider; output checks and the grader work.

use std::time::Instant;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::{
    CompletionRequest, CompletionResponse, ExecutionResult, ExecutionTask, Provider,
    build_http_client, execute_as_completion,
};
use crate::error::{HiveError, Result};
use crate::model::{ProviderConfig, ProviderProbe};

pub const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
pub const DEFAULT_MODEL: &str = "claude-sonnet-4-5";
pub const API_VERSION: &str = "2023-06-01";
pub const DEFAULT_MAX_TOKENS: u32 = 4096;

/// Normalise a base URL: no trailing slash, no trailing `/v1`.
pub fn normalize_base_url(base: Option<&str>) -> String {
    let base = base
        .map(str::trim)
        .filter(|b| !b.is_empty())
        .unwrap_or(DEFAULT_BASE_URL)
        .trim_end_matches('/');
    base.strip_suffix("/v1").unwrap_or(base).to_string()
}

/// Pull the human-readable message out of an API error body.
pub fn error_message(status: reqwest::StatusCode, body: &str) -> String {
    let detail = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| {
            v.get("error")
                .and_then(|e| e.get("message"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| super::process::tail(body, 300));
    format!("HTTP {}: {detail}", status.as_u16())
}

/// Anthropic Messages API provider.
pub struct AnthropicProvider {
    config: ProviderConfig,
    api_key: Option<String>,
    client: reqwest::Client,
    base: String,
}

impl AnthropicProvider {
    pub fn new(config: ProviderConfig, api_key: Option<String>) -> Result<Self> {
        let client = build_http_client(config.timeout_secs)?;
        let base = normalize_base_url(config.base_url.as_deref());
        Ok(Self {
            config,
            api_key,
            client,
            base,
        })
    }

    pub fn model(&self) -> &str {
        self.config
            .model
            .as_deref()
            .map(str::trim)
            .filter(|m| !m.is_empty())
            .unwrap_or(DEFAULT_MODEL)
    }

    fn key(&self) -> Result<&str> {
        self.api_key.as_deref().filter(|k| !k.trim().is_empty()).ok_or_else(|| {
            HiveError::provider(
                &self.config.name,
                "no API key: store one for this provider or set `api_key_env` (or ANTHROPIC_API_KEY)",
            )
        })
    }

    fn fail(&self, message: impl Into<String>) -> HiveError {
        HiveError::provider(&self.config.name, message)
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse> {
        let key = self.key()?.to_string();
        let started = Instant::now();
        let mut body = json!({
            "model": self.model(),
            "max_tokens": req.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
            "messages": [{"role": "user", "content": req.prompt}],
        });
        if let Some(system) = req.system.as_deref().filter(|s| !s.trim().is_empty()) {
            body["system"] = Value::String(system.to_string());
        }
        if let Some(t) = req.temperature {
            body["temperature"] = json!(t);
        }
        let request = self
            .client
            .post(format!("{}/v1/messages", self.base))
            .header("x-api-key", key)
            .header("anthropic-version", API_VERSION)
            .timeout(req.timeout_or_default())
            .json(&body)
            .send();
        let response = tokio::select! {
            r = request => r?,
            _ = req.cancel.cancelled() => return Err(HiveError::Cancelled),
        };
        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() {
            return Err(self.fail(error_message(status, &text)));
        }
        let v: Value = serde_json::from_str(&text)
            .map_err(|e| self.fail(format!("unexpected response: {e}")))?;
        let output = v
            .get("content")
            .and_then(Value::as_array)
            .map(|blocks| {
                blocks
                    .iter()
                    .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
                    .filter_map(|b| b.get("text").and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        let usage = v.get("usage").cloned().unwrap_or(Value::Null);
        let tokens = |k: &str| usage.get(k).and_then(Value::as_u64).unwrap_or(0);
        Ok(CompletionResponse {
            text: output,
            input_tokens: tokens("input_tokens")
                + tokens("cache_creation_input_tokens")
                + tokens("cache_read_input_tokens"),
            output_tokens: tokens("output_tokens"),
            latency_ms: started.elapsed().as_millis() as u64,
            model: v
                .get("model")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| Some(self.model().to_string())),
            cost_usd: None,
        })
    }

    async fn execute(&self, task: ExecutionTask) -> Result<ExecutionResult> {
        execute_as_completion(self, task).await
    }

    async fn probe(&self) -> ProviderProbe {
        let started = Instant::now();
        let checked_at = crate::util::now_ms();
        let key = match self.key() {
            Ok(k) => k.to_string(),
            Err(e) => {
                return ProviderProbe {
                    ok: false,
                    latency_ms: 0,
                    message: e.to_string(),
                    version: None,
                    checked_at,
                };
            }
        };
        let result = self
            .client
            .get(format!("{}/v1/models?limit=100", self.base))
            .header("x-api-key", key)
            .header("anthropic-version", API_VERSION)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await;
        let (ok, message) = match result {
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                if status.is_success() {
                    let count = serde_json::from_str::<Value>(&text)
                        .ok()
                        .and_then(|v| v.get("data").and_then(Value::as_array).map(Vec::len));
                    (
                        true,
                        match count {
                            Some(n) => format!("{n} models available; using {}", self.model()),
                            None => format!("endpoint answered; using {}", self.model()),
                        },
                    )
                } else {
                    (false, error_message(status, &text))
                }
            }
            Err(e) => (false, e.to_string()),
        };
        ProviderProbe {
            ok,
            latency_ms: started.elapsed().as_millis() as u64,
            message,
            version: None,
            checked_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_normalisation() {
        assert_eq!(normalize_base_url(None), "https://api.anthropic.com");
        assert_eq!(
            normalize_base_url(Some("https://proxy.example/v1/")),
            "https://proxy.example"
        );
        assert_eq!(
            normalize_base_url(Some("https://proxy.example/")),
            "https://proxy.example"
        );
    }

    #[test]
    fn error_messages_are_extracted() {
        let m = error_message(
            reqwest::StatusCode::UNAUTHORIZED,
            r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#,
        );
        assert_eq!(m, "HTTP 401: invalid x-api-key");
        assert!(error_message(reqwest::StatusCode::BAD_GATEWAY, "<html>").contains("502"));
    }

    #[tokio::test]
    async fn missing_key_is_a_clear_error() {
        let mut cfg = super::super::scripted::test_support::config("a", std::path::Path::new("/x"));
        cfg.kind = crate::model::ProviderKind::Anthropic;
        cfg.model = None;
        let p = AnthropicProvider::new(cfg, None).unwrap();
        assert_eq!(p.model(), DEFAULT_MODEL);
        let err = p.complete(CompletionRequest::new("hi")).await.unwrap_err();
        assert!(err.to_string().contains("no API key"), "{err}");
        let probe = p.probe().await;
        assert!(!probe.ok);
    }
}
