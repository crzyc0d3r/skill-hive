use std::fs;

use rusqlite::Connection;

use super::compare::{record_verdict, start_comparison, Side};
use super::export::{export_workspace, CheckExport, ExpectationExport, ExportCell, ExportSlice};
use super::history::{write_history, HistoryEntry};
use super::optimize::{
    apply_optimized, hit_stream, miss_stream, optimize, split_indices, Candidate, QueryScore,
};
use super::regression::{evaluate, ExpectationOutcome, RunSlice, SampleCell};
use super::workflow::{demonstrate_d1, demonstrate_d2, demonstrate_d3};
use crate::authoring::versions::VersionSource;
use crate::authoring::VersionStore;
use crate::db::migrations::run_migrations;
use crate::generation::skill_files;
use crate::testing::python_bin;

fn queries(pass: bool) -> Vec<QueryScore> {
    (0..5)
        .map(|index| QueryScore {
            query: format!("q{index}"),
            should_trigger: true,
            stream: if pass {
                hit_stream("invoice")
            } else {
                miss_stream()
            },
        })
        .collect()
}

#[test]
fn held_out_winner_beats_a_failing_candidate_for_any_split() {
    let passing = Candidate {
        description: "precise invoice description".into(),
        queries: queries(true),
    };
    let failing = Candidate {
        description: "vague description".into(),
        queries: queries(false),
    };
    for seed in [1_u64, 2, 7, 99] {
        let (train, test) = split_indices(5, seed);
        assert!(!train.is_empty() && !test.is_empty(), "seed {seed}");
        let outcome = optimize(&[passing.clone(), failing.clone()], "invoice", seed, None);
        assert_eq!(outcome.winner, "precise invoice description");
        assert!((outcome.test_score - 1.0).abs() < f64::EPSILON);
        assert!(!outcome.applied);
        let cancelled = optimize(
            &[passing.clone(), failing.clone()],
            "invoice",
            seed,
            Some(1),
        );
        assert!(cancelled.cancelled);
        assert!(!cancelled.applied);
        assert_eq!(cancelled.iterations, 1);
    }
}

#[test]
fn apply_optimized_is_a_separate_click() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("invoice");
    fs::create_dir_all(&dir).unwrap();
    let mut files = skill_files("invoice", false);
    fs::write(dir.join("SKILL.md"), files.remove("SKILL.md").unwrap()).unwrap();
    let outcome = optimize(
        &[Candidate {
            description: "Optimized invoice description without angle brackets.".into(),
            queries: queries(true),
        }],
        "invoice",
        3,
        None,
    );
    assert!(!outcome.applied);
    let id = apply_optimized(&dir, &outcome.winner).unwrap();
    let text = fs::read_to_string(dir.join("SKILL.md")).unwrap();
    assert!(text.contains("Optimized invoice description"));
    let versions = VersionStore::open(&dir).versions().unwrap();
    assert!(versions
        .iter()
        .any(|version| version.id == id && version.source == VersionSource::Optimized));
}

#[test]
fn comparator_request_hides_every_side_revealing_string() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("hive.db");
    let conn = Connection::open(&db).unwrap();
    run_migrations(&conn).unwrap();
    let left = Side {
        id: "cell-with_skill-9".into(),
        label: "with_skill".into(),
        version: "v2-optimized".into(),
        endpoint: "claude-cli".into(),
        path: "/tmp/skills/invoice/runs/cell-9".into(),
        output:
            "cell-with_skill-9 with_skill v2-optimized claude-cli /tmp/skills/invoice/runs/cell-9"
                .into(),
    };
    let right = Side {
        id: "cell-without-3".into(),
        label: "without_skill".into(),
        version: "v1".into(),
        endpoint: "claude-cli".into(),
        path: "/tmp/skills/invoice/runs/cell-3".into(),
        output: "cell-without-3 without_skill v1 claude-cli /tmp/skills/invoice/runs/cell-3".into(),
    };
    let mut comparison = start_comparison(Some(&conn), "cmp-1", &left, &right, 1).unwrap();
    assert!(comparison.stored_before_request);
    let stored: String = conn
        .query_row(
            "SELECT assignment FROM comparisons WHERE id = 'cmp-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored, comparison.assignment);
    for secret in [
        "with_skill",
        "without_skill",
        "v2-optimized",
        "claude-cli",
        "cell-with_skill-9",
        "cell-without-3",
        "/tmp/skills/invoice/runs/cell-9",
        "/tmp/skills/invoice/runs/cell-3",
    ] {
        assert!(
            !comparison.request.contains(secret),
            "{secret} in {}",
            comparison.request
        );
    }
    assert!(comparison.request.contains("Output A"));
    assert!(comparison.request.contains("Output B"));
    assert!(!comparison.unblinded);
    record_verdict(&mut comparison, "Output A");
    assert!(comparison.unblinded);
    assert_eq!(comparison.verdict.as_deref(), Some("Output A"));
}

