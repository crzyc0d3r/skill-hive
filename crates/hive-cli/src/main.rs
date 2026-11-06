//! `hive`: the Skill Hive command line.
//!
//! Everything the desktop app does, scriptable. `--json` on any command prints the raw data.

mod output;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use hive_core::Hive;
use hive_core::model::*;
use hive_core::paths::HivePaths;
use owo_colors::OwoColorize;

use crate::output::{Out, delta, percent, st, truncate};

#[derive(Parser)]
#[command(name = "hive", version, about = "Collect, curate, deploy and measure agent skills", long_about = None)]
struct Cli {
    /// Print machine-readable JSON instead of tables.
    #[arg(long, global = true)]
    json: bool,
    /// Hive home directory (default: $HIVE_HOME or ~/.skill-hive).
    #[arg(long, global = true, env = "HIVE_HOME")]
    home: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scan this machine's agent directories (and extra folders) for skills, agents and commands.
    Scan(ScanArgs),
    /// Import skills from a path, `owner/repo`, GitHub/git URL or archive URL.
    Add(AddArgs),
    /// Preview what a source contains without importing.
    Preview { source: String },
    /// List library items.
    List(ListArgs),
    /// Full-text search of the library.
    Search {
        query: String,
        #[arg(long, default_value = "20")]
        limit: u32,
    },
    /// Show one item (by id, name or kind:name).
    Show {
        item: String,
        #[arg(long)]
        body: bool,
    },
    /// Remove items from the library (and all their deployments).
    Remove {
        items: Vec<String>,
        #[arg(short, long)]
        yes: bool,
    },
    /// Tag an item (replaces its tags).
    Tag { item: String, tags: Vec<String> },
    /// Check remote sources for updates.
    Updates {
        #[arg(long)]
        apply: bool,
        items: Vec<String>,
    },
    /// Agent targets and what they have installed.
    Targets(TargetsArgs),
    /// Deploy items to an agent.
    Deploy(DeployArgs),
    /// Remove a deployment (by deployment id, or every deployment of an item with --item).
    Undeploy {
        id: Option<String>,
        #[arg(long)]
        item: Option<String>,
    },
    /// Loadouts: curated sets applied as one unit.
    #[command(subcommand)]
    Loadout(LoadoutCmd),
    /// Recommend skills for a task and a conflict-free set.
    Find(FindArgs),
    /// Analyse a set of items for conflicts.
    Conflicts {
        items: Vec<String>,
        #[arg(long)]
        target: Option<String>,
    },
    /// Combine overlapping skills into one, written by a model provider.
    Merge(MergeArgs),
    /// The curated catalog of well-known skill collections.
    Catalog {
        #[arg(long)]
        refresh: bool,
        query: Option<String>,
    },
    /// Search GitHub for skill repositories.
    GithubSearch {
        query: String,
        #[arg(long, default_value = "20")]
        limit: u32,
    },
    /// Projects registered for project-scope deployments.
    #[command(subcommand)]
    Project(ProjectCmd),
    /// Create a new skill in the library.
    New {
        name: String,
        #[arg(long)]
        description: String,
        #[arg(long)]
        body: Option<String>,
        #[arg(long)]
        tag: Vec<String>,
    },
    /// Settings.
    #[command(subcommand)]
    Config(ConfigCmd),
    /// Health check (and optional reindex).
    Doctor {
        #[arg(long)]
        reindex: bool,
    },
    /// Model providers used for evaluations.
    #[command(subcommand)]
    Provider(ProviderCmd),
    /// Comparative evaluations.
    #[command(subcommand)]
    Eval(EvalCmd),
    /// The instruction files agents load on every session (CLAUDE.md, AGENTS.md, rules folders).
    #[command(subcommand)]
    Instructions(InstructionsCmd),
}

#[derive(Args)]
struct ScanArgs {
    /// Extra directories to scan.
    paths: Vec<String>,
    /// Scan only the given paths, not the agent directories.
    #[arg(long)]
    only: bool,
    /// Import everything that is new and valid.
    #[arg(long)]
    import: bool,
    #[arg(long)]
    tag: Vec<String>,
}

#[derive(Args)]
struct AddArgs {
    source: String,
    /// Import only candidates whose name matches (repeatable).
    #[arg(long)]
    name: Vec<String>,
    #[arg(long)]
    tag: Vec<String>,
    /// Replace existing items with the same name.
    #[arg(long)]
    overwrite: bool,
}

