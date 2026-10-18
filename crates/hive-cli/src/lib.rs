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
use hive_core::db::skill_store::{ScenarioRecord, SkillRecord, SkillStore, SkillTargetRecord};
use hive_core::deploy::scenario_service::DeployIntent;
use hive_core::deploy::tool_adapters::{all_tool_adapters, CustomToolDef, ToolCategory};
use hive_core::deploy::tool_service::{self, CustomAdd};
use hive_core::library::central_repo;
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
        /// Override the kind's default model.
        #[arg(long)]
        model: Option<String>,
        /// Local base URL used by live probe, test, and generate.
        #[arg(long)]
        base_url: Option<String>,
        /// Role to enable. Repeat to set more than one. When omitted, the kind's defaults are stored.
        #[arg(long)]
        role: Vec<String>,
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
        /// POST to the provider base URL instead of the offline probe.
        #[arg(long, default_value_t = false)]
        live: bool,
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
        /// Trigger and task repetitions. The product default is 3.
        #[arg(long, default_value_t = 3)]
        repetitions: u32,
        /// emulated, claude-code, sequential, or static.
        #[arg(long, default_value = "emulated")]
        adapter: String,
    },
    Show,
    Suites,
    Status,
    Results,
    Export,
    /// Run with_skill and without_skill through the configured Anthropic or OpenAI adapter.
    Measure {
        #[arg(long)]
        skill: PathBuf,
        #[arg(long, default_value_t = 3)]
        repetitions: u32,
        /// Optional program that speaks Claude Code stream-json. The temp command file is deleted on exit.
        #[arg(long)]
        cli_program: Option<PathBuf>,
        /// emulated, claude-code, sequential, or static.
        #[arg(long, default_value = "emulated")]
        adapter: String,
    },
    /// Train/test description search. The winner is selected on the held-out split and is not applied.
    Optimize {
        #[arg(long)]
        skill: PathBuf,
        #[arg(long, default_value_t = 5)]
        rounds: u32,
    },
    /// Run one skill through the configured endpoint and a second base URL.
    Pair {
        #[arg(long)]
        skill: PathBuf,
        #[arg(long)]
        second_url: String,
    },
    /// Send template P12 to the configured adapter. The side names stay sealed in the saved file.
    Blind {
        #[arg(long)]
        prompt: String,
        #[arg(long)]
        output_a: String,
        #[arg(long)]
        output_b: String,
    },
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
        /// Task text written into the generated skill. Replay uses this instead of a canned task.
        #[arg(long)]
        task: Option<String>,
        #[arg(long, default_value_t = false)]
        replay: bool,
    },
    Agent {
        #[arg(long)]
        name: String,
        /// Task text written into the generated agent file.
        #[arg(long)]
        task: Option<String>,
        #[arg(long, default_value_t = false)]
        replay: bool,
    },
    Evals {
        #[arg(long)]
        name: String,
        #[arg(long, default_value_t = false)]
        replay: bool,
    },
    /// Ask the configured provider the generation interview (template P1).
    Interview {
        #[arg(long)]
        goal: String,
    },
}

#[derive(Subcommand)]
enum SkillsCmd {
    List {
        #[arg(long)]
        tag: Option<String>,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        preset: Option<String>,
    },
    Show {
        #[arg(long)]
        id: String,
    },
    Export {
        #[arg(long)]
        id: String,
        #[arg(long)]
        to: PathBuf,
        /// Replace an existing export tree.
        #[arg(long, default_value_t = false)]
        force: bool,
    },
    Install {
        /// Local directory, local git checkout, or a skills.sh URL.
        #[arg(long)]
        path: String,
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
        /// Agent key (`claude_code`, `codex`) or an explicit directory.
        #[arg(long)]
        to: PathBuf,
    },
    Undeploy {
        #[arg(long)]
        path: Option<PathBuf>,
        /// Agent key whose recorded deployments should be removed.
        #[arg(long)]
        agent: Option<String>,
    },
    Status,
    Sync {
        #[arg(long)]
        preset: Option<String>,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
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
        ProvidersCmd::Add {
            id,
            name,
            kind,
            model,
            base_url,
            role,
        } => providers::providers_add(id, name, kind, model.as_deref(), base_url.as_deref(), role),
        ProvidersCmd::SetSecret { id, secret } => providers::providers_set_secret(id, secret),
        ProvidersCmd::HasSecret { id } => providers::providers_has_secret(id),
        ProvidersCmd::Probe { id, offline, live } => {
            if *live || !offline {
                providers::providers_probe_live(id)
            } else {
                providers::providers_probe_offline(id)
            }
        }
        ProvidersCmd::Test { id } => providers::providers_test(id),
        ProvidersCmd::Models { id } => providers::providers_models(id),
    }
}

