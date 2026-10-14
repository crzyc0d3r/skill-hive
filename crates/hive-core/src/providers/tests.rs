//! Provider acceptance tests. Live calls stay on 127.0.0.1. Vendor hosts are never contacted.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::json;

#[cfg(windows)]
use super::cli_agent;
#[cfg(unix)]
use super::spawn_grouped;
use super::*;

fn request(purpose: &str) -> ChatRequest {
    ChatRequest {
        model: "fixture-model".into(),
        system: None,
        messages: vec![Message {
            role: "user".into(),
            content: vec![ContentBlock::Text("Reply with OK".into())],
        }],
        tools: Vec::new(),
        max_tokens: 16,
        effort: None,
        thinking: ThinkingMode::Off,
        stream: true,
        metadata: RequestMetadata {
            purpose: purpose.into(),
            ref_id: None,
            provider_id: None,
        },
    }
}

fn text_sse() -> String {
    [
        r#"data: {"type":"message_start","message":{"id":"m1","usage":{"input_tokens":4,"output_tokens":1}}}"#,
        "",
        r#"data: {"type":"content_block_delta","delta":{"type":"text_delta","text":"OK"}}"#,
        "",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":1}}"#,
        "",
        r#"data: {"type":"message_stop"}"#,
        "",
    ]
    .join("\n")
}

fn tool_sse() -> String {
    [
        r#"data: {"type":"message_start","message":{"id":"m2","usage":{"input_tokens":4}}}"#,
        "",
        r#"data: {"type":"content_block_start","content_block":{"type":"tool_use","id":"tool-1","name":"echo"}}"#,
        "",
        r#"data: {"type":"content_block_delta","delta":{"type":"input_json_delta","partial_json":"{\"word\":\"OK\"}"}}"#,
        "",
        r#"data: {"type":"content_block_stop"}"#,
        "",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":2}}"#,
        "",
        r#"data: {"type":"message_stop"}"#,
        "",
    ]
    .join("\n")
}

fn openai_sse(tool: bool) -> String {
    if tool {
        [
            r#"data: {"id":"c1","choices":[{"delta":{"tool_calls":[{"index":0,"id":"tool-1","function":{"name":"echo","arguments":"{\"word\":\"OK\"}"}}]}}]}"#,
            "",
            r#"data: {"id":"c1","choices":[{"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":4,"completion_tokens":2}}"#,
            "",
            "data: [DONE]",
            "",
        ]
        .join("\n")
    } else {
        [
            r#"data: {"id":"c1","choices":[{"delta":{"content":"OK"}}]}"#,
            "",
            r#"data: {"id":"c1","choices":[{"finish_reason":"stop"}],"usage":{"prompt_tokens":4,"completion_tokens":1,"prompt_tokens_details":{"cached_tokens":1}}}"#,
            "",
            "data: [DONE]",
            "",
        ]
        .join("\n")
    }
}

type Handler = dyn Fn(&str, &str, &str, &str) -> (u16, String) + Send + Sync;

struct Server {
    base: String,
    shutdown: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<()>>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        if let Some(addr) = self.base.strip_prefix("http://") {
            let _ = TcpStream::connect(addr);
        }
        if let Some(handle) = self.join.take() {
            let _ = handle.join();
        }
    }
}

fn serve(handler: Arc<Handler>) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let shutdown = Arc::new(AtomicBool::new(false));
    let flag = shutdown.clone();
    let join = thread::spawn(move || {
        while !flag.load(Ordering::SeqCst) {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            if flag.load(Ordering::SeqCst) {
                break;
            }
            let _ = stream.set_read_timeout(Some(Duration::from_millis(400)));
            let (method, path, headers, body) = read_http(&mut stream);
            let (status, payload) = handler(&method, &path, &headers, &body);
            let packet = format!(
                "HTTP/1.1 {status} TEST\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                payload.len()
            );
            let _ = stream.write_all(packet.as_bytes());
        }
    });
    Server {
        base: format!("http://127.0.0.1:{port}"),
        shutdown,
        join: Some(join),
    }
}

