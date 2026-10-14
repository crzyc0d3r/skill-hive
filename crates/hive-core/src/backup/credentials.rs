// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

//! Git backup credentials. macOS and Windows use the OS keyring under the
//! service name `skill-hive-git-backup`. A Linux runner without a Secret
//! Service uses the labelled AES-256-GCM fallback.

use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};

use crate::util::crypto;

pub const SERVICE: &str = "skill-hive-git-backup";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialKind {
    Keyring,
    LabelledFallback,
}

pub fn selected_backend() -> CredentialKind {
    let linux_without_secret_service = cfg!(target_os = "linux")
        && std::env::var("SKILL_HIVE_SECRET_SERVICE").ok().as_deref() != Some("1");
    if linux_without_secret_service {
        CredentialKind::LabelledFallback
    } else {
        CredentialKind::Keyring
    }
}

pub fn store_credential(
    account: &str,
    secret: &str,
    fallback_dir: &Path,
) -> Result<CredentialKind> {
    match selected_backend() {
        CredentialKind::Keyring => {
            keyring_set(account, secret)?;
            Ok(CredentialKind::Keyring)
        }
        CredentialKind::LabelledFallback => {
            write_fallback(account, secret, fallback_dir)?;
            Ok(CredentialKind::LabelledFallback)
        }
    }
}

pub fn load_credential(account: &str, fallback_dir: &Path) -> Result<String> {
    match selected_backend() {
        CredentialKind::Keyring => keyring_get(account),
        CredentialKind::LabelledFallback => read_fallback(account, fallback_dir),
    }
}

pub fn delete_credential(account: &str, fallback_dir: &Path) -> Result<()> {
    match selected_backend() {
        CredentialKind::Keyring => {
            let entry = keyring::Entry::new(SERVICE, account)?;
            let _ = entry.delete_credential();
            Ok(())
        }
        CredentialKind::LabelledFallback => {
            let path = fallback_path(fallback_dir, account);
            if path.exists() {
                fs::remove_file(path)?;
            }
            Ok(())
        }
    }
}

fn keyring_set(account: &str, secret: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, account).context("keyring entry")?;
    entry.set_password(secret).context("keyring set")?;
    Ok(())
}

fn keyring_get(account: &str) -> Result<String> {
    let entry = keyring::Entry::new(SERVICE, account).context("keyring entry")?;
    entry.get_password().context("keyring get")
}

fn fallback_path(dir: &Path, account: &str) -> std::path::PathBuf {
    let safe: String = account
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect();
    dir.join(format!("{safe}.credential"))
}

fn write_fallback(account: &str, secret: &str, dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    let key = crypto::load_or_create_key(&dir.join(".secret.key"))?;
    let payload = format!("{SERVICE}\n{account}\n{secret}");
    let ciphertext = crypto::encrypt(&key, &payload)?;
    let labelled = format!("label={SERVICE}\naccount={account}\n{ciphertext}\n");
    fs::write(fallback_path(dir, account), labelled)?;
    Ok(())
}

fn read_fallback(account: &str, dir: &Path) -> Result<String> {
    let raw =
        fs::read_to_string(fallback_path(dir, account)).context("read labelled credential")?;
    if !raw.starts_with(&format!("label={SERVICE}\n")) {
        return Err(anyhow!("credential file is missing the {SERVICE} label"));
    }
    let ciphertext = raw
        .lines()
        .nth(2)
        .ok_or_else(|| anyhow!("credential file is truncated"))?;
    let key = crypto::load_or_create_key(&dir.join(".secret.key"))?;
    let payload = crypto::decrypt(&key, ciphertext)?;
    let mut lines = payload.lines();
    let label = lines.next().unwrap_or("");
    let stored_account = lines.next().unwrap_or("");
    let secret = lines.next().unwrap_or("");
    if label != SERVICE || stored_account != account {
        return Err(anyhow!("labelled credential does not match"));
    }
    Ok(secret.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_matches_platform_policy() {
        let kind = selected_backend();
        if cfg!(target_os = "linux")
            && std::env::var("SKILL_HIVE_SECRET_SERVICE").ok().as_deref() != Some("1")
        {
            assert_eq!(kind, CredentialKind::LabelledFallback);
        } else {
            assert_eq!(kind, CredentialKind::Keyring);
        }
    }

    #[test]
    fn credentials_round_trip_on_the_platform_backend() {
        let tmp = tempfile::tempdir().unwrap();
        let account = format!("backup-{}", std::process::id());
        let secret = "local-bare-remote-token";
        let kind = store_credential(&account, secret, tmp.path()).unwrap();
        if cfg!(target_os = "linux")
            && std::env::var("SKILL_HIVE_SECRET_SERVICE").ok().as_deref() != Some("1")
        {
            assert_eq!(kind, CredentialKind::LabelledFallback);
            let stored = fs::read_to_string(fallback_path(tmp.path(), &account)).unwrap();
            assert!(stored.contains("label=skill-hive-git-backup"));
            assert!(!stored.contains(secret));
        } else {
            assert_eq!(kind, CredentialKind::Keyring);
        }
        assert_eq!(load_credential(&account, tmp.path()).unwrap(), secret);
        delete_credential(&account, tmp.path()).unwrap();
    }
}
