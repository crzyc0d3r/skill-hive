// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

//! Update checks, held-back file removals, and the auto-update schedule.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::library::projects::normalize_newlines;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateStatus {
    Current,
    UpdateAvailable,
    PendingRemovals,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCheck {
    pub status: UpdateStatus,
    pub pending_removals: Vec<String>,
    pub token: Option<String>,
    pub message: String,
}

pub fn check_replacement(
    local: &BTreeMap<String, String>,
    remote: &BTreeMap<String, String>,
) -> UpdateCheck {
    let mut removals = Vec::new();
    for path in local.keys() {
        if ignored_removal(path) {
            continue;
        }
        if !remote.contains_key(path) {
            removals.push(path.clone());
        }
    }
    if !removals.is_empty() {
        let token = approval_token(&removals);
        return UpdateCheck {
            status: UpdateStatus::PendingRemovals,
            message: "This update removes files".into(),
            token: Some(token),
            pending_removals: removals,
        };
    }
    let changed = files_differ(local, remote);
    if changed {
        UpdateCheck {
            status: UpdateStatus::UpdateAvailable,
            pending_removals: Vec::new(),
            token: None,
            message: "Update".into(),
        }
    } else {
        UpdateCheck {
            status: UpdateStatus::Current,
            pending_removals: Vec::new(),
            token: None,
            message: "Up to date".into(),
        }
    }
}

fn ignored_removal(path: &str) -> bool {
    path.split(['/', '\\'])
        .any(|part| part == "__pycache__" || part.ends_with(".pyc"))
}

fn files_differ(local: &BTreeMap<String, String>, remote: &BTreeMap<String, String>) -> bool {
    for (path, remote_body) in remote {
        if ignored_removal(path) {
            continue;
        }
        match local.get(path) {
            Some(local_body)
                if normalize_newlines(local_body) == normalize_newlines(remote_body) => {}
            _ => return true,
        }
    }
    false
}

fn approval_token(paths: &[String]) -> String {
    let mut hasher = Sha256::new();
    for path in paths {
        hasher.update(path.as_bytes());
        hasher.update(b"\n");
    }
    hex::encode(hasher.finalize())
}

/// Apply a checked update. Pending removals apply only when the token matches.
/// A mismatch or a missing token leaves `local` unchanged and returns false.
pub fn apply_replacement(
    local: &mut BTreeMap<String, String>,
    remote: &BTreeMap<String, String>,
    check: &UpdateCheck,
    token: Option<&str>,
) -> bool {
    if check.status == UpdateStatus::PendingRemovals {
        match (check.token.as_deref(), token) {
            (Some(expected), Some(given)) if expected == given => {}
            _ => return false,
        }
    }
    if check.status == UpdateStatus::Failed {
        return false;
    }
    let kept: BTreeMap<String, String> = local
        .iter()
        .filter(|(path, _)| ignored_removal(path))
        .map(|(path, body)| (path.clone(), body.clone()))
        .collect();
    *local = remote.clone();
    for (path, body) in kept {
        local.entry(path).or_insert(body);
    }
    true
}

#[derive(Debug, Clone, Copy)]
pub struct AutoUpdateSchedule {
    pub quiet_secs: u64,
    pub launch_round_secs: u64,
}

impl Default for AutoUpdateSchedule {
    fn default() -> Self {
        Self {
            quiet_secs: 120,
            launch_round_secs: 90,
        }
    }
}

pub fn startup_round_due(started_at: u64, now: u64, already_ran: bool) -> bool {
    !already_ran
        && now.saturating_sub(started_at) >= AutoUpdateSchedule::default().launch_round_secs
}

pub fn quiet_period_elapsed(last_change: u64, now: u64) -> bool {
    now.saturating_sub(last_change) >= AutoUpdateSchedule::default().quiet_secs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn replacement_that_drops_a_file_returns_pending_removals_and_applies_nothing() {
        let local = map(&[("SKILL.md", "v1"), ("notes.txt", "keep")]);
        let remote = map(&[("SKILL.md", "v2")]);
        let check = check_replacement(&local, &remote);
        assert_eq!(check.status, UpdateStatus::PendingRemovals);
        assert_eq!(check.pending_removals, vec!["notes.txt".to_string()]);
        let mut applied = local.clone();
        assert!(!apply_replacement(&mut applied, &remote, &check, None));
        assert_eq!(applied, local);
    }

    #[test]
    fn retrying_with_the_token_applies() {
        let local = map(&[("SKILL.md", "v1"), ("notes.txt", "keep")]);
        let remote = map(&[("SKILL.md", "v2")]);
        let check = check_replacement(&local, &remote);
        let mut applied = local.clone();
        assert!(apply_replacement(
            &mut applied,
            &remote,
            &check,
            check.token.as_deref()
        ));
        assert_eq!(applied.get("SKILL.md").map(String::as_str), Some("v2"));
        assert!(!applied.contains_key("notes.txt"));
    }

    #[test]
    fn pycache_removals_are_ignored() {
        let local = map(&[("SKILL.md", "v1"), ("pkg/__pycache__/x.pyc", "junk")]);
        let remote = map(&[("SKILL.md", "v1")]);
        let check = check_replacement(&local, &remote);
        assert_eq!(check.status, UpdateStatus::Current);
        assert!(check.pending_removals.is_empty());
    }

    #[test]
    fn line_ending_only_difference_reports_current() {
        let local = map(&[("SKILL.md", "hello\r\nworld\r\n")]);
        let remote = map(&[("SKILL.md", "hello\nworld\n")]);
        let check = check_replacement(&local, &remote);
        assert_eq!(check.status, UpdateStatus::Current);
    }

    #[test]
    fn schedule_uses_the_reference_delays() {
        let schedule = AutoUpdateSchedule::default();
        assert_eq!(schedule.quiet_secs, 120);
        assert_eq!(schedule.launch_round_secs, 90);
        assert!(!startup_round_due(0, 89, false));
        assert!(startup_round_due(0, 90, false));
        assert!(!startup_round_due(0, 200, true));
        assert!(quiet_period_elapsed(0, 120));
    }
}
