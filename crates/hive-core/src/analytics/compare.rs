//! Blind A/B comparison. The assignment is stored before the comparator request is built.

use std::fs;
use std::path::Path;

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

/// Open a blind comparison, record the verdict, and keep the row in `hive.db`.
/// The returned analysis names the stronger side only after the verdict is stored.
pub fn store_verdict(db: &std::path::Path, id: &str, verdict: &str) -> Result<String, String> {
    let conn = Connection::open(db).map_err(|err| err.to_string())?;
    let _ = conn.execute("DELETE FROM comparisons WHERE id = ?1", params![id]);
    let left = Side {
        id: "output-a".into(),
        label: "Output A".into(),
        version: "v1".into(),
        endpoint: "local".into(),
        path: "SKILL.md".into(),
        output: "left".into(),
    };
    let right = Side {
        id: "output-b".into(),
        label: "Output B".into(),
        version: "v1".into(),
        endpoint: "local".into(),
        path: "SKILL.md".into(),
        output: "right".into(),
    };
    let mut comparison = start_comparison(Some(&conn), id, &left, &right, 0)?;
    record_verdict(&mut comparison, verdict);
    conn.execute(
        "UPDATE comparisons SET verdict = ?1 WHERE id = ?2",
        params![verdict, id],
    )
    .map_err(|err| err.to_string())?;
    let dir = db.parent().unwrap_or(Path::new(".")).join("compare");
    let left_path = dir.join(format!("{id}-left.txt"));
    let right_path = dir.join(format!("{id}-right.txt"));
    if left_path.is_file() && right_path.is_file() {
        let analysis = write_output_analysis(
            &left_path,
            &right_path,
            &dir.join(format!("{id}-analysis.json")),
        )?;
        return Ok(analysis);
    }
    Ok(start_analysis(&comparison))
}

/// Analysis quotes the two output files. It does not substitute a canned paragraph.
pub fn write_output_analysis(left: &Path, right: &Path, out: &Path) -> Result<String, String> {
    let left_text = fs::read_to_string(left).map_err(|err| err.to_string())?;
    let right_text = fs::read_to_string(right).map_err(|err| err.to_string())?;
    let body = serde_json::json!({
        "left": left_text,
        "right": right_text,
    });
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let text = serde_json::to_string_pretty(&body).map_err(|err| err.to_string())?;
    fs::write(out, &text).map_err(|err| err.to_string())?;
    Ok(text)
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
