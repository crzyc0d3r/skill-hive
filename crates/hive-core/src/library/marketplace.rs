// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

//! skills.sh marketplace client: search JSON, leaderboard HTML, a short cache,
//! and zip extraction that refuses traversal, links, and oversized archives.

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

use crate::library::installer::safe_extract;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketSkill {
    pub owner: String,
    pub name: String,
    pub installs: u64,
    pub source: String,
}

#[derive(Debug, Deserialize)]
struct SearchBody {
    skills: Vec<SearchSkill>,
}

#[derive(Debug, Deserialize)]
struct SearchSkill {
    name: String,
    owner: String,
    #[serde(default)]
    installs: u64,
    #[serde(default)]
    source: String,
}

pub fn parse_search(body: &str) -> Result<Vec<MarketSkill>> {
    let parsed: SearchBody = serde_json::from_str(body).context("search response")?;
    Ok(parsed
        .skills
        .into_iter()
        .map(|skill| MarketSkill {
            source: if skill.source.is_empty() {
                format!("github.com/{}/{}", skill.owner, skill.name)
            } else {
                skill.source
            },
            owner: skill.owner,
            name: skill.name,
            installs: skill.installs,
        })
        .collect())
}

pub fn parse_leaderboard(html: &str) -> Result<Vec<MarketSkill>> {
    if let Some(json) = next_data(html) {
        if let Ok(skills) = parse_next_data(json) {
            if !skills.is_empty() {
                return Ok(skills);
            }
        }
    }
    parse_leaderboard_fallback(html)
}

fn next_data(html: &str) -> Option<&str> {
    let marker = "<script id=\"__NEXT_DATA__\"";
    let start = html.find(marker)?;
    let after = &html[start..];
    let json_start = after.find('>')? + 1;
    let json_end = after[json_start..].find("</script>")?;
    Some(&after[json_start..json_start + json_end])
}

fn parse_next_data(json: &str) -> Result<Vec<MarketSkill>> {
    let value: serde_json::Value = serde_json::from_str(json)?;
    let skills = value
        .pointer("/props/pageProps/skills")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("__NEXT_DATA__ has no skills"))?;
    let mut out = Vec::new();
    for skill in skills {
        let name = skill
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let owner = skill
            .get("owner")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if name.is_empty() || owner.is_empty() {
            continue;
        }
        out.push(MarketSkill {
            installs: skill.get("installs").and_then(|v| v.as_u64()).unwrap_or(0),
            source: skill
                .get("source")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            owner,
            name,
        });
    }
    Ok(out)
}

fn parse_leaderboard_fallback(html: &str) -> Result<Vec<MarketSkill>> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(index) = rest.find("data-skill=\"") {
        let after = &rest[index + "data-skill=\"".len()..];
        let Some(end) = after.find('"') else { break };
        let slug = &after[..end];
        if let Some((owner, name)) = slug.split_once('/') {
            out.push(MarketSkill {
                owner: owner.to_string(),
                name: name.to_string(),
                installs: 0,
                source: format!("github.com/{slug}"),
            });
        }
        rest = &after[end..];
    }
    if out.is_empty() {
        return Err(anyhow!("leaderboard fallback found no skills"));
    }
    Ok(out)
}

pub struct SearchCache {
    entries: HashMap<String, (Instant, Vec<MarketSkill>)>,
    ttl: Duration,
}

impl SearchCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            ttl,
        }
    }

    pub fn get(&mut self, query: &str) -> Option<Vec<MarketSkill>> {
        let expired = self
            .entries
            .get(query)
            .is_some_and(|(at, _)| at.elapsed() > self.ttl);
        if expired {
            self.entries.remove(query);
            return None;
        }
        self.entries.get(query).map(|(_, skills)| skills.clone())
    }

    pub fn insert(&mut self, query: &str, skills: Vec<MarketSkill>) {
        self.entries
            .insert(query.to_string(), (Instant::now(), skills));
    }
}

pub fn marketplace_tab_visible(api_reachable: bool) -> bool {
    api_reachable
}

pub fn install_zip_bytes(bytes: &[u8], dest: &Path, cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::SeqCst) {
        return Err(anyhow!("cancelled"));
    }
    let tmp = dest.with_extension("partial.zip");
    if let Some(parent) = tmp.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&tmp, bytes)?;
    if cancel.load(Ordering::SeqCst) {
        let _ = fs::remove_file(&tmp);
        return Err(anyhow!("cancelled"));
    }
    let result = safe_extract(&tmp, dest);
    let _ = fs::remove_file(&tmp);
    result
}

