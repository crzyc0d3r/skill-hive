//! Trigger evaluation: does the skill's description make an agent pick it for the queries it
//! should handle, and leave it alone otherwise?
//!
//! The provider sees the available skills as `- <name>: <description>` (the skill under test
//! among up to [`MAX_OTHER_SKILLS`] other library skills) and the query, and must answer
//! `{"skill": "<name or null>"}`. Precision, recall and F1 are computed over `should_trigger`.

use tokio_util::sync::CancellationToken;

use crate::error::{HiveError, Result};
use crate::model::{TriggerCase, TriggerResult, TriggerSummary};
use crate::providers::{CompletionRequest, Provider};

/// Other library skills listed alongside the skill under test.
pub const MAX_OTHER_SKILLS: usize = 40;

/// The system prompt listing the skills. The skill under test is inserted at a position
/// derived from the query so it is not always first (position bias).
pub fn system_prompt(skill: (&str, &str), others: &[(String, String)], query: &str) -> String {
    let others: Vec<&(String, String)> = others
        .iter()
        .filter(|(n, _)| n != skill.0)
        .take(MAX_OTHER_SKILLS)
        .collect();
    let slot = query.chars().count() % (others.len() + 1);
    let mut lines = Vec::with_capacity(others.len() + 1);
    for (i, (name, description)) in others.iter().enumerate() {
        if i == slot {
            lines.push(format!("- {}: {}", skill.0, one_line(skill.1)));
        }
        lines.push(format!("- {name}: {}", one_line(description)));
    }
    if slot >= others.len() {
        lines.push(format!("- {}: {}", skill.0, one_line(skill.1)));
    }
    format!(
        "You are an AI coding assistant deciding which skill, if any, to load for a user request. A skill is a set of instructions you load only when the request matches its description.\n\nAvailable skills:\n{}\n\nReply with a single JSON object and nothing else: {{\"skill\": \"<skill name>\"}} naming the one skill whose description best matches the request, or {{\"skill\": null}} when no skill clearly applies. Do not invent skill names.",
        lines.join("\n")
    )
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parse the provider's reply into the chosen skill name (`None` for null / no choice).
/// Falls back to scanning the text for a known skill name when the JSON is unusable.
pub fn parse_choice(raw: &str, known: &[&str]) -> Option<String> {
    if let Some(json) = super::grader::extract_json(raw) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json) {
            return match v.get("skill") {
                Some(serde_json::Value::String(s)) => {
                    let s = s.trim();
                    if s.is_empty()
                        || s.eq_ignore_ascii_case("null")
                        || s.eq_ignore_ascii_case("none")
                    {
                        None
                    } else {
                        Some(s.to_string())
                    }
                }
                _ => None,
            };
        }
    }
    let lower = raw.to_lowercase();
    if lower.trim() == "null" || lower.trim() == "none" {
        return None;
    }
    known
        .iter()
        .filter(|k| !k.is_empty())
        .filter(|k| lower.contains(&k.to_lowercase()))
        .max_by_key(|k| k.len())
        .map(|k| k.to_string())
}

/// Precision/recall/F1 over the results. With no positive predictions precision is 1 when no
/// positive case was missed and 0 otherwise; recall likewise when there is no positive case.
pub fn summarize(results: Vec<TriggerResult>) -> TriggerSummary {
    let tp = results
        .iter()
        .filter(|r| r.should_trigger && r.triggered)
        .count() as f64;
    let fp = results
        .iter()
        .filter(|r| !r.should_trigger && r.triggered)
        .count() as f64;
    let fn_ = results
        .iter()
        .filter(|r| r.should_trigger && !r.triggered)
        .count() as f64;
    let precision = if tp + fp > 0.0 {
        tp / (tp + fp)
    } else if fn_ > 0.0 {
        0.0
    } else {
        1.0
    };
    let recall = if tp + fn_ > 0.0 {
        tp / (tp + fn_)
    } else if fp > 0.0 {
        0.0
    } else {
        1.0
    };
    let f1 = if precision + recall > 0.0 {
        2.0 * precision * recall / (precision + recall)
    } else {
        0.0
    };
    TriggerSummary {
        total: results.len() as u32,
        correct: results.iter().filter(|r| r.correct).count() as u32,
        precision,
        recall,
        f1,
        results,
    }
}

