//! End-to-end recordings of workflows D.1, D.2, and D.3.
//! Every model reply is a frozen fixture. No vendor API is called.

use std::fs;
use std::path::Path;

use serde_json::json;

use crate::authoring::versions::{VersionSource, VersionStore};
use crate::authoring::{validate_agent, validate_skill, Profile, Severity};
use crate::generation::{
    draft_expectations, draft_trigger_set, skill_envelope, skill_files, GenerationSession, Kind,
};

use super::compare::{record_verdict, start_analysis, start_comparison, Side};
use super::export::{export_workspace, ExportCell, ExportCounts, ExportSlice};
use super::history::{write_history, HistoryEntry};
use super::optimize::{apply_optimized, hit_stream, miss_stream, optimize, Candidate, QueryScore};
use super::regression::{
    evaluate, load_baseline, set_baseline, ExpectationOutcome, RunSlice, SampleCell,
};

#[derive(Debug, Clone)]
pub struct WorkflowReport {
    pub name: String,
    pub notes: Vec<String>,
    pub counts: Option<ExportCounts>,
    pub offer: Option<String>,
}

pub fn demonstrate_d1(root: &Path) -> Result<WorkflowReport, String> {
    let mut session = GenerationSession::start(
        root,
        None,
        Kind::Skill,
        "Extract line items from supplier invoice PDFs into a CSV",
        "invoice-extractor",
        Profile::Portable,
    )?;
    session
        .run_review(&skill_envelope("invoice-extractor", false))
        .map_err(|err| format!("{err:?}"))?;
    if session.findings_errors != 0 {
        return Err("draft still has validation errors".into());
    }
    let mut revised = skill_files("invoice-extractor", false);
    let skill = revised
        .get("SKILL.md")
        .cloned()
        .unwrap_or_default()
        .replace(".", ";");
    revised.insert("SKILL.md".into(), skill);
    revised.insert(
        ".studio/changes.md".into(),
        "- SKILL.md: use semicolons in the body\n".into(),
    );
    let reply = envelope_of(&revised);
    session
        .revise("use semicolons in the body", &reply)
        .map_err(|err| format!("{err:?}"))?;
    let saved = session.accept().map_err(|err| format!("{err:?}"))?;
    let sources = VersionStore::open(&saved)
        .versions()
        .map_err(|err| err.to_string())?;
    if !sources
        .iter()
        .any(|version| version.source == VersionSource::Generated)
    {
        return Err("accept did not record a generated version".into());
    }
    let outcome = optimize(
        &[passing_candidate(), failing_candidate()],
        "invoice",
        7,
        None,
    );
    if outcome.applied {
        return Err("optimizer applied a description without a click".into());
    }
    apply_optimized(&saved, &outcome.winner)?;
    let history = write_history(
        &root
            .join("invoice-extractor-workspace")
            .join("history.json"),
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
    )?;
    let baseline = sample_run("baseline-run", true);
    let baseline_dir = root.join("baselines").join("claude-cli");
    set_baseline(&baseline_dir, &baseline)?;
    let loaded = load_baseline(&baseline_dir).ok_or("baseline was not stored")?;
    let regressed = sample_run("regressed-run", false);
    let report = evaluate(Some(&loaded), &regressed);
    if !report.alerts.iter().any(|alert| alert.rule == "R2") {
        return Err("regressed recording did not raise R2".into());
    }
    let counts = export_workspace(&root.join("export-d1"), &demo_slice("invoice-extractor"))?;
    Ok(WorkflowReport {
        name: "D.1".into(),
        notes: vec![
            history["current_best"].as_str().unwrap_or("").to_string(),
            report
                .alerts
                .iter()
                .map(|alert| alert.rule.clone())
                .collect::<Vec<_>>()
                .join(","),
        ],
        counts: Some(counts),
        offer: None,
    })
}

pub fn demonstrate_d2(root: &Path) -> Result<WorkflowReport, String> {
    let skill = root.join("imported-skill");
    fs::create_dir_all(&skill).map_err(|err| err.to_string())?;
    fs::write(
        skill.join("SKILL.md"),
        "---\nname: imported-skill\ndescription: Clean CSV columns before a ledger import. Use when the user hands over a spreadsheet.\n---\n\nClean the columns listed in references/columns.md.\n",
    )
    .map_err(|err| err.to_string())?;
    fs::create_dir_all(skill.join("references")).map_err(|err| err.to_string())?;
    fs::write(
        skill.join("references").join("columns.md"),
        "vendor, date, total\n",
    )
    .map_err(|err| err.to_string())?;
    let expectations = draft_expectations(
        &json!({
            "skill_name": "imported-skill",
            "evals": [{
                "id": 1,
                "prompt": "Clean vendors.csv",
                "expected_output": "A cleaned CSV",
                "files": [],
                "expectations": ["The header row is vendor, date, total"]
            }]
        })
        .to_string(),
    )?;
    let triggers = draft_trigger_set(&recorded_triggers())?;
    fs::create_dir_all(skill.join("evals")).map_err(|err| err.to_string())?;
    fs::write(
        skill.join("evals").join("evals.json"),
        serde_json::to_string_pretty(&expectations).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())?;
    fs::write(
        skill.join("evals").join("trigger.json"),
        serde_json::to_string_pretty(&triggers).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())?;
    let findings = validate_skill(&skill, Profile::Portable);
    if findings
        .iter()
        .any(|finding| finding.severity == Severity::Error)
    {
        return Err(format!("drafted suite failed validation: {findings:?}"));
    }
    set_baseline(
        &root.join("baselines").join("hosted"),
        &sample_run("imported-baseline", true),
    )?;
    let counts = export_workspace(&root.join("export-d2"), &demo_slice("imported-skill"))?;
    let offer = files_changed_offer(true);
    Ok(WorkflowReport {
        name: "D.2".into(),
        notes: vec![format!("{} trigger queries", triggers.len())],
        counts: Some(counts),
        offer: Some(offer),
    })
}

