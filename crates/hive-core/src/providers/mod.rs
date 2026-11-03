//! Model providers: CLI agents and HTTP APIs used to run evaluations, grade results and
//! re-rank recommendations.

use std::process::Command;

use crate::model::{DetectedCli, ProviderKind};

/// Find agent CLIs on PATH and read their versions.
pub fn detect_clis() -> Vec<DetectedCli> {
    let mut out = Vec::new();
    for (kind, command) in [
        (ProviderKind::ClaudeCode, "claude"),
        (ProviderKind::Codex, "codex"),
    ] {
        if let Ok(path) = which::which(command) {
            let version = Command::new(&path)
                .arg("--version")
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|v| !v.is_empty());
            out.push(DetectedCli {
                kind,
                command: command.to_string(),
                path: path.to_string_lossy().to_string(),
                version,
            });
        }
    }
    out
}
