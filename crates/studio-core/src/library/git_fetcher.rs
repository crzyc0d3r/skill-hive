// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::db::skill_store::SkillStore;
use crate::library::installer::install_local;
use crate::library::skill_metadata::{find_skill_dirs, sanitize_skill_name};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedGit {
    pub repo_url: String,
    pub branch: Option<String>,
    pub subpath: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PreviewSkill {
    pub relative_dir: String,
    pub name: String,
    pub selected: bool,
}

#[derive(Debug, Clone)]
pub struct GitPreview {
    pub id: String,
    pub temp_dir: PathBuf,
    pub parsed: ParsedGit,
    pub skills: Vec<PreviewSkill>,
}

static PREVIEWS: Mutex<Vec<GitPreview>> = Mutex::new(Vec::new());

pub fn parse_git_input(input: &str, remote_refs: Option<&[String]>) -> Result<ParsedGit> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.contains(' ') || trimmed.contains('\\') {
        return Err(anyhow!("invalid git url"));
    }
    if let Some(rest) = trimmed.strip_prefix("https://github.com/") {
        return parse_github_path(rest, remote_refs);
    }
    if trimmed.starts_with("https://")
        || trimmed.starts_with("http://")
        || Path::new(trimmed).exists()
    {
        return Ok(ParsedGit {
            repo_url: trimmed.trim_end_matches(".git").to_string(),
            branch: None,
            subpath: None,
        });
    }
    Err(anyhow!("invalid git url"))
}

fn parse_github_path(rest: &str, remote_refs: Option<&[String]>) -> Result<ParsedGit> {
    let parts: Vec<&str> = rest.split('/').filter(|p| !p.is_empty()).collect();
    if parts.len() < 2 {
        return Err(anyhow!("invalid git url"));
    }
    let repo_url = format!(
        "https://github.com/{}/{}",
        parts[0],
        parts[1].trim_end_matches(".git")
    );
    if parts.len() == 2 {
        return Ok(ParsedGit {
            repo_url,
            branch: None,
            subpath: None,
        });
    }
    if parts.get(2) != Some(&"tree") {
        return Err(anyhow!("invalid git url"));
    }
    let remainder = parts[3..].join("/");
    let (branch, subpath) = split_branch(&remainder, remote_refs);
    Ok(ParsedGit {
        repo_url,
        branch: Some(branch),
        subpath,
    })
}

/// Longest known ref that prefixes `remainder` wins, so a branch named
/// `feature/slash` is preferred over a shorter `feature` ref.
pub fn split_branch(remainder: &str, remote_refs: Option<&[String]>) -> (String, Option<String>) {
    if let Some(refs) = remote_refs {
        let mut best: Option<&str> = None;
        for reference in refs {
            let matches = remainder == reference
                || remainder
                    .strip_prefix(reference.as_str())
                    .is_some_and(|tail| tail.starts_with('/'));
            if matches && best.is_none_or(|current| reference.len() > current.len()) {
                best = Some(reference);
            }
        }
        if let Some(branch) = best {
            let sub = remainder
                .strip_prefix(branch)
                .unwrap_or("")
                .trim_start_matches('/');
            let subpath = if sub.is_empty() {
                None
            } else {
                Some(sub.to_string())
            };
            return (branch.to_string(), subpath);
        }
    }
    (remainder.to_string(), None)
}

