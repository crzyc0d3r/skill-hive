//! End-to-end tests of the evaluation engine through the scripted provider (fully offline).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::broadcast;

use crate::hive::Hive;
use crate::library::test_support::write_skill;
use crate::model::{
    CaseStatus, Check, CheckKind, EvalCase, EvalSuite, ExpectationPattern, HiveEvent, LibraryItem,
    ProviderConfig, ProviderInput, ProviderKind, RunConfig, RunStatus, TriggerCase, Verdict,
};
use crate::providers::scripted::test_support::write_script;

use super::grader::NO_GRADER_NOTE;
use super::runner::{BENCHMARK_MD, WITH_SKILL_LABEL, WITHOUT_SKILL_LABEL};

struct Fixture {
    _tmp: tempfile::TempDir,
    hive: Arc<Hive>,
    item: LibraryItem,
    provider: ProviderConfig,
    script_dir: PathBuf,
}

fn suite_for(item: &LibraryItem) -> EvalSuite {
    EvalSuite {
        item_id: item.id.clone(),
        skill_name: item.name.clone(),
        cases: vec![
            EvalCase {
                id: 1,
                name: "status-report".into(),
                prompt: "Write a status report for the project into report.md".into(),
                expected_output: None,
                files: vec!["evals/files/notes.txt".into()],
                expectations: vec![
                    "report.md exists".into(),
                    "The report states the project status".into(),
                ],
                checks: vec![Check {
                    id: String::new(),
                    expectation: Some("report.md exists".into()),
                    kind: CheckKind::FileExists {
                        path: "report.md".into(),
                    },
                }],
            },
            EvalCase {
                id: 2,
                name: "weekly-summary".into(),
                prompt: "Summarise the week into summary.md with a heading".into(),
                expected_output: None,
                files: vec![],
                expectations: vec![
                    "summary.md starts with a markdown heading".into(),
                    "The summary is concise".into(),
                ],
                checks: vec![Check {
                    id: String::new(),
                    expectation: Some("summary.md starts with a markdown heading".into()),
                    kind: CheckKind::FileContains {
                        path: "summary.md".into(),
                        text: "# ".into(),
                    },
                }],
            },
        ],
        trigger_cases: vec![
            TriggerCase {
                query: "Can you write a status report for my project?".into(),
                should_trigger: true,
            },
            TriggerCase {
                query: "Give me the weekly summary".into(),
                should_trigger: true,
            },
            TriggerCase {
                query: "What's the weather in Paris?".into(),
                should_trigger: false,
            },
            TriggerCase {
                query: "Convert this CSV to JSON".into(),
                should_trigger: false,
            },
        ],
        path: String::new(),
        exists: false,
    }
}

/// The reference script: with the skill the agent writes the right file and marks its output;
/// without it, it does not. The grader passes when the output carries the marker; the trigger
/// rules route report/summary queries to the skill.
fn reference_script() -> serde_json::Value {
    serde_json::json!({
        "rules": [
            {"system": "(?i)available skills", "match": "(?i)report|summary", "output": "{\"skill\": \"report-writer\"}"},
            {"system": "(?i)available skills", "output": "{\"skill\": null}"},
            {"system": "(?i)strict grader", "match": "MARKER",
             "output": "{\"expectations\":[{\"text\":\"x\",\"passed\":true,\"evidence\":\"the output carries the marker\"}],\"notes\":\"looks good\"}"},
            {"system": "(?i)strict grader",
             "output": "{\"expectations\":[{\"text\":\"x\",\"passed\":false,\"evidence\":\"no marker in the output\"}],\"notes\":\"nothing was produced\"}"},
            {"match": "(?s)skill is available.*report\\.md", "with_skill": true,
             "output": "Wrote report.md as the skill instructs. MARKER",
             "files": {"report.md": "# Status\n\nThe project status is green.\n"},
             "tool_calls": 3, "input_tokens": 1500, "output_tokens": 200, "cost_usd": 0.01},
            {"match": "(?s)skill is available.*summary\\.md", "with_skill": true,
             "output": "Wrote summary.md. MARKER",
             "files": {"summary.md": "# Week 12\n\nShipped the thing.\n"},
             "tool_calls": 2, "input_tokens": 1400, "output_tokens": 150, "cost_usd": 0.008}
        ],
        "default": {"output": "I am not sure which format you want, so here is a paragraph instead.",
                    "tool_calls": 1, "input_tokens": 700, "output_tokens": 120, "cost_usd": 0.003}
    })
}

