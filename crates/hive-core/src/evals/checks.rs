//! Deterministic checks: verified against the sandbox directory and the final output without a
//! model. Paths are relative to the sandbox and may not escape it.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use crate::model::{Check, CheckKind, ExpectationResult, ExpectationSource};
use crate::util;

/// Wall-clock limit for a `Command` check.
pub const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
/// Characters of a match or of command output quoted as evidence.
pub const EVIDENCE_CHARS: usize = 240;
/// Largest file a `FileContains`/`FileMatches`/`JsonPointer` check reads.
pub const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;

/// Human description of a check, used as the expectation text when the check has none.
pub fn describe(kind: &CheckKind) -> String {
    match kind {
        CheckKind::FileExists { path } => format!("file `{path}` exists"),
        CheckKind::FileContains { path, text } => format!("file `{path}` contains {text:?}"),
        CheckKind::FileMatches { path, pattern } => format!("file `{path}` matches /{pattern}/"),
        CheckKind::OutputContains { text } => format!("output contains {text:?}"),
        CheckKind::OutputMatches { pattern } => format!("output matches /{pattern}/"),
        CheckKind::JsonPointer {
            path,
            pointer,
            equals,
        } => format!("`{path}` has {equals} at `{pointer}`"),
        CheckKind::Command { command, args } => {
            if args.is_empty() {
                format!("`{command}` exits successfully")
            } else {
                format!("`{command} {}` exits successfully", args.join(" "))
            }
        }
    }
}

/// The expectation text a check's result carries.
pub fn check_text(check: &Check) -> String {
    check
        .expectation
        .clone()
        .filter(|e| !e.trim().is_empty())
        .unwrap_or_else(|| describe(&check.kind))
}

fn result(check: &Check, passed: bool, evidence: String) -> ExpectationResult {
    ExpectationResult {
        text: check_text(check),
        passed,
        evidence,
        source: ExpectationSource::Check,
    }
}

/// Resolve a sandbox-relative path, refusing escapes (lexically and through symlinks).
pub fn resolve(sandbox: &Path, rel: &str) -> std::result::Result<PathBuf, String> {
    if !util::is_safe_relative(rel) {
        return Err(format!("path `{rel}` escapes the sandbox"));
    }
    let candidate = sandbox.join(rel);
    if candidate.exists() {
        let root = dunce::canonicalize(sandbox).unwrap_or_else(|_| sandbox.to_path_buf());
        let real = dunce::canonicalize(&candidate).unwrap_or_else(|_| candidate.clone());
        if !real.starts_with(&root) {
            return Err(format!("path `{rel}` resolves outside the sandbox"));
        }
    }
    Ok(candidate)
}