fn read_http(stream: &mut TcpStream) -> (String, String, String, String) {
    let mut buf = Vec::new();
    let mut tmp = [0_u8; 4096];
    loop {
        match stream.read(&mut tmp) {
            Ok(0) | Err(_) => break,
            Ok(count) => {
                buf.extend_from_slice(&tmp[..count]);
                if let Some(end) = buf.windows(4).position(|window| window == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&buf[..end]).to_string();
                    let mut body = buf[end + 4..].to_vec();
                    let need = content_length(&header);
                    while body.len() < need {
                        match stream.read(&mut tmp) {
                            Ok(0) | Err(_) => break,
                            Ok(count) => body.extend_from_slice(&tmp[..count]),
                        }
                    }
                    let first = header.lines().next().unwrap_or("");
                    let mut parts = first.split_whitespace();
                    let method = parts.next().unwrap_or("").to_string();
                    let path = parts.next().unwrap_or("").to_string();
                    let text = String::from_utf8_lossy(&body[..need.min(body.len())]).into_owned();
                    return (method, path, header, text);
                }
            }
        }
    }
    (String::new(), String::new(), String::new(), String::new())
}

fn content_length(header: &str) -> usize {
    header
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                value.trim().parse().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}

fn sample_config(
    kind: ProviderKind,
    flavor: Option<Flavor>,
    base: &str,
    program: Option<String>,
) -> ProviderConfig {
    ProviderConfig {
        id: "provider-1".into(),
        name: "Fixture".into(),
        kind,
        flavor,
        base_url: Some(base.into()),
        default_model: Some("fixture-model".into()),
        auth_mode: "keyring".into(),
        auth_env_var: None,
        cli_program: program,
        proxy_mode: "none".into(),
        supports_tools: true,
        max_retries: 2,
        max_concurrency: 4,
        role_generator: false,
        role_grader: true,
        role_executor: true,
    }
}

struct Flaky {
    hits: AtomicUsize,
}

impl ChatProvider for Flaky {
    fn id(&self) -> &str {
        "flaky"
    }
    fn kind_name(&self) -> &'static str {
        "flaky"
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            tools: false,
            streaming: false,
            system_prompt: true,
        }
    }
    fn complete(
        &self,
        _request: &ChatRequest,
        sink: &mut dyn StreamSink,
        _cancel: &CancellationToken,
    ) -> Result<ChatResponse, ProviderError> {
        let hit = self.hits.fetch_add(1, Ordering::SeqCst);
        if hit < 2 {
            let mut err = ProviderError::from_status(429, "slow down");
            err.retry_after_ms = Some(7);
            return Err(err);
        }
        sink.on_event(&StreamEvent::TextDelta("OK".into()));
        let usage = Usage {
            input_tokens: 3,
            output_tokens: 1,
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
        };
        Ok(ChatResponse {
            id: "ok".into(),
            stop: StopReason::EndTurn,
            text: "OK".into(),
            tool_uses: Vec::new(),
            usage,
            incomplete_tool: false,
        })
    }
    fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(Vec::new())
    }
    fn count_tokens(&self, _request: &ChatRequest) -> Result<Option<u64>, ProviderError> {
        Ok(None)
    }
}

struct Bomb {
    hits: AtomicUsize,
}

impl ChatProvider for Bomb {
    fn id(&self) -> &str {
        "bomb"
    }
    fn kind_name(&self) -> &'static str {
        "bomb"
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            tools: false,
            streaming: false,
            system_prompt: false,
        }
    }
    fn complete(
        &self,
        _request: &ChatRequest,
        _sink: &mut dyn StreamSink,
        _cancel: &CancellationToken,
    ) -> Result<ChatResponse, ProviderError> {
        self.hits.fetch_add(1, Ordering::SeqCst);
        Err(ProviderError::new(ErrorClass::Other, "bomb was called"))
    }
    fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(Vec::new())
    }
    fn count_tokens(&self, _request: &ChatRequest) -> Result<Option<u64>, ProviderError> {
        Ok(None)
    }
}

#[test]
fn scrub_removes_secrets_from_logs_payloads_and_debug() {
    let secret = "hive-fixture-secret-value";
    let leaked = format!("token={secret} tail");
    let cleaned = scrub(&leaked, &[secret.to_string()]);
    assert!(!cleaned.contains(secret));
    assert!(cleaned.contains("<redacted>"));
    let vendor = scrub("sk-ant-endpoint-fixture-secret", &[]);
    assert!(!vendor.contains("sk-ant-endpoint-fixture-secret"));
    let hidden = SecretString::new(secret);
    assert!(!format!("{hidden:?}").contains(secret));
    assert_eq!(hidden.expose(), secret);
}

#[test]
fn public_provider_json_never_contains_the_secret() {
    let config = sample_config(ProviderKind::Anthropic, None, "http://127.0.0.1:9", None);
    let body = provider_public_json(&config, true);
    assert_eq!(body["has_secret"], true);
    assert!(body.get("secret").is_none());
    assert!(body.get("api_key").is_none());
    assert!(!body.to_string().contains("sk-"));
}