fn fixture(script: &serde_json::Value) -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let hive = Hive::open_temp(tmp.path()).unwrap();
    let src = tmp.path().join("src");
    let skill = write_skill(
        &src,
        "report-writer",
        "Write project status reports and weekly summaries. Use when the user asks for a status report or a weekly summary.",
        "# Report writer\n\nWrite a markdown file with a heading and the status.",
    );
    std::fs::create_dir_all(skill.join("evals/files")).unwrap();
    std::fs::write(skill.join("evals/files/notes.txt"), "shipped the thing\n").unwrap();
    let item = hive.import_path(&skill, &[], false).unwrap();
    let other = write_skill(
        &src,
        "pdf-tools",
        "Extract tables from PDF files when the user shares a PDF",
        "# PDF",
    );
    hive.import_path(&other, &[], false).unwrap();
    hive.save_eval_suite(&suite_for(&item)).unwrap();
    let item = hive.get_item(&item.id).unwrap();
    let script_dir = tmp.path().join("scripts");
    std::fs::create_dir_all(&script_dir).unwrap();
    let path = write_script(&script_dir, "exec", script);
    let provider = hive
        .save_provider(
            None,
            &ProviderInput {
                name: "scripted".into(),
                kind: ProviderKind::Scripted,
                model: None,
                base_url: None,
                command: Some(path.to_string_lossy().to_string()),
                args: vec![],
                env: Default::default(),
                api_key_env: None,
                api_key: None,
                roles: vec![],
                timeout_secs: None,
                max_concurrency: Some(4),
            },
        )
        .unwrap();
    Fixture {
        _tmp: tmp,
        hive,
        item,
        provider,
        script_dir,
    }
}

fn config(f: &Fixture, repeats: u32, grader: bool, trigger: bool) -> RunConfig {
    let mut c = f.hive.default_run_config(&f.item.id).unwrap();
    c.repeats = repeats;
    c.grader_provider_id = grader.then(|| f.provider.id.clone());
    c.include_trigger = trigger;
    c
}

async fn wait_for_terminal(
    rx: &mut broadcast::Receiver<HiveEvent>,
    run_id: &str,
    timeout: Duration,
) -> RunStatus {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Ok(HiveEvent::RunUpdated {
                run_id: id, status, ..
            })) if id == run_id && status.is_terminal() => {
                return status;
            }
            Ok(Ok(_)) | Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
            Ok(Err(_)) => panic!("event bus closed"),
            Err(_) => panic!("timed out waiting for run {run_id}"),
        }
    }
}

async fn wait_for_running(rx: &mut broadcast::Receiver<HiveEvent>, run_id: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Ok(HiveEvent::RunUpdated {
                run_id: id,
                status: RunStatus::Running,
                ..
            })) if id == run_id => return,
            Ok(Ok(_)) | Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
            _ => panic!("run {run_id} never started"),
        }
    }
}