pub fn http_get(url: &str) -> Result<String> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| anyhow!("only http fixtures are supported"))?;
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let path = format!("/{path}");
    let mut stream = TcpStream::connect(host).with_context(|| format!("connect {host}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
    )?;
    let mut raw = String::new();
    stream.read_to_string(&mut raw)?;
    let body = raw
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or(raw.as_str());
    Ok(body.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::installer::{MAX_ZIP_ENTRIES, MAX_ZIP_UNCOMPRESSED_BYTES};
    use std::io::{Read, Write};
    use std::net::{Shutdown, TcpListener};
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    const SEARCH_FIXTURE: &str = r#"{"skills":[{"name":"review","owner":"acme","installs":12,"source":"github.com/acme/review"},{"name":"notes","owner":"beta","installs":3}]}"#;

    #[test]
    fn parses_search_fixture_and_leaderboard_next_data() {
        let skills = parse_search(SEARCH_FIXTURE).unwrap();
        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].owner, "acme");
        assert_eq!(skills[1].source, "github.com/beta/notes");

        let html = format!(
            "<html><script id=\"__NEXT_DATA__\" type=\"application/json\">{{\"props\":{{\"pageProps\":{{\"skills\":[{SEARCH}]}}}}}}</script></html>",
            SEARCH = r#"{"name":"review","owner":"acme","installs":4,"source":"github.com/acme/review"}"#
        );
        let board = parse_leaderboard(&html).unwrap();
        assert_eq!(board[0].name, "review");
    }

    #[test]
    fn leaderboard_falls_back_when_next_data_is_absent() {
        let html = r#"<a data-skill="acme/review">Review</a><a data-skill="beta/notes">Notes</a>"#;
        let board = parse_leaderboard(html).unwrap();
        assert_eq!(board.len(), 2);
        assert_eq!(board[1].name, "notes");
    }

    #[test]
    fn unreachable_api_hides_the_tab() {
        assert!(!marketplace_tab_visible(false));
        assert!(marketplace_tab_visible(true));
    }

    #[test]
    fn cache_expires() {
        let mut cache = SearchCache::new(Duration::from_millis(10));
        cache.insert("review", parse_search(SEARCH_FIXTURE).unwrap());
        assert!(cache.get("review").is_some());
        std::thread::sleep(Duration::from_millis(20));
        assert!(cache.get("review").is_none());
    }

    #[test]
    fn recorded_http_fixture_serves_search() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let body = SEARCH_FIXTURE.to_string();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let mut buf = Vec::new();
            let mut chunk = [0u8; 256];
            while !buf.windows(4).any(|window| window == b"\r\n\r\n") {
                match stream.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => buf.extend_from_slice(&chunk[..n]),
                }
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
            stream.flush().unwrap();
            let _ = stream.shutdown(Shutdown::Write);
            let _ = stream.read(&mut chunk);
        });
        let raw = http_get(&format!("http://127.0.0.1:{port}/api/search?q=review")).unwrap();
        server.join().unwrap();
        let skills = parse_search(raw.trim()).unwrap();
        assert_eq!(skills[0].name, "review");
    }

    #[test]
    fn cancelled_install_writes_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let cancel = AtomicBool::new(true);
        let err = install_zip_bytes(b"not a zip", &tmp.path().join("out"), &cancel).unwrap_err();
        assert!(err.to_string().contains("cancelled"));
        assert!(!tmp.path().join("out").exists());
    }

    /// A stored zip whose central-directory host system is Unix and whose
    /// external attributes mark the entry as a symlink (`S_IFLNK`).
    fn unix_symlink_zip(name: &[u8], target: &[u8]) -> Vec<u8> {
        fn le16(n: u16) -> [u8; 2] {
            n.to_le_bytes()
        }
        fn le32(n: u32) -> [u8; 4] {
            n.to_le_bytes()
        }
        let mut crc = 0xffffffffu32;
        for &byte in target {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xedb8_8320 & mask);
            }
        }
        let crc = !crc;
        let mut local = Vec::new();
        local.extend_from_slice(&le32(0x0403_4b50));
        local.extend_from_slice(&le16(20));
        local.extend_from_slice(&le16(0));
        local.extend_from_slice(&le16(0));
        local.extend_from_slice(&le16(0));
        local.extend_from_slice(&le16(0));
        local.extend_from_slice(&le32(crc));
        local.extend_from_slice(&le32(target.len() as u32));
        local.extend_from_slice(&le32(target.len() as u32));
        local.extend_from_slice(&le16(name.len() as u16));
        local.extend_from_slice(&le16(0));
        local.extend_from_slice(name);
        local.extend_from_slice(target);
        let mut central = Vec::new();
        central.extend_from_slice(&le32(0x0201_4b50));
        central.extend_from_slice(&le16(0x0314));
        central.extend_from_slice(&le16(20));
        central.extend_from_slice(&le16(0));
        central.extend_from_slice(&le16(0));
        central.extend_from_slice(&le16(0));
        central.extend_from_slice(&le16(0));
        central.extend_from_slice(&le32(crc));
        central.extend_from_slice(&le32(target.len() as u32));
        central.extend_from_slice(&le32(target.len() as u32));
        central.extend_from_slice(&le16(name.len() as u16));
        central.extend_from_slice(&le16(0));
        central.extend_from_slice(&le16(0));
        central.extend_from_slice(&le16(0));
        central.extend_from_slice(&le16(0));
        central.extend_from_slice(&le32(0o120_777_u32 << 16));
        central.extend_from_slice(&le32(0));
        central.extend_from_slice(name);
        let mut out = local.clone();
        let central_offset = out.len() as u32;
        out.extend_from_slice(&central);
        out.extend_from_slice(&le32(0x0605_4b50));
        out.extend_from_slice(&le16(0));
        out.extend_from_slice(&le16(0));
        out.extend_from_slice(&le16(1));
        out.extend_from_slice(&le16(1));
        out.extend_from_slice(&le32(central.len() as u32));
        out.extend_from_slice(&le32(central_offset));
        out.extend_from_slice(&le16(0));
        out
    }

    fn zip_with<F>(write: F) -> std::path::PathBuf
    where
        F: FnOnce(&mut ZipWriter<std::fs::File>),
    {
        let tmp = std::env::temp_dir().join(format!(
            "skill-hive-zip-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::create_dir_all(&tmp);
        let archive = tmp.join("skills.zip");
        let file = std::fs::File::create(&archive).unwrap();
        let mut zip = ZipWriter::new(file);
        write(&mut zip);
        zip.finish().unwrap();
        archive
    }

    #[test]
    fn zip_traversal_is_refused() {
        let archive = zip_with(|zip| {
            zip.start_file("../evil.txt", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"nope").unwrap();
        });
        let dest = archive.parent().unwrap().join("out");
        let err = safe_extract(&archive, &dest).unwrap_err();
        assert!(
            err.to_string().contains("unsafe") || err.to_string().contains("refusing"),
            "{err}"
        );
        let _ = fs::remove_dir_all(archive.parent().unwrap());
    }

    #[test]
    fn zip_symlink_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("link.zip");
        fs::write(&archive, unix_symlink_zip(b"link", b"target")).unwrap();
        let err = safe_extract(&archive, &tmp.path().join("out")).unwrap_err();
        assert!(err.to_string().contains("symlink"), "{err}");
    }

    #[test]
    fn zip_entry_count_over_4096_is_refused() {
        assert_eq!(MAX_ZIP_ENTRIES, 4096);
        let archive = zip_with(|zip| {
            for index in 0..=MAX_ZIP_ENTRIES {
                zip.start_file(format!("f{index}.txt"), SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(b"x").unwrap();
            }
        });
        let dest = archive.parent().unwrap().join("out");
        let err = safe_extract(&archive, &dest).unwrap_err();
        assert!(err.to_string().contains("4096"), "{err}");
        let _ = fs::remove_dir_all(archive.parent().unwrap());
    }

    #[test]
    fn zip_over_100_mib_is_refused() {
        assert_eq!(MAX_ZIP_UNCOMPRESSED_BYTES, 100 * 1024 * 1024);
        let archive = zip_with(|zip| {
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            zip.start_file("big.bin", options).unwrap();
            let chunk = vec![0u8; 1024 * 1024];
            for _ in 0..101 {
                zip.write_all(&chunk).unwrap();
            }
        });
        let dest = archive.parent().unwrap().join("out");
        let err = safe_extract(&archive, &dest).unwrap_err();
        assert!(err.to_string().contains("100 MiB"), "{err}");
        let _ = fs::remove_dir_all(archive.parent().unwrap());
    }
}
