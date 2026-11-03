//! Parsing and validation of skills, subagent definitions and slash commands.
//!
//! The skill rules follow the Agent Skills specification (<https://agentskills.io/specification>)
//! and the conventions Claude Code, Codex and Cursor share: a directory with a `SKILL.md`
//! whose YAML frontmatter has at least `name` and `description`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::{HiveError, IoContext, Result};
use crate::model::{
    AssetKind, Finding, ParsedDocument, Severity, SkillFrontmatter, ValidationReport,
};

pub const SKILL_FILE_NAMES: [&str; 2] = ["SKILL.md", "skill.md"];
pub const MAX_NAME_LEN: usize = 64;
pub const MAX_DESCRIPTION_LEN: usize = 1024;
pub const MAX_COMPATIBILITY_LEN: usize = 500;
pub const RECOMMENDED_MAX_BODY_LINES: usize = 500;
const KNOWN_SKILL_KEYS: [&str; 6] = [
    "name",
    "description",
    "license",
    "allowed-tools",
    "compatibility",
    "metadata",
];

/// Locate the skill file inside a directory (`SKILL.md` preferred over `skill.md`).
pub fn skill_file(dir: &Path) -> Option<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return None;
    };
    let names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    SKILL_FILE_NAMES
        .iter()
        .find(|candidate| names.iter().any(|n| n == *candidate))
        .map(|n| dir.join(n))
}

pub fn is_skill_dir(dir: &Path) -> bool {
    dir.is_dir() && skill_file(dir).is_some()
}

/// Split a markdown document into its YAML frontmatter (if any) and body.
pub fn split_frontmatter(text: &str) -> Result<(Option<String>, String)> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text.split_inclusive('\n');
    let Some(first) = lines.next() else {
        return Ok((None, String::new()));
    };
    if first.trim_end() != "---" {
        return Ok((None, text.to_string()));
    }
    let mut yaml = String::new();
    let mut body_start = None;
    let mut offset = first.len();
    for line in lines {
        let trimmed = line.trim_end();
        if trimmed == "---" || trimmed == "..." {
            body_start = Some(offset + line.len());
            break;
        }
        yaml.push_str(line);
        offset += line.len();
    }
    match body_start {
        Some(start) => Ok((Some(yaml), text[start..].to_string())),
        None => Err(HiveError::invalid(
            "frontmatter is not closed: expected a line with `---`",
        )),
    }
}

fn yaml_to_json(value: serde_yaml_ng::Value) -> Result<serde_json::Value> {
    serde_json::to_value(value).map_err(|e| {
        HiveError::invalid(format!(
            "frontmatter is not valid JSON-compatible YAML: {e}"
        ))
    })
}

/// Parse frontmatter YAML into a JSON object. Empty frontmatter yields an empty object.
pub fn parse_frontmatter_json(yaml: &str) -> Result<serde_json::Map<String, serde_json::Value>> {
    if yaml.trim().is_empty() {
        return Ok(serde_json::Map::new());
    }
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml)
        .map_err(|e| HiveError::invalid(format!("frontmatter YAML error: {e}")))?;
    match yaml_to_json(value)? {
        serde_json::Value::Object(map) => Ok(map),
        serde_json::Value::Null => Ok(serde_json::Map::new()),
        _ => Err(HiveError::invalid("frontmatter must be a YAML mapping")),
    }
}

fn json_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// `allowed-tools` accepts a whitespace separated string or a list.
fn parse_tool_list(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::String(s) => s.split_whitespace().map(|t| t.to_string()).collect(),
        serde_json::Value::Array(items) => items.iter().filter_map(json_string).collect(),
        _ => Vec::new(),
    }
}

/// Build a [`SkillFrontmatter`] from a parsed frontmatter object, keeping unknown keys.
pub fn skill_frontmatter_from_json(
    map: &serde_json::Map<String, serde_json::Value>,
) -> SkillFrontmatter {
    let mut fm = SkillFrontmatter::default();
    for (key, value) in map {
        match key.as_str() {
            "name" => fm.name = json_string(value).unwrap_or_default().trim().to_string(),
            "description" => {
                fm.description = json_string(value).unwrap_or_default().trim().to_string()
            }
            "license" => fm.license = json_string(value),
            "compatibility" => fm.compatibility = json_string(value),
            "allowed-tools" => fm.allowed_tools = parse_tool_list(value),
            "metadata" => {
                if let serde_json::Value::Object(meta) = value {
                    fm.metadata = meta
                        .iter()
                        .map(|(k, v)| (k.clone(), json_string(v).unwrap_or_else(|| v.to_string())))
                        .collect::<BTreeMap<_, _>>();
                } else {
                    fm.extra.insert(key.clone(), value.clone());
                }
            }
            _ => {
                fm.extra.insert(key.clone(), value.clone());
            }
        }
    }
    fm
}

