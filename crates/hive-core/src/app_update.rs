//! Updates of Skill Hive itself.
//!
//! Every release on GitHub carries the desktop installers, a `latest.json` for the Tauri
//! updater and one `hive` archive per supported target. The desktop app updates through the
//! Tauri updater in the shell (signed bundles; see `docs/releasing.md`). This module serves the
//! CLI: find the latest release, decide whether it is newer than this build, and fetch the
//! `hive` binary built for this machine so `hive self-update` can swap it in.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{HiveError, Result};
use crate::hive::Hive;
use crate::sources::archive;
use crate::sources::github::{GithubClient, ReleaseAsset, ReleaseInfo};

pub const REPO_OWNER: &str = "crzyc0d3r";
pub const REPO_NAME: &str = "skill-hive";
/// Where the releases are published.
pub const RELEASES_URL: &str = "https://github.com/crzyc0d3r/skill-hive/releases";
/// A `hive` archive is a few tens of megabytes; anything near this is not one of ours.
const MAX_CLI_ARCHIVE_BYTES: u64 = 100 * 1024 * 1024;

/// The target triple the release workflow builds the CLI for on this OS and architecture.
pub fn cli_target(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        ("linux", "x86_64") => Some("x86_64-unknown-linux-gnu"),
        ("macos", "aarch64") => Some("aarch64-apple-darwin"),
        ("macos", "x86_64") => Some("x86_64-apple-darwin"),
        ("windows", "x86_64") => Some("x86_64-pc-windows-msvc"),
        _ => None,
    }
}

/// The release asset that holds the CLI for this OS and architecture, named as the release
/// workflow names it: `hive-<target>.tar.gz`, or `.zip` on Windows.
pub fn cli_asset_name(os: &str, arch: &str) -> Option<String> {
    let target = cli_target(os, arch)?;
    Some(if os == "windows" {
        format!("hive-{target}.zip")
    } else {
        format!("hive-{target}.tar.gz")
    })
}

/// The file name of the CLI executable inside the archive.
pub fn cli_binary_name(os: &str) -> &'static str {
    if os == "windows" { "hive.exe" } else { "hive" }
}

/// Parse a version, with or without a leading `v`.
pub fn parse_version(text: &str) -> Result<semver::Version> {
    let text = text.trim().trim_start_matches('v');
    semver::Version::parse(text)
        .map_err(|e| HiveError::invalid(format!("not a version: `{text}` ({e})")))
}

/// Whether `latest` is a newer release than `current`.
pub fn is_newer(current: &str, latest: &str) -> Result<bool> {
    Ok(parse_version(latest)? > parse_version(current)?)
}

/// What `hive self-update` found out about the latest release.
#[derive(Debug, Clone, Serialize)]
pub struct CliUpdateCheck {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub release_url: String,
    pub published_at: Option<String>,
    /// Release notes, Markdown.
    pub notes: Option<String>,
    /// The CLI archive built for this machine, when the release has one.
    pub asset: Option<ReleaseAsset>,
}

/// A `hive` executable extracted from a release archive. Its directory is removed with the
/// value, so copy the binary out before dropping it.
pub struct DownloadedCli {
    _dir: tempfile::TempDir,
    pub binary: PathBuf,
    pub archive_bytes: u64,
}

/// Compare a release with the running version and pick the CLI archive for `os`/`arch`.
pub fn describe_release(
    release: &ReleaseInfo,
    current_version: &str,
    os: &str,
    arch: &str,
) -> Result<CliUpdateCheck> {
    let latest_version = release.tag_name.trim().trim_start_matches('v').to_string();
    let update_available = is_newer(current_version, &latest_version)?;
    let wanted = cli_asset_name(os, arch);
    let asset = wanted
        .as_deref()
        .and_then(|name| release.assets.iter().find(|a| a.name == name).cloned());
    Ok(CliUpdateCheck {
        current_version: current_version.to_string(),
        latest_version,
        update_available,
        release_url: if release.html_url.is_empty() {
            RELEASES_URL.to_string()
        } else {
            release.html_url.clone()
        },
        published_at: release.published_at.clone(),
        notes: release.body.clone(),
        asset,
    })
}

