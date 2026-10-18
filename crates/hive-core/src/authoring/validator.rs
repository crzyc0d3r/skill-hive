//! Rules S-01..S-34, A-01..A-19, and C-01..C-02.
//!
//! Every finding is returned. Nothing stops at the first problem. Severity
//! follows the selected profile: portable (the Agent Skills spec plus
//! quick_validate.py) or Claude Code.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::frontmatter::{estimate_tokens, parse_document, Document, Scalar};
use crate::core::skill_metadata::sanitize_skill_name;

pub const QV_RULES: &[&str] = &[
    "S-01", "S-03", "S-04", "S-05", "S-06", "S-23", "S-07", "S-08", "S-09", "S-10", "S-11", "S-12",
    "S-16", "S-18", "S-17", "S-20",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    Portable,
    ClaudeCode,
}

impl Profile {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "portable" => Some(Self::Portable),
            "claude_code" | "claude-code" => Some(Self::ClaudeCode),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub rule_id: String,
    pub severity: Severity,
    pub path: String,
    pub line: Option<usize>,
    pub message: String,
    pub fix_id: Option<String>,
}

impl Finding {
    fn new(
        rule: &str,
        severity: Severity,
        path: impl Into<String>,
        line: Option<usize>,
        message: impl Into<String>,
        fix: Option<&str>,
    ) -> Self {
        Self {
            rule_id: rule.to_string(),
            severity,
            path: path.into(),
            line,
            message: message.into(),
            fix_id: fix.map(str::to_string),
        }
    }
}

const PORTABLE_KEYS: &[&str] = &[
    "name",
    "description",
    "license",
    "compatibility",
    "metadata",
    "allowed-tools",
];

const CLAUDE_KEYS: &[&str] = &[
    "name",
    "description",
    "license",
    "compatibility",
    "metadata",
    "allowed-tools",
    "argument-hint",
    "when_to_use",
    "arguments",
    "disable-model-invocation",
    "user-invocable",
    "disallowed-tools",
    "model",
    "effort",
    "context",
    "agent",
    "background",
    "hooks",
    "paths",
    "shell",
];

const COMMAND_KEYS: &[&str] = &[
    "description",
    "license",
    "compatibility",
    "metadata",
    "allowed-tools",
    "argument-hint",
    "when_to_use",
    "arguments",
    "disable-model-invocation",
    "user-invocable",
    "disallowed-tools",
    "model",
    "effort",
    "context",
    "agent",
    "background",
    "hooks",
    "shell",
];

pub fn validate_skill(dir: &Path, profile: Profile) -> Vec<Finding> {
    let mut findings = Vec::new();
    let skill_md = dir.join("SKILL.md");
    let legacy = dir.join("skill.md");
    if !skill_md.is_file() {
        findings.push(Finding::new(
            "S-01",
            Severity::Error,
            rel(dir, &skill_md),
            Some(1),
            "directory has no SKILL.md marker",
            None,
        ));
        if legacy.is_file() {
            findings.push(Finding::new(
                "S-01",
                Severity::Warning,
                rel(dir, &legacy),
                Some(1),
                "skill.md is a legacy marker and must be renamed to SKILL.md",
                Some("rename-skill-md"),
            ));
        }
        if is_non_marker(dir) {
            findings.push(Finding::new(
                "S-02",
                Severity::Error,
                dir.display().to_string(),
                None,
                "a directory with only README.md, readme.md, or CLAUDE.md is not a skill",
                None,
            ));
        }
    }
    let markers = packaged_skill_mds(dir);
    if markers.len() != 1 {
        let severity = if profile == Profile::Portable {
            Severity::Error
        } else {
            Severity::Warning
        };
        findings.push(Finding::new(
            "S-03",
            severity,
            dir.display().to_string(),
            None,
            format!(
                "expected exactly one packaged SKILL.md, found {}",
                markers.len()
            ),
            None,
        ));
    }
    if skill_md.is_file() {
        let text = fs::read_to_string(&skill_md).unwrap_or_default();
        findings.extend(validate_skill_text(dir, &skill_md, &text, profile));
    }
    findings.extend(validate_resources(dir, profile));
    findings.extend(validate_evals(dir));
    findings.extend(validate_grading_artifacts(dir));
    findings
}

