//! Child-process plumbing shared by the CLI providers: spawn, stream stdout line by line,
//! enforce a timeout and a cancellation token, and always reap the child.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use crate::error::{HiveError, Result};

/// Bytes of stderr kept for error messages.
pub const STDERR_CAP: usize = 64 * 1024;

/// Outcome of a process that ran to completion.
#[derive(Debug, Clone, Default)]
pub struct ProcessOutput {
    /// Exit code, `None` when killed by a signal.
    pub status: Option<i32>,
    pub success: bool,
    /// Captured stdout (only filled by [`run_capture`]).
    pub stdout: String,
    /// Captured stderr, capped at [`STDERR_CAP`].
    pub stderr: String,
    pub duration_ms: u64,
}

/// Resolve an executable name through `PATH`. Paths are returned unchanged; unknown names are
/// returned unchanged too, so the spawn error names them.
pub fn resolve_executable(name: &str) -> PathBuf {
    let p = Path::new(name);
    if p.is_absolute() || p.components().count() > 1 {
        return p.to_path_buf();
    }
    which::which(name).unwrap_or_else(|_| p.to_path_buf())
}

/// Build a command for `program`. On Windows the npm `.cmd`/`.bat` shims that Claude Code and
/// Codex install run through `cmd /C`.
pub fn command(program: &str) -> Command {
    let path = resolve_executable(program);
    #[cfg(windows)]
    {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());
        if matches!(ext.as_deref(), Some("cmd") | Some("bat")) {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(path);
            return c;
        }
    }
    Command::new(path)
}

/// Last `max` characters of a string, trimmed (for error messages).
pub fn tail(text: &str, max: usize) -> String {
    let trimmed = text.trim();
    let count = trimmed.chars().count();
    if count <= max {
        return trimmed.to_string();
    }
    let skip = count - max;
    format!("…{}", trimmed.chars().skip(skip).collect::<String>())
}

async fn read_capped<R: tokio::io::AsyncRead + Unpin>(mut reader: R, cap: usize) -> String {
    let mut kept = Vec::with_capacity(4096);
    let mut buf = [0u8; 4096];
    loop {
        match reader.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                if kept.len() < cap {
                    let take = n.min(cap - kept.len());
                    kept.extend_from_slice(&buf[..take]);
                }
            }
        }
    }
    String::from_utf8_lossy(&kept).to_string()
}

/// Run `cmd`, calling `on_line` for every stdout line as it arrives. Stdin is closed. The
/// child is killed on timeout ([`HiveError::Timeout`]) and on cancellation
/// ([`HiveError::Cancelled`]); a non-zero exit is reported in the output, not as an error.
pub async fn run_streaming(
    mut cmd: Command,
    timeout: Duration,
    cancel: &CancellationToken,
    mut on_line: impl FnMut(&str),
) -> Result<ProcessOutput> {
    let started = Instant::now();
    let label = cmd.as_std().get_program().to_string_lossy().to_string();
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = cmd
        .spawn()
        .map_err(|e| HiveError::msg(format!("could not start `{label}`: {e}")))?;
    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    let stderr_task = tokio::spawn(read_capped(stderr, STDERR_CAP));
    let mut lines = BufReader::new(stdout).lines();
    let deadline = tokio::time::sleep(timeout);
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            line = lines.next_line() => match line {
                Ok(Some(l)) => on_line(&l),
                Ok(None) => break,
                Err(e) => {
                    let _ = child.kill().await;
                    stderr_task.abort();
                    return Err(HiveError::msg(format!("reading `{label}` output: {e}")));
                }
            },
            _ = cancel.cancelled() => {
                let _ = child.kill().await;
                stderr_task.abort();
                return Err(HiveError::Cancelled);
            }
            _ = &mut deadline => {
                let _ = child.kill().await;
                stderr_task.abort();
                return Err(HiveError::Timeout(timeout.as_secs()));
            }
        }
    }
    // stdout is closed; give the process what is left of the timeout (at least 5 s) to exit.
    let remaining = timeout
        .saturating_sub(started.elapsed())
        .max(Duration::from_secs(5));
    let status = tokio::select! {
        s = child.wait() => s.map_err(|e| HiveError::msg(format!("waiting for `{label}`: {e}")))?,
        _ = cancel.cancelled() => {
            let _ = child.kill().await;
            stderr_task.abort();
            return Err(HiveError::Cancelled);
        }
        _ = tokio::time::sleep(remaining) => {
            let _ = child.kill().await;
            stderr_task.abort();
            return Err(HiveError::Timeout(timeout.as_secs()));
        }
    };
    let stderr = stderr_task.await.unwrap_or_default();
    Ok(ProcessOutput {
        status: status.code(),
        success: status.success(),
        stdout: String::new(),
        stderr,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

/// Run `cmd` and capture all of stdout.
pub async fn run_capture(
    cmd: Command,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<ProcessOutput> {
    let mut out = String::new();
    let mut result = run_streaming(cmd, timeout, cancel, |line| {
        out.push_str(line);
        out.push('\n');
    })
    .await?;
    result.stdout = out;
    Ok(result)
}

/// Run `<program> --version` and return the first line of its output.
pub async fn version_of(program: &str, timeout: Duration) -> Result<String> {
    let mut cmd = command(program);
    cmd.arg("--version");
    let out = run_capture(cmd, timeout, &CancellationToken::new()).await?;
    if !out.success {
        return Err(HiveError::Command {
            command: format!("{program} --version"),
            status: out.status.unwrap_or(-1),
            stderr: tail(&out.stderr, 400),
        });
    }
    Ok(out
        .stdout
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or_default()
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_keeps_the_end() {
        assert_eq!(tail("  abcdef  ", 10), "abcdef");
        assert_eq!(tail("abcdef", 3), "…def");
    }

    #[test]
    fn resolves_paths_unchanged() {
        assert_eq!(
            resolve_executable("/usr/bin/definitely-missing"),
            PathBuf::from("/usr/bin/definitely-missing")
        );
        assert_eq!(
            resolve_executable("definitely-missing-binary-xyz"),
            PathBuf::from("definitely-missing-binary-xyz")
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn streams_lines_and_reports_status() {
        let mut cmd = command("sh");
        cmd.arg("-c")
            .arg("echo one; echo two; echo err >&2; exit 3");
        let mut seen = Vec::new();
        let out = run_streaming(
            cmd,
            Duration::from_secs(10),
            &CancellationToken::new(),
            |l| seen.push(l.to_string()),
        )
        .await
        .unwrap();
        assert_eq!(seen, vec!["one", "two"]);
        assert_eq!(out.status, Some(3));
        assert!(!out.success);
        assert_eq!(out.stderr.trim(), "err");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn timeout_and_cancel_kill_the_child() {
        let mut cmd = command("sleep");
        cmd.arg("30");
        let err = run_capture(cmd, Duration::from_millis(200), &CancellationToken::new())
            .await
            .unwrap_err();
        assert!(matches!(err, HiveError::Timeout(_)), "{err}");

        let mut cmd = command("sleep");
        cmd.arg("30");
        let token = CancellationToken::new();
        let t2 = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            t2.cancel();
        });
        let err = run_capture(cmd, Duration::from_secs(30), &token)
            .await
            .unwrap_err();
        assert!(matches!(err, HiveError::Cancelled), "{err}");
    }

    #[tokio::test]
    async fn missing_program_is_an_error() {
        let cmd = command("definitely-missing-binary-xyz");
        assert!(
            run_capture(cmd, Duration::from_secs(5), &CancellationToken::new())
                .await
                .is_err()
        );
    }
}
