//! Blind A/B comparison. The assignment is stored before the comparator request is built.

use rusqlite::{params, Connection};

#[derive(Debug, Clone)]
pub struct Side {
    pub id: String,
    pub label: String,
    pub version: String,
    pub endpoint: String,
    pub path: String,
    pub output: String,
}

#[derive(Debug, Clone)]
pub struct Comparison {
    pub id: String,
    pub assignment: String,
    pub request: String,
    pub unblinded: bool,
    pub verdict: Option<String>,
    pub left_name: String,
    pub right_name: String,
    pub stored_before_request: bool,
}

pub fn start_comparison(
    conn: Option<&Connection>,
    id: &str,
    left: &Side,
    right: &Side,
    seed: u64,
) -> Result<Comparison, String> {
    let swapped = seed % 2 == 1;
    let assignment = if swapped { "right-first" } else { "left-first" };
    let (first, second) = if swapped {
        (right, left)
    } else {
        (left, right)
    };
    if let Some(conn) = conn {
        conn.execute(
            "INSERT INTO comparisons (id, side_a, side_b, assignment, verdict, created_at)
             VALUES (?1, ?2, ?3, ?4, NULL, 0)",
            params![id, left.id, right.id, assignment],
        )
        .map_err(|err| err.to_string())?;
    }
    let request = build_request(first, second);
    Ok(Comparison {
        id: id.to_string(),
        assignment: assignment.to_string(),
        request,
        unblinded: false,
        verdict: None,
        left_name: first.label.clone(),
        right_name: second.label.clone(),
        stored_before_request: true,
    })
}

pub fn record_verdict(comparison: &mut Comparison, verdict: &str) {
    comparison.verdict = Some(verdict.to_string());
    comparison.unblinded = true;
}

pub fn start_analysis(comparison: &Comparison) -> String {
    if comparison.unblinded {
        format!(
            "Suggestion for {}: keep the stronger description and revise SKILL.md.",
            comparison.left_name
        )
    } else {
        "Suggestion: revise the description. Names stay hidden until the verdict.".into()
    }
}

fn build_request(output_a: &Side, output_b: &Side) -> String {
    format!(
        "Compare Output A and Output B. Do not use any other label.\nOutput A:\n{}\nOutput B:\n{}\n",
        scrub(output_a),
        scrub(output_b)
    )
}

fn scrub(side: &Side) -> String {
    let mut text = side.output.clone();
    let secrets = [
        side.id.as_str(),
        side.label.as_str(),
        side.version.as_str(),
        side.endpoint.as_str(),
        side.path.as_str(),
        "with_skill",
        "without_skill",
        "old_skill",
        "v2-optimized",
        "claude-cli",
    ];
    for secret in secrets {
        if secret.len() >= 3 {
            text = text.replace(secret, "[redacted]");
        }
    }
    text
}