fn validate_skill_text(dir: &Path, skill_md: &Path, text: &str, profile: Profile) -> Vec<Finding> {
    let mut findings = Vec::new();
    let path = rel(dir, skill_md);
    let doc = parse_document(text);
    if !doc.opened_on_line_1 {
        let line = doc.late_marker_line.unwrap_or(1);
        findings.push(Finding::new(
            "S-04",
            Severity::Error,
            path.clone(),
            Some(line),
            "frontmatter must open on line 1; a later --- is skipped",
            None,
        ));
        return findings;
    }
    if !doc.closed {
        findings.push(Finding::new(
            "S-05",
            Severity::Error,
            path.clone(),
            Some(1),
            "frontmatter has no closing ---",
            None,
        ));
        return findings;
    }
    if let Some(err) = &doc.yaml_error {
        findings.push(Finding::new(
            "S-06",
            Severity::Error,
            path.clone(),
            Some(1),
            format!("YAML frontmatter is not a mapping: {err}"),
            None,
        ));
        return findings;
    }
    let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
    findings.extend(frontmatter_rules(dir, &path, &doc, profile, dir_name));
    findings.extend(body_rules(dir, &path, &doc, profile));
    findings
}

fn frontmatter_rules(
    dir: &Path,
    path: &str,
    doc: &Document,
    profile: Profile,
    dir_name: &str,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let allowed: &[&str] = if profile == Profile::Portable {
        PORTABLE_KEYS
    } else {
        CLAUDE_KEYS
    };
    for key in &doc.keys {
        if !allowed.contains(&key.name.as_str()) {
            let severity = if profile == Profile::Portable {
                Severity::Error
            } else {
                Severity::Warning
            };
            findings.push(Finding::new(
                "S-23",
                severity,
                path,
                Some(key.line),
                format!(
                    "key `{}` is not allowed in the {} profile",
                    key.name,
                    profile_name(profile)
                ),
                Some("remove-unknown-key"),
            ));
        }
    }
    match doc.get("name") {
        None => {
            let severity = if profile == Profile::Portable {
                Severity::Error
            } else {
                Severity::Warning
            };
            findings.push(Finding::new(
                "S-07",
                severity,
                path,
                Some(1),
                "name is required",
                None,
            ));
        }
        Some(Scalar::Str { text, .. }) => {
            if !name_charset_ok(text) {
                findings.push(Finding::new(
                    "S-10",
                    Severity::Error,
                    path,
                    Some(line_of(doc, "name")),
                    "name must match ^[a-z0-9-]+$",
                    None,
                ));
            } else if text.starts_with('-') || text.ends_with('-') || text.contains("--") {
                findings.push(Finding::new(
                    "S-11",
                    Severity::Error,
                    path,
                    Some(line_of(doc, "name")),
                    "name must not start or end with a hyphen or contain --",
                    None,
                ));
            } else if text.is_empty() || text.chars().count() > 64 {
                findings.push(Finding::new(
                    "S-12",
                    Severity::Error,
                    path,
                    Some(line_of(doc, "name")),
                    "name must be 1 to 64 characters",
                    None,
                ));
            }
            if text != dir_name {
                findings.push(Finding::new(
                    "S-13",
                    Severity::Warning,
                    path,
                    Some(line_of(doc, "name")),
                    format!("name `{text}` does not match the directory `{dir_name}`"),
                    None,
                ));
            }
            if profile == Profile::ClaudeCode && reserved_name(dir_name, text) {
                findings.push(Finding::new(
                    "S-15",
                    Severity::Error,
                    path,
                    Some(line_of(doc, "name")),
                    "name or directory is reserved",
                    None,
                ));
            }
        }
        Some(_) => findings.push(Finding::new(
            "S-09",
            Severity::Error,
            path,
            Some(line_of(doc, "name")),
            "name must be a string",
            None,
        )),
    }
    if !directory_is_safe(dir_name) {
        findings.push(Finding::new(
            "S-14",
            Severity::Error,
            path,
            None,
            "directory name is not filesystem-safe",
            None,
        ));
    }
    match doc.get("description") {
        None => {
            let severity = if profile == Profile::Portable {
                Severity::Error
            } else {
                Severity::Warning
            };
            findings.push(Finding::new(
                "S-08",
                severity,
                path,
                Some(1),
                "description is required",
                None,
            ));
        }
        Some(Scalar::Str { text, .. }) => {
            if text.trim().is_empty() {
                findings.push(Finding::new(
                    "S-16",
                    Severity::Error,
                    path,
                    Some(line_of(doc, "description")),
                    "description must be a non-empty string",
                    None,
                ));
            }
            if text.chars().count() > 1024 {
                findings.push(Finding::new(
                    "S-17",
                    Severity::Error,
                    path,
                    Some(line_of(doc, "description")),
                    "description must be at most 1024 characters",
                    None,
                ));
            }
            if text.contains('<') || text.contains('>') {
                let severity = if profile == Profile::Portable {
                    Severity::Error
                } else {
                    Severity::Info
                };
                findings.push(Finding::new(
                    "S-18",
                    severity,
                    path,
                    Some(line_of(doc, "description")),
                    "description must not contain angle brackets",
                    None,
                ));
            }
            if profile == Profile::ClaudeCode {
                let extra = doc.string("when_to_use").unwrap_or("");
                if text.chars().count() + extra.chars().count() > 1536 {
                    findings.push(Finding::new(
                        "S-19",
                        Severity::Warning,
                        path,
                        Some(line_of(doc, "description")),
                        "description plus when_to_use exceeds 1536 characters",
                        None,
                    ));
                }
            }
        }
        Some(_) => findings.push(Finding::new(
            "S-16",
            Severity::Error,
            path,
            Some(line_of(doc, "description")),
            "description must be a string",
            None,
        )),
    }
    if let Some(value) = doc.get("compatibility") {
        match value {
            Scalar::Str { text, .. } if text.chars().count() > 500 => findings.push(Finding::new(
                "S-20",
                Severity::Error,
                path,
                Some(line_of(doc, "compatibility")),
                "compatibility must be a string of at most 500 characters",
                None,
            )),
            Scalar::Str { text, .. } if text.is_empty() => findings.push(Finding::new(
                "S-20",
                Severity::Error,
                path,
                Some(line_of(doc, "compatibility")),
                "compatibility must be 1 to 500 characters",
                None,
            )),
            Scalar::Str { .. } => {}
            _ => findings.push(Finding::new(
                "S-20",
                Severity::Error,
                path,
                Some(line_of(doc, "compatibility")),
                "compatibility must be a string",
                None,
            )),
        }
    }
    if let Some(value) = doc.get("metadata") {
        if !is_string_map(value) {
            findings.push(Finding::new(
                "S-21",
                Severity::Warning,
                path,
                Some(line_of(doc, "metadata")),
                "metadata must be a map of strings",
                None,
            ));
        }
    }
    if let Some(value) = doc.get("allowed-tools") {
        let comma_string = matches!(value, Scalar::Str { text, .. } if text.contains(','));
        let not_a_string = !matches!(value, Scalar::Str { .. });
        if not_a_string || (profile == Profile::Portable && comma_string) {
            let severity = if profile == Profile::Portable {
                Severity::Error
            } else {
                Severity::Info
            };
            findings.push(Finding::new(
                "S-22",
                severity,
                path,
                Some(line_of(doc, "allowed-tools")),
                "allowed-tools must be a space-separated string in the portable profile",
                None,
            ));
        }
    }
    if profile == Profile::ClaudeCode {
        findings.extend(claude_enums(dir, path, doc));
    }
    findings
}

