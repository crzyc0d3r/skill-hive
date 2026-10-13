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
}