/// Parse the text of a `SKILL.md`.
pub fn parse_skill_text(text: &str) -> Result<ParsedDocument> {
    let (yaml, body) = split_frontmatter(text)?;
    let yaml = yaml.ok_or_else(|| {
        HiveError::invalid("SKILL.md has no frontmatter (expected `---` on the first line)")
    })?;
    let map = parse_frontmatter_json(&yaml)?;
    let fm = skill_frontmatter_from_json(&map);
    Ok(ParsedDocument {
        kind: AssetKind::Skill,
        name: fm.name.clone(),
        description: fm.description.clone(),
        frontmatter: serde_json::Value::Object(map),
        body,
    })
}

/// Parse an agent or command markdown file. Frontmatter is optional for commands.
pub fn parse_markdown_asset(
    kind: AssetKind,
    text: &str,
    fallback_name: &str,
) -> Result<ParsedDocument> {
    let (yaml, body) = split_frontmatter(text)?;
    let map = match yaml {
        Some(y) => parse_frontmatter_json(&y)?,
        None => serde_json::Map::new(),
    };
    let name = map
        .get("name")
        .and_then(json_string)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| fallback_name.to_string());
    let description = map
        .get("description")
        .and_then(json_string)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| first_paragraph(&body));
    Ok(ParsedDocument {
        kind,
        name,
        description,
        frontmatter: serde_json::Value::Object(map),
        body,
    })
}

