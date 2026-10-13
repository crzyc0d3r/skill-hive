// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

//! Log export and the About-screen warning vocabulary.

use std::fs::File;
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::util::log_sanitize::sanitize;

pub fn export_logs_zip(log_text: &str, dest: &Path) -> Result<String> {
    let clean = sanitize(log_text);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(dest)?;
    let mut zip = ZipWriter::new(file);
    zip.start_file("logs/app.log", SimpleFileOptions::default())?;
    zip.write_all(clean.as_bytes())?;
    zip.finish()?;
    Ok(clean)
}

pub fn read_exported_log(archive: &Path) -> Result<String> {
    let file = File::open(archive)?;
    let mut zip = ZipArchive::new(file)?;
    let mut entry = zip.by_name("logs/app.log")?;
    let mut text = String::new();
    std::io::Read::read_to_string(&mut entry, &mut text)?;
    Ok(text)
}

pub fn validate_proxy_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Ok(());
    }
    if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("socks5://") {
        Ok(())
    } else {
        Err("proxy URL must start with http://, https://, or socks5://".into())
    }
}

pub fn about_banners(warnings: &[String]) -> Vec<&'static str> {
    let mut banners = Vec::new();
    for warning in warnings {
        match warning.as_str() {
            "config_unreadable" => banners.push("The settings file could not be read."),
            "repo_path_invalid" => banners.push("The central repository path is invalid."),
            "migration_incomplete" => banners.push("A library move did not finish."),
            _ => {}
        }
    }
    banners
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crafted_log_exports_sanitised() {
        let tmp = tempfile::tempdir().unwrap();
        let raw =
            "token ghp_ABCDEFGHIJKLMNOPQRST user alice@example.com opened /home/somebody/notes.txt";
        let archive = tmp.path().join("logs.zip");
        let clean = export_logs_zip(raw, &archive).unwrap();
        let exported = read_exported_log(&archive).unwrap();
        assert_eq!(exported, clean);
        assert!(!exported.contains("ghp_"));
        assert!(!exported.contains("alice@example.com"));
        assert!(!exported.contains("/home/somebody"));
        assert!(exported.contains("<token>"));
        assert!(exported.contains("<email>"));
        assert!(exported.contains("~"));
    }

    #[test]
    fn proxy_and_about_banners() {
        assert!(validate_proxy_url("http://127.0.0.1:8888").is_ok());
        assert!(validate_proxy_url("ftp://nope").is_err());
        let banners = about_banners(&[
            "config_unreadable".into(),
            "repo_path_invalid".into(),
            "migration_incomplete".into(),
        ]);
        assert_eq!(banners.len(), 3);
    }
}