#[tokio::test]
async fn end_to_end_with_skill_beats_baseline() {
    let f = fixture(&reference_script());
    let mut rx = f.hive.subscribe();
    let cfg = config(&f, 2, true, true);
    assert_eq!(cfg.arms.len(), 2);
    assert_eq!(cfg.arms[0].label, WITH_SKILL_LABEL);
    assert_eq!(cfg.arms[1].label, WITHOUT_SKILL_LABEL);
    assert_eq!(cfg.executor_provider_id, f.provider.id);
    assert_eq!(cfg.timeout_secs, 600);
    assert!(cfg.include_trigger);

    let run = f.hive.start_run(cfg).await.unwrap();
    assert_eq!(run.status, RunStatus::Queued);
    assert_eq!(run.progress.total, 8);
    assert_eq!(run.skill_hash, f.item.content_hash);
    assert!(
        Path::new(&run.workspace)
            .join("skill-snapshot/SKILL.md")
            .is_file()
    );

    let status = wait_for_terminal(&mut rx, &run.id, Duration::from_secs(60)).await;
    assert_eq!(status, RunStatus::Done);

    let detail = f.hive.run_detail(&run.id).unwrap();
    assert_eq!(detail.run.status, RunStatus::Done);
    assert_eq!(detail.run.progress.done, 8);
    assert_eq!(detail.run.progress.failed, 0);
    assert!(detail.run.started_at.is_some() && detail.run.finished_at.is_some());
    assert_eq!(detail.results.len(), 8);
    assert!(
        detail
            .results
            .iter()
            .all(|r| r.status == CaseStatus::Graded),
        "{:?}",
        detail.results.iter().map(|r| r.status).collect::<Vec<_>>()
    );
    assert!(detail.results.iter().all(|r| r.total == 2));
    for r in &detail.results {
        if r.arm == WITH_SKILL_LABEL {
            assert_eq!(r.pass_rate, 1.0, "{r:?}");
            assert!(r.output_excerpt.contains("MARKER"));
            assert_eq!(r.grader_notes.as_deref(), Some("looks good"));
            assert!(r.tool_calls >= 2);
        } else {
            assert_eq!(r.pass_rate, 0.0, "{r:?}");
            assert!(r.expectations.iter().all(|e| !e.passed));
        }
        assert!(Path::new(&r.sandbox_path).is_dir());
        assert!(r.transcript_path.is_some());
        // The input file was copied to the sandbox root for case 1.
        if r.case_id == 1 {
            assert!(Path::new(&r.sandbox_path).join("notes.txt").is_file());
        }
    }
    // Checks decide their expectation; the grader decides the other.
    let with_case1 = detail
        .results
        .iter()
        .find(|r| r.arm == WITH_SKILL_LABEL && r.case_id == 1)
        .unwrap();
    assert_eq!(
        with_case1.expectations[0].source,
        crate::model::ExpectationSource::Check
    );
    assert_eq!(
        with_case1.expectations[1].source,
        crate::model::ExpectationSource::Grader
    );
    assert_eq!(
        with_case1.expectations[1].evidence,
        "the output carries the marker"
    );

    let summary = detail.run.summary.clone().expect("benchmark");
    let with = summary
        .arms
        .iter()
        .find(|a| a.label == WITH_SKILL_LABEL)
        .unwrap();
    let without = summary
        .arms
        .iter()
        .find(|a| a.label == WITHOUT_SKILL_LABEL)
        .unwrap();
    assert_eq!(summary.baseline, WITHOUT_SKILL_LABEL);
    assert!(with.pass_rate.mean > without.pass_rate.mean);
    assert_eq!(with.pass_rate.mean, 1.0);
    assert_eq!(without.pass_rate.mean, 0.0);
    assert_eq!((with.runs, without.runs), (4, 4));
    assert!(with.tokens.mean > without.tokens.mean);
    assert!(with.cost_usd.unwrap() > without.cost_usd.unwrap());
    assert!(
        matches!(summary.verdict, Verdict::Helps | Verdict::HelpsALot),
        "{:?}",
        summary.verdict
    );
    assert_eq!(summary.deltas.len(), 1);
    assert_eq!(summary.deltas[0].pass_rate_delta, 1.0);
    assert!(summary.deltas[0].ci_low > 0.0);
    assert_eq!(summary.expectations.len(), 4);
    assert!(
        summary
            .expectations
            .iter()
            .any(|r| r.pattern == ExpectationPattern::OnlyWithSkill)
    );
    assert!(
        summary
            .notes
            .iter()
            .any(|n| n.contains("× the tokens of without_skill")),
        "{:?}",
        summary.notes
    );
    let trigger = summary.trigger.as_ref().expect("trigger summary");
    assert_eq!((trigger.total, trigger.correct), (4, 4));
    assert_eq!(trigger.f1, 1.0);
    assert_eq!(summary.executor, "scripted");
    assert_eq!(summary.grader.as_deref(), Some("scripted"));

    assert!(Path::new(&run.workspace).join(BENCHMARK_MD).is_file());
    assert!(Path::new(&run.workspace).join("benchmark.json").is_file());
    let md = f.hive.benchmark_markdown(&run.id).unwrap();
    assert!(md.contains("# Benchmark: report-writer"));
    assert!(md.contains("| with_skill |"));
    assert!(md.contains("## Trigger"));

    let item = f.hive.get_item(&f.item.id).unwrap();
    let eff = item.effectiveness.expect("effectiveness summary");
    assert_eq!(eff.run_id, run.id);
    assert_eq!(eff.verdict, summary.verdict);
    assert_eq!(eff.with_skill_pass_rate, 1.0);
    assert_eq!(eff.delta, 1.0);
    assert_eq!(f.hive.stats().unwrap().tested, 1);

    let history = f.hive.history(&f.item.id).unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].run_id, run.id);
    assert_eq!(history[0].delta, 1.0);
    assert_eq!(f.hive.list_runs(Some(&f.item.id), 10).unwrap().len(), 1);
    assert_eq!(f.hive.list_runs(None, 10).unwrap()[0].id, run.id);

    let transcript = f
        .hive
        .read_case_transcript(&run.id, &with_case1.id)
        .unwrap();
    assert!(transcript.contains("\"type\":\"result\""));
    assert!(f.hive.read_case_transcript(&run.id, "nope").is_err());
    assert!(
        f.hive.cancel_run(&run.id).is_err(),
        "finished runs cannot be cancelled"
    );
    assert!(!f.hive.runs.is_active(&run.id));

    f.hive.delete_run(&run.id).unwrap();
    assert!(!Path::new(&run.workspace).exists());
    assert!(f.hive.run_detail(&run.id).is_err());
    assert!(f.hive.history(&f.item.id).unwrap().is_empty());
}