fn claude_enums(dir: &Path, path: &str, doc: &Document) -> Vec<Finding> {
    let mut findings = Vec::new();
    for name in ["disable-model-invocation", "user-invocable", "background"] {
        if let Some(value) = doc.get(name) {
            if !is_boolish(value) {
                findings.push(Finding::new(
                    "S-24",
                    Severity::Error,
                    path,
                    Some(line_of(doc, name)),
                    format!("{name} must be a boolean"),
                    None,
                ));
            }
        }
    }
    if let Some(value) = doc.string("context") {
        if value != "fork" {
            findings.push(Finding::new(
                "S-25",
                Severity::Error,
                path,
                Some(line_of(doc, "context")),
                "context must be fork",
                None,
            ));
        }
    }
    if let Some(value) = doc.string("effort") {
        if !["low", "medium", "high", "xhigh", "max"].contains(&value) {
            findings.push(Finding::new(
                "S-25",
                Severity::Error,
                path,
                Some(line_of(doc, "effort")),
                "effort is not a known level",
                None,
            ));
        }
    }
    if let Some(value) = doc.string("shell") {
        if !["bash", "powershell"].contains(&value) {
            findings.push(Finding::new(
                "S-25",
                Severity::Error,
                path,
                Some(line_of(doc, "shell")),
                "shell must be bash or powershell",
                None,
            ));
        }
    }
    if let Some(value) = doc.string("agent") {
        if !known_subagent(dir, value) {
            findings.push(Finding::new(
                "S-25",
                Severity::Error,
                path,
                Some(line_of(doc, "agent")),
                "agent is not a known subagent",
                None,
            ));
        }
    }
    let fork = doc.string("context") == Some("fork");
    if !fork && (doc.get("agent").is_some() || doc.get("background").is_some()) {
        findings.push(Finding::new(
            "S-26",
            Severity::Warning,
            path,
            Some(line_of(doc, "agent").max(line_of(doc, "background"))),
            "agent and background require context: fork",
            None,
        ));
    }
    findings
}

