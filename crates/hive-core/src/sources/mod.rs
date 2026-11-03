//! Online and local sources: parse what the user typed, fetch it, preview what it contains,
//! import the parts they want, and check for updates later.
//!
//! Accepted inputs:
//! - `owner/repo`, `owner/repo/path/to/skill`, `owner/repo@branch`, `gh:owner/repo`
//! - `https://github.com/owner/repo`, `.../tree/<ref>/<path>`, `.../blob/<ref>/<path>/SKILL.md`
//! - any git URL (`https://gitlab.com/x/y.git`, `git@host:x/y.git`, `ssh://…`, `file:///…`)
//! - a URL to a `.zip` or `.tar.gz` archive
//! - a local directory or archive path

use std::path::{Path, PathBuf};

use crate::discovery;
use crate::error::{HiveError, IoContext, Result};
use crate::hash;
use crate::hive::Hive;
use crate::model::{
    AssetKind, Candidate, CandidateStatus, CatalogEntry, HiveEvent, ImportResult, ImportSkip,
    LibraryItem, RepoSearchResult, SourceKind, SourcePreview, SourceRef, SourceSpec, UpdateCheck,
};
use crate::util;

pub mod archive;
pub mod catalog;
pub mod git;
pub mod github;

const PREVIEW_DEPTH: usize = 6;
const README_CAP: usize = 24 * 1024;

fn looks_like_local_path(input: &str) -> bool {
    let p = Path::new(input);
    input.starts_with('/')
        || input.starts_with("./")
        || input.starts_with("../")
        || input.starts_with('~')
        || input.starts_with(".\\")
        || (input.len() >= 3
            && input.as_bytes()[1] == b':'
            && (input.as_bytes()[2] == b'\\' || input.as_bytes()[2] == b'/'))
        || p.exists()
}

fn split_ref(segment: &str) -> (String, Option<String>) {
    for sep in ['@', '#'] {
        if let Some((a, b)) = segment.rsplit_once(sep) {
            if !b.is_empty() && !a.is_empty() {
                return (a.to_string(), Some(b.to_string()));
            }
        }
    }
    (segment.to_string(), None)
}

fn github_spec(
    input: &str,
    owner: &str,
    repo: &str,
    reference: Option<String>,
    subpath: Option<String>,
) -> SourceSpec {
    let repo = repo.trim_end_matches(".git");
    SourceSpec {
        kind: SourceKind::Github,
        locator: format!("{owner}/{repo}"),
        clone_url: Some(format!("https://github.com/{owner}/{repo}.git")),
        subpath: subpath
            .map(|s| s.trim_matches('/').to_string())
            .filter(|s| !s.is_empty()),
        reference,
        input: input.to_string(),
    }
}