fn read_text(sandbox: &Path, rel: &str) -> std::result::Result<String, String> {
    let path = resolve(sandbox, rel)?;
    let meta = std::fs::metadata(&path).map_err(|_| format!("`{rel}` does not exist"))?;
    if !meta.is_file() {
        return Err(format!("`{rel}` is not a file"));
    }
    if meta.len() > MAX_FILE_BYTES {
        return Err(format!(
            "`{rel}` is {} and too large to inspect",
            util::human_bytes(meta.len())
        ));
    }
    let bytes = std::fs::read(&path).map_err(|e| format!("could not read `{rel}`: {e}"))?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

fn excerpt(text: &str) -> String {
    util::truncate_chars(text.trim(), EVIDENCE_CHARS).replace('\n', "\\n")
}

fn contains(haystack: &str, needle: &str, what: &str) -> (bool, String) {
    match haystack.find(needle) {
        Some(offset) => {
            let start = haystack[..offset]
                .char_indices()
                .rev()
                .nth(40)
                .map(|(i, _)| i)
                .unwrap_or(0);
            let end = (offset + needle.len() + 40).min(haystack.len());
            let end = haystack
                .char_indices()
                .map(|(i, _)| i)
                .chain(std::iter::once(haystack.len()))
                .find(|i| *i >= end)
                .unwrap_or(haystack.len());
            (
                true,
                format!(
                    "{what} contains {needle:?} at offset {offset}: …{}…",
                    excerpt(&haystack[start..end])
                ),
            )
        }
        None => (
            false,
            format!(
                "{what} does not contain {needle:?} ({} chars; starts with {:?})",
                haystack.chars().count(),
                excerpt(&haystack.chars().take(80).collect::<String>())
            ),
        ),
    }
}

fn matches(haystack: &str, pattern: &str, what: &str) -> (bool, String) {
    let re = match regex::Regex::new(pattern) {
        Ok(re) => re,
        Err(e) => return (false, format!("invalid regex /{pattern}/: {e}")),
    };
    match re.find(haystack) {
        Some(m) => (
            true,
            format!(
                "{what} matches /{pattern}/ at offset {}: {:?}",
                m.start(),
                excerpt(m.as_str())
            ),
        ),
        None => (
            false,
            format!(
                "{what} has no match for /{pattern}/ ({} chars)",
                haystack.chars().count()
            ),
        ),
    }
}

async fn run_command(
    sandbox: &Path,
    command: &str,
    args: &[String],
    timeout: Duration,
) -> (bool, String) {
    let shown = if args.is_empty() {
        command.to_string()
    } else {
        format!("{command} {}", args.join(" "))
    };
    let mut cmd = crate::providers::process::command(command);
    cmd.args(args)
        .current_dir(sandbox)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return (false, format!("could not start `{shown}`: {e}")),
    };
    match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(Ok(out)) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let code = out
                .status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "signal".into());
            let detail = if stderr.trim().is_empty() {
                format!("stdout: {:?}", excerpt(&stdout))
            } else {
                format!(
                    "stdout: {:?}; stderr: {:?}",
                    excerpt(&stdout),
                    excerpt(&stderr)
                )
            };
            (
                out.status.success(),
                format!("`{shown}` exited with {code}; {detail}"),
            )
        }
        Ok(Err(e)) => (false, format!("`{shown}` failed: {e}")),
        Err(_) => (
            false,
            format!("`{shown}` timed out after {} s", timeout.as_secs_f64()),
        ),
    }
}

/// Evaluate one check with a custom command timeout.
pub async fn evaluate_with_timeout(
    check: &Check,
    sandbox: &Path,
    output: &str,
    command_timeout: Duration,
) -> ExpectationResult {
    let (passed, evidence) = match &check.kind {
        CheckKind::FileExists { path } => match resolve(sandbox, path) {
            Err(e) => (false, e),
            Ok(p) => match std::fs::metadata(&p) {
                Ok(meta) if meta.is_file() => (
                    true,
                    format!("`{path}` exists ({})", util::human_bytes(meta.len())),
                ),
                Ok(_) => (true, format!("`{path}` exists (directory)")),
                Err(_) => (false, format!("`{path}` does not exist")),
            },
        },
        CheckKind::FileContains { path, text } => match read_text(sandbox, path) {
            Err(e) => (false, e),
            Ok(content) => contains(&content, text, &format!("`{path}`")),
        },
        CheckKind::FileMatches { path, pattern } => match read_text(sandbox, path) {
            Err(e) => (false, e),
            Ok(content) => matches(&content, pattern, &format!("`{path}`")),
        },
        CheckKind::OutputContains { text } => contains(output, text, "output"),
        CheckKind::OutputMatches { pattern } => matches(output, pattern, "output"),
        CheckKind::JsonPointer {
            path,
            pointer,
            equals,
        } => match read_text(sandbox, path) {
            Err(e) => (false, e),
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Err(e) => (false, format!("`{path}` is not valid JSON: {e}")),
                Ok(json) => match json.pointer(pointer) {
                    None => (false, format!("`{path}` has nothing at `{pointer}`")),
                    Some(actual) if actual == equals => {
                        (true, format!("`{path}` has {actual} at `{pointer}`"))
                    }
                    Some(actual) => (
                        false,
                        format!("`{path}` has {actual} at `{pointer}`, expected {equals}"),
                    ),
                },
            },
        },
        CheckKind::Command { command, args } => {
            run_command(sandbox, command, args, command_timeout).await
        }
    };
    result(check, passed, evidence)
}

/// Evaluate one check against the sandbox and the final output.
pub async fn evaluate(check: &Check, sandbox: &Path, output: &str) -> ExpectationResult {
    evaluate_with_timeout(check, sandbox, output, COMMAND_TIMEOUT).await
}