#[test]
fn history_keeps_a_single_current_best_and_a_parent_chain() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("history.json");
    let value = write_history(
        &path,
        "invoice-extractor",
        &[
            HistoryEntry {
                version: "v1".into(),
                parent: None,
                expectation_pass_rate: 0.92,
                grading_result: "baseline".into(),
                is_current_best: false,
            },
            HistoryEntry {
                version: "v2".into(),
                parent: Some("v1".into()),
                expectation_pass_rate: 0.97,
                grading_result: "won".into(),
                is_current_best: false,
            },
            HistoryEntry {
                version: "v2-optimized".into(),
                parent: Some("v2".into()),
                expectation_pass_rate: 0.97,
                grading_result: "tie".into(),
                is_current_best: true,
            },
        ],
    )
    .unwrap();
    let iterations = value["iterations"].as_array().unwrap();
    assert_eq!(iterations.len(), 3);
    assert_eq!(
        iterations
            .iter()
            .filter(|item| item["is_current_best"] == true)
            .count(),
        1
    );
    assert_eq!(value["current_best"], "v2-optimized");
    assert_eq!(iterations[2]["parent"], "v2");
    assert!(write_history(
        &path,
        "invoice-extractor",
        &[
            HistoryEntry {
                version: "v1".into(),
                parent: None,
                expectation_pass_rate: 1.0,
                grading_result: "baseline".into(),
                is_current_best: true,
            },
            HistoryEntry {
                version: "v2".into(),
                parent: Some("v1".into()),
                expectation_pass_rate: 1.0,
                grading_result: "tie".into(),
                is_current_best: true,
            },
        ],
    )
    .is_err());
}

#[test]
fn each_regression_rule_has_a_positive_and_a_negative_case() {
    let quiet = run_slice("base", 1.0, true, true, 1.0, true, 100.0);
    let small_drop = run_slice("small", 0.95, true, true, 1.0, true, 100.0);
    assert!(evaluate(Some(&quiet), &small_drop)
        .alerts
        .iter()
        .all(|alert| alert.rule != "R1"));
    let big_drop = run_slice("drop", 0.5, true, true, 1.0, true, 100.0);
    assert!(has_rule(&evaluate(Some(&quiet), &big_drop), "R1"));

    let still_passing = run_slice("same", 1.0, true, true, 1.0, true, 100.0);
    assert!(evaluate(Some(&quiet), &still_passing)
        .alerts
        .iter()
        .all(|alert| alert.rule != "R2"));
    let newly_failing = run_slice("fail", 0.0, false, true, 1.0, true, 100.0);
    let r2 = evaluate(Some(&quiet), &newly_failing);
    let alert = r2.alerts.iter().find(|item| item.rule == "R2").unwrap();
    assert!(alert.message.contains("The CSV has the columns"));
    assert!(alert.cells.iter().any(|cell| cell.contains("fail")));

    assert!(evaluate(Some(&quiet), &still_passing)
        .alerts
        .iter()
        .all(|alert| alert.rule != "R3"));
    let missed = run_slice("miss", 1.0, true, true, 0.0, true, 100.0);
    assert!(has_rule(&evaluate(Some(&quiet), &missed), "R3"));

    assert!(evaluate(Some(&quiet), &still_passing)
        .alerts
        .iter()
        .all(|alert| alert.rule != "R4"));
    let broken_check = run_slice("check", 1.0, true, true, 1.0, false, 100.0);
    assert!(has_rule(&evaluate(Some(&quiet), &broken_check), "R4"));

    assert!(evaluate(Some(&quiet), &still_passing)
        .alerts
        .iter()
        .all(|alert| alert.rule != "R5"));
    let costly = run_slice("cost", 1.0, true, true, 1.0, true, 200.0);
    let r5 = evaluate(Some(&quiet), &costly);
    assert!(r5
        .alerts
        .iter()
        .any(|alert| alert.rule == "R5" && alert.severity == "warning"));

    let mut flaky_base = quiet.clone();
    let mut flaky_new = quiet.clone();
    flaky_new.run_id = "flaky-new".into();
    for run in [&mut flaky_base, &mut flaky_new] {
        run.cells.push(sample(
            format!("{}-b", run.run_id),
            0.0,
            false,
            true,
            1.0,
            true,
            100.0,
        ));
    }
    let report = evaluate(Some(&flaky_base), &flaky_new);
    assert!(report.flaky.iter().any(|text| text.contains("columns")));
    assert!(report.alerts.iter().all(|alert| alert.rule != "R2"));

    let bare = evaluate(None, &quiet);
    assert!(bare.alerts.is_empty());
    assert!(bare.offer_set_baseline);
}