fn test_cmd(command: &TestCmd) -> Result<Value, Fail> {
    match command {
        TestCmd::SaveSuite { skill } => eval::test_save_suite(skill),
        TestCmd::Run {
            skill,
            mode,
            repetitions,
            adapter,
        } => {
            eval::replay_mode(mode)?;
            eval::test_run(skill, mode, *repetitions, Some(adapter))
        }
        TestCmd::Show => eval::test_show(),
        TestCmd::Suites => eval::test_suites(),
        TestCmd::Status => eval::test_status(),
        TestCmd::Results => eval::test_results(),
        TestCmd::Export => eval::test_export(),
        TestCmd::Measure {
            skill,
            repetitions,
            cli_program,
            adapter,
        } => eval::test_measure(skill, *repetitions, cli_program.as_deref(), Some(adapter)),
        TestCmd::Optimize { skill, rounds } => eval::test_optimize(skill, *rounds),
        TestCmd::Pair { skill, second_url } => eval::test_pair(skill, second_url),
        TestCmd::Blind {
            prompt,
            output_a,
            output_b,
        } => eval::test_blind(prompt, output_a, output_b),
    }
}

fn generate_cmd(command: &GenerateCmd) -> Result<Value, Fail> {
    match command {
        GenerateCmd::Skill { name, task, replay } => {
            generate::generate_skill(name, task.as_deref(), *replay)
        }
        GenerateCmd::Agent { name, task, replay } => {
            generate::generate_agent(name, task.as_deref(), *replay)
        }
        GenerateCmd::Evals { name, replay } => generate::generate_evals(name, *replay),
        GenerateCmd::Interview { goal } => generate::generate_interview(goal),
    }
}

