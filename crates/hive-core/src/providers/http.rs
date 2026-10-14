//! Anthropic Messages and OpenAI-compatible chat adapters. HTTP is `ureq` so CI does not
//! need a Tokio runtime. Tests point `base_url` at a local server.

use std::io::Read;
use std::time::Duration;

use serde_json::{json, Value};

use super::types::*;

pub struct AnthropicProvider {
    pub id: String,
    pub base_url: String,
    pub api_key: SecretString,
    pub timeout: Duration,
    pub api_version: String,
}

pub struct OpenAiProvider {
    pub id: String,
    pub base_url: String,
    pub flavor: Flavor,
    pub api_key: SecretString,
    pub timeout: Duration,
    pub api_version: Option<String>,
    pub extra_headers: Vec<(String, String)>,
}

impl AnthropicProvider {
    pub fn new(id: impl Into<String>, base_url: impl Into<String>, api_key: SecretString) -> Self {
        Self {
            id: id.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key,
            timeout: Duration::from_secs(30),
            api_version: "2023-06-01".into(),
        }
    }
}

impl OpenAiProvider {
    pub fn new(
        id: impl Into<String>,
        base_url: impl Into<String>,
        flavor: Flavor,
        api_key: SecretString,
    ) -> Self {
        Self {
            id: id.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            flavor,
            api_key,
            timeout: Duration::from_secs(30),
            api_version: None,
            extra_headers: Vec::new(),
        }
    }
}

impl ChatProvider for AnthropicProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind_name(&self) -> &'static str {
        "anthropic"
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            tools: true,
            streaming: true,
            system_prompt: true,
        }
    }

    fn complete(
        &self,
        request: &ChatRequest,
        sink: &mut dyn StreamSink,
        cancel: &CancellationToken,
    ) -> Result<ChatResponse, ProviderError> {
        if cancel.is_cancelled() {
            return Err(ProviderError::new(ErrorClass::Cancelled, "cancelled"));
        }
        let body = anthropic_body(request);
        let url = format!("{}/v1/messages", self.base_url);
        let mut headers = vec![
            ("x-api-key".into(), self.api_key.expose().to_string()),
            ("anthropic-version".into(), self.api_version.clone()),
        ];
        if request.metadata.purpose == "grader" || request.metadata.purpose == "generator" {
            headers.push(("anthropic-beta".into(), "prompt-caching-2024-07-31".into()));
        }
        let response = http_send("POST", &url, &headers, Some(&body), self.timeout, cancel)?;
        parse_anthropic(response.status, &response.body, sink)
    }

    fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        let url = format!("{}/v1/models", self.base_url);
        let headers = vec![
            ("x-api-key".into(), self.api_key.expose().to_string()),
            ("anthropic-version".into(), self.api_version.clone()),
        ];
        let response = http_send(
            "GET",
            &url,
            &headers,
            None,
            self.timeout,
            &CancellationToken::new(),
        )?;
        parse_model_list(response.status, &response.body, "data", "id")
    }

    fn count_tokens(&self, request: &ChatRequest) -> Result<Option<u64>, ProviderError> {
        let url = format!("{}/v1/messages/count_tokens", self.base_url);
        let headers = vec![
            ("x-api-key".into(), self.api_key.expose().to_string()),
            ("anthropic-version".into(), self.api_version.clone()),
        ];
        let body = anthropic_body(request);
        let response = http_send(
            "POST",
            &url,
            &headers,
            Some(&body),
            self.timeout,
            &CancellationToken::new(),
        )?;
        if response.status >= 400 {
            return Err(ProviderError::from_status(response.status, response.body));
        }
        let parsed: Value = serde_json::from_str(&response.body).unwrap_or(Value::Null);
        Ok(parsed.get("input_tokens").and_then(Value::as_u64))
    }
}