#[test]
fn pricing_uses_the_published_table_and_a_user_override() {
    let opus = Pricing::anthropic_defaults("claude-opus-5-5").expect("opus");
    let cost = cost_usd(&opus, 1_000_000, 1_000_000, 1_000_000, 1_000_000);
    assert!((cost - 29.4).abs() < 0.001);
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::migrations::run_migrations(&conn).unwrap();
    conn.execute(
        "INSERT INTO model_pricing (id, provider_config_id, model, input_per_mtok, output_per_mtok, cache_read_per_mtok, cache_write_per_mtok, source, updated_at) VALUES ('custom', 'provider-1', 'fixture-model', 1, 2, 0, 0, 'user', 0)",
        [],
    )
    .unwrap();
    let row = lookup(&conn, "provider-1", "fixture-model");
    assert!((row.input_per_mtok - 1.0).abs() < 0.001);
    assert!((lookup(&conn, "missing", "claude-haiku-5-5").input_per_mtok - 0.10).abs() < 0.001);
    assert_eq!(seed_models().len(), 4);
}

#[test]
fn retry_honours_retry_after_and_stops_after_success() {
    let flaky = Flaky {
        hits: AtomicUsize::new(0),
    };
    let sleeper = NoopSleeper::new();
    let slept = sleeper.slept.clone();
    let middleware = Middleware {
        mode: RunMode::Live,
        recordings_dir: Path::new("unused").into(),
        kind: "anthropic".into(),
        max_retries: 3,
        limiter: Limiter::new(4),
        sleeper: Box::new(sleeper),
        secrets: Vec::new(),
        pricing: Pricing {
            input_per_mtok: 1.0,
            output_per_mtok: 1.0,
            cache_read_per_mtok: 0.0,
            cache_write_per_mtok: 0.0,
        },
        provider_id: "flaky".into(),
        hits: Mutex::new(0),
    };
    let mut sink = VecSink::default();
    let (response, row) = middleware
        .complete(
            &flaky,
            &request("executor"),
            &mut sink,
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(response.text, "OK");
    assert_eq!(flaky.hits.load(Ordering::SeqCst), 3);
    assert_eq!(slept.lock().unwrap().as_slice(), &[7, 7]);
    assert!((row.cost_usd - cost_usd(&middleware.pricing, 3, 1, 0, 0)).abs() < 0.000_001);
}

#[test]
fn noop_sleeper_records_retry_after() {
    let flaky = Flaky {
        hits: AtomicUsize::new(0),
    };
    let sleeper = NoopSleeper::new();
    let middleware = Middleware {
        mode: RunMode::Live,
        recordings_dir: std::env::temp_dir(),
        kind: "anthropic".into(),
        max_retries: 3,
        limiter: Limiter::new(2),
        sleeper: Box::new(sleeper),
        secrets: Vec::new(),
        pricing: Pricing::anthropic_defaults("claude-haiku-5-5").unwrap(),
        provider_id: "flaky".into(),
        hits: Mutex::new(0),
    };
    let mut sink = VecSink::default();
    middleware
        .complete(
            &flaky,
            &request("executor"),
            &mut sink,
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(*middleware.hits.lock().unwrap(), 3);
}

#[test]
fn limiter_peak_stays_at_the_cap() {
    let limiter = Arc::new(Limiter::new(4));
    let mut handles = Vec::new();
    for _ in 0..8 {
        let limiter = limiter.clone();
        handles.push(thread::spawn(move || {
            let permit = limiter.acquire(&CancellationToken::new()).unwrap();
            thread::sleep(Duration::from_millis(30));
            drop(permit);
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
    assert_eq!(*limiter.peak.lock().unwrap(), 4);
}

#[test]
fn missing_recording_does_not_call_the_provider() {
    let bomb = Bomb {
        hits: AtomicUsize::new(0),
    };
    let dir = tempfile::tempdir().unwrap();
    let middleware = Middleware {
        mode: RunMode::Replay,
        recordings_dir: dir.path().to_path_buf(),
        kind: "anthropic".into(),
        max_retries: 0,
        limiter: Limiter::new(1),
        sleeper: Box::new(NoopSleeper::new()),
        secrets: Vec::new(),
        pricing: Pricing {
            input_per_mtok: 0.0,
            output_per_mtok: 0.0,
            cache_read_per_mtok: 0.0,
            cache_write_per_mtok: 0.0,
        },
        provider_id: "bomb".into(),
        hits: Mutex::new(0),
    };
    let mut sink = VecSink::default();
    let err = middleware
        .complete(
            &bomb,
            &request("executor"),
            &mut sink,
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.class, ErrorClass::RecordingMissing);
    assert!(err.message.contains("recording_missing"));
    assert_eq!(bomb.hits.load(Ordering::SeqCst), 0);
    assert_eq!(*middleware.hits.lock().unwrap(), 0);
}

#[test]
fn record_then_replay_is_stable_and_costs_zero() {
    let dir = tempfile::tempdir().unwrap();
    let scripted = ScriptedProvider::new("scripted");
    let live = Middleware {
        mode: RunMode::Record,
        recordings_dir: dir.path().to_path_buf(),
        kind: "anthropic".into(),
        max_retries: 0,
        limiter: Limiter::new(1),
        sleeper: Box::new(NoopSleeper::new()),
        secrets: vec!["sk-ant-endpoint-fixture-secret".into()],
        pricing: Pricing::anthropic_defaults("claude-opus-5-5").unwrap(),
        provider_id: "scripted".into(),
        hits: Mutex::new(0),
    };
    let mut sink = VecSink::default();
    let (first, _) = live
        .complete(
            &scripted,
            &request("executor"),
            &mut sink,
            &CancellationToken::new(),
        )
        .unwrap();
    let bomb = Bomb {
        hits: AtomicUsize::new(0),
    };
    let replay = Middleware {
        mode: RunMode::Replay,
        recordings_dir: dir.path().to_path_buf(),
        kind: "anthropic".into(),
        max_retries: 0,
        limiter: Limiter::new(1),
        sleeper: Box::new(NoopSleeper::new()),
        secrets: Vec::new(),
        pricing: live.pricing,
        provider_id: "scripted".into(),
        hits: Mutex::new(0),
    };
    let mut again = VecSink::default();
    let (second, row) = replay
        .complete(
            &bomb,
            &request("executor"),
            &mut again,
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(bomb.hits.load(Ordering::SeqCst), 0);
    assert_eq!(first.text, second.text);
    assert_eq!(row.status, "replayed");
    assert_eq!(row.cost_usd, 0.0);
    let raw = std::fs::read_dir(dir.path().join("anthropic"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap();
    let bytes = std::fs::read_to_string(raw.path()).unwrap();
    assert_eq!(recording_diff(&bytes, &bytes), "recordings match");
    assert!(recording_exists(
        dir.path(),
        "anthropic",
        &request("executor")
    ));
}

#[test]
fn anthropic_openai_and_ollama_answer_on_localhost() {
    let secret = "sk-ant-endpoint-fixture-secret";
    let calls = Arc::new(AtomicUsize::new(0));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let calls_h = calls.clone();
    let seen_h = seen.clone();
    let server = serve(Arc::new(move |method, path, headers, body| {
        calls_h.fetch_add(1, Ordering::SeqCst);
        seen_h.lock().unwrap().push(format!("{method} {path}"));
        assert!(!body.contains("temperature"));
        if path.contains("/models") && !path.contains("api/tags") {
            if headers.to_ascii_lowercase().contains("x-api-key") {
                return (200, r#"{"data":[{"id":"fixture-model"}]}"#.into());
            }
            return (200, r#"{"data":[]}"#.into());
        }
        if path.ends_with("/api/tags") {
            return (200, r#"{"models":[{"name":"fixture-model"}]}"#.into());
        }
        if path.contains("count_tokens") {
            return (200, r#"{"input_tokens":9}"#.into());
        }
        let tool = body.contains("\"echo\"");
        if path.contains("chat/completions") {
            return (200, openai_sse(tool));
        }
        (200, if tool { tool_sse() } else { text_sse() })
    }));
    let anthropic = AnthropicProvider::new("anthropic", &server.base, SecretString::new(secret));
    let mut sink = VecSink::default();
    let reply = anthropic
        .complete(&request("executor"), &mut sink, &CancellationToken::new())
        .unwrap();
    assert_eq!(reply.text, "OK");
    assert_eq!(anthropic.count_tokens(&request("grader")).unwrap(), Some(9));
    let models = anthropic.list_models().unwrap();
    assert_eq!(models[0].id, "fixture-model");
    let ollama = OpenAiProvider::new(
        "ollama",
        format!("{}/v1", server.base),
        Flavor::Ollama,
        SecretString::new(""),
    );
    assert_eq!(ollama.list_models().unwrap()[0].id, "fixture-model");
    let mut tool_request = request("test_connection");
    tool_request.tools.push(ToolDef {
        name: "echo".into(),
        description: "Echo".into(),
        input_schema: json!({"type":"object"}),
    });
    let mut tool_sink = VecSink::default();
    let tool = ollama
        .complete(&tool_request, &mut tool_sink, &CancellationToken::new())
        .unwrap();
    assert_eq!(tool.stop, StopReason::ToolUse);
    assert!(calls.load(Ordering::SeqCst) > 0);
    let joined = seen.lock().unwrap().join("\n");
    assert!(joined.contains("/v1/messages") || joined.contains("/v1/models"));
    assert!(!joined.contains("api.anthropic.com"));
    assert!(!joined.contains("api.openai.com"));
}

#[test]
fn http_429_and_a_truncated_stream_are_retried() {
    let hits = Arc::new(AtomicUsize::new(0));
    let hits_h = hits.clone();
    let server = serve(Arc::new(move |_method, path, _headers, _body| {
        let hit = hits_h.fetch_add(1, Ordering::SeqCst);
        if path.contains("count") {
            return (200, r#"{"input_tokens":1}"#.into());
        }
        if hit == 0 {
            return (429, "slow".into());
        }
        if hit == 1 {
            return (200, "data: {\"type\":\"ping\"}\n\n".into());
        }
        (200, text_sse())
    }));
    let provider = AnthropicProvider::new(
        "anthropic",
        &server.base,
        SecretString::new("sk-ant-endpoint-fixture-secret"),
    );
    let middleware = Middleware {
        mode: RunMode::Live,
        recordings_dir: std::env::temp_dir(),
        kind: "anthropic".into(),
        max_retries: 4,
        limiter: Limiter::new(1),
        sleeper: Box::new(NoopSleeper::new()),
        secrets: Vec::new(),
        pricing: Pricing {
            input_per_mtok: 0.0,
            output_per_mtok: 0.0,
            cache_read_per_mtok: 0.0,
            cache_write_per_mtok: 0.0,
        },
        provider_id: "anthropic".into(),
        hits: Mutex::new(0),
    };
    let mut sink = VecSink::default();
    let (response, _) = middleware
        .complete(
            &provider,
            &request("executor"),
            &mut sink,
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(response.text, "OK");
    assert!(hits.load(Ordering::SeqCst) >= 3);
}

#[test]
fn seven_stage_probe_covers_anthropic_ollama_and_claude_cli() {
    let secret = "sk-ant-endpoint-fixture-secret";
    let server = serve(Arc::new(|_method, path, headers, body| {
        if path.ends_with("/api/tags") {
            return (200, r#"{"models":[{"name":"fixture-model"}]}"#.into());
        }
        if path.contains("/models") {
            if headers.to_ascii_lowercase().contains("x-api-key") {
                return (200, r#"{"data":[{"id":"fixture-model"}]}"#.into());
            }
            return (200, r#"{"data":[]}"#.into());
        }
        let tool = body.contains("\"echo\"");
        if path.contains("chat/completions") {
            return (200, openai_sse(tool));
        }
        (200, if tool { tool_sse() } else { text_sse() })
    }));
    let anthropic = sample_config(ProviderKind::Anthropic, None, &server.base, None);
    let report = probe_provider(
        &anthropic,
        &SecretString::new(secret),
        &CancellationToken::new(),
    );
    assert_eq!(
        report.stages.len(),
        7,
        "{:?}",
        report
            .stages
            .iter()
            .map(|stage| (&stage.name, stage.ok, &stage.detail))
            .collect::<Vec<_>>()
    );
    assert!(report.ok, "{:?}", report.stages);
    let dumped = serde_json::to_string(&report).unwrap();
    assert!(!dumped.contains(secret));
    let ollama_base = format!("{}/v1", server.base);
    let mut ollama = sample_config(
        ProviderKind::OpenaiCompat,
        Some(Flavor::Ollama),
        &ollama_base,
        None,
    );
    ollama.id = "ollama".into();
    ollama.name = "Ollama".into();
    let ollama_report = probe_provider(&ollama, &SecretString::new(""), &CancellationToken::new());
    assert!(
        ollama_report
            .stages
            .iter()
            .any(|stage| stage.name == "models" && stage.ok),
        "{:?}",
        ollama_report.stages
    );
    let dir = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    let program = {
        let program = dir.path().join("claude");
        std::fs::write(
            &program,
            "#!/bin/sh\ncase \"$1\" in\n  --version) echo fixture-cli;;\n  --help) echo '--agents --agent --model --max-turns --output-format';;\n  *) printf '%s\\n' '{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"OK\"}]}}' '{\"type\":\"result\",\"usage\":{\"input_tokens\":2,\"output_tokens\":1},\"duration_ms\":5}';;\nesac\n",
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&program).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&program, permissions).unwrap();
        program
    };
    #[cfg(windows)]
    let program = {
        let program = dir.path().join("claude.cmd");
        std::fs::write(
            &program,
            "@echo off\r\nif \"%~1\"==\"--version\" goto version\r\nif \"%~1\"==\"--help\" goto help\r\necho {\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"OK\"}]}}\r\necho {\"type\":\"result\",\"usage\":{\"input_tokens\":2,\"output_tokens\":1},\"duration_ms\":5}\r\ngoto end\r\n:version\r\necho fixture-cli\r\ngoto end\r\n:help\r\necho --agents --agent --model --max-turns --output-format\r\n:end\r\n",
        )
        .unwrap();
        program
    };
    let mut cli = sample_config(
        ProviderKind::LocalCli,
        Some(Flavor::ClaudeCli),
        &server.base,
        Some(program.to_string_lossy().into_owned()),
    );
    cli.default_model = None;
    cli.auth_mode = "none".into();
    cli.supports_tools = false;
    let cli_report = probe_provider(&cli, &SecretString::new(""), &CancellationToken::new());
    assert_eq!(cli_report.stages.len(), 7, "{:?}", cli_report.stages);
    assert!(
        cli_report
            .stages
            .iter()
            .any(|stage| stage.name == "cli" && stage.ok),
        "{:?}",
        cli_report.stages
    );
    let found = detect_local_agent_clis(&[dir.path().to_path_buf()]);
    assert!(found.iter().any(|item| item.name == "claude"));
    let flags = help_flags(&program);
    assert!(flags.iter().any(|flag| flag == "--model"));
}

#[test]
fn store_round_trip_keeps_last_test_and_reference_state() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::migrations::run_migrations(&conn).unwrap();
    let config = sample_config(ProviderKind::Anthropic, None, "http://127.0.0.1:9", None);
    upsert_provider(&conn, &config).unwrap();
    set_last_test(&conn, &config.id, true, 12, None).unwrap();
    let loaded = get_provider(&conn, &config.id).unwrap().unwrap();
    assert_eq!(loaded.kind, ProviderKind::Anthropic);
    assert!(!referenced_by_preset(&conn, &config.id).unwrap());
    assert_eq!(list_providers(&conn).unwrap().len(), 1);
}

#[cfg(unix)]
#[test]
fn process_group_kill_stops_the_grandchild() {
    let dir = tempfile::tempdir().unwrap();
    let pid_path = dir.path().join("child.pid");
    let script = format!("sleep 30 & echo $! > '{}'; wait", pid_path.display());
    let mut command = std::process::Command::new("sh");
    command.arg("-c").arg(script);
    let mut child = spawn_grouped(&mut command).unwrap();
    let grandchild = wait_pid(&pid_path);
    kill_tree(&mut child).unwrap();
    thread::sleep(Duration::from_millis(150));
    assert_eq!(unsafe { libc::kill(grandchild, 0) }, -1);
}

#[cfg(unix)]
fn wait_pid(path: &Path) -> i32 {
    for _ in 0..50 {
        if let Ok(text) = std::fs::read_to_string(path) {
            if let Ok(pid) = text.trim().parse::<i32>() {
                return pid;
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("grandchild pid was not written");
}

#[cfg(windows)]
#[test]
fn job_object_kills_the_grandchild() {
    let dir = tempfile::tempdir().unwrap();
    let started = dir.path().join("started.txt");
    let finished = dir.path().join("finished.txt");
    let inner = format!(
        "cmd /c \"echo started>{} & ping -n 30 127.0.0.1 >nul & echo finished>{}\"",
        started.display(),
        finished.display()
    );
    let args = vec!["/c".into(), inner];
    let child = cli_agent::spawn_killed_with_job("cmd", &args, dir.path()).unwrap();
    for _ in 0..50 {
        if started.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(started.exists(), "grandchild did not start");
    cli_agent::kill_job(&child);
    thread::sleep(Duration::from_millis(300));
    assert!(
        !finished.exists(),
        "grandchild kept running after the job was terminated"
    );
}
