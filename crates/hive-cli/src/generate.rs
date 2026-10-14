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
    let mut rows = Vec::new();
    for index in 0..16 {
        rows.push(json!({
            "query": format!("recorded query {index} for {name}"),
            "should_trigger": index < 8
        }));
    }
    let parsed = draft_trigger_set(&Value::Array(rows).to_string()).map_err(Fail::other)?;
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
