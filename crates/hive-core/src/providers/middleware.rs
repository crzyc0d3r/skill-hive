//! Retry, concurrency limit, recording, and the usage ledger. Adapters stay free of these.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::pricing::{self, Pricing};
use super::types::*;
use crate::providers::scrub;

pub trait Sleeper: Send + Sync {
    fn sleep_ms(&self, millis: u64);
}

pub struct RealSleeper;

impl Sleeper for RealSleeper {
    fn sleep_ms(&self, millis: u64) {
        if millis > 0 {
            thread::sleep(Duration::from_millis(millis));
        }
    }
}

pub struct NoopSleeper {
    pub slept: std::sync::Arc<Mutex<Vec<u64>>>,
}

impl NoopSleeper {
    pub fn new() -> Self {
        Self {
            slept: std::sync::Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for NoopSleeper {
    fn default() -> Self {
        Self::new()
    }
}

impl Sleeper for NoopSleeper {
    fn sleep_ms(&self, millis: u64) {
        self.slept.lock().expect("sleeper").push(millis);
    }
}

struct TokenBucket {
    capacity: f64,
    available: f64,
    per_second: f64,
    last_ms: u64,
}

impl TokenBucket {
    fn new(per_minute: f64, now_ms: u64) -> Self {
        Self {
            capacity: per_minute,
            available: per_minute,
            per_second: per_minute / 60.0,
            last_ms: now_ms,
        }
    }

    fn take(&mut self, cost: f64, now_ms: u64) -> bool {
        let elapsed = now_ms.saturating_sub(self.last_ms) as f64 / 1000.0;
        self.last_ms = now_ms;
        self.available = (self.available + elapsed * self.per_second).min(self.capacity);
        if self.available < cost {
            return false;
        }
        self.available -= cost;
        true
    }
}

struct RateGate {
    rpm: Option<TokenBucket>,
    tpm: Option<TokenBucket>,
    now_ms: u64,
    ledger: Option<PathBuf>,
    provider_id: String,
}

pub struct Limiter {
    max: usize,
    current: Mutex<usize>,
    cv: Condvar,
    pub peak: Mutex<usize>,
    rates: Mutex<Option<RateGate>>,
}

impl Limiter {
    pub fn new(max: usize) -> Self {
        Self {
            max: max.max(1),
            current: Mutex::new(0),
            cv: Condvar::new(),
            peak: Mutex::new(0),
            rates: Mutex::new(None),
        }
    }

    /// Load `rpm_limit` and `tpm_limit` from a provider row and arm the token buckets.
    /// `now_secs` is the clock for the current minute so tests can hold time still.
    pub fn load_provider_budget(
        &self,
        db: &Path,
        provider_id: &str,
        now_secs: u64,
    ) -> Result<(), ProviderError> {
        let conn = rusqlite::Connection::open(db)
            .map_err(|err| ProviderError::new(ErrorClass::Other, err.to_string()))?;
        let (rpm, tpm): (Option<i64>, Option<i64>) = conn
            .query_row(
                "SELECT rpm_limit, tpm_limit FROM provider_configs WHERE id = ?1",
                (provider_id,),
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|err| ProviderError::new(ErrorClass::Other, err.to_string()))?;
        let now_ms = now_secs.saturating_mul(1000);
        let rpm_bucket = rpm
            .filter(|value| *value >= 0)
            .map(|value| TokenBucket::new(value as f64, now_ms));
        let tpm_bucket = tpm
            .filter(|value| *value >= 0)
            .map(|value| TokenBucket::new(value as f64, now_ms));
        *self.rates.lock().expect("rates") = Some(RateGate {
            rpm: rpm_bucket,
            tpm: tpm_bucket,
            now_ms,
            ledger: Some(db.to_path_buf()),
            provider_id: provider_id.to_string(),
        });
        Ok(())
    }

    pub fn try_acquire_budget(
        &self,
        model: &str,
        purpose: &str,
        tokens: u64,
    ) -> Result<(), ProviderError> {
        let mut guard = self.rates.lock().expect("rates");
        let Some(gate) = guard.as_mut() else {
            return Ok(());
        };
        if gate.rpm.is_none() && gate.tpm.is_none() {
            return Ok(());
        }
        let now_ms = gate.now_ms;
        let tokens = tokens.max(1);
        let rpm_refused = gate
            .rpm
            .as_mut()
            .is_some_and(|bucket| !bucket.take(1.0, now_ms));
        if rpm_refused {
            let ledger = gate.ledger.clone();
            let provider_id = gate.provider_id.clone();
            drop(guard);
            write_ledger_row(
                ledger.as_deref(),
                &provider_id,
                model,
                purpose,
                0.0,
                "rate_limited",
                Some("rpm"),
                now_ms / 1000,
            )?;
            let mut err =
                ProviderError::new(ErrorClass::RateLimited, "requests per minute exceeded");
            err.retryable = false;
            return Err(err);
        }
        let tpm_refused = gate
            .tpm
            .as_mut()
            .is_some_and(|bucket| !bucket.take(tokens as f64, now_ms));
        if tpm_refused {
            let ledger = gate.ledger.clone();
            let provider_id = gate.provider_id.clone();
            drop(guard);
            write_ledger_row(
                ledger.as_deref(),
                &provider_id,
                model,
                purpose,
                0.0,
                "rate_limited",
                Some("tpm"),
                now_ms / 1000,
            )?;
            let mut err = ProviderError::new(ErrorClass::RateLimited, "tokens per minute exceeded");
            err.retryable = false;
            return Err(err);
        }
        Ok(())
    }

    pub fn record_cost(&self, row: &LedgerRow) {
        let guard = self.rates.lock().expect("rates");
        let Some(gate) = guard.as_ref() else {
            return;
        };
        let now_ms = gate.now_ms;
        let ledger = gate.ledger.clone();
        let provider_id = gate.provider_id.clone();
        drop(guard);
        let _ = write_ledger_row(
            ledger.as_deref(),
            &provider_id,
            &row.model,
            &row.purpose,
            row.cost_usd,
            &row.status,
            row.error_kind.as_deref(),
            now_ms / 1000,
        );
    }

    pub fn acquire(&self, cancel: &CancellationToken) -> Result<Permit<'_>, ProviderError> {
        let mut current = self.current.lock().expect("limiter");
        while *current >= self.max {
            if cancel.is_cancelled() {
                return Err(ProviderError::new(
                    ErrorClass::Cancelled,
                    "cancelled while queued",
                ));
            }
            let (next, _) = self
                .cv
                .wait_timeout(current, Duration::from_millis(20))
                .expect("limiter wait");
            current = next;
        }
        *current += 1;
        let mut peak = self.peak.lock().expect("peak");
        if *current > *peak {
            *peak = *current;
        }
        drop(peak);
        drop(current);
        Ok(Permit { limiter: self })
    }
}

fn write_ledger_row(
    db: Option<&Path>,
    provider_id: &str,
    model: &str,
    purpose: &str,
    cost_usd: f64,
    status: &str,
    error_kind: Option<&str>,
    ts: u64,
) -> Result<(), ProviderError> {
    let Some(db) = db else {
        return Ok(());
    };
    let conn = rusqlite::Connection::open(db)
        .map_err(|err| ProviderError::new(ErrorClass::Other, err.to_string()))?;
    conn.execute(
        "INSERT INTO usage_ledger (ts, provider_config_id, model, purpose, input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, cost_usd, status, error_kind) VALUES (?1, ?2, ?3, ?4, 0, 0, 0, 0, ?5, ?6, ?7)",
        (
            ts as i64,
            provider_id,
            model,
            purpose,
            cost_usd,
            status,
            error_kind,
        ),
    )
    .map_err(|err| ProviderError::new(ErrorClass::Other, err.to_string()))?;
    Ok(())
}

pub struct Permit<'a> {
    limiter: &'a Limiter,
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        let mut current = self.limiter.current.lock().expect("limiter");
        *current = current.saturating_sub(1);
        self.limiter.cv.notify_one();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Live,
    Record,
    Replay,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recording {
    pub key: String,
    pub kind: String,
    pub status: u16,
    pub body: String,
    pub error_class: Option<String>,
}

#[derive(Debug)]
pub struct LedgerRow {
    pub provider_id: String,
    pub model: String,
    pub purpose: String,
    pub ref_id: Option<String>,
    pub usage: Usage,
    pub cost_usd: f64,
    pub status: String,
    pub error_kind: Option<String>,
}

pub struct Middleware {
    pub mode: RunMode,
    pub recordings_dir: PathBuf,
    pub kind: String,
    pub max_retries: u32,
    pub limiter: Limiter,
    pub sleeper: Box<dyn Sleeper>,
    pub secrets: Vec<String>,
    pub pricing: Pricing,
    pub provider_id: String,
    pub hits: Mutex<u64>,
}

impl Middleware {
    pub fn complete(
        &self,
        inner: &dyn ChatProvider,
        request: &ChatRequest,
        sink: &mut dyn StreamSink,
        cancel: &CancellationToken,
    ) -> Result<(ChatResponse, LedgerRow), ProviderError> {
        if self.mode == RunMode::Replay {
            return self.replay(request, sink);
        }
        self.limiter.try_acquire_budget(
            &request.model,
            &request.metadata.purpose,
            u64::from(request.max_tokens).max(1),
        )?;
        let _permit = self.limiter.acquire(cancel)?;
        let mut attempt = 0_u32;
        loop {
            if cancel.is_cancelled() {
                return Err(ProviderError::new(ErrorClass::Cancelled, "cancelled"));
            }
            *self.hits.lock().expect("hits") += 1;
            match inner.complete(request, sink, cancel) {
                Ok(response) => {
                    if self.mode == RunMode::Record {
                        self.write_recording(request, 200, &serialize_response(&response), None)?;
                    }
                    let row = self.row(request, &response.usage, "ok", None);
                    self.limiter.record_cost(&row);
                    return Ok((response, row));
                }
                Err(err) if err.retryable && !err.delivered && attempt < self.max_retries => {
                    let wait = err.retry_after_ms.unwrap_or_else(|| backoff_ms(attempt));
                    self.sleeper.sleep_ms(wait);
                    attempt += 1;
                }
                Err(err) => return Err(err),
            }
        }
    }

    fn replay(
        &self,
        request: &ChatRequest,
        sink: &mut dyn StreamSink,
    ) -> Result<(ChatResponse, LedgerRow), ProviderError> {
        let key = request_key(request);
        let path = self.recording_path(&key);
        if !path.exists() {
            return Err(ProviderError::new(
                ErrorClass::RecordingMissing,
                format!("recording_missing {key}"),
            ));
        }
        let raw = fs::read_to_string(&path)
            .map_err(|err| ProviderError::new(ErrorClass::Other, err.to_string()))?;
        let recording: Recording = serde_json::from_str(&raw)
            .map_err(|err| ProviderError::new(ErrorClass::Other, err.to_string()))?;
        if let Some(class) = &recording.error_class {
            return Err(ProviderError::new(ErrorClass::Other, class.clone()));
        }
        let response = decode_response(&recording.body, sink)?;
        let mut row = self.row(request, &response.usage, "replayed", None);
        row.cost_usd = 0.0;
        Ok((response, row))
    }

    fn write_recording(
        &self,
        request: &ChatRequest,
        status: u16,
        body: &str,
        error_class: Option<String>,
    ) -> Result<(), ProviderError> {
        let key = request_key(request);
        let path = self.recording_path(&key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| ProviderError::new(ErrorClass::Other, err.to_string()))?;
        }
        let recording = Recording {
            key: key.clone(),
            kind: self.kind.clone(),
            status,
            body: scrub(body, &self.secrets),
            error_class,
        };
        let text = serde_json::to_string_pretty(&recording)
            .map_err(|err| ProviderError::new(ErrorClass::Other, err.to_string()))?;
        fs::write(path, text)
            .map_err(|err| ProviderError::new(ErrorClass::Other, err.to_string()))?;
        Ok(())
    }

    fn recording_path(&self, key: &str) -> PathBuf {
        self.recordings_dir
            .join(&self.kind)
            .join(format!("{key}.json"))
    }

    fn row(
        &self,
        request: &ChatRequest,
        usage: &Usage,
        status: &str,
        error_kind: Option<String>,
    ) -> LedgerRow {
        LedgerRow {
            provider_id: self.provider_id.clone(),
            model: request.model.clone(),
            purpose: request.metadata.purpose.clone(),
            ref_id: request.metadata.ref_id.clone(),
            usage: usage.clone(),
            cost_usd: pricing::cost_usd(
                &self.pricing,
                usage.input_tokens,
                usage.output_tokens,
                usage.cache_read_input_tokens,
                usage.cache_creation_input_tokens,
            ),
            status: status.to_string(),
            error_kind,
        }
    }
}

pub fn request_key(request: &ChatRequest) -> String {
    let value = json!({
        "effort": request.effort.map(Effort::as_str),
        "messages": request.messages.iter().map(message_value).collect::<Vec<_>>(),
        "model": request.model,
        "purpose": request.metadata.purpose,
        "system": request.system,
        "tools": request.tools.iter().map(|tool| json!({"name": tool.name, "schema": tool.input_schema})).collect::<Vec<_>>(),
    });
    let canonical = canonical_json(&value);
    let digest = Sha256::digest(canonical.as_bytes());
    hex::encode(digest)
}

fn message_value(message: &Message) -> Value {
    json!({
        "role": message.role,
        "content": message.content.iter().map(block_value).collect::<Vec<_>>()
    })
}

fn block_value(block: &ContentBlock) -> Value {
    match block {
        ContentBlock::Text(text) => json!({"text": text, "type": "text"}),
        ContentBlock::ToolUse { id, name, input } => {
            json!({"id": id, "input": input, "name": name, "type": "tool_use"})
        }
        ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
        } => {
            json!({"content": content, "is_error": is_error, "tool_use_id": tool_use_id, "type": "tool_result"})
        }
    }
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .into_iter()
                .map(|key| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(key).unwrap_or_default(),
                        canonical_json(&map[key])
                    )
                })
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(canonical_json).collect();
            format!("[{}]", parts.join(","))
        }
        other => serde_json::to_string(other).unwrap_or_else(|_| "null".into()),
    }
}