fn body_rules(dir: &Path, path: &str, doc: &Document, profile: Profile) -> Vec<Finding> {
    let mut findings = Vec::new();
    let lines = doc.body.lines().count();
    if lines > 500 {
        findings.push(Finding::new(
            "S-27",
            Severity::Warning,
            path,
            Some(doc.body_starts_at()),
            format!("body is {lines} lines; keep it under 500"),
            None,
        ));
    }
    let tokens = estimate_tokens(&doc.body);
    if tokens >= 5000 {
        findings.push(Finding::new(
            "S-28",
            Severity::Warning,
            path,
            None,
            format!("body is about {tokens} tokens; keep it under 5000"),
            None,
        ));
    }
    if profile == Profile::Portable && claude_only_syntax(&doc.body) {
        findings.push(Finding::new(
            "S-31",
            Severity::Info,
            path,
            None,
            "body uses Claude Code-only command or ${CLAUDE_*} syntax and is not portable",
            None,
        ));
    }
    for reference in referenced_paths(&doc.body) {
        let depth = reference.split('/').filter(|part| !part.is_empty()).count();
        let target = dir.join(&reference);
        if depth > 2 || reference.contains("..") || !target.exists() {
            findings.push(Finding::new(
                "S-30",
                Severity::Warning,
                path,
                None,
                format!("referenced path `{reference}` is missing or deeper than one directory"),
                Some("create-missing-file"),
            ));
        }
    }
    findings
}

fn validate_resources(dir: &Path, _profile: Profile) -> Vec<Finding> {
    let mut findings = Vec::new();
    walk_markdown(dir, dir, &mut findings);
    walk_scripts(dir, &dir.join("scripts"), &mut findings);
    findings
}

fn walk_markdown(root: &Path, dir: &Path, findings: &mut Vec<Finding>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == ".git" || name == "node_modules" || name == "__pycache__" {
                continue;
            }
            walk_markdown(root, &path, findings);
            continue;
        }
        if entry.file_name() == "SKILL.md" {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap_or_default();
        if text.lines().count() > 300 && !has_toc(&text) {
            findings.push(Finding::new(
                "S-29",
                Severity::Info,
                rel(root, &path),
                Some(1),
                "markdown longer than 300 lines needs a table of contents",
                Some("add-toc"),
            ));
        }
    }
}

fn walk_scripts(root: &Path, dir: &Path, findings: &mut Vec<Finding>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_scripts(root, &path, findings);
            continue;
        }
        if !path.is_file() || !should_be_executable(&path) {
            continue;
        }
        if !is_executable(&path) {
            findings.push(Finding::new(
                "S-32",
                Severity::Warning,
                rel(root, &path),
                None,
                "script is not executable",
                Some("set-executable"),
            ));
        }
    }
}

fn validate_evals(dir: &Path) -> Vec<Finding> {
    let path = dir.join("evals").join("evals.json");
    if !path.is_file() {
        return Vec::new();
    }
    let mut findings = Vec::new();
    let text = fs::read_to_string(&path).unwrap_or_default();
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&text);
    let Ok(json) = parsed else {
        findings.push(Finding::new(
            "S-33",
            Severity::Error,
            rel(dir, &path),
            Some(1),
            "evals.json is not JSON",
            Some("sync-skill-name"),
        ));
        return findings;
    };
    let skill_name = dir.join("SKILL.md");
    let expected = fs::read_to_string(skill_name)
        .ok()
        .and_then(|text| parse_document(&text).string("name").map(str::to_string));
    if json.get("skill_name").and_then(|v| v.as_str()) != expected.as_deref() {
        findings.push(Finding::new(
            "S-33",
            Severity::Error,
            rel(dir, &path),
            Some(1),
            "evals.json skill_name must equal the frontmatter name",
            Some("sync-skill-name"),
        ));
    }
    let evals = json.get("evals").and_then(|v| v.as_array());
    let Some(evals) = evals else {
        findings.push(Finding::new(
            "S-33",
            Severity::Error,
            rel(dir, &path),
            None,
            "evals must be an array",
            None,
        ));
        return findings;
    };
    let mut ids = Vec::new();
    for item in evals {
        let id = item.get("id").and_then(|v| v.as_i64());
        if let Some(id) = id {
            if ids.contains(&id) {
                findings.push(Finding::new(
                    "S-33",
                    Severity::Error,
                    rel(dir, &path),
                    None,
                    format!("duplicate eval id {id}"),
                    None,
                ));
            }
            ids.push(id);
        } else {
            findings.push(Finding::new(
                "S-33",
                Severity::Error,
                rel(dir, &path),
                None,
                "eval id must be an integer",
                None,
            ));
        }
        if item
            .get("prompt")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
        {
            findings.push(Finding::new(
                "S-33",
                Severity::Error,
                rel(dir, &path),
                None,
                "eval prompt must be a non-empty string",
                None,
            ));
        }
        if let Some(files) = item.get("files").and_then(|v| v.as_array()) {
            for file in files {
                let rel_path = file.as_str().unwrap_or("");
                if !dir.join(rel_path).is_file() {
                    findings.push(Finding::new(
                        "S-33",
                        Severity::Error,
                        rel(dir, &path),
                        None,
                        format!("eval file `{rel_path}` does not exist"),
                        None,
                    ));
                }
            }
        }
        if let Some(expectations) = item.get("expectations").and_then(|v| v.as_array()) {
            if expectations.iter().any(|item| item.as_str().is_none()) {
                findings.push(Finding::new(
                    "S-33",
                    Severity::Error,
                    rel(dir, &path),
                    None,
                    "expectations must be strings",
                    None,
                ));
            }
        }
    }
    findings
}

