//! Skill, agent, and eval generation from recorded fixtures.
//! A live call is refused because no vendor key is configured.

use std::fs;

use hive_core::authoring::Profile;
use hive_core::authoring::{validate_agent, Severity};
use hive_core::generation::{
    agent_envelope, draft_trigger_set, skill_envelope, GenerationSession, Kind,
};
use hive_core::library::central_repo;
use serde_json::{json, Value};

use crate::Fail;

pub fn generate_skill(name: &str, replay: bool) -> Result<Value, Fail> {
    refuse_live(replay)?;
    let root = central_repo::base_dir().join("generations");
    let mut session = GenerationSession::start(
        &root,
        None,
        Kind::Skill,
        "Extract line items from supplier invoice PDFs into a CSV",
        name,
        Profile::Portable,
    )
    .map_err(Fail::other)?;
    session
        .run_review(&skill_envelope(name, false))
        .map_err(|err| Fail::other(format!("{err:?}")))?;
    Ok(json!({
        "command": "generate.skill",
        "name": name,
        "status": "review",
        "errors": session.findings_errors
    }))
}
pub fn generate_agent(name: &str, replay: bool) -> Result<Value, Fail> {
    refuse_live(replay)?;
    let root = central_repo::base_dir().join("generations");
    let mut session = GenerationSession::start(
        &root,
        None,
        Kind::Agent,
        "Review a diff for security problems",
        name,
        Profile::Portable,
    )
    .map_err(Fail::other)?;
    session
        .run_review(&agent_envelope(name))
        .map_err(|err| Fail::other(format!("{err:?}")))?;
    let saved = session
        .accept()
        .map_err(|err| Fail::other(format!("{err:?}")))?;
    let errors = validate_agent(&saved.join("agent.md"), false)
        .iter()
        .filter(|finding| finding.severity == Severity::Error)
        .count();
    let defs = central_repo::base_dir().join("agent-defs");
    fs::create_dir_all(&defs).map_err(|err| Fail::other(err.to_string()))?;
    fs::copy(saved.join("agent.md"), defs.join(format!("{name}.md")))
        .map_err(|err| Fail::other(err.to_string()))?;
    Ok(json!({
        "command": "generate.agent",
        "name": name,
        "status": "saved",
        "errors": errors
    }))
}
pub fn generate_evals(name: &str, replay: bool) -> Result<Value, Fail> {
    refuse_live(replay)?;
    let root = central_repo::base_dir().join("generations").join(name);
    fs::create_dir_all(root.join("evals")).map_err(|err| Fail::other(err.to_string()))?;
    let skill_text = fs::read_to_string(root.join("SKILL.md")).unwrap_or_default();
    let mut seeds: Vec<String> = skill_text
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric())
                .to_string()
        })
        .filter(|word| word.len() > 2)
        .collect();
    if seeds.is_empty() {
        seeds.push(name.to_string());
    }
    let mut rows = Vec::new();
    while rows.len() < 16 {
        let seed = &seeds[rows.len() % seeds.len()];
        rows.push(json!({
            "query": format!("{seed} {name}"),
            "should_trigger": rows.len() < 8
        }));
        if rows.len() == seeds.len() * 2 && seeds.len() < 8 {
            seeds.push(format!("{name}-{}", rows.len()));
        }
    }
    let parsed = draft_trigger_set(&Value::Array(rows).to_string()).map_err(Fail::other)?;
    let document = serde_json::to_string_pretty(&parsed).unwrap_or_else(|_| "[]".into());
    fs::write(root.join("evals").join("trigger.json"), document)
        .map_err(|err| Fail::other(err.to_string()))?;
    Ok(json!({
        "command": "generate.evals",
        "name": name,
        "queries": parsed.len()
    }))
}

fn refuse_live(replay: bool) -> Result<(), Fail> {
    if replay {
        Ok(())
    } else {
        Err(Fail::other(
            "live generation is unavailable without a vendor API key; pass --replay",
        ))
    }
}
