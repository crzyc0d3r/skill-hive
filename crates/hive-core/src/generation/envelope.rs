//! File envelope parser. Paths may not escape the subject directory.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequiredFile {
    Skill,
    Agent,
    Either,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopeError {
    pub path: String,
    pub reason: String,
}

pub fn parse_envelope(
    text: &str,
    required: RequiredFile,
) -> Result<BTreeMap<String, String>, EnvelopeError> {
    let mut files: BTreeMap<String, String> = BTreeMap::new();
    let mut current: Option<String> = None;
    let mut body = String::new();
    for line in text.lines() {
        if let Some(path) = file_marker(line) {
            if let Some(path) = current.take() {
                commit(&mut files, path, &body)?;
                body.clear();
            }
            check_path(&path)?;
            current = Some(path);
        } else if line.trim() == "===== END =====" {
            if let Some(path) = current.take() {
                commit(&mut files, path, &body)?;
                body.clear();
            }
        } else if current.is_some() {
            body.push_str(line);
            body.push('\n');
        }
    }
    if let Some(path) = current.take() {
        commit(&mut files, path, &body)?;
    }
    let has_skill = files.contains_key("SKILL.md");
    let has_agent = files.contains_key("agent.md");
    let ok = match required {
        RequiredFile::Skill => has_skill,
        RequiredFile::Agent => has_agent,
        RequiredFile::Either => has_skill || has_agent,
    };
    if !ok {
        let path = match required {
            RequiredFile::Agent => "agent.md",
            _ => "SKILL.md",
        };
        return Err(EnvelopeError {
            path: path.to_string(),
            reason: "envelope is missing the required marker file".into(),
        });
    }
    Ok(files)
}

fn commit(
    files: &mut BTreeMap<String, String>,
    path: String,
    body: &str,
) -> Result<(), EnvelopeError> {
    if body.contains('\0') {
        return Err(EnvelopeError {
            path,
            reason: "file is not text".into(),
        });
    }
    files.insert(path, body.to_string());
    Ok(())
}

fn file_marker(line: &str) -> Option<String> {
    let line = line.trim();
    let rest = line.strip_prefix("===== FILE:")?.strip_suffix("=====")?;
    Some(rest.trim().to_string())
}

pub fn check_path(path: &str) -> Result<(), EnvelopeError> {
    let reason = if path.is_empty() {
        Some("path is empty")
    } else if path.starts_with('/') || path.starts_with('\\') || path.contains(':') {
        Some("absolute paths are rejected")
    } else if path.split(['/', '\\']).any(|part| part == "..") {
        Some("parent segments are rejected")
    } else if path
        .split(['/', '\\'])
        .filter(|part| !part.is_empty())
        .count()
        > 2
    {
        Some("paths may have at most two segments")
    } else {
        None
    };
    if let Some(reason) = reason {
        return Err(EnvelopeError {
            path: path.to_string(),
            reason: reason.into(),
        });
    }
    Ok(())
}

pub fn write_files(dir: &Path, files: &BTreeMap<String, String>) -> std::io::Result<()> {
    for (rel, body) in files {
        if rel.starts_with(".studio/") {
            continue;
        }
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, body)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path)?.permissions();
            perms.set_mode(0o644);
            fs::set_permissions(&path, perms)?;
        }
    }
    Ok(())
}