impl ChatProvider for OpenAiProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind_name(&self) -> &'static str {
        "openai_compat"
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            tools: true,
            streaming: true,
            system_prompt: true,
        }
    }

    fn complete(
        &self,
        request: &ChatRequest,
        sink: &mut dyn StreamSink,
        cancel: &CancellationToken,
    ) -> Result<ChatResponse, ProviderError> {
        if cancel.is_cancelled() {
            return Err(ProviderError::new(ErrorClass::Cancelled, "cancelled"));
        }
        let body = openai_body(request);
        let mut url = format!("{}/chat/completions", self.base_url);
        if self.flavor == Flavor::Azure {
            if let Some(version) = &self.api_version {
                url.push_str(&format!("?api-version={version}"));
            }
        }
        let mut headers = Vec::new();
        if !self.api_key.is_empty() {
            if self.flavor == Flavor::Azure {
                headers.push(("api-key".into(), self.api_key.expose().to_string()));
            } else {
                headers.push((
                    "Authorization".into(),
                    format!("Bearer {}", self.api_key.expose()),
                ));
            }
        }
        headers.extend(self.extra_headers.clone());
        let response = http_send("POST", &url, &headers, Some(&body), self.timeout, cancel)?;
        parse_openai(response.status, &response.body, sink)
    }

    fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        let headers = auth_headers(self);
        let url = format!("{}/models", self.base_url);
        let response = http_send(
            "GET",
            &url,
            &headers,
            None,
            self.timeout,
            &CancellationToken::new(),
        )?;
        let mut models =
            parse_model_list(response.status, &response.body, "data", "id").unwrap_or_default();
        if models.is_empty() && self.flavor == Flavor::Ollama {
            let tags_url = ollama_tags_url(&self.base_url);
            let tags = http_send(
                "GET",
                &tags_url,
                &[],
                None,
                self.timeout,
                &CancellationToken::new(),
            )?;
            models = parse_ollama_tags(tags.status, &tags.body)?;
        }
        if models.is_empty() && matches!(self.flavor, Flavor::Ollama | Flavor::Lmstudio) {
            return Err(ProviderError::new(
                ErrorClass::NoModelLoaded,
                "no model loaded",
            ));
        }
        Ok(models)
    }

    fn count_tokens(&self, _request: &ChatRequest) -> Result<Option<u64>, ProviderError> {
        Ok(None)
    }
}

fn auth_headers(provider: &OpenAiProvider) -> Vec<(String, String)> {
    if provider.api_key.is_empty() {
        Vec::new()
    } else if provider.flavor == Flavor::Azure {
        vec![("api-key".into(), provider.api_key.expose().to_string())]
    } else {
        vec![(
            "Authorization".into(),
            format!("Bearer {}", provider.api_key.expose()),
        )]
    }
}

fn ollama_tags_url(base: &str) -> String {
    let trimmed = base.trim_end_matches('/').trim_end_matches("/v1");
    format!("{trimmed}/api/tags")
}

fn anthropic_body(request: &ChatRequest) -> Value {
    let mut body = json!({
        "model": request.model,
        "max_tokens": request.max_tokens,
        "messages": anthropic_messages(&request.messages),
        "stream": true,
    });
    if let Some(system) = &request.system {
        if request.metadata.purpose == "grader" || request.metadata.purpose == "generator" {
            body["system"] = json!([{
                "type": "text",
                "text": system,
                "cache_control": {"type": "ephemeral"}
            }]);
        } else {
            body["system"] = json!(system);
        }
    }
    if !request.tools.is_empty() {
        body["tools"] = Value::Array(
            request
                .tools
                .iter()
                .map(|tool| {
                    json!({
                        "name": tool.name,
                        "description": tool.description,
                        "input_schema": tool.input_schema,
                        "strict": true
                    })
                })
                .collect(),
        );
    }
    if request.thinking == ThinkingMode::Adaptive {
        body["thinking"] = json!({"type": "adaptive"});
    }
    if let Some(effort) = request.effort {
        body["output_config"] = json!({"effort": effort.as_str()});
    }
    body
}

fn anthropic_messages(messages: &[Message]) -> Value {
    Value::Array(
        messages
            .iter()
            .map(|message| {
                json!({
                    "role": message.role,
                    "content": message.content.iter().map(anthropic_block).collect::<Vec<_>>()
                })
            })
            .collect(),
    )
}

