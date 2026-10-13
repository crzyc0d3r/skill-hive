// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

//! System-git backup engine: init, commit, push, pull, snapshot tags, restore
//! with a safety tag, and clone onto a second base directory.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupStatus {
    pub branch: String,
    pub clean: bool,
    pub remote: Option<String>,
    pub head: String,
}

pub fn init_bare(path: &Path) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    git(parent, &["init", "--bare", &path.to_string_lossy()])?;
    Ok(())
}

pub fn init_repo(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    git(path, &["init", "-b", "main"])?;
    Ok(())
}

pub fn commit_all(path: &Path, message: &str) -> Result<String> {
    git(path, &["add", "-A"])?;
    git(path, &["commit", "-m", message])?;
    git(path, &["rev-parse", "HEAD"])
}

pub fn set_remote(path: &Path, url: &Path) -> Result<()> {
    let existing = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(path)
        .output()?;
    if existing.status.success() {
        git(
            path,
            &["remote", "set-url", "origin", &url.to_string_lossy()],
        )?;
    } else {
        git(path, &["remote", "add", "origin", &url.to_string_lossy()])?;
    }
    Ok(())
}

pub fn push(path: &Path) -> Result<()> {
    git(path, &["push", "-u", "origin", "HEAD"])?;
    Ok(())
}

pub fn pull(path: &Path) -> Result<()> {
    git(path, &["pull", "--no-rebase", "origin", "main"])?;
    Ok(())
}

pub fn clone_repo(url: &Path, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    git(
        dest.parent().unwrap_or_else(|| Path::new(".")),
        &["clone", &url.to_string_lossy(), &dest.to_string_lossy()],
    )?;
    Ok(())
}

pub fn snapshot(path: &Path, name: &str) -> Result<String> {
    git(path, &["tag", name])?;
    Ok(name.to_string())
}

/// Tag the current commit as a safety point, then reset to `tag`.
pub fn restore_snapshot(path: &Path, tag: &str) -> Result<String> {
    let safety = format!("safety-{}", chrono::Utc::now().timestamp_millis());
    git(path, &["tag", &safety])?;
    git(path, &["reset", "--hard", tag])?;
    Ok(safety)
}

pub fn tag_exists(path: &Path, name: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["rev-parse", "-q", "--verify", &format!("refs/tags/{name}")])
        .current_dir(path)
        .output()?;
    Ok(output.status.success())
}

pub fn status(path: &Path) -> Result<BackupStatus> {
    let branch = git(path, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let head = git(path, &["rev-parse", "HEAD"])?;
    let porcelain = git(path, &["status", "--porcelain"])?;
    let remote = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(path)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    Ok(BackupStatus {
        clean: porcelain.is_empty(),
        remote,
        branch,
        head,
    })
}

pub fn prune_sync_refs(path: &Path) -> Result<Vec<String>> {
    let listed = git(path, &["tag", "-l", "sync-*"])?;
    let mut removed = Vec::new();
    for tag in listed.lines().filter(|line| !line.is_empty()) {
        git(path, &["tag", "-d", tag])?;
        removed.push(tag.to_string());
    }
    Ok(removed)
}

pub fn read_file(repo: &Path, relative: &str) -> Result<String> {
    fs::read_to_string(repo.join(relative)).with_context(|| format!("read {relative}"))
}

fn git(cwd: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=Skill Hive",
            "-c",
            "user.email=hive@localhost",
            "-c",
            "core.editor=true",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "Skill Hive")
        .env("GIT_AUTHOR_EMAIL", "hive@localhost")
        .env("GIT_COMMITTER_NAME", "Skill Hive")
        .env("GIT_COMMITTER_EMAIL", "hive@localhost")
        .env("GIT_MERGE_AUTOEDIT", "no")
        .output()
        .context("git is not installed")?;
    if !output.status.success() {
        return Err(anyhow!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Init, back up, clone onto a second directory, edit both sides, sync, then
/// restore a snapshot and keep the safety tag.
pub fn round_trip(root: &Path) -> Result<PathBuf> {
    let remote = root.join("remote.git");
    init_bare(&remote)?;
    let first = root.join("device-a");
    let second = root.join("device-b");
    init_repo(&first)?;
    fs::write(first.join("skill.md"), "v1\n")?;
    commit_all(&first, "v1")?;
    set_remote(&first, &remote)?;
    push(&first)?;
    clone_repo(&remote, &second)?;
    fs::write(second.join("other.md"), "from-b\n")?;
    commit_all(&second, "from b")?;
    push(&second)?;
    fs::write(first.join("skill.md"), "v2\n")?;
    commit_all(&first, "v2")?;
    pull(&first)?;
    push(&first)?;
    pull(&second)?;
    assert_eq!(read_file(&second, "skill.md")?, "v2\n");
    assert_eq!(read_file(&first, "other.md")?, "from-b\n");
    let snap = snapshot(&first, "snap-1")?;
    let before = read_file(&first, "skill.md")?;
    fs::write(first.join("skill.md"), "v3\n")?;
    commit_all(&first, "v3")?;
    let safety = restore_snapshot(&first, &snap)?;
    assert_eq!(read_file(&first, "skill.md")?, before);
    assert!(tag_exists(&first, &safety)?, "safety tag {safety}");
    git(&first, &["tag", "sync-leaked"])?;
    let removed = prune_sync_refs(&first)?;
    assert_eq!(removed, vec!["sync-leaked".to_string()]);
    Ok(second)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_round_trip_against_local_bare_remote() {
        let tmp = tempfile::tempdir().unwrap();
        let second = round_trip(tmp.path()).unwrap();
        let state = status(&second).unwrap();
        assert_eq!(state.branch, "main");
        assert!(state.clean);
        assert!(state.remote.is_some());
    }
}