fn backoff_ms(attempt: u32) -> u64 {
    let cap = 60_000_u64;
    let base = 1_000_u64.saturating_mul(1_u64 << attempt.min(6));
    let bound = base.min(cap);
    (std::process::id() as u64).wrapping_mul(attempt as u64 + 1) % (bound + 1)
}

fn serialize_response(response: &ChatResponse) -> String {
    let tools: Vec<Value> = response
        .tool_uses
        .iter()
        .map(|tool| {
            json!({
                "id": tool.id,
                "incomplete": tool.incomplete,
                "input": tool.input,
                "name": tool.name,
                "raw": tool.raw_input
            })
        })
        .collect();
    json!({
        "id": response.id,
        "stop": stop_value(&response.stop),
        "text": response.text,
        "tools": tools,
        "usage": {
            "input_tokens": response.usage.input_tokens,
            "output_tokens": response.usage.output_tokens,
            "cache_read_input_tokens": response.usage.cache_read_input_tokens,
            "cache_creation_input_tokens": response.usage.cache_creation_input_tokens
        },
        "incomplete_tool": response.incomplete_tool
    })
    .to_string()
}

fn stop_value(stop: &StopReason) -> Value {
    match stop {
        StopReason::EndTurn => json!("end_turn"),
        StopReason::ToolUse => json!("tool_use"),
        StopReason::MaxTokens => json!("max_tokens"),
        StopReason::StopSequence => json!("stop_sequence"),
        StopReason::Refusal { category } => json!({"refusal": category}),
        StopReason::PauseTurn => json!("pause_turn"),
        StopReason::Other(value) => json!(value),
    }
}