fn anthropic_block(block: &ContentBlock) -> Value {
    match block {
        ContentBlock::Text(text) => json!({"type": "text", "text": text}),
        ContentBlock::ToolUse { id, name, input } => {
            json!({"type": "tool_use", "id": id, "name": name, "input": input})
        }
        ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
        } => json!({
            "type": "tool_result",
            "tool_use_id": tool_use_id,
            "content": content,
            "is_error": is_error
        }),
    }
}

fn openai_body(request: &ChatRequest) -> Value {
    let mut messages = Vec::new();
    if let Some(system) = &request.system {
        messages.push(json!({"role": "system", "content": system}));
    }
    for message in &request.messages {
        messages.push(openai_message(message));
    }
    let mut body = json!({
        "model": request.model,
        "messages": messages,
        "stream": true,
        "stream_options": {"include_usage": true},
        "max_tokens": request.max_tokens,
    });
    if !request.tools.is_empty() {
        body["tools"] = Value::Array(
            request
                .tools
                .iter()
                .map(|tool| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": tool.name,
                            "description": tool.description,
                            "parameters": tool.input_schema
                        }
                    })
                })
                .collect(),
        );
    }
    body
}

fn openai_message(message: &Message) -> Value {
    let mut text = String::new();
    let mut tool_calls = Vec::new();
    let mut tool_results = Vec::new();
    for block in &message.content {
        match block {
            ContentBlock::Text(value) => text.push_str(value),
            ContentBlock::ToolUse { id, name, input } => tool_calls.push(json!({
                "id": id,
                "type": "function",
                "function": {"name": name, "arguments": input.to_string()}
            })),
            ContentBlock::ToolResult {
                tool_use_id,
                content,
                ..
            } => {
                tool_results
                    .push(json!({"role": "tool", "tool_call_id": tool_use_id, "content": content}));
            }
        }
    }
    if !tool_results.is_empty() && message.role == "user" {
        return tool_results[0].clone();
    }
    let mut value = json!({"role": message.role, "content": text});
    if !tool_calls.is_empty() {
        value["tool_calls"] = Value::Array(tool_calls);
    }
    value
}

struct HttpResponse {
    status: u16,
    body: String,
}

fn http_send(
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: Option<&Value>,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<HttpResponse, ProviderError> {
    if cancel.is_cancelled() {
        return Err(ProviderError::new(ErrorClass::Cancelled, "cancelled"));
    }
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout_read(timeout)
        .build();
    let mut request = match method {
        "POST" => agent.post(url),
        _ => agent.get(url),
    };
    for (name, value) in headers {
        request = request.set(name, value);
    }
    let result = if let Some(body) = body {
        request.send_json(body.clone())
    } else {
        request.call()
    };
    match result {
        Ok(response) => read_response(response, cancel),
        Err(ureq::Error::Status(code, response)) => {
            let retry_after_ms = response.header("retry-after").and_then(parse_retry_after);
            let mut text = String::new();
            let _ = response
                .into_reader()
                .take(64 * 1024)
                .read_to_string(&mut text);
            let mut err = ProviderError::from_status(code, text);
            err.retry_after_ms = retry_after_ms;
            Err(err)
        }
        Err(ureq::Error::Transport(err)) => {
            let class = if err.kind() == ureq::ErrorKind::Io {
                ErrorClass::Network
            } else {
                ErrorClass::Timeout
            };
            Err(ProviderError::new(class, err.to_string()))
        }
    }
}

fn read_response(
    response: ureq::Response,
    cancel: &CancellationToken,
) -> Result<HttpResponse, ProviderError> {
    let status = response.status();
    let mut reader = response.into_reader().take(2 * 1024 * 1024);
    let mut body = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        if cancel.is_cancelled() {
            return Err(ProviderError::new(ErrorClass::Cancelled, "cancelled"));
        }
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(count) => body.extend_from_slice(&chunk[..count]),
            Err(err) => {
                return Err(ProviderError::new(ErrorClass::Network, err.to_string()));
            }
        }
    }
    let text = String::from_utf8_lossy(&body).into_owned();
    Ok(HttpResponse { status, body: text })
}

