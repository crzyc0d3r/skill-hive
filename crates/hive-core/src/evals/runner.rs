//! The evaluation engine.
//!
//! [`Hive::start_run`] validates a [`RunConfig`], snapshots the skill into the run workspace,
//! creates the run row and spawns a background task that executes every selected case × arm ×
//! repeat through the executor provider (bounded by a semaphore), scores each result with the
//! deterministic checks and the grader, persists it, evaluates the trigger cases, aggregates a
//! [`crate::model::Benchmark`], writes `benchmark.json` / `benchmark.md` into the workspace and
//! updates the item's effectiveness summary. A run always ends in a terminal state: errors and
//! panics mark it `Failed`, cancellation marks it `Cancelled`.
//!
//! Workspace layout: `<workspaces>/<skill-slug>/<run-id>/` with `skill-snapshot/` (the skill as
//! tested), `cases/<id>-<case-slug>/<arm>/<repeat>/work/` (the sandbox the agent worked in,
//! transcript under `work/.hive/`) and `result.json` next to each `work/`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::error::{HiveError, IoContext, Result};
use crate::hive::Hive;
use crate::model::{
    ArmKind, ArmSpec, AssetKind, CaseResult, CaseStatus, EffectivenessSummary, EvalCase, EvalRun,
    EvalRunDetail, EvalSuite, ExpectationResult, ExpectationSource, HistoryPoint, HiveEvent,
    ProviderRole, RunConfig, RunProgress, RunStatus,
};
use crate::providers::{ExecutionResult, ExecutionTask, Provider, sandbox};
use crate::util;

use super::{analytics, checks, grader, trigger};

/// Label the with-skill arm must carry.
pub const WITH_SKILL_LABEL: &str = "with_skill";
/// Label the without-skill arm must carry.
pub const WITHOUT_SKILL_LABEL: &str = "without_skill";
pub const SNAPSHOT_DIR: &str = "skill-snapshot";
pub const CASES_DIR: &str = "cases";
pub const WORK_DIR: &str = "work";
pub const RESULT_FILE: &str = "result.json";
pub const BENCHMARK_JSON: &str = "benchmark.json";
pub const BENCHMARK_MD: &str = "benchmark.md";
/// Largest transcript returned by [`Hive::read_case_transcript`].
pub const MAX_TRANSCRIPT_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_REPEATS: u32 = 50;
pub const MAX_CONCURRENCY: u32 = 16;
pub const MIN_TIMEOUT_SECS: u32 = 5;
pub const DEFAULT_TIMEOUT_SECS: u32 = 600;
pub const DEFAULT_CONCURRENCY: u32 = 2;
/// Characters of the final answer kept in `CaseResult::output_excerpt`.
pub const OUTPUT_EXCERPT_CHARS: usize = 400;
/// Input files under this prefix land at the sandbox root (skill-creator keeps them there).
pub const EVAL_FILES_PREFIX: &str = "evals/files/";
/// Extra time a provider gets beyond the case timeout before the engine abandons it.
const TIMEOUT_GRACE: Duration = Duration::from_secs(30);

