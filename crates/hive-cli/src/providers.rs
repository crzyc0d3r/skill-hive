//! Provider commands. Live probes call the configured adapter.

use hive_core::library::central_repo;
use hive_core::providers::{
    list_named_providers, load_provider_secret, probe_offline, probe_provider, save_named_provider,
    store_provider_secret, CancellationToken, ProviderConfig, ProviderKind, SecretString,
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

/// Probe the configured base URL with the vendor wire protocol.
/// A missing key returns an error. A closed port returns the failed live stages.
pub fn providers_probe_live(id: &str) -> Result<Value, Fail> {
    let config = require_provider(id)?;
    if !config.base_url.as_ref().is_some_and(|url| !url.is_empty()) {
        return Err(Fail::other(
            "live vendor probes are unavailable without an API key",
        ));
    }
    let secret = std::env::var("SKILL_HIVE_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            load_provider_secret(id, &central_repo::base_dir().join("provider-secrets")).ok()
        });
    let Some(secret) = secret else {
        return Err(Fail::other(
            "live vendor probes are unavailable without an API key",
        ));
    };
    let report = probe_provider(
        &config,
        &SecretString::new(secret),
        &CancellationToken::new(),
    );
    Ok(json!({
        "command": "providers.probe",
        "id": id,
        "ok": report.ok,
        "stages": report.stages.len() as i64,
        "live": report.ok,
        "refused": !report.ok,
    }))
}

pub(crate) struct MeasureTarget {
    pub url: String,
    pub kind: String,
    pub model: String,
    pub id: String,
}

/// The first configured provider, including vendor hosts. Measured runs use this.
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
    let secret = std::env::var("SKILL_HIVE_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            load_provider_secret(id, &central_repo::base_dir().join("provider-secrets")).ok()
        });
    let (Some(url), Some(secret)) = (config.base_url.as_ref(), secret) else {
        return Err(Fail::other(
            "providers test needs a base URL and SKILL_HIVE_API_KEY or a stored provider secret",
        ));
    };
    if url.is_empty() {
        return Err(Fail::other(
            "providers test needs a base URL and SKILL_HIVE_API_KEY or a stored provider secret",
        ));
    }
    let report = probe_provider(
        &config,
        &SecretString::new(secret),
        &CancellationToken::new(),
    );
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
