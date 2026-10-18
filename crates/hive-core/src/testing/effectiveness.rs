//! Measured skill effectiveness.
//!
//! The product path builds `AnthropicProvider` or `OpenAiProvider` and asks that
//! provider. Scores for the radar, the influence map, and the leaderboard are
//! computed from those runs. They do not call a model of their own.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

use crate::analytics::{set_baseline, write_history, HistoryEntry, RunSlice, SampleCell};
use crate::generation::{render, Template};
use crate::providers::{
    scrub, AnthropicProvider, CancellationToken, ChatProvider, ChatRequest, ChatResponse,
    ContentBlock, Flavor, Limiter, Message, Middleware, NoopSleeper, OpenAiProvider, Pricing,
    RequestMetadata, RunMode, SecretString, ThinkingMode, ToolDef, ToolUse, VecSink,
};
use crate::testing::{run_plan, save_suite, CheckSpec, Plan, Suite, SuiteCase, TriggerRow};

use super::suite::load_suite;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibrarySkill {
    pub name: String,
    pub description: String,
    pub content_hash: String,
    pub source_type: String,
    pub source_ref: String,
    pub source_revision: String,
}

#[derive(Clone, Debug)]
pub struct ModelChoice {
    pub id: String,
    pub strong: bool,
}

#[derive(Clone, Debug)]
pub struct MeasureRequest {
    pub skill_dir: PathBuf,
    pub workspace: PathBuf,
    pub db_path: PathBuf,
    pub base_url: String,
    pub api_key: String,
    pub kind: String,
    pub flavor: Option<String>,
    pub models: Vec<ModelChoice>,
    pub adapter: String,
    pub repetitions: u32,
    pub threshold: f64,
    pub pass_bar: f64,
    pub distractors: Vec<LibrarySkill>,
    pub seed: u64,
    pub cli_program: Option<PathBuf>,
    pub cli_prefix: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivationRow {
    pub owner: String,
    pub query_id: String,
    pub query: String,
    pub should_trigger: bool,
    pub outcome: String,
    pub culprit: Option<String>,
    pub ordered: Vec<String>,
    pub adapter: String,
    pub model: String,
    pub strong: bool,
    pub context: String,
    pub transcript: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Axis {
    pub name: String,
    pub value: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RankRow {
    pub skill: String,
    pub fires: f64,
    pub quality: f64,
    pub value_added: f64,
    pub efficiency: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoldLabel {
    pub kind: String,
    pub id: String,
    pub outcome: String,
    pub culprit: Option<String>,
    pub passed: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlapHit {
    pub left: String,
    pub right: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SkillScore {
    pub name: String,
    pub won: u64,
    pub grade: String,
    pub score: f64,
}

pub fn classify_activation(candidate: &str, ordered: &[String]) -> (String, Option<String>) {
    let present = ordered.iter().any(|name| name == candidate);
    let others: Vec<&String> = ordered
        .iter()
        .filter(|name| name.as_str() != candidate)
        .collect();
    if ordered.is_empty() {
        ("none".to_string(), None)
    } else if present && others.is_empty() {
        ("fired_alone".to_string(), None)
    } else if present {
        (
            "fired_with_others".to_string(),
            others.first().map(|name| (*name).clone()),
        )
    } else {
        (
            "hijacked".to_string(),
            others.first().map(|name| (*name).clone()),
        )
    }
}

pub fn conflict_matrix(rows: &[ActivationRow], skills: &[String]) -> Vec<Value> {
    let mut cells = Vec::new();
    for owner in skills {
        let owned: Vec<&ActivationRow> = rows.iter().filter(|row| &row.owner == owner).collect();
        for other in skills {
            if other == owner {
                continue;
            }
            let mut stole = Vec::new();
            let mut cofired = Vec::new();
            let mut neither = Vec::new();
            for row in &owned {
                let other_present = row.ordered.iter().any(|name| name == other);
                let owner_present = row.ordered.iter().any(|name| name == owner);
                if row.outcome == "hijacked" && row.culprit.as_deref() == Some(other.as_str()) {
                    stole.push(row.query_id.clone());
                } else if owner_present && other_present {
                    cofired.push(row.query_id.clone());
                } else if !owner_present && !other_present {
                    neither.push(row.query_id.clone());
                }
            }
            if stole.is_empty() && cofired.is_empty() && neither.is_empty() {
                continue;
            }
            cells.push(json!({
                "owner": owner,
                "other": other,
                "stole": stole.len(),
                "co_fired": cofired.len(),
                "neither": neither.len(),
                "stole_queries": stole,
                "co_fired_queries": cofired,
                "neither_queries": neither,
            }));
        }
    }
    cells
}

pub fn export_snapshot(skills: &[LibrarySkill]) -> Value {
    json!({
        "skills": skills.iter().map(|skill| json!({
            "name": skill.name,
            "description": skill.description,
            "content_hash": skill.content_hash,
            "source_type": skill.source_type,
            "source_ref": skill.source_ref,
            "source_revision": skill.source_revision,
        })).collect::<Vec<_>>()
    })
}

pub fn import_snapshot(document: &Value, dest: &Path) -> Result<Vec<LibrarySkill>, String> {
    let rows = document
        .get("skills")
        .and_then(Value::as_array)
        .ok_or_else(|| "snapshot has no skills array".to_string())?;
    fs::create_dir_all(dest).map_err(|err| err.to_string())?;
    let mut skills = Vec::new();
    for row in rows {
        let name = row.get("name").and_then(Value::as_str).unwrap_or("").trim();
        if name.is_empty() || name.contains("..") || name.contains('/') || name.contains('\\') {
            return Err(format!("snapshot skill name is not usable: {name}"));
        }
        let skill = LibrarySkill {
            name: name.to_string(),
            description: text_field(row, "description"),
            content_hash: text_field(row, "content_hash"),
            source_type: text_field(row, "source_type"),
            source_ref: text_field(row, "source_ref"),
            source_revision: text_field(row, "source_revision"),
        };
        let dir = dest.join(&skill.name);
        fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
        let body = format!(
            "---\nname: {}\ndescription: {}\n---\n",
            skill.name, skill.description
        );
        fs::write(dir.join("SKILL.md"), body).map_err(|err| err.to_string())?;
        skills.push(skill);
    }
    Ok(skills)
}

pub fn static_overlap(
    skills: &[LibrarySkill],
    triggers: &[(String, Vec<String>)],
) -> Vec<OverlapHit> {
    let mut hits = Vec::new();
    for (index, left) in skills.iter().enumerate() {
        for right in skills.iter().skip(index + 1) {
            if let Some(phrase) = shared_phrase(&left.description, &right.description) {
                hits.push(OverlapHit {
                    left: left.name.clone(),
                    right: right.name.clone(),
                    reason: format!("shared phrase: {phrase}"),
                });
            }
            let left_queries = queries_for(&left.name, triggers);
            let right_queries = queries_for(&right.name, triggers);
            for query in &left_queries {
                if query.len() >= 8 && contains_ignore(&right.description, query) {
                    hits.push(OverlapHit {
                        left: left.name.clone(),
                        right: right.name.clone(),
                        reason: format!("trigger contained in description: {query}"),
                    });
                }
            }
            for query in &right_queries {
                if query.len() >= 8 && contains_ignore(&left.description, query) {
                    hits.push(OverlapHit {
                        left: right.name.clone(),
                        right: left.name.clone(),
                        reason: format!("trigger contained in description: {query}"),
                    });
                }
            }
        }
    }
    hits
}

pub fn bisect_distractors(
    failing: &[String],
    mut breaks: impl FnMut(&[String]) -> bool,
) -> Vec<String> {
    let mut set: Vec<String> = failing.to_vec();
    if set.is_empty() || !breaks(&set) {
        return Vec::new();
    }
    loop {
        if set.len() <= 1 {
            return set;
        }
        let mid = set.len() / 2;
        let left = set[..mid].to_vec();
        let right = set[mid..].to_vec();
        if breaks(&left) {
            set = left;
            continue;
        }
        if breaks(&right) {
            set = right;
            continue;
        }
        let mut shrunk = false;
        for index in 0..set.len() {
            let mut next = set.clone();
            next.remove(index);
            if !next.is_empty() && breaks(&next) {
                set = next;
                shrunk = true;
                break;
            }
        }
        if !shrunk {
            return set;
        }
    }
}

pub fn radar_axes(input: &RadarInput) -> Vec<Axis> {
    let survival = match (input.fired_alone_alone, input.fired_alone_library) {
        (Some(alone), Some(library)) if alone > 0.0 => Some((library / alone * 100.0).min(100.0)),
        (Some(_), Some(_)) => None,
        _ => None,
    };
    let value = match (input.with_rate, input.without_rate) {
        (Some(with_rate), Some(without_rate)) => {
            Some((50.0 + 50.0 * (with_rate - without_rate)).clamp(0.0, 100.0))
        }
        _ => None,
    };
    let consistency = if input.repetition_rates.is_empty() {
        None
    } else {
        let deviation = stddev(&input.repetition_rates).min(1.0);
        Some(100.0 * (1.0 - deviation))
    };
    let efficiency = match (
        input.without_tokens,
        input.with_tokens,
        input.without_time,
        input.with_time,
    ) {
        (Some(base_tokens), Some(with_tokens), Some(base_time), Some(with_time)) => {
            Some(ratio(base_tokens, with_tokens).min(ratio(base_time, with_time)))
        }
        _ => None,
    };
    vec![
        axis(
            "Trigger pass rate",
            input.trigger_pass_rate.map(|rate| rate * 100.0),
        ),
        axis(
            "Negative-query pass rate",
            input.negative_pass_rate.map(|rate| rate * 100.0),
        ),
        axis("Survival in a crowded library", survival),
        axis(
            "With-skill task pass rate",
            input.with_rate.map(|rate| rate * 100.0),
        ),
        axis("Value added over without_skill", value),
        axis("Consistency", consistency),
        axis(
            "Weak-model performance",
            input.weak_pass_rate.map(|rate| rate * 100.0),
        ),
        axis("Efficiency", efficiency),
    ]
}

pub struct RadarInput {
    pub trigger_pass_rate: Option<f64>,
    pub negative_pass_rate: Option<f64>,
    pub fired_alone_alone: Option<f64>,
    pub fired_alone_library: Option<f64>,
    pub with_rate: Option<f64>,
    pub without_rate: Option<f64>,
    pub repetition_rates: Vec<f64>,
    pub weak_pass_rate: Option<f64>,
    pub with_tokens: Option<f64>,
    pub without_tokens: Option<f64>,
    pub with_time: Option<f64>,
    pub without_time: Option<f64>,
}

pub fn letter_grade(axes: &[Axis]) -> (String, f64) {
    let present: Vec<f64> = axes.iter().filter_map(|item| item.value).collect();
    if present.is_empty() {
        return ("F".to_string(), 0.0);
    }
    let mean = present.iter().sum::<f64>() / present.len() as f64;
    let letter = if mean >= 90.0 {
        "A"
    } else if mean >= 80.0 {
        "B"
    } else if mean >= 70.0 {
        "C"
    } else if mean >= 60.0 {
        "D"
    } else {
        "F"
    };
    (letter.to_string(), mean)
}

pub fn strong_model_only(rows: &[(bool, f64)], bar: f64) -> bool {
    let mut saw_strong = false;
    let mut strong_ok = true;
    let mut weak_miss = false;
    for (strong, rate) in rows {
        if *strong {
            saw_strong = true;
            if *rate < bar {
                strong_ok = false;
            }
        } else if *rate < bar {
            weak_miss = true;
        }
    }
    saw_strong && strong_ok && weak_miss
}

pub fn rank_leaderboard(rows: &[RankRow]) -> Vec<RankRow> {
    let mut ranked = rows.to_vec();
    ranked.sort_by(|left, right| {
        let left_score = left.fires * left.quality;
        let right_score = right.fires * right.quality;
        right_score
            .partial_cmp(&left_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(
                right
                    .value_added
                    .partial_cmp(&left.value_added)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(
                right
                    .efficiency
                    .partial_cmp(&left.efficiency)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
    });
    ranked
}

pub fn influence_map(skills: &[SkillScore], cells: &[Value]) -> Value {
    let mut edges = Vec::new();
    for cell in cells {
        let stole = cell.get("stole").and_then(Value::as_u64).unwrap_or(0);
        if stole == 0 {
            continue;
        }
        let owner = cell.get("owner").and_then(Value::as_str).unwrap_or("");
        let other = cell.get("other").and_then(Value::as_str).unwrap_or("");
        let owned = cell.get("stole").and_then(Value::as_u64).unwrap_or(0)
            + cell.get("co_fired").and_then(Value::as_u64).unwrap_or(0)
            + cell.get("neither").and_then(Value::as_u64).unwrap_or(0);
        let rate = if owned == 0 {
            0.0
        } else {
            stole as f64 / owned as f64
        };
        edges.push(json!({
            "source": other,
            "target": owner,
            "steal_rate": rate,
            "queries": cell.get("stole_queries").cloned().unwrap_or(json!([])),
        }));
    }
    json!({
        "nodes": skills.iter().map(|skill| json!({
            "id": skill.name,
            "won": skill.won,
            "grade": skill.grade,
            "score": skill.score,
        })).collect::<Vec<_>>(),
        "edges": edges,
    })
}

pub fn install_warnings(
    before: &[ActivationRow],
    after: &[ActivationRow],
    new_skill: &str,
) -> Vec<Value> {
    let mut warnings = Vec::new();
    for earlier in before {
        let Some(later) = after.iter().find(|row| row.query_id == earlier.query_id) else {
            continue;
        };
        if earlier.outcome == "fired_alone" && later.outcome != "fired_alone" {
            warnings.push(json!({
                "query_id": earlier.query_id,
                "before": earlier.outcome,
                "after": later.outcome,
                "suspect": new_skill,
                "culprit": later.culprit,
            }));
        }
    }
    warnings
}

pub fn calibrate_gold(
    labels: &[GoldLabel],
    rows: &[ActivationRow],
    expectations: &[(String, bool)],
) -> Value {
    if labels.is_empty() {
        return json!({"requested": false, "agreement": null, "disagreements": []});
    }
    let mut agreement = 0_u64;
    let mut disagreements = Vec::new();
    for label in labels {
        if label.kind == "trigger" {
            let Some(row) = rows.iter().find(|row| row.query_id == label.id) else {
                disagreements.push(json!({"id": label.id, "reason": "missing activation"}));
                continue;
            };
            let culprit_ok = label.culprit.as_deref() == row.culprit.as_deref();
            if row.outcome == label.outcome && (label.outcome != "hijacked" || culprit_ok) {
                agreement += 1;
            } else {
                disagreements.push(json!({
                    "id": label.id,
                    "expected": label.outcome,
                    "actual": row.outcome,
                    "expected_culprit": label.culprit,
                    "actual_culprit": row.culprit,
                }));
            }
        } else if label.kind == "expectation" {
            let Some((_, passed)) = expectations.iter().find(|(id, _)| id == &label.id) else {
                disagreements.push(json!({"id": label.id, "reason": "missing expectation"}));
                continue;
            };
            if Some(*passed) == label.passed {
                agreement += 1;
            } else {
                disagreements.push(json!({
                    "id": label.id,
                    "expected": label.passed,
                    "actual": passed,
                }));
            }
        }
    }
    json!({
        "requested": true,
        "agreement": agreement,
        "disagreements": disagreements,
    })
}

pub fn verdict_code(
    hijacked: bool,
    cofired: bool,
    none_on_positive: bool,
    strong_only: bool,
    value_below: bool,
    install_regressed: bool,
) -> &'static str {
    if hijacked {
        "hijacked"
    } else if install_regressed {
        "install"
    } else if none_on_positive {
        "none"
    } else if value_below {
        "value"
    } else if cofired {
        "cofired"
    } else if strong_only {
        "strong"
    } else {
        "healthy"
    }
}

pub fn snapshot_skill(skill_dir: &Path) -> Result<(), String> {
    let dest = skill_dir.join("skill-snapshot");
    if dest.exists() {
        fs::remove_dir_all(&dest).map_err(|err| err.to_string())?;
    }
    fs::create_dir_all(&dest).map_err(|err| err.to_string())?;
    copy_tree(skill_dir, &dest)?;
    Ok(())
}

pub fn run_cli_trigger(
    program: &Path,
    prefix: &[String],
    project: &Path,
    skill: &str,
    query: &str,
) -> Result<String, String> {
    let commands = project.join(".claude").join("commands");
    fs::create_dir_all(&commands).map_err(|err| err.to_string())?;
    let path = commands.join(format!("{skill}-skill-measure.md"));
    fs::write(
        &path,
        format!("---\ndescription: |\n  {skill}\n---\n# {skill}\n"),
    )
    .map_err(|err| err.to_string())?;
    let _guard = DeleteFile(path);
    let mut command = Command::new(program);
    command.args(prefix);
    command.arg("-p");
    command.arg(query);
    command.arg("--output-format");
    command.arg("stream-json");
    command.current_dir(project);
    let output = command.output().map_err(|err| err.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "trigger command exited {}",
            output.status.code().unwrap_or(-1)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn interview_goal(
    provider: &dyn ChatProvider,
    model: &str,
    goal: &str,
    secret: &str,
) -> Result<Value, String> {
    let mut vars = std::collections::BTreeMap::new();
    vars.insert("kind".to_string(), "skill".to_string());
    vars.insert("goal".to_string(), goal.to_string());
    vars.insert("library_listing".to_string(), String::new());
    vars.insert("profile".to_string(), "portable".to_string());
    vars.insert("name".to_string(), "generated".to_string());
    let prompt = format!("HIVE_INTERVIEW\n{}", render(Template::P1, &vars));
    let response = ask(provider, model, None, &prompt, &[], secret, "generator")?;
    let parsed: Value = serde_json::from_str(response.text.trim()).unwrap_or(Value::Null);
    Ok(parsed)
}

pub fn suggest_description(
    provider: &dyn ChatProvider,
    model: &str,
    name: &str,
    description: &str,
    train_results: &str,
    secret: &str,
) -> Result<String, String> {
    let mut vars = std::collections::BTreeMap::new();
    vars.insert("name".to_string(), name.to_string());
    vars.insert("description".to_string(), description.to_string());
    vars.insert("train_results".to_string(), train_results.to_string());
    let prompt = format!("HIVE_REWRITE\n{}", render(Template::P8, &vars));
    let response = ask(provider, model, None, &prompt, &[], secret, "generator")?;
    let parsed: Value = serde_json::from_str(response.text.trim()).unwrap_or(Value::Null);
    parsed
        .get("description")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "description rewrite did not return a description".to_string())
}

pub struct BlindRequest<'a> {
    pub provider: &'a dyn ChatProvider,
    pub model: &'a str,
    pub prompt: &'a str,
    pub output_a: &'a str,
    pub output_b: &'a str,
    pub sealed_a: &'a str,
    pub sealed_b: &'a str,
    pub secret: &'a str,
}

pub fn blind_compare(request: &BlindRequest<'_>) -> Result<Value, String> {
    let mut vars = std::collections::BTreeMap::new();
    vars.insert("prompt".to_string(), request.prompt.to_string());
    vars.insert("outputs_a".to_string(), request.output_a.to_string());
    vars.insert("outputs_b".to_string(), request.output_b.to_string());
    let body = format!("HIVE_BLIND\n{}", render(Template::P12, &vars));
    let response = ask(
        request.provider,
        request.model,
        None,
        &body,
        &[],
        request.secret,
        "generator",
    )?;
    let parsed: Value = serde_json::from_str(response.text.trim()).unwrap_or(Value::Null);
    let winner = parsed
        .get("winner")
        .and_then(Value::as_str)
        .unwrap_or("TIE")
        .to_string();
    Ok(json!({
        "winner": winner,
        "revealed": false,
        "output_a": request.output_a,
        "output_b": request.output_b,
        "sealed": {"A": request.sealed_a, "B": request.sealed_b},
    }))
}

pub fn apply_description(
    skill_dir: &Path,
    description: &str,
    restored: bool,
) -> Result<(), String> {
    if !restored {
        return Err("Apply stays disabled until the re-test restores fired alone".to_string());
    }
    let path = skill_dir.join("SKILL.md");
    let text = fs::read_to_string(&path).map_err(|err| err.to_string())?;
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let mut replaced = false;
    for line in &mut lines {
        if line.starts_with("description:") {
            *line = format!("description: {description}");
            replaced = true;
        }
    }
    if !replaced {
        return Err("SKILL.md has no description line".to_string());
    }
    fs::write(&path, format!("{}\n", lines.join("\n"))).map_err(|err| err.to_string())?;
    Ok(())
}

pub fn run_measured(request: &MeasureRequest) -> Result<Value, String> {
    if request.models.is_empty() {
        return Err("a measured run needs at least one model".to_string());
    }
    let suite = load_suite(&request.skill_dir).map_err(|err| err.to_string())?;
    let document = fs::read_to_string(request.skill_dir.join("SKILL.md")).unwrap_or_default();
    let description = front_description(&document);
    let hash = crate::core::content_hash::hash_directory(&request.skill_dir)
        .unwrap_or_else(|_| "unhashed".to_string());
    let candidate = LibrarySkill {
        name: suite.skill_name.clone(),
        description: description.clone(),
        content_hash: hash,
        source_type: "local".to_string(),
        source_ref: String::new(),
        source_revision: String::new(),
    };
    fs::create_dir_all(&request.workspace).map_err(|err| err.to_string())?;
    fs::write(
        request.workspace.join("progress.json"),
        json!({"steps": ["preparing", "running"]}).to_string(),
    )
    .map_err(|err| err.to_string())?;
    let lookalikes = write_lookalikes(&request.workspace, &candidate)?;
    let mut library = request.distractors.clone();
    library.extend(lookalikes.iter().cloned());
    let provider = provider_for(request)?;
    let model = request.models[0].id.clone();
    let plan = Plan {
        skill_dir: request.skill_dir.clone(),
        workspace: request.workspace.clone(),
        db_path: request.db_path.clone(),
        mode: RunMode::Live,
        seed: request.seed,
        repetitions: 1,
        budget_usd: None,
        cancel_after: None,
        crash_after: None,
        recordings: request.workspace.join("recordings"),
        personal_skills: None,
        secret: request.api_key.clone(),
        usd_per_cell: None,
        model: model.clone(),
        kind: request.kind.clone(),
    };
    let report = run_plan(&plan, provider.as_ref()).map_err(|err| err.to_string())?;
    let (with_rate, without_rate, with_tokens, without_tokens, with_time, without_time) =
        configuration_stats(&report);
    let repetitions = request.repetitions.max(1);
    let mut activations = Vec::new();
    let queries = trigger_queries(&suite);
    for choice in &request.models {
        for (context, skills) in [
            ("alone", vec![candidate.clone()]),
            ("library", {
                let mut all = vec![candidate.clone()];
                all.extend(library.iter().cloned());
                all
            }),
        ] {
            for (index, query) in queries.iter().enumerate() {
                let mut runs = Vec::new();
                let query_id = format!("q{index}");
                for _ in 0..repetitions {
                    let row = ask_trigger(&TriggerAsk {
                        provider: provider.as_ref(),
                        model: &choice.id,
                        strong: choice.strong,
                        adapter: &request.adapter,
                        context,
                        candidate: &candidate.name,
                        skills: skills.as_slice(),
                        query_id: &query_id,
                        query: &query.query,
                        should_trigger: query.should_trigger,
                        secret: &request.api_key,
                        workspace: &request.workspace,
                    })?;
                    runs.push(row);
                }
                activations.push(aggregate_trigger(&runs, repetitions, request.threshold));
            }
        }
    }
    if let Some(program) = &request.cli_program {
        for (index, query) in queries.iter().enumerate() {
            let project = request.workspace.join("cli-project");
            fs::create_dir_all(&project).map_err(|err| err.to_string())?;
            let stream = run_cli_trigger(
                program,
                &request.cli_prefix,
                &project,
                &candidate.name,
                &query.query,
            )?;
            let names = vec![candidate.name.clone()];
            let ordered = ordered_from_stream(&stream, &names);
            let (outcome, culprit) = classify_activation(&candidate.name, &ordered);
            let transcript = request.workspace.join(format!("cli-q{index}.jsonl"));
            fs::write(
                &transcript,
                scrub(&stream, std::slice::from_ref(&request.api_key)),
            )
            .map_err(|err| err.to_string())?;
            activations.push(ActivationRow {
                owner: candidate.name.clone(),
                query_id: format!("cli-q{index}"),
                query: query.query.clone(),
                should_trigger: query.should_trigger,
                outcome,
                culprit,
                ordered,
                adapter: "claude-code".to_string(),
                model: model.clone(),
                strong: true,
                context: "cli".to_string(),
                transcript: transcript.display().to_string(),
            });
        }
    }
    let library_rows: Vec<ActivationRow> = activations
        .iter()
        .filter(|row| {
            row.context == "library" && row.model == model && row.adapter == request.adapter
        })
        .cloned()
        .collect();
    let alone_rows: Vec<ActivationRow> = activations
        .iter()
        .filter(|row| {
            row.context == "alone" && row.model == model && row.adapter == request.adapter
        })
        .cloned()
        .collect();
    let trigger_pass = pass_rate(&library_rows, true, request.threshold);
    let negative_pass = pass_rate(&library_rows, false, request.threshold);
    let alone_fired = fired_alone_rate(&alone_rows);
    let library_fired = fired_alone_rate(&library_rows);
    let weak_rates: Vec<f64> = request
        .models
        .iter()
        .filter(|choice| !choice.strong)
        .map(|choice| {
            let rows: Vec<ActivationRow> = activations
                .iter()
                .filter(|row| row.model == choice.id && row.context == "library")
                .cloned()
                .collect();
            pass_rate(&rows, true, request.threshold).unwrap_or(0.0)
        })
        .collect();
    let weak_pass = if request.models.iter().all(|choice| choice.strong) {
        None
    } else {
        Some(if weak_rates.is_empty() {
            0.0
        } else {
            weak_rates.iter().sum::<f64>() / weak_rates.len() as f64
        })
    };
    let repetition_rates = library_rows
        .iter()
        .filter(|row| row.should_trigger)
        .map(|row| {
            if row.outcome == "fired_alone" {
                1.0
            } else {
                0.0
            }
        })
        .collect::<Vec<_>>();
    let axes = radar_axes(&RadarInput {
        trigger_pass_rate: trigger_pass,
        negative_pass_rate: negative_pass,
        fired_alone_alone: alone_fired,
        fired_alone_library: library_fired,
        with_rate: Some(with_rate),
        without_rate: Some(without_rate),
        repetition_rates,
        weak_pass_rate: weak_pass,
        with_tokens,
        without_tokens,
        with_time,
        without_time,
    });
    let (grade, score) = letter_grade(&axes);
    let per_model: Vec<(bool, f64)> = request
        .models
        .iter()
        .map(|choice| {
            let rows: Vec<ActivationRow> = activations
                .iter()
                .filter(|row| {
                    row.model == choice.id
                        && row.context == "library"
                        && row.adapter == request.adapter
                })
                .cloned()
                .collect();
            (
                choice.strong,
                pass_rate(&rows, true, request.threshold).unwrap_or(0.0),
            )
        })
        .collect();
    let strong_only = strong_model_only(&per_model, request.pass_bar);
    let hijacked = library_rows.iter().any(|row| row.outcome == "hijacked");
    let cofired = library_rows
        .iter()
        .any(|row| row.outcome == "fired_with_others");
    let none_on_positive = library_rows
        .iter()
        .any(|row| row.should_trigger && row.outcome == "none");
    let value_score = axes
        .iter()
        .find(|item| item.name == "Value added over without_skill")
        .and_then(|item| item.value)
        .unwrap_or(50.0);
    let interference = interference_report(&InterferenceAsk {
        request,
        provider: provider.as_ref(),
        candidate: &candidate,
        library: &library,
        model: &model,
        rows: &library_rows,
    })?;
    let install_regressed = interference
        .get("warnings")
        .and_then(Value::as_array)
        .is_some_and(|rows| !rows.is_empty());
    let code = verdict_code(
        hijacked,
        cofired,
        none_on_positive,
        strong_only,
        value_score < 50.0,
        install_regressed,
    );
    let names: Vec<String> = std::iter::once(candidate.name.clone())
        .chain(library.iter().map(|skill| skill.name.clone()))
        .collect();
    let matrix = conflict_matrix(&library_rows, &names);
    let won = library_rows
        .iter()
        .filter(|row| row.should_trigger && row.outcome == "fired_alone")
        .count() as u64;
    let influence = influence_map(
        &[SkillScore {
            name: candidate.name.clone(),
            won,
            grade: grade.clone(),
            score,
        }],
        &matrix,
    );
    let efficiency = axes
        .iter()
        .find(|item| item.name == "Efficiency")
        .and_then(|item| item.value)
        .unwrap_or(0.0);
    let fires = trigger_pass.unwrap_or(0.0);
    let board = rank_leaderboard(&[RankRow {
        skill: candidate.name.clone(),
        fires,
        quality: with_rate,
        value_added: value_score,
        efficiency,
    }]);
    let overlap = static_overlap(
        &{
            let mut all = vec![candidate.clone()];
            all.extend(library.iter().cloned());
            all
        },
        &[(
            candidate.name.clone(),
            queries.iter().map(|row| row.query.clone()).collect(),
        )],
    );
    let gold = read_gold(&request.skill_dir)?;
    let expectations = expectation_pairs(&request.workspace);
    let calibration = calibrate_gold(&gold, &library_rows, &expectations);
    let history =
        write_measured_history(&request.skill_dir, &candidate.name, with_rate, without_rate)?;
    let slice = RunSlice {
        run_id: report.run_id.clone(),
        endpoint_model: model.clone(),
        cells: report
            .cells
            .iter()
            .map(|cell| SampleCell {
                id: cell.id.clone(),
                eval_id: "1".to_string(),
                configuration: cell.configuration.clone(),
                pass_rate: cell.pass_rate,
                expectations: Vec::new(),
                trigger_should: None,
                trigger_rate: None,
                checks_ok: cell.status == "graded",
                tokens: 1.0,
                cost: 0.0,
                time_seconds: 1.0,
            })
            .collect(),
    };
    let baseline_dir = request.workspace.join("baseline");
    let loaded = crate::analytics::load_baseline(&baseline_dir);
    let regression = crate::analytics::evaluate(loaded.as_ref(), &slice);
    if loaded.is_none() {
        set_baseline(&baseline_dir, &slice)?;
    }
    write_exports(&request.workspace, &candidate.name, &suite, &report)?;
    snapshot_skill(&request.skill_dir)?;
    let weakest = axes
        .iter()
        .filter_map(|item| item.value.map(|value| (item.name.clone(), value)))
        .min_by(|left, right| {
            left.1
                .partial_cmp(&right.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(name, _)| name)
        .unwrap_or_default();
    let thickest = influence
        .get("edges")
        .and_then(Value::as_array)
        .and_then(|edges| {
            edges.iter().max_by(|left, right| {
                let left_rate = left
                    .get("steal_rate")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let right_rate = right
                    .get("steal_rate")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                left_rate
                    .partial_cmp(&right_rate)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        })
        .map(|edge| {
            format!(
                "{} -> {}",
                edge.get("source").and_then(Value::as_str).unwrap_or(""),
                edge.get("target").and_then(Value::as_str).unwrap_or("")
            )
        })
        .unwrap_or_default();
    let omitted: Vec<String> = axes
        .iter()
        .filter(|item| item.value.is_none())
        .map(|item| item.name.clone())
        .collect();
    let body = json!({
        "run_id": report.run_id,
        "skill": candidate.name,
        "grade": grade,
        "grade_score": score,
        "verdict": code,
        "omitted_axes": omitted,
        "weakest_axis": weakest,
        "thickest_arrow": thickest,
        "leader_name": board.first().map(|row| row.skill.clone()).unwrap_or_default(),
        "leader_factor": "fires and quality",
        "axes": axes.iter().map(|item| json!({"name": item.name, "value": item.value})).collect::<Vec<_>>(),
        "influence": influence,
        "leaderboard": board.iter().enumerate().map(|(index, row)| json!({
            "rank": index + 1,
            "skill": row.skill,
            "fires": row.fires,
            "quality": row.quality,
            "value_added": row.value_added,
            "efficiency": row.efficiency,
            "score": row.fires * row.quality,
        })).collect::<Vec<_>>(),
        "conflicts": matrix,
        "activations": activations.iter().map(activation_json).collect::<Vec<_>>(),
        "models": request.models.iter().zip(per_model.iter()).map(|(choice, (_, rate))| json!({
            "id": choice.id,
            "adapter": request.adapter,
            "strong": choice.strong,
            "pass_rate": rate,
            "strong_model_only": strong_only,
        })).collect::<Vec<_>>(),
        "gold": calibration,
        "overlap": overlap.iter().map(|hit| json!({"left": hit.left, "right": hit.right, "reason": hit.reason})).collect::<Vec<_>>(),
        "interference": interference,
        "history": history,
        "alerts": regression.alerts.iter().map(|alert| json!({"rule": alert.rule, "message": alert.message})).collect::<Vec<_>>(),
        "offer_set_baseline": regression.offer_set_baseline,
        "with_rate": with_rate,
        "without_rate": without_rate,
        "value_delta": with_rate - without_rate,
        "cells": report.cells.iter().map(|cell| json!({
            "id": cell.id,
            "configuration": cell.configuration,
            "pass_rate": cell.pass_rate,
            "status": cell.status,
            "dir": cell.dir.display().to_string(),
        })).collect::<Vec<_>>(),
        "matrix": {
            "configurations": ["with_skill", "without_skill"],
            "models": request.models.iter().map(|choice| choice.id.clone()).collect::<Vec<_>>(),
            "adapter": request.adapter,
        },
    });
    fs::write(
        request.workspace.join("effectiveness.json"),
        serde_json::to_string_pretty(&body).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())?;
    fs::write(
        request.workspace.join("progress.json"),
        json!({"steps": ["preparing", "running", "triggers", "done"]}).to_string(),
    )
    .map_err(|err| err.to_string())?;
    Ok(body)
}

fn provider_for(request: &MeasureRequest) -> Result<Box<dyn ChatProvider>, String> {
    let key = SecretString::new(request.api_key.clone());
    if request.kind == "openai" {
        let flavor = if request.flavor.as_deref() == Some("azure") {
            Flavor::Azure
        } else {
            Flavor::Openai
        };
        Ok(Box::new(OpenAiProvider::new(
            "measured",
            request.base_url.clone(),
            flavor,
            key,
        )))
    } else if request.kind == "anthropic" {
        Ok(Box::new(AnthropicProvider::new(
            "measured",
            request.base_url.clone(),
            key,
        )))
    } else {
        Err(format!(
            "measured runs use the Anthropic or OpenAI adapter (got {})",
            request.kind
        ))
    }
}

#[derive(Copy, Clone)]
struct TriggerAsk<'a> {
    provider: &'a dyn ChatProvider,
    model: &'a str,
    strong: bool,
    adapter: &'a str,
    context: &'a str,
    candidate: &'a str,
    skills: &'a [LibrarySkill],
    query_id: &'a str,
    query: &'a str,
    should_trigger: bool,
    secret: &'a str,
    workspace: &'a Path,
}

fn ask_trigger(ask_for: &TriggerAsk<'_>) -> Result<ActivationRow, String> {
    let TriggerAsk {
        provider,
        model,
        strong,
        adapter,
        context,
        candidate,
        skills,
        query_id,
        query,
        should_trigger,
        secret,
        workspace,
    } = *ask_for;
    let listed = serde_json::to_string(
        &skills
            .iter()
            .map(|skill| json!({"name": skill.name, "description": skill.description}))
            .collect::<Vec<_>>(),
    )
    .map_err(|err| err.to_string())?;
    let system = format!("HIVE_TRIGGER\navailable_skills: {listed}");
    let tools = vec![
        ToolDef {
            name: "Skill".to_string(),
            description: "Activate one skill from available_skills".to_string(),
            input_schema: json!({"type": "object", "properties": {"skill": {"type": "string"}}, "required": ["skill"]}),
        },
        ToolDef {
            name: "Read".to_string(),
            description: "Read a file under a skill directory".to_string(),
            input_schema: json!({"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}),
        },
    ];
    let response = ask(
        provider,
        model,
        Some(&system),
        query,
        &tools,
        secret,
        "trigger",
    )?;
    let ordered = ordered_from_tools(&response.tool_uses, skills);
    let (outcome, culprit) = classify_activation(candidate, &ordered);
    let transcript = workspace
        .join("triggers")
        .join(format!("{context}-{model}-{query_id}.jsonl"));
    if let Some(parent) = transcript.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let logged = scrub(&response.text, &[secret.to_string()]);
    fs::write(&transcript, logged).map_err(|err| err.to_string())?;
    Ok(ActivationRow {
        owner: candidate.to_string(),
        query_id: query_id.to_string(),
        query: query.to_string(),
        should_trigger,
        outcome,
        culprit,
        ordered,
        adapter: adapter.to_string(),
        model: model.to_string(),
        strong,
        context: context.to_string(),
        transcript: transcript.display().to_string(),
    })
}

fn aggregate_trigger(runs: &[ActivationRow], _repetitions: u32, _threshold: f64) -> ActivationRow {
    let fired = runs
        .iter()
        .filter(|row| row.outcome == "fired_alone")
        .count();
    let majority = if fired * 2 >= runs.len() {
        "fired_alone"
    } else {
        runs.last()
            .map(|row| row.outcome.as_str())
            .unwrap_or("none")
    };
    let mut row = runs.last().cloned().unwrap_or_else(|| ActivationRow {
        owner: String::new(),
        query_id: String::new(),
        query: String::new(),
        should_trigger: false,
        outcome: "none".to_string(),
        culprit: None,
        ordered: Vec::new(),
        adapter: String::new(),
        model: String::new(),
        strong: false,
        context: String::new(),
        transcript: String::new(),
    });
    if majority == "fired_alone" {
        row.outcome = "fired_alone".to_string();
        row.culprit = None;
    }
    row
}

fn pass_rate(rows: &[ActivationRow], positive: bool, threshold: f64) -> Option<f64> {
    let selected: Vec<&ActivationRow> = rows
        .iter()
        .filter(|row| row.should_trigger == positive)
        .collect();
    if selected.is_empty() {
        return None;
    }
    let passed = selected
        .iter()
        .filter(|row| {
            if positive {
                row.outcome == "fired_alone"
            } else {
                row.outcome == "none" || row.outcome == "hijacked"
            }
        })
        .count();
    let rate = passed as f64 / selected.len() as f64;
    let _ = threshold;
    Some(rate)
}

fn fired_alone_rate(rows: &[ActivationRow]) -> Option<f64> {
    let positives: Vec<&ActivationRow> = rows.iter().filter(|row| row.should_trigger).collect();
    if positives.is_empty() {
        return None;
    }
    let fired = positives
        .iter()
        .filter(|row| row.outcome == "fired_alone")
        .count();
    Some(fired as f64 / positives.len() as f64)
}

fn ask(
    provider: &dyn ChatProvider,
    model: &str,
    system: Option<&str>,
    user: &str,
    tools: &[ToolDef],
    secret: &str,
    purpose: &str,
) -> Result<ChatResponse, String> {
    let middleware = Middleware {
        mode: RunMode::Live,
        recordings_dir: PathBuf::from("recordings"),
        kind: provider.kind_name().to_string(),
        max_retries: 2,
        limiter: Limiter::new(4),
        sleeper: Box::new(NoopSleeper::new()),
        secrets: vec![secret.to_string()],
        pricing: Pricing {
            input_per_mtok: 2.0,
            output_per_mtok: 10.0,
            cache_read_per_mtok: 0.2,
            cache_write_per_mtok: 2.5,
        },
        provider_id: "measured".to_string(),
        hits: std::sync::Mutex::new(0),
    };
    let request = ChatRequest {
        model: model.to_string(),
        system: system.map(str::to_string),
        messages: vec![Message {
            role: "user".to_string(),
            content: vec![ContentBlock::Text(user.to_string())],
        }],
        tools: tools.to_vec(),
        max_tokens: 256,
        effort: None,
        thinking: ThinkingMode::Off,
        stream: true,
        metadata: RequestMetadata {
            purpose: purpose.to_string(),
            ref_id: None,
            provider_id: None,
        },
    };
    let mut sink = VecSink::default();
    let (response, _) = middleware
        .complete(provider, &request, &mut sink, &CancellationToken::new())
        .map_err(|err| err.to_string())?;
    Ok(response)
}

fn ordered_from_tools(tools: &[ToolUse], skills: &[LibrarySkill]) -> Vec<String> {
    let mut ordered = Vec::new();
    for tool in tools {
        if tool.name != "Skill" && tool.name != "Read" {
            continue;
        }
        let blob = format!("{} {}", tool.raw_input, tool.input);
        if let Some(name) = quoted_skill(&blob, skills) {
            if !ordered.iter().any(|have| have == &name) {
                ordered.push(name);
            }
        }
    }
    ordered
}

fn ordered_from_stream(stream: &str, names: &[String]) -> Vec<String> {
    let skills: Vec<LibrarySkill> = names
        .iter()
        .map(|name| LibrarySkill {
            name: name.clone(),
            description: String::new(),
            content_hash: String::new(),
            source_type: String::new(),
            source_ref: String::new(),
            source_revision: String::new(),
        })
        .collect();
    let mut ordered = Vec::new();
    let mut buffer = String::new();
    let mut current = false;
    for line in stream.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) == Some("assistant") {
            if let Some(blocks) = value.pointer("/message/content").and_then(Value::as_array) {
                for block in blocks {
                    if block.get("type").and_then(Value::as_str) == Some("tool_use") {
                        if let Some(name) = quoted_skill(&block.to_string(), &skills) {
                            if !ordered.iter().any(|have| have == &name) {
                                ordered.push(name);
                            }
                        }
                    }
                }
            }
        }
        let event = if value.get("type").and_then(Value::as_str) == Some("stream_event") {
            value.get("event").cloned().unwrap_or(value.clone())
        } else {
            value.clone()
        };
        match event.get("type").and_then(Value::as_str).unwrap_or("") {
            "content_block_start" => {
                let block = &event["content_block"];
                let name = block.get("name").and_then(Value::as_str).unwrap_or("");
                current = block.get("type").and_then(Value::as_str) == Some("tool_use")
                    && (name == "Skill" || name == "Read");
                if current {
                    buffer.clear();
                }
            }
            "content_block_delta" if current => {
                if let Some(partial) = event.pointer("/delta/partial_json").and_then(Value::as_str)
                {
                    buffer.push_str(partial);
                }
            }
            "content_block_stop" if current => {
                if let Some(name) = quoted_skill(&buffer, &skills) {
                    if !ordered.iter().any(|have| have == &name) {
                        ordered.push(name);
                    }
                }
                current = false;
                buffer.clear();
            }
            _ => {}
        }
    }
    ordered
}

fn quoted_skill(blob: &str, skills: &[LibrarySkill]) -> Option<String> {
    let mut best: Option<&LibrarySkill> = None;
    for skill in skills {
        let needle = format!("\"{}\"", skill.name);
        if blob.contains(&needle)
            && best
                .map(|have| skill.name.len() > have.name.len())
                .unwrap_or(true)
        {
            best = Some(skill);
        }
    }
    best.map(|skill| skill.name.clone())
}

fn configuration_stats(
    report: &crate::testing::RunReport,
) -> (f64, f64, Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
    let mean = |name: &str| -> f64 {
        let rows: Vec<f64> = report
            .cells
            .iter()
            .filter(|cell| cell.configuration == name)
            .map(|cell| cell.pass_rate)
            .collect();
        if rows.is_empty() {
            0.0
        } else {
            rows.iter().sum::<f64>() / rows.len() as f64
        }
    };
    let timing = |name: &str| -> (Option<f64>, Option<f64>) {
        let mut tokens = 0.0;
        let mut seconds = 0.0;
        let mut count = 0.0;
        for cell in report
            .cells
            .iter()
            .filter(|cell| cell.configuration == name)
        {
            if let Ok(text) = fs::read_to_string(cell.dir.join("timing.json")) {
                if let Ok(value) = serde_json::from_str::<Value>(&text) {
                    tokens += value
                        .get("total_tokens")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0);
                    seconds += value
                        .get("total_duration_seconds")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0);
                    count += 1.0;
                }
            }
        }
        if count == 0.0 {
            (None, None)
        } else {
            (Some(tokens), Some(seconds))
        }
    };
    let (with_tokens, with_time) = timing("with_skill");
    let (without_tokens, without_time) = timing("without_skill");
    (
        mean("with_skill"),
        mean("without_skill"),
        with_tokens,
        without_tokens,
        with_time,
        without_time,
    )
}

fn trigger_queries(suite: &Suite) -> Vec<TriggerRow> {
    if suite.triggers.is_empty() {
        vec![
            TriggerRow {
                query: format!("use {} now", suite.skill_name),
                should_trigger: true,
                rationale: None,
            },
            TriggerRow {
                query: "should-not forecast the weather".to_string(),
                should_trigger: false,
                rationale: None,
            },
        ]
    } else {
        suite.triggers.clone()
    }
}

struct InterferenceAsk<'a> {
    request: &'a MeasureRequest,
    provider: &'a dyn ChatProvider,
    candidate: &'a LibrarySkill,
    library: &'a [LibrarySkill],
    model: &'a str,
    rows: &'a [ActivationRow],
}

fn interference_report(ask: &InterferenceAsk<'_>) -> Result<Value, String> {
    let mut skills = vec![ask.candidate.clone()];
    skills.extend(ask.library.iter().cloned());
    let snapshot = export_snapshot(&skills);
    let snapshot_path = ask.request.workspace.join("library-snapshot.json");
    let previous_snapshot = fs::read_to_string(&snapshot_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok());
    fs::write(
        &snapshot_path,
        serde_json::to_string_pretty(&snapshot).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())?;
    import_snapshot(&snapshot, &ask.request.workspace.join("context"))?;
    let previous_path = ask.request.workspace.join("previous-activations.json");
    let previous = read_activation_file(&previous_path);
    let suspects = new_skill_names(previous_snapshot.as_ref(), &snapshot);
    let warnings = if previous.is_empty() {
        Vec::new()
    } else if suspects.is_empty() {
        install_warnings(&previous, ask.rows, "library")
    } else {
        let mut found = Vec::new();
        for name in &suspects {
            found.extend(install_warnings(&previous, ask.rows, name));
        }
        found
    };
    let current = Value::Array(ask.rows.iter().map(activation_json).collect());
    fs::write(
        &previous_path,
        serde_json::to_string_pretty(&current).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())?;
    let failing: Vec<&ActivationRow> = ask
        .rows
        .iter()
        .filter(|row| row.outcome == "hijacked" || row.outcome == "fired_with_others")
        .collect();
    let names: Vec<String> = ask.library.iter().map(|skill| skill.name.clone()).collect();
    let sample = failing.first().copied();
    let bisect = if let Some(sample) = sample {
        let choice = ask
            .request
            .models
            .iter()
            .find(|item| item.id == ask.model)
            .cloned();
        bisect_distractors(&names, |set| {
            let Some(choice) = choice.as_ref() else {
                return false;
            };
            let subset: Vec<LibrarySkill> = ask
                .library
                .iter()
                .filter(|skill| set.iter().any(|name| name == &skill.name))
                .cloned()
                .collect();
            let mut context = vec![ask.candidate.clone()];
            context.extend(subset);
            match ask_trigger(&TriggerAsk {
                provider: ask.provider,
                model: &choice.id,
                strong: choice.strong,
                adapter: &ask.request.adapter,
                context: "bisect",
                candidate: &ask.candidate.name,
                skills: &context,
                query_id: &sample.query_id,
                query: &sample.query,
                should_trigger: sample.should_trigger,
                secret: &ask.request.api_key,
                workspace: &ask.request.workspace,
            }) {
                Ok(row) => row.outcome == "hijacked" || row.outcome == "fired_with_others",
                Err(_) => false,
            }
        })
    } else {
        Vec::new()
    };
    Ok(json!({
        "snapshot": snapshot_path.display().to_string(),
        "changed": snapshot_changes(previous_snapshot.as_ref(), &snapshot),
        "new_skills": suspects,
        "warnings": warnings,
        "bisect": bisect,
        "broken_queries": failing.iter().map(|row| row.query_id.clone()).collect::<Vec<_>>(),
    }))
}

fn skill_pairs(document: &Value) -> Vec<(String, String)> {
    document
        .get("skills")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let name = row.get("name").and_then(Value::as_str)?.to_string();
                    let hash = row
                        .get("content_hash")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    Some((name, hash))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn new_skill_names(previous: Option<&Value>, current: &Value) -> Vec<String> {
    let Some(previous) = previous else {
        return Vec::new();
    };
    let old = skill_pairs(previous);
    skill_pairs(current)
        .into_iter()
        .filter(|(name, _)| old.iter().all(|(old_name, _)| old_name != name))
        .map(|(name, _)| name)
        .collect()
}

fn snapshot_changes(previous: Option<&Value>, current: &Value) -> Vec<String> {
    let Some(previous) = previous else {
        return Vec::new();
    };
    let old = skill_pairs(previous);
    skill_pairs(current)
        .into_iter()
        .filter(|(name, hash)| {
            old.iter()
                .any(|(old_name, old_hash)| old_name == name && old_hash != hash)
                || old.iter().all(|(old_name, _)| old_name != name)
        })
        .map(|(name, _)| name)
        .collect()
}

fn read_activation_file(path: &Path) -> Vec<ActivationRow> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&raw) else {
        return Vec::new();
    };
    value
        .as_array()
        .map(|rows| rows.iter().filter_map(activation_from_json).collect())
        .unwrap_or_default()
}

fn activation_from_json(value: &Value) -> Option<ActivationRow> {
    Some(ActivationRow {
        owner: text_field(value, "owner"),
        query_id: value.get("query_id").and_then(Value::as_str)?.to_string(),
        query: text_field(value, "query"),
        should_trigger: value
            .get("should_trigger")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        outcome: text_field(value, "outcome"),
        culprit: value
            .get("culprit")
            .and_then(Value::as_str)
            .map(str::to_string),
        ordered: value
            .get("ordered")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        adapter: text_field(value, "adapter"),
        model: text_field(value, "model"),
        strong: value.get("strong").and_then(Value::as_bool).unwrap_or(true),
        context: text_field(value, "context"),
        transcript: text_field(value, "transcript"),
    })
}

fn write_lookalikes(
    workspace: &Path,
    candidate: &LibrarySkill,
) -> Result<Vec<LibrarySkill>, String> {
    let dir = workspace
        .join("lookalikes")
        .join(format!("{}-near", candidate.name));
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    let description = format!("{} near miss", candidate.description);
    let skill = LibrarySkill {
        name: format!("{}-near", candidate.name),
        description: description.clone(),
        content_hash: "lookalike".to_string(),
        source_type: "generated".to_string(),
        source_ref: String::new(),
        source_revision: String::new(),
    };
    fs::write(
        dir.join("SKILL.md"),
        format!(
            "---\nname: {}\ndescription: {}\n---\n",
            skill.name, description
        ),
    )
    .map_err(|err| err.to_string())?;
    Ok(vec![skill])
}

fn write_measured_history(
    skill_dir: &Path,
    name: &str,
    with_rate: f64,
    without_rate: f64,
) -> Result<Value, String> {
    let with_wins = with_rate >= without_rate;
    let grading = if (with_rate - without_rate).abs() < f64::EPSILON {
        "tie"
    } else if with_rate > without_rate {
        "won"
    } else {
        "lost"
    };
    let entries = vec![
        HistoryEntry {
            version: "without_skill".to_string(),
            parent: None,
            expectation_pass_rate: without_rate,
            grading_result: "baseline".to_string(),
            is_current_best: !with_wins,
        },
        HistoryEntry {
            version: "with_skill".to_string(),
            parent: Some("without_skill".to_string()),
            expectation_pass_rate: with_rate,
            grading_result: grading.to_string(),
            is_current_best: with_wins,
        },
    ];
    write_history(&skill_dir.join("history.json"), name, &entries)
}

fn write_exports(
    workspace: &Path,
    skill: &str,
    suite: &Suite,
    report: &crate::testing::RunReport,
) -> Result<(), String> {
    let prompt = suite
        .evals
        .first()
        .map(|case| case.prompt.clone())
        .unwrap_or_default();
    let slug = slug_words(&prompt);
    let viewer = workspace.join("viewer").join(&slug);
    fs::create_dir_all(&viewer).map_err(|err| err.to_string())?;
    fs::write(
        viewer.join("eval_metadata.json"),
        serde_json::to_string_pretty(&json!({"eval_name": slug, "skill": skill, "prompt": prompt}))
            .map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())?;
    let iteration = workspace.join("iteration-1").join("eval-1");
    if iteration.is_dir() {
        fs::write(
            iteration.join("eval_metadata.json"),
            json!({"eval_id": 1, "eval_name": slug, "prompt": prompt}).to_string(),
        )
        .map_err(|err| err.to_string())?;
    }
    fs::write(
        workspace.join("feedback.json"),
        json!({"feedback": [{"skill": skill, "note": "graded from the measured transcript"}]})
            .to_string(),
    )
    .map_err(|err| err.to_string())?;
    let mut csv = String::from("configuration,pass_rate,status\n");
    let mut junit = String::from("<?xml version=\"1.0\"?><testsuite name=\"measured\">");
    let mut html = String::from("<!DOCTYPE html><html><body><h1>Measured review</h1><table><tr><th>configuration</th><th>pass rate</th></tr>");
    for cell in &report.cells {
        csv.push_str(&format!(
            "{},{},{}\n",
            cell.configuration, cell.pass_rate, cell.status
        ));
        junit.push_str(&format!("<testcase name=\"{}\" />", cell.configuration));
        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td></tr>",
            cell.configuration, cell.pass_rate
        ));
    }
    junit.push_str("</testsuite>");
    html.push_str("</table></body></html>");
    fs::write(workspace.join("cells.csv"), csv).map_err(|err| err.to_string())?;
    fs::write(workspace.join("junit.xml"), junit).map_err(|err| err.to_string())?;
    fs::write(workspace.join("review.html"), html).map_err(|err| err.to_string())?;
    Ok(())
}

fn read_gold(skill_dir: &Path) -> Result<Vec<GoldLabel>, String> {
    let path = skill_dir.join("evals").join("gold.json");
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let value: Value =
        serde_json::from_str(&fs::read_to_string(&path).map_err(|err| err.to_string())?)
            .map_err(|err| err.to_string())?;
    let rows = value
        .get("labels")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(rows
        .iter()
        .map(|row| GoldLabel {
            kind: text_field(row, "kind"),
            id: text_field(row, "id"),
            outcome: text_field(row, "outcome"),
            culprit: row
                .get("culprit")
                .and_then(Value::as_str)
                .map(str::to_string),
            passed: row.get("passed").and_then(Value::as_bool),
        })
        .collect())
}

fn expectation_pairs(workspace: &Path) -> Vec<(String, bool)> {
    let mut pairs = Vec::new();
    let root = workspace.join("iteration-1");
    if let Ok(evals) = fs::read_dir(&root) {
        for eval in evals.flatten() {
            if let Ok(configs) = fs::read_dir(eval.path()) {
                for config in configs.flatten() {
                    let path = config.path().join("grading.json");
                    if let Ok(text) = fs::read_to_string(&path) {
                        if let Ok(value) = serde_json::from_str::<Value>(&text) {
                            if let Some(rows) = value.get("expectations").and_then(Value::as_array)
                            {
                                for (index, row) in rows.iter().enumerate() {
                                    pairs.push((
                                        format!("{}-{index}", config.file_name().to_string_lossy()),
                                        row.get("passed").and_then(Value::as_bool).unwrap_or(false),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    pairs
}

fn activation_json(row: &ActivationRow) -> Value {
    json!({
        "owner": row.owner,
        "query_id": row.query_id,
        "query": row.query,
        "should_trigger": row.should_trigger,
        "outcome": row.outcome,
        "culprit": row.culprit,
        "ordered": row.ordered,
        "adapter": row.adapter,
        "model": row.model,
        "strong": row.strong,
        "context": row.context,
        "transcript": row.transcript,
    })
}

fn axis(name: &str, value: Option<f64>) -> Axis {
    Axis {
        name: name.to_string(),
        value,
    }
}

fn ratio(baseline: f64, used: f64) -> f64 {
    if used <= 0.0 {
        100.0
    } else {
        (baseline / used * 100.0).min(100.0)
    }
}

fn stddev(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / values.len() as f64;
    variance.sqrt()
}

fn text_field(row: &Value, key: &str) -> String {
    row.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn front_description(text: &str) -> String {
    text.lines()
        .find_map(|line| line.strip_prefix("description:"))
        .map(str::trim)
        .unwrap_or("")
        .to_string()
}

fn queries_for(name: &str, triggers: &[(String, Vec<String>)]) -> Vec<String> {
    triggers
        .iter()
        .find(|(skill, _)| skill == name)
        .map(|(_, queries)| queries.clone())
        .unwrap_or_default()
}

fn contains_ignore(haystack: &str, needle: &str) -> bool {
    haystack
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

fn shared_phrase(left: &str, right: &str) -> Option<String> {
    let left_words = words(left);
    let right_words = words(right);
    if left_words.len() < 3 || right_words.len() < 3 {
        return None;
    }
    for start in 0..=left_words.len() - 3 {
        let phrase = left_words[start..start + 3].join(" ");
        let joined = right_words.join(" ");
        if joined.contains(&phrase) {
            return Some(phrase);
        }
    }
    None
}

fn words(text: &str) -> Vec<String> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| word.len() > 2)
        .map(|word| word.to_ascii_lowercase())
        .collect()
}

fn slug_words(prompt: &str) -> String {
    let slug = words(prompt)
        .into_iter()
        .take(4)
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "eval".to_string()
    } else {
        slug
    }
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    for entry in fs::read_dir(from).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let name = entry.file_name();
        if name == "skill-snapshot" {
            continue;
        }
        let target = to.join(&name);
        if entry.path().is_dir() {
            fs::create_dir_all(&target).map_err(|err| err.to_string())?;
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target).map_err(|err| err.to_string())?;
        }
    }
    Ok(())
}

struct DeleteFile(PathBuf);

impl Drop for DeleteFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Save the suite a measured run expects. Callers that already have a suite skip this.
pub fn write_default_suite(skill_dir: &Path, name: &str, prompt: &str) -> Result<(), String> {
    let suite = Suite {
        skill_name: name.to_string(),
        evals: vec![SuiteCase {
            id: 1,
            prompt: prompt.to_string(),
            expected_output: "The answer file contains PINECONE".to_string(),
            files: Vec::new(),
            expectations: vec![
                "answer file exists".to_string(),
                "The answer file contains PINECONE".to_string(),
            ],
        }],
        triggers: vec![
            TriggerRow {
                query: format!("use {name} to file the report"),
                should_trigger: true,
                rationale: None,
            },
            TriggerRow {
                query: "should-not forecast the weather".to_string(),
                should_trigger: false,
                rationale: None,
            },
            TriggerRow {
                query: format!("hijack-target the {name} question"),
                should_trigger: true,
                rationale: None,
            },
        ],
        checks: vec![CheckSpec {
            id: "file".to_string(),
            expectation: "answer file exists".to_string(),
            kind: "file_exists".to_string(),
            path: "outputs/answer.txt".to_string(),
            pattern: None,
        }],
    };
    save_suite(skill_dir, &suite).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;

    #[test]
    fn scores_are_derived_without_calling_a_model() {
        assert_eq!(
            classify_activation("pine", &["pine".to_string()]),
            ("fired_alone".to_string(), None)
        );
        assert_eq!(
            classify_activation("pine", &["culprit".to_string()]).0,
            "hijacked"
        );
        assert_eq!(classify_activation("pine", &[]).0, "none");
        let axes = radar_axes(&RadarInput {
            trigger_pass_rate: Some(1.0),
            negative_pass_rate: Some(1.0),
            fired_alone_alone: Some(1.0),
            fired_alone_library: Some(0.5),
            with_rate: Some(1.0),
            without_rate: Some(0.5),
            repetition_rates: vec![1.0, 0.0],
            weak_pass_rate: None,
            with_tokens: Some(10.0),
            without_tokens: Some(10.0),
            with_time: Some(2.0),
            without_time: Some(2.0),
        });
        assert!(axes
            .iter()
            .any(|item| item.name == "Weak-model performance" && item.value.is_none()));
        let value = axes
            .iter()
            .find(|item| item.name == "Value added over without_skill")
            .and_then(|item| item.value)
            .unwrap();
        assert!((value - 75.0).abs() < 0.01);
        let (letter, _) = letter_grade(&axes);
        assert!(!letter.is_empty());
        assert!(strong_model_only(&[(true, 1.0), (false, 0.2)], 0.5));
        assert!(!strong_model_only(&[(true, 1.0)], 0.5));
        let ranked = rank_leaderboard(&[
            RankRow {
                skill: "low".into(),
                fires: 1.0,
                quality: 0.2,
                value_added: 90.0,
                efficiency: 90.0,
            },
            RankRow {
                skill: "high".into(),
                fires: 0.8,
                quality: 0.8,
                value_added: 10.0,
                efficiency: 10.0,
            },
        ]);
        assert_eq!(ranked[0].skill, "high");
        let found = bisect_distractors(&["a".into(), "b".into(), "c".into()], |set| {
            set.iter().any(|name| name == "b")
        });
        assert_eq!(found, vec!["b".to_string()]);
        let skills = vec![
            LibrarySkill {
                name: "pine".into(),
                description: "File the weekly pine report".into(),
                content_hash: "h".into(),
                source_type: "local".into(),
                source_ref: String::new(),
                source_revision: String::new(),
            },
            LibrarySkill {
                name: "other".into(),
                description: "File the weekly pine report for someone else".into(),
                content_hash: "h2".into(),
                source_type: "local".into(),
                source_ref: String::new(),
                source_revision: String::new(),
            },
        ];
        let hits = static_overlap(
            &skills,
            &[("pine".into(), vec!["file the weekly pine report".into()])],
        );
        assert!(!hits.is_empty());
        let document = export_snapshot(&skills);
        assert!(document.to_string().contains("content_hash"));
        assert!(!document.to_string().contains("secret"));
        let root = tempfile::tempdir().unwrap();
        let imported = import_snapshot(&document, &root.path().join("context")).unwrap();
        assert_eq!(imported.len(), 2);
        assert!(root
            .path()
            .join("context")
            .join("pine")
            .join("SKILL.md")
            .is_file());
        let before = ActivationRow {
            owner: "pine".into(),
            query_id: "q".into(),
            query: "q".into(),
            should_trigger: true,
            outcome: "fired_alone".into(),
            culprit: None,
            ordered: vec!["pine".into()],
            adapter: "emulated".into(),
            model: "m".into(),
            strong: true,
            context: "alone".into(),
            transcript: String::new(),
        };
        let mut after = before.clone();
        after.outcome = "hijacked".into();
        after.culprit = Some("other".into());
        let warnings = install_warnings(
            std::slice::from_ref(&before),
            std::slice::from_ref(&after),
            "other",
        );
        assert_eq!(warnings.len(), 1);
        let gold = calibrate_gold(
            &[GoldLabel {
                kind: "trigger".into(),
                id: "q".into(),
                outcome: "hijacked".into(),
                culprit: Some("different".into()),
                passed: None,
            }],
            &[before],
            &[],
        );
        assert!(gold["disagreements"].as_array().unwrap().len() == 1);
        let empty = calibrate_gold(&[], &[], &[]);
        assert_eq!(empty["requested"], false);
        let stream = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"pine"}}]}}"#;
        assert_eq!(
            ordered_from_stream(stream, &["pine".into()]),
            vec!["pine".to_string()]
        );
    }

    #[test]
    fn cli_trigger_deletes_the_command_file_on_every_exit() {
        let root = tempfile::tempdir().unwrap();
        let script = root.path().join("speak.py");
        fs::write(
            &script,
            "import sys\nprint('{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",\"name\":\"Skill\",\"input\":{\"skill\":\"pine\"}}]}}')\n",
        )
        .unwrap();
        let project = root.path().join("project");
        fs::create_dir_all(&project).unwrap();
        let program = python_exe();
        let stream = run_cli_trigger(
            &program,
            &[script.display().to_string()],
            &project,
            "pine",
            "use pine",
        )
        .unwrap();
        assert!(stream.contains("pine"));
        assert!(!project
            .join(".claude")
            .join("commands")
            .join("pine-skill-measure.md")
            .exists());
        let fail = root.path().join("fail.py");
        fs::write(&fail, "import sys\nsys.exit(1)\n").unwrap();
        let err = run_cli_trigger(
            &program,
            &[fail.display().to_string()],
            &project,
            "pine",
            "use pine",
        );
        assert!(err.is_err());
        assert!(!project
            .join(".claude")
            .join("commands")
            .join("pine-skill-measure.md")
            .exists());
    }

    #[test]
    fn measured_run_talks_to_the_anthropic_wire_and_writes_disk_and_rows() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let hits = Arc::new(AtomicUsize::new(0));
        let (port, flag) = serve(seen.clone(), hits.clone());
        let root = tempfile::tempdir().unwrap();
        let library = root.path().join("library");
        fs::create_dir_all(&library).unwrap();
        let skill = root.path().join("pine");
        fs::create_dir_all(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: pine\ndescription: File the weekly pine report\n---\nHIVE_SKILL_MARKER_9f3a\n",
        )
        .unwrap();
        write_default_suite(&skill, "pine", "Complete this task: file the token").unwrap();
        let workspace = root.path().join("ws");
        fs::create_dir_all(&workspace).unwrap();
        fs::write(
            workspace.join("library-snapshot.json"),
            r#"{"skills":[{"name":"pine","description":"File the weekly pine report","content_hash":"old","source_type":"local","source_ref":"","source_revision":""}]}"#,
        )
        .unwrap();
        fs::write(
            workspace.join("previous-activations.json"),
            r#"[{"owner":"pine","query_id":"q2","query":"hijack-target the pine question","should_trigger":true,"outcome":"fired_alone","ordered":["pine"],"adapter":"emulated","model":"claude-sonnet-5-5","strong":true,"context":"library","transcript":""}]"#,
        )
        .unwrap();
        fs::write(
            skill.join("evals").join("gold.json"),
            r#"{"labels":[{"kind":"trigger","id":"q2","outcome":"hijacked","culprit":"someone-else"}]}"#,
        )
        .unwrap();
        let script = root.path().join("speak.py");
        fs::write(
            &script,
            "import sys\nprint('{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",\"name\":\"Skill\",\"input\":{\"skill\":\"pine\"}}]}}')\n",
        )
        .unwrap();
        let request = MeasureRequest {
            skill_dir: skill.clone(),
            workspace: root.path().join("ws"),
            db_path: root.path().join("hive.db"),
            base_url: format!("http://127.0.0.1:{port}"),
            api_key: "sk-measured-secret-should-not-leak".into(),
            kind: "anthropic".into(),
            flavor: None,
            models: vec![
                ModelChoice {
                    id: "claude-sonnet-5-5".into(),
                    strong: true,
                },
                ModelChoice {
                    id: "weak-haiku".into(),
                    strong: false,
                },
            ],
            adapter: "emulated".into(),
            repetitions: 1,
            threshold: 0.5,
            pass_bar: 0.5,
            distractors: vec![LibrarySkill {
                name: "culprit".into(),
                description: "Answer someone else's pine questions".into(),
                content_hash: "c".into(),
                source_type: "local".into(),
                source_ref: String::new(),
                source_revision: String::new(),
            }],
            seed: 7,
            cli_program: Some(python_exe()),
            cli_prefix: vec![script.display().to_string()],
        };
        let report = run_measured(&request).unwrap();
        assert!(hits.load(Ordering::SeqCst) > 0);
        let bodies = seen.lock().unwrap().clone();
        assert!(bodies
            .iter()
            .any(|(path, headers, _)| path.contains("/v1/messages")
                && headers.to_ascii_lowercase().contains("anthropic-version")
                && headers.contains("x-api-key")));
        assert!(report["with_rate"].as_f64().unwrap() > report["without_rate"].as_f64().unwrap());
        assert!(root
            .path()
            .join("ws")
            .join("iteration-1")
            .join("benchmark.json")
            .is_file());
        assert!(root.path().join("ws").join("review.html").is_file());
        assert!(root.path().join("ws").join("cells.csv").is_file());
        assert!(root.path().join("ws").join("junit.xml").is_file());
        assert!(skill.join("history.json").is_file());
        assert!(skill.join("skill-snapshot").join("SKILL.md").is_file());
        assert!(root
            .path()
            .join("ws")
            .join("lookalikes")
            .join("pine-near")
            .join("SKILL.md")
            .is_file());
        assert!(!library.join("pine-near").exists());
        let hijack = report["activations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| {
                row["query"]
                    .as_str()
                    .unwrap_or("")
                    .contains("hijack-target")
                    && row["context"] == "library"
                    && row["model"] == "claude-sonnet-5-5"
            })
            .unwrap();
        assert_eq!(hijack["outcome"], "hijacked");
        assert_eq!(hijack["culprit"], "culprit");
        assert_eq!(hijack["adapter"], "emulated");
        assert!(!report["gold"]["disagreements"]
            .as_array()
            .unwrap()
            .is_empty());
        assert!(report["interference"]["bisect"]
            .as_array()
            .unwrap()
            .iter()
            .any(|name| name == "culprit"));
        assert!(report["interference"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["suspect"] == "culprit" && row["after"] == "hijacked"));
        assert!(root
            .path()
            .join("ws")
            .join("library-snapshot.json")
            .is_file());
        assert!(root
            .path()
            .join("ws")
            .join("context")
            .join("culprit")
            .join("SKILL.md")
            .is_file());
        assert!(root
            .path()
            .join("ws")
            .join("previous-activations.json")
            .is_file());
        assert!(report["influence"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["source"] == "culprit" && edge["target"] == "pine"));
        let conn = rusqlite::Connection::open(root.path().join("hive.db")).unwrap();
        let runs: i64 = conn
            .query_row("SELECT COUNT(*) FROM runs", [], |row| row.get(0))
            .unwrap();
        let cases: i64 = conn
            .query_row("SELECT COUNT(*) FROM run_cases", [], |row| row.get(0))
            .unwrap();
        let ledger: i64 = conn
            .query_row("SELECT COUNT(*) FROM usage_ledger", [], |row| row.get(0))
            .unwrap();
        assert!(runs >= 1 && cases >= 1 && ledger >= 1);
        let transcript = fs::read_to_string(
            root.path()
                .join("ws")
                .join("iteration-1")
                .join("eval-1")
                .join("with_skill")
                .join("transcript.jsonl"),
        )
        .unwrap_or_default();
        if transcript.is_empty() {
            let found = walk_contains(root.path(), "sk-measured-secret-should-not-leak");
            assert!(!found, "secret leaked into the workspace");
        } else {
            assert!(!transcript.contains("sk-measured-secret-should-not-leak"));
        }
        let provider = AnthropicProvider::new(
            "measured",
            format!("http://127.0.0.1:{port}"),
            SecretString::new("sk-measured-secret-should-not-leak"),
        );
        let questions = interview_goal(
            &provider,
            "claude-sonnet-5-5",
            "File the weekly pine report",
            "sk-measured-secret-should-not-leak",
        )
        .unwrap();
        assert!(questions["questions"].as_array().unwrap().len() >= 2);
        let rewrite = suggest_description(
            &provider,
            "claude-sonnet-5-5",
            "pine",
            "File the weekly pine report",
            "hijacked",
            "sk-measured-secret-should-not-leak",
        )
        .unwrap();
        assert!(rewrite.contains("narrowed-description"));
        let compared = blind_compare(&BlindRequest {
            provider: &provider,
            model: "claude-sonnet-5-5",
            prompt: "file the token",
            output_a: "alpha",
            output_b: "beta",
            sealed_a: "with_skill",
            sealed_b: "without_skill",
            secret: "sk-measured-secret-should-not-leak",
        })
        .unwrap();
        assert_eq!(compared["revealed"], false);
        assert_eq!(compared["winner"], "A");
        let recorded = seen.lock().unwrap().clone();
        let blind_body = recorded
            .iter()
            .find(|(_, _, body)| body.contains("HIVE_BLIND"))
            .map(|(_, _, body)| body.clone())
            .unwrap_or_default();
        assert!(!blind_body.contains("with_skill"));
        assert!(!blind_body.contains("without_skill"));
        let err = apply_description(&skill, &rewrite, false);
        assert!(err.is_err());
        apply_description(&skill, &rewrite, true).unwrap();
        let updated = fs::read_to_string(skill.join("SKILL.md")).unwrap();
        assert!(updated.contains("narrowed-description"));
        flag.store(false, Ordering::SeqCst);
        let _ = std::net::TcpStream::connect(format!("127.0.0.1:{port}"));
    }

    fn walk_contains(dir: &Path, needle: &str) -> bool {
        let Ok(entries) = fs::read_dir(dir) else {
            return false;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if walk_contains(&path, needle) {
                    return true;
                }
            } else if let Ok(text) = fs::read_to_string(&path) {
                if text.contains(needle) {
                    return true;
                }
            }
        }
        false
    }

    fn python_exe() -> PathBuf {
        for name in ["python3", "python"] {
            if Command::new(name)
                .arg("--version")
                .output()
                .map(|out| out.status.success())
                .unwrap_or(false)
            {
                return PathBuf::from(name);
            }
        }
        PathBuf::from("python3")
    }

    fn serve(
        seen: Arc<Mutex<Vec<(String, String, String)>>>,
        hits: Arc<AtomicUsize>,
    ) -> (u16, Arc<AtomicBool>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let flag = Arc::new(AtomicBool::new(true));
        let shutdown = flag.clone();
        thread::spawn(move || {
            while flag.load(Ordering::SeqCst) {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                if !flag.load(Ordering::SeqCst) {
                    break;
                }
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(800)));
                let mut buf = Vec::new();
                let mut tmp = [0_u8; 8192];
                loop {
                    match stream.read(&mut tmp) {
                        Ok(0) | Err(_) => break,
                        Ok(count) => {
                            buf.extend_from_slice(&tmp[..count]);
                            if buf.windows(4).any(|window| window == b"\r\n\r\n") {
                                break;
                            }
                        }
                    }
                }
                let header_end = buf
                    .windows(4)
                    .position(|window| window == b"\r\n\r\n")
                    .unwrap_or(buf.len());
                let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
                let need = head
                    .lines()
                    .find_map(|line| {
                        line.split_once(':').and_then(|(name, value)| {
                            if name.eq_ignore_ascii_case("content-length") {
                                value.trim().parse::<usize>().ok()
                            } else {
                                None
                            }
                        })
                    })
                    .unwrap_or(0);
                let mut body = if buf.len() > header_end + 4 {
                    buf[header_end + 4..].to_vec()
                } else {
                    Vec::new()
                };
                while body.len() < need {
                    match stream.read(&mut tmp) {
                        Ok(0) | Err(_) => break,
                        Ok(count) => body.extend_from_slice(&tmp[..count]),
                    }
                }
                let text = String::from_utf8_lossy(&body).to_string();
                let path = head
                    .lines()
                    .next()
                    .unwrap_or("")
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("")
                    .to_string();
                hits.fetch_add(1, Ordering::SeqCst);
                seen.lock()
                    .unwrap()
                    .push((path.clone(), head.clone(), text.clone()));
                let payload = fixture_sse(&path, &text);
                let packet = format!(
                    "HTTP/1.1 200 TEST\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                    payload.len()
                );
                let _ = stream.write_all(packet.as_bytes());
            }
        });
        (port, shutdown)
    }

    fn fixture_sse(path: &str, request: &str) -> String {
        if path.contains("chat/completions") {
            return openai_text("openai");
        }
        if request.contains("HIVE_TRIGGER") {
            if request.contains("should-not") || request.contains("weak-haiku") {
                return sse_text("none");
            }
            if request.contains("narrowed-description") {
                return sse_skill("pine");
            }
            if request.contains("hijack-target") && request.contains("culprit") {
                return sse_skill("culprit");
            }
            return sse_skill("pine");
        }
        if request.contains("HIVE_INTERVIEW") {
            return sse_text(
                r#"{"assumed_name":"pine","questions":["Who is the reader?","Which files matter?"],"skipped":[]}"#,
            );
        }
        if request.contains("HIVE_BLIND") {
            return sse_text(r#"{"winner":"A"}"#);
        }
        if request.contains("HIVE_REWRITE") {
            return sse_text(
                r#"{"description":"narrowed-description File the weekly pine report","changes":["narrowed"]}"#,
            );
        }
        if request.contains("GRADE") {
            let passed = request.matches("PINECONE").count() > 1;
            return sse_text(&format!(
                r#"{{"expectations":[{{"text":"The answer file contains PINECONE","passed":{passed},"evidence":"transcript"}}]}}"#
            ));
        }
        if request.contains("tool_result") {
            let text = if request.contains("HIVE_SKILL_MARKER_9f3a") {
                "Wrote PINECONE."
            } else {
                "plain"
            };
            return sse_text(text);
        }
        let content = if request.contains("HIVE_SKILL_MARKER_9f3a") {
            "PINECONE"
        } else {
            "plain"
        };
        sse_tool(content)
    }

    fn sse_text(text: &str) -> String {
        format!(
            "data: {{\"type\":\"message_start\",\"message\":{{\"id\":\"m\",\"usage\":{{\"input_tokens\":4}}}}}}\n\ndata: {{\"type\":\"content_block_delta\",\"delta\":{{\"type\":\"text_delta\",\"text\":{}}}}}\n\ndata: {{\"type\":\"message_delta\",\"delta\":{{\"stop_reason\":\"end_turn\"}},\"usage\":{{\"output_tokens\":2}}}}\n\ndata: {{\"type\":\"message_stop\"}}\n\n",
            serde_json::to_string(text).unwrap()
        )
    }

    fn sse_tool(content: &str) -> String {
        let partial =
            serde_json::json!({"path": "outputs/answer.txt", "content": content}).to_string();
        format!(
            "data: {{\"type\":\"message_start\",\"message\":{{\"id\":\"m\",\"usage\":{{\"input_tokens\":4}}}}}}\n\ndata: {{\"type\":\"content_block_start\",\"content_block\":{{\"type\":\"tool_use\",\"id\":\"tool-1\",\"name\":\"write_file\"}}}}\n\ndata: {{\"type\":\"content_block_delta\",\"delta\":{{\"type\":\"input_json_delta\",\"partial_json\":{}}}}}\n\ndata: {{\"type\":\"content_block_stop\"}}\n\ndata: {{\"type\":\"message_delta\",\"delta\":{{\"stop_reason\":\"tool_use\"}},\"usage\":{{\"output_tokens\":3}}}}\n\ndata: {{\"type\":\"message_stop\"}}\n\n",
            serde_json::to_string(&partial).unwrap()
        )
    }

    fn sse_skill(name: &str) -> String {
        let partial = serde_json::json!({"skill": name}).to_string();
        format!(
            "data: {{\"type\":\"message_start\",\"message\":{{\"id\":\"m\",\"usage\":{{\"input_tokens\":3}}}}}}\n\ndata: {{\"type\":\"content_block_start\",\"content_block\":{{\"type\":\"tool_use\",\"id\":\"skill-1\",\"name\":\"Skill\"}}}}\n\ndata: {{\"type\":\"content_block_delta\",\"delta\":{{\"type\":\"input_json_delta\",\"partial_json\":{}}}}}\n\ndata: {{\"type\":\"content_block_stop\"}}\n\ndata: {{\"type\":\"message_delta\",\"delta\":{{\"stop_reason\":\"tool_use\"}},\"usage\":{{\"output_tokens\":2}}}}\n\ndata: {{\"type\":\"message_stop\"}}\n\n",
            serde_json::to_string(&partial).unwrap()
        )
    }

    fn openai_text(text: &str) -> String {
        format!(
            "data: {{\"id\":\"c\",\"choices\":[{{\"delta\":{{\"content\":{}}}}}]}}\n\ndata: {{\"id\":\"c\",\"choices\":[{{\"finish_reason\":\"stop\"}}],\"usage\":{{\"prompt_tokens\":2,\"completion_tokens\":1}}}}\n\ndata: [DONE]\n\n",
            serde_json::to_string(text).unwrap()
        )
    }
}
