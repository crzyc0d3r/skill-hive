//! The eval suite stored inside a skill at `<skill>/evals/`.
//!
//! * `evals.json` uses the skill-creator format, so suites written by Anthropic's
//!   `skill-creator` work unchanged and suites written here work there:
//!   `{"skill_name": "...", "evals": [{"id": 1, "prompt": "...", "expected_output": "...",
//!   "files": ["evals/files/x.pdf"], "expectations": ["..."]}]}`. An optional `name` per eval is
//!   accepted and only written when it differs from the slug derived from the prompt.
//! * `checks.json` is a Hive extension holding deterministic checks keyed by eval id:
//!   `{"evals": {"1": [{"id": "c1-1", "expectation": "...", "kind": "file_exists", "path": "out.md"}]}}`.
//! * `trigger.json` is a Hive extension: `[{"query": "...", "shouldTrigger": true}]`.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{HiveError, IoContext, Result};
use crate::hive::Hive;
use crate::model::{AssetKind, Check, CheckKind, EvalCase, EvalSuite, HiveEvent, TriggerCase};
use crate::util;

pub const SUITE_DIR: &str = "evals";
pub const EVALS_FILE: &str = "evals.json";
pub const CHECKS_FILE: &str = "checks.json";
pub const TRIGGER_FILE: &str = "trigger.json";
/// Longest auto-derived case name.
pub const MAX_CASE_NAME_LEN: usize = 40;

