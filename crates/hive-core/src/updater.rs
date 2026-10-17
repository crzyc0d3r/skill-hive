//! Updater decisions shared by the desktop shell.
//!
//! Signature checks use `minisign-verify`, the same verifier Tauri's updater
//! plugin uses for the signature stored in `latest.json`.

use std::time::Duration;

use minisign_verify::{PublicKey, Signature};
use serde_json::Value;

/// Compare the installed version with a release tag.
/// `offline:<version>` means no tag was available to compare.
pub fn classify_release(local: &str, remote_tag: Option<&str>) -> String {
    let local = local.trim().trim_start_matches('v');
    match remote_tag.map(str::trim).filter(|tag| !tag.is_empty()) {
        None => format!("offline:{local}"),
        Some(tag) => {
            let remote = tag.trim_start_matches('v');
            if remote == local {
                "current".into()
            } else {
                format!("available:{remote}")
            }
        }
    }
}

/// Read `tag_name` from a GitHub-style latest-release document.
pub fn fetch_latest_release_tag(url: &str) -> Result<String, String> {
    let response = ureq::get(url)
        .timeout(Duration::from_secs(2))
        .set("user-agent", "skill-hive")
        .call()
        .map_err(|err| err.to_string())?;
    let body = response.into_string().map_err(|err| err.to_string())?;
    let value: Value = serde_json::from_str(&body).map_err(|err| err.to_string())?;
    value
        .get("tag_name")
        .and_then(|tag| tag.as_str())
        .map(str::to_string)
        .ok_or_else(|| "release has no tag_name".into())
}

pub fn update_install_blocker(active_runs: u32) -> Option<&'static str> {
    if active_runs > 0 {
        Some("A test run is still active.")
    } else {
        None
    }
}

pub fn updater_proxy(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        Some(trimmed.to_string())
    } else {
        None
    }
}

pub fn latest_platform_signature(latest_json: &str, platform: &str) -> Result<String, String> {
    let value: Value = serde_json::from_str(latest_json).map_err(|err| err.to_string())?;
    value
        .get("platforms")
        .and_then(|platforms| platforms.get(platform))
        .and_then(|entry| entry.get("signature"))
        .and_then(|signature| signature.as_str())
        .map(str::to_string)
        .ok_or_else(|| format!("latest.json has no signature for {platform}"))
}

pub fn verify_updater_signature(
    public_b64: &str,
    signature: &str,
    artifact: &[u8],
) -> Result<(), String> {
    let key = PublicKey::from_base64(public_b64).map_err(|err| err.to_string())?;
    let parsed = Signature::decode(signature).map_err(|err| err.to_string())?;
    key.verify(artifact, &parsed, false)
        .map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        classify_release, latest_platform_signature, update_install_blocker, updater_proxy,
        verify_updater_signature,
    };
    use minisign::{sign, KeyPair};
    use std::io::Cursor;

    #[test]
    fn an_active_run_blocks_install_and_a_proxy_must_be_http() {
        assert_eq!(
            update_install_blocker(1),
            Some("A test run is still active.")
        );
        assert_eq!(update_install_blocker(0), None);
        assert_eq!(updater_proxy(""), None);
        assert_eq!(updater_proxy("ftp://files"), None);
        assert_eq!(
            updater_proxy(" http://127.0.0.1:8888 ").as_deref(),
            Some("http://127.0.0.1:8888")
        );
    }

    #[test]
    fn a_tampered_latest_json_signature_is_rejected() {
        let KeyPair { pk, sk } = KeyPair::generate_unencrypted_keypair().expect("keypair");
        let artifact = b"skill-hive-updater-artifact";
        let signed = sign(Some(&pk), &sk, Cursor::new(&artifact[..]), None, None)
            .expect("sign")
            .into_string();
        let public_key = pk.to_base64();
        verify_updater_signature(&public_key, &signed, artifact).expect("fresh signature");

        let latest = serde_json::json!({
            "version": "1.0.0",
            "platforms": {
                "linux-x86_64": {
                    "signature": signed,
                    "url": "https://github.com/crzyc0d3r/skill-hive/releases/download/v1.0.0/app.AppImage"
                }
            }
        })
        .to_string();
        let extracted = latest_platform_signature(&latest, "linux-x86_64").expect("field");
        verify_updater_signature(&public_key, &extracted, artifact)
            .expect("latest.json signature matches the artifact");

        let mut document: serde_json::Value = serde_json::from_str(&latest).expect("json");
        let current = document["platforms"]["linux-x86_64"]["signature"]
            .as_str()
            .expect("signature")
            .to_string();
        let mut lines: Vec<String> = current.lines().map(str::to_string).collect();
        let body = lines
            .iter()
            .position(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && !trimmed.to_ascii_lowercase().contains("comment")
            })
            .expect("minisign signature body");
        let mut chars: Vec<char> = lines[body].chars().collect();
        let index = chars
            .iter()
            .position(|ch| ch.is_ascii_alphanumeric())
            .expect("signature body");
        chars[index] = if chars[index] == 'A' { 'B' } else { 'A' };
        lines[body] = chars.into_iter().collect();
        document["platforms"]["linux-x86_64"]["signature"] =
            serde_json::Value::String(lines.join("\n"));
        let tampered = document.to_string();
        let tampered_signature =
            latest_platform_signature(&tampered, "linux-x86_64").expect("tampered field");
        assert_ne!(tampered_signature, extracted);
        assert!(
            verify_updater_signature(&public_key, &tampered_signature, artifact).is_err(),
            "a tampered latest.json signature is rejected"
        );
        assert!(verify_updater_signature(&public_key, &extracted, b"other-bytes").is_err());
    }

    #[test]
    fn a_release_tag_is_current_only_when_it_matches_the_installed_version() {
        assert_eq!(classify_release("1.2.0", None), "offline:1.2.0");
        assert_eq!(classify_release("1.2.0", Some("v1.2.0")), "current");
        assert_eq!(classify_release("1.2.0", Some("v1.3.0")), "available:1.3.0");
        assert_eq!(classify_release("1.2.0", Some("  ")), "offline:1.2.0");
    }
}