fn validate_grading_artifacts(dir: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    for candidate in [
        dir.join("grading.json"),
        dir.join("evals").join("grading.json"),
    ] {
        if !candidate.is_file() {
            continue;
        }
        let Ok(json) = serde_json::from_str::<serde_json::Value>(
            &fs::read_to_string(&candidate).unwrap_or_default(),
        ) else {
            findings.push(Finding::new(
                "S-34",
                Severity::Error,
                rel(dir, &candidate),
                Some(1),
                "grading.json is not JSON",
                None,
            ));
            continue;
        };
        let Some(expectations) = json.get("expectations").and_then(|v| v.as_array()).cloned()
        else {
            findings.push(Finding::new(
                "S-34",
                Severity::Error,
                rel(dir, &candidate),
                Some(1),
                "grading.json needs an expectations array",
                None,
            ));
            continue;
        };
        for item in expectations {
            let keys: Vec<&str> = item
                .as_object()
                .map(|map| map.keys().map(String::as_str).collect())
                .unwrap_or_default();
            if keys != ["text", "passed", "evidence"]
                && !keys_exact(&keys, &["text", "passed", "evidence"])
            {
                findings.push(Finding::new(
                    "S-34",
                    Severity::Error,
                    rel(dir, &candidate),
                    None,
                    "grading expectation must use exactly text, passed, and evidence",
                    None,
                ));
            }
        }
    }
    for candidate in [
        dir.join("benchmark.json"),
        dir.join("evals").join("benchmark.json"),
    ] {
        if !candidate.is_file() {
            continue;
        }
        let Ok(json) = serde_json::from_str::<serde_json::Value>(
            &fs::read_to_string(&candidate).unwrap_or_default(),
        ) else {
            findings.push(Finding::new(
                "S-34",
                Severity::Error,
                rel(dir, &candidate),
                Some(1),
                "benchmark.json is not JSON",
                None,
            ));
            continue;
        };
        let rows = if let Some(rows) = json.as_array().cloned() {
            rows
        } else if let Some(rows) = json.get("runs").and_then(|v| v.as_array()).cloned() {
            rows
        } else {
            findings.push(Finding::new(
                "S-34",
                Severity::Error,
                rel(dir, &candidate),
                Some(1),
                "benchmark.json must list runs",
                None,
            ));
            continue;
        };
        for row in rows {
            let configuration = row
                .get("configuration")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if configuration != "with_skill" && configuration != "without_skill" {
                findings.push(Finding::new(
                    "S-34",
                    Severity::Error,
                    rel(dir, &candidate),
                    None,
                    "benchmark configuration must be with_skill or without_skill",
                    None,
                ));
            }
            if row.get("result").and_then(|v| v.get("pass_rate")).is_none() {
                findings.push(Finding::new(
                    "S-34",
                    Severity::Error,
                    rel(dir, &candidate),
                    None,
                    "benchmark pass_rate must be under result",
                    None,
                ));
            }
        }
    }
    findings
}

pub fn validate_agent(path: &Path, plugin_mode: bool) -> Vec<Finding> {
    let peers = sibling_agent_peers(path);
    validate_agent_with_peers(path, plugin_mode, &peers)
}

fn sibling_agent_peers(path: &Path) -> Vec<(String, String)> {
    let Some(parent) = path.parent() else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut peers = Vec::new();
    for entry in entries.flatten() {
        let candidate = entry.path();
        if candidate == path || candidate.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let text = fs::read_to_string(&candidate).unwrap_or_default();
        let doc = parse_document(&text);
        if let Some(name) = doc.string("name") {
            peers.push((
                name.to_string(),
                doc.string("description").unwrap_or("").to_string(),
            ));
        }
    }
    peers
}