fn decode_response(body: &str, sink: &mut dyn StreamSink) -> Result<ChatResponse, ProviderError> {
    let value: Value = serde_json::from_str(body)
        .map_err(|err| ProviderError::new(ErrorClass::Other, err.to_string()))?;
    let usage = Usage {
        input_tokens: value
            .pointer("/usage/input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        output_tokens: value
            .pointer("/usage/output_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        cache_read_input_tokens: value
            .pointer("/usage/cache_read_input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        cache_creation_input_tokens: value
            .pointer("/usage/cache_creation_input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
    };
    let mut tool_uses = Vec::new();
    if let Some(tools) = value.get("tools").and_then(Value::as_array) {
        for tool in tools {
            let id = tool
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let name = tool
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            sink.on_event(&StreamEvent::ToolUseStart {
                id: id.clone(),
                name: name.clone(),
            });
            tool_uses.push(ToolUse {
                id,
                name,
                input: tool.get("input").cloned().unwrap_or(Value::Null),
                raw_input: tool
                    .get("raw")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                incomplete: tool
                    .get("incomplete")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            });
        }
    }
    let text = value
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if !text.is_empty() {
        sink.on_event(&StreamEvent::TextDelta(text.clone()));
    }
    let stop = match value.get("stop") {
        Some(Value::String(reason)) => match reason.as_str() {
            "tool_use" => StopReason::ToolUse,
            "max_tokens" => StopReason::MaxTokens,
            "stop_sequence" => StopReason::StopSequence,
            "pause_turn" => StopReason::PauseTurn,
            _ => StopReason::EndTurn,
        },
        Some(Value::Object(map)) => StopReason::Refusal {
            category: map
                .get("refusal")
                .and_then(Value::as_str)
                .map(str::to_string),
        },
        _ => StopReason::EndTurn,
    };
    sink.on_event(&StreamEvent::Usage(usage.clone()));
    sink.on_event(&StreamEvent::MessageEnd {
        stop_reason: stop.clone(),
    });
    Ok(ChatResponse {
        id: value
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("replay")
            .to_string(),
        stop,
        text,
        tool_uses,
        usage,
        incomplete_tool: value
            .get("incomplete_tool")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

pub fn recording_diff(left: &str, right: &str) -> String {
    if left == right {
        "recordings match".into()
    } else {
        format!(
            "recordings differ ({} vs {} bytes)",
            left.len(),
            right.len()
        )
    }
}

pub fn recording_exists(dir: &Path, kind: &str, request: &ChatRequest) -> bool {
    dir.join(kind)
        .join(format!("{}.json", request_key(request)))
        .is_file()
}