fn repo(ctx: &Ctx, command: &RepoCmd) -> Result<Value, Fail> {
    match command {
        RepoCmd::Status => {
            let skills = ctx
                .store
                .get_all_skills()
                .map_err(|err| Fail::other(err.to_string()))?;
            let presets = ctx
                .store
                .get_all_scenarios()
                .map_err(|err| Fail::other(err.to_string()))?;
            let targets = ctx
                .store
                .get_all_targets()
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({
                "command": "repo.status",
                "base": path_str(&ctx.base),
                "skills": path_str(&ctx.skills),
                "counts": {
                    "skills": skills.len() as i64,
                    "presets": presets.len() as i64,
                    "deployments": targets.len() as i64
                },
                "pending": pending_library_path()
            }))
        }
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
            let adapters = all_tool_adapters(&ctx.store);
            let agents: Vec<Value> = rows
                .iter()
                .map(|row| {
                    let relative = adapters
                        .iter()
                        .find(|adapter| adapter.key == row.key)
                        .map(|adapter| adapter.relative_skills_dir.clone())
                        .unwrap_or_default();
                    let skills_dir = home.join(relative);
                    json!({
                        "key": row.key,
                        "name": row.display_name,
                        "enabled": row.enabled,
                        "installed": row.installed,
                        "skills_dir": path_str(&skills_dir),
                        "project_dir": row.project_dir
                    })
                })
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
        SkillsCmd::List {
            tag,
            source,
            preset,
        } => {
            let mut rows = ctx
                .store
                .get_all_skills()
                .map_err(|err| Fail::other(err.to_string()))?;
            let tags_map = ctx
                .store
                .get_tags_map()
                .map_err(|err| Fail::other(err.to_string()))?;
            let targets = ctx
                .store
                .get_all_targets()
                .map_err(|err| Fail::other(err.to_string()))?;
            if let Some(preset) = preset {
                let ids = ctx
                    .store
                    .get_skill_ids_for_scenario(preset)
                    .map_err(|err| Fail::other(err.to_string()))?;
                rows.retain(|row| ids.iter().any(|id| id == &row.id));
            }
            if let Some(tag) = tag {
                rows.retain(|row| {
                    tags_map
                        .get(&row.id)
                        .is_some_and(|tags| tags.iter().any(|item| item == tag))
                });
            }
            if let Some(source) = source {
                rows.retain(|row| {
                    row.source_type == *source || row.source_ref.as_deref() == Some(source.as_str())
                });
            }
            let skills: Vec<Value> = rows
                .iter()
                .map(|row| {
                    let tags = tags_map.get(&row.id).cloned().unwrap_or_default();
                    let agents: Vec<String> = targets
                        .iter()
                        .filter(|target| target.skill_id == row.id)
                        .map(|target| target.tool.clone())
                        .collect();
                    json!({
                        "id": row.id,
                        "name": row.name,
                        "description": row.description,
                        "path": row.central_path,
                        "tags": tags,
                        "source": row.source_ref.clone().unwrap_or_else(|| row.source_type.clone()),
                        "sourceType": row.source_type,
                        "enabled": row.enabled,
                        "deployed": !agents.is_empty(),
                        "agents": agents
                    })
                })
                .collect();
            Ok(json!({"command": "skills.list", "skills": skills}))
        }
        SkillsCmd::Show { id } => {
            let skill = require_skill(&ctx.store, id)?;
            let tags = ctx
                .store
                .get_tags_map()
                .map_err(|err| Fail::other(err.to_string()))?
                .remove(id)
                .unwrap_or_default();
            Ok(json!({
                "command": "skills.show",
                "id": skill.id,
                "name": skill.name,
                "description": skill.description,
                "path": skill.central_path,
                "tags": tags,
                "source": skill.source_ref.clone().unwrap_or(skill.source_type.clone()),
                "sourceType": skill.source_type
            }))
        }
        SkillsCmd::Install { path } => install_skill(ctx, path),
        SkillsCmd::Export { id, to, force } => export_skill(ctx, id, to, *force),
        SkillsCmd::Check { id, from } => {
            let check = compare_skill(&ctx.store, id, from.as_deref())?;
            let status = status_name(check.status);
            let remote_revision = from.as_ref().and_then(|path| git_head(path)).or_else(|| {
                require_skill(&ctx.store, id)
                    .ok()
                    .and_then(|skill| skill.source_ref)
                    .and_then(|source| {
                        let path = PathBuf::from(&source);
                        path.is_dir().then(|| git_head(&path)).flatten()
                    })
            });
            if let Some(revision) = &remote_revision {
                ctx.store
                    .update_skill_check_state(id, Some(revision), status, None)
                    .map_err(|err| Fail::other(err.to_string()))?;
            }
            Ok(json!({
                "command": "skills.check",
                "status": status,
                "pendingRemovals": check.pending_removals,
                "token": check.token,
                "remoteRevision": remote_revision
            }))
        }
        SkillsCmd::Update { id, from, token } => {
            let skill = require_skill(&ctx.store, id)?;
            if let Some(from) = from.as_ref().filter(|path| path.join(".git").exists()) {
                let check = compare_skill(&ctx.store, id, Some(from))?;
                copy_tree(from, Path::new(&skill.central_path))
                    .map_err(|err| Fail::other(err.to_string()))?;
                return Ok(json!({
                    "command": "skills.update",
                    "status": status_name(check.status),
                    "applied": true
                }));
            }
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
            let skill = require_skill(&ctx.store, id)?;
            let targets = ctx
                .store
                .get_targets_for_skill(id)
                .map_err(|err| Fail::other(err.to_string()))?;
            for target in &targets {
                sync_engine::remove_target(Path::new(&target.target_path))
                    .map_err(|err| Fail::other(err.to_string()))?;
                ctx.store
                    .delete_target(id, &target.tool)
                    .map_err(|err| Fail::other(err.to_string()))?;
            }
            remove_path(Path::new(&skill.central_path))?;
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
            let targets = ctx
                .store
                .get_targets_for_skill(id)
                .map_err(|err| Fail::other(err.to_string()))?;
            for target in &targets {
                sync_engine::remove_target(Path::new(&target.target_path))
                    .map_err(|err| Fail::other(err.to_string()))?;
                ctx.store
                    .delete_target(id, &target.tool)
                    .map_err(|err| Fail::other(err.to_string()))?;
            }
            Ok(json!({"command": "skills.disable", "id": id}))
        }
        SkillsCmd::Deploy { id, to } => {
            let skill = require_skill(&ctx.store, id)?;
            let (tool, dest) = deploy_destination(ctx, to, &skill.name);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).map_err(|err| Fail::other(err.to_string()))?;
            }
            let mode = sync_engine::sync_skill(
                Path::new(&skill.central_path),
                &dest,
                sync_engine::SyncMode::Symlink,
                sync_engine::ReplacePolicy::NoClobber,
            )
            .map_err(deploy_error)?;
            record_target(&ctx.store, id, &tool, &dest, mode.as_str())?;
            Ok(json!({"command": "skills.deploy", "mode": mode.as_str(), "path": path_str(&dest)}))
        }
        SkillsCmd::Undeploy { path, agent } => {
            undeploy_skill(ctx, path.as_deref(), agent.as_deref())
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
        SkillsCmd::Sync { preset, dry_run } => sync_skills(ctx, preset.as_deref(), *dry_run),
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
            let source_path = Path::new(source);
            let status = if source_path.is_dir() {
                let check = compare_skill(&ctx.store, id, Some(source_path))?;
                let status = status_name(check.status);
                ctx.store
                    .update_skill_check_state(id, None, status, None)
                    .map_err(|err| Fail::other(err.to_string()))?;
                status
            } else {
                "stored"
            };
            Ok(json!({"command": "skills.set-source", "source": source, "status": status}))
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
            let rows = ctx
                .store
                .get_all_scenarios()
                .map_err(|err| Fail::other(err.to_string()))?;
            let existing = rows
                .iter()
                .find(|row| row.id == *id)
                .ok_or_else(|| Fail::App(AppError::not_found(format!("preset {id} not found"))))?;
            let description = existing.description.clone();
            let icon = existing.icon.clone();
            ctx.store
                .update_scenario(id, name, description.as_deref(), icon.as_deref())
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
            let (deploy, undeploy) = preview_changes(ctx, &skills)?;
            Ok(json!({
                "command": "presets.preview",
                "targets": skills,
                "deploy": deploy,
                "undeploy": undeploy
            }))
        }
        PresetsCmd::Apply { id } => {
            ctx.store
                .set_active_scenario(id)
                .map_err(|err| Fail::other(err.to_string()))?;
            let roots = temp_installed_roots(ctx);
            if !roots.is_empty() {
                write_preset(ctx, id, &roots, DeployIntent::Add)?;
            }
            Ok(json!({"command": "presets.apply", "id": id}))
        }
        PresetsCmd::Deactivate => {
            if let Some(id) = ctx
                .store
                .get_active_scenario_id()
                .map_err(|err| Fail::other(err.to_string()))?
            {
                remove_preset_deployments(&ctx.store, &id)?;
            }
            ctx.store
                .clear_active_scenario()
                .map_err(|err| Fail::other(err.to_string()))?;
            Ok(json!({"command": "presets.deactivate", "cleared": true}))
        }
        PresetsCmd::Deploy { id, tool, root } => {
            preset_sync(ctx, id, tool.as_deref(), root.as_deref(), DeployIntent::Add)?;
            Ok(json!({"command": "presets.deploy", "id": id}))
        }
        PresetsCmd::Undeploy { id, tool, root } => {
            preset_sync(
                ctx,
                id,
                tool.as_deref(),
                root.as_deref(),
                DeployIntent::Remove,
            )?;
            Ok(json!({"command": "presets.undeploy", "id": id}))
        }
        PresetsCmd::Status { id } => {
            let skills = ctx
                .store
                .get_skill_ids_for_scenario(id)
                .map_err(|err| Fail::other(err.to_string()))?;
            let targets = ctx
                .store
                .get_all_targets()
                .map_err(|err| Fail::other(err.to_string()))?;
            let members: Vec<Value> = skills
                .iter()
                .map(|skill_id| {
                    let deployed = targets.iter().any(|target| {
                        target.skill_id == *skill_id
                            && symlink_exists(Path::new(&target.target_path))
                    });
                    json!({"id": skill_id, "deployed": deployed})
                })
                .collect();
            let deployed = members
                .iter()
                .filter(|member| member.get("deployed").and_then(Value::as_bool) == Some(true))
                .count() as i64;
            Ok(json!({
                "command": "presets.status",
                "id": id,
                "count": skills.len() as i64,
                "deployed": deployed,
                "members": members
            }))
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
    ctx: &Ctx,
    id: &str,
    tool: Option<&str>,
    root: Option<&Path>,
    intent: DeployIntent,
) -> Result<(), Fail> {
    if tool.is_none() && root.is_none() && matches!(intent, DeployIntent::Remove) {
        return remove_preset_deployments(&ctx.store, id);
    }
    let roots = resolve_preset_roots(ctx, id, tool, root)?;
    if matches!(intent, DeployIntent::Add) {
        if let (Some(tool), Some(root)) = (tool, root) {
            ctx.store
                .set_setting(
                    &format!("preset-root:{id}"),
                    &json!({"tool": tool, "root": path_str(root)}).to_string(),
                )
                .map_err(|err| Fail::other(err.to_string()))?;
        }
    }
    write_preset(ctx, id, &roots, intent)
}

