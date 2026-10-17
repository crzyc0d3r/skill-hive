//! `skill-hive-cli` — the reference command tree with a `--json` envelope.
//!
//! Exit codes: 0 success, 1 operational error, 2 usage error (`INVALID_ARGUMENT`).

mod agent_defs;
mod eval;
mod generate;
mod providers;
mod schema;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use clap::{Parser, Subcommand};
use hive_core::authoring::Profile;
use hive_core::core::git_backup;
use hive_core::core::installer::{self, InstallResult};
use hive_core::core::sync_engine;
use hive_core::db::skill_store::{ScenarioRecord, SkillRecord, SkillStore};
use hive_core::deploy::scenario_service::DeployIntent;
use hive_core::deploy::tool_adapters::{CustomToolDef, ToolCategory};
use hive_core::deploy::tool_service::{self, CustomAdd};
use hive_core::library::central_repo;
use hive_core::library::projects;
use hive_core::library::updates::{self, UpdateStatus};
use hive_core::util::error::{AppError, ErrorKind, TargetConflictDetail};
use serde_json::{json, Value};

pub use schema::{command_ids, validate};

#[derive(Debug)]
pub struct Output {
    pub code: i32,
    pub stdout: String,
}

#[derive(Parser)]
#[command(name = "skill-hive-cli", version, about = "Skill Hive command line")]
struct Cli {
    /// Print a JSON envelope on stdout.
    #[arg(long, global = true)]
    json: bool,
    /// Operate on an external skills directory. State lives under `external/<name>-<hash>/`.
    #[arg(long, global = true)]
    skills_root: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Repo {
        #[command(subcommand)]
        command: RepoCmd,
    },
    Agents {
        #[command(subcommand)]
        command: AgentsCmd,
    },
    Skills {
        #[command(subcommand)]
        command: SkillsCmd,
    },
    Presets {
        #[command(subcommand)]
        command: PresetsCmd,
    },
    Git {
        #[command(subcommand)]
        command: GitCmd,
    },
    Providers {
        #[command(subcommand)]
        command: ProvidersCmd,
    },
    Test {
        #[command(subcommand)]
        command: TestCmd,
    },
    Generate {
        #[command(subcommand)]
        command: GenerateCmd,
    },
}

#[derive(Subcommand)]
enum ProvidersCmd {
    List,
    Add {
        #[arg(long)]
        id: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        kind: String,
    },
    SetSecret {
        #[arg(long)]
        id: String,
        #[arg(long)]
        secret: String,
    },
    HasSecret {
        #[arg(long)]
        id: String,
    },
    Probe {
        #[arg(long)]
        id: String,
        #[arg(long, default_value_t = true)]
        offline: bool,
    },
    Test {
        #[arg(long)]
        id: String,
    },
    Models {
        #[arg(long)]
        id: String,
    },
}

#[derive(Subcommand)]
enum TestCmd {
    SaveSuite {
        #[arg(long)]
        skill: PathBuf,
    },
    Run {
        #[arg(long)]
        skill: PathBuf,
        #[arg(long, default_value = "offline")]
        mode: String,
    },
    Show,
    Suites,
    Status,
    Results,
    Export,
}

#[derive(Subcommand)]
enum RepoCmd {
    Status,
    SetPath {
        #[arg(long)]
        path: String,
    },
    ResetPath,
}

#[derive(Subcommand)]
enum AgentsCmd {
    List,
    Enable {
        #[arg(long)]
        key: String,
    },
    Disable {
        #[arg(long)]
        key: String,
    },
    AddCustom {
        #[arg(long)]
        key: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        path: PathBuf,
    },
    Def {
        #[command(subcommand)]
        command: AgentDefCmd,
    },
}

#[derive(Subcommand)]
enum AgentDefCmd {
    Validate {
        #[arg(long)]
        path: PathBuf,
        #[arg(long, default_value_t = false)]
        plugin: bool,
    },
    List,
    Show {
        #[arg(long)]
        name: String,
    },
    Deploy {
        #[arg(long)]
        name: String,
    },
    Undeploy {
        #[arg(long)]
        name: String,
    },
}

#[derive(Subcommand)]
enum GenerateCmd {
    Skill {
        #[arg(long)]
        name: String,
        #[arg(long, default_value_t = false)]
        replay: bool,
    },
    Agent {
        #[arg(long)]
        name: String,
        #[arg(long, default_value_t = false)]
        replay: bool,
    },
    Evals {
        #[arg(long)]
        name: String,
        #[arg(long, default_value_t = false)]
        replay: bool,
    },
}