/// First non-heading paragraph of a markdown body, collapsed to one line.
pub fn first_paragraph(body: &str) -> String {
    let mut out = String::new();
    for line in body.lines() {
        let t = line.trim();
        if t.is_empty() {
            if !out.is_empty() {
                break;
            }
            continue;
        }
        if t.starts_with('#') || t.starts_with("```") {
            if !out.is_empty() {
                break;
            }
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(t);
        if out.len() > 300 {
            break;
        }
    }
    crate::util::truncate_chars(&out, 300)
}

/// Read and parse the skill in `dir`.
pub fn read_skill_dir(dir: &Path) -> Result<ParsedDocument> {
    let file = skill_file(dir)
        .ok_or_else(|| HiveError::not_found(format!("no SKILL.md in {}", dir.display())))?;
    let text = std::fs::read_to_string(&file).at(&file)?;
    parse_skill_text(&text)
}

/// Read and parse an agent or command file.
pub fn read_markdown_asset(kind: AssetKind, file: &Path) -> Result<ParsedDocument> {
    let text = std::fs::read_to_string(file).at(file)?;
    let stem = file
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    parse_markdown_asset(kind, &text, &stem)
}

/// Read any asset given its kind and path (directory for skills, file otherwise).
pub fn read_asset(kind: AssetKind, path: &Path) -> Result<ParsedDocument> {
    match kind {
        AssetKind::Skill => read_skill_dir(path),
        other => read_markdown_asset(other, path),
    }
}

/// The name rule: lowercase letters, digits and single hyphens, 1–64 characters.
pub fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_NAME_LEN
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn finding(
    code: &str,
    severity: Severity,
    message: impl Into<String>,
    field: Option<&str>,
) -> Finding {
    Finding {
        code: code.to_string(),
        severity,
        message: message.into(),
        field: field.map(|f| f.to_string()),
    }
}

/// Validate a parsed skill. `dir_name` is the directory the skill lives in, when known.
pub fn validate_skill(doc: &ParsedDocument, dir_name: Option<&str>) -> ValidationReport {
    let mut findings = Vec::new();
    let map = doc.frontmatter.as_object().cloned().unwrap_or_default();
    let fm = skill_frontmatter_from_json(&map);

    // name
    if fm.name.is_empty() {
        findings.push(finding(
            "name.missing",
            Severity::Error,
            "`name` is required",
            Some("name"),
        ));
    } else if !is_valid_name(&fm.name) {
        findings.push(finding(
            "name.invalid",
            Severity::Error,
            format!(
                "`name` must be 1–{MAX_NAME_LEN} lowercase letters, digits or single hyphens (got `{}`)",
                fm.name
            ),
            Some("name"),
        ));
    }
    if let Some(dir) = dir_name {
        if !fm.name.is_empty() && fm.name != dir {
            findings.push(finding(
                "name.dir_mismatch",
                Severity::Warning,
                format!(
                    "`name` (`{}`) should match the directory name (`{dir}`)",
                    fm.name
                ),
                Some("name"),
            ));
        }
    }

    // description
    if fm.description.is_empty() {
        findings.push(finding(
            "description.missing",
            Severity::Error,
            "`description` is required: agents decide whether to load a skill from it",
            Some("description"),
        ));
    } else {
        let len = fm.description.chars().count();
        if len > MAX_DESCRIPTION_LEN {
            // Real collections exceed the specification's limit; agents still load them.
            findings.push(finding(
                "description.too_long",
                Severity::Warning,
                format!("`description` is {len} characters; the limit is {MAX_DESCRIPTION_LEN}"),
                Some("description"),
            ));
        } else if len < 20 {
            findings.push(finding(
                "description.too_short",
                Severity::Warning,
                "`description` is very short; say what the skill does and when to use it",
                Some("description"),
            ));
        }
        let lower = fm.description.to_lowercase();
        if !(lower.contains("use when")
            || lower.contains("use this")
            || lower.contains("when ")
            || lower.contains("trigger")
            || lower.contains("for "))
        {
            findings.push(finding(
                "description.no_trigger",
                Severity::Info,
                "Consider stating when the skill should be used (\"Use when …\"): it improves triggering",
                Some("description"),
            ));
        }
    }

    if let Some(compat) = &fm.compatibility {
        if compat.chars().count() > MAX_COMPATIBILITY_LEN {
            findings.push(finding(
                "compatibility.too_long",
                Severity::Warning,
                format!("`compatibility` exceeds {MAX_COMPATIBILITY_LEN} characters"),
                Some("compatibility"),
            ));
        }
    }

    if let Some(serde_json::Value::Object(meta)) = map.get("metadata") {
        for (k, v) in meta {
            if !v.is_string() && !v.is_number() && !v.is_boolean() {
                findings.push(finding(
                    "metadata.non_scalar",
                    Severity::Warning,
                    format!("`metadata.{k}` should be a string"),
                    Some("metadata"),
                ));
            }
        }
    } else if map.get("metadata").is_some() {
        findings.push(finding(
            "metadata.invalid",
            Severity::Warning,
            "`metadata` should be a mapping",
            Some("metadata"),
        ));
    }

    if let Some(v) = map.get("allowed-tools") {
        if !(v.is_string() || v.is_array()) {
            findings.push(finding(
                "allowed_tools.invalid",
                Severity::Warning,
                "`allowed-tools` should be a space separated string or a list",
                Some("allowed-tools"),
            ));
        }
    }

    for key in map.keys() {
        if !KNOWN_SKILL_KEYS.contains(&key.as_str()) {
            findings.push(finding(
                "frontmatter.unknown_key",
                Severity::Warning,
                format!("Unknown frontmatter key `{key}` (agents ignore it; `metadata` holds custom fields)"),
                Some(key),
            ));
        }
    }

    // body
    let body_trimmed = doc.body.trim();
    if body_trimmed.is_empty() {
        findings.push(finding(
            "body.empty",
            Severity::Error,
            "The skill has no instructions after the frontmatter",
            None,
        ));
    } else {
        let lines = doc.body.lines().count();
        if lines > RECOMMENDED_MAX_BODY_LINES {
            findings.push(finding(
                "body.long",
                Severity::Info,
                format!("SKILL.md is {lines} lines; keep it under {RECOMMENDED_MAX_BODY_LINES} and move detail into reference files"),
                None,
            ));
        }
        if body_trimmed.lines().all(|l| l.trim().starts_with('#')) {
            findings.push(finding(
                "body.only_headings",
                Severity::Warning,
                "The body contains only headings",
                None,
            ));
        }
        if doc.body.contains("](../") || doc.body.contains(" ../") {
            findings.push(finding(
                "body.parent_path",
                Severity::Warning,
                "The body references a path outside the skill directory (`../`); bundled files should live inside the skill",
                None,
            ));
        }
    }

    ValidationReport { findings }
}

/// Validate a subagent definition (Claude Code style frontmatter).
pub fn validate_agent(doc: &ParsedDocument) -> ValidationReport {
    let mut findings = Vec::new();
    let map = doc.frontmatter.as_object().cloned().unwrap_or_default();
    match map.get("name").and_then(json_string) {
        None => findings.push(finding(
            "name.missing",
            Severity::Error,
            "`name` is required for an agent",
            Some("name"),
        )),
        Some(n) if !is_valid_name(n.trim()) => findings.push(finding(
            "name.invalid",
            Severity::Warning,
            "Agent `name` should be lowercase letters, digits and hyphens",
            Some("name"),
        )),
        _ => {}
    }
    if map
        .get("description")
        .and_then(json_string)
        .map(|d| d.trim().is_empty())
        .unwrap_or(true)
    {
        findings.push(finding(
            "description.missing",
            Severity::Error,
            "`description` is required: it tells the main agent when to delegate",
            Some("description"),
        ));
    }
    if let Some(model) = map.get("model").and_then(json_string) {
        let m = model.trim().to_lowercase();
        if !["sonnet", "opus", "haiku", "inherit"].contains(&m.as_str())
            && !m.starts_with("claude-")
        {
            findings.push(finding(
                "model.unknown",
                Severity::Info,
                format!("`model` is `{model}`; expected an alias (sonnet, opus, haiku, inherit) or a model id"),
                Some("model"),
            ));
        }
    }
    if doc.body.trim().is_empty() {
        findings.push(finding(
            "body.empty",
            Severity::Error,
            "The agent has no system prompt body",
            None,
        ));
    }
    ValidationReport { findings }
}

/// Validate a slash command file.
pub fn validate_command(doc: &ParsedDocument) -> ValidationReport {
    let mut findings = Vec::new();
    if doc.body.trim().is_empty() {
        findings.push(finding(
            "body.empty",
            Severity::Error,
            "The command has no prompt body",
            None,
        ));
    }
    if doc.description.trim().is_empty() {
        findings.push(finding(
            "description.missing",
            Severity::Warning,
            "Add a `description` so the command is listed with a purpose",
            Some("description"),
        ));
    }
    ValidationReport { findings }
}

pub fn validate(doc: &ParsedDocument, dir_name: Option<&str>) -> ValidationReport {
    match doc.kind {
        AssetKind::Skill => validate_skill(doc, dir_name),
        AssetKind::Agent => validate_agent(doc),
        AssetKind::Command => validate_command(doc),
    }
}

/// Serialise frontmatter + body back into a markdown document.
pub fn render_document(frontmatter: &serde_json::Value, body: &str) -> Result<String> {
    let yaml = serde_yaml_ng::to_string(frontmatter)?;
    let yaml = yaml.trim_end();
    let body = body.trim_start_matches('\n');
    Ok(format!("---\n{yaml}\n---\n\n{body}"))
}

/// Pull candidate trigger phrases out of a description: clauses after "use when", "when", "for".
pub fn trigger_phrases(description: &str) -> Vec<String> {
    let lower = description.to_lowercase();
    let mut out = Vec::new();
    for marker in [
        "use when ",
        "use this when ",
        "use it when ",
        "trigger when ",
        "when ",
        "for ",
    ] {
        let mut start = 0;
        while let Some(idx) = lower[start..].find(marker) {
            let begin = start + idx + marker.len();
            let rest = &lower[begin..];
            let end = rest.find(['.', ';', '\n']).unwrap_or(rest.len());
            let phrase = rest[..end].trim().trim_end_matches(',').to_string();
            if phrase.split_whitespace().count() >= 2 {
                out.push(phrase);
            }
            start = begin;
        }
    }
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "---\nname: pdf-tools\ndescription: Extract text and tables from PDF files. Use when the user mentions a PDF or asks to read a document.\nlicense: MIT\nallowed-tools: Read Bash(python:*)\nmetadata:\n  author: hive\n  version: \"1.0\"\n---\n\n# PDF tools\n\nRun `scripts/extract.py`.\n";

    #[test]
    fn parses_frontmatter_and_body() {
        let doc = parse_skill_text(GOOD).unwrap();
        assert_eq!(doc.name, "pdf-tools");
        assert!(doc.description.starts_with("Extract text"));
        assert!(doc.body.contains("# PDF tools"));
        let fm = skill_frontmatter_from_json(doc.frontmatter.as_object().unwrap());
        assert_eq!(fm.allowed_tools, vec!["Read", "Bash(python:*)"]);
        assert_eq!(fm.metadata.get("version").unwrap(), "1.0");
        assert_eq!(fm.license.as_deref(), Some("MIT"));
    }

    #[test]
    fn good_skill_validates_clean() {
        let doc = parse_skill_text(GOOD).unwrap();
        let report = validate_skill(&doc, Some("pdf-tools"));
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.severity != Severity::Error),
            "{report:?}"
        );
        assert!(report.is_ok());
    }

    #[test]
    fn missing_name_and_bad_description() {
        let text = "---\ndescription: short\nfoo: bar\n---\n";
        let doc = parse_skill_text(text).unwrap();
        let report = validate_skill(&doc, None);
        let codes: Vec<&str> = report.findings.iter().map(|f| f.code.as_str()).collect();
        assert!(codes.contains(&"name.missing"));
        assert!(codes.contains(&"description.too_short"));
        assert!(codes.contains(&"frontmatter.unknown_key"));
        assert!(codes.contains(&"body.empty"));
        assert!(!report.is_ok());
    }

    #[test]
    fn invalid_names() {
        for bad in ["Has Space", "UPPER", "-lead", "trail-", "double--dash", ""] {
            assert!(!is_valid_name(bad), "{bad}");
        }
        assert!(is_valid_name("a-b-1"));
        assert_eq!(
            parse_skill_text(
                "---\nname: My Skill\ndescription: does things when asked to do things\n---\nbody"
            )
            .map(|d| validate_skill(&d, None).status())
            .unwrap(),
            crate::model::ValidationStatus::Errors
        );
    }

    #[test]
    fn unclosed_frontmatter_is_an_error() {
        assert!(parse_skill_text("---\nname: x\n").is_err());
        assert!(parse_skill_text("no frontmatter").is_err());
    }

    #[test]
    fn dir_mismatch_is_warning() {
        let doc = parse_skill_text(GOOD).unwrap();
        let report = validate_skill(&doc, Some("other"));
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.code == "name.dir_mismatch" && f.severity == Severity::Warning)
        );
    }

    #[test]
    fn agent_and_command_parsing() {
        let agent = parse_markdown_asset(
            AssetKind::Agent,
            "---\nname: reviewer\ndescription: Reviews code for bugs\ntools: Read, Grep\nmodel: sonnet\n---\nYou are a reviewer.",
            "reviewer",
        )
        .unwrap();
        assert_eq!(agent.name, "reviewer");
        assert!(validate_agent(&agent).is_ok());

        let cmd = parse_markdown_asset(
            AssetKind::Command,
            "Summarise the diff in $ARGUMENTS",
            "summary",
        )
        .unwrap();
        assert_eq!(cmd.name, "summary");
        assert_eq!(cmd.description, "Summarise the diff in $ARGUMENTS");
        let report = validate_command(&cmd);
        assert!(report.is_ok());
    }

    #[test]
    fn round_trip_render() {
        let doc = parse_skill_text(GOOD).unwrap();
        let text = render_document(&doc.frontmatter, &doc.body).unwrap();
        let again = parse_skill_text(&text).unwrap();
        assert_eq!(again.name, doc.name);
        assert_eq!(again.body.trim(), doc.body.trim());
    }

    #[test]
    fn extracts_trigger_phrases() {
        let phrases = trigger_phrases(
            "Extract text from PDFs. Use when the user mentions a PDF or asks to read a document.",
        );
        assert!(
            phrases.iter().any(|p| p.contains("mentions a pdf")),
            "{phrases:?}"
        );
    }

    #[test]
    fn reads_from_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("pdf-tools");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), GOOD).unwrap();
        assert!(is_skill_dir(&dir));
        let doc = read_skill_dir(&dir).unwrap();
        assert_eq!(doc.name, "pdf-tools");
        assert!(!is_skill_dir(tmp.path()));
    }
}