fn write_preset(
    ctx: &Ctx,
    id: &str,
    roots: &[(String, PathBuf)],
    intent: DeployIntent,
) -> Result<(), Fail> {
    let skills = ctx
        .store
        .get_skill_ids_for_scenario(id)
        .map_err(|err| Fail::other(err.to_string()))?;
    let adding = matches!(intent, DeployIntent::Add);
    if adding && skills.is_empty() {
        return Err(Fail::other("preset has no skills to deploy"));
    }
    let mut wrote = false;
    for (tool, root) in roots {
        for skill_id in &skills {
            ctx.store
                .set_scenario_skill_tool_enabled(id, skill_id, tool, adding)
                .map_err(|err| Fail::other(err.to_string()))?;
            let skill = require_skill(&ctx.store, skill_id)?;
            let dest = root.join(&skill.name);
            if adding {
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent).map_err(|err| Fail::other(err.to_string()))?;
                }
                let mode = sync_engine::sync_skill(
                    Path::new(&skill.central_path),
                    &dest,
                    sync_engine::SyncMode::Copy,
                    sync_engine::ReplacePolicy::UserConfirmed,
                )
                .map_err(|err| Fail::other(err.to_string()))?;
                record_target(&ctx.store, skill_id, tool, &dest, mode.as_str())?;
                if !symlink_exists(&dest) {
                    return Err(Fail::other(format!(
                        "deploy reported success but {} was not written",
                        dest.display()
                    )));
                }
                wrote = true;
            } else {
                sync_engine::remove_target(&dest).map_err(|err| Fail::other(err.to_string()))?;
                ctx.store
                    .delete_target(skill_id, tool)
                    .map_err(|err| Fail::other(err.to_string()))?;
                wrote = true;
            }
        }
    }
    if adding && !wrote {
        return Err(Fail::other(
            "no tool root could be resolved for this preset",
        ));
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
            let indexed = reconcile_library(ctx, &repo)?;
            Ok(json!({"command": "git.pull", "branch": rev_parse(&repo)?, "indexed": indexed}))
        }
        GitCmd::Clone { url, to } => {
            let dest = to.clone().unwrap_or_else(|| ctx.base.join("git-backup"));
            git_backup::clone_into(&dest, &url.to_string_lossy())
                .map_err(|err| Fail::other(err.to_string()))?;
            let indexed = reconcile_library(ctx, &dest)?;
            Ok(json!({"command": "git.clone", "path": path_str(&dest), "indexed": indexed}))
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

fn install_skill(ctx: &Ctx, raw: &str) -> Result<Value, Fail> {
    let (resolved, subpath) = normalize_install_source(raw);
    if resolved != raw && resolved.contains("github.com") {
        let record = json!({
            "source": raw,
            "resolved": resolved,
            "subpath": subpath
        });
        fs::write(
            ctx.base.join("install-resolutions.json"),
            record.to_string(),
        )
        .map_err(|err| Fail::other(err.to_string()))?;
        ctx.store
            .set_setting("install-resolution", &record.to_string())
            .map_err(|err| Fail::other(err.to_string()))?;
        let id = resolved.trim_start_matches("https://").to_string();
        return Ok(json!({
            "command": "skills.install",
            "id": id,
            "resolved": resolved,
            "subpath": subpath
        }));
    }
    let path = PathBuf::from(raw);
    if !path.exists() {
        return Err(Fail::other(format!(
            "install source not found: {}",
            path.display()
        )));
    }
    let skill_dir = locate_skill_dir(&path);
    let git = path.join(".git").exists() || skill_dir.join(".git").exists();
    let installed = if git {
        installer::install_from_git_dir(&skill_dir, None)
    } else {
        installer::install_from_local(&skill_dir, None)
    }
    .map_err(|err| Fail::other(err.to_string()))?;
    let source_type = if git { "git" } else { "local" };
    let id = remember_install(
        &ctx.store,
        &installed,
        source_type,
        Some(path.display().to_string()),
    )?;
    if let Some(revision) = git_head(&path).or_else(|| git_head(&skill_dir)) {
        let _ = ctx
            .store
            .update_skill_check_state(&id, Some(&revision), "current", None);
    }
    Ok(json!({"command": "skills.install", "id": id, "source": source_type}))
}

fn normalize_install_source(url: &str) -> (String, Option<String>) {
    let trimmed = url.trim();
    let bare = trimmed
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let Some(rest) = bare.strip_prefix("skills.sh/") else {
        return (trimmed.to_string(), None);
    };
    let parts: Vec<&str> = rest.split('/').filter(|part| !part.is_empty()).collect();
    if parts.len() < 2 {
        return (trimmed.to_string(), None);
    }
    let repo = format!("https://github.com/{}/{}", parts[0], parts[1]);
    let subpath = if parts.len() > 2 {
        Some(parts[2..].join("/"))
    } else {
        None
    };
    (repo, subpath)
}

fn locate_skill_dir(path: &Path) -> PathBuf {
    if path.join("SKILL.md").is_file() || path.join("skill.md").is_file() {
        return path.to_path_buf();
    }
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let child = entry.path();
            if child.is_dir()
                && (child.join("SKILL.md").is_file() || child.join("skill.md").is_file())
            {
                return child;
            }
        }
    }
    path.to_path_buf()
}

