//! Shallow clones through the `git` command line, cached under the Hive home.
//!
//! The system `git` is used on purpose: it honours the user's proxies, credential helpers and SSH
//! keys, works with every host, and avoids bundling OpenSSL.

use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio::process::Command;

use crate::error::{HiveError, IoContext, Result};
use crate::hash;
use crate::util;

const CLONE_TIMEOUT: Duration = Duration::from_secs(600);
const QUICK_TIMEOUT: Duration = Duration::from_secs(90);

/// Metadata kept next to each cached clone.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CloneInfo {
    pub url: String,
    pub reference: Option<String>,
    pub revision: String,
    pub updated_at: i64,
}

pub fn git_available() -> bool {
    which::which("git").is_ok()
}

/// Cache directory for a clone URL + reference.
pub fn cache_dir_for(cache_root: &Path, url: &str, reference: Option<&str>) -> PathBuf {
    let key = hash::hash_bytes(format!("{url}\n{}", reference.unwrap_or("")).as_bytes());
    cache_root.join(&key[..20])
}

async fn run_git(args: &[&str], cwd: Option<&Path>, timeout: Duration) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_LFS_SKIP_SMUDGE", "1")
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true);
    crate::util::hide_console(&mut cmd);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let output = tokio::time::timeout(timeout, cmd.output())
        .await
        .map_err(|_| HiveError::Timeout(timeout.as_secs()))?
        .map_err(|e| HiveError::msg(format!("failed to start git: {e}")))?;
    if !output.status.success() {
        return Err(HiveError::Command {
            command: format!("git {}", args.join(" ")),
            status: output.status.code().unwrap_or(-1),
            stderr: sanitize(&String::from_utf8_lossy(&output.stderr)),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Strip anything that looks like a token from git's stderr before it reaches logs or the UI.
fn sanitize(text: &str) -> String {
    let re = regex::Regex::new(r"(?i)(https?://)[^/\s:@]+:[^/\s@]+@").expect("static regex");
    re.replace_all(text.trim(), "$1***@").to_string()
}

/// Remote HEAD (or the given reference) commit without cloning.
pub async fn remote_revision(url: &str, reference: Option<&str>) -> Result<Option<String>> {
    let mut args = vec!["ls-remote", url];
    let refspec;
    match reference {
        Some(r) => {
            refspec = r.to_string();
            args.push(&refspec);
        }
        None => args.push("HEAD"),
    }
    let out = run_git(&args, None, QUICK_TIMEOUT).await?;
    // Prefer an exact branch/tag match, then anything.
    let mut best: Option<String> = None;
    for line in out.lines() {
        let mut parts = line.split_whitespace();
        let (Some(sha), Some(name)) = (parts.next(), parts.next()) else {
            continue;
        };
        let exact = match reference {
            Some(r) => {
                name == format!("refs/heads/{r}") || name == format!("refs/tags/{r}") || name == r
            }
            None => name == "HEAD",
        };
        if exact {
            return Ok(Some(sha.to_string()));
        }
        if best.is_none() {
            best = Some(sha.to_string());
        }
    }
    Ok(best)
}

/// Clone (or refresh) `url` at `reference` into the cache. Returns the checkout directory and
/// the commit that was checked out.
pub async fn clone_or_update(
    cache_root: &Path,
    url: &str,
    reference: Option<&str>,
) -> Result<(PathBuf, String)> {
    if !git_available() {
        return Err(HiveError::msg("git is not installed or not on PATH"));
    }
    std::fs::create_dir_all(cache_root).at(cache_root)?;
    let dir = cache_dir_for(cache_root, url, reference);
    let info_path = dir.join(".hive-clone.json");
    if dir.join(".git").exists() {
        // Refresh in place; on any failure fall back to a fresh clone.
        let refreshed = async {
            let target = reference.unwrap_or("HEAD");
            run_git(
                &["fetch", "--depth", "1", "origin", target],
                Some(&dir),
                CLONE_TIMEOUT,
            )
            .await?;
            run_git(
                &["reset", "--hard", "FETCH_HEAD"],
                Some(&dir),
                QUICK_TIMEOUT,
            )
            .await?;
            run_git(
                &["clean", "-fdx", "-e", ".hive-clone.json"],
                Some(&dir),
                QUICK_TIMEOUT,
            )
            .await?;
            Ok::<(), HiveError>(())
        }
        .await;
        if refreshed.is_err() {
            util::remove_path(&dir)?;
        }
    }
    if !dir.join(".git").exists() {
        let _ = util::remove_path(&dir);
        let dir_str = dir.to_string_lossy().to_string();
        let mut args = vec!["clone", "--depth", "1", "--quiet", "--no-tags"];
        if let Some(r) = reference {
            args.push("--branch");
            args.push(r);
        }
        args.push(url);
        args.push(&dir_str);
        if let Err(err) = run_git(&args, None, CLONE_TIMEOUT).await {
            // A commit sha cannot be passed to --branch; fetch it explicitly.
            if reference
                .map(|r| r.len() >= 7 && r.chars().all(|c| c.is_ascii_hexdigit()))
                .unwrap_or(false)
            {
                let _ = util::remove_path(&dir);
                std::fs::create_dir_all(&dir).at(&dir)?;
                run_git(&["init", "--quiet"], Some(&dir), QUICK_TIMEOUT).await?;
                run_git(&["remote", "add", "origin", url], Some(&dir), QUICK_TIMEOUT).await?;
                run_git(
                    &["fetch", "--depth", "1", "origin", reference.unwrap()],
                    Some(&dir),
                    CLONE_TIMEOUT,
                )
                .await?;
                run_git(
                    &["checkout", "--quiet", "FETCH_HEAD"],
                    Some(&dir),
                    QUICK_TIMEOUT,
                )
                .await?;
            } else {
                let _ = util::remove_path(&dir);
                return Err(err);
            }
        }
    }
    let revision = run_git(&["rev-parse", "HEAD"], Some(&dir), QUICK_TIMEOUT)
        .await?
        .trim()
        .to_string();
    let info = CloneInfo {
        url: url.to_string(),
        reference: reference.map(|s| s.to_string()),
        revision: revision.clone(),
        updated_at: util::now_ms(),
    };
    util::write_json_atomic(&info_path, &info)?;
    Ok((dir, revision))
}

/// Drop every cached clone.
pub fn clear_cache(cache_root: &Path) -> Result<u64> {
    let size = util::dir_size(cache_root);
    if cache_root.exists() {
        util::remove_path(cache_root)?;
        std::fs::create_dir_all(cache_root).at(cache_root)?;
    }
    Ok(size)
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    fn git(args: &[&str], cwd: &Path) {
        let out = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@example.com")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@example.com")
            .output()
            .expect("git runs");
        assert!(
            out.status.success(),
            "git {:?}: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// Create a repository with a `skills/<name>` skill and return its path (usable as a clone URL).
    pub fn make_repo(root: &Path) -> PathBuf {
        let repo = root.join("remote-repo");
        std::fs::create_dir_all(&repo).unwrap();
        git(&["init", "--quiet", "-b", "main"], &repo);
        crate::library::test_support::write_skill(
            &repo.join("skills"),
            "remote-skill",
            "A remote skill that formats commit messages when committing",
            "Body v1",
        );
        std::fs::write(
            repo.join("README.md"),
            "# Remote repo\nSkills live in skills/.",
        )
        .unwrap();
        git(&["add", "-A"], &repo);
        git(&["commit", "--quiet", "-m", "v1"], &repo);
        repo
    }

    pub fn commit_change(repo: &Path, body: &str) {
        crate::library::test_support::write_skill(
            &repo.join("skills"),
            "remote-skill",
            "A remote skill that formats commit messages when committing",
            body,
        );
        git(&["add", "-A"], repo);
        git(&["commit", "--quiet", "-m", "update"], repo);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn clone_refresh_and_remote_revision() {
        if !git_available() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let repo = test_support::make_repo(tmp.path());
        let url = repo.to_string_lossy().to_string();
        let cache = tmp.path().join("cache");
        let (dir, rev1) = clone_or_update(&cache, &url, None).await.unwrap();
        assert!(dir.join("skills/remote-skill/SKILL.md").is_file());
        assert_eq!(
            remote_revision(&url, None).await.unwrap().as_deref(),
            Some(rev1.as_str())
        );

        test_support::commit_change(&repo, "Body v2");
        let rev2 = remote_revision(&url, Some("main")).await.unwrap().unwrap();
        assert_ne!(rev1, rev2);
        let (dir2, rev) = clone_or_update(&cache, &url, None).await.unwrap();
        assert_eq!(dir2, dir);
        assert_eq!(rev, rev2);
        assert!(
            std::fs::read_to_string(dir.join("skills/remote-skill/SKILL.md"))
                .unwrap()
                .contains("Body v2")
        );

        // A specific commit works too.
        let (dir3, rev3) = clone_or_update(&cache, &url, Some(&rev1)).await.unwrap();
        assert_ne!(dir3, dir);
        assert_eq!(rev3, rev1);
        assert!(clear_cache(&cache).unwrap() > 0);
        assert!(cache.exists());
    }

    #[test]
    fn stderr_is_sanitized() {
        assert_eq!(
            sanitize("fatal: https://user:tok@github.com/x"),
            "fatal: https://***@github.com/x"
        );
    }
}
