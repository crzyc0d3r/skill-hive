//! Offline provider and test commands. They never call a vendor API.

use std::fs;
use std::path::Path;

use hive_core::library::central_repo;
use hive_core::providers::{
    list_named_providers, probe_offline, save_named_provider, store_provider_secret,
    ProviderConfig, ProviderKind, RunMode, ScriptedProvider,
};
use hive_core::testing::{run_plan, save_suite, CheckSpec, Plan, Suite, SuiteCase};
use serde_json::{json, Value};

use super::Fail;

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
        default_model: Some("fixture-model".into()),
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

pub fn test_save_suite(skill: &Path) -> Result<Value, Fail> {
    fs::create_dir_all(skill.join("evals")).map_err(|err| Fail::other(err.to_string()))?;
    fs::write(
        skill.join("SKILL.md"),
        "---\nname: fixture-skill\n---\nHIVE_SKILL_MARKER_9f3a\n",
    )
    .map_err(|err| Fail::other(err.to_string()))?;
    let suite = Suite {
        skill_name: "fixture-skill".into(),
        evals: vec![SuiteCase {
            id: 1,
            prompt: "Write the fixture answer.".into(),
            expected_output: "PINECONE".into(),
            files: Vec::new(),
            expectations: vec!["answer file exists".into()],
        }],
        triggers: Vec::new(),
        checks: vec![CheckSpec {
            id: "file".into(),
            expectation: "answer file exists".into(),
            kind: "file_exists".into(),
            path: "outputs/answer.txt".into(),
            pattern: None,
        }],
    };
    save_suite(skill, &suite).map_err(|err| Fail::other(err.to_string()))?;
    Ok(json!({"command": "test.save-suite", "skill": "fixture-skill"}))
}

pub fn test_run(skill: &Path) -> Result<Value, Fail> {
    let root = central_repo::base_dir().join("test-runs");
    fs::create_dir_all(&root).map_err(|err| Fail::other(err.to_string()))?;
    let spec = Plan {
        skill_dir: skill.to_path_buf(),
        workspace: root.join("workspace"),
        db_path: root.join("hive.db"),
        mode: RunMode::Live,
        seed: 4,
        repetitions: 1,
        budget_usd: None,
        cancel_after: None,
        crash_after: None,
        recordings: root.join("recordings"),
        personal_skills: None,
        secret: String::new(),
        usd_per_cell: None,
        model: "fixture-model".into(),
        kind: "scripted".into(),
    };
    let report = run_plan(&spec, &ScriptedProvider::new("offline"))
        .map_err(|err| Fail::other(err.to_string()))?;
    fs::write(
        root.join("last.json"),
        json!({"run_id": report.run_id, "status": report.status}).to_string(),
    )
    .map_err(|err| Fail::other(err.to_string()))?;
    Ok(json!({"command": "test.run", "status": report.status, "run_id": report.run_id}))
}

pub fn test_show() -> Result<Value, Fail> {
    let path = central_repo::base_dir().join("test-runs").join("last.json");
    let raw = fs::read_to_string(&path).map_err(|err| Fail::other(err.to_string()))?;
    let parsed: Value = serde_json::from_str(&raw).map_err(|err| Fail::other(err.to_string()))?;
    Ok(json!({"command": "test.show", "status": parsed["status"], "run_id": parsed["run_id"]}))
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