fn parse_retry_after(value: &str) -> Option<u64> {
    value
        .trim()
        .parse::<u64>()
        .ok()
        .map(|seconds| seconds.saturating_mul(1000))
}

fn parse_model_list(
    status: u16,
    body: &str,
    array_key: &str,
    id_key: &str,
) -> Result<Vec<ModelInfo>, ProviderError> {
    if status >= 400 {
        return Err(ProviderError::from_status(status, body.to_string()));
    }
    let parsed: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let Some(rows) = parsed.get(array_key).and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    Ok(rows
        .iter()
        .filter_map(|row| {
            row.get(id_key).and_then(Value::as_str).map(|id| ModelInfo {
                id: id.to_string(),
                supports_tools: None,
            })
        })
        .collect())
}

fn parse_ollama_tags(status: u16, body: &str) -> Result<Vec<ModelInfo>, ProviderError> {
    if status >= 400 {
        return Err(ProviderError::from_status(status, body.to_string()));
    }
    let parsed: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let Some(rows) = parsed.get("models").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    Ok(rows
        .iter()
        .filter_map(|row| {
            row.get("name").and_then(Value::as_str).map(|id| ModelInfo {
                id: id.to_string(),
                supports_tools: None,
            })
        })
        .collect())
}

struct SseEvent {
    data: String,
}

fn parse_sse(body: &str) -> Vec<SseEvent> {
    let mut events = Vec::new();
    let mut data = String::new();
    for line in body.split('\n') {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            if !data.is_empty() {
                events.push(SseEvent { data: data.clone() });
                data.clear();
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(rest.trim_start());
        }
    }
    if !data.is_empty() {
        events.push(SseEvent { data });
    }
    events
}

fn parse_anthropic(
    status: u16,
    body: &str,
    sink: &mut dyn StreamSink,
) -> Result<ChatResponse, ProviderError> {
    if status >= 400 {
        return Err(ProviderError::from_status(status, body.to_string()));
    }
    let events = parse_sse(body);
    let mut response = ChatResponse {
        id: String::new(),
        stop: StopReason::EndTurn,
        text: String::new(),
        tool_uses: Vec::new(),
        usage: Usage::zero(),
        incomplete_tool: false,
    };
    let mut tool_buf: Option<(String, String, String)> = None;
    let mut saw_stop = false;
    let mut delivered = false;
    for event in &events {
        let Ok(value) = serde_json::from_str::<Value>(&event.data) else {
            continue;
        };
        match value.get("type").and_then(Value::as_str).unwrap_or("") {
            "message_start" => {
                if let Some(id) = value.pointer("/message/id").and_then(Value::as_str) {
                    response.id = id.to_string();
                    sink.on_event(&StreamEvent::MessageStart {
                        id: response.id.clone(),
                    });
                }
                apply_usage(value.pointer("/message/usage"), &mut response.usage);
            }
            "content_block_start" => {
                let block = &value["content_block"];
                if block.get("type").and_then(Value::as_str) == Some("tool_use") {
                    let id = block
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let name = block
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    sink.on_event(&StreamEvent::ToolUseStart {
                        id: id.clone(),
                        name: name.clone(),
                    });
                    tool_buf = Some((id, name, String::new()));
                    delivered = true;
                }
            }
            "content_block_delta" => {
                let delta = &value["delta"];
                match delta.get("type").and_then(Value::as_str).unwrap_or("") {
                    "text_delta" => {
                        let text = delta.get("text").and_then(Value::as_str).unwrap_or("");
                        response.text.push_str(text);
                        sink.on_event(&StreamEvent::TextDelta(text.to_string()));
                        delivered = true;
                    }
                    "input_json_delta" => {
                        let partial = delta
                            .get("partial_json")
                            .and_then(Value::as_str)
                            .unwrap_or("");
                        if let Some((id, _, buf)) = tool_buf.as_mut() {
                            buf.push_str(partial);
                            sink.on_event(&StreamEvent::ToolInputDelta {
                                id: id.clone(),
                                partial_json: partial.to_string(),
                            });
                            delivered = true;
                        }
                    }
                    "thinking_delta" => {
                        let text = delta.get("thinking").and_then(Value::as_str).unwrap_or("");
                        sink.on_event(&StreamEvent::ThinkingDelta(text.to_string()));
                    }
                    _ => {}
                }
            }
            "content_block_stop" => {
                if let Some((id, name, raw)) = tool_buf.take() {
                    push_tool(&mut response, sink, id, name, raw);
                }
            }
            "message_delta" => {
                if let Some(reason) = value.pointer("/delta/stop_reason").and_then(Value::as_str) {
                    let category = value
                        .pointer("/delta/stop_details/category")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    response.stop = map_anthropic_stop(reason, category);
                }
                apply_usage(value.get("usage"), &mut response.usage);
                saw_stop = true;
            }
            "message_stop" => {
                saw_stop = true;
            }
            _ => {}
        }
    }
    if !saw_stop {
        let mut err = ProviderError::new(ErrorClass::Network, "stream ended before message_stop");
        err.delivered = delivered;
        err.retryable = !delivered;
        return Err(err);
    }
    finish_stop(&mut response, sink);
    sink.on_event(&StreamEvent::Usage(response.usage.clone()));
    sink.on_event(&StreamEvent::MessageEnd {
        stop_reason: response.stop.clone(),
    });
    Ok(response)
}

