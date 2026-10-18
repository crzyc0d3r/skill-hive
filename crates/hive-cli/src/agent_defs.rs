//! Saved agent definitions: list, show, deploy, and undeploy.

use std::fs;

use hive_core::library::central_repo;
use serde_json::{json, Value};

use crate::Fail;

pub fn agent_def_list() -> Result<Value, Fail> {
    let defs = central_repo::base_dir().join("agent-defs");
    let mut agents = Vec::new();
    if defs.is_dir() {
        for entry in fs::read_dir(&defs).map_err(|err| Fail::other(err.to_string()))? {
            let entry = entry.map_err(|err| Fail::other(err.to_string()))?;
            if entry.path().extension().and_then(|ext| ext.to_str()) == Some("md") {
                if let Some(name) = entry.path().file_stem().and_then(|stem| stem.to_str()) {
                    agents.push(name.to_string());
                }
            }
        }
    }
    agents.sort();
    Ok(json!({"command": "agents.def.list", "agents": agents}))
}
pub fn agent_def_show(name: &str) -> Result<Value, Fail> {
    let path = central_repo::base_dir()
        .join("agent-defs")
        .join(format!("{name}.md"));
    if !path.is_file() {
        return Err(Fail::other(format!("unknown agent {name}")));
    }
    let text = fs::read_to_string(&path).map_err(|err| Fail::other(err.to_string()))?;
    Ok(json!({"command": "agents.def.show", "name": name, "text": text}))
}
pub fn agent_def_deploy(name: &str) -> Result<Value, Fail> {
    let source = central_repo::base_dir()
        .join("agent-defs")
        .join(format!("{name}.md"));
    if !source.is_file() {
        return Err(Fail::other(format!("unknown agent {name}")));
    }
    let dest_dir = central_repo::base_dir().join("deployed-agents");
    fs::create_dir_all(&dest_dir).map_err(|err| Fail::other(err.to_string()))?;
    fs::copy(&source, dest_dir.join(format!("{name}.md")))
        .map_err(|err| Fail::other(err.to_string()))?;
    Ok(json!({"command": "agents.def.deploy", "name": name}))
}
pub fn agent_def_undeploy(name: &str) -> Result<Value, Fail> {
    let path = central_repo::base_dir()
        .join("deployed-agents")
        .join(format!("{name}.md"));
    if path.is_file() {
        fs::remove_file(&path).map_err(|err| Fail::other(err.to_string()))?;
    }
    Ok(json!({"command": "agents.def.undeploy", "name": name}))
}