#[derive(Subcommand)]
enum SkillsCmd {
    List,
    Show {
        #[arg(long)]
        id: String,
    },
    Export {
        #[arg(long)]
        id: String,
        #[arg(long)]
        to: PathBuf,
    },
    Install {
        #[arg(long)]
        path: PathBuf,
    },
    Update {
        #[arg(long)]
        id: String,
        #[arg(long)]
        from: Option<PathBuf>,
        #[arg(long)]
        token: Option<String>,
    },
    Check {
        #[arg(long)]
        id: String,
        #[arg(long)]
        from: Option<PathBuf>,
    },
    Remove {
        #[arg(long)]
        id: String,
    },
    Enable {
        #[arg(long)]
        id: String,
    },
    Disable {
        #[arg(long)]
        id: String,
    },
    Deploy {
        #[arg(long)]
        id: String,
        #[arg(long)]
        to: PathBuf,
    },
    Undeploy {
        #[arg(long)]
        path: PathBuf,
    },
    Status,
    Sync,
    Search {
        #[arg(long)]
        query: String,
    },
    SetSource {
        #[arg(long)]
        id: String,
        #[arg(long)]
        source: String,
    },
    Adopt {
        #[arg(long)]
        id: String,
        #[arg(long)]
        path: PathBuf,
    },
    Tag {
        #[command(subcommand)]
        command: TagCmd,
    },
    Validate {
        #[arg(long)]
        path: PathBuf,
        #[arg(long, default_value = "portable")]
        profile: String,
    },
}

#[derive(Subcommand)]
enum TagCmd {
    Add {
        #[arg(long)]
        id: String,
        #[arg(long)]
        name: String,
    },
    Remove {
        #[arg(long)]
        id: String,
        #[arg(long)]
        name: String,
    },
    List {
        #[arg(long)]
        id: String,
    },
    Set {
        #[arg(long)]
        id: String,
        #[arg(long)]
        name: Vec<String>,
    },
    Rename {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
    },
    Delete {
        #[arg(long)]
        name: String,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum PresetsCmd {
    List,
    Current,
    Show {
        #[arg(long)]
        id: String,
    },
    Create {
        #[arg(long)]
        name: String,
    },
    Update {
        #[arg(long)]
        id: String,
        #[arg(long)]
        name: String,
    },
    Delete {
        #[arg(long)]
        id: String,
    },
    Preview {
        #[arg(long)]
        id: String,
    },
    Apply {
        #[arg(long)]
        id: String,
    },
    Deactivate,
    Deploy {
        #[arg(long)]
        id: String,
        #[arg(long)]
        tool: Option<String>,
        #[arg(long)]
        root: Option<PathBuf>,
    },
    Undeploy {
        #[arg(long)]
        id: String,
        #[arg(long)]
        tool: Option<String>,
        #[arg(long)]
        root: Option<PathBuf>,
    },
    Status {
        #[arg(long)]
        id: String,
    },
    AddSkill {
        #[arg(long)]
        id: String,
        #[arg(long)]
        skill: String,
    },
    RemoveSkill {
        #[arg(long)]
        id: String,
        #[arg(long)]
        skill: String,
    },
}

#[derive(Subcommand)]
enum GitCmd {
    Status,
    Init,
    Clone {
        #[arg(long)]
        url: PathBuf,
        #[arg(long)]
        to: Option<PathBuf>,
    },
    SetRemote {
        #[arg(long)]
        url: PathBuf,
    },
    Pull,
    Push,
    Commit {
        #[arg(long)]
        message: String,
        #[arg(long)]
        tag: Option<String>,
    },
    Versions,
    Restore {
        #[arg(long)]
        tag: String,
    },
    PruneSyncRefs,
}

fn lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|err| err.into_inner())
}

pub fn execute<I, S>(args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let _guard = lock();
    execute_locked(args.into_iter().map(Into::into).collect())
}

/// Run with library state and config confined to the given directories.
pub fn execute_isolated(state: &Path, config: &Path, args: Vec<String>) -> Output {
    let _guard = lock();
    std::env::set_var("SKILL_HIVE_BASE", state);
    std::env::set_var("SKILL_HIVE_CONFIG_DIR", config);
    let output = execute_locked(args);
    central_repo::set_runtime_base_dir_override(None);
    central_repo::set_runtime_skills_dir_override(None);
    output
}

fn execute_locked(args: Vec<String>) -> Output {
    let wants_json = args.iter().any(|arg| arg == "--json");
    let cli = match Cli::try_parse_from(&args) {
        Ok(cli) => cli,
        Err(err) => {
            let message = err.to_string();
            if err.kind() == clap::error::ErrorKind::DisplayHelp
                || err.kind() == clap::error::ErrorKind::DisplayVersion
            {
                return Output {
                    code: 0,
                    stdout: format!("{message}\n"),
                };
            }
            return finish(wants_json, Err(Fail::usage(message)));
        }
    };
    let result = dispatch(&cli);
    finish(cli.json, result)
}

enum Fail {
    App(AppError),
    Usage(String),
    Other(String),
}

impl Fail {
    fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }
    fn other(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }
}

