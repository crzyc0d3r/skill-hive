//! LLM-as-judge for expectations that no deterministic check decides.
//!
//! The grader sees the task prompt, the expectations, the final output, a truncated transcript
//! and the files the run created (with the content of small text files), and must answer with
//! one JSON object. The burden of proof is on the expectation and there is no partial credit.
//! Parsing is forgiving (code fences, prose around the object); a reply without usable JSON is
//! retried once with a "return only JSON" nudge, and a second failure marks every graded
//! expectation failed with the evidence [`NO_JSON_EVIDENCE`].

use std::path::Path;

use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::error::{HiveError, Result};
use crate::model::{Check, EvalCase, ExpectationResult, ExpectationSource};
use crate::providers::{CompletionRequest, Provider};
use crate::util;

/// Characters of transcript shown to the grader.
pub const TRANSCRIPT_CAP: usize = 12_000;
/// Total characters of file content shown to the grader.
pub const FILES_CAP: usize = 8_000;
/// Characters shown per file.
pub const FILE_CONTENT_CAP: usize = 2_000;
/// Files larger than this are listed but not quoted.
pub const SMALL_FILE_BYTES: u64 = 64 * 1024;
/// Characters of the final output shown to the grader.
pub const OUTPUT_CAP: usize = 12_000;
/// Evidence used when the grader never produced parsable JSON.
pub const NO_JSON_EVIDENCE: &str = "grader returned no parsable JSON";
/// Prefix of the `grader_notes` written when no grader provider is configured.
pub const NO_GRADER_NOTE: &str = "no grader configured";

/// A created file as presented to the grader.
#[derive(Debug, Clone)]
pub struct GradedFile {
    pub path: String,
    pub size_bytes: u64,
    /// Content for small text files (already capped), `None` for binary or large files.
    pub content: Option<String>,
}

/// Everything the grader needs for one case.
#[derive(Debug, Clone)]
pub struct GradeInput<'a> {
    pub task_prompt: &'a str,
    pub expectations: &'a [String],
    pub output: &'a str,
    pub transcript: Option<&'a str>,
    pub files: &'a [GradedFile],
}

/// What grading produced.
#[derive(Debug, Clone, Default)]
pub struct GradeOutcome {
    pub results: Vec<ExpectationResult>,
    pub notes: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    /// True when the first reply had to be retried.
    pub retried: bool,
}

fn lenient_bool<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<bool, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    Ok(match v {
        serde_json::Value::Bool(b) => b,
        serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
        serde_json::Value::String(s) => matches!(
            s.trim().to_lowercase().as_str(),
            "true" | "yes" | "pass" | "passed" | "1"
        ),
        _ => false,
    })
}

/// One expectation verdict as the grader returns it.
#[derive(Debug, Clone, Deserialize)]
pub struct GraderItem {
    #[serde(default)]
    pub text: String,
    #[serde(deserialize_with = "lenient_bool")]
    pub passed: bool,
    #[serde(default)]
    pub evidence: String,
}

/// The grader's whole answer.
#[derive(Debug, Clone, Deserialize)]
pub struct GraderAnswer {
    #[serde(default)]
    pub expectations: Vec<GraderItem>,
    #[serde(default)]
    pub notes: Option<String>,
}

/// The grader's standing instructions.
pub const SYSTEM_PROMPT: &str = "You are a strict grader for an automated evaluation of an AI agent. You receive a task prompt, a list of expectations, the agent's final output, an excerpt of its transcript and the files it created.\n\nRules:\n- The burden of proof is on the expectation: mark it passed only when the evidence in front of you clearly shows it is met. When in doubt, fail it.\n- Judge only what is visible. Do not assume work happened that the output, transcript or files do not show.\n- No partial credit: an expectation is either met or not.\n- Cite the specific evidence that decides each verdict: the file and line, or a short quote from the output or transcript.\n\nAnswer with a single JSON object and nothing else, in this exact shape:\n{\"expectations\":[{\"text\":\"<expectation verbatim>\",\"passed\":true,\"evidence\":\"<quoted evidence>\"}],\"notes\":\"<one or two sentences of overall observations>\"}\nInclude every expectation exactly once, in the given order. No markdown fences, no prose outside the JSON.";

