//! Provider commands. Live probes only POST to a configured local base URL.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use hive_core::library::central_repo;
use hive_core::providers::{
    list_named_providers, probe_offline, save_named_provider, store_provider_secret,
    ProviderConfig, ProviderKind,
};
use serde_json::{json, Value};

use crate::Fail;

pub fn providers_list() -> Result<Value, Fail> {
    let rows = list_named_providers(&central_repo::db_path()).map_err(Fail::other)?;
    let providers: Vec<Value> = rows
        .iter()
        .map(|row| {
            json!({
                "id": row.id,
                "name": row.name,
                "kind": kind_name(row.kind),
                "model": row.default_model,
                "roles": role_names(row)
            })
        })
        .collect();
    Ok(json!({"command": "providers.list", "providers": providers}))
}

pub fn providers_add(
    id: &str,
    name: &str,
    kind: &str,
    model: Option<&str>,
    base_url: Option<&str>,
    roles: &[String],
) -> Result<Value, Fail> {
    let (mut role_generator, mut role_grader, mut role_executor) = default_roles(kind);
    if !roles.is_empty() {
        role_generator = false;
        role_grader = false;
        role_executor = false;
        for role in roles {
            match role.as_str() {
                "generator" => role_generator = true,
                "grader" => role_grader = true,
                "executor" => role_executor = true,
                other => {
                    return Err(Fail::usage(format!(
                        "role must be generator, grader, or executor (got {other})"
                    )))
                }
            }
        }
    }
    let chosen_model = model
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default_model_for(kind))
        .to_string();
    let config = ProviderConfig {
        id: id.into(),
        name: name.into(),
        kind: parse_kind(kind),
        flavor: None,
        base_url: base_url
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        default_model: Some(chosen_model.clone()),
        auth_mode: "keyring".into(),
        auth_env_var: None,
        cli_program: None,
        proxy_mode: "none".into(),
        supports_tools: true,
        max_retries: 2,
        max_concurrency: 4,
        role_generator,
        role_grader,
        role_executor,
    };
    save_named_provider(&central_repo::db_path(), &config).map_err(Fail::other)?;
    Ok(json!({
        "command": "providers.add",
        "id": id,
        "kind": kind,
        "model": chosen_model,
        "roles": role_names(&config)
    }))
}

pub fn providers_set_secret(id: &str, secret: &str) -> Result<Value, Fail> {
    let dir = central_repo::base_dir().join("provider-secrets");
    store_provider_secret(id, secret, &dir).map_err(|err| Fail::other(err.to_string()))?;
    Ok(json!({"command": "providers.set-secret", "id": id, "stored": true}))
}

pub fn providers_has_secret(id: &str) -> Result<Value, Fail> {
    let dir = central_repo::base_dir().join("provider-secrets");
    let has_secret = hive_core::providers::has_provider_secret(id, &dir);
    Ok(json!({"command": "providers.has-secret", "id": id, "has_secret": has_secret}))
}

pub fn providers_probe_offline(id: &str) -> Result<Value, Fail> {
    let config = require_provider(id)?;
    let report = probe_offline(&config);
    let dumped = serde_json::to_string(&report).unwrap_or_default();
    if dumped.contains("offline-secret") {
        return Err(Fail::other("probe leaked a secret"));
    }
    Ok(json!({
        "command": "providers.probe",
        "id": id,
        "ok": report.ok,
        "stages": report.stages.len() as i64
    }))
}

/// POST to the provider's configured base URL when it is a local HTTP server.
/// A missing server, or any non-local host, returns the offline probe with
/// `live: false` so the caller still gets the existing offline result.
pub fn providers_probe_live(id: &str) -> Result<Value, Fail> {
    let config = require_provider(id)?;
    let Some(url) = config.base_url.clone().filter(|url| is_local_http(url)) else {
        return Err(Fail::other(
            "live vendor probes are unavailable without an API key",
        ));
    };
    let body = json!({
        "id": id,
        "model": config.default_model,
    })
    .to_string();
    match post_local(&url, &body) {
        Ok(response) => {
            let parsed: Value = serde_json::from_str(&response).unwrap_or(Value::Null);
            let ok = parsed.get("ok").and_then(Value::as_bool).unwrap_or(true);
            let stages = parsed.get("stages").and_then(Value::as_i64).unwrap_or(1);
            Ok(json!({
                "command": "providers.probe",
                "id": id,
                "ok": ok,
                "stages": stages,
                "live": true,
                "body": parsed
            }))
        }
        Err(_) => {
            let mut offline = providers_probe_offline(id)?;
            if let Some(object) = offline.as_object_mut() {
                object.insert("live".into(), json!(false));
                object.insert("refused".into(), json!(true));
            }
            Ok(offline)
        }
    }
}

pub(crate) struct MeasureTarget {
    pub url: String,
    pub kind: String,
    pub model: String,
    pub id: String,
}

