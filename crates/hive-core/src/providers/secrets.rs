//! Provider secrets. Service name `skill-hive-provider`. Linux without a Secret Service
//! uses the labelled AES-256-GCM fallback. `auth_mode = env` never stores a secret.

use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};

use crate::util::crypto;

pub const SERVICE: &str = "skill-hive-provider";
pub const GIT_SERVICE: &str = "skill-hive-git-backup";
pub const GIT_ACCOUNT: &str = "github";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretBackend {
    Keyring,
    LabelledFallback,
    Environment,
}

pub fn selected_backend() -> SecretBackend {
    // A headless macOS runner has no login-keychain dialog, so set_password
    // waits forever. The labelled file is the same fallback Linux uses.
    let headless_macos = cfg!(target_os = "macos") && std::env::var_os("CI").is_some();
    let linux_without_secret_service = cfg!(target_os = "linux")
        && std::env::var("SKILL_HIVE_SECRET_SERVICE").ok().as_deref() != Some("1");
    if linux_without_secret_service || headless_macos {
        SecretBackend::LabelledFallback
    } else {
        SecretBackend::Keyring
    }
}

pub fn store_provider_secret(
    account: &str,
    secret: &str,
    fallback_dir: &Path,
) -> Result<SecretBackend> {
    store_secret(SERVICE, account, secret, fallback_dir)
}

pub fn load_provider_secret(account: &str, fallback_dir: &Path) -> Result<String> {
    load_secret(SERVICE, account, fallback_dir)
}

pub fn has_provider_secret(account: &str, fallback_dir: &Path) -> bool {
    load_provider_secret(account, fallback_dir).is_ok()
}

pub fn clear_provider_secret(account: &str, fallback_dir: &Path) -> Result<()> {
    clear_secret(SERVICE, account, fallback_dir)
}

pub fn store_git_token(secret: &str, fallback_dir: &Path) -> Result<SecretBackend> {
    store_secret(GIT_SERVICE, GIT_ACCOUNT, secret, fallback_dir)
}

pub fn clear_git_token(fallback_dir: &Path) -> Result<()> {
    clear_secret(GIT_SERVICE, GIT_ACCOUNT, fallback_dir)
}

pub fn has_git_token(fallback_dir: &Path) -> bool {
    load_secret(GIT_SERVICE, GIT_ACCOUNT, fallback_dir).is_ok()
}

fn store_secret(
    service: &str,
    account: &str,
    secret: &str,
    fallback_dir: &Path,
) -> Result<SecretBackend> {
    match selected_backend() {
        SecretBackend::Keyring => {
            let entry = keyring::Entry::new(service, account).context("keyring entry")?;
            entry.set_password(secret).context("keyring set")?;
            Ok(SecretBackend::Keyring)
        }
        SecretBackend::LabelledFallback => {
            write_fallback(service, account, secret, fallback_dir)?;
            Ok(SecretBackend::LabelledFallback)
        }
        SecretBackend::Environment => Ok(SecretBackend::Environment),
    }
}

fn load_secret(service: &str, account: &str, fallback_dir: &Path) -> Result<String> {
    match selected_backend() {
        SecretBackend::Keyring => {
            let entry = keyring::Entry::new(service, account).context("keyring entry")?;
            entry.get_password().context("keyring get")
        }
        SecretBackend::LabelledFallback => read_fallback(service, account, fallback_dir),
        SecretBackend::Environment => Err(anyhow!("environment secrets are not stored")),
    }
}

fn clear_secret(service: &str, account: &str, fallback_dir: &Path) -> Result<()> {
    match selected_backend() {
        SecretBackend::Keyring => {
            let entry = keyring::Entry::new(service, account)?;
            let _ = entry.delete_credential();
            Ok(())
        }
        SecretBackend::LabelledFallback => {
            let path = fallback_path(fallback_dir, service, account);
            if path.exists() {
                fs::remove_file(path)?;
            }
            Ok(())
        }
        SecretBackend::Environment => Ok(()),
    }
}

pub fn load_env_secret(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("environment variable {name} is unset"))
}

fn fallback_path(dir: &Path, service: &str, account: &str) -> std::path::PathBuf {
    let safe = |value: &str| {
        value
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
            .collect::<String>()
    };
    dir.join(format!("{}.{}.secret", safe(service), safe(account)))
}

fn write_fallback(service: &str, account: &str, secret: &str, dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    let key = crypto::load_or_create_key(&dir.join(".secret.key"))?;
    let payload = format!("{service}\n{account}\n{secret}");
    let ciphertext = crypto::encrypt(&key, &payload)?;
    let labelled = format!("label={service}\naccount={account}\n{ciphertext}\n");
    fs::write(fallback_path(dir, service, account), labelled)?;
    Ok(())
}

fn read_fallback(service: &str, account: &str, dir: &Path) -> Result<String> {
    let raw = fs::read_to_string(fallback_path(dir, service, account))
        .context("read labelled provider secret")?;
    if !raw.starts_with(&format!("label={service}\n")) {
        return Err(anyhow!(
            "provider secret file is missing the {service} label"
        ));
    }
    let ciphertext = raw
        .lines()
        .nth(2)
        .ok_or_else(|| anyhow!("provider secret file is truncated"))?;
    let key = crypto::load_or_create_key(&dir.join(".secret.key"))?;
    let payload = crypto::decrypt(&key, ciphertext)?;
    let mut lines = payload.lines();
    let label = lines.next().unwrap_or("");
    let stored_account = lines.next().unwrap_or("");
    let secret = lines.next().unwrap_or("");
    if label != service || stored_account != account {
        return Err(anyhow!("labelled provider secret does not match"));
    }
    Ok(secret.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_without_dbus_uses_labelled_fallback_and_hides_the_secret() {
        let tmp = tempfile::tempdir().unwrap();
        let account = format!("endpoint-{}", std::process::id());
        let secret = "sk-ant-endpoint-fixture-secret";
        let kind = store_provider_secret(&account, secret, tmp.path()).unwrap();
        let headless_macos = cfg!(target_os = "macos") && std::env::var_os("CI").is_some();
        let fallback = headless_macos
            || (cfg!(target_os = "linux")
                && std::env::var("SKILL_HIVE_SECRET_SERVICE").ok().as_deref() != Some("1"));
        if fallback {
            assert_eq!(kind, SecretBackend::LabelledFallback);
            let stored = fs::read_to_string(fallback_path(tmp.path(), SERVICE, &account)).unwrap();
            assert!(stored.contains("label=skill-hive-provider"));
            assert!(!stored.contains(secret));
        } else {
            assert_eq!(kind, SecretBackend::Keyring);
        }
        assert_eq!(load_provider_secret(&account, tmp.path()).unwrap(), secret);
        assert!(has_provider_secret(&account, tmp.path()));
        clear_provider_secret(&account, tmp.path()).unwrap();
        assert!(!has_provider_secret(&account, tmp.path()));
    }
}
