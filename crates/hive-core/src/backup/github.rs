// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

//! GitHub device flow and PAT checks. The real account sign-in is manual; these
//! functions talk to any HTTP client so tests can use a recorded fixture.

use anyhow::{anyhow, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceCodes {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevicePoll {
    Pending,
    Approved { access_token: String },
    Denied,
}

pub trait GitHubHttp {
    fn post_form(&self, url: &str, body: &str) -> Result<(u16, String)>;
    fn get(&self, url: &str, bearer: Option<&str>) -> Result<(u16, String)>;
}

pub fn begin_device_flow(http: &dyn GitHubHttp, client_id: &str) -> Result<DeviceCodes> {
    let body = format!("client_id={client_id}&scope=repo");
    let (status, raw) = http.post_form("https://github.com/login/device/code", &body)?;
    if status != 200 {
        return Err(anyhow!("device flow start failed: {status}"));
    }
    Ok(DeviceCodes {
        device_code: form_value(&raw, "device_code")?,
        user_code: form_value(&raw, "user_code")?,
        verification_uri: form_value(&raw, "verification_uri")?,
        interval: form_value(&raw, "interval")?.parse().unwrap_or(5),
    })
}

pub fn poll_device_flow(
    http: &dyn GitHubHttp,
    client_id: &str,
    device_code: &str,
) -> Result<DevicePoll> {
    let body = format!("client_id={client_id}&device_code={device_code}&grant_type=urn:ietf:params:oauth:grant-type:device_code");
    let (status, raw) = http.post_form("https://github.com/login/oauth/access_token", &body)?;
    if status != 200 {
        return Err(anyhow!("device poll failed: {status}"));
    }
    if raw.contains("authorization_pending") {
        return Ok(DevicePoll::Pending);
    }
    if raw.contains("access_denied") {
        return Ok(DevicePoll::Denied);
    }
    Ok(DevicePoll::Approved {
        access_token: form_value(&raw, "access_token")?,
    })
}

pub fn validate_pat(http: &dyn GitHubHttp, pat: &str) -> Result<String> {
    let (status, raw) = http.get("https://api.github.com/user", Some(pat))?;
    if status != 200 {
        return Err(anyhow!("PAT was rejected"));
    }
    let login = json_string(&raw, "login").ok_or_else(|| anyhow!("PAT response has no login"))?;
    Ok(login)
}

pub fn find_or_create_private_repo(http: &dyn GitHubHttp, pat: &str, name: &str) -> Result<String> {
    let (status, raw) = http.get(
        &format!("https://api.github.com/repos/me/{name}"),
        Some(pat),
    )?;
    if status == 200 {
        return json_string(&raw, "ssh_url").ok_or_else(|| anyhow!("repo has no ssh_url"));
    }
    let body = format!("{{\"name\":\"{name}\",\"private\":true}}");
    let (created, raw) = http.post_form("https://api.github.com/user/repos", &body)?;
    if created != 201 && created != 200 {
        return Err(anyhow!("could not create private repo"));
    }
    json_string(&raw, "ssh_url").ok_or_else(|| anyhow!("created repo has no ssh_url"))
}

pub fn public_repo_warning(private: bool) -> Option<&'static str> {
    if private {
        None
    } else {
        Some("This remote is public. Skill contents will be visible to anyone.")
    }
}

fn form_value(body: &str, key: &str) -> Result<String> {
    for pair in body.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k.trim() == key {
                return Ok(v.trim().to_string());
            }
        }
    }
    if let Some(value) = json_string(body, key) {
        return Ok(value);
    }
    Err(anyhow!("missing {key}"))
}

fn json_string(body: &str, key: &str) -> Option<String> {
    let marker = format!("\"{key}\"");
    let index = body.find(&marker)?;
    let after = &body[index + marker.len()..];
    let colon = after.find(':')?;
    let rest = after[colon + 1..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    struct Scripted {
        calls: Cell<usize>,
    }

    impl GitHubHttp for Scripted {
        fn post_form(&self, url: &str, _body: &str) -> Result<(u16, String)> {
            if url.contains("device/code") {
                return Ok((
                    200,
                    "device_code=dc&user_code=ABCD-1234&verification_uri=https://github.com/login/device&interval=5".into(),
                ));
            }
            if url.contains("access_token") {
                let n = self.calls.get();
                self.calls.set(n + 1);
                if n == 0 {
                    return Ok((200, "error=authorization_pending".into()));
                }
                return Ok((200, "access_token=gho_test&token_type=bearer".into()));
            }
            if url.contains("/user/repos") {
                return Ok((
                    201,
                    r#"{"ssh_url":"git@github.com:octocat/skill-hive.git"}"#.into(),
                ));
            }
            Err(anyhow!("unexpected post {url}"))
        }

        fn get(&self, url: &str, _bearer: Option<&str>) -> Result<(u16, String)> {
            if url.ends_with("/user") {
                return Ok((200, r#"{"login":"octocat"}"#.into()));
            }
            if url.contains("/repos/") {
                return Ok((404, "{}".into()));
            }
            Err(anyhow!("unexpected get {url}"))
        }
    }

    #[test]
    fn device_flow_and_pat_against_a_fixture() {
        let http = Scripted {
            calls: Cell::new(0),
        };
        let codes = begin_device_flow(&http, "client").unwrap();
        assert_eq!(codes.user_code, "ABCD-1234");
        assert_eq!(
            poll_device_flow(&http, "client", "dc").unwrap(),
            DevicePoll::Pending
        );
        match poll_device_flow(&http, "client", "dc").unwrap() {
            DevicePoll::Approved { access_token } => assert_eq!(access_token, "gho_test"),
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(validate_pat(&http, "pat").unwrap(), "octocat");
        let url = find_or_create_private_repo(&http, "pat", "skill-hive").unwrap();
        assert!(url.contains("skill-hive.git"));
        assert!(public_repo_warning(false).is_some());
        assert!(public_repo_warning(true).is_none());
    }
}