/// Validate arms: non-empty, unique labels, the fixed labels for the with/without arms, and
/// variant paths that are skill directories.
pub fn validate_arms(arms: &[ArmSpec]) -> Result<()> {
    if arms.is_empty() {
        return Err(HiveError::invalid("a run needs at least one arm"));
    }
    let mut labels = HashSet::new();
    for arm in arms {
        let label = arm.label.trim();
        if label.is_empty() {
            return Err(HiveError::invalid("every arm needs a label"));
        }
        if !labels.insert(label.to_string()) {
            return Err(HiveError::invalid(format!(
                "arm label `{label}` is used twice"
            )));
        }
        match &arm.kind {
            ArmKind::WithSkill if label != WITH_SKILL_LABEL => {
                return Err(HiveError::invalid(format!(
                    "the with-skill arm must be labelled `{WITH_SKILL_LABEL}` (got `{label}`)"
                )));
            }
            ArmKind::WithoutSkill if label != WITHOUT_SKILL_LABEL => {
                return Err(HiveError::invalid(format!(
                    "the without-skill arm must be labelled `{WITHOUT_SKILL_LABEL}` (got `{label}`)"
                )));
            }
            ArmKind::Variant { path } => {
                if label == WITH_SKILL_LABEL || label == WITHOUT_SKILL_LABEL {
                    return Err(HiveError::invalid(format!(
                        "variant arms may not use the reserved label `{label}`"
                    )));
                }
                let dir = crate::paths::expand_tilde(path.trim());
                if !crate::skill::is_skill_dir(&dir) {
                    return Err(HiveError::invalid(format!(
                        "variant arm `{label}`: {path} is not a skill directory"
                    )));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Where an input file lands inside the sandbox.
pub fn sandbox_relative(file: &str) -> &str {
    file.strip_prefix(EVAL_FILES_PREFIX).unwrap_or(file)
}

fn status_word(status: CaseStatus) -> &'static str {
    match status {
        CaseStatus::Queued => "queued",
        CaseStatus::Running => "running",
        CaseStatus::Executed => "executed",
        CaseStatus::Graded => "graded",
        CaseStatus::Failed => "failed",
        CaseStatus::Timeout => "timed out",
        CaseStatus::Cancelled => "cancelled",
    }
}

struct Job {
    case: EvalCase,
    arm: ArmSpec,
    repeat: u32,
}

struct RunContext {
    hive: Arc<Hive>,
    run_id: String,
    item_id: String,
    skill_name: String,
    skill_description: String,
    workspace: PathBuf,
    /// The skill snapshot every arm and input file resolves against.
    skill_root: PathBuf,
    suite: EvalSuite,
    config: RunConfig,
    executor: Arc<dyn Provider>,
    grader: Option<Arc<dyn Provider>>,
    cancel: CancellationToken,
    progress: Mutex<RunProgress>,
    semaphore: Semaphore,
}

impl RunContext {
    fn jobs(&self) -> Vec<Job> {
        let mut jobs = Vec::new();
        for case in &self.suite.cases {
            for arm in &self.config.arms {
                for repeat in 1..=self.config.repeats.max(1) {
                    jobs.push(Job {
                        case: case.clone(),
                        arm: arm.clone(),
                        repeat,
                    });
                }
            }
        }
        jobs
    }

    fn progress_snapshot(&self, message: Option<&str>) -> RunProgress {
        let mut p = self.progress.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(m) = message {
            p.message = Some(m.to_string());
        }
        p.clone()
    }

    /// Count a finished case and persist the progress. Serialized by the progress mutex.
    fn bump(&self, status: CaseStatus) -> RunProgress {
        let mut p = self.progress.lock().unwrap_or_else(|e| e.into_inner());
        p.done += 1;
        if matches!(status, CaseStatus::Failed | CaseStatus::Timeout) {
            p.failed += 1;
        }
        p.message = Some(format!("{} of {} cases finished", p.done, p.total));
        let snapshot = p.clone();
        if let Ok(mut run) = self.hive.db.require_run(&self.run_id) {
            if !run.status.is_terminal() {
                run.progress = snapshot.clone();
                let _ = self.hive.db.update_run(&run);
            }
        }
        snapshot
    }

    fn emit_run(&self, status: RunStatus, progress: RunProgress) {
        self.hive.emit(HiveEvent::RunUpdated {
            run_id: self.run_id.clone(),
            status,
            progress,
        });
    }

    fn update_status(&self, status: RunStatus, message: &str) -> Result<EvalRun> {
        let mut run = self.hive.db.require_run(&self.run_id)?;
        run.status = status;
        run.progress = self.progress_snapshot(Some(message));
        let now = util::now_ms();
        if status == RunStatus::Running && run.started_at.is_none() {
            run.started_at = Some(now);
        }
        if status.is_terminal() {
            run.finished_at = Some(now);
        }
        self.hive.db.update_run(&run)?;
        self.emit_run(status, run.progress.clone());
        Ok(run)
    }

    fn mark_cancelled(&self) -> Result<()> {
        let mut run = self.hive.db.require_run(&self.run_id)?;
        if run.status.is_terminal() {
            return Ok(());
        }
        run.status = RunStatus::Cancelled;
        run.error = Some("cancelled".into());
        run.finished_at = Some(util::now_ms());
        run.progress = self.progress_snapshot(Some("cancelled"));
        self.hive.db.update_run(&run)?;
        self.emit_run(RunStatus::Cancelled, run.progress);
        Ok(())
    }
}

/// Supervise the run task: whatever happens, the run ends in a terminal state.
async fn supervise(ctx: Arc<RunContext>) {
    let worker = tokio::spawn({
        let ctx = Arc::clone(&ctx);
        async move { execute_run(ctx).await }
    });
    let error = match worker.await {
        Ok(Ok(())) => None,
        Ok(Err(e)) => Some(e.to_string()),
        Err(join) => Some(format!("run task panicked: {join}")),
    };
    if let Some(message) = &error {
        tracing::warn!(run = %ctx.run_id, "evaluation run ended with an error: {message}");
    }
    if let Ok(mut run) = ctx.hive.db.require_run(&ctx.run_id) {
        if !run.status.is_terminal() {
            let cancelled = ctx.cancel.is_cancelled();
            run.status = if cancelled {
                RunStatus::Cancelled
            } else {
                RunStatus::Failed
            };
            run.error = error.or_else(|| {
                Some(if cancelled {
                    "cancelled".to_string()
                } else {
                    "run ended without a result".to_string()
                })
            });
            run.finished_at = Some(util::now_ms());
            run.progress = ctx.progress_snapshot(None);
            let _ = ctx.hive.db.update_run(&run);
            ctx.emit_run(run.status, run.progress.clone());
        }
    }
    ctx.hive.runs.remove(&ctx.run_id);
}

async fn execute_run(ctx: Arc<RunContext>) -> Result<()> {
    let hive = &ctx.hive;
    ctx.update_status(RunStatus::Running, "running cases")?;

    let mut set = JoinSet::new();
    for job in ctx.jobs() {
        let ctx = Arc::clone(&ctx);
        set.spawn(async move { run_job(ctx, job).await });
    }
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                set.abort_all();
                return Err(e);
            }
            Err(join) => {
                set.abort_all();
                return Err(HiveError::msg(format!("case task panicked: {join}")));
            }
        }
    }
    if ctx.cancel.is_cancelled() {
        return ctx.mark_cancelled();
    }

    let mut run = ctx.update_status(RunStatus::Grading, "aggregating results")?;
    let mut trigger_error = None;
    let trigger_summary = if ctx.config.include_trigger && !ctx.suite.trigger_cases.is_empty() {
        ctx.emit_run(
            RunStatus::Grading,
            ctx.progress_snapshot(Some("evaluating trigger queries")),
        );
        let provider = ctx
            .grader
            .clone()
            .unwrap_or_else(|| Arc::clone(&ctx.executor));
        let others = other_skills(hive, &ctx.item_id)?;
        match trigger::evaluate(
            provider.as_ref(),
            &ctx.skill_name,
            &ctx.skill_description,
            &others,
            &ctx.suite.trigger_cases,
            &ctx.cancel,
        )
        .await
        {
            Ok(summary) => Some(summary),
            Err(HiveError::Cancelled) => return ctx.mark_cancelled(),
            Err(e) => {
                trigger_error = Some(e.to_string());
                None
            }
        }
    } else {
        None
    };
    if ctx.cancel.is_cancelled() {
        return ctx.mark_cancelled();
    }

    let results = hive.db.case_results(&ctx.run_id)?;
    let mut benchmark = analytics::aggregate(
        &run,
        &results,
        trigger_summary,
        ctx.executor.name(),
        ctx.grader.as_ref().map(|g| g.name()),
    );
    if let Some(e) = trigger_error {
        benchmark
            .notes
            .push(format!("trigger evaluation failed: {e}"));
    }
    util::write_json_atomic(&ctx.workspace.join(BENCHMARK_JSON), &benchmark)?;
    util::write_atomic(
        &ctx.workspace.join(BENCHMARK_MD),
        analytics::benchmark_markdown(&benchmark).as_bytes(),
    )?;

    let finished_at = util::now_ms();
    run.summary = Some(benchmark.clone());
    run.status = RunStatus::Done;
    run.finished_at = Some(finished_at);
    run.progress = ctx.progress_snapshot(Some("done"));
    hive.db.update_run(&run)?;

    let with = benchmark.arms.iter().find(|a| a.label == WITH_SKILL_LABEL);
    let base = benchmark
        .arms
        .iter()
        .find(|a| a.label == benchmark.baseline);
    if let (Some(with), Some(base)) = (with, base) {
        if with.label != base.label && hive.db.get_item(&ctx.item_id)?.is_some() {
            hive.db.set_item_effectiveness(
                &ctx.item_id,
                Some(&EffectivenessSummary {
                    run_id: ctx.run_id.clone(),
                    verdict: benchmark.verdict,
                    with_skill_pass_rate: with.pass_rate.mean,
                    baseline_pass_rate: base.pass_rate.mean,
                    delta: with.pass_rate.mean - base.pass_rate.mean,
                    finished_at,
                }),
            )?;
            hive.emit(HiveEvent::LibraryChanged);
        }
    }
    ctx.emit_run(RunStatus::Done, run.progress.clone());
    Ok(())
}

fn other_skills(hive: &Hive, item_id: &str) -> Result<Vec<(String, String)>> {
    Ok(hive
        .db
        .all_items()?
        .into_iter()
        .filter(|i| i.kind == AssetKind::Skill && i.id != item_id)
        .take(trigger::MAX_OTHER_SKILLS)
        .map(|i| (i.name, i.description))
        .collect())
}

async fn run_job(ctx: Arc<RunContext>, job: Job) -> Result<()> {
    let permit = tokio::select! {
        p = ctx.semaphore.acquire() => p.map_err(|_| HiveError::msg("run semaphore closed"))?,
        _ = ctx.cancel.cancelled() => return Ok(()),
    };
    if ctx.cancel.is_cancelled() {
        return Ok(());
    }
    let result = execute_job(&ctx, &job).await;
    drop(permit);
    ctx.hive.db.upsert_case_result(&result)?;
    if let Some(case_dir) = Path::new(&result.sandbox_path).parent() {
        let _ = util::write_json_atomic(&case_dir.join(RESULT_FILE), &result);
    }
    let progress = ctx.bump(result.status);
    ctx.hive.emit(HiveEvent::CaseFinished {
        run_id: ctx.run_id.clone(),
        result,
    });
    ctx.emit_run(RunStatus::Running, progress);
    Ok(())
}

fn prepare_sandbox(ctx: &RunContext, job: &Job, sandbox: &Path) -> Result<Option<PathBuf>> {
    std::fs::create_dir_all(sandbox).at(sandbox)?;
    for file in &job.case.files {
        if !util::is_safe_relative(file) {
            return Err(HiveError::invalid(format!(
                "input file `{file}` escapes the skill directory"
            )));
        }
        let src = ctx.skill_root.join(file);
        if !src.exists() {
            return Err(HiveError::not_found(format!(
                "input file `{file}` (expected at {})",
                src.display()
            )));
        }
        let dest = sandbox.join(sandbox_relative(file));
        if src.is_dir() {
            util::copy_dir(&src, &dest)?;
        } else {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).at(parent)?;
            }
            std::fs::copy(&src, &dest).at(&src)?;
        }
    }
    Ok(match &job.arm.kind {
        ArmKind::WithSkill => Some(ctx.skill_root.clone()),
        ArmKind::WithoutSkill => None,
        ArmKind::Variant { path } => Some(crate::paths::expand_tilde(path.trim())),
    })
}

