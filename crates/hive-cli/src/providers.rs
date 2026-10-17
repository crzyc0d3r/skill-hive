//! Offline provider commands. They never call a vendor API.

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
        .map(|row| json!({"id": row.id, "name": row.name, "kind": kind_name(row.kind)}))
        .collect();
    Ok(json!({"command": "providers.list", "providers": providers}))
}
pub fn providers_add(id: &str, name: &str, kind: &str) -> Result<Value, Fail> {
    let config = ProviderConfig {
        id: id.into(),
        name: name.into(),
        kind: parse_kind(kind),
        flavor: None,
        base_url: None,
        default_model: Some(default_model_for(kind).into()),
        auth_mode: "keyring".into(),
        auth_env_var: None,
        cli_program: None,
        proxy_mode: "none".into(),
        supports_tools: true,
        max_retries: 2,
        max_concurrency: 4,
        role_generator: false,
        role_grader: true,
        role_executor: true,
    };
    save_named_provider(&central_repo::db_path(), &config).map_err(Fail::other)?;
    Ok(json!({"command": "providers.add", "id": id, "kind": kind}))
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
    let rows = list_named_providers(&central_repo::db_path()).map_err(Fail::other)?;
    let config = rows
        .into_iter()
        .find(|row| row.id == id)
        .ok_or_else(|| Fail::other(format!("unknown provider {id}")))?;
    let report = probe_offline(&config);
    let dumped = serde_json::to_string(&report).unwrap_or_default();
    if dumped.contains("offline-secret") {
        return Err(Fail::other("probe leaked a secret"));
    }
    Ok(
        json!({"command": "providers.probe", "id": id, "ok": report.ok, "stages": report.stages.len()}),
    )
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

fn require_provider(id: &str) -> Result<hive_core::providers::ProviderConfig, Fail> {
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
    Ok(
        json!({"command": "providers.test", "id": id, "ok": report.ok, "stages": report.stages.len()}),
    )
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
