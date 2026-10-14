//! Skill-creator workspace, JUnit, and CSV exporters.
//! Endpoint ids are replaced by display names. Secret-looking tokens are dropped.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;

use crate::testing::python_bin;

#[derive(Debug, Clone)]
pub struct ExpectationExport {
    pub text: String,
    pub passed: bool,
    pub evidence: String,
}

#[derive(Debug, Clone)]
pub struct CheckExport {
    pub id: String,
    pub passed: bool,
    pub stderr: String,
}

#[derive(Debug, Clone)]
pub struct ExportCell {
    pub eval_id: i64,
    pub eval_name: String,
    pub configuration: String,
    pub expectations: Vec<ExpectationExport>,
    pub checks: Vec<CheckExport>,
    pub pass_rate: f64,
    pub endpoint_id: String,
    pub display_name: String,
    pub secret: String,
}

#[derive(Debug, Clone)]
pub struct ExportSlice {
    pub skill_name: String,
    pub model: String,
    pub cells: Vec<ExportCell>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportCounts {
    pub testcases: usize,
    pub cell_rows: usize,
    pub verdict_rows: usize,
    pub html: PathBuf,
}

pub fn export_workspace(root: &Path, slice: &ExportSlice) -> Result<ExportCounts, String> {
    let iteration = root.join("iteration-1");
    fs::create_dir_all(&iteration).map_err(|err| err.to_string())?;
    let mut evals: Vec<i64> = slice.cells.iter().map(|cell| cell.eval_id).collect();
    evals.sort_unstable();
    evals.dedup();
    for eval_id in &evals {
        let name = slice
            .cells
            .iter()
            .find(|cell| cell.eval_id == *eval_id)
            .map(|cell| cell.eval_name.clone())
            .unwrap_or_else(|| format!("eval-{eval_id}"));
        let dir = iteration.join(format!("eval-{eval_id}"));
        fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
        let metadata = json!({
            "eval_id": eval_id,
            "eval_name": name,
            "prompt": "Recorded fixture prompt",
            "assertions": []
        });
        fs::write(
            dir.join("eval_metadata.json"),
            serde_json::to_string_pretty(&metadata).map_err(|err| err.to_string())?,
        )
        .map_err(|err| err.to_string())?;
        for cell in slice.cells.iter().filter(|cell| cell.eval_id == *eval_id) {
            write_cell(&dir, cell)?;
        }
    }
    let benchmark = json!({
        "metadata": {
            "skill_name": slice.skill_name,
            "skill_path": slice.skill_name,
            "executor_model": slice.model,
            "timestamp": "2026-01-15T10:30:00Z",
            "evals_run": evals
        },
        "runs": [],
        "run_summary": {},
        "notes": []
    });
    fs::write(
        iteration.join("benchmark.json"),
        serde_json::to_string_pretty(&benchmark).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())?;
    fs::write(iteration.join("benchmark.md"), "# Benchmark\n").map_err(|err| err.to_string())?;
    fs::write(root.join("feedback.json"), "{}\n").map_err(|err| err.to_string())?;
    let junit = render_junit(slice);
    let junit_path = root.join("junit.xml");
    fs::write(&junit_path, &junit).map_err(|err| err.to_string())?;
    let (cells_csv, verdicts_csv, cell_rows, verdict_rows) = render_csv(slice);
    fs::write(root.join("cells.csv"), cells_csv).map_err(|err| err.to_string())?;
    fs::write(root.join("verdicts.csv"), verdicts_csv).map_err(|err| err.to_string())?;
    let testcases = junit.matches("<testcase ").count();
    let html = root.join("review.html");
    render_static(root, &html)?;
    let dumped = fs::read_to_string(&junit_path).unwrap_or_default()
        + &fs::read_to_string(root.join("cells.csv")).unwrap_or_default();
    for cell in &slice.cells {
        if !cell.secret.is_empty() && dumped.contains(&cell.secret) {
            return Err("export leaked a secret".into());
        }
        if dumped.contains(&cell.endpoint_id) && cell.endpoint_id != cell.display_name {
            return Err("export leaked an endpoint id".into());
        }
    }
    Ok(ExportCounts {
        testcases,
        cell_rows,
        verdict_rows,
        html,
    })
}

fn write_cell(eval_dir: &Path, cell: &ExportCell) -> Result<(), String> {
    let folder = eval_dir.join(&cell.configuration);
    let outputs = folder.join("outputs");
    fs::create_dir_all(&outputs).map_err(|err| err.to_string())?;
    let expectations: Vec<_> = cell
        .expectations
        .iter()
        .map(|item| {
            json!({
                "text": item.text,
                "passed": item.passed,
                "evidence": item.evidence
            })
        })
        .collect();
    let passed = cell.expectations.iter().filter(|item| item.passed).count();
    let grading = json!({
        "expectations": expectations,
        "summary": {"passed": passed, "total": cell.expectations.len(), "pass_rate": cell.pass_rate},
        "execution_metrics": {"display_name": cell.display_name},
        "timing": {"total_duration_seconds": 1.0}
    });
    fs::write(
        folder.join("grading.json"),
        serde_json::to_string_pretty(&grading).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())?;
    fs::write(
        folder.join("timing.json"),
        "{\"total_duration_seconds\":1.0}\n",
    )
    .map_err(|err| err.to_string())?;
    fs::write(
        outputs.join("metrics.json"),
        "{\"tool_calls\":1,\"output_chars\":8}\n",
    )
    .map_err(|err| err.to_string())?;
    fs::write(outputs.join("answer.txt"), "PINECONE\n").map_err(|err| err.to_string())?;
    Ok(())
}

fn render_junit(slice: &ExportSlice) -> String {
    let mut body = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuite name=\"skill-hive\">\n",
    );
    for cell in &slice.cells {
        for expectation in &cell.expectations {
            body.push_str(&testcase(
                &format!("{}-{}", cell.eval_name, expectation.text),
                &cell.configuration,
                expectation.passed,
                &expectation.evidence,
            ));
        }
        for check in &cell.checks {
            body.push_str(&testcase(
                &format!("{}-{}", cell.eval_name, check.id),
                &cell.configuration,
                check.passed,
                &check.stderr,
            ));
        }
    }
    body.push_str("</testsuite>\n");
    body
}