#[derive(Args)]
struct ListArgs {
    #[arg(long, value_enum)]
    kind: Option<KindArg>,
    #[arg(long)]
    tag: Vec<String>,
    #[arg(long)]
    deployed_to: Option<String>,
    #[arg(long)]
    favorites: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum KindArg {
    Skill,
    Agent,
    Command,
}

impl From<KindArg> for AssetKind {
    fn from(k: KindArg) -> Self {
        match k {
            KindArg::Skill => AssetKind::Skill,
            KindArg::Agent => AssetKind::Agent,
            KindArg::Command => AssetKind::Command,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum ScopeArg {
    User,
    Project,
}

impl From<ScopeArg> for Scope {
    fn from(s: ScopeArg) -> Self {
        match s {
            ScopeArg::User => Scope::User,
            ScopeArg::Project => Scope::Project,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum ModeArg {
    Link,
    Copy,
}

#[derive(Args)]
struct TargetsArgs {
    /// Show one target in detail.
    target: Option<String>,
    #[arg(long, value_enum, default_value = "user")]
    scope: ScopeArg,
    #[arg(long)]
    project: Option<String>,
    /// Include targets that are not detected on this machine.
    #[arg(long)]
    all: bool,
}

#[derive(Args)]
struct DeployArgs {
    items: Vec<String>,
    #[arg(long, short)]
    to: String,
    #[arg(long, value_enum, default_value = "user")]
    scope: ScopeArg,
    #[arg(long)]
    project: Option<String>,
    #[arg(long, value_enum)]
    mode: Option<ModeArg>,
    /// Replace unmanaged assets with the same name.
    #[arg(long)]
    force: bool,
}

#[derive(Subcommand)]
enum LoadoutCmd {
    List,
    Show {
        loadout: String,
    },
    Create {
        name: String,
        #[arg(long, default_value = "")]
        description: String,
        #[arg(long)]
        item: Vec<String>,
    },
    Add {
        loadout: String,
        items: Vec<String>,
    },
    #[command(name = "rm")]
    RemoveItems {
        loadout: String,
        items: Vec<String>,
    },
    Delete {
        loadout: String,
    },
    /// Dry run of applying a loadout to a target.
    Plan(ApplyArgs),
    Apply(ApplyArgs),
}

#[derive(Args)]
struct ApplyArgs {
    loadout: String,
    #[arg(long, short)]
    to: String,
    #[arg(long, value_enum, default_value = "user")]
    scope: ScopeArg,
    #[arg(long)]
    project: Option<String>,
    /// Remove Hive-managed assets that are not in the loadout.
    #[arg(long)]
    exclusive: bool,
    #[arg(long, value_enum)]
    mode: Option<ModeArg>,
}

#[derive(Args)]
struct FindArgs {
    task: Vec<String>,
    #[arg(long, default_value = "10")]
    limit: u32,
    #[arg(long)]
    target: Option<String>,
    #[arg(long, value_enum)]
    kind: Option<KindArg>,
    /// Ask a provider with the recommender role to re-rank.
    #[arg(long)]
    llm: bool,
    /// Deploy the suggested set to --target.
    #[arg(long)]
    deploy: bool,
}

#[derive(Args)]
struct MergeArgs {
    /// Two or more skills (by id, name or `skill:name`).
    items: Vec<String>,
    /// Name of the merged skill; the model proposes one when omitted. Using a source's name
    /// replaces that source in place (it keeps its id, deployments and history).
    #[arg(long)]
    name: Option<String>,
    /// Guidance for the model ("keep the python workflow", "prefer the shorter wording").
    #[arg(long)]
    guidance: Option<String>,
    /// Provider to ask (default: the recommender, then the grader, then the executor).
    #[arg(long)]
    provider: Option<String>,
    /// Keep the source skills in the library instead of retiring them and moving their
    /// deployments to the merged skill.
    #[arg(long)]
    keep_sources: bool,
    /// Do not carry the sources' evaluation cases over.
    #[arg(long)]
    no_evals: bool,
    /// Show the proposal and stop without writing anything.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Subcommand)]
enum ProjectCmd {
    List,
    Add { path: String },
    Remove { path: String },
}

#[derive(Subcommand)]
enum ConfigCmd {
    Show,
    /// Set a setting: link-mode, budget-chars, overlap, theme, executor, grader, github-token, scan-dirs (comma separated).
    Set {
        key: String,
        value: String,
    },
}

#[derive(Subcommand)]
enum ProviderCmd {
    List,
    /// Detect agent CLIs on PATH and register them as providers when none exist.
    Detect,
    Add(ProviderAddArgs),
    Remove {
        provider: String,
    },
    /// Test a provider.
    Probe {
        provider: String,
    },
}

#[derive(Args)]
struct ProviderAddArgs {
    name: String,
    #[arg(long, value_enum)]
    kind: ProviderKindArg,
    #[arg(long)]
    model: Option<String>,
    #[arg(long)]
    base_url: Option<String>,
    #[arg(long)]
    command: Option<String>,
    #[arg(long)]
    api_key: Option<String>,
    #[arg(long)]
    api_key_env: Option<String>,
    #[arg(long, value_enum)]
    role: Vec<RoleArg>,
}

#[derive(Clone, Copy, ValueEnum)]
enum ProviderKindArg {
    ClaudeCode,
    Codex,
    Anthropic,
    OpenaiCompatible,
    Scripted,
}

#[derive(Clone, Copy, ValueEnum)]
enum RoleArg {
    Executor,
    Grader,
    Recommender,
}

#[derive(Subcommand)]
enum InstructionsCmd {
    /// List the instruction locations of the detected agents, whether they exist or not.
    List {
        /// Project root for project-scope files; omitted lists the user-scope files.
        #[arg(long)]
        project: Option<String>,
    },
    /// Print an instruction file (or one of its backups).
    Show {
        path: String,
        #[arg(long)]
        project: Option<String>,
    },
    /// Write an instruction file from a file (`--from`) or standard input. A backup of the
    /// previous content is kept under the Hive home.
    Write {
        path: String,
        #[arg(long)]
        from: Option<PathBuf>,
        #[arg(long)]
        project: Option<String>,
    },
    /// Delete an instruction file (a backup is kept).
    Delete {
        path: String,
        #[arg(long)]
        project: Option<String>,
    },
    /// Backups Hive kept of an instruction file, oldest first.
    Backups { path: String },
}

#[derive(Subcommand)]
enum EvalCmd {
    /// Show (or draft) the evaluation suite of a skill.
    Suite {
        item: String,
        #[arg(long)]
        draft: bool,
        #[arg(long)]
        write: bool,
    },
    /// Run a comparative evaluation and wait for it to finish.
    Run(EvalRunArgs),
    /// List runs.
    Runs {
        item: Option<String>,
        #[arg(long, default_value = "20")]
        limit: u32,
    },
    /// Show a run's benchmark.
    Report {
        run: String,
        #[arg(long)]
        markdown: bool,
    },
    /// Effectiveness history of a skill.
    History {
        item: String,
    },
    Cancel {
        run: String,
    },
    Delete {
        run: String,
    },
}

#[derive(Args)]
struct EvalRunArgs {
    item: String,
    #[arg(long, default_value = "1")]
    repeats: u32,
    #[arg(long, default_value = "2")]
    concurrency: u32,
    #[arg(long)]
    executor: Option<String>,
    #[arg(long)]
    grader: Option<String>,
    /// Skip the without-skill baseline.
    #[arg(long)]
    no_baseline: bool,
    /// Extra arm: another skill directory to compare against (label=path).
    #[arg(long)]
    variant: Vec<String>,
    #[arg(long, default_value = "600")]
    timeout: u32,
    #[arg(long)]
    trigger: bool,
    #[arg(long)]
    case: Vec<u32>,
}

fn main() {
    let cli = Cli::parse();
    let filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    if let Err(err) = runtime.block_on(run(cli)) {
        eprintln!("{} {err:#}", "error:".style(st::red()).style(st::bold()));
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    let out = Out { json: cli.json };
    let paths = match cli.home {
        Some(home) => HivePaths::new(home),
        None => HivePaths::resolve(),
    };
    let hive = Hive::open(paths).context("opening the Skill Hive home")?;

    match cli.command {
        Command::Scan(args) => cmd_scan(&hive, &out, args)?,
        Command::Add(args) => {
            // With --name, preview first and import only the matching candidates.
            let paths: Option<Vec<String>> = if args.name.is_empty() {
                None
            } else {
                let preview = hive.preview_source(&args.source).await?;
                let wanted: Vec<String> = preview
                    .candidates
                    .iter()
                    .filter(|c| args.name.iter().any(|n| n.eq_ignore_ascii_case(&c.name)))
                    .map(|c| c.path.clone())
                    .collect();
                if wanted.is_empty() {
                    bail!(
                        "no candidate named {} in {} (available: {})",
                        args.name.join(", "),
                        args.source,
                        preview
                            .candidates
                            .iter()
                            .map(|c| c.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                Some(wanted)
            };
            let result = hive
                .import_from_source(&args.source, paths.as_deref(), &args.tag, args.overwrite)
                .await?;
            report_import(&out, &result)?;
        }
        Command::Preview { source } => {
            let preview = hive.preview_source(&source).await?;
            out.emit(&preview, || {
                println!(
                    "{} {}{}",
                    "Source:".style(st::bold()),
                    preview.spec.locator,
                    preview
                        .revision
                        .as_deref()
                        .map(|r| format!(" @ {}", &r[..r.len().min(10)]))
                        .unwrap_or_default()
                );
                for w in &preview.warnings {
                    out.warn(w);
                }
                out.table(
                    &["Kind", "Name", "Status", "Description", "Path in source"],
                    preview
                        .candidates
                        .iter()
                        .map(|c| {
                            vec![
                                c.kind.label().into(),
                                c.name.clone(),
                                status_label(c.status).into(),
                                truncate(&c.description, 60),
                                c.source.subpath.clone().unwrap_or_default(),
                            ]
                        })
                        .collect(),
                );
            })?;
        }
        Command::List(args) => {
            let items = hive.list_items(&LibraryQuery {
                kinds: args.kind.map(|k| vec![k.into()]).unwrap_or_default(),
                tags: args.tag,
                deployed_to: args.deployed_to,
                favorites_only: args.favorites,
                sort: LibrarySort::Name,
                ..Default::default()
            })?;
            print_items(&out, &items)?;
        }
        Command::Search { query, limit } => {
            let items = hive.list_items(&LibraryQuery {
                text: Some(query),
                limit: Some(limit),
                ..Default::default()
            })?;
            print_items(&out, &items)?;
        }
        Command::Show { item, body } => {
            let item = hive.resolve_item(&item)?;
            let detail = hive.item_detail(&item.id)?;
            out.emit(&detail, || {
                let i = &detail.item;
                println!(
                    "{} {} {}",
                    i.kind.label().style(st::dim()),
                    i.name.style(st::bold()),
                    if i.favorite { "★" } else { "" }
                );
                println!("{}", i.description);
                println!();
                println!("{:<12} {}", "id".style(st::dim()), i.id);
                println!("{:<12} {}", "path".style(st::dim()), i.path);
                println!("{:<12} {}", "source".style(st::dim()), i.source.display());
                if let Some(rev) = &i.source.revision {
                    println!(
                        "{:<12} {}",
                        "revision".style(st::dim()),
                        &rev[..rev.len().min(12)]
                    );
                }
                println!(
                    "{:<12} {}",
                    "tags".style(st::dim()),
                    if i.tags.is_empty() {
                        "-".into()
                    } else {
                        i.tags.join(", ")
                    }
                );
                println!(
                    "{:<12} ~{} tokens ({} in the description)",
                    "size".style(st::dim()),
                    i.token_estimate,
                    i.description_tokens
                );
                println!(
                    "{:<12} {}",
                    "validation".style(st::dim()),
                    i.validation.as_str()
                );
                if let Some(e) = &i.effectiveness {
                    println!(
                        "{:<12} {} ({} with skill vs {} baseline)",
                        "measured".style(st::dim()),
                        e.verdict.label(),
                        percent(e.with_skill_pass_rate),
                        percent(e.baseline_pass_rate)
                    );
                }
                if !detail.deployments.is_empty() {
                    println!("{}", "deployments".style(st::dim()));
                    for d in &detail.deployments {
                        println!(
                            "  {} {} {} ({:?}) {}",
                            d.target_id,
                            d.scope.as_str(),
                            d.mode.as_str(),
                            d.state,
                            d.path
                        );
                    }
                }
                for f in &detail.validation.findings {
                    println!("  {:?}: {} [{}]", f.severity, f.message, f.code);
                }
                if body {
                    println!("\n{}", detail.body);
                }
            })?;
        }
        Command::Remove { items, yes } => {
            let resolved: Vec<LibraryItem> = items
                .iter()
                .map(|i| hive.resolve_item(i))
                .collect::<hive_core::Result<_>>()?;
            if !yes && !cli.json {
                println!(
                    "Remove {}? Re-run with --yes.",
                    resolved
                        .iter()
                        .map(|i| i.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                return Ok(());
            }
            for item in &resolved {
                hive.remove_item(&item.id)?;
                out.ok(format!("removed {}", item.name));
            }
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string(&resolved.iter().map(|i| &i.id).collect::<Vec<_>>())?
                );
            }
        }
        Command::Tag { item, tags } => {
            let item = hive.resolve_item(&item)?;
            let item = hive.set_tags(&item.id, &tags)?;
            out.emit(&item, || {
                out.ok(format!("{}: {}", item.name, item.tags.join(", ")))
            })?;
        }
        Command::Updates { apply, items } => {
            let ids: Option<Vec<String>> = if items.is_empty() {
                None
            } else {
                Some(
                    items
                        .iter()
                        .map(|i| hive.resolve_item(i).map(|x| x.id))
                        .collect::<hive_core::Result<_>>()?,
                )
            };
            let checks = hive.check_updates(ids.as_deref()).await?;
            let mut updated = Vec::new();
            if apply {
                for c in checks.iter().filter(|c| c.update_available) {
                    let item = hive.update_item_from_source(&c.item_id).await?;
                    out.ok(format!("updated {}", item.name));
                    updated.push(item);
                }
            }
            out.emit(
                &serde_json::json!({"checks": checks, "updated": updated}),
                || {
                    out.table(
                        &["Item", "Current", "Latest", "Update"],
                        checks
                            .iter()
                            .map(|c| {
                                let name = hive
                                    .get_item(&c.item_id)
                                    .map(|i| i.name)
                                    .unwrap_or_else(|_| c.item_id.clone());
                                vec![
                                    name,
                                    c.current_revision
                                        .as_deref()
                                        .map(|r| r[..r.len().min(10)].to_string())
                                        .unwrap_or("-".into()),
                                    c.latest_revision
                                        .as_deref()
                                        .map(|r| r[..r.len().min(10)].to_string())
                                        .unwrap_or("-".into()),
                                    c.error.clone().unwrap_or_else(|| {
                                        if c.update_available {
                                            "yes".into()
                                        } else {
                                            "no".into()
                                        }
                                    }),
                                ]
                            })
                            .collect(),
                    );
                },
            )?;
        }
        Command::Targets(args) => cmd_targets(&hive, &out, args)?,
        Command::Deploy(args) => {
            let item_ids = resolve_ids(&hive, &args.items)?;
            let result = hive.deploy(&DeployRequest {
                item_ids,
                target_id: args.to.clone(),
                scope: args.scope.into(),
                project_path: args.project.clone(),
                mode: args.mode.map(|m| match m {
                    ModeArg::Link => LinkMode::Link,
                    ModeArg::Copy => LinkMode::Copy,
                }),
                force: args.force,
            })?;
            out.emit(&result, || {
                for d in &result.deployed {
                    out.ok(format!(
                        "{} → {} ({})",
                        d.item_name,
                        d.path,
                        d.mode.as_str()
                    ));
                }
                for f in &result.failed {
                    out.warn(format!("{}: {}", f.item_id, f.reason));
                }
            })?;
            if !result.failed.is_empty() {
                std::process::exit(2);
            }
        }
        Command::Undeploy { id, item } => {
            if let Some(item) = item {
                let item = hive.resolve_item(&item)?;
                let removed = hive.undeploy_item(&item.id)?;
                out.emit(&removed, || {
                    out.ok(format!(
                        "removed {} deployment(s) of {}",
                        removed.len(),
                        item.name
                    ))
                })?;
            } else if let Some(id) = id {
                let dep = hive.undeploy(&id)?;
                out.emit(&dep, || out.ok(format!("removed {}", dep.path)))?;
            } else {
                bail!("give a deployment id or --item");
            }
        }
        Command::Loadout(cmd) => cmd_loadout(&hive, &out, cmd)?,
        Command::Find(args) => cmd_find(&hive, &out, args).await?,
        Command::Conflicts { items, target } => {
            let ids = resolve_ids(&hive, &items)?;
            let report = hive.analyze_items(&ids, target.as_deref())?;
            out.emit(&report, || print_conflicts(&report))?;
        }
        Command::Merge(args) => cmd_merge(&hive, &out, args).await?,
        Command::Catalog { refresh, query } => {
            let entries = hive.catalog(refresh).await?;
            let entries = match query {
                Some(q) => hive_core::sources::catalog::filter(&entries, &q),
                None => entries,
            };
            out.emit(&entries, || {
                out.table(
                    &["Id", "Name", "Source", "Publisher", "Tags"],
                    entries
                        .iter()
                        .map(|e| {
                            vec![
                                e.id.clone(),
                                e.name.clone(),
                                e.source.clone(),
                                e.publisher.clone(),
                                e.tags.join(", "),
                            ]
                        })
                        .collect(),
                );
                out.note(
                    "Preview one with `hive preview <source>`; import with `hive add <source>`.",
                );
            })?;
        }
        Command::GithubSearch { query, limit } => {
            let results = hive.search_github(&query, limit).await?;
            out.emit(&results, || {
                out.table(
                    &["Repository", "Stars", "Description"],
                    results
                        .iter()
                        .map(|r| {
                            vec![
                                r.full_name.clone(),
                                r.stars.to_string(),
                                truncate(&r.description, 70),
                            ]
                        })
                        .collect(),
                );
            })?;
        }
        Command::Project(cmd) => match cmd {
            ProjectCmd::List => {
                let projects = hive.projects()?;
                out.emit(&projects, || {
                    out.table(
                        &["Name", "Path"],
                        projects
                            .iter()
                            .map(|p| vec![p.name.clone(), p.path.clone()])
                            .collect(),
                    )
                })?;
            }
            ProjectCmd::Add { path } => {
                let p = hive.add_project(&path)?;
                out.emit(&p, || out.ok(format!("added {} ({})", p.name, p.path)))?;
            }
            ProjectCmd::Remove { path } => {
                hive.remove_project(&path)?;
                out.ok("removed");
            }
        },
        Command::New {
            name,
            description,
            body,
            tag,
        } => {
            let body =
                body.unwrap_or_else(|| "# Overview\n\nDescribe the procedure here.\n".to_string());
            let item = hive.create_skill(&name, &description, &body, &tag)?;
            out.emit(&item, || {
                out.ok(format!("created {} at {}", item.name, item.path))
            })?;
        }
        Command::Config(cmd) => match cmd {
            ConfigCmd::Show => {
                let s = hive.settings()?;
                out.emit(&s, || {
                    println!("{:<22} {}", "home", s.home);
                    println!("{:<22} {}", "link-mode", s.default_link_mode.as_str());
                    println!(
                        "{:<22} {}",
                        "budget-chars", s.default_description_budget_chars
                    );
                    println!("{:<22} {}", "overlap", s.overlap_threshold);
                    println!(
                        "{:<22} {}",
                        "executor",
                        s.default_executor_id.as_deref().unwrap_or("-")
                    );
                    println!(
                        "{:<22} {}",
                        "grader",
                        s.default_grader_id.as_deref().unwrap_or("-")
                    );
                    println!("{:<22} {}", "scan-dirs", s.extra_scan_dirs.join(", "));
                    println!(
                        "{:<22} {}",
                        "github-token",
                        if s.github_token_set { "set" } else { "not set" }
                    );
                    println!("{:<22} {}", "theme", s.theme);
                })?;
            }
            ConfigCmd::Set { key, value } => {
                let mut patch = SettingsPatch::default();
                match key.as_str() {
                    "link-mode" => {
                        patch.default_link_mode =
                            Some(LinkMode::parse(&value).context("link or copy")?)
                    }
                    "budget-chars" => patch.default_description_budget_chars = Some(value.parse()?),
                    "overlap" => patch.overlap_threshold = Some(value.parse()?),
                    "theme" => patch.theme = Some(value),
                    "executor" => {
                        patch.default_executor_id =
                            Some(Some(value).filter(|v| v != "-" && !v.is_empty()))
                    }
                    "grader" => {
                        patch.default_grader_id =
                            Some(Some(value).filter(|v| v != "-" && !v.is_empty()))
                    }
                    "github-token" => patch.github_token = Some(value),
                    "scan-dirs" => {
                        patch.extra_scan_dirs = Some(
                            value
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect(),
                        )
                    }
                    other => bail!("unknown setting {other}"),
                }
                let s = hive.update_settings(&patch)?;
                out.emit(&s, || out.ok(format!("{key} updated")))?;
            }
        },
        Command::Doctor { reindex } => {
            let report = hive.doctor(reindex)?;
            out.emit(&report, || {
                println!("{:<18} {}", "home", report.home);
                println!(
                    "{:<18} {}",
                    "database",
                    if report.db_ok {
                        "ok".style(st::green()).to_string()
                    } else {
                        "corrupt".style(st::red()).to_string()
                    }
                );
                println!("{:<18} {}", "library items", report.library_items);
                println!(
                    "{:<18} {}",
                    "git",
                    if report.git_available {
                        "available"
                    } else {
                        "missing"
                    }
                );
                println!(
                    "{:<18} {}",
                    "agent CLIs",
                    if report.detected_clis.is_empty() {
                        "none detected".into()
                    } else {
                        report
                            .detected_clis
                            .iter()
                            .map(|c| {
                                format!("{} {}", c.command, c.version.clone().unwrap_or_default())
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                );
                for d in &report.orphan_dirs {
                    out.warn(format!("not indexed: {d}"));
                }
                for d in &report.missing_dirs {
                    out.warn(format!("missing files: {d}"));
                }
                for d in &report.broken_deployments {
                    out.warn(format!("broken deployment: {} ({})", d.path, d.item_name));
                }
                for n in &report.notes {
                    out.note(n);
                }
                if !report.orphan_dirs.is_empty() || !report.missing_dirs.is_empty() {
                    out.note("Run `hive doctor --reindex` to repair the index.");
                }
            })?;
        }
        Command::Provider(cmd) => cmd_provider(&hive, &out, cmd).await?,
        Command::Eval(cmd) => cmd_eval(&hive, &out, cmd).await?,
        Command::Instructions(cmd) => cmd_instructions(&hive, &out, cmd)?,
    }
    Ok(())
}

fn status_label(s: CandidateStatus) -> &'static str {
    match s {
        CandidateStatus::New => "new",
        CandidateStatus::InLibrary => "in library",
        CandidateStatus::Modified => "modified",
        CandidateStatus::Managed => "managed",
        CandidateStatus::Invalid => "invalid",
    }
}

fn resolve_ids(hive: &Hive, refs: &[String]) -> Result<Vec<String>> {
    refs.iter()
        .map(|r| hive.resolve_item(r).map(|i| i.id).map_err(Into::into))
        .collect()
}

fn report_import(out: &Out, result: &ImportResult) -> Result<()> {
    out.emit(result, || {
        for i in &result.imported {
            out.ok(format!(
                "imported {} {} ← {}",
                i.kind.label().to_lowercase(),
                i.name.style(st::bold()),
                i.source.display()
            ));
        }
        for s in &result.skipped {
            out.warn(format!("skipped {}: {}", s.path, s.reason));
        }
        if result.imported.is_empty() && result.skipped.is_empty() {
            out.note("nothing to import");
        }
    })
}

fn print_items(out: &Out, items: &[LibraryItem]) -> Result<()> {
    out.emit(&items, || {
        if items.is_empty() {
            out.note("no items");
            return;
        }
        out.table(
            &[
                "Kind",
                "Name",
                "Description",
                "Tags",
                "Deployed",
                "Measured",
                "Source",
            ],
            items
                .iter()
                .map(|i| {
                    vec![
                        i.kind.label().into(),
                        i.name.clone(),
                        truncate(&i.description, 56),
                        i.tags.join(", "),
                        if i.deployment_count > 0 {
                            i.deployment_count.to_string()
                        } else {
                            String::new()
                        },
                        i.effectiveness
                            .as_ref()
                            .map(|e| format!("{} {}", e.verdict.label(), delta(e.delta)))
                            .unwrap_or_default(),
                        i.source.kind.label().into(),
                    ]
                })
                .collect(),
        );
    })
}

fn print_conflicts(report: &ConflictReport) {
    if report.conflicts.is_empty() {
        println!("{} no conflicts", "✓".style(st::green()).style(st::bold()));
        return;
    }
    for c in &report.conflicts {
        let sev = match c.severity {
            ConflictSeverity::High => "HIGH".style(st::red()).style(st::bold()).to_string(),
            ConflictSeverity::Medium => "MEDIUM".style(st::yellow()).style(st::bold()).to_string(),
            ConflictSeverity::Low => "LOW".style(st::dim()).to_string(),
        };
        println!("{sev} {:?} {}", c.kind, c.names.join(" ↔ "));
        println!("    {}", c.detail);
        if !c.shared_terms.is_empty() {
            println!("    shared: {}", c.shared_terms.join(", ").style(st::dim()));
        }
    }
}

fn cmd_scan(hive: &Hive, out: &Out, args: ScanArgs) -> Result<()> {
    let report = if args.only {
        hive.scan_paths(&args.paths)?
    } else {
        hive.scan_local(&args.paths)?
    };
    let mut imported: Option<ImportResult> = None;
    if args.import {
        let paths: Vec<String> = report
            .candidates
            .iter()
            .filter(|c| c.status == CandidateStatus::New)
            .map(|c| c.path.clone())
            .collect();
        if !paths.is_empty() {
            let lib = hive.library();
            let mut result = ImportResult {
                imported: vec![],
                skipped: vec![],
            };
            for cand in report
                .candidates
                .iter()
                .filter(|c| c.status == CandidateStatus::New)
            {
                match lib.import_path(
                    std::path::Path::new(&cand.path),
                    cand.source.clone(),
                    &args.tag,
                    false,
                ) {
                    Ok(item) => result.imported.push(item),
                    Err(e) => result.skipped.push(ImportSkip {
                        path: cand.path.clone(),
                        reason: e.to_string(),
                    }),
                }
            }
            if !result.imported.is_empty() {
                hive.emit(HiveEvent::LibraryChanged);
            }
            imported = Some(result);
        }
    }
    out.emit(
        &serde_json::json!({"scan": report, "imported": imported}),
        || {
            out.note(format!(
                "scanned {} roots in {} ms",
                report.roots_scanned.len(),
                report.duration_ms
            ));
            out.table(
                &["Kind", "Name", "Status", "Agent", "Path"],
                report
                    .candidates
                    .iter()
                    .map(|c| {
                        vec![
                            c.kind.label().into(),
                            c.name.clone(),
                            status_label(c.status).into(),
                            c.source.target_id.clone().unwrap_or_default(),
                            c.path.clone(),
                        ]
                    })
                    .collect(),
            );
            for e in &report.errors {
                out.warn(e);
            }
            if let Some(r) = &imported {
                for i in &r.imported {
                    out.ok(format!("imported {}", i.name));
                }
                for s in &r.skipped {
                    out.warn(format!("skipped {}: {}", s.path, s.reason));
                }
            } else if report
                .candidates
                .iter()
                .any(|c| c.status == CandidateStatus::New)
            {
                out.note("Import new items with `hive scan --import` or `hive add <path>`.");
            }
        },
    )
}

fn cmd_targets(hive: &Hive, out: &Out, args: TargetsArgs) -> Result<()> {
    if let Some(target) = args.target {
        let status = hive.target_status(&target, args.scope.into(), args.project.as_deref())?;
        return out.emit(&status, || {
            println!(
                "{} {}",
                status.spec.name.style(st::bold()),
                if status.installed {
                    "(detected)".style(st::green()).to_string()
                } else {
                    "(not detected)".style(st::dim()).to_string()
                }
            );
            println!("{:<10} {}", "skills".style(st::dim()), status.skills_path);
            println!(
                "{:<10} {} chars / {} budget (~{} tokens){}",
                "context".style(st::dim()),
                status.context_chars,
                status
                    .spec
                    .description_budget_chars
                    .map(|b| b.to_string())
                    .unwrap_or("?".into()),
                status.context_tokens,
                status
                    .budget_used
                    .map(|f| format!(" = {:.0}%", f * 100.0))
                    .unwrap_or_default()
            );
            out.table(
                &["Kind", "Name", "State", "Library", "Path"],
                status
                    .assets
                    .iter()
                    .map(|a| {
                        vec![
                            a.kind.label().into(),
                            a.name.clone(),
                            format!("{:?}", a.state),
                            if a.library_item_id.is_some() {
                                "yes".into()
                            } else {
                                "no".into()
                            },
                            a.path.clone(),
                        ]
                    })
                    .collect(),
            );
            print_conflicts(&status.conflicts);
        });
    }
    let statuses = if args.all {
        let resolver = hive.resolver();
        resolver
            .specs()
            .iter()
            .map(|s| hive.target_status(&s.id, Scope::User, None))
            .collect::<hive_core::Result<Vec<_>>>()?
    } else {
        hive.installed_target_statuses()?
    };
    out.emit(&statuses, || {
        out.table(
            &[
                "Target",
                "Detected",
                "Assets",
                "Context",
                "Conflicts",
                "Skills dir",
            ],
            statuses
                .iter()
                .map(|s| {
                    vec![
                        s.spec.id.clone(),
                        if s.installed {
                            "yes".into()
                        } else {
                            "no".into()
                        },
                        s.assets.len().to_string(),
                        s.budget_used
                            .map(|f| format!("{:.0}%", f * 100.0))
                            .unwrap_or_default(),
                        s.conflicts.conflicts.len().to_string(),
                        s.skills_path.clone(),
                    ]
                })
                .collect(),
        );
    })
}

fn cmd_loadout(hive: &Hive, out: &Out, cmd: LoadoutCmd) -> Result<()> {
    match cmd {
        LoadoutCmd::List => {
            let loadouts = hive.loadouts()?;
            out.emit(&loadouts, || {
                out.table(
                    &["Name", "Items", "Description"],
                    loadouts
                        .iter()
                        .map(|l| {
                            vec![
                                l.name.clone(),
                                l.item_ids.len().to_string(),
                                l.description.clone(),
                            ]
                        })
                        .collect(),
                );
            })
        }
        LoadoutCmd::Show { loadout } => {
            let l = hive.resolve_loadout(&loadout)?;
            let items = hive.db.items_by_ids(&l.item_ids)?;
            let report = hive.analyze_items(&l.item_ids, None)?;
            out.emit(
                &serde_json::json!({"loadout": l, "items": items, "conflicts": report}),
                || {
                    println!("{} — {}", l.name.style(st::bold()), l.description);
                    print_items(out, &items).ok();
                    print_conflicts(&report);
                },
            )
        }
        LoadoutCmd::Create {
            name,
            description,
            item,
        } => {
            let l = hive.create_loadout(&LoadoutInput {
                name,
                description,
                item_ids: item,
                color: None,
            })?;
            out.emit(&l, || {
                out.ok(format!(
                    "created loadout {} ({} items)",
                    l.name,
                    l.item_ids.len()
                ))
            })
        }
        LoadoutCmd::Add { loadout, items } => {
            let l = hive.resolve_loadout(&loadout)?;
            let ids = resolve_ids(hive, &items)?;
            let l = hive.add_to_loadout(&l.id, &ids)?;
            out.emit(&l, || {
                out.ok(format!("{} now has {} items", l.name, l.item_ids.len()))
            })
        }
        LoadoutCmd::RemoveItems { loadout, items } => {
            let l = hive.resolve_loadout(&loadout)?;
            let ids = resolve_ids(hive, &items)?;
            let l = hive.remove_from_loadout(&l.id, &ids)?;
            out.emit(&l, || {
                out.ok(format!("{} now has {} items", l.name, l.item_ids.len()))
            })
        }
        LoadoutCmd::Delete { loadout } => {
            let l = hive.resolve_loadout(&loadout)?;
            hive.delete_loadout(&l.id)?;
            out.ok(format!("deleted {}", l.name));
            Ok(())
        }
        LoadoutCmd::Plan(args) => {
            let req = apply_request(hive, &args)?;
            let plan = hive.plan_loadout(&req)?;
            out.emit(&plan, || print_plan(&plan))
        }
        LoadoutCmd::Apply(args) => {
            let req = apply_request(hive, &args)?;
            let result = hive.apply_loadout(&req)?;
            out.emit(&result, || {
                print_plan(&result.plan);
                for d in &result.deployed {
                    out.ok(format!("deployed {} → {}", d.item_name, d.path));
                }
                for r in &result.removed {
                    out.ok(format!("removed {r}"));
                }
                for f in &result.failed {
                    out.warn(format!("{}: {}", f.item_id, f.reason));
                }
            })
        }
    }
}

fn apply_request(hive: &Hive, args: &ApplyArgs) -> Result<ApplyLoadoutRequest> {
    let l = hive.resolve_loadout(&args.loadout)?;
    Ok(ApplyLoadoutRequest {
        loadout_id: l.id,
        target_id: args.to.clone(),
        scope: args.scope.into(),
        project_path: args.project.clone(),
        strategy: if args.exclusive {
            ApplyStrategy::Exclusive
        } else {
            ApplyStrategy::Additive
        },
        mode: args.mode.map(|m| match m {
            ModeArg::Link => LinkMode::Link,
            ModeArg::Copy => LinkMode::Copy,
        }),
    })
}

fn print_plan(plan: &LoadoutPlan) {
    let names = |v: &[PlannedItem]| {
        v.iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    println!("{:<10} {}", "add".style(st::green()), names(&plan.adds));
    println!("{:<10} {}", "remove".style(st::red()), names(&plan.removes));
    println!("{:<10} {}", "keep".style(st::dim()), names(&plan.keeps));
    if !plan.unmanaged.is_empty() {
        println!(
            "{:<10} {}",
            "unmanaged".style(st::yellow()),
            names(&plan.unmanaged)
        );
    }
    println!(
        "{:<10} {} chars{}",
        "budget".style(st::dim()),
        plan.budget.description_chars,
        plan.budget
            .used_fraction
            .map(|f| format!(" ({:.0}% of budget)", f * 100.0))
            .unwrap_or_default()
    );
    print_conflicts(&plan.conflicts);
}

fn cmd_instructions(hive: &Hive, out: &Out, cmd: InstructionsCmd) -> Result<()> {
    match cmd {
        InstructionsCmd::List { project } => {
            let scope = if project.is_some() {
                Scope::Project
            } else {
                Scope::User
            };
            let files = hive.instruction_files(scope, project.as_deref())?;
            out.emit(&files, || {
                if files.is_empty() {
                    out.note("No agent with instruction files is detected on this machine.");
                    return;
                }
                out.table(
                    &["Agents", "File", "What", "Status", "Tokens"],
                    files
                        .iter()
                        .map(|f| {
                            vec![
                                f.target_names.join(", "),
                                f.path.clone(),
                                f.label.clone(),
                                match (f.kind, f.exists) {
                                    (InstructionKind::Directory, true) => "folder".into(),
                                    (InstructionKind::Directory, false) => "no folder".into(),
                                    (InstructionKind::File, true) => {
                                        format!("{} lines", f.lines)
                                    }
                                    (InstructionKind::File, false) => "missing".into(),
                                },
                                if f.exists && f.kind == InstructionKind::File {
                                    f.tokens.to_string()
                                } else {
                                    String::new()
                                },
                            ]
                        })
                        .collect(),
                );
                out.note(
                    "Write one with `hive instructions write <path> --from file.md` (or from stdin); a backup is kept.",
                );
            })?;
        }
        InstructionsCmd::Show { path, project } => {
            let doc = hive.read_instruction_file(&path, project.as_deref())?;
            out.emit(&doc, || {
                if !doc.file.exists {
                    out.warn(format!("{} does not exist yet", doc.file.display));
                } else {
                    // Piped output is byte-identical so `show <backup> | write <path>` restores
                    // exactly; a terminal gets a final newline for a tidy prompt.
                    print!("{}", doc.text);
                    if !doc.text.ends_with('\n')
                        && std::io::IsTerminal::is_terminal(&std::io::stdout())
                    {
                        println!();
                    }
                }
            })?;
        }
        InstructionsCmd::Write {
            path,
            from,
            project,
        } => {
            let text = match from {
                Some(file) => std::fs::read_to_string(&file)
                    .with_context(|| format!("reading {}", file.display()))?,
                None => {
                    let mut buf = String::new();
                    std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)
                        .context("reading standard input")?;
                    buf
                }
            };
            let doc = hive.write_instruction_file(&path, &text, project.as_deref())?;
            out.emit(&doc, || {
                out.ok(format!(
                    "wrote {} ({} lines, ~{} tokens)",
                    doc.file.path, doc.file.lines, doc.file.tokens
                ))
            })?;
        }
        InstructionsCmd::Delete { path, project } => {
            let file = hive.delete_instruction_file(&path, project.as_deref())?;
            out.emit(&file, || {
                out.ok(format!("deleted {} (backup kept)", file.path))
            })?;
        }
        InstructionsCmd::Backups { path } => {
            let backups = hive.instruction_backups(&path)?;
            out.emit(&backups, || {
                if backups.is_empty() {
                    out.note("No backups yet.");
                    return;
                }
                out.table(
                    &["Saved", "Size", "Backup"],
                    backups
                        .iter()
                        .map(|b| {
                            vec![
                                hive_core::util::iso_from_ms(b.saved_at),
                                hive_core::util::human_bytes(b.size_bytes),
                                b.path.clone(),
                            ]
                        })
                        .collect(),
                );
                out.note("Restore one with `hive instructions show <backup> | hive instructions write <path>`.");
            })?;
        }
    }
    Ok(())
}

async fn cmd_merge(hive: &Hive, out: &Out, args: MergeArgs) -> Result<()> {
    if args.items.len() < 2 {
        bail!("name at least two skills, e.g. `hive merge pdf-extract pdf-reader`");
    }
    let ids = resolve_ids(hive, &args.items)?;
    let proposal = hive
        .propose_merge(&MergeRequest {
            item_ids: ids.clone(),
            name: args.name.clone(),
            guidance: args.guidance.clone(),
            provider_id: args.provider.clone(),
        })
        .await?;
    let errors: Vec<&Finding> = proposal
        .validation
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .collect();
    if args.dry_run || !errors.is_empty() {
        out.emit(&proposal, || print_proposal(&proposal))?;
        if !errors.is_empty() {
            bail!(
                "the proposal does not validate: {}",
                errors
                    .iter()
                    .map(|f| f.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            );
        }
        return Ok(());
    }
    if !out.json {
        print_proposal(&proposal);
    }
    let result = hive.apply_merge(&MergeCommit {
        source_ids: ids,
        name: proposal.name.clone(),
        description: proposal.description.clone(),
        body: proposal.body.clone(),
        frontmatter: proposal.frontmatter.clone(),
        files: proposal.files.clone(),
        tags: proposal.tags.clone(),
        retire_sources: !args.keep_sources,
        carry_evals: !args.no_evals,
    })?;
    out.emit(&result, || {
        out.ok(format!(
            "{} {} at {}",
            if result.replaced_id.is_some() {
                "replaced"
            } else {
                "created"
            },
            result.item.name.style(st::bold()),
            result.item.path
        ));
        for r in &result.retired {
            out.note(format!("retired {}", r.name));
        }
        for d in &result.deployed {
            out.ok(format!("deployed to {} ({})", d.target_id, d.path));
        }
        for f in &result.failed {
            out.warn(format!("deployment failed: {}", f.reason));
        }
        for w in &result.warnings {
            out.warn(w);
        }
        if result.eval_cases > 0 {
            out.note(format!(
                "carried {} evaluation case(s); run `hive eval run {}` to measure the merged skill",
                result.eval_cases, result.item.name
            ));
        }
    })
}

fn print_proposal(p: &MergeProposal) {
    println!(
        "{} {} ← {}",
        "merge:".style(st::bold()),
        p.name,
        p.sources
            .iter()
            .map(|s| s.item.name.as_str())
            .collect::<Vec<_>>()
            .join(" + ")
    );
    println!("{} {}", "description:".style(st::dim()), p.description);
    println!(
        "{} {} → {} instruction tokens, {} → {} description tokens",
        "size:".style(st::dim()),
        p.body_tokens_before,
        p.body_tokens_after,
        p.description_tokens_before,
        p.description_tokens_after
    );
    let kept: Vec<&str> = p
        .files
        .iter()
        .filter(|f| f.keep)
        .map(|f| f.dest.as_str())
        .collect();
    let dropped: Vec<&str> = p
        .files
        .iter()
        .filter(|f| !f.keep)
        .map(|f| f.dest.as_str())
        .collect();
    if !kept.is_empty() {
        println!("{} {}", "files kept:".style(st::dim()), kept.join(", "));
    }
    if !dropped.is_empty() {
        println!(
            "{} {}",
            "files dropped:".style(st::dim()),
            dropped.join(", ")
        );
    }
    if p.eval_cases > 0 {
        println!("{} {}", "eval cases:".style(st::dim()), p.eval_cases);
    }
    if !p.notes.is_empty() {
        println!("{} {}", "model:".style(st::dim()), p.notes);
    }
    for d in &p.dropped {
        println!("{} {d}", "left out:".style(st::yellow()));
    }
    for w in &p.warnings {
        println!("{} {w}", "!".style(st::yellow()).style(st::bold()));
    }
    for f in &p.validation.findings {
        let tag = match f.severity {
            Severity::Error => "error".style(st::red()).style(st::bold()).to_string(),
            Severity::Warning => "warning".style(st::yellow()).to_string(),
            Severity::Info => "info".style(st::dim()).to_string(),
        };
        println!("{tag} {}", f.message);
    }
    println!(
        "{} {} · {} in, {} out · {} ms",
        "provider:".style(st::dim()),
        p.provider_name,
        p.input_tokens,
        p.output_tokens,
        p.latency_ms
    );
    println!("\n{}", "--- SKILL.md body ---".style(st::dim()));
    println!("{}", p.body);
    println!("{}", "---".style(st::dim()));
}

async fn cmd_find(hive: &Hive, out: &Out, args: FindArgs) -> Result<()> {
    let task = args.task.join(" ");
    if task.trim().is_empty() {
        bail!("describe the task, e.g. `hive find pull tables out of a pdf invoice`");
    }
    let response = hive
        .recommend(&RecommendRequest {
            task,
            kinds: args.kind.map(|k| vec![k.into()]).unwrap_or_default(),
            limit: Some(args.limit),
            target_id: args.target.clone(),
            exclude_ids: vec![],
            use_llm: args.llm,
        })
        .await?;
    out.emit(&response, || {
        out.table(
            &["", "Score", "Name", "Why"],
            response
                .results
                .iter()
                .map(|r| {
                    vec![
                        if r.selected {
                            "★".into()
                        } else {
                            String::new()
                        },
                        format!("{:.2}", r.score),
                        r.item.name.clone(),
                        r.reasons.join("; "),
                    ]
                })
                .collect(),
        );
        let set: Vec<&str> = response
            .results
            .iter()
            .filter(|r| r.selected)
            .map(|r| r.item.name.as_str())
            .collect();
        println!("{} {}", "suggested set:".style(st::bold()), set.join(", "));
        println!(
            "{} {} chars{}",
            "budget:".style(st::dim()),
            response.budget.description_chars,
            response
                .budget
                .used_fraction
                .map(|f| format!(" ({:.0}%)", f * 100.0))
                .unwrap_or_default()
        );
        print_conflicts(&response.conflicts);
        if let Some(notes) = &response.llm_notes {
            println!("{} {notes}", "model:".style(st::dim()));
        }
    })?;
    if args.deploy {
        let Some(target) = args.target else {
            bail!("--deploy needs --target")
        };
        let result = hive.deploy(&DeployRequest {
            item_ids: response.suggested_ids.clone(),
            target_id: target,
            scope: Scope::User,
            project_path: None,
            mode: None,
            force: false,
        })?;
        for d in &result.deployed {
            out.ok(format!("deployed {} → {}", d.item_name, d.path));
        }
        for f in &result.failed {
            out.warn(format!("{}: {}", f.item_id, f.reason));
        }
    }
    Ok(())
}

async fn cmd_provider(hive: &Hive, out: &Out, cmd: ProviderCmd) -> Result<()> {
    match cmd {
        ProviderCmd::List => {
            let providers = hive.list_providers()?;
            out.emit(&providers, || {
                out.table(
                    &["Id", "Name", "Kind", "Model", "Roles", "Last probe"],
                    providers
                        .iter()
                        .map(|p| {
                            vec![
                                p.id.clone(),
                                p.name.clone(),
                                p.kind.label().into(),
                                p.model.clone().unwrap_or_default(),
                                p.roles
                                    .iter()
                                    .map(|r| format!("{r:?}").to_lowercase())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                p.last_probe
                                    .as_ref()
                                    .map(|pr| {
                                        if pr.ok {
                                            format!("ok ({} ms)", pr.latency_ms)
                                        } else {
                                            format!("failed: {}", pr.message)
                                        }
                                    })
                                    .unwrap_or_default(),
                            ]
                        })
                        .collect(),
                );
            })
        }
        ProviderCmd::Detect => {
            let clis = hive_core::providers::detect_clis();
            let created = hive.ensure_default_providers()?;
            out.emit(
                &serde_json::json!({"detected": clis, "providers": created}),
                || {
                    for c in &clis {
                        out.ok(format!(
                            "{} {} at {}",
                            c.command,
                            c.version.clone().unwrap_or_default(),
                            c.path
                        ));
                    }
                    if clis.is_empty() {
                        out.note("no agent CLIs found on PATH (claude, codex)");
                    }
                    for p in &created {
                        out.note(format!("provider {} ({})", p.name, p.id));
                    }
                },
            )
        }
        ProviderCmd::Add(args) => {
            let roles: Vec<ProviderRole> = if args.role.is_empty() {
                match args.kind {
                    ProviderKindArg::ClaudeCode
                    | ProviderKindArg::Codex
                    | ProviderKindArg::Scripted => vec![
                        ProviderRole::Executor,
                        ProviderRole::Grader,
                        ProviderRole::Recommender,
                    ],
                    _ => vec![ProviderRole::Grader, ProviderRole::Recommender],
                }
            } else {
                args.role
                    .iter()
                    .map(|r| match r {
                        RoleArg::Executor => ProviderRole::Executor,
                        RoleArg::Grader => ProviderRole::Grader,
                        RoleArg::Recommender => ProviderRole::Recommender,
                    })
                    .collect()
            };
            let input = ProviderInput {
                name: args.name,
                kind: match args.kind {
                    ProviderKindArg::ClaudeCode => ProviderKind::ClaudeCode,
                    ProviderKindArg::Codex => ProviderKind::Codex,
                    ProviderKindArg::Anthropic => ProviderKind::Anthropic,
                    ProviderKindArg::OpenaiCompatible => ProviderKind::OpenaiCompatible,
                    ProviderKindArg::Scripted => ProviderKind::Scripted,
                },
                model: args.model,
                base_url: args.base_url,
                command: args.command,
                args: vec![],
                env: Default::default(),
                api_key_env: args.api_key_env,
                api_key: args.api_key,
                roles,
                timeout_secs: None,
                max_concurrency: None,
            };
            let p = hive.save_provider(None, &input)?;
            out.emit(&p, || {
                out.ok(format!("added provider {} ({})", p.name, p.id))
            })
        }
        ProviderCmd::Remove { provider } => {
            hive.delete_provider(&provider)?;
            out.ok("removed");
            Ok(())
        }
        ProviderCmd::Probe { provider } => {
            let probe = hive.probe_provider(&provider).await?;
            out.emit(&probe, || {
                if probe.ok {
                    out.ok(format!(
                        "{} ({} ms){}",
                        probe.message,
                        probe.latency_ms,
                        probe
                            .version
                            .as_deref()
                            .map(|v| format!(" version {v}"))
                            .unwrap_or_default()
                    ));
                } else {
                    out.warn(probe.message.clone());
                }
            })
        }
    }
}

async fn cmd_eval(hive: &Arc<Hive>, out: &Out, cmd: EvalCmd) -> Result<()> {
    match cmd {
        EvalCmd::Suite { item, draft, write } => {
            let item = hive.resolve_item(&item)?;
            let suite = if draft {
                let s = hive.draft_eval_suite(&item.id)?;
                if write { hive.save_eval_suite(&s)? } else { s }
            } else {
                hive.eval_suite(&item.id)?
            };
            out.emit(&suite, || {
                println!(
                    "{} ({}) {}",
                    suite.skill_name.style(st::bold()),
                    suite.path,
                    if suite.exists { "" } else { "(not saved yet)" }
                );
                for c in &suite.cases {
                    println!("\n#{} {}", c.id, c.name.style(st::bold()));
                    println!("  prompt: {}", truncate(&c.prompt, 100));
                    for e in &c.expectations {
                        println!("  - {e}");
                    }
                    for ch in &c.checks {
                        println!(
                            "  check {}: {}",
                            ch.id,
                            serde_json::to_string(&ch.kind).unwrap_or_default()
                        );
                    }
                }
                if !suite.trigger_cases.is_empty() {
                    println!("\ntrigger cases:");
                    for t in &suite.trigger_cases {
                        println!(
                            "  [{}] {}",
                            if t.should_trigger { "yes" } else { "no " },
                            t.query
                        );
                    }
                }
                if draft && !write {
                    out.note("\nSave this draft with `hive eval suite <skill> --draft --write`.");
                }
            })
        }
        EvalCmd::Run(args) => {
            let item = hive.resolve_item(&args.item)?;
            let mut config = hive.default_run_config(&item.id)?;
            if let Some(e) = args.executor {
                config.executor_provider_id = e;
            }
            if let Some(g) = args.grader {
                config.grader_provider_id = Some(g).filter(|g| g != "none");
            }
            if config.executor_provider_id.is_empty() {
                bail!(
                    "no executor provider; add one with `hive provider add` or `hive provider detect`"
                );
            }
            config.repeats = args.repeats.max(1);
            config.concurrency = args.concurrency.max(1);
            config.timeout_secs = args.timeout;
            config.include_trigger = args.trigger;
            config.case_ids = args.case;
            if args.no_baseline {
                config
                    .arms
                    .retain(|a| !matches!(a.kind, ArmKind::WithoutSkill));
            }
            for v in &args.variant {
                let (label, path) = v.split_once('=').unwrap_or(("variant", v.as_str()));
                config.arms.push(ArmSpec {
                    label: label.to_string(),
                    kind: ArmKind::Variant {
                        path: path.to_string(),
                    },
                });
            }
            let mut rx = hive.subscribe();
            let run = hive.start_run(config).await?;
            out.note(format!(
                "run {} started ({} arms × {} repeats)",
                run.id,
                run.config.arms.len(),
                run.config.repeats
            ));
            loop {
                match rx.recv().await {
                    Ok(HiveEvent::CaseFinished { run_id, result }) if run_id == run.id => {
                        out.note(format!(
                            "  {} [{}] repeat {} → {}/{} ({})",
                            result.case_name,
                            result.arm,
                            result.repeat,
                            result.passed,
                            result.total,
                            format!("{:?}", result.status).to_lowercase()
                        ));
                    }
                    Ok(HiveEvent::RunUpdated {
                        run_id,
                        status,
                        progress,
                    }) if run_id == run.id => {
                        if status.is_terminal() {
                            break;
                        }
                        if let Some(m) = progress.message {
                            tracing::debug!("{m}");
                        }
                    }
                    Ok(_) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
            let detail = hive.run_detail(&run.id)?;
            print_run(out, &detail)?;
            if detail.run.status != RunStatus::Done {
                std::process::exit(2);
            }
            Ok(())
        }
        EvalCmd::Runs { item, limit } => {
            let item_id = item
                .map(|i| hive.resolve_item(&i).map(|x| x.id))
                .transpose()?;
            let runs = hive.list_runs(item_id.as_deref(), limit)?;
            out.emit(&runs, || {
                out.table(
                    &[
                        "Run", "Skill", "Status", "Verdict", "With", "Baseline", "Created",
                    ],
                    runs.iter()
                        .map(|r| {
                            let (w, b) = r
                                .summary
                                .as_ref()
                                .map(|s| {
                                    let w = s
                                        .arms
                                        .iter()
                                        .find(|a| a.label == "with_skill")
                                        .map(|a| percent(a.pass_rate.mean))
                                        .unwrap_or_default();
                                    let b = s
                                        .arms
                                        .iter()
                                        .find(|a| a.label == s.baseline)
                                        .map(|a| percent(a.pass_rate.mean))
                                        .unwrap_or_default();
                                    (w, b)
                                })
                                .unwrap_or_default();
                            vec![
                                r.id.clone(),
                                r.skill_name.clone(),
                                r.status.as_str().into(),
                                r.summary
                                    .as_ref()
                                    .map(|s| s.verdict.label().to_string())
                                    .unwrap_or_default(),
                                w,
                                b,
                                hive_core::util::iso_from_ms(r.created_at),
                            ]
                        })
                        .collect(),
                );
            })
        }
        EvalCmd::Report { run, markdown } => {
            if markdown {
                println!("{}", hive.benchmark_markdown(&run)?);
                return Ok(());
            }
            let detail = hive.run_detail(&run)?;
            print_run(out, &detail)
        }
        EvalCmd::History { item } => {
            let item = hive.resolve_item(&item)?;
            let points = hive.history(&item.id)?;
            out.emit(&points, || {
                out.table(
                    &[
                        "When",
                        "Verdict",
                        "With skill",
                        "Baseline",
                        "Delta",
                        "Executor",
                        "Skill hash",
                    ],
                    points
                        .iter()
                        .map(|p| {
                            vec![
                                hive_core::util::iso_from_ms(p.finished_at),
                                p.verdict.label().into(),
                                percent(p.with_skill_pass_rate),
                                percent(p.baseline_pass_rate),
                                delta(p.delta),
                                p.executor.clone(),
                                p.skill_hash[..p.skill_hash.len().min(10)].into(),
                            ]
                        })
                        .collect(),
                );
            })
        }
        EvalCmd::Cancel { run } => {
            hive.cancel_run(&run)?;
            out.ok("cancel requested");
            Ok(())
        }
        EvalCmd::Delete { run } => {
            hive.delete_run(&run)?;
            out.ok("deleted");
            Ok(())
        }
    }
}

fn print_run(out: &Out, detail: &EvalRunDetail) -> Result<()> {
    out.emit(detail, || {
        let r = &detail.run;
        println!(
            "{} {} [{}]",
            "run".style(st::dim()),
            r.id,
            r.status.as_str()
        );
        if let Some(err) = &r.error {
            out.warn(err);
        }
        if let Some(b) = &r.summary {
            println!("{} {}", "verdict:".style(st::bold()), b.verdict.label());
            out.table(
                &[
                    "Arm",
                    "Runs",
                    "Pass rate",
                    "Tokens",
                    "Duration",
                    "Tool calls",
                ],
                b.arms
                    .iter()
                    .map(|a| {
                        vec![
                            a.label.clone(),
                            a.runs.to_string(),
                            format!(
                                "{} ± {}",
                                percent(a.pass_rate.mean),
                                percent(a.pass_rate.stddev)
                            ),
                            format!("{:.0}", a.tokens.mean),
                            format!("{:.1}s", a.duration_ms.mean / 1000.0),
                            format!("{:.1}", a.tool_calls.mean),
                        ]
                    })
                    .collect(),
            );
            if b.arms.iter().any(|a| a.activity.is_some()) {
                println!("\n{}", "activity (mean per traced run)".style(st::bold()));
                out.table(
                    &[
                        "Arm",
                        "Turns",
                        "Reads",
                        "Searches",
                        "Writes",
                        "Commands",
                        "Tool errors",
                        "Thinking tok",
                        "IO share",
                        "Opened skill",
                    ],
                    b.arms
                        .iter()
                        .filter_map(|a| a.activity.as_ref().map(|act| (a, act)))
                        .map(|(a, act)| {
                            vec![
                                a.label.clone(),
                                format!("{:.1}", act.turns.mean),
                                format!("{:.1}", act.reads.mean),
                                format!("{:.1}", act.searches.mean),
                                format!("{:.1}", act.writes.mean),
                                format!("{:.1}", act.commands.mean),
                                format!("{:.1}", act.tool_errors.mean),
                                format!("{:.0}", act.thinking_tokens.mean),
                                percent(act.io_share.mean),
                                percent(act.skill_opened),
                            ]
                        })
                        .collect(),
                );
            }
            for d in &b.deltas {
                println!(
                    "{} vs {}: {} pass rate (95% CI {} … {}){}",
                    d.arm,
                    d.baseline,
                    delta(d.pass_rate_delta),
                    delta(d.ci_low),
                    delta(d.ci_high),
                    d.p_value
                        .map(|p| format!(", p = {p:.3}"))
                        .unwrap_or_default()
                );
            }
            if !b.expectations.is_empty() {
                println!("\n{}", "expectations".style(st::bold()));
                for e in &b.expectations {
                    let cells: Vec<String> = e
                        .by_arm
                        .iter()
                        .map(|(arm, frac)| format!("{arm} {}", percent(*frac)))
                        .collect();
                    println!(
                        "  {:<16} {} — {}",
                        format!("{:?}", e.pattern).to_lowercase(),
                        truncate(&e.text, 60),
                        cells.join(" | ")
                    );
                }
            }
            if let Some(t) = &b.trigger {
                println!(
                    "\n{} precision {} recall {} f1 {:.2} ({}/{} correct)",
                    "trigger:".style(st::bold()),
                    percent(t.precision),
                    percent(t.recall),
                    t.f1,
                    t.correct,
                    t.total
                );
            }
            for n in &b.notes {
                println!("  • {n}");
            }
            println!("\n{} {}", "workspace:".style(st::dim()), r.workspace);
        } else {
            println!(
                "{} {}/{} done",
                "progress:".style(st::dim()),
                r.progress.done,
                r.progress.total
            );
        }
    })
}