/// Build the user prompt for one case.
pub fn user_prompt(input: &GradeInput<'_>) -> String {
    let mut s = String::new();
    s.push_str("## Task prompt\n\n");
    s.push_str(input.task_prompt.trim());
    s.push_str("\n\n## Expectations\n\n");
    for (i, e) in input.expectations.iter().enumerate() {
        s.push_str(&format!("{}. {}\n", i + 1, e.trim()));
    }
    s.push_str("\n## Final output\n\n");
    if input.output.trim().is_empty() {
        s.push_str("(empty)\n");
    } else {
        s.push_str(&util::truncate_chars(input.output.trim(), OUTPUT_CAP));
        s.push('\n');
    }
    s.push_str("\n## Transcript (excerpt)\n\n");
    match input.transcript {
        Some(t) if !t.trim().is_empty() => {
            s.push_str(t.trim());
            s.push('\n');
        }
        _ => s.push_str("(no transcript available)\n"),
    }
    s.push_str("\n## Files created\n\n");
    if input.files.is_empty() {
        s.push_str("(none)\n");
    } else {
        let mut budget = FILES_CAP;
        for f in input.files {
            s.push_str(&format!(
                "- {} ({})\n",
                f.path,
                util::human_bytes(f.size_bytes)
            ));
            if let Some(content) = &f.content {
                if budget == 0 {
                    continue;
                }
                let shown = util::truncate_chars(content, FILE_CONTENT_CAP.min(budget));
                budget = budget.saturating_sub(shown.chars().count());
                s.push_str("```\n");
                s.push_str(&shown);
                if !shown.ends_with('\n') {
                    s.push('\n');
                }
                s.push_str("```\n");
            }
        }
    }
    s.push_str("\nReturn the JSON object now.");
    s
}

/// Pull the first `{` … last `}` out of a reply, after dropping code fences.
pub fn extract_json(text: &str) -> Option<String> {
    let mut cleaned = String::with_capacity(text.len());
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            continue;
        }
        cleaned.push_str(line);
        cleaned.push('\n');
    }
    let start = cleaned.find('{')?;
    let end = cleaned.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(cleaned[start..=end].to_string())
}

/// Parse a grader reply, tolerating fences and surrounding prose.
pub fn parse_answer(text: &str) -> Option<GraderAnswer> {
    if let Some(candidate) = extract_json(text) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&candidate) {
            // Only an object that actually carries verdicts counts as an answer.
            if value.get("expectations").is_some() {
                if let Ok(answer) = serde_json::from_value::<GraderAnswer>(value) {
                    return Some(answer);
                }
            }
        }
    }
    // A bare array of items is accepted too.
    let trimmed = text.trim().trim_matches('`');
    if let (Some(s), Some(e)) = (trimmed.find('['), trimmed.rfind(']')) {
        if e > s {
            if let Ok(items) = serde_json::from_str::<Vec<GraderItem>>(&trimmed[s..=e]) {
                return Some(GraderAnswer {
                    expectations: items,
                    notes: None,
                });
            }
        }
    }
    None
}

/// Lowercase, collapse whitespace, strip surrounding punctuation: used to match expectation
/// texts between the suite, the checks and the grader's reply.
pub fn normalize(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
        .trim_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace())
        .to_string()
}

/// The expectation text a check decides (its `expectation`, else a description of the check).
pub fn check_text(check: &Check) -> String {
    super::checks::check_text(check)
}

/// Expectations of a case that no check decides: these go to the grader.
pub fn graded_expectations(case: &EvalCase) -> Vec<String> {
    let checked: Vec<String> = case
        .checks
        .iter()
        .filter_map(|c| c.expectation.as_deref())
        .map(normalize)
        .collect();
    case.expectations
        .iter()
        .filter(|e| !checked.contains(&normalize(e)))
        .cloned()
        .collect()
}

