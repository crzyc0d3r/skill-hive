//! Shared provider types. The trait is synchronous: cancellation is checked between I/O chunks,
//! and the desktop command boundary does not need an async runtime.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SecretString([redacted])")
    }
}

#[derive(Clone, Debug)]
pub struct CancellationToken {
    flag: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Anthropic,
    OpenaiCompat,
    LocalCli,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Flavor {
    Openai,
    Azure,
    Openrouter,
    Ollama,
    Lmstudio,
    Vllm,
    Custom,
    ClaudeCli,
    CodexCli,
}

impl Flavor {
    pub fn default_base_url(self) -> Option<&'static str> {
        match self {
            Self::Openai => Some("https://api.openai.com/v1"),
            Self::Openrouter => Some("https://openrouter.ai/api/v1"),
            Self::Ollama => Some("http://127.0.0.1:11434/v1"),
            Self::Lmstudio => Some("http://127.0.0.1:1234/v1"),
            Self::Vllm => Some("http://127.0.0.1:8000/v1"),
            Self::Azure | Self::Custom | Self::ClaudeCli | Self::CodexCli => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effort {
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

impl Effort {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Xhigh => "xhigh",
            Self::Max => "max",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingMode {
    Adaptive,
    Off,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
}

impl Usage {
    pub fn zero() -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
        }
    }

    pub fn total(&self) -> u64 {
        self.input_tokens
            + self.output_tokens
            + self.cache_read_input_tokens
            + self.cache_creation_input_tokens
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
    StopSequence,
    Refusal { category: Option<String> },
    PauseTurn,
    Other(String),
}

#[derive(Debug, Clone)]
pub enum ContentBlock {
    Text(String),
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Clone)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Clone)]
pub struct RequestMetadata {
    pub purpose: String,
    pub ref_id: Option<String>,
    pub provider_id: Option<String>,
}

impl Default for RequestMetadata {
    fn default() -> Self {
        Self {
            purpose: "executor".into(),
            ref_id: None,
            provider_id: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub system: Option<String>,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDef>,
    pub max_tokens: u32,
    pub effort: Option<Effort>,
    pub thinking: ThinkingMode,
    pub stream: bool,
    pub metadata: RequestMetadata,
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    MessageStart { id: String },
    TextDelta(String),
    ThinkingDelta(String),
    ToolUseStart { id: String, name: String },
    ToolInputDelta { id: String, partial_json: String },
    ToolUseEnd { id: String },
    Usage(Usage),
    MessageEnd { stop_reason: StopReason },
    ErrorStep { message: String },
}

pub trait StreamSink {
    fn on_event(&mut self, event: &StreamEvent);
}

#[derive(Default)]
pub struct VecSink {
    pub events: Vec<StreamEvent>,
}

impl StreamSink for VecSink {
    fn on_event(&mut self, event: &StreamEvent) {
        self.events.push(event.clone());
    }
}

#[derive(Debug, Clone)]
pub struct ToolUse {
    pub id: String,
    pub name: String,
    pub input: Value,
    pub raw_input: String,
    pub incomplete: bool,
}

#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub id: String,
    pub stop: StopReason,
    pub text: String,
    pub tool_uses: Vec<ToolUse>,
    pub usage: Usage,
    pub incomplete_tool: bool,
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub supports_tools: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Capabilities {
    pub tools: bool,
    pub streaming: bool,
    pub system_prompt: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    Authentication,
    Billing,
    UnknownModel,
    Network,
    RateLimited,
    NoModelLoaded,
    NoToolSupport,
    Timeout,
    RecordingMissing,
    Cancelled,
    Overloaded,
    BadRequest,
    Other,
}

impl ErrorClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Authentication => "authentication",
            Self::Billing => "billing",
            Self::UnknownModel => "unknown_model",
            Self::Network => "network",
            Self::RateLimited => "rate_limited",
            Self::NoModelLoaded => "no_model_loaded",
            Self::NoToolSupport => "no_tool_support",
            Self::Timeout => "timeout",
            Self::RecordingMissing => "recording_missing",
            Self::Cancelled => "cancelled",
            Self::Overloaded => "overloaded",
            Self::BadRequest => "bad_request",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProviderError {
    pub class: ErrorClass,
    pub message: String,
    pub retryable: bool,
    pub delivered: bool,
    pub status: Option<u16>,
    pub retry_after_ms: Option<u64>,
}

impl ProviderError {
    pub fn new(class: ErrorClass, message: impl Into<String>) -> Self {
        let retryable = matches!(
            class,
            ErrorClass::RateLimited | ErrorClass::Overloaded | ErrorClass::Network
        );
        Self {
            class,
            message: message.into(),
            retryable,
            delivered: false,
            status: None,
            retry_after_ms: None,
        }
    }

    pub fn from_status(status: u16, message: impl Into<String>) -> Self {
        let class = match status {
            401 | 403 => ErrorClass::Authentication,
            402 => ErrorClass::Billing,
            404 => ErrorClass::UnknownModel,
            429 => ErrorClass::RateLimited,
            529 => ErrorClass::Overloaded,
            500 | 502 | 503 | 504 => ErrorClass::Network,
            400 | 413 => ErrorClass::BadRequest,
            _ => ErrorClass::Other,
        };
        let mut err = Self::new(class, message);
        err.status = Some(status);
        err
    }
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.class.as_str(), self.message)
    }
}

pub trait ChatProvider: Send + Sync {
    fn id(&self) -> &str;
    fn kind_name(&self) -> &'static str;
    fn capabilities(&self) -> Capabilities;
    fn complete(
        &self,
        request: &ChatRequest,
        sink: &mut dyn StreamSink,
        cancel: &CancellationToken,
    ) -> Result<ChatResponse, ProviderError>;
    fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError>;
    fn count_tokens(&self, request: &ChatRequest) -> Result<Option<u64>, ProviderError>;
}

#[derive(Debug, Clone, Serialize)]
pub struct ProbeStage {
    pub index: u8,
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProbeReport {
    pub ok: bool,
    pub stages: Vec<ProbeStage>,
    pub latency_ms: u64,
    pub error_class: Option<String>,
    pub models: Vec<String>,
    pub supports_tools: bool,
}
