//! GitHub REST API client: repository search, metadata and tarball download.
//!
//! Used for discovery and as a fallback when `git` is unavailable. A token raises rate limits
//! and unlocks private repositories; without one the API allows 60 requests per hour.

use serde::{Deserialize, Serialize};

use crate::error::{HiveError, Result};
use crate::model::RepoSearchResult;

pub const API_BASE: &str = "https://api.github.com";

#[derive(Debug, Clone)]
pub struct GithubClient {
    http: reqwest::Client,
    token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RepoInfo {
    pub full_name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub default_branch: String,
    #[serde(default)]
    pub stargazers_count: u32,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub pushed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    items: Vec<RepoInfo>,
}

/// One file attached to a release.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub size: u64,
}

/// A published release, as `GET /repos/{owner}/{repo}/releases/latest` returns it.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReleaseInfo {
    pub tag_name: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub published_at: Option<String>,
    /// Release notes, Markdown.
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub assets: Vec<ReleaseAsset>,
}

impl GithubClient {
    pub fn new(http: reqwest::Client, token: Option<String>) -> Self {
        Self { http, token }
    }

    fn request(&self, method: reqwest::Method, url: &str) -> reqwest::RequestBuilder {
        let mut req = self
            .http
            .request(method, url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        req
    }

    async fn check(resp: reqwest::Response) -> Result<reqwest::Response> {
        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }
        let remaining = resp
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let body = resp.text().await.unwrap_or_default();
        let message = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| {
                v.get("message")
                    .and_then(|m| m.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| body.chars().take(200).collect());
        if status.as_u16() == 403 && remaining.as_deref() == Some("0") {
            return Err(HiveError::msg(
                "GitHub API rate limit reached; add a GitHub token in Settings to raise it",
            ));
        }
        Err(HiveError::msg(format!("GitHub API {status}: {message}")))
    }

    pub async fn repo(&self, owner: &str, repo: &str) -> Result<RepoInfo> {
        let url = format!("{API_BASE}/repos/{owner}/{repo}");
        let resp = Self::check(self.request(reqwest::Method::GET, &url).send().await?).await?;
        Ok(resp.json::<RepoInfo>().await?)
    }

    /// The newest release that is neither a draft nor a pre-release.
    pub async fn latest_release(&self, owner: &str, repo: &str) -> Result<ReleaseInfo> {
        let url = format!("{API_BASE}/repos/{owner}/{repo}/releases/latest");
        let resp = Self::check(self.request(reqwest::Method::GET, &url).send().await?).await?;
        Ok(resp.json::<ReleaseInfo>().await?)
    }

    /// Download a release asset of at most `max_bytes`. Assets of public releases need no
    /// token, and the download redirects to another host, so the request carries none.
    pub async fn download_asset(&self, url: &str, max_bytes: u64) -> Result<Vec<u8>> {
        let resp = Self::check(self.http.get(url).send().await?).await?;
        if resp.content_length().is_some_and(|len| len > max_bytes) {
            return Err(HiveError::invalid("the release asset is too large"));
        }
        let bytes = resp.bytes().await?;
        if bytes.len() as u64 > max_bytes {
            return Err(HiveError::invalid("the release asset is too large"));
        }
        Ok(bytes.to_vec())
    }

    /// Download the repository tarball at `reference` (branch, tag or sha; `None` = default).
    pub async fn tarball(
        &self,
        owner: &str,
        repo: &str,
        reference: Option<&str>,
    ) -> Result<Vec<u8>> {
        let url = match reference {
            Some(r) => format!("{API_BASE}/repos/{owner}/{repo}/tarball/{r}"),
            None => format!("{API_BASE}/repos/{owner}/{repo}/tarball"),
        };
        let resp = Self::check(self.request(reqwest::Method::GET, &url).send().await?).await?;
        let bytes = resp.bytes().await?;
        if bytes.len() as u64 > super::archive::MAX_ARCHIVE_BYTES {
            return Err(HiveError::invalid("repository tarball is too large"));
        }
        Ok(bytes.to_vec())
    }

    /// Latest commit sha on a reference (used for update checks without git).
    pub async fn latest_commit(
        &self,
        owner: &str,
        repo: &str,
        reference: Option<&str>,
    ) -> Result<Option<String>> {
        let r = match reference {
            Some(r) => r.to_string(),
            None => self.repo(owner, repo).await?.default_branch,
        };
        let url = format!("{API_BASE}/repos/{owner}/{repo}/commits/{r}");
        let resp = Self::check(
            self.request(reqwest::Method::GET, &url)
                .header("Accept", "application/vnd.github.sha")
                .send()
                .await?,
        )
        .await?;
        let sha = resp.text().await?;
        Ok(Some(sha.trim().to_string()).filter(|s| !s.is_empty()))
    }

    /// Search repositories. The query is combined with topics people use for skill collections.
    pub async fn search(&self, query: &str, limit: u32) -> Result<Vec<RepoSearchResult>> {
        let q = build_search_query(query);
        let url = format!("{API_BASE}/search/repositories");
        let resp = Self::check(
            self.request(reqwest::Method::GET, &url)
                .query(&[
                    ("q", q.as_str()),
                    ("sort", "stars"),
                    ("order", "desc"),
                    ("per_page", &limit.min(50).to_string()),
                ])
                .send()
                .await?,
        )
        .await?;
        let parsed: SearchResponse = resp.json().await?;
        Ok(parsed
            .items
            .into_iter()
            .map(|r| RepoSearchResult {
                full_name: r.full_name,
                description: r.description.unwrap_or_default(),
                url: r.html_url,
                stars: r.stargazers_count,
                topics: r.topics,
                updated_at: r.pushed_at,
            })
            .collect())
    }
}

/// Build a GitHub search query that favours skill collections.
pub fn build_search_query(query: &str) -> String {
    let q = query.trim();
    if q.is_empty() {
        "topic:agent-skills OR topic:claude-skills OR topic:claude-code-skills OR topic:codex-skills".to_string()
    } else if q.contains(':') {
        q.to_string()
    } else {
        format!("{q} skills in:name,description,readme")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_query_shapes() {
        assert!(build_search_query("").contains("topic:agent-skills"));
        assert_eq!(build_search_query("topic:x"), "topic:x");
        assert_eq!(
            build_search_query("pdf"),
            "pdf skills in:name,description,readme"
        );
    }
}