fn export_skill(ctx: &Ctx, id: &str, to: &Path, force: bool) -> Result<Value, Fail> {
    let skill = require_skill(&ctx.store, id)?;
    if !force && export_occupied(to) {
        return Err(Fail::App(AppError::target_conflict(
            format!(
                "Refused to replace {}. The content was left untouched.",
                to.display()
            ),
            vec![TargetConflictDetail {
                path: to.display().to_string(),
                reason: "live".into(),
            }],
        )));
    }
    if force && to.exists() {
        remove_path(to)?;
    }
    copy_tree(Path::new(&skill.central_path), to).map_err(|err| Fail::other(err.to_string()))?;
    Ok(json!({"command": "skills.export", "path": path_str(to)}))
}

fn export_occupied(path: &Path) -> bool {
    if path.join("SKILL.md").exists() {
        return true;
    }
    fs::read_dir(path)
        .ok()
        .and_then(|mut entries| entries.next())
        .is_some()
}

fn undeploy_skill(ctx: &Ctx, path: Option<&Path>, agent: Option<&str>) -> Result<Value, Fail> {
    if path.is_none() && agent.is_none() {
        return Err(Fail::usage("pass --path or --agent"));
    }
    let mut removed = Vec::new();
    if let Some(agent) = agent {
        removed.extend(undeploy_agent(ctx, agent)?);
    }
    if let Some(path) = path {
        let raw = path.to_string_lossy();
        if !raw.contains('/') && !raw.contains('\\') && adapter_key(ctx, &raw).is_some() {
            removed.extend(undeploy_agent(ctx, &raw)?);
        } else {
            sync_engine::remove_target(path).map_err(|err| Fail::other(err.to_string()))?;
            delete_targets_at(&ctx.store, path)?;
            removed.push(path_str(path));
        }
    }
    let reported = removed
        .first()
        .cloned()
        .or_else(|| path.map(path_str))
        .or_else(|| agent.map(str::to_string))
        .unwrap_or_default();
    Ok(json!({"command": "skills.undeploy", "path": reported, "removed": removed}))
}

