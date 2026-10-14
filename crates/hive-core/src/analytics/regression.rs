//! Regression rules R1 to R6 for one endpoint-model against a pinned baseline.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExpectationOutcome {
    pub text: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleCell {
    pub id: String,
    pub eval_id: String,
    pub configuration: String,
    pub pass_rate: f64,
    pub expectations: Vec<ExpectationOutcome>,
    pub trigger_should: Option<bool>,
    pub trigger_rate: Option<f64>,
    pub checks_ok: bool,
    pub tokens: f64,
    pub cost: f64,
    pub time_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSlice {
    pub run_id: String,
    pub endpoint_model: String,
    pub cells: Vec<SampleCell>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Alert {
    pub rule: String,
    pub severity: String,
    pub message: String,
    pub eval_id: String,
    pub cells: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RegressionReport {
    pub alerts: Vec<Alert>,
    pub flaky: Vec<String>,
    pub offer_set_baseline: bool,
}

pub fn set_baseline(dir: &Path, run: &RunSlice) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    let bytes = serde_json::to_vec_pretty(run).map_err(|err| err.to_string())?;
    fs::write(dir.join("baseline.json"), bytes).map_err(|err| err.to_string())
}

pub fn load_baseline(dir: &Path) -> Option<RunSlice> {
    let text = fs::read_to_string(dir.join("baseline.json")).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn evaluate(baseline: Option<&RunSlice>, current: &RunSlice) -> RegressionReport {
    let Some(baseline) = baseline else {
        return RegressionReport {
            alerts: Vec::new(),
            flaky: Vec::new(),
            offer_set_baseline: true,
        };
    };
    if baseline.endpoint_model != current.endpoint_model {
        return RegressionReport {
            alerts: Vec::new(),
            flaky: Vec::new(),
            offer_set_baseline: true,
        };
    }
    let mut alerts = Vec::new();
    alerts.extend(rule_pass_rate(baseline, current));
    let (expectation_alerts, flaky) = rule_expectations(baseline, current);
    alerts.extend(expectation_alerts);
    alerts.extend(rule_trigger(baseline, current));
    alerts.extend(rule_checks(baseline, current));
    alerts.extend(rule_cost(baseline, current));
    RegressionReport {
        alerts,
        flaky,
        offer_set_baseline: false,
    }
}

fn with_skill(run: &RunSlice) -> Vec<&SampleCell> {
    run.cells
        .iter()
        .filter(|cell| cell.configuration == "with_skill")
        .collect()
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn stddev(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let avg = mean(values);
    let variance = values
        .iter()
        .map(|value| (value - avg).powi(2))
        .sum::<f64>()
        / values.len() as f64;
    variance.sqrt()
}

fn rule_pass_rate(baseline: &RunSlice, current: &RunSlice) -> Vec<Alert> {
    let base_rates: Vec<f64> = with_skill(baseline)
        .iter()
        .map(|cell| cell.pass_rate)
        .collect();
    let new_rates: Vec<f64> = with_skill(current)
        .iter()
        .map(|cell| cell.pass_rate)
        .collect();
    let drop = mean(&base_rates) - mean(&new_rates);
    let limit = 0.10_f64.max(1.5 * stddev(&base_rates));
    if drop > limit {
        vec![Alert {
            rule: "R1".into(),
            severity: "error".into(),
            message: format!("pass rate dropped by {drop:.2}, above {limit:.2}"),
            eval_id: current
                .cells
                .first()
                .map(|cell| cell.eval_id.clone())
                .unwrap_or_default(),
            cells: current.cells.iter().map(|cell| cell.id.clone()).collect(),
        }]
    } else {
        Vec::new()
    }
}

fn expectation_rate(cells: &[&SampleCell], text: &str) -> Option<f64> {
    let mut seen = 0.0;
    let mut passed = 0.0;
    for cell in cells {
        for expectation in &cell.expectations {
            if expectation.text == text {
                seen += 1.0;
                if expectation.passed {
                    passed += 1.0;
                }
            }
        }
    }
    if seen == 0.0 {
        None
    } else {
        Some(passed / seen)
    }
}

fn texts(run: &RunSlice) -> Vec<String> {
    let mut names = Vec::new();
    for cell in &run.cells {
        for expectation in &cell.expectations {
            if !names.iter().any(|item| item == &expectation.text) {
                names.push(expectation.text.clone());
            }
        }
    }
    names
}

fn rule_expectations(baseline: &RunSlice, current: &RunSlice) -> (Vec<Alert>, Vec<String>) {
    let base_cells = with_skill(baseline);
    let new_cells = with_skill(current);
    let mut alerts = Vec::new();
    let mut flaky = Vec::new();
    for text in texts(current) {
        let Some(base_rate) = expectation_rate(&base_cells, &text) else {
            continue;
        };
        let Some(new_rate) = expectation_rate(&new_cells, &text) else {
            continue;
        };
        let base_open = base_rate > 0.0 && base_rate < 1.0;
        let new_open = new_rate > 0.0 && new_rate < 1.0;
        if base_open && new_open {
            flaky.push(text);
            continue;
        }
        if base_rate == 1.0 && new_rate == 0.0 {
            let cells: Vec<String> = new_cells.iter().map(|cell| cell.id.clone()).collect();
            alerts.push(Alert {
                rule: "R2".into(),
                severity: "error".into(),
                message: format!(
                    "{text} passed every baseline repetition and failed every new repetition"
                ),
                eval_id: new_cells
                    .first()
                    .map(|cell| cell.eval_id.clone())
                    .unwrap_or_default(),
                cells,
            });
        }
    }
    (alerts, flaky)
}

fn trigger_mean(cells: &[&SampleCell], should: bool) -> Option<f64> {
    let rates: Vec<f64> = cells
        .iter()
        .filter(|cell| cell.trigger_should == Some(should))
        .filter_map(|cell| cell.trigger_rate)
        .collect();
    if rates.is_empty() {
        None
    } else {
        Some(mean(&rates))
    }
}

fn rule_trigger(baseline: &RunSlice, current: &RunSlice) -> Vec<Alert> {
    let mut alerts = Vec::new();
    let base_cells = with_skill(baseline);
    let new_cells = with_skill(current);
    if let (Some(before), Some(after)) = (
        trigger_mean(&base_cells, true),
        trigger_mean(&new_cells, true),
    ) {
        if before == 1.0 && after < 0.5 {
            alerts.push(Alert {
                rule: "R3".into(),
                severity: "error".into(),
                message: "a should-trigger query passed the baseline and now fails".into(),
                eval_id: String::new(),
                cells: new_cells.iter().map(|cell| cell.id.clone()).collect(),
            });
        }
    }
    if let (Some(before), Some(after)) = (
        trigger_mean(&base_cells, false),
        trigger_mean(&new_cells, false),
    ) {
        if before < 0.5 && after >= 0.5 {
            alerts.push(Alert {
                rule: "R3".into(),
                severity: "error".into(),
                message: "a should-not-trigger query stayed quiet at baseline and now fires".into(),
                eval_id: String::new(),
                cells: new_cells.iter().map(|cell| cell.id.clone()).collect(),
            });
        }
    }
    alerts
}

fn rule_checks(baseline: &RunSlice, current: &RunSlice) -> Vec<Alert> {
    let base_ok = with_skill(baseline).iter().all(|cell| cell.checks_ok);
    let failing: Vec<String> = with_skill(current)
        .iter()
        .filter(|cell| !cell.checks_ok)
        .map(|cell| cell.id.clone())
        .collect();
    if base_ok && !failing.is_empty() {
        vec![Alert {
            rule: "R4".into(),
            severity: "error".into(),
            message: "a deterministic check passed every baseline cell and failed a new cell"
                .into(),
            eval_id: String::new(),
            cells: failing,
        }]
    } else {
        Vec::new()
    }
}

fn rule_cost(baseline: &RunSlice, current: &RunSlice) -> Vec<Alert> {
    let mut alerts = Vec::new();
    type Metric = (&'static str, fn(&SampleCell) -> f64);
    let metrics: [Metric; 3] = [
        ("tokens", |cell| cell.tokens),
        ("cost", |cell| cell.cost),
        ("time", |cell| cell.time_seconds),
    ];
    for (name, pick) in metrics {
        let before = mean(
            &with_skill(baseline)
                .iter()
                .map(|cell| pick(cell))
                .collect::<Vec<_>>(),
        );
        let after = mean(
            &with_skill(current)
                .iter()
                .map(|cell| pick(cell))
                .collect::<Vec<_>>(),
        );
        if before > 0.0 && after > before * 1.25 {
            alerts.push(Alert {
                rule: "R5".into(),
                severity: "warning".into(),
                message: format!("{name} exceeded the baseline by more than 25%"),
                eval_id: String::new(),
                cells: Vec::new(),
            });
        }
    }
    alerts
}