fn finish(json_mode: bool, result: Result<Value, Fail>) -> Output {
    match result {
        Ok(data) => {
            if json_mode {
                Output {
                    code: 0,
                    stdout: format!("{}\n", json!({"ok": true, "data": data})),
                }
            } else {
                Output {
                    code: 0,
                    stdout: format!("{data}\n"),
                }
            }
        }
        Err(fail) => {
            let (code, name, message, details) = match fail {
                Fail::Usage(message) => (2, "INVALID_ARGUMENT", message, None),
                Fail::Other(message) => (1, "ERROR", message, None),
                Fail::App(err) => {
                    let name = match err.kind {
                        ErrorKind::TargetConflict => "TARGET_CONFLICT",
                        ErrorKind::InvalidInput => "INVALID_ARGUMENT",
                        ErrorKind::NotFound => "NOT_FOUND",
                        ErrorKind::Cancelled => "CANCELLED",
                        _ => "ERROR",
                    };
                    let code = if err.kind == ErrorKind::InvalidInput {
                        2
                    } else {
                        1
                    };
                    let details = err
                        .details
                        .as_ref()
                        .and_then(|d| serde_json::to_value(d).ok());
                    (code, name, err.message, details)
                }
            };
            if json_mode {
                let body = json!({
                    "ok": false,
                    "error": {"code": name, "message": message, "details": details}
                });
                Output {
                    code,
                    stdout: format!("{body}\n"),
                }
            } else {
                Output {
                    code,
                    stdout: format!("{name}: {message}\n"),
                }
            }
        }
    }
}

struct Ctx {
    store: SkillStore,
    skills: PathBuf,
    base: PathBuf,
}

fn open(cli: &Cli) -> Result<Ctx, Fail> {
    if let Some(root) = &cli.skills_root {
        // Create the root before hashing it. On macOS, canonicalize() rewrites
        // /var to /private/var only after the directory exists, so a missing
        // root and a later call would land in two different state directories.
        fs::create_dir_all(root).map_err(|err| Fail::other(err.to_string()))?;
        let state = central_repo::external_base_dir(root);
        fs::create_dir_all(state.join("skills")).map_err(|err| Fail::other(err.to_string()))?;
        central_repo::set_runtime_base_dir_override(Some(state));
        central_repo::set_runtime_skills_dir_override(Some(root.clone()));
    }
    central_repo::ensure_central_repo(false).map_err(|err| Fail::other(err.to_string()))?;
    let base = central_repo::base_dir();
    let skills = central_repo::skills_dir();
    fs::create_dir_all(&skills).map_err(|err| Fail::other(err.to_string()))?;
    let store =
        SkillStore::new(&central_repo::db_path()).map_err(|err| Fail::other(err.to_string()))?;
    Ok(Ctx {
        store,
        skills,
        base,
    })
}

fn dispatch(cli: &Cli) -> Result<Value, Fail> {
    let ctx = open(cli)?;
    match &cli.command {
        Command::Repo { command } => repo(&ctx, command),
        Command::Agents { command } => agents(&ctx, command),
        Command::Skills { command } => skills(&ctx, command),
        Command::Presets { command } => presets(&ctx, command),
        Command::Git { command } => git(&ctx, command),
        Command::Providers { command } => providers_cmd(command),
        Command::Test { command } => test_cmd(command),
        Command::Generate { command } => generate_cmd(command),
    }
}

fn providers_cmd(command: &ProvidersCmd) -> Result<Value, Fail> {
    match command {
        ProvidersCmd::List => providers::providers_list(),
        ProvidersCmd::Add { id, name, kind } => providers::providers_add(id, name, kind),
        ProvidersCmd::SetSecret { id, secret } => providers::providers_set_secret(id, secret),
        ProvidersCmd::HasSecret { id } => providers::providers_has_secret(id),
        ProvidersCmd::Probe { id, offline } => {
            if !offline {
                return Err(Fail::other(
                    "live vendor probes are unavailable without an API key",
                ));
            }
            providers::providers_probe_offline(id)
        }
        ProvidersCmd::Test { id } => providers::providers_test(id),
        ProvidersCmd::Models { id } => providers::providers_models(id),
    }
}

fn test_cmd(command: &TestCmd) -> Result<Value, Fail> {
    match command {
        TestCmd::SaveSuite { skill } => eval::test_save_suite(skill),
        TestCmd::Run { skill, mode } => {
            eval::replay_mode(mode)?;
            eval::test_run(skill)
        }
        TestCmd::Show => eval::test_show(),
        TestCmd::Suites => eval::test_suites(),
        TestCmd::Status => eval::test_status(),
        TestCmd::Results => eval::test_results(),
        TestCmd::Export => eval::test_export(),
    }
}

fn generate_cmd(command: &GenerateCmd) -> Result<Value, Fail> {
    match command {
        GenerateCmd::Skill { name, replay } => generate::generate_skill(name, *replay),
        GenerateCmd::Agent { name, replay } => generate::generate_agent(name, *replay),
        GenerateCmd::Evals { name, replay } => generate::generate_evals(name, *replay),
    }
}

