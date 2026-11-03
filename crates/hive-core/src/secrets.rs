//! Secret storage for API keys and tokens.
//!
//! Uses the OS keychain when the `keyring` feature is on and the platform store answers;
//! otherwise (or when the keychain is unavailable, as on headless Linux) a JSON file under the
//! Hive home with owner-only permissions. Secrets are never written to the database or logs.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::error::{IoContext, Result};
use crate::paths::HivePaths;

const SERVICE: &str = "dev.skillhive.app";

#[derive(Debug)]
pub struct SecretStore {
    file: PathBuf,
    use_keyring: bool,
    lock: Mutex<()>,
}

impl SecretStore {
    pub fn new(paths: &HivePaths) -> Self {
        Self {
            file: paths.secrets_file(),
            use_keyring: keyring_available(),
            lock: Mutex::new(()),
        }
    }

    /// Store that only ever uses the file backend (tests, CI).
    pub fn file_only(paths: &HivePaths) -> Self {
        Self {
            file: paths.secrets_file(),
            use_keyring: false,
            lock: Mutex::new(()),
        }
    }

    pub fn backend_name(&self) -> &'static str {
        if self.use_keyring {
            "os-keychain"
        } else {
            "file"
        }
    }

    pub fn get(&self, name: &str) -> Result<Option<String>> {
        if self.use_keyring {
            if let Some(v) = keyring_get(name) {
                return Ok(Some(v));
            }
        }
        Ok(self.read_file()?.get(name).cloned())
    }

    pub fn set(&self, name: &str, value: &str) -> Result<()> {
        if value.is_empty() {
            return self.delete(name);
        }
        if self.use_keyring && keyring_set(name, value) {
            // Make sure no stale file copy lingers.
            let _ = self.remove_from_file(name);
            return Ok(());
        }
        let _guard = self.lock.lock().unwrap_or_else(|p| p.into_inner());
        let mut map = self.read_file()?;
        map.insert(name.to_string(), value.to_string());
        self.write_file(&map)
    }

    pub fn delete(&self, name: &str) -> Result<()> {
        if self.use_keyring {
            keyring_delete(name);
        }
        self.remove_from_file(name)
    }

    pub fn has(&self, name: &str) -> bool {
        self.get(name).ok().flatten().is_some()
    }

    fn remove_from_file(&self, name: &str) -> Result<()> {
        let _guard = self.lock.lock().unwrap_or_else(|p| p.into_inner());
        let mut map = self.read_file()?;
        if map.remove(name).is_some() {
            self.write_file(&map)?;
        }
        Ok(())
    }

    fn read_file(&self) -> Result<BTreeMap<String, String>> {
        match std::fs::read_to_string(&self.file) {
            Ok(text) => Ok(serde_json::from_str(&text).unwrap_or_default()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
            Err(e) => Err(crate::error::HiveError::io(&self.file, e)),
        }
    }

    fn write_file(&self, map: &BTreeMap<String, String>) -> Result<()> {
        if let Some(parent) = self.file.parent() {
            std::fs::create_dir_all(parent).at(parent)?;
        }
        crate::util::write_json_atomic(&self.file, map)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&self.file, std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }
}

#[cfg(feature = "keyring")]
fn keyring_available() -> bool {
    if std::env::var_os("HIVE_NO_KEYRING").is_some() {
        return false;
    }
    keyring::Entry::store_status().is_ok()
}

#[cfg(feature = "keyring")]
fn keyring_get(name: &str) -> Option<String> {
    keyring::Entry::new(SERVICE, name).ok()?.get_password().ok()
}

#[cfg(feature = "keyring")]
fn keyring_set(name: &str, value: &str) -> bool {
    keyring::Entry::new(SERVICE, name)
        .and_then(|e| e.set_password(value))
        .is_ok()
}

#[cfg(feature = "keyring")]
fn keyring_delete(name: &str) {
    if let Ok(entry) = keyring::Entry::new(SERVICE, name) {
        let _ = entry.delete_credential();
    }
}

#[cfg(not(feature = "keyring"))]
fn keyring_available() -> bool {
    false
}
#[cfg(not(feature = "keyring"))]
fn keyring_get(_name: &str) -> Option<String> {
    None
}
#[cfg(not(feature = "keyring"))]
fn keyring_set(_name: &str, _value: &str) -> bool {
    false
}
#[cfg(not(feature = "keyring"))]
fn keyring_delete(_name: &str) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_backend_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = HivePaths::new(tmp.path());
        let store = SecretStore::file_only(&paths);
        assert!(store.get("k").unwrap().is_none());
        store.set("k", "v").unwrap();
        assert_eq!(store.get("k").unwrap().as_deref(), Some("v"));
        assert!(store.has("k"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(paths.secrets_file())
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600);
        }
        store.set("k", "").unwrap();
        assert!(!store.has("k"));
        store.set("a", "1").unwrap();
        store.delete("a").unwrap();
        assert!(store.get("a").unwrap().is_none());
    }
}
