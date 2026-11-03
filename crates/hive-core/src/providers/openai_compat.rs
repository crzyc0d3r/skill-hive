//! OpenAI-compatible chat completions provider.
//!
//! `complete()` posts to `{base_url}/chat/completions` with `Authorization: Bearer <key>`. It
//! works for OpenAI (`https://api.openai.com/v1`, the default), OpenRouter
//! (`https://openrouter.ai/api/v1`), Ollama (`http://localhost:11434/v1`) and LM Studio
//! (`http://localhost:1234/v1`); local servers need no key. The default model is `gpt-4o-mini`;
//! set `model` for anything else (Ollama and LM Studio require it).
//!
//! **Limitation:** this provider has no tools. `execute()` is a single completion with the
//! skill's `SKILL.md` as the system prompt (see [`super::execute_as_completion`]): the answer is
//! the output and no files are created, so file-based checks fail for it; output checks and the
//! grader work.

use std::time::Instant;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::anthropic::error_message;
use super::{
    CompletionRequest, CompletionResponse, ExecutionResult, ExecutionTask, Provider,
    build_http_client, execute_as_completion,
};
use crate::error::{HiveError, Result};
use crate::model::{ProviderConfig, ProviderProbe};

pub const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
pub const DEFAULT_MODEL: &str = "gpt-4o-mini";
pub const DEFAULT_MAX_TOKENS: u32 = 4096;

/// Normalise a base URL: trimmed, no trailing slash.
pub fn normalize_base_url(base: Option<&str>) -> String {
    base.map(str::trim)
        .filter(|b| !b.is_empty())
        .unwrap_or(DEFAULT_BASE_URL)
        .trim_end_matches('/')
        .to_string()
}

/// Text of the first choice; content may be a string or an array of text parts.
pub fn first_choice_text(v: &Value) -> Option<String> {
    let message = v.get("choices")?.as_array()?.first()?.get("message")?;
    match message.get("content") {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Array(parts)) => Some(
            parts
                .iter()
                .filter_map(|p| p.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join(""),
        ),
        _ => message
            .get("refusal")
            .and_then(Value::as_str)
            .map(str::to_string),
    }
}

/// OpenAI-compatible provider.
pub struct OpenAiCompatProvider {
    config: ProviderConfig,
    api_key: Option<String>,
    client: reqwest::Client,
    base: String,
}

impl OpenAiCompatProvider {
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

    fn authed(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match self.api_key.as_deref().filter(|k| !k.trim().is_empty()) {
            Some(key) => req.bearer_auth(key.trim()),
            None => req,
        }
    }

    fn fail(&self, message: impl Into<String>) -> HiveError {
        HiveError::provider(&self.config.name, message)
    }

    async fn post(
        &self,
        body: &Value,
        req: &CompletionRequest,
    ) -> Result<(reqwest::StatusCode, String)> {
        let request = self
            .authed(self.client.post(format!("{}/chat/completions", self.base)))
            .timeout(req.timeout_or_default())
            .json(body)
            .send();
        let response = tokio::select! {
            r = request => r?,
            _ = req.cancel.cancelled() => return Err(HiveError::Cancelled),
        };
        let status = response.status();
        let text = response.text().await?;
        Ok((status, text))
    }
}

#[async_trait]
impl Provider for OpenAiCompatProvider {
    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse> {
        let started = Instant::now();
        let mut messages = Vec::new();
        if let Some(system) = req.system.as_deref().filter(|s| !s.trim().is_empty()) {
            messages.push(json!({"role": "system", "content": system}));
        }
        messages.push(json!({"role": "user", "content": req.prompt}));
        let max_tokens = req.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);
        let mut body = json!({
            "model": self.model(),
            "messages": messages,
            "max_tokens": max_tokens,
        });
        if let Some(t) = req.temperature {
            body["temperature"] = json!(t);
        }
        let (mut status, mut text) = self.post(&body, &req).await?;
        // Newer OpenAI models reject `max_tokens` in favour of `max_completion_tokens`.
        if status == reqwest::StatusCode::BAD_REQUEST && text.contains("max_completion_tokens") {
            if let Some(obj) = body.as_object_mut() {
                obj.remove("max_tokens");
                obj.insert("max_completion_tokens".into(), json!(max_tokens));
            }
            (status, text) = self.post(&body, &req).await?;
        }
        if !status.is_success() {
            return Err(self.fail(error_message(status, &text)));
        }
        let v: Value = serde_json::from_str(&text)
            .map_err(|e| self.fail(format!("unexpected response: {e}")))?;
        let output = first_choice_text(&v).ok_or_else(|| self.fail("response has no choices"))?;
        let usage = v.get("usage").cloned().unwrap_or(Value::Null);
        let tokens = |k: &str| usage.get(k).and_then(Value::as_u64).unwrap_or(0);
        Ok(CompletionResponse {
            text: output,
            input_tokens: tokens("prompt_tokens"),
            output_tokens: tokens("completion_tokens"),
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
        let result = self
            .authed(self.client.get(format!("{}/models", self.base)))
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await;
        let (ok, message) = match result {
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                if status.is_success() {
                    let ids: Vec<String> = serde_json::from_str::<Value>(&text)
                        .ok()
                        .and_then(|v| v.get("data").and_then(Value::as_array).cloned())
                        .unwrap_or_default()
                        .iter()
                        .filter_map(|m| m.get("id").and_then(Value::as_str).map(str::to_string))
                        .collect();
                    let model = self.model();
                    let note = if ids.is_empty() {
                        format!("endpoint answered; using {model}")
                    } else if ids.iter().any(|id| id == model) {
                        format!("{} models available; {model} is one of them", ids.len())
                    } else {
                        format!(
                            "{} models available but `{model}` is not listed (check the model name)",
                            ids.len()
                        )
                    };
                    (true, note)
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
            checked_at: crate::util::now_ms(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_and_choice_parsing() {
        assert_eq!(normalize_base_url(None), DEFAULT_BASE_URL);
        assert_eq!(
            normalize_base_url(Some(" http://localhost:11434/v1/ ")),
            "http://localhost:11434/v1"
        );
        let v = json!({"choices": [{"message": {"role": "assistant", "content": "hello"}}]});
        assert_eq!(first_choice_text(&v).as_deref(), Some("hello"));
        let v = json!({"choices": [{"message": {"content": [{"type": "text", "text": "a"}, {"type": "text", "text": "b"}]}}]});
        assert_eq!(first_choice_text(&v).as_deref(), Some("ab"));
        assert!(first_choice_text(&json!({"choices": []})).is_none());
    }

    #[test]
    fn model_default_and_key_optional() {
        let mut cfg = super::super::scripted::test_support::config("o", std::path::Path::new("/x"));
        cfg.kind = crate::model::ProviderKind::OpenaiCompatible;
        cfg.model = None;
        let p = OpenAiCompatProvider::new(cfg, None).unwrap();
        assert_eq!(p.model(), DEFAULT_MODEL);
        assert!(!p.is_agentic());
    }
}