pub fn validate_agent_with_peers(
    path: &Path,
    plugin_mode: bool,
    peers: &[(String, String)],
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let display = path.display().to_string();
    if !looks_like_agent_location(path) {
        findings.push(Finding::new(
            "A-01",
            Severity::Info,
            display.clone(),
            None,
            "agent files normally live under .claude/agents or a plugin agents directory",
            None,
        ));
    }
    let text = fs::read_to_string(path).unwrap_or_default();
    let doc = parse_document(&text);
    if !doc.opened_on_line_1 {
        findings.push(Finding::new(
            "A-02",
            Severity::Error,
            display.clone(),
            doc.late_marker_line,
            "frontmatter must open on line 1; Claude Code skips a late ---",
            None,
        ));
        return findings;
    }
    if !doc.closed {
        findings.push(Finding::new(
            "A-02",
            Severity::Error,
            display,
            Some(1),
            "frontmatter has no closing --- so the agent is skipped",
            None,
        ));
        return findings;
    }
    if doc.yaml_error.is_some() {
        findings.push(Finding::new(
            "A-02",
            Severity::Error,
            display,
            Some(1),
            "unparseable YAML is skipped by Claude Code",
            None,
        ));
        return findings;
    }
    let name = doc.string("name");
    match name {
        None => findings.push(Finding::new(
            "A-02",
            Severity::Error,
            display.clone(),
            Some(1),
            "name is required; Claude Code skips the file",
            None,
        )),
        Some(value) => {
            if value.chars().count() > 256 || value.starts_with('-') || value.contains(':') {
                findings.push(Finding::new(
                    "A-04",
                    Severity::Error,
                    display.clone(),
                    Some(line_of(&doc, "name")),
                    "name must be at most 256 characters, must not start with -, and must not contain :",
                    None,
                ));
            }
            if [
                "Explore",
                "Plan",
                "general-purpose",
                "claude",
                "statusline-setup",
                "claude-code-guide",
            ]
            .contains(&value)
            {
                findings.push(Finding::new(
                    "A-17",
                    Severity::Warning,
                    display.clone(),
                    Some(line_of(&doc, "name")),
                    "name is a reserved built-in",
                    None,
                ));
            }
        }
    }
    if doc.get("description").is_none()
        || doc
            .string("description")
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
    {
        findings.push(Finding::new(
            "A-03",
            Severity::Error,
            display.clone(),
            Some(1),
            "description is required; a name without a description is skipped",
            None,
        ));
    }
    for key in &doc.keys {
        if let Some(fixed) = near_miss(&key.name) {
            findings.push(Finding::new(
                "A-05",
                Severity::Warning,
                display.clone(),
                Some(key.line),
                format!("`{}` is ignored; rename it to `{fixed}`", key.name),
                Some("rename-key"),
            ));
        }
        if key.name == "cacheTtl" {
            findings.push(Finding::new(
                "A-14",
                Severity::Error,
                display.clone(),
                Some(key.line),
                "cacheTtl belongs under experimental",
                Some("rename-key"),
            ));
        }
    }
    if let Some(tools) = tool_names(&doc) {
        if tools.iter().any(|tool| tool == "Skill") {
            findings.push(Finding::new(
                "A-06",
                Severity::Warning,
                display.clone(),
                Some(line_of(&doc, "tools")),
                "list skills in skills, not tools",
                Some("move-skill-tool"),
            ));
        }
    } else if doc.get("tools").is_some() {
        findings.push(Finding::new(
            "A-06",
            Severity::Warning,
            display.clone(),
            Some(line_of(&doc, "tools")),
            "tools must be a comma-separated string or a list",
            None,
        ));
    }
    if let Some(model) = doc.string("model") {
        if !model_ok(model) {
            findings.push(Finding::new(
                "A-07",
                Severity::Error,
                display.clone(),
                Some(line_of(&doc, "model")),
                "model is not an alias, inherit, or a model id",
                None,
            ));
        }
    }
    enum_field(
        &doc,
        "permissionMode",
        &[
            "default",
            "acceptEdits",
            "auto",
            "dontAsk",
            "bypassPermissions",
            "plan",
            "manual",
        ],
        "A-08",
        &display,
        &mut findings,
    );
    enum_field(
        &doc,
        "memory",
        &["user", "project", "local"],
        "A-09",
        &display,
        &mut findings,
    );
    enum_field(
        &doc,
        "color",
        &[
            "red", "blue", "green", "yellow", "purple", "orange", "pink", "cyan",
        ],
        "A-10",
        &display,
        &mut findings,
    );
    enum_field(
        &doc,
        "effort",
        &["low", "medium", "high", "xhigh", "max"],
        "A-11",
        &display,
        &mut findings,
    );
    enum_field(
        &doc,
        "isolation",
        &["worktree"],
        "A-12",
        &display,
        &mut findings,
    );
    if let Some(turns) = doc.get("maxTurns") {
        let ok = match turns {
            Scalar::Int(value) => *value > 0,
            Scalar::Str { text, .. } => text.parse::<i64>().ok().filter(|n| *n > 0).is_some(),
            _ => false,
        };
        if !ok {
            findings.push(Finding::new(
                "A-13",
                Severity::Error,
                display.clone(),
                Some(line_of(&doc, "maxTurns")),
                "maxTurns must be a positive integer",
                None,
            ));
        }
    }
    if let Some(Scalar::Map(map)) = doc.get("experimental") {
        if let Some((_, ttl)) = map.iter().find(|(key, _)| key == "cacheTtl") {
            if ttl != "5m" && ttl != "1h" {
                findings.push(Finding::new(
                    "A-14",
                    Severity::Error,
                    display.clone(),
                    Some(line_of(&doc, "experimental")),
                    "experimental.cacheTtl must be 5m or 1h",
                    None,
                ));
            }
        }
    }
    if plugin_mode {
        for key in ["hooks", "mcpServers", "permissionMode", "initialPrompt"] {
            if doc.get(key).is_some() {
                findings.push(Finding::new(
                    "A-15",
                    Severity::Warning,
                    display.clone(),
                    Some(line_of(&doc, key)),
                    format!("{key} has no effect on a plugin agent"),
                    None,
                ));
            }
        }
    }
    if doc.body.trim().is_empty() {
        findings.push(Finding::new(
            "A-16",
            Severity::Warning,
            display.clone(),
            None,
            "the system prompt body is empty",
            None,
        ));
    }
    if let Some(name) = name {
        let duplicates = peers.iter().filter(|(other, _)| other == name).count();
        if duplicates > 0 {
            findings.push(Finding::new(
                "A-18",
                Severity::Warning,
                display.clone(),
                None,
                format!("name `{name}` is already used"),
                None,
            ));
        }
    }
    let mut descriptions: Vec<&str> = peers
        .iter()
        .map(|(_, description)| description.as_str())
        .collect();
    if let Some(description) = doc.string("description") {
        descriptions.push(description);
    }
    if estimate_tokens(&descriptions.join("\n")) > 15_000 {
        findings.push(Finding::new(
            "A-19",
            Severity::Warning,
            display,
            None,
            "combined agent descriptions exceed 15000 tokens",
            None,
        ));
    }
    findings
}