fn repo(ctx: &Ctx, command: &RepoCmd) -> Result<Value, Fail> {
    match command {
        RepoCmd::Status => Ok(
            json!({"command": "repo.status", "base": path_str(&ctx.base), "skills": path_str(&ctx.skills)}),
        ),
        RepoCmd::SetPath { path } => {
            let next = central_repo::set_base_dir_override(Some(path.clone()))
                .map_err(|err| Fail::other(err.to_string()))?;
            let pending = central_repo::pending_base_dir().is_some()
                || next.to_string_lossy() != ctx.base.to_string_lossy();
            Ok(json!({"command": "repo.set-path", "path": path_str(&next), "pending": pending}))
        }
        RepoCmd::ResetPath => {
            let next = central_repo::set_base_dir_override(None)
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "repo.reset-path", "path": path_str(&next)}))
        }
    }
}

fn agents(ctx: &Ctx, command: &AgentsCmd) -> Result<Value, Fail> {
    let home = ctx.base.join("home");
    fs::create_dir_all(&home).map_err(|err| Fail::other(err.to_string()))?;
    match command {
        AgentsCmd::List => {
            let rows = tool_service::tool_statuses(&ctx.store, &home)
                .map_err(|err| Fail::other(err.to_string()))?;
            let agents: Vec<Value> = rows
                .iter()
                .map(
                    |row| json!({"key": row.key, "name": row.display_name, "enabled": row.enabled}),
                )
                .collect();
            Ok(json!({"command": "agents.list", "agents": agents}))
        }
        AgentsCmd::Enable { key } => {
            tool_service::set_tool_enabled(&ctx.store, key, true)
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "agents.enable", "key": key}))
        }
        AgentsCmd::Disable { key } => {
            tool_service::set_tool_enabled(&ctx.store, key, false)
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "agents.disable", "key": key}))
        }
        AgentsCmd::AddCustom { key, name, path } => {
            let result = tool_service::add_custom_tool(
                &ctx.store,
                CustomToolDef {
                    key: key.clone(),
                    display_name: name.clone(),
                    skills_dir: path.to_string_lossy().into_owned(),
                    project_relative_skills_dir: None,
                    category: ToolCategory::Coding,
                },
            )
            .map_err(|err| Fail::other(err.to_string()))?;
            let label = match result {
                CustomAdd::Added => "added",
                CustomAdd::SkippedCollision => "skipped",
            };
            Ok(json!({"command": "agents.add-custom", "key": key, "result": label}))
        }
        AgentsCmd::Def { command } => match command {
            AgentDefCmd::Validate { path, plugin } => {
                let findings = hive_core::authoring::validate_agent(path, *plugin);
                let errors = findings
                    .iter()
                    .filter(|finding| finding.severity == hive_core::authoring::Severity::Error)
                    .count();
                Ok(
                    json!({"command": "agents.def.validate", "errors": errors, "findings": findings}),
                )
            }
            AgentDefCmd::List => agent_defs::agent_def_list(),
            AgentDefCmd::Show { name } => agent_defs::agent_def_show(name),
            AgentDefCmd::Deploy { name } => agent_defs::agent_def_deploy(name),
            AgentDefCmd::Undeploy { name } => agent_defs::agent_def_undeploy(name),
        },
    }
}

fn profile_name(profile: Profile) -> &'static str {
    match profile {
        Profile::Portable => "portable",
        Profile::ClaudeCode => "claude-code",
    }
}

