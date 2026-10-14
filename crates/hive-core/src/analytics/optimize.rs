//! Description optimization. The held-out split is scored from recorded trigger streams.
//! A candidate is never written until `apply_optimized`.

use std::fs;
use std::path::Path;

use crate::authoring::versions::{VersionSource, VersionStore};
use crate::testing::judge_stream;

#[derive(Debug, Clone)]
pub struct QueryScore {
    pub query: String,
    pub should_trigger: bool,
    pub stream: String,
}

#[derive(Debug, Clone)]
pub struct Candidate {
    pub description: String,
    pub queries: Vec<QueryScore>,
}

#[derive(Debug, Clone)]
pub struct OptimizeOutcome {
    pub train: usize,
    pub test: usize,
    pub test_score: f64,
    pub winner: String,
    pub applied: bool,
    pub cancelled: bool,
    pub iterations: usize,
}

pub fn split_indices(count: usize, seed: u64) -> (Vec<usize>, Vec<usize>) {
    let mut order: Vec<usize> = (0..count).collect();
    let mut state = seed | 1;
    for index in (1..count).rev() {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let swap_with = (state as usize) % (index + 1);
        order.swap(index, swap_with);
    }
    let train_count = ((count * 60) / 100).clamp(1, count.saturating_sub(1));
    let test = order.split_off(train_count);
    (order, test)
}

pub fn query_passes(query: &QueryScore, command_name: &str) -> bool {
    let triggered = judge_stream(&query.stream, command_name);
    if query.should_trigger {
        triggered
    } else {
        !triggered
    }
}

pub fn score_indices(queries: &[QueryScore], indices: &[usize], command_name: &str) -> f64 {
    if indices.is_empty() {
        return 0.0;
    }
    let hits = indices
        .iter()
        .filter(|index| query_passes(&queries[**index], command_name))
        .count();
    hits as f64 / indices.len() as f64
}

pub fn optimize(
    candidates: &[Candidate],
    command_name: &str,
    seed: u64,
    cancel_after: Option<u32>,
) -> OptimizeOutcome {
    let empty = OptimizeOutcome {
        train: 0,
        test: 0,
        test_score: 0.0,
        winner: String::new(),
        applied: false,
        cancelled: false,
        iterations: 0,
    };
    let Some(first) = candidates.first() else {
        return empty;
    };
    let (train, test) = split_indices(first.queries.len(), seed);
    let mut best_score = -1.0_f64;
    let mut winner = String::new();
    let mut iterations: u32 = 0;
    for candidate in candidates {
        if cancel_after.is_some_and(|limit| iterations >= limit) {
            return OptimizeOutcome {
                train: train.len(),
                test: test.len(),
                test_score: best_score.max(0.0),
                winner,
                applied: false,
                cancelled: true,
                iterations: iterations as usize,
            };
        }
        iterations += 1;
        let score = score_indices(&candidate.queries, &test, command_name);
        if score > best_score {
            best_score = score;
            winner = candidate.description.clone();
        }
    }
    OptimizeOutcome {
        train: train.len(),
        test: test.len(),
        test_score: best_score.max(0.0),
        winner,
        applied: false,
        cancelled: false,
        iterations: iterations as usize,
    }
}

pub fn apply_optimized(subject_dir: &Path, description: &str) -> Result<String, String> {
    let path = subject_dir.join("SKILL.md");
    let text = fs::read_to_string(&path).map_err(|err| err.to_string())?;
    let next = replace_description(&text, description)?;
    fs::write(&path, next).map_err(|err| err.to_string())?;
    let version = VersionStore::open(subject_dir)
        .save_with_source(VersionSource::Optimized, None)
        .map_err(|err| err.to_string())?;
    Ok(version.id)
}

fn replace_description(text: &str, description: &str) -> Result<String, String> {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let Some(index) = lines
        .iter()
        .position(|line| line.starts_with("description:"))
    else {
        return Err("SKILL.md has no description".into());
    };
    lines[index] = format!("description: {description}");
    let mut out = lines.join("\n");
    out.push('\n');
    Ok(out)
}

pub fn hit_stream(command_name: &str) -> String {
    format!(
        "{{\"type\":\"stream_event\",\"event\":{{\"type\":\"content_block_start\",\"content_block\":{{\"type\":\"tool_use\",\"name\":\"Skill\"}}}}}}\n{{\"type\":\"stream_event\",\"event\":{{\"type\":\"content_block_delta\",\"delta\":{{\"type\":\"input_json_delta\",\"partial_json\":\"{{\\\"command\\\":\\\"{command_name}\\\"}}\"}}}}}}\n"
    )
}

pub fn miss_stream() -> String {
    "{\"type\":\"stream_event\",\"event\":{\"type\":\"content_block_start\",\"content_block\":{\"type\":\"tool_use\",\"name\":\"Bash\"}}}\n".into()
}