/// Parse a source string. Never touches the network.
pub fn parse_spec(input: &str) -> Result<SourceSpec> {
    let raw = input.trim();
    if raw.is_empty() {
        return Err(HiveError::invalid("source is empty"));
    }
    let lower = raw.to_lowercase();

    // Local paths first so that `./owner/repo` style paths are not mistaken for GitHub.
    if looks_like_local_path(raw) && !lower.starts_with("http") {
        let path = crate::paths::expand_tilde(raw);
        return Ok(SourceSpec {
            kind: SourceKind::LocalPath,
            locator: path.to_string_lossy().to_string(),
            clone_url: None,
            subpath: None,
            reference: None,
            input: raw.to_string(),
        });
    }

    // Explicit prefixes.
    for prefix in ["gh:", "github:"] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            return parse_spec(&raw[raw.len() - rest.len()..]);
        }
    }

    // GitHub URLs.
    if let Some(rest) = lower
        .strip_prefix("https://github.com/")
        .or_else(|| lower.strip_prefix("http://github.com/"))
        .or_else(|| lower.strip_prefix("github.com/"))
        .or_else(|| lower.strip_prefix("https://www.github.com/"))
    {
        let rest = &raw[raw.len() - rest.len()..];
        let rest = rest.split(['?', '#']).next().unwrap_or(rest);
        let parts: Vec<&str> = rest
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() < 2 {
            return Err(HiveError::invalid("GitHub URL needs owner and repository"));
        }
        let (owner, repo) = (parts[0], parts[1]);
        let mut reference = None;
        let mut subpath = None;
        if parts.len() >= 4 && (parts[2] == "tree" || parts[2] == "blob") {
            reference = Some(parts[3].to_string());
            let mut tail: Vec<&str> = parts[4..].to_vec();
            if parts[2] == "blob" {
                tail.pop(); // the file itself; we want its directory
            }
            if !tail.is_empty() {
                subpath = Some(tail.join("/"));
            }
        } else if parts.len() == 3 && parts[2] == "commits" {
            // ignore
        }
        return Ok(github_spec(raw, owner, repo, reference, subpath));
    }

    // raw.githubusercontent.com/owner/repo/ref/path/SKILL.md
    if let Some(rest) = lower.strip_prefix("https://raw.githubusercontent.com/") {
        let rest = &raw[raw.len() - rest.len()..];
        let parts: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();
        if parts.len() >= 3 {
            let mut tail: Vec<&str> = parts[3..].to_vec();
            if tail
                .last()
                .map(|f| f.to_lowercase().ends_with(".md"))
                .unwrap_or(false)
            {
                tail.pop();
            }
            let subpath = (!tail.is_empty()).then(|| tail.join("/"));
            return Ok(github_spec(
                raw,
                parts[0],
                parts[1],
                Some(parts[2].to_string()),
                subpath,
            ));
        }
    }

    // Archive URLs.
    if (lower.starts_with("http://") || lower.starts_with("https://"))
        && (lower.ends_with(".zip") || lower.ends_with(".tar.gz") || lower.ends_with(".tgz"))
    {
        return Ok(SourceSpec {
            kind: SourceKind::Url,
            locator: raw.to_string(),
            clone_url: None,
            subpath: None,
            reference: None,
            input: raw.to_string(),
        });
    }

    // Generic git URLs.
    let is_git_url = lower.starts_with("git@")
        || lower.starts_with("ssh://")
        || lower.starts_with("git://")
        || lower.starts_with("file://")
        || ((lower.starts_with("http://") || lower.starts_with("https://")) && lower.contains('.'));
    if is_git_url {
        let (url, reference) = split_ref(raw);
        let locator = url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("ssh://")
            .trim_start_matches("git://")
            .trim_end_matches('/')
            .trim_end_matches(".git")
            .to_string();
        return Ok(SourceSpec {
            kind: SourceKind::Git,
            locator,
            clone_url: Some(url),
            subpath: None,
            reference,
            input: raw.to_string(),
        });
    }

    // owner/repo[/subpath][@ref]
    let (body, reference) = split_ref(raw);
    let parts: Vec<&str> = body.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() >= 2
        && parts[..2].iter().all(|p| {
            p.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        })
    {
        let subpath = (parts.len() > 2).then(|| parts[2..].join("/"));
        return Ok(github_spec(raw, parts[0], parts[1], reference, subpath));
    }

    Err(HiveError::invalid(format!(
        "could not understand source '{raw}'. Use owner/repo, a GitHub or git URL, an archive URL or a local path"
    )))
}

/// A fetched source on local disk.
#[derive(Debug, Clone)]
pub struct Fetched {
    /// Repository/archive root on disk.
    pub root: PathBuf,
    /// `root` joined with the spec's subpath.
    pub content: PathBuf,
    pub revision: Option<String>,
}

impl Hive {
    pub fn parse_source(&self, input: &str) -> Result<SourceSpec> {
        parse_spec(input)
    }