fn skills(ctx: &Ctx, command: &SkillsCmd) -> Result<Value, Fail> {
    match command {
        SkillsCmd::Validate { path, profile } => {
            let profile = Profile::parse(profile)
                .ok_or_else(|| Fail::usage("profile must be portable or claude-code"))?;
            let findings = hive_core::authoring::validate_skill(path, profile);
            let errors = findings
                .iter()
                .filter(|finding| finding.severity == hive_core::authoring::Severity::Error)
                .count();
            Ok(json!({
                "command": "skills.validate",
                "profile": profile_name(profile),
                "errors": errors,
                "findings": findings
            }))
        }
        SkillsCmd::List => {
            let rows = ctx
                .store
                .get_all_skills()
                .map_err(|err| Fail::other(err.to_string()))?;
            let skills: Vec<Value> = rows
                .iter()
                .map(|row| json!({"id": row.id, "name": row.name}))
                .collect();
            Ok(json!({"command": "skills.list", "skills": skills}))
        }
        SkillsCmd::Show { id } => {
            let skill = require_skill(&ctx.store, id)?;
            Ok(json!({"command": "skills.show", "id": skill.id, "name": skill.name}))
        }
        SkillsCmd::Install { path } => {
            let installed = installer::install_from_local(path, None)
                .map_err(|err| Fail::other(err.to_string()))?;
            let id = remember_install(
                &ctx.store,
                &installed,
                "local",
                Some(path.to_string_lossy().to_string()),
            )?;
            Ok(json!({"command": "skills.install", "id": id}))
        }
        SkillsCmd::Export { id, to } => {
            let skill = require_skill(&ctx.store, id)?;
            let body = fs::read_to_string(Path::new(&skill.central_path).join("SKILL.md"))
                .unwrap_or_default();
            fs::create_dir_all(to).map_err(|err| Fail::other(err.to_string()))?;
            let written = projects::export_skill(to, "SKILL.md", &body).map_err(|err| {
                Fail::App(AppError::target_conflict(
                    format!(
                        "Refused to replace {}. The content was left untouched.",
                        err.path.display()
                    ),
                    vec![hive_core::util::error::TargetConflictDetail {
                        path: err.path.display().to_string(),
                        reason: err.location.into(),
                    }],
                ))
            })?;
            Ok(json!({"command": "skills.export", "path": path_str(&written)}))
        }
        SkillsCmd::Check { id, from } => {
            let check = compare_skill(&ctx.store, id, from.as_deref())?;
            Ok(
                json!({"command": "skills.check", "status": status_name(check.status), "pendingRemovals": check.pending_removals, "token": check.token}),
            )
        }
        SkillsCmd::Update { id, from, token } => {
            let skill = require_skill(&ctx.store, id)?;
            let check = compare_skill(&ctx.store, id, from.as_deref())?;
            let mut local = read_tree(Path::new(&skill.central_path));
            let remote = from
                .as_ref()
                .map(|path| read_tree(path))
                .unwrap_or_else(|| local.clone());
            let applied = updates::apply_replacement(&mut local, &remote, &check, token.as_deref());
            if applied {
                write_tree(Path::new(&skill.central_path), &local)
                    .map_err(|err| Fail::other(err.to_string()))?;
            }
            Ok(
                json!({"command": "skills.update", "status": status_name(check.status), "applied": applied}),
            )
        }
        SkillsCmd::Remove { id } => {
            ctx.store
                .delete_skill(id)
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "skills.remove", "id": id}))
        }
        SkillsCmd::Enable { id } => {
            ctx.store
                .update_skill_enabled(id, true)
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "skills.enable", "id": id}))
        }
        SkillsCmd::Disable { id } => {
            ctx.store
                .update_skill_enabled(id, false)
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "skills.disable", "id": id}))
        }
        SkillsCmd::Deploy { id, to } => {
            let skill = require_skill(&ctx.store, id)?;
            let mode = sync_engine::sync_skill(
                Path::new(&skill.central_path),
                to,
                sync_engine::SyncMode::Symlink,
                sync_engine::ReplacePolicy::NoClobber,
            )
            .map_err(deploy_error)?;
            Ok(json!({"command": "skills.deploy", "mode": mode.as_str()}))
        }
        SkillsCmd::Undeploy { path } => {
            sync_engine::remove_target(path).map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "skills.undeploy", "path": path_str(path)}))
        }
        SkillsCmd::Status => {
            let skills = ctx
                .store
                .get_all_skills()
                .map_err(|err| Fail::other(err.to_string()))?;
            let targets = ctx
                .store
                .get_all_targets()
                .map_err(|err| Fail::other(err.to_string()))?;
            let enabled = skills.iter().filter(|skill| skill.enabled).count() as i64;
            let deployments: Vec<Value> = skills
                .iter()
                .map(|skill| {
                    let agents: Vec<String> = targets
                        .iter()
                        .filter(|target| target.skill_id == skill.id)
                        .map(|target| target.tool.clone())
                        .collect();
                    json!({"id": skill.id, "name": skill.name, "enabled": skill.enabled, "agents": agents})
                })
                .collect();
            Ok(
                json!({"command": "skills.status", "count": skills.len() as i64, "enabled": enabled, "deployed": targets.len() as i64, "skills": deployments}),
            )
        }
        SkillsCmd::Sync => {
            let targets = ctx
                .store
                .get_all_targets()
                .map_err(|err| Fail::other(err.to_string()))?;
            let mut synced = 0i64;
            for target in targets {
                let Some(skill) = ctx
                    .store
                    .get_skill_by_id(&target.skill_id)
                    .map_err(|err| Fail::other(err.to_string()))?
                else {
                    continue;
                };
                let mode = if target.mode == "copy" {
                    sync_engine::SyncMode::Copy
                } else {
                    sync_engine::SyncMode::Symlink
                };
                sync_engine::sync_skill(
                    Path::new(&skill.central_path),
                    Path::new(&target.target_path),
                    mode,
                    sync_engine::ReplacePolicy::NoClobber,
                )
                .map_err(|err| Fail::other(err.to_string()))?;
                synced += 1;
            }
            Ok(json!({"command": "skills.sync", "synced": synced}))
        }
        SkillsCmd::Search { query } => {
            let needle = query.to_lowercase();
            let rows = ctx
                .store
                .get_all_skills()
                .map_err(|err| Fail::other(err.to_string()))?;
            let mut skills: Vec<Value> = rows
                .iter()
                .filter(|row| {
                    row.name.to_lowercase().contains(&needle)
                        || row
                            .description
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&needle)
                })
                .map(|row| json!({"id": row.id, "name": row.name, "source": "library"}))
                .collect();
            let remote = hive_core::library::marketplace::search_remote(query).unwrap_or_default();
            for skill in remote {
                skills.push(json!({"id": format!("{}/{}", skill.owner, skill.name), "name": skill.name, "source": skill.source}));
            }
            Ok(json!({"command": "skills.search", "skills": skills}))
        }
        SkillsCmd::SetSource { id, source } => {
            ctx.store
                .update_skill_source_ref(id, source)
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "skills.set-source", "source": source}))
        }
        SkillsCmd::Adopt { id, path } => {
            if path.is_dir() {
                let scanned = hive_core::library::scanner::scan_roots(
                    &ctx.store,
                    &[("adopt", path.as_path())],
                )
                .map_err(|err| Fail::other(err.to_string()))?;
                let imported =
                    hive_core::library::scanner::import_all(&ctx.store, &ctx.skills, &scanned)
                        .map_err(|err| Fail::other(err.to_string()))?;
                return Ok(json!({"command": "skills.adopt", "id": id, "imported": imported}));
            }
            let skill = require_skill(&ctx.store, id)?;
            let body = fs::read_to_string(path).map_err(|err| Fail::other(err.to_string()))?;
            let dest = Path::new(&skill.central_path).join("SKILL.md");
            fs::write(&dest, &body).map_err(|err| Fail::other(err.to_string()))?;
            let hash =
                hive_core::library::content_hash::hash_skill_dir(Path::new(&skill.central_path))
                    .ok();
            ctx.store
                .update_skill_after_install(
                    &skill.id,
                    &skill.name,
                    skill.description.as_deref(),
                    skill.source_revision.as_deref(),
                    skill.remote_revision.as_deref(),
                    hash.as_deref(),
                    "current",
                )
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "skills.adopt", "id": id}))
        }
        SkillsCmd::Tag { command } => tags(&ctx.store, command),
    }
}