async fn execute_job(ctx: &RunContext, job: &Job) -> CaseResult {
    let case_slug = format!("{}-{}", job.case.id, util::slugify(&job.case.name));
    let case_dir = ctx
        .workspace
        .join(CASES_DIR)
        .join(&case_slug)
        .join(&job.arm.label)
        .join(job.repeat.to_string());
    let sandbox = case_dir.join(WORK_DIR);
    let label = format!("{case_slug}/{}/{}", job.arm.label, job.repeat);
    let mut result = CaseResult {
        id: util::new_id(),
        run_id: ctx.run_id.clone(),
        case_id: job.case.id,
        case_name: job.case.name.clone(),
        arm: job.arm.label.clone(),
        repeat: job.repeat,
        status: CaseStatus::Running,
        expectations: Vec::new(),
        passed: 0,
        failed: 0,
        total: 0,
        pass_rate: 0.0,
        input_tokens: 0,
        output_tokens: 0,
        duration_ms: 0,
        tool_calls: 0,
        cost_usd: None,
        output_excerpt: String::new(),
        sandbox_path: sandbox.to_string_lossy().to_string(),
        transcript_path: None,
        grader_notes: None,
        error: None,
    };
    let started = Instant::now();
    let timeout = Duration::from_secs(ctx.config.timeout_secs.max(MIN_TIMEOUT_SECS) as u64);
    let outcome: Result<ExecutionResult> = match prepare_sandbox(ctx, job, &sandbox) {
        Err(e) => Err(e),
        Ok(skill_dir) => {
            let task = ExecutionTask {
                prompt: job.case.prompt.clone(),
                cwd: sandbox.clone(),
                skill_dir,
                timeout,
                cancel: ctx.cancel.child_token(),
                label,
            };
            match tokio::time::timeout(timeout + TIMEOUT_GRACE, ctx.executor.execute(task)).await {
                Ok(r) => r,
                Err(_) => Err(HiveError::Timeout(timeout.as_secs())),
            }
        }
    };
    let elapsed = started.elapsed().as_millis() as u64;
    let mut output = String::new();
    let mut files_created = Vec::new();
    let mut transcript = None;
    match outcome {
        Ok(res) => {
            result.status = CaseStatus::Executed;
            result.input_tokens = res.input_tokens;
            result.output_tokens = res.output_tokens;
            result.duration_ms = if res.duration_ms > 0 {
                res.duration_ms
            } else {
                elapsed
            };
            result.tool_calls = res.tool_calls;
            result.cost_usd = res.cost_usd;
            result.transcript_path = res
                .transcript_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string());
            output = res.output;
            files_created = res.files_created;
            transcript = res.transcript_path;
        }
        Err(HiveError::Timeout(secs)) => {
            result.status = CaseStatus::Timeout;
            result.error = Some(format!("timed out after {secs} s"));
            result.duration_ms = elapsed;
        }
        Err(HiveError::Cancelled) => {
            result.status = CaseStatus::Cancelled;
            result.error = Some("cancelled".into());
            result.duration_ms = elapsed;
        }
        Err(e) => {
            result.status = CaseStatus::Failed;
            result.error = Some(e.to_string());
            result.duration_ms = elapsed;
        }
    }
    if result.transcript_path.is_none() {
        let p = sandbox::transcript_path(&sandbox);
        if p.is_file() {
            result.transcript_path = Some(p.to_string_lossy().to_string());
        }
    }
    if result.status == CaseStatus::Cancelled {
        return result;
    }
    result.output_excerpt = util::truncate_chars(&output, OUTPUT_EXCERPT_CHARS);

    let graded = grader::graded_expectations(&job.case);
    let mut expectations: Vec<ExpectationResult>;
    if result.status == CaseStatus::Executed {
        expectations = checks::run_checks(&job.case.checks, &sandbox, &output).await;
        if !graded.is_empty() {
            match &ctx.grader {
                Some(g) => {
                    let transcript_text = transcript
                        .as_deref()
                        .and_then(|p| grader::read_transcript_excerpt(p, grader::TRANSCRIPT_CAP));
                    let files = grader::files_for_grader(&sandbox, &files_created);
                    let input = grader::GradeInput {
                        task_prompt: &job.case.prompt,
                        expectations: &graded,
                        output: &output,
                        transcript: transcript_text.as_deref(),
                        files: &files,
                    };
                    match grader::grade(g.as_ref(), &input, &ctx.cancel).await {
                        Ok(outcome) => {
                            expectations.extend(outcome.results);
                            result.grader_notes = outcome.notes;
                            result.status = CaseStatus::Graded;
                        }
                        Err(HiveError::Cancelled) => {
                            result.status = CaseStatus::Cancelled;
                            result.error = Some("cancelled during grading".into());
                            return result;
                        }
                        Err(e) => {
                            result.grader_notes = Some(format!(
                                "grader failed: {e}. {} expectation(s) without checks were not scored.",
                                graded.len()
                            ));
                        }
                    }
                }
                None => {
                    result.grader_notes = Some(format!(
                        "{}: {} expectation(s) without deterministic checks were not scored.",
                        grader::NO_GRADER_NOTE,
                        graded.len()
                    ));
                }
            }
        }
    } else {
        let evidence = format!(
            "execution {}: {}",
            status_word(result.status),
            result.error.clone().unwrap_or_default()
        );
        expectations = job
            .case
            .checks
            .iter()
            .map(|c| ExpectationResult {
                text: checks::check_text(c),
                passed: false,
                evidence: evidence.clone(),
                source: ExpectationSource::Check,
            })
            .collect();
        if ctx.grader.is_some() {
            expectations.extend(graded.iter().map(|t| ExpectationResult {
                text: t.clone(),
                passed: false,
                evidence: evidence.clone(),
                source: ExpectationSource::Grader,
            }));
        }
    }
    result.total = expectations.len() as u32;
    result.passed = expectations.iter().filter(|e| e.passed).count() as u32;
    result.failed = result.total - result.passed;
    result.pass_rate = if result.total > 0 {
        result.passed as f64 / result.total as f64
    } else {
        0.0
    };
    result.expectations = expectations;
    result
}