#[test]
fn static_viewer_junit_and_csv_match_the_slice() {
    let tmp = tempfile::tempdir().unwrap();
    let slice = ExportSlice {
        skill_name: "invoice-extractor".into(),
        model: "fixture-model".into(),
        cells: vec![
            export_cell(1, "with_skill", true),
            export_cell(1, "without_skill", false),
        ],
    };
    let counts = export_workspace(tmp.path(), &slice).unwrap();
    assert_eq!(counts.testcases, 6);
    assert_eq!(counts.cell_rows, 2);
    assert_eq!(counts.verdict_rows, 4);
    let junit = fs::read_to_string(tmp.path().join("junit.xml")).unwrap();
    assert_eq!(junit.matches("<testcase ").count(), 6);
    assert!(junit.contains("<failure>"));
    assert!(!junit.contains("sk-live-secret"));
    assert!(!junit.contains("endpoint-secret-id"));
    let cells = fs::read_to_string(tmp.path().join("cells.csv")).unwrap();
    let verdicts = fs::read_to_string(tmp.path().join("verdicts.csv")).unwrap();
    assert_eq!(cells.lines().count(), 3);
    assert_eq!(verdicts.lines().count(), 5);
    assert!(cells.contains("Fixture model"));
    let html = fs::read_to_string(&counts.html).unwrap();
    assert!(html.contains("eval-1"));
    let missing = tmp.path().join("empty");
    fs::create_dir_all(&missing).unwrap();
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/eval-viewer/generate_review.py");
    let output = python_bin()
        .arg(script)
        .arg(&missing)
        .arg("--static")
        .arg(missing.join("out.html"))
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn workflows_d1_d2_and_d3_run_on_recorded_fixtures() {
    let tmp = tempfile::tempdir().unwrap();
    let d1 = demonstrate_d1(&tmp.path().join("d1")).unwrap();
    assert_eq!(d1.notes[0], "v2-optimized");
    assert!(d1.notes[1].contains("R2"));
    assert_eq!(d1.counts.unwrap().testcases, 6);
    let d2 = demonstrate_d2(&tmp.path().join("d2")).unwrap();
    assert_eq!(d2.offer.as_deref(), Some("Run baseline matrix now?"));
    assert!(d2.notes[0].contains("16"));
    let d3 = demonstrate_d3(&tmp.path().join("d3")).unwrap();
    assert!(d3.notes[0].contains("Suggestion"));
    assert_eq!(d3.counts.unwrap().cell_rows, 2);
}

fn has_rule(report: &super::regression::RegressionReport, rule: &str) -> bool {
    report.alerts.iter().any(|alert| alert.rule == rule)
}

fn run_slice(
    id: &str,
    pass_rate: f64,
    expectation_passed: bool,
    should_trigger: bool,
    trigger_rate: f64,
    checks_ok: bool,
    tokens: f64,
) -> RunSlice {
    RunSlice {
        run_id: id.into(),
        endpoint_model: "claude-cli/fixture-model".into(),
        cells: vec![sample(
            format!("{id}-cell"),
            pass_rate,
            expectation_passed,
            should_trigger,
            trigger_rate,
            checks_ok,
            tokens,
        )],
    }
}

fn sample(
    id: String,
    pass_rate: f64,
    expectation_passed: bool,
    should_trigger: bool,
    trigger_rate: f64,
    checks_ok: bool,
    tokens: f64,
) -> SampleCell {
    SampleCell {
        id,
        eval_id: "1".into(),
        configuration: "with_skill".into(),
        pass_rate,
        expectations: vec![ExpectationOutcome {
            text: "The CSV has the columns vendor, date and total".into(),
            passed: expectation_passed,
        }],
        trigger_should: Some(should_trigger),
        trigger_rate: Some(trigger_rate),
        checks_ok,
        tokens,
        cost: tokens / 10_000.0,
        time_seconds: tokens / 100.0,
    }
}

fn export_cell(eval_id: i64, configuration: &str, passed: bool) -> ExportCell {
    ExportCell {
        eval_id,
        eval_name: format!("eval-{eval_id}"),
        configuration: configuration.into(),
        expectations: vec![
            ExpectationExport {
                text: "columns present".into(),
                passed,
                evidence: if passed {
                    String::new()
                } else {
                    "missing header".into()
                },
            },
            ExpectationExport {
                text: "script ran".into(),
                passed,
                evidence: if passed {
                    String::new()
                } else {
                    "script was skipped".into()
                },
            },
        ],
        checks: vec![CheckExport {
            id: "file-exists".into(),
            passed,
            stderr: if passed {
                String::new()
            } else {
                "outputs/answer.txt missing".into()
            },
        }],
        pass_rate: if passed { 1.0 } else { 0.0 },
        endpoint_id: "endpoint-secret-id".into(),
        display_name: "Fixture model".into(),
        secret: "sk-live-secret".into(),
    }
}