pub fn list_remote_heads(repo: &Path) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["ls-remote", "--heads", &repo.to_string_lossy()])
        .output()
        .context("git ls-remote")?;
    if !output.status.success() {
        return Err(anyhow!(
            "git ls-remote failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut refs = Vec::new();
    for line in text.lines() {
        let Some((_, reference)) = line.split_once('\t') else {
            continue;
        };
        if let Some(name) = reference.strip_prefix("refs/heads/") {
            refs.push(name.to_string());
        }
    }
    Ok(refs)
}

pub fn resolve_tree_url(url: &str, ls_remote_repo: Option<&Path>) -> Result<ParsedGit> {
    let refs = match ls_remote_repo {
        Some(repo) => Some(list_remote_heads(repo)?),
        None => None,
    };
    parse_git_input(url, refs.as_deref())
}

pub fn preview_local_repo(
    repo: &Path,
    subpath: Option<&str>,
    cancel: &AtomicBool,
) -> Result<GitPreview> {
    let id = format!("preview-{}", chrono::Utc::now().timestamp_millis());
    let temp_dir = std::env::temp_dir().join("skill-studio-previews").join(&id);
    fs::create_dir_all(&temp_dir)?;
    if cancel.load(Ordering::SeqCst) {
        let _ = fs::remove_dir_all(&temp_dir);
        return Err(anyhow!("Installation cancelled by user"));
    }
    let source = match subpath {
        Some(sub) => repo.join(sub),
        None => repo.to_path_buf(),
    };
    copy_repo(&source, &temp_dir)?;
    if cancel.load(Ordering::SeqCst) {
        let _ = fs::remove_dir_all(&temp_dir);
        return Err(anyhow!("Installation cancelled by user"));
    }
    let skills = find_skill_dirs(&temp_dir, 4)
        .into_iter()
        .map(|dir| {
            let relative = dir
                .strip_prefix(&temp_dir)
                .unwrap_or(&dir)
                .to_string_lossy()
                .replace('\\', "/");
            let name = crate::library::skill_metadata::read_skill_document(&dir)
                .map(|doc| doc.name)
                .unwrap_or_else(|_| sanitize_skill_name(&relative));
            PreviewSkill {
                relative_dir: if relative.is_empty() {
                    ".".into()
                } else {
                    relative
                },
                name,
                selected: true,
            }
        })
        .collect();
    let preview = GitPreview {
        id: id.clone(),
        temp_dir,
        parsed: ParsedGit {
            repo_url: repo.to_string_lossy().to_string(),
            branch: None,
            subpath: subpath.map(|s| s.to_string()),
        },
        skills,
    };
    PREVIEWS.lock().expect("previews").push(preview.clone());
    Ok(preview)
}

pub fn cancel_git_preview(id: &str) -> Result<bool> {
    let mut guard = PREVIEWS.lock().expect("previews");
    if let Some(index) = guard.iter().position(|p| p.id == id) {
        let preview = guard.remove(index);
        let _ = fs::remove_dir_all(&preview.temp_dir);
        return Ok(!preview.temp_dir.exists());
    }
    Ok(false)
}

pub fn confirm_git_install(
    store: &SkillStore,
    skills_root: &Path,
    preview_id: &str,
    selected: &[(String, Option<String>)],
) -> Result<Vec<String>> {
    let preview = {
        let guard = PREVIEWS.lock().expect("previews");
        guard
            .iter()
            .find(|p| p.id == preview_id)
            .cloned()
            .ok_or_else(|| anyhow!("preview not found"))?
    };
    let mut names = Vec::new();
    for (relative, rename) in selected {
        let dir = if relative == "." {
            preview.temp_dir.clone()
        } else {
            preview.temp_dir.join(relative)
        };
        let mut installed = install_local(store, skills_root, &dir)?;
        if let Some(new_name) = rename {
            let sanitised = sanitize_skill_name(new_name);
            installed.record.name = sanitised.clone();
            store.update_skill_after_install(
                &installed.record.id,
                &sanitised,
                installed.record.description.as_deref(),
                None,
                None,
                installed.record.content_hash.as_deref(),
                "unknown",
            )?;
            names.push(sanitised);
        } else {
            names.push(installed.record.name);
        }
    }
    let _ = cancel_git_preview(preview_id)?;
    Ok(names)
}

fn copy_repo(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == ".git" {
            continue;
        }
        let dest = to.join(&name);
        if entry.path().is_dir() {
            copy_repo(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}