impl Hive {
    /// Validate the configuration, snapshot the skill, create the run and start executing it
    /// in the background. Returns the queued run immediately; follow progress through
    /// [`HiveEvent::RunUpdated`] / [`HiveEvent::CaseFinished`] or [`Hive::run_detail`].
    pub async fn start_run(self: &Arc<Self>, config: RunConfig) -> Result<EvalRun> {
        let mut config = config;
        let item = self.get_item(&config.item_id)?;
        if item.kind != AssetKind::Skill {
            return Err(HiveError::invalid(
                "evaluations are only supported for skills",
            ));
        }
        let mut suite = self.eval_suite(&item.id)?;
        if suite.cases.is_empty() {
            return Err(HiveError::invalid(
                "the skill has no eval cases; create a suite first",
            ));
        }
        validate_arms(&config.arms)?;
        for arm in &mut config.arms {
            arm.label = arm.label.trim().to_string();
        }
        if !config.case_ids.is_empty() {
            let known: HashSet<u32> = suite.cases.iter().map(|c| c.id).collect();
            if let Some(missing) = config.case_ids.iter().find(|id| !known.contains(id)) {
                return Err(HiveError::invalid(format!(
                    "case {missing} does not exist in the suite"
                )));
            }
            suite.cases.retain(|c| config.case_ids.contains(&c.id));
        }
        config.repeats = config.repeats.clamp(1, MAX_REPEATS);
        config.timeout_secs = config.timeout_secs.max(MIN_TIMEOUT_SECS);
        let executor_cfg = self.db.require_provider(&config.executor_provider_id)?;
        let executor = self.provider(&executor_cfg.id)?;
        let grader = match config.grader_provider_id.as_deref() {
            Some(id) if !id.trim().is_empty() => Some(self.provider(id)?),
            _ => {
                config.grader_provider_id = None;
                None
            }
        };
        config.concurrency = config
            .concurrency
            .clamp(1, MAX_CONCURRENCY)
            .min(executor_cfg.max_concurrency.max(1));

        let run_id = util::new_id();
        let workspace = self.paths.workspaces_dir().join(&item.slug).join(&run_id);
        std::fs::create_dir_all(&workspace).at(&workspace)?;
        let skill_root = workspace.join(SNAPSHOT_DIR);
        util::copy_dir(Path::new(&item.path), &skill_root)?;
        let total = (suite.cases.len() * config.arms.len() * config.repeats as usize) as u32;
        let progress = RunProgress {
            total,
            done: 0,
            failed: 0,
            message: Some("queued".into()),
        };
        let run = EvalRun {
            id: run_id.clone(),
            item_id: Some(item.id.clone()),
            skill_name: item.name.clone(),
            skill_hash: item.content_hash.clone(),
            config: config.clone(),
            status: RunStatus::Queued,
            progress: progress.clone(),
            workspace: workspace.to_string_lossy().to_string(),
            summary: None,
            error: None,
            created_at: util::now_ms(),
            started_at: None,
            finished_at: None,
        };
        self.db.insert_run(&run)?;
        let cancel = self.runs.register(&run_id);
        self.emit(HiveEvent::RunUpdated {
            run_id: run_id.clone(),
            status: RunStatus::Queued,
            progress: progress.clone(),
        });
        let ctx = Arc::new(RunContext {
            hive: Arc::clone(self),
            run_id,
            item_id: item.id.clone(),
            skill_name: item.name.clone(),
            skill_description: item.description.clone(),
            workspace,
            skill_root,
            suite,
            semaphore: Semaphore::new(config.concurrency as usize),
            config,
            executor,
            grader,
            cancel,
            progress: Mutex::new(progress),
        });
        tokio::spawn(supervise(ctx));
        Ok(run)
    }