/// Map the grader's items onto the expectations: by normalized text, else by position when
/// the counts agree, else the expectation fails for lack of a verdict.
pub fn map_answer(expectations: &[String], answer: &GraderAnswer) -> Vec<ExpectationResult> {
    let same_len = answer.expectations.len() == expectations.len();
    expectations
        .iter()
        .enumerate()
        .map(|(i, text)| {
            let wanted = normalize(text);
            let item = answer
                .expectations
                .iter()
                .find(|it| normalize(&it.text) == wanted)
                .or_else(|| same_len.then(|| &answer.expectations[i]));
            match item {
                Some(it) => ExpectationResult {
                    text: text.clone(),
                    passed: it.passed,
                    evidence: if it.evidence.trim().is_empty() {
                        "(grader gave no evidence)".into()
                    } else {
                        it.evidence.trim().to_string()
                    },
                    source: ExpectationSource::Grader,
                },
                None => ExpectationResult {
                    text: text.clone(),
                    passed: false,
                    evidence: "grader did not address this expectation".into(),
                    source: ExpectationSource::Grader,
                },
            }
        })
        .collect()
}

fn all_failed(expectations: &[String], evidence: &str) -> Vec<ExpectationResult> {
    expectations
        .iter()
        .map(|text| ExpectationResult {
            text: text.clone(),
            passed: false,
            evidence: evidence.to_string(),
            source: ExpectationSource::Grader,
        })
        .collect()
}

/// Grade the expectations through `provider.complete()`.
pub async fn grade(
    provider: &dyn Provider,
    input: &GradeInput<'_>,
    cancel: &CancellationToken,
) -> Result<GradeOutcome> {
    if input.expectations.is_empty() {
        return Ok(GradeOutcome::default());
    }
    let prompt = user_prompt(input);
    let request = |text: String| {
        CompletionRequest::new(text)
            .with_system(SYSTEM_PROMPT)
            .with_cancel(cancel.clone())
    };
    let first = provider.complete(request(prompt.clone())).await?;
    let mut input_tokens = first.input_tokens;
    let mut output_tokens = first.output_tokens;
    if let Some(answer) = parse_answer(&first.text) {
        return Ok(GradeOutcome {
            results: map_answer(input.expectations, &answer),
            notes: answer.notes.filter(|n| !n.trim().is_empty()),
            input_tokens,
            output_tokens,
            retried: false,
        });
    }
    if cancel.is_cancelled() {
        return Err(HiveError::Cancelled);
    }
    let nudge = format!(
        "{prompt}\n\nYour previous reply was not valid JSON. Return only the JSON object described in the instructions, with no prose and no code fences."
    );
    let second = provider.complete(request(nudge)).await?;
    input_tokens += second.input_tokens;
    output_tokens += second.output_tokens;
    match parse_answer(&second.text) {
        Some(answer) => Ok(GradeOutcome {
            results: map_answer(input.expectations, &answer),
            notes: answer.notes.filter(|n| !n.trim().is_empty()),
            input_tokens,
            output_tokens,
            retried: true,
        }),
        None => Ok(GradeOutcome {
            results: all_failed(input.expectations, NO_JSON_EVIDENCE),
            notes: Some(format!(
                "{NO_JSON_EVIDENCE} after one retry; last reply started with {:?}",
                util::truncate_chars(second.text.trim(), 120)
            )),
            input_tokens,
            output_tokens,
            retried: true,
        }),
    }
}

/// Read a transcript for the grader: the head and the tail when it exceeds `cap` characters.
pub fn read_transcript_excerpt(path: &Path, cap: usize) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let count = text.chars().count();
    if count <= cap {
        return Some(text.to_string());
    }
    let head_len = cap / 3;
    let tail_len = cap - head_len;
    let head: String = text.chars().take(head_len).collect();
    let tail: String = text.chars().skip(count - tail_len).collect();
    Some(format!(
        "{head}\n… [{} characters omitted] …\n{tail}",
        count - head_len - tail_len
    ))
}