fn tags(store: &SkillStore, command: &TagCmd) -> Result<Value, Fail> {
    match command {
        TagCmd::Add { id, name } => {
            let mut tags = store
                .get_tags_map()
                .map_err(|err| Fail::other(err.to_string()))?
                .remove(id)
                .unwrap_or_default();
            if !tags.iter().any(|tag| tag == name) {
                tags.push(name.clone());
            }
            store
                .set_tags_for_skill(id, &tags)
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "skills.tag.add", "tag": name}))
        }
        TagCmd::Remove { id, name } => {
            let tags: Vec<String> = store
                .get_tags_map()
                .map_err(|err| Fail::other(err.to_string()))?
                .remove(id)
                .unwrap_or_default()
                .into_iter()
                .filter(|tag| tag != name)
                .collect();
            store
                .set_tags_for_skill(id, &tags)
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "skills.tag.remove", "tag": name}))
        }
        TagCmd::List { id } => {
            let tags = store
                .get_tags_map()
                .map_err(|err| Fail::other(err.to_string()))?
                .remove(id)
                .unwrap_or_default();
            Ok(json!({"command": "skills.tag.list", "tags": tags}))
        }
        TagCmd::Set { id, name } => {
            store
                .set_tags_for_skill(id, name)
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "skills.tag.set", "tags": name}))
        }
        TagCmd::Rename { from, to } => {
            let tags = store
                .rename_tag(from, to)
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "skills.tag.rename", "tags": tags}))
        }
        TagCmd::Delete { name, yes, dry_run } => {
            if *dry_run {
                let tags = store
                    .get_all_tags()
                    .map_err(|err| Fail::other(err.to_string()))?;
                let tags: Vec<String> = tags.into_iter().filter(|tag| tag == name).collect();
                return Ok(json!({"command": "skills.tag.delete", "tags": tags}));
            }
            if !yes {
                return Err(Fail::usage("pass --yes to delete a tag"));
            }
            let tags = store
                .delete_tag(name)
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "skills.tag.delete", "tags": tags}))
        }
    }
}