fn undeploy_agent(ctx: &Ctx, agent: &str) -> Result<Vec<String>, Fail> {
    let targets = ctx
        .store
        .get_all_targets()
        .map_err(|err| Fail::other(err.to_string()))?;
    let mut removed = Vec::new();
    for target in targets.into_iter().filter(|target| target.tool == agent) {
        sync_engine::remove_target(Path::new(&target.target_path))
            .map_err(|err| Fail::other(err.to_string()))?;
        ctx.store
            .delete_target(&target.skill_id, &target.tool)
            .map_err(|err| Fail::other(err.to_string()))?;
        removed.push(target.target_path);
    }
    Ok(removed)
}

fn delete_targets_at(store: &SkillStore, path: &Path) -> Result<(), Fail> {
    let needle = path_str(path);
    let targets = store
        .get_all_targets()
        .map_err(|err| Fail::other(err.to_string()))?;
    for target in targets
        .into_iter()
        .filter(|target| target.target_path == needle)
    {
        store
            .delete_target(&target.skill_id, &target.tool)
            .map_err(|err| Fail::other(err.to_string()))?;
    }
    Ok(())
}

fn sync_skills(ctx: &Ctx, preset: Option<&str>, dry_run: bool) -> Result<Value, Fail> {
    if let Some(preset) = preset {
        let plan = preset_sync_plan(ctx, preset)?;
        let paths: Vec<String> = plan.iter().map(|item| path_str(&item.dest)).collect();
        if dry_run {
            return Ok(json!({
                "command": "skills.sync",
                "synced": 0,
                "paths": paths,
                "dryRun": true
            }));
        }
        let mut synced = 0i64;
        for item in &plan {
            if let Some(parent) = item.dest.parent() {
                fs::create_dir_all(parent).map_err(|err| Fail::other(err.to_string()))?;
            }
            let mode = sync_engine::sync_skill(
                &item.source,
                &item.dest,
                sync_engine::SyncMode::Symlink,
                sync_engine::ReplacePolicy::NoClobber,
            )
            .map_err(deploy_error)?;
            record_target(
                &ctx.store,
                &item.skill_id,
                &item.tool,
                &item.dest,
                mode.as_str(),
            )?;
            synced += 1;
        }
        return Ok(json!({"command": "skills.sync", "synced": synced, "paths": paths}));
    }
    let targets = ctx
        .store
        .get_all_targets()
        .map_err(|err| Fail::other(err.to_string()))?;
    let mut paths = Vec::new();
    let mut synced = 0i64;
    for target in targets {
        paths.push(target.target_path.clone());
        if dry_run {
            continue;
        }
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
    if dry_run {
        return Ok(json!({
            "command": "skills.sync",
            "synced": 0,
            "paths": paths,
            "dryRun": true
        }));
    }
    Ok(json!({"command": "skills.sync", "synced": synced, "paths": paths}))
}

struct SyncItem {
    skill_id: String,
    tool: String,
    source: PathBuf,
    dest: PathBuf,
}

fn preset_sync_plan(ctx: &Ctx, preset: &str) -> Result<Vec<SyncItem>, Fail> {
    let skills = ctx
        .store
        .get_skill_ids_for_scenario(preset)
        .map_err(|err| Fail::other(err.to_string()))?;
    let roots = temp_installed_roots(ctx);
    let mut plan = Vec::new();
    for skill_id in skills {
        let skill = require_skill(&ctx.store, &skill_id)?;
        if roots.is_empty() {
            for target in ctx
                .store
                .get_targets_for_skill(&skill_id)
                .map_err(|err| Fail::other(err.to_string()))?
            {
                plan.push(SyncItem {
                    skill_id: skill_id.clone(),
                    tool: target.tool,
                    source: PathBuf::from(&skill.central_path),
                    dest: PathBuf::from(target.target_path),
                });
            }
        } else {
            for (tool, root) in &roots {
                plan.push(SyncItem {
                    skill_id: skill_id.clone(),
                    tool: tool.clone(),
                    source: PathBuf::from(&skill.central_path),
                    dest: root.join(&skill.name),
                });
            }
        }
    }
    Ok(plan)
}

fn deploy_destination(ctx: &Ctx, to: &Path, skill_name: &str) -> (String, PathBuf) {
    let raw = to.to_string_lossy();
    if !raw.contains('/') && !raw.contains('\\') {
        if let Some(key) = adapter_key(ctx, &raw) {
            let relative = all_tool_adapters(&ctx.store)
                .into_iter()
                .find(|adapter| adapter.key == key)
                .map(|adapter| adapter.relative_skills_dir)
                .unwrap_or_default();
            return (key, ctx.base.join("home").join(relative).join(skill_name));
        }
    }
    (raw.into_owned(), to.to_path_buf())
}

fn adapter_key(ctx: &Ctx, key: &str) -> Option<String> {
    all_tool_adapters(&ctx.store)
        .into_iter()
        .find(|adapter| adapter.key == key)
        .map(|adapter| adapter.key)
}

fn temp_installed_roots(ctx: &Ctx) -> Vec<(String, PathBuf)> {
    let home = ctx.base.join("home");
    let statuses = tool_service::tool_statuses(&ctx.store, &home).unwrap_or_default();
    let mut roots = Vec::new();
    for adapter in all_tool_adapters(&ctx.store) {
        if !home.join(&adapter.relative_detect_dir).exists() {
            continue;
        }
        let enabled = statuses
            .iter()
            .find(|row| row.key == adapter.key)
            .map(|row| row.enabled)
            .unwrap_or(true);
        if !enabled {
            continue;
        }
        roots.push((adapter.key, home.join(&adapter.relative_skills_dir)));
    }
    roots
}

fn resolve_preset_roots(
    ctx: &Ctx,
    id: &str,
    tool: Option<&str>,
    root: Option<&Path>,
) -> Result<Vec<(String, PathBuf)>, Fail> {
    match (tool, root) {
        (Some(tool), Some(root)) => Ok(vec![(tool.to_string(), root.to_path_buf())]),
        (Some(tool), None) => {
            let relative = all_tool_adapters(&ctx.store)
                .into_iter()
                .find(|adapter| adapter.key == tool)
                .map(|adapter| adapter.relative_skills_dir)
                .ok_or_else(|| Fail::other(format!("unknown tool {tool}")))?;
            Ok(vec![(
                tool.to_string(),
                ctx.base.join("home").join(relative),
            )])
        }
        (None, Some(root)) => Ok(vec![("directory".into(), root.to_path_buf())]),
        (None, None) => {
            let installed = temp_installed_roots(ctx);
            if !installed.is_empty() {
                return Ok(installed);
            }
            if let Some(saved) = saved_preset_root(&ctx.store, id)? {
                return Ok(vec![saved]);
            }
            Err(Fail::other(
                "no tool root could be resolved for this preset",
            ))
        }
    }
}

fn saved_preset_root(store: &SkillStore, id: &str) -> Result<Option<(String, PathBuf)>, Fail> {
    let raw = store
        .get_setting(&format!("preset-root:{id}"))
        .map_err(|err| Fail::other(err.to_string()))?;
    let Some(raw) = raw else {
        return Ok(None);
    };
    let parsed: Value = serde_json::from_str(&raw).map_err(|err| Fail::other(err.to_string()))?;
    let tool = parsed
        .get("tool")
        .and_then(Value::as_str)
        .unwrap_or("directory")
        .to_string();
    let Some(root) = parsed.get("root").and_then(Value::as_str) else {
        return Ok(None);
    };
    Ok(Some((tool, PathBuf::from(root))))
}

fn remove_preset_deployments(store: &SkillStore, id: &str) -> Result<(), Fail> {
    let skills = store
        .get_skill_ids_for_scenario(id)
        .map_err(|err| Fail::other(err.to_string()))?;
    let targets = store
        .get_all_targets()
        .map_err(|err| Fail::other(err.to_string()))?;
    let mine: Vec<_> = targets
        .into_iter()
        .filter(|target| skills.iter().any(|skill| skill == &target.skill_id))
        .collect();
    if mine.is_empty() {
        if let Some((_, root)) = saved_preset_root(store, id)? {
            for skill_id in &skills {
                let skill = require_skill(store, skill_id)?;
                let dest = root.join(&skill.name);
                if symlink_exists(&dest) {
                    sync_engine::remove_target(&dest)
                        .map_err(|err| Fail::other(err.to_string()))?;
                }
            }
        }
        return Ok(());
    }
    for target in mine {
        sync_engine::remove_target(Path::new(&target.target_path))
            .map_err(|err| Fail::other(err.to_string()))?;
        store
            .delete_target(&target.skill_id, &target.tool)
            .map_err(|err| Fail::other(err.to_string()))?;
    }
    Ok(())
}

fn preview_changes(ctx: &Ctx, skills: &[String]) -> Result<(Vec<Value>, Vec<Value>), Fail> {
    let targets = ctx
        .store
        .get_all_targets()
        .map_err(|err| Fail::other(err.to_string()))?;
    let roots = temp_installed_roots(ctx);
    let mut desired = Vec::new();
    for skill_id in skills {
        let skill = require_skill(&ctx.store, skill_id)?;
        for (tool, root) in &roots {
            desired.push((skill_id.clone(), tool.clone(), root.join(&skill.name)));
        }
    }
    let deploy: Vec<Value> = desired
        .iter()
        .filter(|(skill_id, _, path)| {
            !targets.iter().any(|target| {
                target.skill_id == *skill_id && Path::new(&target.target_path) == path
            })
        })
        .map(|(skill_id, tool, path)| {
            json!({"skill": skill_id, "tool": tool, "path": path_str(path), "action": "deploy"})
        })
        .collect();
    let desired_paths: Vec<String> = desired.iter().map(|(_, _, path)| path_str(path)).collect();
    let undeploy: Vec<Value> = targets
        .iter()
        .filter(|target| {
            skills.iter().any(|skill| skill == &target.skill_id)
                && !desired_paths.iter().any(|path| path == &target.target_path)
        })
        .map(|target| {
            json!({
                "skill": target.skill_id,
                "tool": target.tool,
                "path": target.target_path,
                "action": "undeploy"
            })
        })
        .collect();
    Ok((deploy, undeploy))
}

fn record_target(
    store: &SkillStore,
    skill_id: &str,
    tool: &str,
    dest: &Path,
    mode: &str,
) -> Result<(), Fail> {
    store
        .insert_target(&SkillTargetRecord {
            id: format!("{skill_id}:{tool}"),
            skill_id: skill_id.to_string(),
            tool: tool.to_string(),
            target_path: path_str(dest),
            mode: mode.to_string(),
            status: "ok".into(),
            synced_at: Some(now_ms()),
            last_error: None,
            source_hash: None,
        })
        .map_err(|err| Fail::other(err.to_string()))
}

fn reconcile_library(ctx: &Ctx, root: &Path) -> Result<Vec<String>, Fail> {
    let existing = ctx
        .store
        .get_all_skills()
        .map_err(|err| Fail::other(err.to_string()))?;
    let mut ids = Vec::new();
    for dir in find_skill_dirs(root) {
        let name = skill_name_of(&dir);
        if let Some(row) = existing.iter().find(|row| row.name == name) {
            copy_tree(&dir, Path::new(&row.central_path))
                .map_err(|err| Fail::other(err.to_string()))?;
            if let Some(revision) = git_head(root) {
                let _ =
                    ctx.store
                        .update_skill_check_state(&row.id, Some(&revision), "current", None);
            }
            ids.push(row.id.clone());
        } else {
            let installed = installer::install_from_local(&dir, None)
                .map_err(|err| Fail::other(err.to_string()))?;
            let id = remember_install(&ctx.store, &installed, "git", Some(path_str(root)))?;
            if let Some(revision) = git_head(root) {
                let _ = ctx
                    .store
                    .update_skill_check_state(&id, Some(&revision), "current", None);
            }
            ids.push(id);
        }
    }
    let index = json!({"root": path_str(root), "skills": ids});
    fs::write(ctx.base.join("library-index.json"), index.to_string())
        .map_err(|err| Fail::other(err.to_string()))?;
    Ok(ids)
}

fn find_skill_dirs(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if root.join("SKILL.md").is_file() {
        out.push(root.to_path_buf());
        return out;
    }
    fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
        if depth > 3 {
            return;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            if entry.file_name() == ".git" {
                continue;
            }
            let path = entry.path();
            if path.is_dir() {
                if path.join("SKILL.md").is_file() {
                    out.push(path);
                } else {
                    walk(&path, depth + 1, out);
                }
            }
        }
    }
    walk(root, 0, &mut out);
    out
}