    /// Fetch a source to local disk (clone, download or just resolve a path).
    pub async fn fetch_source(&self, spec: &SourceSpec) -> Result<Fetched> {
        let fetched = match spec.kind {
            SourceKind::LocalPath => {
                let root = PathBuf::from(&spec.locator);
                if !root.exists() {
                    return Err(HiveError::not_found(format!(
                        "{} does not exist",
                        root.display()
                    )));
                }
                if root.is_file() {
                    let bytes = std::fs::read(&root).at(&root)?;
                    let dest = self
                        .paths
                        .cache_dir()
                        .join("archives")
                        .join(&hash::hash_bytes(&bytes)[..20]);
                    if !dest.exists() {
                        archive::extract(&bytes, &spec.locator, &dest)?;
                    }
                    Fetched {
                        root: archive::single_root(&dest),
                        content: archive::single_root(&dest),
                        revision: None,
                    }
                } else {
                    Fetched {
                        root: root.clone(),
                        content: root,
                        revision: None,
                    }
                }
            }
            SourceKind::Github | SourceKind::Git | SourceKind::Catalog => {
                let url = spec
                    .clone_url
                    .clone()
                    .ok_or_else(|| HiveError::invalid("source has no clone URL"))?;
                self.progress("fetch", format!("Fetching {}", spec.locator), None, None);
                let (root, revision) = if git::git_available() {
                    git::clone_or_update(
                        &self.paths.git_cache_dir(),
                        &url,
                        spec.reference.as_deref(),
                    )
                    .await?
                } else if spec.kind == SourceKind::Github {
                    self.github_tarball(spec).await?
                } else {
                    return Err(HiveError::msg("git is required to fetch this source"));
                };
                let content = match &spec.subpath {
                    Some(sub) => root.join(sub),
                    None => root.clone(),
                };
                Fetched {
                    root,
                    content,
                    revision: Some(revision),
                }
            }
            SourceKind::Url => {
                self.progress("fetch", format!("Downloading {}", spec.locator), None, None);
                let resp = self
                    .http
                    .get(&spec.locator)
                    .send()
                    .await?
                    .error_for_status()?;
                let bytes = resp.bytes().await?;
                if bytes.len() as u64 > archive::MAX_ARCHIVE_BYTES {
                    return Err(HiveError::invalid("archive is too large"));
                }
                let dest = self
                    .paths
                    .cache_dir()
                    .join("archives")
                    .join(&hash::hash_bytes(&bytes)[..20]);
                if !dest.exists() {
                    archive::extract(&bytes, &spec.locator, &dest)?;
                }
                let root = archive::single_root(&dest);
                Fetched {
                    content: root.clone(),
                    root,
                    revision: Some(hash::hash_bytes(&bytes)[..12].to_string()),
                }
            }
            SourceKind::AgentDir | SourceKind::Authored => {
                return Err(HiveError::invalid("this source kind cannot be fetched"));
            }
        };
        if !fetched.content.exists() {
            return Err(HiveError::not_found(format!(
                "path '{}' does not exist in {}",
                spec.subpath.clone().unwrap_or_default(),
                spec.locator
            )));
        }
        Ok(fetched)
    }

    async fn github_tarball(&self, spec: &SourceSpec) -> Result<(PathBuf, String)> {
        let (owner, repo) = spec
            .locator
            .split_once('/')
            .ok_or_else(|| HiveError::invalid("bad GitHub locator"))?;
        let client = github::GithubClient::new(self.http.clone(), self.github_token());
        let revision = client
            .latest_commit(owner, repo, spec.reference.as_deref())
            .await?
            .unwrap_or_else(|| "unknown".into());
        let dest = git::cache_dir_for(
            &self.paths.git_cache_dir(),
            spec.clone_url.as_deref().unwrap_or(""),
            Some(&revision),
        );
        if !dest.join(".hive-tarball").exists() {
            let bytes = client.tarball(owner, repo, Some(&revision)).await?;
            let _ = util::remove_path(&dest);
            archive::extract(&bytes, "repo.tar.gz", &dest)?;
            std::fs::write(dest.join(".hive-tarball"), &revision).at(&dest)?;
        }
        Ok((archive::single_root(&dest), revision))
    }