#[tokio::test]
async fn checks_only_when_no_grader_is_configured() {
    let f = fixture(&reference_script());
    let mut rx = f.hive.subscribe();
    let run = f.hive.start_run(config(&f, 1, false, false)).await.unwrap();
    assert_eq!(
        wait_for_terminal(&mut rx, &run.id, Duration::from_secs(60)).await,
        RunStatus::Done
    );
    let detail = f.hive.run_detail(&run.id).unwrap();
    assert_eq!(detail.results.len(), 4);
    for r in &detail.results {
        assert_eq!(r.status, CaseStatus::Executed);
        assert_eq!(r.total, 1, "only the checked expectation counts");
        assert!(
            r.grader_notes
                .as_deref()
                .unwrap()
                .starts_with(NO_GRADER_NOTE)
        );
        assert_eq!(
            r.pass_rate,
            if r.arm == WITH_SKILL_LABEL { 1.0 } else { 0.0 }
        );
    }
    let summary = detail.run.summary.unwrap();
    assert!(summary.trigger.is_none());
    assert!(summary.grader.is_none());
    // Two cases give two scored results per arm: enough for a verdict, but both arms have
    // zero variance so there is no p-value.
    assert!(
        matches!(summary.verdict, Verdict::Helps | Verdict::HelpsALot),
        "{:?}",
        summary.verdict
    );
    assert_eq!(summary.deltas[0].p_value, None);
    assert!(
        summary
            .notes
            .iter()
            .any(|n| n.contains("no grader configured")),
        "{:?}",
        summary.notes
    );
    assert!(
        summary.notes.iter().any(|n| n.contains("repeat the run")),
        "{:?}",
        summary.notes
    );
}

#[tokio::test]
async fn cancellation_marks_the_run_cancelled() {
    let script = serde_json::json!({"default": {"output": "late", "duration_ms": 5000}});
    let f = fixture(&script);
    let mut rx = f.hive.subscribe();
    let run = f.hive.start_run(config(&f, 1, false, false)).await.unwrap();
    wait_for_running(&mut rx, &run.id).await;
    assert!(f.hive.runs.is_active(&run.id));
    assert!(
        f.hive.delete_run(&run.id).is_err(),
        "in-flight runs cannot be deleted"
    );
    let started = Instant::now();
    f.hive.cancel_run(&run.id).unwrap();
    assert_eq!(
        wait_for_terminal(&mut rx, &run.id, Duration::from_secs(30)).await,
        RunStatus::Cancelled
    );
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "cancellation must not wait for the slow rule"
    );
    let detail = f.hive.run_detail(&run.id).unwrap();
    assert_eq!(detail.run.status, RunStatus::Cancelled);
    assert!(detail.run.finished_at.is_some());
    assert!(detail.run.summary.is_none());
    assert!(
        detail
            .results
            .iter()
            .all(|r| r.status == CaseStatus::Cancelled),
        "{:?}",
        detail.results
    );
    assert!(
        !detail.results.is_empty(),
        "in-flight cases are recorded as cancelled"
    );
    assert!(detail.results.len() < 4, "queued cases never start");
    assert!(f.hive.cancel_run(&run.id).is_err());
    assert!(f.hive.benchmark_markdown(&run.id).is_err());
    f.hive.delete_run(&run.id).unwrap();
}