/// Run every trigger case through `provider.complete()`. A provider error on one query is
/// recorded in that result's `raw` (not triggered); cancellation stops the evaluation.
pub async fn evaluate(
    provider: &dyn Provider,
    skill_name: &str,
    skill_description: &str,
    others: &[(String, String)],
    cases: &[TriggerCase],
    cancel: &CancellationToken,
) -> Result<TriggerSummary> {
    let mut known: Vec<&str> = others.iter().map(|(n, _)| n.as_str()).collect();
    known.push(skill_name);
    let mut results = Vec::with_capacity(cases.len());
    for case in cases {
        if cancel.is_cancelled() {
            return Err(HiveError::Cancelled);
        }
        let system = system_prompt((skill_name, skill_description), others, &case.query);
        let req = CompletionRequest::new(case.query.clone())
            .with_system(system)
            .with_cancel(cancel.clone());
        let (raw, chosen) = match provider.complete(req).await {
            Ok(resp) => {
                let chosen = parse_choice(&resp.text, &known);
                (resp.text, chosen)
            }
            Err(HiveError::Cancelled) => return Err(HiveError::Cancelled),
            Err(e) => (format!("provider error: {e}"), None),
        };
        let triggered = chosen
            .as_deref()
            .map(|c| c.trim().eq_ignore_ascii_case(skill_name.trim()))
            .unwrap_or(false);
        results.push(TriggerResult {
            query: case.query.clone(),
            should_trigger: case.should_trigger,
            triggered,
            chosen,
            correct: triggered == case.should_trigger,
            raw,
        });
    }
    Ok(summarize(results))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::ScriptedProvider;
    use crate::providers::scripted::test_support::{config, write_script};

    #[test]
    fn prompt_lists_skills_and_moves_the_tested_one() {
        let others = vec![
            ("alpha".to_string(), "Does alpha\nthings".to_string()),
            ("beta".to_string(), "Does beta".to_string()),
        ];
        let p = system_prompt(("pdf-tools", "Reads PDFs"), &others, "q");
        assert!(p.contains("- alpha: Does alpha things"));
        assert!(p.contains("- pdf-tools: Reads PDFs"));
        assert!(p.contains("Available skills"));
        let positions: Vec<usize> = ["q", "qq", "qqq"]
            .iter()
            .map(|q| {
                system_prompt(("pdf-tools", "Reads PDFs"), &others, q)
                    .find("- pdf-tools")
                    .unwrap()
            })
            .collect();
        assert!(positions.iter().any(|p| *p != positions[0]));
        // The tested skill is never duplicated when it also appears in `others`.
        let with_dup = vec![("pdf-tools".to_string(), "x".to_string())];
        assert_eq!(
            system_prompt(("pdf-tools", "Reads PDFs"), &with_dup, "q")
                .matches("pdf-tools")
                .count(),
            1
        );
    }

    #[test]
    fn choice_parsing() {
        let known = ["pdf-tools", "alpha"];
        assert_eq!(
            parse_choice(r#"{"skill": "pdf-tools"}"#, &known).as_deref(),
            Some("pdf-tools")
        );
        assert_eq!(
            parse_choice("```json\n{\"skill\": null}\n```", &known),
            None
        );
        assert_eq!(parse_choice(r#"{"skill": "none"}"#, &known), None);
        assert_eq!(
            parse_choice("I would use the pdf-tools skill here.", &known).as_deref(),
            Some("pdf-tools")
        );
        assert_eq!(parse_choice("None of them.", &known), None);
        assert_eq!(parse_choice("null", &known), None);
    }

    #[test]
    fn precision_recall_f1() {
        let r = |should: bool, triggered: bool| TriggerResult {
            query: String::new(),
            should_trigger: should,
            triggered,
            chosen: None,
            correct: should == triggered,
            raw: String::new(),
        };
        let s = summarize(vec![
            r(true, true),
            r(true, false),
            r(false, true),
            r(false, false),
        ]);
        assert_eq!(s.total, 4);
        assert_eq!(s.correct, 2);
        assert!((s.precision - 0.5).abs() < 1e-9);
        assert!((s.recall - 0.5).abs() < 1e-9);
        assert!((s.f1 - 0.5).abs() < 1e-9);
        let s = summarize(vec![r(true, true), r(false, false)]);
        assert_eq!((s.precision, s.recall, s.f1), (1.0, 1.0, 1.0));
        let s = summarize(vec![r(false, false), r(false, false)]);
        assert_eq!((s.precision, s.recall), (1.0, 1.0));
        let s = summarize(vec![r(true, false)]);
        assert_eq!((s.precision, s.recall, s.f1), (0.0, 0.0, 0.0));
        let s = summarize(vec![]);
        assert_eq!(s.total, 0);
    }

    #[tokio::test]
    async fn evaluates_through_a_provider() {
        let tmp = tempfile::tempdir().unwrap();
        let script = serde_json::json!({
            "rules": [
                {"match": "(?i)pdf", "output": "{\"skill\": \"pdf-tools\"}"},
                {"match": "(?i)alpha", "output": "{\"skill\": \"alpha\"}"},
                {"match": "boom", "fail": true}
            ],
            "default": {"output": "{\"skill\": null}"}
        });
        let path = write_script(tmp.path(), "t", &script);
        let provider = ScriptedProvider::new(config("t", &path)).unwrap();
        let cases = vec![
            TriggerCase {
                query: "Read this PDF for me".into(),
                should_trigger: true,
            },
            TriggerCase {
                query: "Summarise the pdf".into(),
                should_trigger: true,
            },
            TriggerCase {
                query: "Do alpha things".into(),
                should_trigger: false,
            },
            TriggerCase {
                query: "Weather?".into(),
                should_trigger: false,
            },
            TriggerCase {
                query: "boom".into(),
                should_trigger: false,
            },
        ];
        let others = vec![("alpha".to_string(), "Alpha things".to_string())];
        let s = evaluate(
            &provider,
            "pdf-tools",
            "Reads PDFs",
            &others,
            &cases,
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(s.total, 5);
        assert_eq!(s.correct, 5);
        assert_eq!(s.results[2].chosen.as_deref(), Some("alpha"));
        assert!(!s.results[2].triggered);
        assert!(s.results[4].raw.contains("provider error"));
        assert_eq!(s.f1, 1.0);

        let token = CancellationToken::new();
        token.cancel();
        assert!(matches!(
            evaluate(&provider, "pdf-tools", "d", &others, &cases, &token).await,
            Err(HiveError::Cancelled)
        ));
    }
}