    /// Candidates inside a fetched source, relative paths recorded in each candidate's source.
    pub fn candidates_in(&self, spec: &SourceSpec, fetched: &Fetched) -> Vec<Candidate> {
        let lib = self.library();
        let mut found: Vec<(AssetKind, PathBuf)> =
            discovery::find_skill_dirs(&fetched.content, PREVIEW_DEPTH)
                .into_iter()
                .map(|p| (AssetKind::Skill, p))
                .collect();
        // Agents and commands in conventional folders.
        for (kind, names) in [
            (AssetKind::Agent, ["agents", ".claude/agents"]),
            (AssetKind::Command, ["commands", ".claude/commands"]),
        ] {
            for name in names {
                let dir = fetched.content.join(name);
                if dir.is_dir() {
                    for file in discovery::find_markdown_assets(&dir, kind) {
                        found.push((kind, file));
                    }
                }
            }
        }
        let now = util::now_ms();
        let mut out: Vec<Candidate> = found
            .into_iter()
            .map(|(kind, path)| {
                let rel = path
                    .strip_prefix(&fetched.root)
                    .map(|r| r.to_string_lossy().replace('\\', "/"))
                    .ok()
                    .filter(|r| !r.is_empty());
                let source = SourceRef {
                    kind: if spec.kind == SourceKind::Catalog {
                        SourceKind::Github
                    } else {
                        spec.kind
                    },
                    locator: spec.locator.clone(),
                    subpath: rel,
                    reference: spec.reference.clone(),
                    revision: fetched.revision.clone(),
                    target_id: None,
                    fetched_at: now,
                };
                lib.candidate_for(kind, &path, source)
            })
            .collect();
        out.sort_by_key(|c| c.name.to_lowercase());
        out
    }