fn parse_openai(
    status: u16,
    body: &str,
    sink: &mut dyn StreamSink,
) -> Result<ChatResponse, ProviderError> {
    if status >= 400 {
        return Err(ProviderError::from_status(status, body.to_string()));
    }
    let mut response = ChatResponse {
        id: String::new(),
        stop: StopReason::EndTurn,
        text: String::new(),
        tool_uses: Vec::new(),
        usage: Usage::zero(),
        incomplete_tool: false,
    };
    let mut tools: Vec<(String, String, String)> = Vec::new();
    let mut saw_done = false;
    let mut delivered = false;
    for event in parse_sse(body) {
        if event.data.trim() == "[DONE]" {
            saw_done = true;
            break;
        }
        let Ok(value) = serde_json::from_str::<Value>(&event.data) else {
            continue;
        };
        if response.id.is_empty() {
            if let Some(id) = value.get("id").and_then(Value::as_str) {
                response.id = id.to_string();
                sink.on_event(&StreamEvent::MessageStart {
                    id: response.id.clone(),
                });
            }
        }
        if let Some(usage) = value.get("usage") {
            response.usage.input_tokens = usage
                .get("prompt_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(response.usage.input_tokens);
            response.usage.output_tokens = usage
                .get("completion_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(response.usage.output_tokens);
            response.usage.cache_read_input_tokens = usage
                .pointer("/prompt_tokens_details/cached_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(response.usage.cache_read_input_tokens);
        }
        let choice = &value["choices"][0];
        if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
            response.stop = map_openai_stop(reason);
            saw_done = true;
        }
        let delta = &choice["delta"];
        if let Some(text) = delta.get("content").and_then(Value::as_str) {
            response.text.push_str(text);
            sink.on_event(&StreamEvent::TextDelta(text.to_string()));
            delivered = true;
        }
        if let Some(calls) = delta.get("tool_calls").and_then(Value::as_array) {
            for call in calls {
                let index = call.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
                while tools.len() <= index {
                    tools.push((String::new(), String::new(), String::new()));
                }
                if let Some(id) = call.get("id").and_then(Value::as_str) {
                    tools[index].0 = id.to_string();
                }
                if let Some(name) = call.pointer("/function/name").and_then(Value::as_str) {
                    tools[index].1 = name.to_string();
                    sink.on_event(&StreamEvent::ToolUseStart {
                        id: tools[index].0.clone(),
                        name: name.to_string(),
                    });
                    delivered = true;
                }
                if let Some(args) = call.pointer("/function/arguments").and_then(Value::as_str) {
                    tools[index].2.push_str(args);
                    sink.on_event(&StreamEvent::ToolInputDelta {
                        id: tools[index].0.clone(),
                        partial_json: args.to_string(),
                    });
                    delivered = true;
                }
            }
        }
    }
    if !saw_done
        && response.stop == StopReason::EndTurn
        && response.text.is_empty()
        && tools.is_empty()
    {
        let mut err = ProviderError::new(ErrorClass::Network, "stream ended before finish_reason");
        err.delivered = delivered;
        err.retryable = !delivered;
        return Err(err);
    }
    for (id, name, raw) in tools {
        push_tool(&mut response, sink, id, name, raw);
    }
    finish_stop(&mut response, sink);
    sink.on_event(&StreamEvent::Usage(response.usage.clone()));
    sink.on_event(&StreamEvent::MessageEnd {
        stop_reason: response.stop.clone(),
    });
    Ok(response)
}

fn push_tool(
    response: &mut ChatResponse,
    sink: &mut dyn StreamSink,
    id: String,
    name: String,
    raw: String,
) {
    let parsed = serde_json::from_str::<Value>(&raw);
    let incomplete = parsed.is_err();
    if incomplete {
        response.incomplete_tool = true;
        sink.on_event(&StreamEvent::ErrorStep {
            message: format!("incomplete tool input for {name}"),
        });
    }
    sink.on_event(&StreamEvent::ToolUseEnd { id: id.clone() });
    response.tool_uses.push(ToolUse {
        id,
        name,
        input: parsed.unwrap_or(Value::Null),
        raw_input: raw,
        incomplete,
    });
}

fn finish_stop(response: &mut ChatResponse, sink: &mut dyn StreamSink) {
    if matches!(response.stop, StopReason::Refusal { .. }) {
        let category = match &response.stop {
            StopReason::Refusal { category } => {
                category.clone().unwrap_or_else(|| "refusal".into())
            }
            _ => "refusal".into(),
        };
        sink.on_event(&StreamEvent::ErrorStep {
            message: format!("refusal:{category}"),
        });
    }
    if response.incomplete_tool && matches!(response.stop, StopReason::MaxTokens) {
        sink.on_event(&StreamEvent::ErrorStep {
            message: "max_tokens cut off a tool input".into(),
        });
    }
}

fn apply_usage(value: Option<&Value>, usage: &mut Usage) {
    let Some(value) = value else {
        return;
    };
    if let Some(input) = value.get("input_tokens").and_then(Value::as_u64) {
        usage.input_tokens = input;
    }
    if let Some(output) = value.get("output_tokens").and_then(Value::as_u64) {
        usage.output_tokens = output;
    }
    if let Some(cache) = value.get("cache_read_input_tokens").and_then(Value::as_u64) {
        usage.cache_read_input_tokens = cache;
    }
    if let Some(cache) = value
        .get("cache_creation_input_tokens")
        .and_then(Value::as_u64)
    {
        usage.cache_creation_input_tokens = cache;
    }
}

fn map_anthropic_stop(reason: &str, category: Option<String>) -> StopReason {
    match reason {
        "end_turn" => StopReason::EndTurn,
        "tool_use" => StopReason::ToolUse,
        "max_tokens" => StopReason::MaxTokens,
        "stop_sequence" => StopReason::StopSequence,
        "refusal" => StopReason::Refusal { category },
        "pause_turn" => StopReason::PauseTurn,
        other => StopReason::Other(other.to_string()),
    }
}

fn map_openai_stop(reason: &str) -> StopReason {
    match reason {
        "stop" => StopReason::EndTurn,
        "tool_calls" => StopReason::ToolUse,
        "length" => StopReason::MaxTokens,
        "content_filter" => StopReason::Refusal {
            category: Some("content_filter".into()),
        },
        other => StopReason::Other(other.to_string()),
    }
}
