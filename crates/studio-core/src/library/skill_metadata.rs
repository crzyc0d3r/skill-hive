// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

use std::fs;
use std::path::{Path, PathBuf};

/// A skill directory existed on disk with no sidecar metadata to reindex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmptySkillMetadata;

impl std::fmt::Display for EmptySkillMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EmptySkillMetadata")
    }
}

impl std::error::Error for EmptySkillMetadata {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillDocument {
    pub name: String,
    pub description: Option<String>,
    pub body: String,
}

/// Lowercase, hyphenated skill name. Path separators and other unsafe
/// characters are stripped so the result can be a single directory segment.
pub fn sanitize_skill_name(raw: &str) -> String {
    let mut out = String::new();
    let mut prev_hyphen = false;
    for ch in raw.trim().chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            Some(ch.to_ascii_lowercase())
        } else if ch == '-' || ch == '_' || ch.is_whitespace() {
            Some('-')
        } else {
            None
        };
        if let Some(c) = mapped {
            if c == '-' {
                if prev_hyphen || out.is_empty() {
                    continue;
                }
                prev_hyphen = true;
            } else {
                prev_hyphen = false;
            }
            out.push(c);
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "skill".to_string()
    } else {
        out
    }
}

pub fn parse_skill_md(text: &str) -> SkillDocument {
    let (front, body) = split_frontmatter(text);
    let mut name = None;
    let mut description = None;
    if let Some(front) = front {
        for line in front.lines() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let key = key.trim();
            let value = unquote(value.trim());
            if value.is_empty() {
                continue;
            }
            match key {
                "name" => name = Some(sanitize_skill_name(&value)),
                "description" => description = Some(value),
                _ => {}
            }
        }
    }
    SkillDocument {
        name: name.unwrap_or_else(|| "skill".to_string()),
        description,
        body: body.trim().to_string(),
    }
}

fn unquote(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        if (bytes[0] == b'"' && bytes[trimmed.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[trimmed.len() - 1] == b'\'')
        {
            return trimmed[1..trimmed.len() - 1].to_string();
        }
    }
    trimmed.to_string()
}

fn split_frontmatter(text: &str) -> (Option<&str>, &str) {
    let rest = text.trim_start_matches('\u{feff}');
    let Some(rest) = rest.strip_prefix("---") else {
        return (None, text);
    };
    let rest = rest
        .strip_prefix('\n')
        .or_else(|| rest.strip_prefix("\r\n"))
        .unwrap_or(rest);
    if let Some(idx) = rest.find("\n---") {
        let front = &rest[..idx];
        let after = &rest[idx + 4..];
        let body = after
            .strip_prefix('\n')
            .or_else(|| after.strip_prefix("\r\n"))
            .unwrap_or(after);
        return (Some(front), body);
    }
    (None, text)
}

pub fn read_skill_document(dir: &Path) -> std::io::Result<SkillDocument> {
    let text = fs::read_to_string(dir.join("SKILL.md"))?;
    let mut doc = parse_skill_md(&text);
    if doc.name == "skill" {
        if let Some(folder) = dir.file_name().and_then(|n| n.to_str()) {
            doc.name = sanitize_skill_name(folder);
        }
    }
    Ok(doc)
}

/// Directories that contain `SKILL.md`, searched up to `max_depth` directory
/// levels below `root` (the root itself is depth 0).
pub fn find_skill_dirs(root: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk(root, 0, max_depth, &mut found);
    found.sort();
    found
}

fn walk(dir: &Path, depth: usize, max_depth: usize, found: &mut Vec<PathBuf>) {
    if dir.join("SKILL.md").is_file() {
        found.push(dir.to_path_buf());
        return;
    }
    if depth >= max_depth {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, depth + 1, max_depth, found);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitises_names_and_parses_frontmatter() {
        assert_eq!(sanitize_skill_name("My Skill/Name"), "my-skillname");
        assert_eq!(sanitize_skill_name("../etc"), "etc");
        let doc =
            parse_skill_md("---\nname: Research Notes\ndescription: \"Find papers\"\n---\nHello\n");
        assert_eq!(doc.name, "research-notes");
        assert_eq!(doc.description.as_deref(), Some("Find papers"));
        assert_eq!(doc.body, "Hello");
    }

    #[test]
    fn marker_search_stops_at_depth_4() {
        let tmp = tempfile::tempdir().unwrap();
        let deep = tmp.path().join("a").join("b").join("c").join("d").join("e");
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("SKILL.md"), "---\nname: too-deep\n---\n").unwrap();
        let ok = tmp.path().join("a").join("b").join("c").join("skill");
        fs::create_dir_all(&ok).unwrap();
        fs::write(ok.join("SKILL.md"), "---\nname: ok\n---\n").unwrap();
        let found = find_skill_dirs(tmp.path(), 4);
        assert_eq!(found, vec![ok]);
    }
}