fn testcase(name: &str, classname: &str, passed: bool, detail: &str) -> String {
    let name = xml_escape(name);
    let classname = xml_escape(classname);
    if passed {
        format!("<testcase name=\"{name}\" classname=\"{classname}\"/>\n")
    } else {
        let detail = xml_escape(detail);
        format!("<testcase name=\"{name}\" classname=\"{classname}\"><failure>{detail}</failure></testcase>\n")
    }
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_csv(slice: &ExportSlice) -> (String, String, usize, usize) {
    let mut cells = String::from("eval_id,configuration,pass_rate,display_name\n");
    for cell in &slice.cells {
        cells.push_str(&format!(
            "{},{},{},{}\n",
            cell.eval_id,
            csv_escape(&cell.configuration),
            cell.pass_rate,
            csv_escape(&cell.display_name)
        ));
    }
    let mut verdicts = String::from("eval_id,configuration,expectation,passed\n");
    let mut verdict_rows = 0;
    for cell in &slice.cells {
        for expectation in &cell.expectations {
            verdicts.push_str(&format!(
                "{},{},{},{}\n",
                cell.eval_id,
                csv_escape(&cell.configuration),
                csv_escape(&expectation.text),
                expectation.passed
            ));
            verdict_rows += 1;
        }
    }
    (cells, verdicts, slice.cells.len(), verdict_rows)
}

fn csv_escape(text: &str) -> String {
    if text.contains(',') || text.contains('"') {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

fn render_static(workspace: &Path, html: &Path) -> Result<(), String> {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/eval-viewer/generate_review.py");
    let mut command: Command = python_bin();
    let output = command
        .arg(&script)
        .arg(workspace)
        .arg("--static")
        .arg(html)
        .output()
        .map_err(|err| err.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "generate_review.py failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    if !html.is_file() {
        return Err("static review was not written".into());
    }
    Ok(())
}