/// `evals.json` as skill-creator writes it.
#[derive(Debug, Serialize, Deserialize)]
struct EvalsFile {
    skill_name: String,
    evals: Vec<EvalEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EvalEntry {
    id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    prompt: String,
    #[serde(default)]
    expected_output: String,
    #[serde(default)]
    files: Vec<String>,
    #[serde(default)]
    expectations: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ChecksFile {
    #[serde(default)]
    evals: BTreeMap<String, Vec<Check>>,
}

/// `<skill>/evals`.
pub fn suite_dir(skill_root: &Path) -> PathBuf {
    skill_root.join(SUITE_DIR)
}

/// A slug for a case derived from the first words of its prompt.
pub fn case_name_from_prompt(prompt: &str) -> String {
    let head: String = prompt
        .split_whitespace()
        .take(6)
        .collect::<Vec<_>>()
        .join(" ");
    let mut slug = util::slugify(&head);
    if slug.len() > MAX_CASE_NAME_LEN {
        slug.truncate(MAX_CASE_NAME_LEN);
        slug = slug.trim_end_matches('-').to_string();
    }
    if slug.is_empty() || slug == "item" {
        "case".to_string()
    } else {
        slug
    }
}

fn str_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// The eval's id: a number, a numeric string, or the 1-based position when absent. An
/// unusable id yields 0, which [`normalize_suite`] replaces with the next free id.
fn entry_id(entry: &Value, index: usize) -> u32 {
    match entry.get("id") {
        Some(Value::Number(n)) => n.as_u64().unwrap_or(0) as u32,
        Some(Value::String(s)) => s.trim().parse().unwrap_or(0),
        None => index as u32 + 1,
        Some(_) => 0,
    }
}

fn read_trigger_file(path: &Path) -> Result<Vec<TriggerCase>> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(path).at(path)?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|e| HiveError::invalid(format!("{TRIGGER_FILE} is not valid JSON: {e}")))?;
    let items = match value {
        Value::Array(a) => a,
        Value::Object(mut o) => o
            .remove("cases")
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    let mut out = Vec::new();
    for item in items {
        let query = item
            .get("query")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default()
            .to_string();
        if query.is_empty() {
            continue;
        }
        let should = item
            .get("shouldTrigger")
            .or_else(|| item.get("should_trigger"))
            .and_then(Value::as_bool)
            .unwrap_or(true);
        out.push(TriggerCase {
            query,
            should_trigger: should,
        });
    }
    Ok(out)
}

/// Read the suite of the skill at `skill_root`. A skill without `evals/evals.json` yields an
/// empty suite with `exists: false`.
pub fn read_suite(item_id: &str, skill_name: &str, skill_root: &Path) -> Result<EvalSuite> {
    let dir = suite_dir(skill_root);
    let mut suite = EvalSuite {
        item_id: item_id.to_string(),
        skill_name: skill_name.to_string(),
        cases: Vec::new(),
        trigger_cases: Vec::new(),
        path: dir.to_string_lossy().to_string(),
        exists: false,
    };
    let evals_path = dir.join(EVALS_FILE);
    if !evals_path.is_file() {
        return Ok(suite);
    }
    let text = std::fs::read_to_string(&evals_path).at(&evals_path)?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|e| HiveError::invalid(format!("{EVALS_FILE} is not valid JSON: {e}")))?;
    let entries = match &value {
        Value::Array(a) => a.clone(),
        Value::Object(o) => o
            .get("evals")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    let checks_path = dir.join(CHECKS_FILE);
    let mut checks: ChecksFile = if checks_path.is_file() {
        let text = std::fs::read_to_string(&checks_path).at(&checks_path)?;
        serde_json::from_str(&text)
            .map_err(|e| HiveError::invalid(format!("{CHECKS_FILE} is not valid: {e}")))?
    } else {
        ChecksFile::default()
    };
    for (index, entry) in entries.iter().enumerate() {
        let prompt = entry
            .get("prompt")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default()
            .to_string();
        if prompt.is_empty() {
            return Err(HiveError::invalid(format!(
                "{EVALS_FILE}: eval #{} has no prompt",
                index + 1
            )));
        }
        let id = entry_id(entry, index);
        let name = entry
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(util::slugify)
            .unwrap_or_else(|| case_name_from_prompt(&prompt));
        let expected_output = entry
            .get("expected_output")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        suite.cases.push(EvalCase {
            id,
            name,
            prompt,
            expected_output,
            files: str_list(entry.get("files")),
            expectations: str_list(entry.get("expectations")),
            checks: checks.evals.remove(&id.to_string()).unwrap_or_default(),
        });
    }
    suite.trigger_cases = read_trigger_file(&dir.join(TRIGGER_FILE))?;
    suite.exists = true;
    normalize_suite(&mut suite)?;
    Ok(suite)
}

fn validate_check(case_id: u32, check: &Check) -> Result<()> {
    let bad =
        |msg: String| HiveError::invalid(format!("case {case_id}, check `{}`: {msg}", check.id));
    match &check.kind {
        CheckKind::FileExists { path }
        | CheckKind::FileContains { path, .. }
        | CheckKind::FileMatches { path, .. }
        | CheckKind::JsonPointer { path, .. } => {
            if path.trim().is_empty() {
                return Err(bad("path is empty".into()));
            }
            if !util::is_safe_relative(path) {
                return Err(bad(format!("path `{path}` escapes the sandbox")));
            }
        }
        CheckKind::OutputContains { .. } | CheckKind::OutputMatches { .. } => {}
        CheckKind::Command { command, .. } => {
            if command.trim().is_empty() {
                return Err(bad("command is empty".into()));
            }
        }
    }
    match &check.kind {
        CheckKind::FileMatches { pattern, .. } | CheckKind::OutputMatches { pattern } => {
            regex::Regex::new(pattern)
                .map_err(|e| bad(format!("regex `{pattern}` is invalid: {e}")))?;
        }
        CheckKind::FileContains { text, .. } | CheckKind::OutputContains { text }
            if text.is_empty() =>
        {
            return Err(bad("text is empty".into()));
        }
        _ => {}
    }
    Ok(())
}

/// Assign ids and names, trim text, validate checks and paths. Idempotent.
pub fn normalize_suite(suite: &mut EvalSuite) -> Result<()> {
    let mut next_id = suite.cases.iter().map(|c| c.id).max().unwrap_or(0) + 1;
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    for case in &mut suite.cases {
        case.prompt = case.prompt.trim().to_string();
        if case.prompt.is_empty() {
            return Err(HiveError::invalid(format!(
                "case {} has an empty prompt",
                case.id
            )));
        }
        if case.id == 0 || !ids.insert(case.id) {
            case.id = next_id;
            next_id += 1;
            ids.insert(case.id);
        }
        let mut name = util::slugify(case.name.trim());
        if name == "item" || case.name.trim().is_empty() {
            name = case_name_from_prompt(&case.prompt);
        }
        if !names.insert(name.clone()) {
            name = format!("{name}-{}", case.id);
            names.insert(name.clone());
        }
        case.name = name;
        case.expected_output = case
            .expected_output
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let mut seen = HashSet::new();
        case.expectations = case
            .expectations
            .iter()
            .map(|e| e.trim().to_string())
            .filter(|e| !e.is_empty() && seen.insert(e.to_lowercase()))
            .collect();
        for file in &mut case.files {
            *file = file.trim().replace('\\', "/");
            if !util::is_safe_relative(file) {
                return Err(HiveError::invalid(format!(
                    "case {}: input file `{file}` escapes the skill directory",
                    case.id
                )));
            }
        }
        case.files.retain(|f| !f.is_empty());
        for (i, check) in case.checks.iter_mut().enumerate() {
            if check.id.trim().is_empty() {
                check.id = format!("c{}-{}", case.id, i + 1);
            }
            check.expectation = check
                .expectation
                .as_deref()
                .map(str::trim)
                .filter(|e| !e.is_empty())
                .map(str::to_string);
            validate_check(case.id, check)?;
        }
    }
    for t in &mut suite.trigger_cases {
        t.query = t.query.trim().to_string();
    }
    suite.trigger_cases.retain(|t| !t.query.is_empty());
    Ok(())
}

/// Write `evals.json`, `checks.json` and `trigger.json` under `<skill_root>/evals/`. Files
/// are written atomically; extension files are removed when they would be empty.
pub fn write_suite(skill_root: &Path, suite: &EvalSuite) -> Result<()> {
    let dir = suite_dir(skill_root);
    std::fs::create_dir_all(&dir).at(&dir)?;
    let evals = EvalsFile {
        skill_name: suite.skill_name.clone(),
        evals: suite
            .cases
            .iter()
            .map(|c| EvalEntry {
                id: c.id,
                name: (c.name != case_name_from_prompt(&c.prompt)).then(|| c.name.clone()),
                prompt: c.prompt.clone(),
                expected_output: c.expected_output.clone().unwrap_or_default(),
                files: c.files.clone(),
                expectations: c.expectations.clone(),
            })
            .collect(),
    };
    util::write_json_atomic(&dir.join(EVALS_FILE), &evals)?;

    let checks: BTreeMap<String, Vec<Check>> = suite
        .cases
        .iter()
        .filter(|c| !c.checks.is_empty())
        .map(|c| (c.id.to_string(), c.checks.clone()))
        .collect();
    let checks_path = dir.join(CHECKS_FILE);
    if checks.is_empty() {
        util::remove_path(&checks_path)?;
    } else {
        util::write_json_atomic(&checks_path, &ChecksFile { evals: checks })?;
    }

    let trigger_path = dir.join(TRIGGER_FILE);
    if suite.trigger_cases.is_empty() {
        util::remove_path(&trigger_path)?;
    } else {
        util::write_json_atomic(&trigger_path, &suite.trigger_cases)?;
    }
    Ok(())
}

fn first_sentence(description: &str) -> String {
    let s = description
        .split(['.', '!', '?', '\n'])
        .map(str::trim)
        .find(|s| !s.is_empty())
        .unwrap_or("do what the skill describes");
    s.trim_end_matches(',').to_string()
}

fn lower_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_lowercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// A starter suite built from the skill's description and trigger phrases, without any model
/// call: two or three task cases with checkable expectations and four trigger queries.
pub fn draft_suite(
    item_id: &str,
    skill_name: &str,
    description: &str,
    skill_root: &Path,
) -> EvalSuite {
    let summary = first_sentence(description);
    let phrases = crate::skill::trigger_phrases(description);
    let mut cases = vec![EvalCase {
        id: 1,
        name: "core-task".into(),
        prompt: format!(
            "I need you to do the following: {}. Work in the current directory, write the final result to `result.md` and finish with a short description of what you did.",
            lower_first(&summary)
        ),
        expected_output: None,
        files: Vec::new(),
        expectations: vec![
            "The final result is written to result.md".into(),
            "The work follows the procedure the skill describes".into(),
            "The response describes what was done".into(),
        ],
        checks: vec![Check {
            id: "c1-1".into(),
            expectation: Some("The final result is written to result.md".into()),
            kind: CheckKind::FileExists {
                path: "result.md".into(),
            },
        }],
    }];
    let mut phrase_iter = phrases.iter().take(2);
    if let Some(phrase) = phrase_iter.next() {
        cases.push(EvalCase {
            id: 2,
            name: case_name_from_prompt(phrase),
            prompt: format!(
                "I'm working on this: {phrase}. Walk me through it step by step and produce any artifacts that are needed in the current directory."
            ),
            expected_output: None,
            files: Vec::new(),
            expectations: vec![
                "The response gives concrete, ordered steps".into(),
                "The steps are consistent with the skill's instructions".into(),
            ],
            checks: Vec::new(),
        });
    } else {
        cases.push(EvalCase {
            id: 2,
            name: "approach-and-pitfalls".into(),
            prompt: format!(
                "Explain how you would approach this task and list the pitfalls to avoid: {}.",
                lower_first(&summary)
            ),
            expected_output: None,
            files: Vec::new(),
            expectations: vec![
                "The explanation covers the main steps".into(),
                "Common pitfalls are named".into(),
            ],
            checks: Vec::new(),
        });
    }
    if let Some(phrase) = phrase_iter.next() {
        cases.push(EvalCase {
            id: 3,
            name: case_name_from_prompt(phrase),
            prompt: format!("What is the best way to handle this: {phrase}? Be specific."),
            expected_output: None,
            files: Vec::new(),
            expectations: vec![
                "The answer is specific to the situation".into(),
                "The answer is consistent with the skill's instructions".into(),
            ],
            checks: Vec::new(),
        });
    }
    let mut trigger_cases: Vec<TriggerCase> = phrases
        .iter()
        .take(2)
        .map(|p| TriggerCase {
            query: format!("Can you help me {p}?"),
            should_trigger: true,
        })
        .collect();
    if trigger_cases.len() < 2 {
        trigger_cases.push(TriggerCase {
            query: format!("Please {}.", lower_first(&summary)),
            should_trigger: true,
        });
    }
    if trigger_cases.len() < 2 {
        trigger_cases.push(TriggerCase {
            query: format!("I need help with {skill_name}."),
            should_trigger: true,
        });
    }
    trigger_cases.push(TriggerCase {
        query: "What is the weather like in Paris today?".into(),
        should_trigger: false,
    });
    trigger_cases.push(TriggerCase {
        query: "Write a haiku about autumn leaves.".into(),
        should_trigger: false,
    });
    let mut suite = EvalSuite {
        item_id: item_id.to_string(),
        skill_name: skill_name.to_string(),
        cases,
        trigger_cases,
        path: suite_dir(skill_root).to_string_lossy().to_string(),
        exists: false,
    };
    let _ = normalize_suite(&mut suite);
    suite
}

impl Hive {
    /// The eval suite of a skill (an empty suite with `exists: false` when none is stored).
    pub fn eval_suite(&self, item_id: &str) -> Result<EvalSuite> {
        let item = self.get_item(item_id)?;
        if item.kind != AssetKind::Skill {
            return Err(HiveError::invalid(
                "evaluations are only supported for skills",
            ));
        }
        read_suite(&item.id, &item.name, Path::new(&item.path))
    }