pub fn validate_command_file(path: &Path, skill_names: &[String]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let display = path.display().to_string();
    let text = fs::read_to_string(path).unwrap_or_default();
    let doc = parse_document(&text);
    if !doc.opened_on_line_1 || !doc.closed || doc.yaml_error.is_some() {
        findings.push(Finding::new(
            "C-01",
            Severity::Error,
            display.clone(),
            Some(1),
            "command frontmatter must be a closed mapping on line 1",
            None,
        ));
        return findings;
    }
    for key in &doc.keys {
        if key.name == "name" || key.name == "paths" || !COMMAND_KEYS.contains(&key.name.as_str()) {
            findings.push(Finding::new(
                "C-01",
                Severity::Error,
                display.clone(),
                Some(key.line),
                format!("command frontmatter cannot use `{}`", key.name),
                None,
            ));
        }
    }
    if let Some(invocation) = command_invocation(path) {
        if skill_names.iter().any(|skill| skill == &invocation) {
            findings.push(Finding::new(
                "C-02",
                Severity::Info,
                display,
                None,
                format!("/{invocation} collides with a skill of the same name, and the skill wins"),
                None,
            ));
        }
    }
    findings
}

pub fn agent_budget_finding(descriptions: &[&str]) -> Option<Finding> {
    if estimate_tokens(&descriptions.join("\n")) > 15_000 {
        Some(Finding::new(
            "A-19",
            Severity::Warning,
            "agents",
            None,
            "combined agent descriptions exceed 15000 tokens",
            None,
        ))
    } else {
        None
    }
}

fn command_invocation(path: &Path) -> Option<String> {
    let parts: Vec<String> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    let commands = parts.iter().position(|part| part == "commands")?;
    let rest = &parts[commands + 1..];
    if rest.is_empty() {
        return None;
    }
    let file = rest.last()?.trim_end_matches(".md");
    if rest.len() == 1 {
        Some(file.to_string())
    } else {
        Some(format!("{}:{}", rest[..rest.len() - 1].join(":"), file))
    }
}

fn enum_field(
    doc: &Document,
    key: &str,
    allowed: &[&str],
    rule: &str,
    path: &str,
    findings: &mut Vec<Finding>,
) {
    if let Some(value) = doc.string(key) {
        if !allowed.contains(&value) {
            findings.push(Finding::new(
                rule,
                Severity::Error,
                path,
                Some(line_of(doc, key)),
                format!("{key} must be one of {}", allowed.join(", ")),
                None,
            ));
        }
    }
}

fn near_miss(key: &str) -> Option<&'static str> {
    match key {
        "disallowed_tools" => Some("disallowedTools"),
        "permission_mode" => Some("permissionMode"),
        "max_turns" => Some("maxTurns"),
        "mcp_servers" => Some("mcpServers"),
        "omit_claude_md" => Some("omitClaudeMd"),
        "initial_prompt" => Some("initialPrompt"),
        _ => None,
    }
}

fn tool_names(doc: &Document) -> Option<Vec<String>> {
    match doc.get("tools")? {
        Scalar::Seq(items) => Some(items.clone()),
        Scalar::Str { text, .. } => Some(
            text.split(',')
                .map(|part| part.trim().to_string())
                .filter(|part| !part.is_empty())
                .collect(),
        ),
        _ => None,
    }
}

