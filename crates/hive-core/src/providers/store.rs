//! Provider rows. Secrets are never columns and never appear in the public JSON.

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use serde_json::{json, Value};

use super::types::{Flavor, ProviderKind};

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub kind: ProviderKind,
    pub flavor: Option<Flavor>,
    pub base_url: Option<String>,
    pub default_model: Option<String>,
    pub auth_mode: String,
    pub auth_env_var: Option<String>,
    pub cli_program: Option<String>,
    pub proxy_mode: String,
    pub supports_tools: bool,
    pub max_retries: u32,
    pub max_concurrency: u32,
    pub role_generator: bool,
    pub role_grader: bool,
    pub role_executor: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderPublic {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub flavor: Option<String>,
    pub base_url: Option<String>,
    pub default_model: Option<String>,
    pub auth_mode: String,
    pub has_secret: bool,
    pub role_generator: bool,
}

pub fn save_named_provider(db: &std::path::Path, config: &ProviderConfig) -> Result<(), String> {
    let conn = open_file(db)?;
    upsert_provider(&conn, config).map_err(|err| err.to_string())
}

pub fn list_named_providers(db: &std::path::Path) -> Result<Vec<ProviderConfig>, String> {
    let conn = open_file(db)?;
    list_providers(&conn).map_err(|err| err.to_string())
}

fn open_file(db: &std::path::Path) -> Result<Connection, String> {
    if let Some(parent) = db.parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let conn = Connection::open(db).map_err(|err| err.to_string())?;
    crate::db::migrations::run_migrations(&conn).map_err(|err| err.to_string())?;
    Ok(conn)
}

pub fn upsert_provider(conn: &Connection, config: &ProviderConfig) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO provider_configs
         (id, name, kind, flavor, base_url, default_model, auth_mode, auth_env_var, keyring_account, cli_program, proxy_mode, supports_tools, max_retries, max_concurrency, role_generator, role_grader, role_executor, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?1, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, 0, 0)
         ON CONFLICT(id) DO UPDATE SET
           name = excluded.name,
           kind = excluded.kind,
           flavor = excluded.flavor,
           base_url = excluded.base_url,
           default_model = excluded.default_model,
           auth_mode = excluded.auth_mode,
           auth_env_var = excluded.auth_env_var,
           cli_program = excluded.cli_program,
           proxy_mode = excluded.proxy_mode,
           supports_tools = excluded.supports_tools,
           max_retries = excluded.max_retries,
           max_concurrency = excluded.max_concurrency,
           role_generator = excluded.role_generator,
           role_grader = excluded.role_grader,
           role_executor = excluded.role_executor,
           updated_at = 0",
        (
            config.id.as_str(),
            config.name.as_str(),
            kind_str(config.kind),
            config.flavor.map(flavor_str),
            config.base_url.as_deref(),
            config.default_model.as_deref(),
            config.auth_mode.as_str(),
            config.auth_env_var.as_deref(),
            config.cli_program.as_deref(),
            config.proxy_mode.as_str(),
            i64::from(config.supports_tools),
            config.max_retries,
            config.max_concurrency,
            i64::from(config.role_generator),
            i64::from(config.role_grader),
            i64::from(config.role_executor),
        ),
    )?;
    Ok(())
}

pub fn list_providers(conn: &Connection) -> rusqlite::Result<Vec<ProviderConfig>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, kind, flavor, base_url, default_model, auth_mode, auth_env_var, cli_program, proxy_mode, supports_tools, max_retries, max_concurrency, role_generator, role_grader, role_executor
         FROM provider_configs ORDER BY name",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ProviderConfig {
            id: row.get(0)?,
            name: row.get(1)?,
            kind: parse_kind(&row.get::<_, String>(2)?),
            flavor: row
                .get::<_, Option<String>>(3)?
                .as_deref()
                .map(parse_flavor),
            base_url: row.get(4)?,
            default_model: row.get(5)?,
            auth_mode: row.get(6)?,
            auth_env_var: row.get(7)?,
            cli_program: row.get(8)?,
            proxy_mode: row.get(9)?,
            supports_tools: row.get::<_, i64>(10)? != 0,
            max_retries: u32::try_from(row.get::<_, i64>(11)?).unwrap_or(3),
            max_concurrency: u32::try_from(row.get::<_, i64>(12)?).unwrap_or(4),
            role_generator: row.get::<_, i64>(13)? != 0,
            role_grader: row.get::<_, i64>(14)? != 0,
            role_executor: row.get::<_, i64>(15)? != 0,
        })
    })?;
    rows.collect()
}

pub fn set_last_test(
    conn: &Connection,
    id: &str,
    ok: bool,
    latency_ms: u64,
    error: Option<&str>,
) -> rusqlite::Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0);
    set_last_test_at(conn, id, now, ok, latency_ms, error)
}

pub fn set_last_test_at(
    conn: &Connection,
    id: &str,
    at: i64,
    ok: bool,
    latency_ms: u64,
    error: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE provider_configs SET last_test_at = ?2, last_test_ok = ?3, last_test_latency_ms = ?4, last_test_error = ?5 WHERE id = ?1",
        (
            id,
            at,
            i64::from(ok),
            i64::try_from(latency_ms).unwrap_or(0),
            error,
        ),
    )?;
    Ok(())
}

pub fn provider_public_json(config: &ProviderConfig, has_secret: bool) -> Value {
    json!({
        "id": config.id,
        "name": config.name,
        "kind": kind_str(config.kind),
        "flavor": config.flavor.map(flavor_str),
        "base_url": config.base_url,
        "default_model": config.default_model,
        "auth_mode": config.auth_mode,
        "has_secret": has_secret,
        "role_generator": config.role_generator
    })
}

pub fn get_provider(conn: &Connection, id: &str) -> rusqlite::Result<Option<ProviderConfig>> {
    Ok(list_providers(conn)?.into_iter().find(|row| row.id == id))
}

pub fn referenced_by_preset(conn: &Connection, id: &str) -> rusqlite::Result<bool> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM runs WHERE executor_provider_id = ?1 OR grader_provider_id = ?1",
            (id,),
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or(0);
    Ok(count > 0)
}

fn kind_str(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Anthropic => "anthropic",
        ProviderKind::OpenaiCompat => "openai_compat",
        ProviderKind::LocalCli => "local_cli",
    }
}

fn flavor_str(flavor: Flavor) -> &'static str {
    match flavor {
        Flavor::Openai => "openai",
        Flavor::Azure => "azure",
        Flavor::Openrouter => "openrouter",
        Flavor::Ollama => "ollama",
        Flavor::Lmstudio => "lmstudio",
        Flavor::Vllm => "vllm",
        Flavor::Custom => "custom",
        Flavor::ClaudeCli => "claude_cli",
        Flavor::CodexCli => "codex_cli",
    }
}

fn parse_kind(value: &str) -> ProviderKind {
    match value {
        "openai_compat" => ProviderKind::OpenaiCompat,
        "local_cli" => ProviderKind::LocalCli,
        _ => ProviderKind::Anthropic,
    }
}

fn parse_flavor(value: &str) -> Flavor {
    match value {
        "openai" => Flavor::Openai,
        "azure" => Flavor::Azure,
        "openrouter" => Flavor::Openrouter,
        "ollama" => Flavor::Ollama,
        "lmstudio" => Flavor::Lmstudio,
        "vllm" => Flavor::Vllm,
        "claude_cli" => Flavor::ClaudeCli,
        "codex_cli" => Flavor::CodexCli,
        _ => Flavor::Custom,
    }
}