pub fn demonstrate_d3(root: &Path) -> Result<WorkflowReport, String> {
    let mut session = GenerationSession::start(
        root,
        None,
        Kind::Agent,
        "Review a pull request diff for security problems",
        "security-reviewer",
        Profile::Portable,
    )?;
    session
        .run_review(&crate::generation::agent_envelope("security-reviewer"))
        .map_err(|err| format!("{err:?}"))?;
    let saved = session.accept().map_err(|err| format!("{err:?}"))?;
    let findings = validate_agent(&saved.join("agent.md"), false);
    if findings
        .iter()
        .any(|finding| finding.severity == Severity::Error)
    {
        return Err(format!("generated agent has errors: {findings:?}"));
    }
    let left = Side {
        id: "cell-left".into(),
        label: "with_skill".into(),
        version: "v2-optimized".into(),
        endpoint: "claude-cli".into(),
        path: saved.display().to_string(),
        output: "Findings list the seeded vulnerability. with_skill v2-optimized claude-cli".into(),
    };
    let right = Side {
        id: "cell-right".into(),
        label: "without_skill".into(),
        version: "v1".into(),
        endpoint: "claude-cli".into(),
        path: root.display().to_string(),
        output: "No findings. without_skill claude-cli".into(),
    };
    let mut comparison = start_comparison(None, "cmp-d3", &left, &right, 2)?;
    if comparison.unblinded
        || comparison.request.contains("with_skill")
        || comparison.request.contains("claude-cli")
    {
        return Err("comparison request leaked a side".into());
    }
    record_verdict(&mut comparison, "Output A");
    let suggestion = start_analysis(&comparison);
    session
        .revise(
            &suggestion,
            &crate::generation::agent_envelope("security-reviewer"),
        )
        .map_err(|err| format!("{err:?}"))?;
    write_history(
        &root
            .join("security-reviewer-workspace")
            .join("history.json"),
        "security-reviewer",
        &[HistoryEntry {
            version: "v1".into(),
            parent: None,
            expectation_pass_rate: 1.0,
            grading_result: "baseline".into(),
            is_current_best: true,
        }],
    )?;
    let counts = export_workspace(&root.join("export-d3"), &demo_slice("security-reviewer"))?;
    Ok(WorkflowReport {
        name: "D.3".into(),
        notes: vec![suggestion, comparison.left_name],
        counts: Some(counts),
        offer: None,
    })
}

pub fn files_changed_offer(changed: bool) -> String {
    if changed {
        "Run baseline matrix now?".into()
    } else {
        String::new()
    }
}

fn envelope_of(files: &std::collections::BTreeMap<String, String>) -> String {
    let mut reply = String::new();
    for (path, body) in files {
        reply.push_str(&format!("===== FILE: {path} =====\n{body}"));
        if !body.ends_with('\n') {
            reply.push('\n');
        }
    }
    reply.push_str("===== END =====\n");
    reply
}

fn recorded_triggers() -> String {
    let mut rows = Vec::new();
    for index in 0..16 {
        rows.push(json!({
            "query": format!("recorded query {index} about invoices or a nearby miss"),
            "should_trigger": index < 8
        }));
    }
    json!(rows).to_string()
}

fn passing_candidate() -> Candidate {
    Candidate {
        description: "Extract supplier invoice line items into a CSV.".into(),
        queries: (0..5)
            .map(|index| QueryScore {
                query: format!("query {index}"),
                should_trigger: true,
                stream: hit_stream("invoice"),
            })
            .collect(),
    }
}

fn failing_candidate() -> Candidate {
    Candidate {
        description: "A vague helper.".into(),
        queries: (0..5)
            .map(|index| QueryScore {
                query: format!("query {index}"),
                should_trigger: true,
                stream: miss_stream(),
            })
            .collect(),
    }
}

fn sample_run(run_id: &str, healthy: bool) -> RunSlice {
    let passed = healthy;
    RunSlice {
        run_id: run_id.into(),
        endpoint_model: "claude-cli/fixture-model".into(),
        cells: vec![SampleCell {
            id: format!("{run_id}-cell"),
            eval_id: "1".into(),
            configuration: "with_skill".into(),
            pass_rate: if healthy { 1.0 } else { 0.0 },
            expectations: vec![ExpectationOutcome {
                text: "The CSV has the columns vendor, date and total".into(),
                passed,
            }],
            trigger_should: Some(true),
            trigger_rate: Some(if healthy { 1.0 } else { 0.0 }),
            checks_ok: healthy,
            tokens: 100.0,
            cost: 0.01,
            time_seconds: 1.0,
        }],
    }
}

fn demo_slice(skill_name: &str) -> ExportSlice {
    ExportSlice {
        skill_name: skill_name.into(),
        model: "fixture-model".into(),
        cells: vec![
            demo_cell(1, "with_skill", true),
            demo_cell(1, "without_skill", false),
        ],
    }
}

fn demo_cell(eval_id: i64, configuration: &str, passed: bool) -> ExportCell {
    use super::export::{CheckExport, ExpectationExport};
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