    /// Write the suite into the skill directory, then refresh the library item (its content
    /// hash changed). Returns the suite as re-read from disk.
    pub fn save_eval_suite(&self, suite: &EvalSuite) -> Result<EvalSuite> {
        let item = self.get_item(&suite.item_id)?;
        if item.kind != AssetKind::Skill {
            return Err(HiveError::invalid(
                "evaluations are only supported for skills",
            ));
        }
        let mut suite = suite.clone();
        suite.skill_name = item.name.clone();
        normalize_suite(&mut suite)?;
        write_suite(Path::new(&item.path), &suite)?;
        self.library().refresh(&item.id)?;
        self.emit(HiveEvent::LibraryChanged);
        self.eval_suite(&item.id)
    }

    /// A starter suite derived from the skill's description (not written to disk).
    pub fn draft_eval_suite(&self, item_id: &str) -> Result<EvalSuite> {
        let item = self.get_item(item_id)?;
        if item.kind != AssetKind::Skill {
            return Err(HiveError::invalid(
                "evaluations are only supported for skills",
            ));
        }
        let doc = crate::skill::read_skill_dir(Path::new(&item.path))?;
        Ok(draft_suite(
            &item.id,
            &item.name,
            &doc.description,
            Path::new(&item.path),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::test_support::write_skill;

    fn sample_suite(item_id: &str, root: &Path) -> EvalSuite {
        EvalSuite {
            item_id: item_id.into(),
            skill_name: "pdf-tools".into(),
            cases: vec![
                EvalCase {
                    id: 1,
                    name: String::new(),
                    prompt: "Extract the tables from report.pdf into tables.csv".into(),
                    expected_output: Some("A CSV with every table".into()),
                    files: vec!["evals/files/report.pdf".into()],
                    expectations: vec![
                        "tables.csv exists".into(),
                        "Every table from the PDF is present".into(),
                    ],
                    checks: vec![Check {
                        id: String::new(),
                        expectation: Some("tables.csv exists".into()),
                        kind: CheckKind::FileExists {
                            path: "tables.csv".into(),
                        },
                    }],
                },
                EvalCase {
                    id: 0,
                    name: "custom-name".into(),
                    prompt: "Summarise the PDF".into(),
                    expected_output: None,
                    files: vec![],
                    expectations: vec!["The summary is short".into()],
                    checks: vec![],
                },
            ],
            trigger_cases: vec![
                TriggerCase {
                    query: "Read this PDF".into(),
                    should_trigger: true,
                },
                TriggerCase {
                    query: "Weather?".into(),
                    should_trigger: false,
                },
            ],
            path: suite_dir(root).to_string_lossy().to_string(),
            exists: false,
        }
    }

    #[test]
    fn round_trip_preserves_everything() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("pdf-tools");
        std::fs::create_dir_all(&root).unwrap();
        let suite = sample_suite("item-1", &root);
        let mut normalized = suite.clone();
        normalize_suite(&mut normalized).unwrap();
        write_suite(&root, &normalized).unwrap();
        let read = read_suite("item-1", "pdf-tools", &root).unwrap();
        assert!(read.exists);
        assert_eq!(read.cases.len(), 2);
        assert_eq!(read.cases[0].id, 1);
        assert_eq!(
            read.cases[0].name,
            "extract-the-tables-from-report-pdf-into"
        );
        assert_eq!(read.cases[0].prompt, suite.cases[0].prompt);
        assert_eq!(
            read.cases[0].expected_output.as_deref(),
            Some("A CSV with every table")
        );
        assert_eq!(read.cases[0].files, vec!["evals/files/report.pdf"]);
        assert_eq!(read.cases[0].expectations, suite.cases[0].expectations);
        assert_eq!(read.cases[0].checks.len(), 1);
        assert_eq!(read.cases[0].checks[0].id, "c1-1");
        assert_eq!(
            read.cases[0].checks[0].kind,
            CheckKind::FileExists {
                path: "tables.csv".into()
            }
        );
        assert_eq!(read.cases[1].id, 2, "id 0 gets the next free id");
        assert_eq!(read.cases[1].name, "custom-name");
        assert_eq!(read.cases[1].expected_output, None);
        assert_eq!(read.trigger_cases, suite.trigger_cases);
        // Writing the same suite again yields identical files (idempotent).
        let before = std::fs::read_to_string(root.join("evals/evals.json")).unwrap();
        write_suite(&root, &read).unwrap();
        let after = std::fs::read_to_string(root.join("evals/evals.json")).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn evals_json_is_skill_creator_compatible() {
        /// The skill-creator schema, strictly: nothing else may be required or present.
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct StrictEntry {
            id: u32,
            prompt: String,
            expected_output: String,
            files: Vec<String>,
            expectations: Vec<String>,
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct StrictFile {
            skill_name: String,
            evals: Vec<StrictEntry>,
        }
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("pdf-tools");
        std::fs::create_dir_all(&root).unwrap();
        let mut suite = sample_suite("item-1", &root);
        suite.cases[1].name = String::new(); // auto-derived names are not written
        normalize_suite(&mut suite).unwrap();
        write_suite(&root, &suite).unwrap();
        let text = std::fs::read_to_string(root.join("evals/evals.json")).unwrap();
        let strict: StrictFile = serde_json::from_str(&text).unwrap();
        assert_eq!(strict.skill_name, "pdf-tools");
        assert_eq!(strict.evals.len(), 2);
        assert_eq!(strict.evals[0].id, 1);
        assert_eq!(strict.evals[0].expected_output, "A CSV with every table");
        assert_eq!(strict.evals[1].expected_output, "");
        assert_eq!(strict.evals[0].files.len(), 1);
        assert_eq!(strict.evals[0].expectations.len(), 2);
        assert_eq!(strict.evals[0].prompt, suite.cases[0].prompt);

        // A file written by skill-creator (no name, no checks, no trigger) reads fine.
        let foreign = r#"{"skill_name": "x", "evals": [{"id": 7, "prompt": "Do the thing with the file", "expected_output": "", "files": [], "expectations": ["It works"]}]}"#;
        std::fs::create_dir_all(tmp.path().join("foreign/evals")).unwrap();
        std::fs::write(tmp.path().join("foreign/evals/evals.json"), foreign).unwrap();
        let read = read_suite("i", "x", &tmp.path().join("foreign")).unwrap();
        assert_eq!(read.cases[0].id, 7);
        assert_eq!(read.cases[0].name, "do-the-thing-with-the-file");
        assert!(read.cases[0].checks.is_empty());
        assert!(read.trigger_cases.is_empty());
    }

    #[test]
    fn missing_suite_and_validation_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let suite = read_suite("i", "x", tmp.path()).unwrap();
        assert!(!suite.exists);
        assert!(suite.cases.is_empty());
        assert!(suite.path.ends_with("evals"));

        let mut bad = sample_suite("i", tmp.path());
        bad.cases[0].checks[0].kind = CheckKind::FileExists {
            path: "../escape".into(),
        };
        assert!(normalize_suite(&mut bad).is_err());
        let mut bad = sample_suite("i", tmp.path());
        bad.cases[0].checks[0].kind = CheckKind::OutputMatches {
            pattern: "(".into(),
        };
        assert!(normalize_suite(&mut bad).is_err());
        let mut bad = sample_suite("i", tmp.path());
        bad.cases[0].prompt = "  ".into();
        assert!(normalize_suite(&mut bad).is_err());
        let mut bad = sample_suite("i", tmp.path());
        bad.cases[0].files = vec!["/etc/passwd".into()];
        assert!(normalize_suite(&mut bad).is_err());
    }

    #[test]
    fn case_names() {
        assert_eq!(
            case_name_from_prompt("Write a report for the Q3 numbers please"),
            "write-a-report-for-the-q3"
        );
        assert_eq!(case_name_from_prompt("!!!"), "case");
        assert!(
            case_name_from_prompt(&"averyveryverylongword ".repeat(5)).len() <= MAX_CASE_NAME_LEN
        );
    }

    #[test]
    fn draft_has_cases_and_trigger_queries() {
        let tmp = tempfile::tempdir().unwrap();
        let s = draft_suite(
            "i",
            "pdf-tools",
            "Extract text and tables from PDF files. Use when the user mentions a PDF or asks to read a document.",
            tmp.path(),
        );
        assert!(
            s.cases.len() >= 2 && s.cases.len() <= 3,
            "{}",
            s.cases.len()
        );
        assert!(s.cases[0].prompt.contains("extract text and tables"));
        assert_eq!(s.cases[0].checks.len(), 1);
        assert!(s.cases.iter().all(|c| !c.expectations.is_empty()));
        assert_eq!(
            s.trigger_cases.iter().filter(|t| t.should_trigger).count(),
            2
        );
        assert_eq!(
            s.trigger_cases.iter().filter(|t| !t.should_trigger).count(),
            2
        );
        assert!(!s.exists);
        let ids: HashSet<u32> = s.cases.iter().map(|c| c.id).collect();
        assert_eq!(ids.len(), s.cases.len());
        // No trigger phrases at all still yields two cases.
        let s = draft_suite("i", "x", "Tidy notes", tmp.path());
        assert_eq!(s.cases.len(), 2);
        assert_eq!(s.trigger_cases.len(), 4);
    }

    #[test]
    fn facade_reads_writes_and_refreshes_hash() {
        let tmp = tempfile::tempdir().unwrap();
        let hive = Hive::open_temp(tmp.path()).unwrap();
        let dir = write_skill(
            &tmp.path().join("src"),
            "pdf-tools",
            "Extract tables from PDF files when the user shares a PDF",
            "# Body",
        );
        let item = hive.import_path(&dir, &[], false).unwrap();
        let empty = hive.eval_suite(&item.id).unwrap();
        assert!(!empty.exists);
        let draft = hive.draft_eval_suite(&item.id).unwrap();
        assert!(!draft.cases.is_empty());
        let saved = hive.save_eval_suite(&draft).unwrap();
        assert!(saved.exists);
        assert_eq!(saved.cases.len(), draft.cases.len());
        assert!(Path::new(&item.path).join("evals/evals.json").is_file());
        assert!(Path::new(&item.path).join("evals/checks.json").is_file());
        assert!(Path::new(&item.path).join("evals/trigger.json").is_file());
        let refreshed = hive.get_item(&item.id).unwrap();
        assert_ne!(refreshed.content_hash, item.content_hash);
        // Clearing checks and trigger cases removes the extension files.
        let mut cleared = saved.clone();
        for c in &mut cleared.cases {
            c.checks.clear();
        }
        cleared.trigger_cases.clear();
        let again = hive.save_eval_suite(&cleared).unwrap();
        assert!(again.cases.iter().all(|c| c.checks.is_empty()));
        assert!(!Path::new(&item.path).join("evals/checks.json").exists());
        assert!(!Path::new(&item.path).join("evals/trigger.json").exists());
    }
}