fn skill_name_of(dir: &Path) -> String {
    let text = fs::read_to_string(dir.join("SKILL.md")).unwrap_or_default();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("name:") {
            let name = rest.trim();
            if !name.is_empty() {
                return name.to_string();
            }
        }
    }
    dir.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "skill".into())
}

fn git_head(dir: &Path) -> Option<String> {
    let git_dir = dir.join(".git");
    if !git_dir.exists() {
        return None;
    }
    if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir)
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    let head = fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();
    if let Some(rest) = head.strip_prefix("ref:") {
        let rel = rest.trim();
        let body = fs::read_to_string(git_dir.join(rel)).ok()?;
        let revision = body.trim();
        if !revision.is_empty() {
            return Some(revision.to_string());
        }
    }
    if head.is_empty() {
        None
    } else {
        Some(head.to_string())
    }
}

fn copy_tree(src: &Path, dst: &Path) -> std::io::Result<()> {
    fn rec(src: &Path, dst: &Path) -> std::io::Result<()> {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            if entry.file_name() == ".git" {
                continue;
            }
            let from = entry.path();
            let to = dst.join(entry.file_name());
            if from.is_dir() {
                rec(&from, &to)?;
            } else if from.is_file() {
                if let Some(parent) = to.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&from, &to)?;
            }
        }
        Ok(())
    }
    rec(src, dst)
}

fn remove_path(path: &Path) -> Result<(), Fail> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(Fail::other(err.to_string())),
    };
    if metadata.file_type().is_symlink() || metadata.is_file() {
        fs::remove_file(path).map_err(|err| Fail::other(err.to_string()))?;
    } else if metadata.is_dir() {
        fs::remove_dir_all(path).map_err(|err| Fail::other(err.to_string()))?;
    }
    Ok(())
}

fn symlink_exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn pending_library_path() -> String {
    let Some(dir) = std::env::var_os("SKILL_HIVE_CONFIG_DIR") else {
        return String::new();
    };
    let raw = fs::read_to_string(PathBuf::from(dir).join("repo-config.json")).unwrap_or_default();
    let Ok(value) = serde_json::from_str::<Value>(&raw) else {
        return String::new();
    };
    if value
        .get("pending_migration_from")
        .and_then(Value::as_str)
        .is_some()
    {
        return value
            .get("repo_path")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
    }
    String::new()
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