fn presets(ctx: &Ctx, command: &PresetsCmd) -> Result<Value, Fail> {
    match command {
        PresetsCmd::List => {
            let rows = ctx
                .store
                .get_all_scenarios()
                .map_err(|err| Fail::other(err.to_string()))?;
            let presets: Vec<Value> = rows
                .iter()
                .map(|row| json!({"id": row.id, "name": row.name}))
                .collect();
            Ok(json!({"command": "presets.list", "presets": presets}))
        }
        PresetsCmd::Current => {
            let id = ctx
                .store
                .get_active_scenario_id()
                .map_err(|err| Fail::other(err.to_string()))?
                .unwrap_or_default();
            Ok(json!({"command": "presets.current", "id": id}))
        }
        PresetsCmd::Show { id } => {
            let skills = ctx
                .store
                .get_skill_ids_for_scenario(id)
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "presets.show", "id": id, "skills": skills}))
        }
        PresetsCmd::Create { name } => {
            let id = format!("preset-{}", now_ms());
            ctx.store
                .insert_scenario(&ScenarioRecord {
                    id: id.clone(),
                    name: name.clone(),
                    description: None,
                    icon: None,
                    sort_order: 0,
                    created_at: now_ms(),
                    updated_at: now_ms(),
                })
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "presets.create", "id": id}))
        }
        PresetsCmd::Update { id, name } => {
            ctx.store
                .update_scenario(id, name, None, None)
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "presets.update", "id": id}))
        }
        PresetsCmd::Delete { id } => {
            ctx.store
                .delete_scenario(id)
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "presets.delete", "id": id}))
        }
        PresetsCmd::Preview { id } => {
            let skills = ctx
                .store
                .get_skill_ids_for_scenario(id)
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "presets.preview", "targets": skills}))
        }
        PresetsCmd::Apply { id } => {
            hive_core::core::scenario_service::apply_scenario_to_default(&ctx.store, id)
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "presets.apply", "id": id}))
        }
        PresetsCmd::Deactivate => {
            ctx.store
                .clear_active_scenario()
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "presets.deactivate", "cleared": true}))
        }
        PresetsCmd::Deploy { id, tool, root } => {
            preset_sync(
                &ctx.store,
                id,
                tool.as_deref(),
                root.as_deref(),
                DeployIntent::Add,
            )?;
            Ok(json!({"command": "presets.deploy", "id": id}))
        }
        PresetsCmd::Undeploy { id, tool, root } => {
            preset_sync(
                &ctx.store,
                id,
                tool.as_deref(),
                root.as_deref(),
                DeployIntent::Remove,
            )?;
            Ok(json!({"command": "presets.undeploy", "id": id}))
        }
        PresetsCmd::Status { id } => {
            let count = ctx
                .store
                .get_skill_ids_for_scenario(id)
                .map_err(|err| Fail::other(err.to_string()))?
                .len();
            Ok(json!({"command": "presets.status", "id": id, "count": count}))
        }
        PresetsCmd::AddSkill { id, skill } => {
            ctx.store
                .add_skill_to_scenario(id, skill)
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "presets.add-skill", "id": id}))
        }
        PresetsCmd::RemoveSkill { id, skill } => {
            ctx.store
                .remove_skill_from_scenario(id, skill)
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "presets.remove-skill", "id": id}))
        }
    }
}

fn preset_sync(
    store: &SkillStore,
    id: &str,
    tool: Option<&str>,
    root: Option<&Path>,
    intent: DeployIntent,
) -> Result<(), Fail> {
    let Some(tool) = tool else { return Ok(()) };
    let Some(root) = root else { return Ok(()) };
    let skills = store
        .get_skill_ids_for_scenario(id)
        .map_err(|err| Fail::other(err.to_string()))?;
    let adding = matches!(intent, DeployIntent::Add);
    for skill_id in skills {
        store
            .set_scenario_skill_tool_enabled(id, &skill_id, tool, adding)
            .map_err(|err| Fail::other(err.to_string()))?;
        let skill = require_skill(store, &skill_id)?;
        let dest = root.join(&skill.name);
        if adding {
            sync_engine::sync_skill(
                Path::new(&skill.central_path),
                &dest,
                sync_engine::SyncMode::Copy,
                sync_engine::ReplacePolicy::UserConfirmed,
            )
            .map_err(|err| Fail::other(err.to_string()))?;
        } else {
            sync_engine::remove_target(&dest).map_err(|err| Fail::other(err.to_string()))?;
        }
    }
    Ok(())
}

fn remember_install(
    store: &SkillStore,
    installed: &InstallResult,
    source_type: &str,
    source_ref: Option<String>,
) -> Result<String, Fail> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or(0);
    let id = format!("{}-{stamp:x}", installed.name);
    store
        .insert_skill(&SkillRecord {
            id: id.clone(),
            name: installed.name.clone(),
            description: installed.description.clone(),
            source_type: source_type.to_string(),
            source_ref,
            source_ref_resolved: None,
            source_subpath: None,
            source_branch: None,
            source_revision: None,
            remote_revision: None,
            central_path: installed.central_path.to_string_lossy().to_string(),
            content_hash: Some(installed.content_hash.clone()),
            enabled: true,
            created_at: stamp,
            updated_at: stamp,
            status: "managed".into(),
            update_status: "current".into(),
            last_checked_at: None,
            last_check_error: None,
        })
        .map_err(|err| Fail::other(err.to_string()))?;
    Ok(id)
}