    /// Cancel an in-flight run. In-flight cases end as `Cancelled`; queued ones never start.
    pub fn cancel_run(&self, run_id: &str) -> Result<()> {
        let mut run = self.db.require_run(run_id)?;
        if run.status.is_terminal() {
            return Err(HiveError::invalid(format!(
                "run {run_id} already finished ({})",
                run.status.as_str()
            )));
        }
        if !self.runs.cancel(run_id) {
            // Not running in this process (left over from a previous one): close it directly.
            run.status = RunStatus::Cancelled;
            run.error = Some("cancelled".into());
            run.finished_at = Some(util::now_ms());
            self.db.update_run(&run)?;
            self.emit(HiveEvent::RunUpdated {
                run_id: run_id.to_string(),
                status: RunStatus::Cancelled,
                progress: run.progress,
            });
        }
        Ok(())
    }

    /// Recent runs, newest first, optionally for one item.
    pub fn list_runs(&self, item_id: Option<&str>, limit: u32) -> Result<Vec<EvalRun>> {
        self.db.list_runs(item_id, limit.clamp(1, 500))
    }

    /// A run with every case result.
    pub fn run_detail(&self, run_id: &str) -> Result<EvalRunDetail> {
        Ok(EvalRunDetail {
            run: self.db.require_run(run_id)?,
            results: self.db.case_results(run_id)?,
        })
    }

