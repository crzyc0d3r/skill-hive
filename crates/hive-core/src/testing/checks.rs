//! Deterministic checkers. A bound expectation never goes to the grader.

use std::fs;
use std::path::Path;

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct DeterministicCheck {
    pub id: String,
    pub expectation: String,
    pub kind: String,
    pub path: String,
    pub pattern: Option<String>,
    pub minimum: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    pub id: String,
    pub passed: bool,
    pub stdout: String,
    pub stderr: String,
}

pub fn run_check(
    check: &DeterministicCheck,
    cell_dir: &Path,
    transcript: &str,
    tool_calls: &Value,
) -> CheckResult {
    let target = cell_dir.join(&check.path);
    let (passed, stdout, stderr) = match check.kind.as_str() {
        "file_exists" => {
            let ok = target.is_file();
            (
                ok,
                format!("exists={ok} path={}", check.path),
                String::new(),
            )
        }
        "regex" => {
            let pattern = check.pattern.clone().unwrap_or_default();
            let text = fs::read_to_string(&target).unwrap_or_default();
            let ok = regex::Regex::new(&pattern)
                .map(|re| re.is_match(&text))
                .unwrap_or(false);
            (ok, format!("regex {pattern} matched={ok}"), String::new())
        }
        "json_path" => {
            let text = fs::read_to_string(&target).unwrap_or_default();
            let parsed: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
            let pointer = check.pattern.clone().unwrap_or_else(|| "".into());
            let ok = parsed.pointer(&pointer).is_some();
            (ok, format!("json {pointer} present={ok}"), String::new())
        }
        "csv_columns" => {
            let text = fs::read_to_string(&target).unwrap_or_default();
            let header = text.lines().next().unwrap_or("");
            let needed = check.pattern.clone().unwrap_or_default();
            let ok = needed
                .split(',')
                .all(|column| header.split(',').any(|have| have.trim() == column.trim()));
            (ok, format!("columns {needed} ok={ok}"), String::new())
        }
        "transcript_forbids" => {
            let needle = check.pattern.clone().unwrap_or_default();
            let ok = !transcript.contains(&needle);
            (ok, format!("forbidden {needle} absent={ok}"), String::new())
        }
        "tool_call_count" => {
            let total = tool_calls.get("total").and_then(Value::as_u64).unwrap_or(0);
            let minimum = check.minimum.unwrap_or(1);
            let ok = total >= minimum;
            (
                ok,
                format!("tool calls {total} >= {minimum}"),
                String::new(),
            )
        }
        other => (false, String::new(), format!("unknown check {other}")),
    };
    CheckResult {
        id: check.id.clone(),
        passed,
        stdout,
        stderr,
    }
}