/// Describe created files for the grader, quoting small text files.
pub fn files_for_grader(sandbox: &Path, rel_paths: &[String]) -> Vec<GradedFile> {
    rel_paths
        .iter()
        .map(|rel| {
            let path = sandbox.join(rel);
            let size_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let content = if size_bytes <= SMALL_FILE_BYTES {
                std::fs::read(&path)
                    .ok()
                    .filter(|b| !b.contains(&0))
                    .and_then(|b| String::from_utf8(b).ok())
                    .map(|t| util::truncate_chars(&t, FILE_CONTENT_CAP))
            } else {
                None
            };
            GradedFile {
                path: rel.clone(),
                size_bytes,
                content,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CheckKind;
    use crate::providers::scripted::test_support::{config, write_script};
    use crate::providers::{Provider, ScriptedProvider};

    #[test]
    fn json_extraction_from_noisy_replies() {
        let fenced = "Sure! Here is the grade:\n```json\n{\"expectations\":[{\"text\":\"a\",\"passed\":true,\"evidence\":\"line 1\"}],\"notes\":\"fine\"}\n```\nLet me know.";
        let a = parse_answer(fenced).unwrap();
        assert_eq!(a.expectations.len(), 1);
        assert!(a.expectations[0].passed);
        assert_eq!(a.notes.as_deref(), Some("fine"));

        let lenient = r#"{"expectations":[{"text":"a","passed":"yes"},{"text":"b","passed":0}]}"#;
        let a = parse_answer(lenient).unwrap();
        assert!(a.expectations[0].passed);
        assert!(!a.expectations[1].passed);
        assert_eq!(a.expectations[0].evidence, "");

        let array = "[{\"text\":\"a\",\"passed\":false,\"evidence\":\"no\"}]";
        assert_eq!(parse_answer(array).unwrap().expectations.len(), 1);

        assert!(parse_answer("I cannot grade this.").is_none());
        assert!(parse_answer("{ not json }").is_none());
        assert_eq!(
            extract_json("x { \"a\": {\"b\": 1} } y").unwrap(),
            "{ \"a\": {\"b\": 1} }"
        );
    }

    #[test]
    fn mapping_by_text_then_index() {
        let expectations = vec!["The file exists".to_string(), "It is concise".to_string()];
        let answer = GraderAnswer {
            expectations: vec![
                GraderItem {
                    text: "it is concise.".into(),
                    passed: true,
                    evidence: "short".into(),
                },
                GraderItem {
                    text: "the file exists".into(),
                    passed: false,
                    evidence: "missing".into(),
                },
            ],
            notes: None,
        };
        let mapped = map_answer(&expectations, &answer);
        assert!(!mapped[0].passed);
        assert_eq!(mapped[0].evidence, "missing");
        assert!(mapped[1].passed);
        assert!(mapped.iter().all(|m| m.source == ExpectationSource::Grader));

        // Different texts, same count: by position.
        let answer = GraderAnswer {
            expectations: vec![
                GraderItem {
                    text: "1".into(),
                    passed: true,
                    evidence: String::new(),
                },
                GraderItem {
                    text: "2".into(),
                    passed: false,
                    evidence: "x".into(),
                },
            ],
            notes: None,
        };
        let mapped = map_answer(&expectations, &answer);
        assert!(mapped[0].passed);
        assert_eq!(mapped[0].evidence, "(grader gave no evidence)");
        assert!(!mapped[1].passed);

        // Missing verdicts fail.
        let answer = GraderAnswer {
            expectations: vec![],
            notes: None,
        };
        let mapped = map_answer(&expectations, &answer);
        assert!(mapped.iter().all(|m| !m.passed));
        assert!(mapped[0].evidence.contains("did not address"));
    }

    #[test]
    fn check_decided_expectations_are_not_graded() {
        let case = EvalCase {
            id: 1,
            name: "n".into(),
            prompt: "p".into(),
            expected_output: None,
            files: vec![],
            expectations: vec![
                "report.md exists".into(),
                "The report is accurate".into(),
                "Output mentions done".into(),
            ],
            checks: vec![
                Check {
                    id: "c1".into(),
                    expectation: Some("  Report.md exists ".into()),
                    kind: CheckKind::FileExists {
                        path: "report.md".into(),
                    },
                },
                Check {
                    id: "c2".into(),
                    expectation: None,
                    kind: CheckKind::OutputContains {
                        text: "done".into(),
                    },
                },
            ],
        };
        assert_eq!(
            graded_expectations(&case),
            vec!["The report is accurate", "Output mentions done"]
        );
        assert_eq!(check_text(&case.checks[1]), "output contains \"done\"");
    }

    #[test]
    fn prompt_lists_everything_within_caps() {
        let files = vec![
            GradedFile {
                path: "a.md".into(),
                size_bytes: 10,
                content: Some("# A\nhello\n".into()),
            },
            GradedFile {
                path: "big.bin".into(),
                size_bytes: 10_000_000,
                content: None,
            },
        ];
        let expectations = vec!["E1".to_string(), "E2".to_string()];
        let input = GradeInput {
            task_prompt: "Do X",
            expectations: &expectations,
            output: "Done X",
            transcript: Some("t1\nt2"),
            files: &files,
        };
        let p = user_prompt(&input);
        assert!(p.contains("1. E1"));
        assert!(p.contains("2. E2"));
        assert!(p.contains("Done X"));
        assert!(p.contains("t1\nt2"));
        assert!(p.contains("- a.md (10 B)"));
        assert!(p.contains("# A\nhello"));
        assert!(p.contains("- big.bin (9.5 MB)"));
        assert!(SYSTEM_PROMPT.contains("burden of proof"));

        let tmp = tempfile::tempdir().unwrap();
        let long = "x".repeat(30_000);
        std::fs::write(tmp.path().join("t.jsonl"), &long).unwrap();
        let excerpt = read_transcript_excerpt(&tmp.path().join("t.jsonl"), 12_000).unwrap();
        assert!(excerpt.chars().count() < 12_200);
        assert!(excerpt.contains("characters omitted"));
        std::fs::write(tmp.path().join("small.txt"), "hi").unwrap();
        std::fs::write(tmp.path().join("bin.dat"), [0u8, 1, 2]).unwrap();
        let files = files_for_grader(
            tmp.path(),
            &["small.txt".into(), "bin.dat".into(), "missing".into()],
        );
        assert_eq!(files[0].content.as_deref(), Some("hi"));
        assert!(files[1].content.is_none());
        assert_eq!(files[2].size_bytes, 0);
    }

    #[tokio::test]
    async fn grading_with_retry_and_failure_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let script = serde_json::json!({
            "rules": [
                {"match": "previous reply was not valid JSON", "output": "```json\n{\"expectations\":[{\"text\":\"E1\",\"passed\":true,\"evidence\":\"ok\"}],\"notes\":\"second try\"}\n```"},
                {"match": "NEVER", "output": "still prose"},
            ],
            "default": {"output": "Hmm, I think it passed."}
        });
        let path = write_script(tmp.path(), "g", &script);
        let provider = ScriptedProvider::new(config("g", &path)).unwrap();
        let expectations = vec!["E1".to_string()];
        let input = GradeInput {
            task_prompt: "task",
            expectations: &expectations,
            output: "out",
            transcript: None,
            files: &[],
        };
        let outcome = grade(&provider, &input, &CancellationToken::new())
            .await
            .unwrap();
        assert!(outcome.retried);
        assert!(outcome.results[0].passed);
        assert_eq!(outcome.notes.as_deref(), Some("second try"));
        assert!(outcome.input_tokens > 0);

        // Never JSON: all failed with the fixed evidence.
        let script = serde_json::json!({"default": {"output": "no json here"}});
        let path = write_script(tmp.path(), "g2", &script);
        let provider = ScriptedProvider::new(config("g2", &path)).unwrap();
        let outcome = grade(&provider, &input, &CancellationToken::new())
            .await
            .unwrap();
        assert!(outcome.retried);
        assert!(!outcome.results[0].passed);
        assert_eq!(outcome.results[0].evidence, NO_JSON_EVIDENCE);
        assert!(outcome.notes.unwrap().contains(NO_JSON_EVIDENCE));

        // Provider errors propagate; empty expectation lists skip the model.
        let script = serde_json::json!({"default": {"fail": true, "error": "down"}});
        let path = write_script(tmp.path(), "g3", &script);
        let provider = ScriptedProvider::new(config("g3", &path)).unwrap();
        assert!(
            grade(&provider, &input, &CancellationToken::new())
                .await
                .is_err()
        );
        let none = GradeInput {
            task_prompt: "t",
            expectations: &[],
            output: "",
            transcript: None,
            files: &[],
        };
        assert!(
            grade(&provider, &none, &CancellationToken::new())
                .await
                .unwrap()
                .results
                .is_empty()
        );
        assert!(provider.probe().await.ok);
    }
}
