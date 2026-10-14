//! In-process provider used by offline probes and the engine tests.

use std::sync::{Arc, Mutex};

use serde_json::json;

use super::types::*;

#[derive(Clone)]
pub struct ScriptedProvider {
    pub id: String,
    pub model: String,
    pub kind_label: &'static str,
    pub prompts: Arc<Mutex<Vec<String>>>,
}

impl ScriptedProvider {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            model: "fixture-model".into(),
            kind_label: "scripted",
            prompts: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl ChatProvider for ScriptedProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind_name(&self) -> &'static str {
        self.kind_label
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
        sink.on_event(&StreamEvent::MessageStart {
            id: "scripted".into(),
        });
        if request.metadata.purpose == "grader" {
            let prompt = prompt_text(request);
            self.prompts.lock().expect("prompts").push(prompt);
            return grade_reply(request, sink);
        }
        if request.metadata.purpose == "test_connection" {
            return if request.tools.is_empty() {
                text_reply(
                    sink,
                    "OK",
                    Usage {
                        input_tokens: 4,
                        output_tokens: 1,
                        cache_read_input_tokens: 0,
                        cache_creation_input_tokens: 0,
                    },
                )
            } else {
                tool_reply(sink, "echo", json!({"word": "OK"}))
            };
        }
        let skilled = request
            .system
            .as_deref()
            .unwrap_or("")
            .contains("HIVE_SKILL_MARKER_9f3a");
        let has_result = request.messages.iter().any(|message| {
            message
                .content
                .iter()
                .any(|block| matches!(block, ContentBlock::ToolResult { .. }))
        });
        if !has_result {
            let content = if skilled { "PINECONE" } else { "plain" };
            return tool_reply(
                sink,
                "write_file",
                json!({"path": "outputs/answer.txt", "content": content}),
            );
        }
        let text = if skilled {
            "Wrote PINECONE."
        } else {
            "No fixture skill was loaded."
        };
        text_reply(
            sink,
            text,
            Usage {
                input_tokens: 20,
                output_tokens: 8,
                cache_read_input_tokens: 2,
                cache_creation_input_tokens: 1,
            },
        )
    }

    fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(vec![ModelInfo {
            id: self.model.clone(),
            supports_tools: Some(true),
        }])
    }

    fn count_tokens(&self, request: &ChatRequest) -> Result<Option<u64>, ProviderError> {
        let chars = request.system.as_deref().unwrap_or("").len() + 16;
        Ok(Some(u64::try_from(chars).unwrap_or(1)))
    }
}

fn text_reply(
    sink: &mut dyn StreamSink,
    text: &str,
    usage: Usage,
) -> Result<ChatResponse, ProviderError> {
    sink.on_event(&StreamEvent::TextDelta(text.to_string()));
    sink.on_event(&StreamEvent::Usage(usage.clone()));
    sink.on_event(&StreamEvent::MessageEnd {
        stop_reason: StopReason::EndTurn,
    });
    Ok(ChatResponse {
        id: "scripted".into(),
        stop: StopReason::EndTurn,
        text: text.into(),
        tool_uses: Vec::new(),
        usage,
        incomplete_tool: false,
    })
}

fn tool_reply(
    sink: &mut dyn StreamSink,
    name: &str,
    input: serde_json::Value,
) -> Result<ChatResponse, ProviderError> {
    let usage = Usage {
        input_tokens: 11,
        output_tokens: 6,
        cache_read_input_tokens: 1,
        cache_creation_input_tokens: 0,
    };
    sink.on_event(&StreamEvent::ToolUseStart {
        id: "tool-1".into(),
        name: name.into(),
    });
    let raw = input.to_string();
    sink.on_event(&StreamEvent::ToolInputDelta {
        id: "tool-1".into(),
        partial_json: raw.clone(),
    });
    sink.on_event(&StreamEvent::ToolUseEnd {
        id: "tool-1".into(),
    });
    sink.on_event(&StreamEvent::Usage(usage.clone()));
    sink.on_event(&StreamEvent::MessageEnd {
        stop_reason: StopReason::ToolUse,
    });
    Ok(ChatResponse {
        id: "scripted".into(),
        stop: StopReason::ToolUse,
        text: String::new(),
        tool_uses: vec![ToolUse {
            id: "tool-1".into(),
            name: name.into(),
            input,
            raw_input: raw,
            incomplete: false,
        }],
        usage,
        incomplete_tool: false,
    })
}

fn prompt_text(request: &ChatRequest) -> String {
    request
        .messages
        .iter()
        .flat_map(|message| message.content.iter())
        .filter_map(|block| match block {
            ContentBlock::Text(text) => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn grade_reply(
    request: &ChatRequest,
    sink: &mut dyn StreamSink,
) -> Result<ChatResponse, ProviderError> {
    let prompt = prompt_text(request);
    if prompt.contains("MALFORMED") {
        return text_reply(sink, "this is not json", Usage::zero());
    }
    let passed = prompt.contains("PINECONE");
    let body = json!({
        "expectations": [{
            "text": "The answer file contains PINECONE",
            "passed": passed,
            "evidence": "step 2"
        }],
        "claims": [],
        "execution_metrics": {"total_tool_calls": 999},
        "timing": {"total_tokens": 1}
    });
    text_reply(
        sink,
        &body.to_string(),
        Usage {
            input_tokens: 30,
            output_tokens: 12,
            cache_read_input_tokens: 4,
            cache_creation_input_tokens: 0,
        },
    )
}