fn model_ok(model: &str) -> bool {
    ["sonnet", "opus", "haiku", "fable", "inherit"].contains(&model)
        || (model.len() >= 3
            && model
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | ':' | '-' | '/'))
            && model.contains('-'))
}

fn looks_like_agent_location(path: &Path) -> bool {
    let text = path.to_string_lossy().replace('\\', "/");
    text.contains("/.claude/agents/") || text.contains("/agents/")
}

fn keys_exact(keys: &[&str], expected: &[&str]) -> bool {
    let mut left = keys.to_vec();
    let mut right = expected.to_vec();
    left.sort_unstable();
    right.sort_unstable();
    left == right
}

fn is_string_map(value: &Scalar) -> bool {
    match value {
        Scalar::Map(entries) => entries.iter().all(|(_, item)| !item.starts_with('\u{1}')),
        _ => false,
    }
}

fn known_subagent(dir: &Path, name: &str) -> bool {
    if ["Explore", "Plan", "general-purpose"].contains(&name) {
        return true;
    }
    let mut current = dir.to_path_buf();
    loop {
        if current
            .join(".claude")
            .join("agents")
            .join(format!("{name}.md"))
            .is_file()
        {
            return true;
        }
        if !current.pop() {
            return false;
        }
    }
}

fn is_boolish(value: &Scalar) -> bool {
    match value {
        Scalar::Bool(_) => true,
        Scalar::Int(0 | 1) => true,
        Scalar::Str { text, .. } => matches!(
            text.as_str(),
            "true" | "false" | "yes" | "no" | "on" | "off" | "1" | "0"
        ),
        _ => false,
    }
}

fn name_charset_ok(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

fn reserved_name(dir_name: &str, name: &str) -> bool {
    dir_name == "synced" || name == "anthropic-skills" || name.starts_with("anthropic-skills:")
}

pub fn directory_is_safe(name: &str) -> bool {
    sanitize_skill_name(name).as_deref() == Some(name)
}

fn line_of(doc: &Document, name: &str) -> usize {
    doc.keys
        .iter()
        .find(|key| key.name == name)
        .map(|key| key.line)
        .unwrap_or(1)
}

fn profile_name(profile: Profile) -> &'static str {
    match profile {
        Profile::Portable => "portable",
        Profile::ClaudeCode => "claude-code",
    }
}

fn is_non_marker(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    let names: Vec<String> = entries
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    !names.is_empty()
        && names
            .iter()
            .all(|name| matches!(name.as_str(), "README.md" | "readme.md" | "CLAUDE.md"))
}

fn packaged_skill_mds(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk_markers(root, true, &mut found);
    found
}

fn walk_markers(dir: &Path, at_root: bool, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "__pycache__" || name == "node_modules" || (at_root && name == "evals") {
                continue;
            }
            walk_markers(&path, false, found);
        } else if entry.file_name() == "SKILL.md" {
            found.push(path);
        }
    }
}

fn referenced_paths(body: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for token in body.split(|ch: char| {
        ch.is_whitespace()
            || matches!(
                ch,
                '(' | ')' | '"' | '\'' | '`' | '<' | '>' | '[' | ']' | ','
            )
    }) {
        let token = token.trim_matches(|ch: char| matches!(ch, '.' | ':' | ';'));
        if token.is_empty() || token.contains("://") || token.starts_with('#') {
            continue;
        }
        let relative_file = token.rsplit_once('.').is_some_and(|(_, ext)| {
            matches!(
                ext,
                "md" | "py" | "sh" | "bash" | "js" | "json" | "txt" | "yml" | "yaml" | "toml"
            )
        });
        if (token.contains('/') || relative_file)
            && !token.starts_with('/')
            && token
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '_' | '-' | '.'))
        {
            paths.push(token.trim_start_matches("./").to_string());
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

fn claude_only_syntax(body: &str) -> bool {
    body.contains("${CLAUDE_") || body.contains("!`") || body.contains("```!")
}

fn has_toc(text: &str) -> bool {
    text.lines().take(40).any(|line| {
        let lower = line.to_ascii_lowercase();
        lower.contains("table of contents") || lower.contains("contents")
    })
}

fn should_be_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    !matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("md" | "txt" | "json" | "yaml" | "yml")
    )
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path)
            .map(|meta| meta.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        true
    }
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// The first portable error quick_validate.py would report, if any.
pub fn first_qv_error(findings: &[Finding]) -> Option<&Finding> {
    QV_RULES.iter().find_map(|rule| {
        findings.iter().find(|finding| {
            finding.rule_id == *rule
                && finding.severity == Severity::Error
                && !(finding.rule_id == "S-16" && finding.message.contains("non-empty"))
                && !(finding.rule_id == "S-20" && finding.message.contains("1 to 500"))
        })
    })
}