/// The first configured provider, including vendor hosts. `test measure` uses this.
/// `post_local` still refuses those hosts.
pub(crate) fn measure_target() -> Result<MeasureTarget, String> {
    let rows = list_named_providers(&central_repo::db_path()).map_err(|err| err.to_string())?;
    let row = rows
        .into_iter()
        .find(|row| row.base_url.as_ref().is_some_and(|url| !url.is_empty()))
        .ok_or_else(|| "add a provider with a base URL before measuring".to_string())?;
    let kind = match row.kind {
        ProviderKind::OpenaiCompat => "openai",
        ProviderKind::Anthropic => "anthropic",
        ProviderKind::LocalCli => "local_cli",
    };
    let model = row
        .default_model
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default_model_for(kind).to_string());
    Ok(MeasureTarget {
        url: row.base_url.unwrap_or_default(),
        kind: kind.to_string(),
        model,
        id: row.id,
    })
}

pub(crate) fn local_base_url() -> Option<String> {
    let rows = list_named_providers(&central_repo::db_path()).ok()?;
    rows.into_iter().find_map(|row| {
        let url = row.base_url?;
        is_local_http(&url).then_some(url)
    })
}

pub(crate) fn is_local_http(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    if lower.contains("api.openai.com")
        || lower.contains("api.anthropic.com")
        || lower.contains("openrouter.ai")
    {
        return false;
    }
    lower.starts_with("http://127.0.0.1") || lower.starts_with("http://localhost")
}

/// POST JSON to an `http://127.0.0.1` or `http://localhost` URL. Returns the
/// response body. Vendor hosts are refused before any socket is opened.
pub(crate) fn post_local(url: &str, body: &str) -> Result<String, String> {
    if !is_local_http(url) {
        return Err("refused non-local host".into());
    }
    let (host, port, path) = split_http(url)?;
    let addr = SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, port));
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(400))
        .map_err(|err| err.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|err| err.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|err| err.to_string())?;
    let payload = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(payload.as_bytes())
        .map_err(|err| err.to_string())?;
    let mut raw = String::new();
    stream
        .read_to_string(&mut raw)
        .map_err(|err| err.to_string())?;
    let (head, response_body) = raw.split_once("\r\n\r\n").unwrap_or(("", raw.as_str()));
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .unwrap_or(0);
    if !(200..300).contains(&status) {
        return Err(format!("probe POST returned {status}"));
    }
    Ok(response_body.to_string())
}

fn split_http(url: &str) -> Result<(String, u16, String), String> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| "only http:// is allowed".to_string())?;
    let (hostport, path) = match rest.split_once('/') {
        Some((hostport, path)) => (hostport, format!("/{path}")),
        None => (rest, "/".to_string()),
    };
    let (host, port) = match hostport.rsplit_once(':') {
        Some((host, port)) => {
            let port = port
                .parse::<u16>()
                .map_err(|_| format!("bad port in {url}"))?;
            (host.to_string(), port)
        }
        None => (hostport.to_string(), 80),
    };
    if host != "127.0.0.1" && host != "localhost" {
        return Err("refused non-local host".into());
    }
    Ok((host, port, path))
}

fn parse_kind(kind: &str) -> ProviderKind {
    match kind {
        "openai" | "openai_compat" => ProviderKind::OpenaiCompat,
        "cli" | "local_cli" => ProviderKind::LocalCli,
        _ => ProviderKind::Anthropic,
    }
}

fn kind_name(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Anthropic => "anthropic",
        ProviderKind::OpenaiCompat => "openai_compat",
        ProviderKind::LocalCli => "local_cli",
    }
}

fn default_model_for(kind: &str) -> &'static str {
    match kind {
        "openai" | "openai_compat" => "gpt-4o-mini",
        "cli" | "local_cli" => "local-cli",
        _ => "claude-sonnet-4",
    }
}

fn default_roles(kind: &str) -> (bool, bool, bool) {
    match kind {
        "openai" | "openai_compat" => (false, false, true),
        "cli" | "local_cli" => (true, false, false),
        _ => (true, true, false),
    }
}

fn role_names(config: &ProviderConfig) -> Vec<&'static str> {
    let mut roles = Vec::new();
    if config.role_generator {
        roles.push("generator");
    }
    if config.role_grader {
        roles.push("grader");
    }
    if config.role_executor {
        roles.push("executor");
    }
    roles
}

fn require_provider(id: &str) -> Result<ProviderConfig, Fail> {
    if id.is_empty() {
        return Err(Fail::other("provider id is required"));
    }
    list_named_providers(&central_repo::db_path())
        .map_err(Fail::other)?
        .into_iter()
        .find(|row| row.id == id)
        .ok_or_else(|| Fail::other(format!("unknown provider {id}")))
}

pub fn providers_test(id: &str) -> Result<Value, Fail> {
    let config = require_provider(id)?;
    let report = probe_offline(&config);
    Ok(json!({
        "command": "providers.test",
        "id": id,
        "ok": report.ok,
        "stages": report.stages.len() as i64
    }))
}

pub fn providers_models(id: &str) -> Result<Value, Fail> {
    let config = require_provider(id)?;
    let mut models: Vec<String> = config.default_model.into_iter().collect();
    if models.is_empty() {
        return Err(Fail::other(format!("provider {id} has no models")));
    }
    models.sort();
    models.dedup();
    Ok(json!({
        "command": "providers.models",
        "id": id,
        "models": models
    }))
}