    /// Delete a finished run, its case results and its workspace directory.
    pub fn delete_run(&self, run_id: &str) -> Result<()> {
        let run = self.db.require_run(run_id)?;
        if !run.status.is_terminal() && self.runs.is_active(run_id) {
            return Err(HiveError::invalid("cancel the run before deleting it"));
        }
        self.db.delete_run(run_id)?;
        let workspace = PathBuf::from(&run.workspace);
        if workspace.starts_with(self.paths.workspaces_dir()) && workspace.exists() {
            util::remove_path(&workspace)?;
            // Drop the skill directory when it holds no other runs.
            if let Some(parent) = workspace.parent() {
                if std::fs::read_dir(parent)
                    .map(|mut d| d.next().is_none())
                    .unwrap_or(false)
                {
                    let _ = std::fs::remove_dir(parent);
                }
            }
        }
        Ok(())
    }

    /// Effectiveness history of an item from its finished runs.
    pub fn history(&self, item_id: &str) -> Result<Vec<HistoryPoint>> {
        self.db.history_for_item(item_id, 200)
    }

    /// The markdown report of a finished run.
    pub fn benchmark_markdown(&self, run_id: &str) -> Result<String> {
        let run = self.db.require_run(run_id)?;
        let path = Path::new(&run.workspace).join(BENCHMARK_MD);
        if path.is_file() {
            return std::fs::read_to_string(&path).at(&path);
        }
        match &run.summary {
            Some(b) => Ok(analytics::benchmark_markdown(b)),
            None => Err(HiveError::invalid(format!(
                "run {run_id} has no benchmark yet ({})",
                run.status.as_str()
            ))),
        }
    }