/// Evaluate every check in order.
pub async fn run_checks(checks: &[Check], sandbox: &Path, output: &str) -> Vec<ExpectationResult> {
    let mut out = Vec::with_capacity(checks.len());
    for check in checks {
        out.push(evaluate(check, sandbox, output).await);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(kind: CheckKind) -> Check {
        Check {
            id: "c".into(),
            expectation: None,
            kind,
        }
    }

    fn sandbox() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("out")).unwrap();
        std::fs::write(
            tmp.path().join("out/report.md"),
            "# Report\nTotal: 42 units\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("data.json"),
            r#"{"summary": {"total": 42, "ok": true}, "items": ["a", "b"]}"#,
        )
        .unwrap();
        tmp
    }

    #[tokio::test]
    async fn file_checks_pass_and_fail() {
        let tmp = sandbox();
        let sb = tmp.path();
        let r = evaluate(
            &check(CheckKind::FileExists {
                path: "out/report.md".into(),
            }),
            sb,
            "",
        )
        .await;
        assert!(r.passed, "{}", r.evidence);
        assert_eq!(r.source, ExpectationSource::Check);
        assert_eq!(r.text, "file `out/report.md` exists");
        let r = evaluate(
            &check(CheckKind::FileExists {
                path: "missing.md".into(),
            }),
            sb,
            "",
        )
        .await;
        assert!(!r.passed);
        assert!(r.evidence.contains("does not exist"));

        let r = evaluate(
            &check(CheckKind::FileContains {
                path: "out/report.md".into(),
                text: "Total: 42".into(),
            }),
            sb,
            "",
        )
        .await;
        assert!(r.passed, "{}", r.evidence);
        assert!(r.evidence.contains("offset"));
        let r = evaluate(
            &check(CheckKind::FileContains {
                path: "out/report.md".into(),
                text: "Total: 43".into(),
            }),
            sb,
            "",
        )
        .await;
        assert!(!r.passed);
        let r = evaluate(
            &check(CheckKind::FileContains {
                path: "nope.md".into(),
                text: "x".into(),
            }),
            sb,
            "",
        )
        .await;
        assert!(!r.passed);

        let r = evaluate(
            &check(CheckKind::FileMatches {
                path: "out/report.md".into(),
                pattern: r"Total: \d+".into(),
            }),
            sb,
            "",
        )
        .await;
        assert!(r.passed, "{}", r.evidence);
        assert!(r.evidence.contains("Total: 42"));
        let r = evaluate(
            &check(CheckKind::FileMatches {
                path: "out/report.md".into(),
                pattern: r"^Nothing".into(),
            }),
            sb,
            "",
        )
        .await;
        assert!(!r.passed);
        let r = evaluate(
            &check(CheckKind::FileMatches {
                path: "out/report.md".into(),
                pattern: "(".into(),
            }),
            sb,
            "",
        )
        .await;
        assert!(!r.passed);
        assert!(r.evidence.contains("invalid regex"));
    }

    #[tokio::test]
    async fn output_and_json_checks() {
        let tmp = sandbox();
        let sb = tmp.path();
        let out = "Done. I wrote the report and the total is 42.";
        assert!(
            evaluate(
                &check(CheckKind::OutputContains {
                    text: "total is 42".into()
                }),
                sb,
                out
            )
            .await
            .passed
        );
        assert!(
            !evaluate(
                &check(CheckKind::OutputContains {
                    text: "failed".into()
                }),
                sb,
                out
            )
            .await
            .passed
        );
        assert!(
            evaluate(
                &check(CheckKind::OutputMatches {
                    pattern: r"(?i)done\.".into()
                }),
                sb,
                out
            )
            .await
            .passed
        );
        assert!(
            !evaluate(
                &check(CheckKind::OutputMatches {
                    pattern: r"^Nope".into()
                }),
                sb,
                out
            )
            .await
            .passed
        );

        let ok = check(CheckKind::JsonPointer {
            path: "data.json".into(),
            pointer: "/summary/total".into(),
            equals: serde_json::json!(42),
        });
        let r = evaluate(&ok, sb, "").await;
        assert!(r.passed, "{}", r.evidence);
        let wrong = check(CheckKind::JsonPointer {
            path: "data.json".into(),
            pointer: "/summary/total".into(),
            equals: serde_json::json!("42"),
        });
        let r = evaluate(&wrong, sb, "").await;
        assert!(!r.passed);
        assert!(r.evidence.contains("expected"));
        let missing = check(CheckKind::JsonPointer {
            path: "data.json".into(),
            pointer: "/nope".into(),
            equals: serde_json::json!(null),
        });
        assert!(!evaluate(&missing, sb, "").await.passed);
        let not_json = check(CheckKind::JsonPointer {
            path: "out/report.md".into(),
            pointer: "/a".into(),
            equals: serde_json::json!(1),
        });
        assert!(
            evaluate(&not_json, sb, "")
                .await
                .evidence
                .contains("not valid JSON")
        );
        let arr = check(CheckKind::JsonPointer {
            path: "data.json".into(),
            pointer: "/items/1".into(),
            equals: serde_json::json!("b"),
        });
        assert!(evaluate(&arr, sb, "").await.passed);
    }

    #[tokio::test]
    async fn path_escapes_are_rejected() {
        let tmp = sandbox();
        let sb = tmp.path();
        for kind in [
            CheckKind::FileExists {
                path: "../x".into(),
            },
            CheckKind::FileContains {
                path: "/etc/passwd".into(),
                text: "root".into(),
            },
            CheckKind::FileMatches {
                path: "a/../../b".into(),
                pattern: ".".into(),
            },
            CheckKind::JsonPointer {
                path: "../../data.json".into(),
                pointer: "/a".into(),
                equals: serde_json::json!(1),
            },
        ] {
            let r = evaluate(&check(kind), sb, "").await;
            assert!(!r.passed);
            assert!(r.evidence.contains("escapes the sandbox"), "{}", r.evidence);
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/", sb.join("rootlink")).unwrap();
            let r = evaluate(
                &check(CheckKind::FileExists {
                    path: "rootlink/etc".into(),
                }),
                sb,
                "",
            )
            .await;
            assert!(!r.passed);
            assert!(r.evidence.contains("outside the sandbox"), "{}", r.evidence);
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn command_checks_success_failure_and_timeout() {
        let tmp = sandbox();
        let sb = tmp.path();
        let r = evaluate(
            &check(CheckKind::Command {
                command: "sh".into(),
                args: vec!["-c".into(), "test -f out/report.md && echo fine".into()],
            }),
            sb,
            "",
        )
        .await;
        assert!(r.passed, "{}", r.evidence);
        assert!(r.evidence.contains("fine"));
        let r = evaluate(
            &check(CheckKind::Command {
                command: "sh".into(),
                args: vec!["-c".into(), "echo bad >&2; exit 2".into()],
            }),
            sb,
            "",
        )
        .await;
        assert!(!r.passed);
        assert!(r.evidence.contains("exited with 2"), "{}", r.evidence);
        assert!(r.evidence.contains("bad"));
        let r = evaluate(
            &check(CheckKind::Command {
                command: "definitely-missing-binary-xyz".into(),
                args: vec![],
            }),
            sb,
            "",
        )
        .await;
        assert!(!r.passed);
        assert!(r.evidence.contains("could not start"));
        let r = evaluate_with_timeout(
            &check(CheckKind::Command {
                command: "sleep".into(),
                args: vec!["20".into()],
            }),
            sb,
            "",
            Duration::from_millis(200),
        )
        .await;
        assert!(!r.passed);
        assert!(r.evidence.contains("timed out"), "{}", r.evidence);
    }

    #[tokio::test]
    async fn expectation_text_and_batch() {
        let tmp = sandbox();
        let with_text = Check {
            id: "c".into(),
            expectation: Some("The report exists".into()),
            kind: CheckKind::FileExists {
                path: "out/report.md".into(),
            },
        };
        let results = run_checks(
            &[
                with_text,
                check(CheckKind::OutputContains { text: "x".into() }),
            ],
            tmp.path(),
            "",
        )
        .await;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].text, "The report exists");
        assert!(results[0].passed);
        assert_eq!(results[1].text, "output contains \"x\"");
        assert!(!results[1].passed);
    }
}