    /// Fetch a source and list what it contains.
    pub async fn preview_source(&self, input: &str) -> Result<SourcePreview> {
        let spec = parse_spec(input)?;
        let fetched = self.fetch_source(&spec).await?;
        let candidates = self.candidates_in(&spec, &fetched);
        let mut warnings = Vec::new();
        if candidates.is_empty() {
            warnings.push(
                "No SKILL.md directories, agents or commands were found at this location".into(),
            );
        }
        let invalid = candidates
            .iter()
            .filter(|c| c.status == CandidateStatus::Invalid)
            .count();
        if invalid > 0 {
            warnings.push(format!(
                "{invalid} item(s) fail validation and will be skipped on import"
            ));
        }
        let readme = ["README.md", "readme.md", "README.MD", "Readme.md"]
            .iter()
            .map(|n| fetched.content.join(n))
            .chain(["README.md"].iter().map(|n| fetched.root.join(n)))
            .find(|p| p.is_file())
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|t| util::truncate_chars(&t, README_CAP));
        Ok(SourcePreview {
            spec,
            revision: fetched.revision,
            candidates,
            readme,
            warnings,
        })
    }

    /// Import from a source. `paths` restricts the import to those candidate paths (from a
    /// preview); `None` imports every valid candidate.
    pub async fn import_from_source(
        &self,
        input: &str,
        paths: Option<&[String]>,
        tags: &[String],
        overwrite: bool,
    ) -> Result<ImportResult> {
        let spec = parse_spec(input)?;
        let fetched = self.fetch_source(&spec).await?;
        let candidates = self.candidates_in(&spec, &fetched);
        let lib = self.library();
        let mut imported = Vec::new();
        let mut skipped = Vec::new();
        let total = candidates.len() as u32;
        for (i, cand) in candidates.into_iter().enumerate() {
            if let Some(filter) = paths {
                if !filter.iter().any(|p| p == &cand.path) {
                    continue;
                }
            }
            self.progress(
                "import",
                format!("Importing {}", cand.name),
                Some(i as u32 + 1),
                Some(total),
            );
            if cand.status == CandidateStatus::Invalid {
                skipped.push(ImportSkip {
                    path: cand.path,
                    reason: cand
                        .validation
                        .findings
                        .iter()
                        .find(|f| f.severity == crate::model::Severity::Error)
                        .map(|f| f.message.clone())
                        .unwrap_or_else(|| "invalid".into()),
                });
                continue;
            }
            match lib.import_path(Path::new(&cand.path), cand.source.clone(), tags, overwrite) {
                Ok(item) => imported.push(item),
                Err(err) => skipped.push(ImportSkip {
                    path: cand.path,
                    reason: err.to_string(),
                }),
            }
        }
        if !imported.is_empty() {
            self.emit(HiveEvent::LibraryChanged);
        }
        Ok(ImportResult { imported, skipped })
    }

    /// Rebuild the [`SourceSpec`] an item was imported from.
    pub fn spec_for_item(&self, item: &LibraryItem) -> Option<SourceSpec> {
        let s = &item.source;
        match s.kind {
            SourceKind::Github | SourceKind::Catalog => {
                let mut spec = parse_spec(&s.locator).ok()?;
                spec.subpath = s.subpath.clone();
                spec.reference = s.reference.clone();
                Some(spec)
            }
            SourceKind::Git => {
                let url = if s.locator.contains("://") || s.locator.starts_with("git@") {
                    s.locator.clone()
                } else {
                    format!("https://{}.git", s.locator)
                };
                Some(SourceSpec {
                    kind: SourceKind::Git,
                    locator: s.locator.clone(),
                    clone_url: Some(url),
                    subpath: s.subpath.clone(),
                    reference: s.reference.clone(),
                    input: s.locator.clone(),
                })
            }
            SourceKind::Url => Some(SourceSpec {
                kind: SourceKind::Url,
                locator: s.locator.clone(),
                clone_url: None,
                subpath: s.subpath.clone(),
                reference: None,
                input: s.locator.clone(),
            }),
            SourceKind::LocalPath | SourceKind::AgentDir => {
                let path = Path::new(&s.locator);
                path.exists().then(|| SourceSpec {
                    kind: SourceKind::LocalPath,
                    locator: s.locator.clone(),
                    clone_url: None,
                    subpath: None,
                    reference: None,
                    input: s.locator.clone(),
                })
            }
            SourceKind::Authored => None,
        }
    }

    /// Check whether newer content exists at the item's source. `deep` fetches and compares
    /// content; otherwise only the remote revision is compared.
    pub async fn check_update(&self, item_id: &str, deep: bool) -> Result<UpdateCheck> {
        let item = self.get_item(item_id)?;
        let mut check = UpdateCheck {
            item_id: item.id.clone(),
            current_revision: item.source.revision.clone(),
            latest_revision: None,
            update_available: false,
            content_changed: false,
            error: None,
        };
        let Some(spec) = self.spec_for_item(&item) else {
            check.error = Some("item has no remote source".into());
            return Ok(check);
        };
        let result: Result<()> = async {
            match spec.kind {
                SourceKind::Github | SourceKind::Git if !deep => {
                    let url = spec.clone_url.clone().unwrap_or_default();
                    let latest = if git::git_available() {
                        git::remote_revision(&url, spec.reference.as_deref()).await?
                    } else if let Some((o, r)) = spec.locator.split_once('/') {
                        github::GithubClient::new(self.http.clone(), self.github_token())
                            .latest_commit(o, r, spec.reference.as_deref())
                            .await?
                    } else {
                        None
                    };
                    check.update_available = latest.is_some() && latest != check.current_revision;
                    check.latest_revision = latest;
                }
                _ => {
                    let fetched = self.fetch_source(&spec).await?;
                    let (new_hash, _) = hash::hash_path(&fetched.content)?;
                    check.latest_revision = fetched.revision.clone();
                    check.content_changed = new_hash != item.content_hash;
                    check.update_available = check.content_changed;
                }
            }
            Ok(())
        }
        .await;
        if let Err(err) = result {
            check.error = Some(err.to_string());
        }
        Ok(check)
    }

    pub async fn check_updates(&self, item_ids: Option<&[String]>) -> Result<Vec<UpdateCheck>> {
        let items: Vec<LibraryItem> = match item_ids {
            Some(ids) => self.db.items_by_ids(ids)?,
            None => self
                .db
                .all_items()?
                .into_iter()
                .filter(|i| {
                    matches!(
                        i.source.kind,
                        SourceKind::Github
                            | SourceKind::Git
                            | SourceKind::Url
                            | SourceKind::Catalog
                    )
                })
                .collect(),
        };
        let mut out = Vec::new();
        for item in items {
            out.push(self.check_update(&item.id, false).await?);
        }
        Ok(out)
    }

    /// Replace an item's content with the latest version from its source.
    pub async fn update_item_from_source(&self, item_id: &str) -> Result<LibraryItem> {
        let item = self.get_item(item_id)?;
        let spec = self
            .spec_for_item(&item)
            .ok_or_else(|| HiveError::invalid("item has no remote source to update from"))?;
        let fetched = self.fetch_source(&spec).await?;
        let path = &fetched.content;
        if crate::library::detect_kind(path) != Some(item.kind) {
            return Err(HiveError::invalid(format!(
                "{} no longer contains a {} at {}",
                spec.locator,
                item.kind.label().to_lowercase(),
                spec.subpath.clone().unwrap_or_default()
            )));
        }
        let mut source = item.source.clone();
        source.revision = fetched.revision.clone();
        source.fetched_at = util::now_ms();
        let updated = self
            .library()
            .replace_content(&item, path, Some(source), &[])?;
        // Copy-mode deployments are now stale; link-mode ones follow automatically.
        self.emit(HiveEvent::LibraryChanged);
        self.emit(HiveEvent::DeploymentsChanged);
        self.db.audit("update", &serde_json::json!({"id": updated.id, "name": updated.name, "revision": updated.source.revision}))?;
        Ok(updated)
    }

    /// The curated catalog, refreshed from the remote file at most once a day.
    pub async fn catalog(&self, refresh: bool) -> Result<Vec<CatalogEntry>> {
        if !refresh {
            if let Some(cached) = self
                .db
                .cache_get::<Vec<CatalogEntry>>(catalog::CACHE_KEY, catalog::CACHE_MAX_AGE_MS)?
            {
                return Ok(cached);
            }
        }
        let remote = async {
            let text = self
                .http
                .get(catalog::REMOTE_URL)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
                .ok()?
                .error_for_status()
                .ok()?
                .text()
                .await
                .ok()?;
            catalog::parse_remote(&text)
        }
        .await;
        let entries = remote.unwrap_or_else(catalog::builtin);
        let _ = self.db.cache_put(catalog::CACHE_KEY, &entries);
        Ok(entries)
    }

    pub fn catalog_builtin(&self) -> Vec<CatalogEntry> {
        catalog::builtin()
    }

    /// Search GitHub for skill repositories.
    pub async fn search_github(&self, query: &str, limit: u32) -> Result<Vec<RepoSearchResult>> {
        github::GithubClient::new(self.http.clone(), self.github_token())
            .search(query, limit)
            .await
    }

    /// Delete cached clones and archives. Returns bytes freed.
    pub fn clear_source_cache(&self) -> Result<u64> {
        let mut freed = git::clear_cache(&self.paths.git_cache_dir())?;
        let archives = self.paths.cache_dir().join("archives");
        freed += util::dir_size(&archives);
        let _ = util::remove_path(&archives);
        Ok(freed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_github_forms() {
        let s = parse_spec("anthropics/skills").unwrap();
        assert_eq!(s.kind, SourceKind::Github);
        assert_eq!(s.locator, "anthropics/skills");
        assert_eq!(
            s.clone_url.as_deref(),
            Some("https://github.com/anthropics/skills.git")
        );
        assert!(s.subpath.is_none());

        let s = parse_spec("anthropics/skills/skills/pdf@main").unwrap();
        assert_eq!(s.subpath.as_deref(), Some("skills/pdf"));
        assert_eq!(s.reference.as_deref(), Some("main"));

        let s = parse_spec("https://github.com/anthropics/skills/tree/main/skills/pdf").unwrap();
        assert_eq!(s.locator, "anthropics/skills");
        assert_eq!(s.subpath.as_deref(), Some("skills/pdf"));
        assert_eq!(s.reference.as_deref(), Some("main"));

        let s = parse_spec("https://github.com/anthropics/skills/blob/main/skills/pdf/SKILL.md")
            .unwrap();
        assert_eq!(s.subpath.as_deref(), Some("skills/pdf"));

        let s = parse_spec("https://github.com/anthropics/skills.git").unwrap();
        assert_eq!(s.locator, "anthropics/skills");

        let s = parse_spec("gh:obra/superpowers").unwrap();
        assert_eq!(s.locator, "obra/superpowers");

        let s = parse_spec("https://raw.githubusercontent.com/o/r/main/skills/x/SKILL.md").unwrap();
        assert_eq!(s.subpath.as_deref(), Some("skills/x"));
        assert_eq!(s.reference.as_deref(), Some("main"));
    }

    #[test]
    fn parses_git_archive_and_local() {
        let s = parse_spec("https://gitlab.com/group/proj.git#v2").unwrap();
        assert_eq!(s.kind, SourceKind::Git);
        assert_eq!(
            s.clone_url.as_deref(),
            Some("https://gitlab.com/group/proj.git")
        );
        assert_eq!(s.reference.as_deref(), Some("v2"));
        assert_eq!(s.locator, "gitlab.com/group/proj");

        let s = parse_spec("git@github.com:owner/repo.git").unwrap();
        assert_eq!(s.kind, SourceKind::Git);

        let s = parse_spec("https://example.com/bundle.zip").unwrap();
        assert_eq!(s.kind, SourceKind::Url);

        let s = parse_spec("~/skills").unwrap();
        assert_eq!(s.kind, SourceKind::LocalPath);
        let s = parse_spec("./rel/path").unwrap();
        assert_eq!(s.kind, SourceKind::LocalPath);

        assert!(parse_spec("").is_err());
        assert!(parse_spec("just-one-word").is_err());
    }

    #[tokio::test]
    async fn preview_import_update_from_local_git_repo() {
        if !git::git_available() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let hive = Hive::open_temp(tmp.path()).unwrap();
        let repo = git::test_support::make_repo(tmp.path());
        let url = format!("file://{}", repo.display());

        let preview = hive.preview_source(&url).await.unwrap();
        assert_eq!(preview.spec.kind, SourceKind::Git);
        assert_eq!(preview.candidates.len(), 1);
        assert_eq!(preview.candidates[0].name, "remote-skill");
        assert_eq!(
            preview.candidates[0].source.subpath.as_deref(),
            Some("skills/remote-skill")
        );
        assert!(
            preview
                .readme
                .as_deref()
                .unwrap_or("")
                .contains("Remote repo")
        );
        assert!(preview.revision.is_some());

        let result = hive
            .import_from_source(&url, None, &["remote".into()], false)
            .await
            .unwrap();
        assert_eq!(result.imported.len(), 1, "{:?}", result.skipped);
        let item = &result.imported[0];
        assert_eq!(item.source.kind, SourceKind::Git);
        assert_eq!(item.source.revision, preview.revision);
        assert_eq!(item.tags, vec!["remote"]);

        // No change yet.
        let check = hive.check_update(&item.id, false).await.unwrap();
        assert!(!check.update_available, "{check:?}");

        git::test_support::commit_change(&repo, "Body v2");
        let check = hive.check_update(&item.id, false).await.unwrap();
        assert!(check.update_available);
        let deep = hive.check_update(&item.id, true).await.unwrap();
        assert!(deep.content_changed);

        let updated = hive.update_item_from_source(&item.id).await.unwrap();
        assert_ne!(updated.content_hash, item.content_hash);
        assert_eq!(updated.source.revision, check.latest_revision);
        assert!(
            hive.read_item_file(&updated.id, "SKILL.md")
                .unwrap()
                .contains("Body v2")
        );

        // Importing again is a no-op (identical) unless something changed.
        let again = hive
            .import_from_source(&url, None, &[], false)
            .await
            .unwrap();
        assert!(again.imported.is_empty());
        assert_eq!(again.skipped.len(), 1);

        assert_eq!(hive.check_updates(None).await.unwrap().len(), 1);
        assert!(hive.clear_source_cache().unwrap() > 0);
    }

    #[tokio::test]
    async fn preview_local_directory_and_subpath_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let hive = Hive::open_temp(tmp.path()).unwrap();
        let src = tmp.path().join("collection");
        crate::library::test_support::write_skill(
            &src.join("a"),
            "one",
            "One does one thing when asked for one",
            "1",
        );
        crate::library::test_support::write_skill(
            &src.join("b/c"),
            "two",
            "Two does two things when asked for two",
            "2",
        );
        let preview = hive.preview_source(&src.to_string_lossy()).await.unwrap();
        assert_eq!(preview.candidates.len(), 2);
        let only_two: Vec<String> = preview
            .candidates
            .iter()
            .filter(|c| c.name == "two")
            .map(|c| c.path.clone())
            .collect();
        let result = hive
            .import_from_source(&src.to_string_lossy(), Some(&only_two), &[], false)
            .await
            .unwrap();
        assert_eq!(result.imported.len(), 1);
        assert_eq!(result.imported[0].name, "two");
        assert!(
            hive.preview_source(&tmp.path().join("missing").to_string_lossy())
                .await
                .is_err()
        );
        assert!(!hive.catalog_builtin().is_empty());
    }
}