    /// The transcript of one case result, capped at 2 MB.
    pub fn read_case_transcript(&self, run_id: &str, case_result_id: &str) -> Result<String> {
        let results = self.db.case_results(run_id)?;
        let result = results
            .iter()
            .find(|r| r.id == case_result_id)
            .ok_or_else(|| HiveError::not_found(format!("case result {case_result_id}")))?;
        let path = result
            .transcript_path
            .as_deref()
            .ok_or_else(|| HiveError::not_found("this case has no transcript"))?;
        let bytes = std::fs::read(path).at(path)?;
        if bytes.len() <= MAX_TRANSCRIPT_BYTES {
            return Ok(String::from_utf8_lossy(&bytes).to_string());
        }
        let mut text = String::from_utf8_lossy(&bytes[..MAX_TRANSCRIPT_BYTES]).to_string();
        text.push_str(&format!(
            "\n… [truncated: {} of {} bytes shown]\n",
            MAX_TRANSCRIPT_BYTES,
            bytes.len()
        ));
        Ok(text)
    }

    /// A sensible configuration for a skill: with-skill vs without-skill, one repeat, two
    /// parallel cases, a 10 minute timeout, the default executor and grader, and the trigger
    /// evaluation when the suite has trigger cases.
    pub fn default_run_config(&self, item_id: &str) -> Result<RunConfig> {
        let item = self.get_item(item_id)?;
        let suite = self.eval_suite(&item.id)?;
        let executor = self
            .default_provider_for(ProviderRole::Executor)?
            .map(|p| p.id)
            .unwrap_or_default();
        let grader = self
            .default_provider_for(ProviderRole::Grader)?
            .map(|p| p.id);
        Ok(RunConfig {
            item_id: item.id,
            arms: vec![
                ArmSpec {
                    label: WITH_SKILL_LABEL.into(),
                    kind: ArmKind::WithSkill,
                },
                ArmSpec {
                    label: WITHOUT_SKILL_LABEL.into(),
                    kind: ArmKind::WithoutSkill,
                },
            ],
            case_ids: Vec::new(),
            repeats: 1,
            concurrency: DEFAULT_CONCURRENCY,
            executor_provider_id: executor,
            grader_provider_id: grader,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            include_trigger: !suite.trigger_cases.is_empty(),
            baseline_label: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arm_validation() {
        let with = ArmSpec {
            label: WITH_SKILL_LABEL.into(),
            kind: ArmKind::WithSkill,
        };
        let without = ArmSpec {
            label: WITHOUT_SKILL_LABEL.into(),
            kind: ArmKind::WithoutSkill,
        };
        assert!(validate_arms(&[with.clone(), without.clone()]).is_ok());
        assert!(validate_arms(&[]).is_err());
        assert!(validate_arms(&[with.clone(), with.clone()]).is_err());
        assert!(
            validate_arms(&[ArmSpec {
                label: "skill".into(),
                kind: ArmKind::WithSkill
            }])
            .is_err()
        );
        assert!(
            validate_arms(&[ArmSpec {
                label: "base".into(),
                kind: ArmKind::WithoutSkill
            }])
            .is_err()
        );
        assert!(
            validate_arms(&[ArmSpec {
                label: " ".into(),
                kind: ArmKind::WithoutSkill
            }])
            .is_err()
        );
        assert!(
            validate_arms(&[ArmSpec {
                label: WITH_SKILL_LABEL.into(),
                kind: ArmKind::Variant { path: "/x".into() }
            }])
            .is_err()
        );
        assert!(
            validate_arms(&[ArmSpec {
                label: "v2".into(),
                kind: ArmKind::Variant {
                    path: "/definitely/missing".into()
                }
            }])
            .is_err()
        );
        let tmp = tempfile::tempdir().unwrap();
        let dir = crate::library::test_support::write_skill(
            tmp.path(),
            "v2",
            "A variant for testing things",
            "b",
        );
        assert!(
            validate_arms(&[
                with,
                ArmSpec {
                    label: "v2".into(),
                    kind: ArmKind::Variant {
                        path: dir.to_string_lossy().to_string()
                    }
                }
            ])
            .is_ok()
        );
    }

    #[test]
    fn input_file_destinations() {
        assert_eq!(sandbox_relative("evals/files/report.pdf"), "report.pdf");
        assert_eq!(sandbox_relative("data/in.csv"), "data/in.csv");
    }
}