#[tokio::test]
async fn executor_failures_are_recorded_and_the_run_finishes() {
    let script = serde_json::json!({
        "rules": [{"with_skill": true, "fail": true, "error": "simulated crash"}],
        "default": {"output": "plain answer", "duration_ms": 0}
    });
    let f = fixture(&script);
    let mut rx = f.hive.subscribe();
    let mut cfg = config(&f, 2, true, false);
    cfg.case_ids = vec![1];
    let run = f.hive.start_run(cfg).await.unwrap();
    assert_eq!(run.progress.total, 4);
    assert_eq!(
        wait_for_terminal(&mut rx, &run.id, Duration::from_secs(60)).await,
        RunStatus::Done
    );
    let detail = f.hive.run_detail(&run.id).unwrap();
    assert_eq!(detail.results.len(), 4);
    assert_eq!(detail.run.progress.failed, 2);
    for r in &detail.results {
        assert_eq!(r.case_id, 1);
        if r.arm == WITH_SKILL_LABEL {
            assert_eq!(r.status, CaseStatus::Failed);
            assert!(r.error.as_deref().unwrap().contains("simulated crash"));
            assert_eq!(r.total, 2, "planned expectations are recorded as failed");
            assert!(
                r.expectations
                    .iter()
                    .all(|e| !e.passed && e.evidence.contains("execution failed"))
            );
        } else {
            assert_eq!(r.status, CaseStatus::Graded);
        }
    }
    let summary = detail.run.summary.unwrap();
    let with = summary
        .arms
        .iter()
        .find(|a| a.label == WITH_SKILL_LABEL)
        .unwrap();
    assert_eq!(with.errors, 2);
    assert_eq!(with.pass_rate.mean, 0.0);
    assert!(
        summary
            .notes
            .iter()
            .any(|n| n.contains("failed or timed out in with_skill")),
        "{:?}",
        summary.notes
    );
    assert!(
        matches!(
            summary.verdict,
            Verdict::Neutral | Verdict::Hurts | Verdict::HurtsALot
        ),
        "{:?}",
        summary.verdict
    );
}

#[tokio::test]
async fn trigger_evaluation_uses_the_executor_when_there_is_no_grader() {
    let f = fixture(&reference_script());
    let mut rx = f.hive.subscribe();
    let mut cfg = config(&f, 1, false, true);
    cfg.case_ids = vec![2];
    let run = f.hive.start_run(cfg).await.unwrap();
    assert_eq!(
        wait_for_terminal(&mut rx, &run.id, Duration::from_secs(60)).await,
        RunStatus::Done
    );
    let detail = f.hive.run_detail(&run.id).unwrap();
    assert_eq!(detail.results.len(), 2);
    let trigger = detail
        .run
        .summary
        .unwrap()
        .trigger
        .expect("trigger summary");
    assert_eq!(trigger.total, 4);
    assert_eq!(trigger.correct, 4);
    assert!(trigger.results.iter().all(|r| r.correct));
    assert_eq!(trigger.results[0].chosen.as_deref(), Some("report-writer"));
    assert_eq!(trigger.results[2].chosen, None);
    assert!((trigger.precision - 1.0).abs() < 1e-9);
    assert!((trigger.recall - 1.0).abs() < 1e-9);
}

#[tokio::test]
async fn start_run_validates_its_input() {
    let f = fixture(&reference_script());
    let good = config(&f, 1, true, false);

    let mut bad = good.clone();
    bad.arms[0].label = "skill".into();
    assert!(f.hive.start_run(bad).await.is_err());

    let mut bad = good.clone();
    bad.arms.clear();
    assert!(f.hive.start_run(bad).await.is_err());

    let mut bad = good.clone();
    bad.executor_provider_id = "missing".into();
    assert!(f.hive.start_run(bad).await.is_err());

    let mut bad = good.clone();
    bad.grader_provider_id = Some("missing".into());
    assert!(f.hive.start_run(bad).await.is_err());

    let mut bad = good.clone();
    bad.case_ids = vec![99];
    assert!(f.hive.start_run(bad).await.is_err());

    let mut bad = good.clone();
    bad.item_id = "missing".into();
    assert!(f.hive.start_run(bad).await.is_err());

    // A skill without a suite cannot run.
    let bare = write_skill(
        &f.script_dir,
        "bare",
        "A skill without any evals for testing",
        "b",
    );
    let bare_item = f.hive.import_path(&bare, &[], false).unwrap();
    let mut bad = good.clone();
    bad.item_id = bare_item.id.clone();
    assert!(f.hive.start_run(bad).await.is_err());
    assert!(
        f.hive
            .default_run_config(&bare_item.id)
            .unwrap()
            .case_ids
            .is_empty()
    );
    assert!(
        !f.hive
            .default_run_config(&bare_item.id)
            .unwrap()
            .include_trigger
    );

    // Nothing was left behind.
    assert!(f.hive.list_runs(None, 10).unwrap().is_empty());
    assert!(f.hive.runs.active().is_empty());
    assert!(f.hive.cancel_run("missing").is_err());
}