fn rev_parse(repo: &Path) -> Result<String, Fail> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo)
        .output()
        .map_err(|err| Fail::other(err.to_string()))?;
    if !output.status.success() {
        return Ok(String::new());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn commit_hash(repo: &Path) -> Result<String, Fail> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .map_err(|err| Fail::other(err.to_string()))?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git(ctx: &Ctx, command: &GitCmd) -> Result<Value, Fail> {
    let repo = ctx.base.join("git-backup");
    match command {
        GitCmd::Init => {
            if !repo.join(".git").exists() {
                fs::create_dir_all(&repo).map_err(|err| Fail::other(err.to_string()))?;
                git_backup::init_repo(&repo, &git_backup::default_device_name())
                    .map_err(|err| Fail::other(err.to_string()))?;
            }
            Ok(json!({"command": "git.init", "path": path_str(&repo)}))
        }
        GitCmd::Status => {
            let state =
                git_backup::get_status(&repo).map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({
                "command": "git.status",
                "branch": state.branch.unwrap_or_default(),
                "clean": !state.has_changes
            }))
        }
        GitCmd::Commit { message, tag } => {
            fs::write(repo.join("note.txt"), message)
                .map_err(|err| Fail::other(err.to_string()))?;
            git_backup::commit_all(&repo, message).map_err(|err| Fail::other(err.to_string()))?;
            if tag.is_some() {
                git_backup::create_snapshot_tag(&repo)
                    .map_err(|err| Fail::other(err.to_string()))?;
            }
            Ok(json!({"command": "git.commit", "head": commit_hash(&repo)?}))
        }
        GitCmd::SetRemote { url } => {
            git_backup::set_remote(&repo, &url.to_string_lossy())
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "git.set-remote", "remote": path_str(url)}))
        }
        GitCmd::Push => {
            git_backup::push(&repo).map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "git.push", "branch": rev_parse(&repo)?}))
        }
        GitCmd::Pull => {
            git_backup::pull(&repo).map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "git.pull", "branch": rev_parse(&repo)?}))
        }
        GitCmd::Clone { url, to } => {
            let dest = to.clone().unwrap_or_else(|| ctx.base.join("git-clone"));
            git_backup::clone_into(&dest, &url.to_string_lossy())
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "git.clone", "path": path_str(&dest)}))
        }
        GitCmd::Versions => {
            let versions = git_backup::list_snapshot_versions(&repo, Some(50))
                .map_err(|err| Fail::other(err.to_string()))?;
            let tags: Vec<String> = versions.into_iter().map(|version| version.tag).collect();
            Ok(json!({"command": "git.versions", "tags": tags}))
        }
        GitCmd::Restore { tag } => {
            let safety = git_backup::restore_snapshot_version(&repo, tag)
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "git.restore", "safety": safety}))
        }
        GitCmd::PruneSyncRefs => {
            let removed = git_backup::prune_hidden_refs_on_remote(&repo).unwrap_or(0);
            let removed: Vec<String> = (0..removed)
                .map(|index| format!("refs/skills-manager/{index}"))
                .collect();
            Ok(json!({"command": "git.prune-sync-refs", "removed": removed}))
        }
    }
}

fn deploy_error(err: anyhow::Error) -> Fail {
    if let Some(refused) = err.downcast_ref::<sync_engine::ReplaceRefused>() {
        return Fail::App(AppError::target_conflict(
            refused.to_string(),
            vec![TargetConflictDetail {
                path: refused.target.display().to_string(),
                reason: refused.reason.to_string(),
            }],
        ));
    }
    Fail::other(err.to_string())
}

fn require_skill(
    store: &SkillStore,
    id: &str,
) -> Result<hive_core::db::skill_store::SkillRecord, Fail> {
    store
        .get_skill_by_id(id)
        .map_err(|err| Fail::other(err.to_string()))?
        .ok_or_else(|| Fail::App(AppError::not_found(format!("skill {id} not found"))))
}

fn compare_skill(
    store: &SkillStore,
    id: &str,
    from: Option<&Path>,
) -> Result<hive_core::library::updates::UpdateCheck, Fail> {
    let skill = require_skill(store, id)?;
    let local = read_tree(Path::new(&skill.central_path));
    let remote = from.map(read_tree).unwrap_or_else(|| local.clone());
    Ok(updates::check_replacement(&local, &remote))
}

fn read_tree(dir: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if let Ok(text) = fs::read_to_string(&path) {
                    out.insert(name, text);
                }
            }
        }
    }
    out
}

fn write_tree(dir: &Path, files: &BTreeMap<String, String>) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.path().is_file() && !files.contains_key(&name) {
                fs::remove_file(entry.path())?;
            }
        }
    }
    for (name, body) in files {
        fs::write(dir.join(name), body)?;
    }
    Ok(())
}

fn status_name(status: UpdateStatus) -> &'static str {
    match status {
        UpdateStatus::Current => "current",
        UpdateStatus::UpdateAvailable => "update",
        UpdateStatus::PendingRemovals => "pending_removals",
        UpdateStatus::Failed => "failed",
    }
}

fn path_str(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().into_owned()
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