/// Locate the CLI executable in an extracted archive: at the root, or one directory down when
/// the archive wraps its content in a folder.
pub fn find_cli_binary(dir: &Path, file_name: &str) -> Result<PathBuf> {
    let direct = dir.join(file_name);
    if direct.is_file() {
        return Ok(direct);
    }
    let nested = archive::single_root(dir).join(file_name);
    if nested.is_file() {
        return Ok(nested);
    }
    Err(HiveError::not_found(format!(
        "`{file_name}` in the downloaded archive"
    )))
}

impl Hive {
    fn github(&self) -> GithubClient {
        GithubClient::new(self.http.clone(), self.github_token())
    }

    /// The latest release on GitHub and whether it is newer than this build.
    pub async fn check_cli_update(&self) -> Result<CliUpdateCheck> {
        let release = self.github().latest_release(REPO_OWNER, REPO_NAME).await?;
        describe_release(
            &release,
            crate::version(),
            std::env::consts::OS,
            std::env::consts::ARCH,
        )
    }

    /// Download the CLI archive `check` names and extract the `hive` executable from it.
    pub async fn download_cli_update(&self, check: &CliUpdateCheck) -> Result<DownloadedCli> {
        let os = std::env::consts::OS;
        let asset = check.asset.as_ref().ok_or_else(|| {
            HiveError::not_found(format!(
                "a CLI build for {os}-{} in release {}",
                std::env::consts::ARCH,
                check.latest_version
            ))
        })?;
        let bytes = self
            .github()
            .download_asset(&asset.browser_download_url, MAX_CLI_ARCHIVE_BYTES)
            .await?;
        let archive_bytes = bytes.len() as u64;
        let cache = self.paths.cache_dir();
        std::fs::create_dir_all(&cache).map_err(|e| HiveError::io(&cache, e))?;
        let dir = tempfile::Builder::new()
            .prefix("self-update-")
            .tempdir_in(&cache)
            .map_err(|e| HiveError::io(&cache, e))?;
        let name = asset.name.clone();
        let dest = dir.path().to_path_buf();
        tokio::task::spawn_blocking(move || archive::extract(&bytes, &name, &dest))
            .await
            .map_err(|e| HiveError::msg(format!("extraction failed: {e}")))??;
        let binary = find_cli_binary(dir.path(), cli_binary_name(os))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| HiveError::io(&binary, e))?;
        }
        Ok(DownloadedCli {
            _dir: dir,
            binary,
            archive_bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn targets_and_asset_names() {
        assert_eq!(
            cli_asset_name("linux", "x86_64").as_deref(),
            Some("hive-x86_64-unknown-linux-gnu.tar.gz")
        );
        assert_eq!(
            cli_asset_name("macos", "aarch64").as_deref(),
            Some("hive-aarch64-apple-darwin.tar.gz")
        );
        assert_eq!(
            cli_asset_name("macos", "x86_64").as_deref(),
            Some("hive-x86_64-apple-darwin.tar.gz")
        );
        assert_eq!(
            cli_asset_name("windows", "x86_64").as_deref(),
            Some("hive-x86_64-pc-windows-msvc.zip")
        );
        assert_eq!(cli_asset_name("linux", "aarch64"), None);
        assert_eq!(cli_asset_name("freebsd", "x86_64"), None);
        assert_eq!(cli_binary_name("windows"), "hive.exe");
        assert_eq!(cli_binary_name("linux"), "hive");
    }

    #[test]
    fn version_comparison() {
        assert!(is_newer("0.1.3", "v0.2.0").unwrap());
        assert!(is_newer("0.1.3", "0.1.4").unwrap());
        assert!(is_newer("0.9.9", "1.0.0").unwrap());
        assert!(!is_newer("0.1.3", "0.1.3").unwrap());
        assert!(!is_newer("0.2.0", "v0.1.9").unwrap());
        // A pre-release of the same version is older, not newer.
        assert!(!is_newer("0.2.0", "0.2.0-rc.1").unwrap());
        assert!(parse_version("latest").is_err());
    }

    fn release(tag: &str) -> ReleaseInfo {
        serde_json::from_value(serde_json::json!({
            "tag_name": tag,
            "name": format!("Skill Hive {tag}"),
            "html_url": format!("https://github.com/crzyc0d3r/skill-hive/releases/tag/{tag}"),
            "published_at": "2026-11-06T01:20:51Z",
            "body": "## What's Changed\n* Merge overlapping skills",
            "assets": [
                {"name": "latest.json", "browser_download_url": "https://example.invalid/latest.json", "size": 1200},
                {"name": "hive-x86_64-unknown-linux-gnu.tar.gz", "browser_download_url": "https://example.invalid/linux.tar.gz", "size": 9_000_000},
                {"name": "hive-x86_64-pc-windows-msvc.zip", "browser_download_url": "https://example.invalid/win.zip", "size": 8_000_000}
            ]
        }))
        .unwrap()
    }

    #[test]
    fn describes_a_release() {
        let check = describe_release(&release("v0.2.0"), "0.1.3", "linux", "x86_64").unwrap();
        assert!(check.update_available);
        assert_eq!(check.latest_version, "0.2.0");
        assert_eq!(check.current_version, "0.1.3");
        assert_eq!(
            check.asset.as_ref().map(|a| a.name.as_str()),
            Some("hive-x86_64-unknown-linux-gnu.tar.gz")
        );
        assert_eq!(check.asset.as_ref().map(|a| a.size), Some(9_000_000));
        assert!(check.release_url.ends_with("/tag/v0.2.0"));
        assert!(
            check
                .notes
                .as_deref()
                .unwrap()
                .contains("Merge overlapping")
        );

        // Windows picks the zip; an architecture without a build gets no asset.
        let win = describe_release(&release("v0.2.0"), "0.1.3", "windows", "x86_64").unwrap();
        assert_eq!(
            win.asset.as_ref().map(|a| a.name.as_str()),
            Some("hive-x86_64-pc-windows-msvc.zip")
        );
        let arm = describe_release(&release("v0.2.0"), "0.1.3", "linux", "aarch64").unwrap();
        assert!(arm.update_available);
        assert!(arm.asset.is_none());

        // The same or an older release is not an update.
        assert!(
            !describe_release(&release("v0.1.3"), "0.1.3", "linux", "x86_64")
                .unwrap()
                .update_available
        );
        assert!(
            !describe_release(&release("v0.1.0"), "0.1.3", "linux", "x86_64")
                .unwrap()
                .update_available
        );
        assert!(describe_release(&release("nightly"), "0.1.3", "linux", "x86_64").is_err());
    }

    fn tar_gz(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        for (name, data) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder.append_data(&mut header, name, *data).unwrap();
        }
        let tar = builder.into_inner().unwrap();
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        gz.write_all(&tar).unwrap();
        gz.finish().unwrap()
    }

    fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, data) in entries {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn finds_the_binary_in_release_archives() {
        // The workflow's tar.gz: the binary at the root.
        let tmp = tempfile::tempdir().unwrap();
        let flat = tmp.path().join("flat");
        archive::extract(
            &tar_gz(&[("hive", b"#!/bin/sh\necho hive\n")]),
            "hive-x.tar.gz",
            &flat,
        )
        .unwrap();
        assert_eq!(find_cli_binary(&flat, "hive").unwrap(), flat.join("hive"));

        // An archive that wraps everything in one folder.
        let nested = tmp.path().join("nested");
        archive::extract(
            &tar_gz(&[
                ("hive-0.2.0/hive", b"bin"),
                ("hive-0.2.0/README.md", b"doc"),
            ]),
            "hive-x.tar.gz",
            &nested,
        )
        .unwrap();
        assert_eq!(
            find_cli_binary(&nested, "hive").unwrap(),
            nested.join("hive-0.2.0").join("hive")
        );

        // The Windows zip.
        let win = tmp.path().join("win");
        archive::extract(&zip(&[("hive.exe", b"MZ")]), "hive-x.zip", &win).unwrap();
        assert_eq!(
            find_cli_binary(&win, "hive.exe").unwrap(),
            win.join("hive.exe")
        );
        assert!(find_cli_binary(&win, "hive").is_err());
    }
}
