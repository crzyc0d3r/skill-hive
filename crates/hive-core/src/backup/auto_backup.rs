// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

//! Debounced automatic backup: 120s of quiet, exponential backoff, and a 90s
//! round after launch.

#[derive(Debug, Clone, Copy)]
pub struct AutoBackupPolicy {
    pub quiet_secs: u64,
    pub launch_round_secs: u64,
    pub backoff_base_secs: u64,
}

impl Default for AutoBackupPolicy {
    fn default() -> Self {
        Self {
            quiet_secs: 120,
            launch_round_secs: 90,
            backoff_base_secs: 30,
        }
    }
}

pub fn backoff_delay(policy: AutoBackupPolicy, failures: u32) -> u64 {
    let shift = failures.min(6);
    policy.backoff_base_secs.saturating_mul(1u64 << shift)
}

pub fn maybe_run_auto_backup(
    store: &crate::db::skill_store::SkillStore,
    repo: &std::path::Path,
    now_secs: u64,
) -> anyhow::Result<bool> {
    let enabled = store
        .get_setting("backup_auto")
        .ok()
        .flatten()
        .unwrap_or_default();
    if enabled != "1" && enabled != "true" {
        return Ok(false);
    }
    let started = setting_u64(store, "backup_auto_started_at").unwrap_or(now_secs);
    if store
        .get_setting("backup_auto_started_at")
        .ok()
        .flatten()
        .is_none()
    {
        store.set_setting("backup_auto_started_at", &started.to_string())?;
    }
    let last_change = setting_u64(store, "backup_auto_last_change");
    let last_attempt = setting_u64(store, "backup_auto_last_attempt");
    let failures = setting_u64(store, "backup_auto_failures").unwrap_or(0) as u32;
    let launch_done = store
        .get_setting("backup_auto_launch_done")
        .ok()
        .flatten()
        .as_deref()
        == Some("1");
    if !due(
        AutoBackupPolicy::default(),
        started,
        last_change,
        last_attempt,
        failures,
        now_secs,
        launch_done,
    ) {
        return Ok(false);
    }
    let committed = match crate::core::git_backup::commit_all(repo, "automatic backup") {
        Ok(()) => true,
        Err(err) if err.to_string().contains("Nothing to commit") => false,
        Err(err) => {
            let _ = store.set_setting(
                "backup_auto_failures",
                &failures.saturating_add(1).to_string(),
            );
            let _ = store.set_setting("backup_auto_last_attempt", &now_secs.to_string());
            return Err(err);
        }
    };
    store.set_setting("backup_auto_launch_done", "1")?;
    store.set_setting("backup_auto_failures", "0")?;
    store.set_setting("backup_auto_last_attempt", &now_secs.to_string())?;
    Ok(committed)
}

fn setting_u64(store: &crate::db::skill_store::SkillStore, key: &str) -> Option<u64> {
    store
        .get_setting(key)
        .ok()
        .flatten()
        .and_then(|value| value.parse().ok())
}

pub fn due(
    policy: AutoBackupPolicy,
    started_at: u64,
    last_change: Option<u64>,
    last_attempt: Option<u64>,
    failures: u32,
    now: u64,
    launch_round_done: bool,
) -> bool {
    if !launch_round_done && now.saturating_sub(started_at) >= policy.launch_round_secs {
        return true;
    }
    let Some(changed) = last_change else {
        return false;
    };
    if now.saturating_sub(changed) < policy.quiet_secs {
        return false;
    }
    match last_attempt {
        None => true,
        Some(attempt) => now.saturating_sub(attempt) >= backoff_delay(policy, failures),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_period_and_backoff() {
        let policy = AutoBackupPolicy::default();
        assert_eq!(policy.quiet_secs, 120);
        assert_eq!(policy.launch_round_secs, 90);
        assert!(!due(policy, 0, Some(0), None, 0, 119, true));
        assert!(due(policy, 0, Some(0), None, 0, 120, true));
        assert!(!due(policy, 0, Some(0), Some(120), 2, 200, true));
        assert!(due(policy, 0, None, None, 0, 90, false));
        assert_eq!(backoff_delay(policy, 2), 120);
    }

    #[test]
    fn a_due_round_commits_the_backup_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("skills");
        std::fs::create_dir_all(&repo).unwrap();
        crate::core::git_backup::init_repo(&repo, "hive-test").unwrap();
        std::fs::write(repo.join("note.txt"), "changed\n").unwrap();
        let store = crate::db::skill_store::SkillStore::new(&tmp.path().join("hive.db")).unwrap();
        store.set_setting("backup_auto", "1").unwrap();
        store.set_setting("backup_auto_started_at", "0").unwrap();
        assert!(!maybe_run_auto_backup(&store, &repo, 10).unwrap());
        assert!(maybe_run_auto_backup(&store, &repo, 90).unwrap());
        let log = std::process::Command::new("git")
            .args(["log", "-1", "--pretty=%s"])
            .current_dir(&repo)
            .output()
            .unwrap();
        let subject = String::from_utf8_lossy(&log.stdout);
        assert!(subject.contains("automatic backup"), "{subject}");
        assert!(std::fs::read_to_string(repo.join("note.txt"))
            .unwrap()
            .contains("changed"));
        assert_eq!(
            store
                .get_setting("backup_auto_launch_done")
                .unwrap()
                .as_deref(),
            Some("1")
        );
        assert!(!maybe_run_auto_backup(&store, &repo, 100).unwrap());
    }
}
