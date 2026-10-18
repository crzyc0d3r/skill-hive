//! Agent drafts, camelCase frontmatter, export, and the `--agents` JSON.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::frontmatter::{estimate_tokens, parse_document, Document, Scalar};
use super::validator::{validate_agent, Finding};
use crate::util::error::{AppError, TargetConflictDetail};

#[derive(Debug, Clone)]
pub struct AgentExportRequest<'a> {
    pub source: &'a Path,
    pub target: &'a Path,
    pub expected_hash: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentExport {
    pub path: PathBuf,
    pub hash: String,
    pub bytes: Vec<u8>,
}

pub fn render_agent_file(text: &str) -> Result<String, AppError> {
    let doc = parse_document(text);
    if doc.yaml_error.is_some() || !doc.closed {
        return Err(AppError::invalid_input(
            "agent frontmatter is not a closed mapping",
        ));
    }
    Ok(serialize_agent(&doc))
}

pub fn render_agents_cli_json(text: &str) -> Result<Value, AppError> {
    let doc = parse_document(text);
    let name = doc
        .string("name")
        .ok_or_else(|| AppError::invalid_input("agent name is required"))?;
    let mut fields = serde_json::Map::new();
    if let Some(description) = doc.string("description") {
        fields.insert("description".into(), json!(description));
    }
    fields.insert(
        "prompt".into(),
        json!(doc.body.trim_end_matches(['\n', '\r'])),
    );
    for key in &doc.keys {
        if key.name == "name" || key.name == "description" {
            continue;
        }
        fields.insert(key.name.clone(), scalar_json(&key.value));
    }
    Ok(json!({ name: Value::Object(fields) }))
}

pub fn export_agent(request: AgentExportRequest<'_>) -> Result<AgentExport, AppError> {
    let text = fs::read_to_string(request.source)?;
    let rendered = render_agent_file(&text)?;
    let bytes = rendered.into_bytes();
    let hash = sha256(&bytes);
    if request.target.exists() {
        let existing = fs::read(request.target).unwrap_or_default();
        let existing_hash = sha256(&existing);
        let owned = request.expected_hash == Some(existing_hash.as_str()) || existing == bytes;
        if !owned {
            return Err(AppError::target_conflict(
                format!(
                    "Refused to replace {}. The content was left untouched.",
                    request.target.display()
                ),
                vec![TargetConflictDetail {
                    path: request.target.display().to_string(),
                    reason: "foreign file".into(),
                }],
            ));
        }
    } else if let Some(parent) = request.target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(request.target, &bytes)?;
    Ok(AgentExport {
        path: request.target.to_path_buf(),
        hash,
        bytes,
    })
}

pub fn export_status(
    draft_hash: &str,
    exported_hash: Option<&str>,
    disk_hash: Option<&str>,
) -> &'static str {
    match (exported_hash, disk_hash) {
        (_, None) => "missing",
        (Some(exported), Some(disk)) if exported != disk => "modified",
        (Some(exported), Some(_)) if exported != draft_hash => "stale",
        (Some(_), Some(_)) => "current",
        (None, Some(_)) => "modified",
    }
}

pub fn validate_agent_file(path: &Path, plugin_mode: bool) -> Vec<Finding> {
    validate_agent(path, plugin_mode)
}

pub fn apply_agent_fix(path: &Path, fix_id: &str) -> Result<bool, AppError> {
    let text = fs::read_to_string(path)?;
    let mut doc = parse_document(&text);
    let changed = match fix_id {
        "rename-key" => {
            let mut any = false;
            for (from, to) in [
                ("disallowed_tools", "disallowedTools"),
                ("permission_mode", "permissionMode"),
                ("max_turns", "maxTurns"),
                ("mcp_servers", "mcpServers"),
                ("omit_claude_md", "omitClaudeMd"),
                ("initial_prompt", "initialPrompt"),
            ] {
                if doc.rename_key(from, to) {
                    any = true;
                }
            }
            any
        }
        "move-skill-tool" => move_skill_tool(&mut doc),
        _ => false,
    };
    if changed {
        let reparsed = parse_document(&doc.serialize());
        fs::write(path, serialize_agent(&reparsed))?;
    }
    Ok(changed)
}

pub fn agent_description_budget(descriptions: &[&str]) -> usize {
    estimate_tokens(&descriptions.join("\n"))
}

fn move_skill_tool(doc: &mut Document) -> bool {
    let Some(tools) = doc.get("tools").cloned() else {
        return false;
    };
    let names: Vec<String> = match tools {
        Scalar::Seq(items) => items,
        Scalar::Str { text, .. } => text
            .split(',')
            .map(|part| part.trim().to_string())
            .filter(|part| !part.is_empty())
            .collect(),
        _ => return false,
    };
    if !names.iter().any(|name| name == "Skill") {
        return false;
    }
    let kept: Vec<String> = names.into_iter().filter(|name| name != "Skill").collect();
    let line = doc
        .keys
        .iter()
        .find(|key| key.name == "tools")
        .map(|key| key.start_line);
    if let Some(start) = line {
        doc.lines_set(start, format!("tools: {}", kept.join(", ")));
    }
    if doc.get("skills").is_none() {
        doc.insert_plain("skills", "Skill");
    }
    true
}

fn serialize_agent(doc: &Document) -> String {
    let mut out = String::from("---\n");
    for key in &doc.keys {
        out.push_str(&render_key(&key.name, &key.value));
        out.push('\n');
    }
    out.push_str("---\n");
    out.push_str(&doc.body);
    if !doc.body.is_empty() && !doc.body.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn render_key(name: &str, value: &Scalar) -> String {
    match value {
        Scalar::Str { text, .. } => format!("{name}: {text}"),
        Scalar::Bool(value) => format!("{name}: {value}"),
        Scalar::Int(value) => format!("{name}: {value}"),
        Scalar::Null => format!("{name}:"),
        Scalar::Seq(items) => format!("{name}: {}", items.join(", ")),
        Scalar::Map(entries) => {
            let mut text = format!("{name}:");
            for (key, value) in entries {
                let value = value.strip_prefix('\u{1}').unwrap_or(value);
                text.push_str(&format!("\n  {key}: {value}"));
            }
            text
        }
        Scalar::Invalid(text) => format!("{name}: {text}"),
    }
}

fn scalar_json(value: &Scalar) -> Value {
    match value {
        Scalar::Str { text, .. } => json!(text),
        Scalar::Bool(value) => json!(value),
        Scalar::Int(value) => json!(value),
        Scalar::Null => Value::Null,
        Scalar::Seq(items) => json!(items),
        Scalar::Map(entries) => {
            let mut map = serde_json::Map::new();
            for (key, value) in entries {
                let value = value.strip_prefix('\u{1}').unwrap_or(value);
                map.insert(key.clone(), json!(value));
            }
            Value::Object(map)
        }
        Scalar::Invalid(text) => json!(text),
    }
}

fn sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

impl Document {
    fn lines_set(&mut self, index: usize, text: String) {
        if let Some(line) = self.lines.get_mut(index) {
            line.text = text;
        }
        self.original = self.serialize();
    }

    fn insert_plain(&mut self, name: &str, value: &str) {
        let insert_at = self.keys.last().map(|key| key.end_line + 1).unwrap_or(1);
        self.lines.insert(
            insert_at,
            super::frontmatter::SrcLine {
                text: format!("{name}: {value}"),
                nl: "\n".to_string(),
            },
        );
        self.original = self.serialize();
        *self = parse_document(&self.original);
    }
}
