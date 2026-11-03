//! Domain model shared by every service, the CLI, the desktop shell and the UI.
//!
//! Every type here is serialisable and exported to TypeScript through `ts-rs`
//! (`cargo test -p hive-core export_bindings`). Field names use camelCase on the wire.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ---------------------------------------------------------------------------
// Assets: skills, subagents and commands
// ---------------------------------------------------------------------------

/// What kind of thing a library item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum AssetKind {
    /// A directory with a `SKILL.md` (Agent Skills format).
    Skill,
    /// A subagent definition: markdown file with frontmatter (`name`, `description`, `tools`, `model`).
    Agent,
    /// A slash command: markdown file with optional frontmatter.
    Command,
}

impl AssetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AssetKind::Skill => "skill",
            AssetKind::Agent => "agent",
            AssetKind::Command => "command",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "skill" => Some(AssetKind::Skill),
            "agent" => Some(AssetKind::Agent),
            "command" => Some(AssetKind::Command),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AssetKind::Skill => "Skill",
            AssetKind::Agent => "Agent",
            AssetKind::Command => "Command",
        }
    }
}

/// Frontmatter of a `SKILL.md` as defined by the Agent Skills specification.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SkillFrontmatter {
    pub name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Pre-approved tools (`allowed-tools`), split on whitespace or taken from a list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<String>,
    /// Free-form `metadata` map (author, version, …).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
    /// Any other keys, kept verbatim so nothing is lost on round trips.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[ts(type = "Record<string, unknown>")]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// A parsed markdown-with-frontmatter document (skill, agent or command).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ParsedDocument {
    pub kind: AssetKind,
    pub name: String,
    pub description: String,
    /// Full frontmatter as JSON (skill: [`SkillFrontmatter`]; agent/command: their own keys).
    #[ts(type = "Record<string, unknown>")]
    pub frontmatter: serde_json::Value,
    /// Markdown body after the frontmatter.
    pub body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// One validation finding against the skill format rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Finding {
    /// Stable code such as `name.missing`, `description.too_long`.
    pub code: String,
    pub severity: Severity,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ValidationStatus {
    Ok,
    Warnings,
    Errors,
    Unknown,
}

impl ValidationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ValidationStatus::Ok => "ok",
            ValidationStatus::Warnings => "warnings",
            ValidationStatus::Errors => "errors",
            ValidationStatus::Unknown => "unknown",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "ok" => ValidationStatus::Ok,
            "warnings" => ValidationStatus::Warnings,
            "errors" => ValidationStatus::Errors,
            _ => ValidationStatus::Unknown,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ValidationReport {
    pub findings: Vec<Finding>,
}

impl ValidationReport {
    pub fn status(&self) -> ValidationStatus {
        if self.findings.iter().any(|f| f.severity == Severity::Error) {
            ValidationStatus::Errors
        } else if self
            .findings
            .iter()
            .any(|f| f.severity == Severity::Warning)
        {
            ValidationStatus::Warnings
        } else {
            ValidationStatus::Ok
        }
    }
    pub fn is_ok(&self) -> bool {
        !self.findings.iter().any(|f| f.severity == Severity::Error)
    }
}

// ---------------------------------------------------------------------------
// Provenance
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SourceKind {
    /// Copied from a directory on this machine.
    LocalPath,
    /// Collected from an agent's own skills directory (e.g. `~/.claude/skills`).
    AgentDir,
    /// Cloned from a git repository (any host).
    Git,
    /// A GitHub repository, fetched through the API or a clone.
    Github,
    /// Entry from the built-in or a remote catalog.
    Catalog,
    /// Downloaded archive or single file from a URL.
    Url,
    /// Written inside Skill Hive.
    Authored,
}

impl SourceKind {
    pub fn label(self) -> &'static str {
        match self {
            SourceKind::LocalPath => "Local",
            SourceKind::AgentDir => "Agent",
            SourceKind::Git => "Git",
            SourceKind::Github => "GitHub",
            SourceKind::Catalog => "Catalog",
            SourceKind::Url => "URL",
            SourceKind::Authored => "Authored",
        }
    }
}

/// Where a library item came from, enough to check for updates later.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SourceRef {
    pub kind: SourceKind,
    /// Path, URL or `owner/repo`.
    pub locator: String,
    /// Path of the asset inside the repository, when it is not the root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subpath: Option<String>,
    /// Branch or tag that was requested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// Commit that was actually fetched.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    /// Agent target the asset was collected from (for `AgentDir`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
    pub fetched_at: i64,
}

impl SourceRef {
    pub fn local(path: impl Into<String>) -> Self {
        Self {
            kind: SourceKind::LocalPath,
            locator: path.into(),
            subpath: None,
            reference: None,
            revision: None,
            target_id: None,
            fetched_at: crate::util::now_ms(),
        }
    }
    pub fn authored() -> Self {
        Self {
            kind: SourceKind::Authored,
            locator: String::new(),
            subpath: None,
            reference: None,
            revision: None,
            target_id: None,
            fetched_at: crate::util::now_ms(),
        }
    }
    /// Human readable one-liner.
    pub fn display(&self) -> String {
        let mut s = self.locator.clone();
        if let Some(sub) = &self.subpath {
            if !sub.is_empty() {
                s.push('/');
                s.push_str(sub);
            }
        }
        if let Some(r) = &self.reference {
            s.push('@');
            s.push_str(r);
        }
        s
    }
}

// ---------------------------------------------------------------------------
// Library
// ---------------------------------------------------------------------------

/// Summary of the latest evaluation of an item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EffectivenessSummary {
    pub run_id: String,
    pub verdict: Verdict,
    pub with_skill_pass_rate: f64,
    pub baseline_pass_rate: f64,
    pub delta: f64,
    pub finished_at: i64,
}

/// An item in the library.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LibraryItem {
    pub id: String,
    pub kind: AssetKind,
    pub name: String,
    pub description: String,
    /// Directory (skills) or file name (agents, commands) inside the library.
    pub slug: String,
    /// Absolute path of the item inside the library.
    pub path: String,
    pub content_hash: String,
    pub size_bytes: u64,
    /// Estimated tokens of the whole instruction body.
    pub token_estimate: u32,
    /// Estimated tokens of the description (what agents load into context up front).
    pub description_tokens: u32,
    pub tags: Vec<String>,
    pub source: SourceRef,
    #[ts(type = "Record<string, unknown>")]
    pub frontmatter: serde_json::Value,
    pub validation: ValidationStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effectiveness: Option<EffectivenessSummary>,
    pub favorite: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub deployment_count: u32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FileEntry {
    /// Path relative to the item root.
    pub path: String,
    pub size_bytes: u64,
    pub is_dir: bool,
}

/// Everything the detail view needs for one item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LibraryItemDetail {
    pub item: LibraryItem,
    /// Markdown body of the main document.
    pub body: String,
    pub files: Vec<FileEntry>,
    pub validation: ValidationReport,
    pub deployments: Vec<Deployment>,
    pub loadouts: Vec<LoadoutRef>,
    pub history: Vec<HistoryPoint>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LibraryQuery {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub kinds: Vec<AssetKind>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source_kinds: Vec<SourceKind>,
    #[serde(default)]
    pub favorites_only: bool,
    /// Only items deployed to this target id.
    #[serde(default)]
    pub deployed_to: Option<String>,
    #[serde(default)]
    pub sort: LibrarySort,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum LibrarySort {
    #[default]
    Relevance,
    Name,
    Updated,
    Effectiveness,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LibraryStats {
    pub total: u32,
    pub skills: u32,
    pub agents: u32,
    pub commands: u32,
    pub deployed: u32,
    pub tested: u32,
    pub with_errors: u32,
    pub tags: Vec<TagCount>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TagCount {
    pub tag: String,
    pub count: u32,
}

/// Something found on disk or at a remote source that can be imported.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Candidate {
    /// Stable within one preview/scan response.
    pub id: String,
    pub kind: AssetKind,
    pub name: String,
    pub description: String,
    /// Local path (already on disk or in the fetch cache).
    pub path: String,
    pub content_hash: String,
    pub size_bytes: u64,
    pub token_estimate: u32,
    pub source: SourceRef,
    pub validation: ValidationReport,
    pub status: CandidateStatus,
    /// Name of the library item this collides with, when relevant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library_item_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum CandidateStatus {
    /// Not in the library.
    New,
    /// Byte-identical content is already in the library.
    InLibrary,
    /// Same name as a library item but different content.
    Modified,
    /// A symlink that already points into the library.
    Managed,
    /// Could not be parsed; see validation findings.
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScanReport {
    pub candidates: Vec<Candidate>,
    pub roots_scanned: Vec<String>,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportRequest {
    /// Candidate paths to import (from a scan or a source preview).
    pub paths: Vec<String>,
    pub source: SourceRef,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Replace an existing item with the same name.
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportResult {
    pub imported: Vec<LibraryItem>,
    pub skipped: Vec<ImportSkip>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportSkip {
    pub path: String,
    pub reason: String,
}

/// Result of checking a remote source for a newer revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateCheck {
    pub item_id: String,
    pub current_revision: Option<String>,
    pub latest_revision: Option<String>,
    pub update_available: bool,
    pub content_changed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Sources (online)
// ---------------------------------------------------------------------------

/// A parsed source specification such as `anthropics/skills`, a GitHub tree URL or a git URL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SourceSpec {
    pub kind: SourceKind,
    /// Normalised locator (`owner/repo` for GitHub, clone URL for git, path for local).
    pub locator: String,
    /// Clone URL for git-based sources.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clone_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subpath: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// What the user typed.
    pub input: String,
}

/// Preview of a source before anything is imported.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SourcePreview {
    pub spec: SourceSpec,
    pub revision: Option<String>,
    pub candidates: Vec<Candidate>,
    /// Repository README (markdown), when available, to help the user decide.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readme: Option<String>,
    pub warnings: Vec<String>,
}

/// Entry in the curated catalog of well-known skill collections.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Source string accepted by `sources::parse_spec`.
    pub source: String,
    pub homepage: Option<String>,
    pub tags: Vec<String>,
    pub publisher: String,
    /// Rough number of skills, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approx_count: Option<u32>,
}

// ---------------------------------------------------------------------------
// Agent targets and deployments
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Scope {
    /// The user's home directory configuration of the agent.
    User,
    /// A project directory.
    Project,
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Scope::User => "user",
            Scope::Project => "project",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "user" => Some(Scope::User),
            "project" => Some(Scope::Project),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum LinkMode {
    /// Symlink (junction on Windows). The agent always sees the library version.
    Link,
    /// Independent copy. Re-sync to pick up library changes.
    Copy,
}

impl LinkMode {
    pub fn as_str(self) -> &'static str {
        match self {
            LinkMode::Link => "link",
            LinkMode::Copy => "copy",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "link" => Some(LinkMode::Link),
            "copy" => Some(LinkMode::Copy),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TargetCategory {
    /// Coding agents and IDE assistants.
    Coding,
    /// General personal assistants.
    Assistant,
}

/// Static description of an agent tool that can consume skills.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TargetSpec {
    /// Stable id, e.g. `claude-code`.
    pub id: String,
    pub name: String,
    pub category: TargetCategory,
    /// Skills directory relative to the home directory, e.g. `.claude/skills`.
    pub user_skills_dir: String,
    /// Skills directory relative to a project root, when the agent reads project skills.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_skills_dir: Option<String>,
    /// Subagent directory relative to home, when supported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_agents_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_agents_dir: Option<String>,
    /// Slash command directory relative to home, when supported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_commands_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_commands_dir: Option<String>,
    /// Directory whose presence means the tool is installed, relative to home.
    pub detect_dir: String,
    /// Character budget the agent allows for the list of skill descriptions in its prompt
    /// (Claude Code truncates the list at roughly 15 000 characters). `None` when unknown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_budget_chars: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs_url: Option<String>,
    /// User-defined target.
    #[serde(default)]
    pub custom: bool,
}

/// A skill (or agent/command) found inside a target directory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct InstalledAsset {
    pub kind: AssetKind,
    pub name: String,
    pub description: String,
    pub path: String,
    pub content_hash: String,
    pub description_tokens: u32,
    /// Library item this install corresponds to, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library_item_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deployment_id: Option<String>,
    pub state: InstallState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum InstallState {
    /// Linked to the library.
    Linked,
    /// Copied and identical to the library version.
    InSync,
    /// Copied; the library version changed since.
    Stale,
    /// Copied; the installed copy was edited locally.
    Modified,
    /// Not managed by Skill Hive.
    Unmanaged,
    /// A link whose destination no longer exists.
    Broken,
}

/// Live status of one target at one scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TargetStatus {
    pub spec: TargetSpec,
    pub installed: bool,
    pub scope: Scope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    pub skills_path: String,
    pub assets: Vec<InstalledAsset>,
    /// Sum of description tokens of installed skills: the standing context cost.
    pub context_tokens: u32,
    pub context_chars: u32,
    /// Fraction of the description budget consumed, when the budget is known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_used: Option<f64>,
    pub conflicts: ConflictReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Deployment {
    pub id: String,
    pub item_id: String,
    pub item_name: String,
    pub target_id: String,
    pub scope: Scope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    /// Absolute path of the link or copy.
    pub path: String,
    pub mode: LinkMode,
    /// Library content hash at deploy time.
    pub deployed_hash: String,
    pub state: InstallState,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeployRequest {
    pub item_ids: Vec<String>,
    pub target_id: String,
    pub scope: Scope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<LinkMode>,
    /// Replace an unmanaged asset with the same name.
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeployResult {
    pub deployed: Vec<Deployment>,
    pub failed: Vec<DeployFailure>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeployFailure {
    pub item_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProjectRef {
    pub path: String,
    pub name: String,
    pub added_at: i64,
}

// ---------------------------------------------------------------------------
// Loadouts, conflicts, recommendations
// ---------------------------------------------------------------------------

/// A named set of library items meant to be deployed together.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Loadout {
    pub id: String,
    pub name: String,
    pub description: String,
    pub item_ids: Vec<String>,
    /// Optional accent colour for the UI (hex).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LoadoutRef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LoadoutInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub item_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ApplyStrategy {
    /// Remove every Hive-managed asset not in the loadout, then deploy the loadout.
    Exclusive,
    /// Deploy the loadout on top of what is installed.
    Additive,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ApplyLoadoutRequest {
    pub loadout_id: String,
    pub target_id: String,
    pub scope: Scope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    pub strategy: ApplyStrategy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<LinkMode>,
}

/// What applying a loadout would do.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LoadoutPlan {
    pub adds: Vec<PlannedItem>,
    pub removes: Vec<PlannedItem>,
    pub keeps: Vec<PlannedItem>,
    /// Unmanaged assets that stay and may collide.
    pub unmanaged: Vec<PlannedItem>,
    pub conflicts: ConflictReport,
    pub budget: BudgetSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlannedItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    pub name: String,
    pub kind: AssetKind,
    pub description_tokens: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ApplyLoadoutResult {
    pub plan: LoadoutPlan,
    pub deployed: Vec<Deployment>,
    pub removed: Vec<String>,
    pub failed: Vec<DeployFailure>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ConflictKind {
    /// Two assets share a name: only one can be installed in a directory, and agents
    /// cannot tell them apart.
    NameCollision,
    /// Identical content under different names.
    DuplicateContent,
    /// Descriptions are near-duplicates: the agent will pick between them at random.
    DescriptionOverlap,
    /// Both claim the same trigger phrases ("use when …").
    TriggerOverlap,
    /// Both pre-approve the same tools with different instructions.
    ToolOverlap,
    /// The set exceeds the description budget of the target.
    BudgetExceeded,
    /// Instructions give opposite directives on the same topic.
    ContradictoryDirective,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ConflictSeverity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Conflict {
    pub kind: ConflictKind,
    pub severity: ConflictSeverity,
    /// Library item ids involved (empty for unmanaged assets).
    pub item_ids: Vec<String>,
    pub names: Vec<String>,
    pub detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub similarity: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shared_terms: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ConflictReport {
    pub conflicts: Vec<Conflict>,
    /// 0 (clean) … 1 (severe) summary score.
    pub score: f64,
}

impl ConflictReport {
    pub fn has_high(&self) -> bool {
        self.conflicts
            .iter()
            .any(|c| c.severity == ConflictSeverity::High)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BudgetSummary {
    pub description_tokens: u32,
    pub description_chars: u32,
    pub body_tokens: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_chars: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub used_fraction: Option<f64>,
    pub over_budget: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RecommendRequest {
    /// Natural-language description of the job to do.
    pub task: String,
    #[serde(default)]
    pub kinds: Vec<AssetKind>,
    #[serde(default)]
    pub limit: Option<u32>,
    /// Target whose description budget constrains the suggested set.
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default)]
    pub exclude_ids: Vec<String>,
    /// Ask a configured provider to re-rank and explain the top candidates.
    #[serde(default)]
    pub use_llm: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Recommendation {
    pub item: LibraryItem,
    /// Higher is better; normalised to 0…1 within one response.
    pub score: f64,
    pub reasons: Vec<String>,
    pub matched_terms: Vec<String>,
    /// Part of the suggested conflict-free set.
    pub selected: bool,
    /// Ids of other recommended items this one conflicts with.
    pub conflicts_with: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RecommendResponse {
    pub results: Vec<Recommendation>,
    /// Ids of the suggested set: best score, no high-severity conflicts, within budget.
    pub suggested_ids: Vec<String>,
    pub conflicts: ConflictReport,
    pub budget: BudgetSummary,
    /// Present when an LLM re-ranked the results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_notes: Option<String>,
}

// ---------------------------------------------------------------------------
// Providers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ProviderKind {
    /// Claude Code CLI (`claude -p`). Runs with real tools inside a sandbox directory.
    ClaudeCode,
    /// OpenAI Codex CLI (`codex exec`).
    Codex,
    /// Anthropic Messages API (no tools; the skill is injected as a system prompt).
    Anthropic,
    /// Any OpenAI-compatible chat completions endpoint (OpenAI, OpenRouter, Ollama, LM Studio…).
    OpenaiCompatible,
    /// Deterministic provider driven by a script file; used for tests and offline demos.
    Scripted,
}

impl ProviderKind {
    pub fn label(self) -> &'static str {
        match self {
            ProviderKind::ClaudeCode => "Claude Code CLI",
            ProviderKind::Codex => "Codex CLI",
            ProviderKind::Anthropic => "Anthropic API",
            ProviderKind::OpenaiCompatible => "OpenAI-compatible API",
            ProviderKind::Scripted => "Scripted (offline)",
        }
    }
    /// True when the provider executes tasks with tools (an agent) rather than only answering.
    pub fn is_agentic(self) -> bool {
        matches!(
            self,
            ProviderKind::ClaudeCode | ProviderKind::Codex | ProviderKind::Scripted
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ProviderRole {
    /// Runs eval tasks.
    Executor,
    /// Judges outputs against expectations.
    Grader,
    /// Re-ranks and explains recommendations.
    Recommender,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub kind: ProviderKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// API base URL for HTTP providers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Executable for CLI providers (defaults to `claude` / `codex`), or script path for `Scripted`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// Name of the environment variable holding the API key, when not using the secret store.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    /// True when a secret is stored for this provider in the secret store.
    #[serde(default)]
    pub has_secret: bool,
    pub roles: Vec<ProviderRole>,
    pub timeout_secs: u32,
    pub max_concurrency: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_probe: Option<ProviderProbe>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProviderInput {
    pub name: String,
    pub kind: ProviderKind,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub api_key_env: Option<String>,
    /// Secret to store; `None` leaves the stored secret unchanged.
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub roles: Vec<ProviderRole>,
    #[serde(default)]
    pub timeout_secs: Option<u32>,
    #[serde(default)]
    pub max_concurrency: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProviderProbe {
    pub ok: bool,
    pub latency_ms: u64,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub checked_at: i64,
}

/// A CLI agent detected on this machine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DetectedCli {
    pub kind: ProviderKind,
    pub command: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

// ---------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------

/// Deterministic check that can be verified without a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum CheckKind {
    /// A file exists in the sandbox (path relative to the sandbox root).
    FileExists { path: String },
    /// A file contains a substring.
    FileContains { path: String, text: String },
    /// A file matches a regular expression.
    FileMatches { path: String, pattern: String },
    /// The final assistant output contains a substring.
    OutputContains { text: String },
    /// The final assistant output matches a regular expression.
    OutputMatches { pattern: String },
    /// A JSON file has a value at a JSON pointer.
    JsonPointer {
        path: String,
        pointer: String,
        #[ts(type = "unknown")]
        equals: serde_json::Value,
    },
    /// A command exits successfully inside the sandbox.
    Command {
        command: String,
        #[serde(default)]
        args: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Check {
    pub id: String,
    /// Expectation text this check verifies; when it matches an expectation the grader is skipped for it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expectation: Option<String>,
    #[serde(flatten)]
    pub kind: CheckKind,
}

/// One task prompt with expectations (skill-creator compatible).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EvalCase {
    pub id: u32,
    pub name: String,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_output: Option<String>,
    /// Input files relative to the skill root, copied into the sandbox.
    #[serde(default)]
    pub files: Vec<String>,
    pub expectations: Vec<String>,
    #[serde(default)]
    pub checks: Vec<Check>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TriggerCase {
    pub query: String,
    pub should_trigger: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EvalSuite {
    pub item_id: String,
    pub skill_name: String,
    pub cases: Vec<EvalCase>,
    pub trigger_cases: Vec<TriggerCase>,
    /// Directory holding `evals.json` and friends (inside the skill).
    pub path: String,
    /// True when the suite was read from disk rather than generated empty.
    pub exists: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum ArmKind {
    /// The library version of the skill is available.
    WithSkill,
    /// No skill is available.
    WithoutSkill,
    /// Another skill directory (an alternative wording, an older snapshot, a competitor).
    Variant { path: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ArmSpec {
    pub label: String,
    #[serde(flatten)]
    pub kind: ArmKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RunConfig {
    pub item_id: String,
    pub arms: Vec<ArmSpec>,
    /// Subset of case ids; empty means all.
    #[serde(default)]
    pub case_ids: Vec<u32>,
    pub repeats: u32,
    pub concurrency: u32,
    pub executor_provider_id: String,
    /// Grader provider; when absent only deterministic checks are evaluated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grader_provider_id: Option<String>,
    pub timeout_secs: u32,
    #[serde(default)]
    pub include_trigger: bool,
    /// Which arm is the baseline for deltas; defaults to the first non-`WithSkill` arm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum RunStatus {
    Queued,
    Running,
    Grading,
    Done,
    Failed,
    Cancelled,
}

impl RunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            RunStatus::Queued => "queued",
            RunStatus::Running => "running",
            RunStatus::Grading => "grading",
            RunStatus::Done => "done",
            RunStatus::Failed => "failed",
            RunStatus::Cancelled => "cancelled",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "queued" => RunStatus::Queued,
            "running" => RunStatus::Running,
            "grading" => RunStatus::Grading,
            "done" => RunStatus::Done,
            "cancelled" => RunStatus::Cancelled,
            _ => RunStatus::Failed,
        }
    }
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            RunStatus::Done | RunStatus::Failed | RunStatus::Cancelled
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RunProgress {
    pub total: u32,
    pub done: u32,
    pub failed: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EvalRun {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    pub skill_name: String,
    /// Content hash of the skill as tested.
    pub skill_hash: String,
    pub config: RunConfig,
    pub status: RunStatus,
    pub progress: RunProgress,
    pub workspace: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<Benchmark>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub created_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ExpectationSource {
    Check,
    Grader,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ExpectationResult {
    pub text: String,
    pub passed: bool,
    pub evidence: String,
    pub source: ExpectationSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum CaseStatus {
    Queued,
    Running,
    Executed,
    Graded,
    Failed,
    Timeout,
    Cancelled,
}

/// Result of one (case, arm, repeat) execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CaseResult {
    pub id: String,
    pub run_id: String,
    pub case_id: u32,
    pub case_name: String,
    pub arm: String,
    pub repeat: u32,
    pub status: CaseStatus,
    pub expectations: Vec<ExpectationResult>,
    pub passed: u32,
    pub failed: u32,
    pub total: u32,
    pub pass_rate: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub duration_ms: u64,
    pub tool_calls: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    /// First characters of the final answer.
    pub output_excerpt: String,
    pub sandbox_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcript_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grader_notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TriggerResult {
    pub query: String,
    pub should_trigger: bool,
    pub triggered: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chosen: Option<String>,
    pub correct: bool,
    pub raw: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Stats {
    pub n: u32,
    pub mean: f64,
    pub stddev: f64,
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ArmSummary {
    pub label: String,
    pub runs: u32,
    pub errors: u32,
    pub pass_rate: Stats,
    pub tokens: Stats,
    pub duration_ms: Stats,
    pub tool_calls: Stats,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Verdict {
    HelpsALot,
    Helps,
    Neutral,
    Hurts,
    HurtsALot,
    Inconclusive,
}

impl Verdict {
    pub fn label(self) -> &'static str {
        match self {
            Verdict::HelpsALot => "Helps a lot",
            Verdict::Helps => "Helps",
            Verdict::Neutral => "Neutral",
            Verdict::Hurts => "Hurts",
            Verdict::HurtsALot => "Hurts a lot",
            Verdict::Inconclusive => "Inconclusive",
        }
    }
}

/// Comparison of one arm against the baseline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ArmDelta {
    pub arm: String,
    pub baseline: String,
    pub pass_rate_delta: f64,
    /// 95% bootstrap confidence interval of the pass-rate delta.
    pub ci_low: f64,
    pub ci_high: f64,
    /// Welch t-test p-value (two sided); `None` when there is too little data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_value: Option<f64>,
    pub tokens_delta: f64,
    pub duration_delta_ms: f64,
    pub verdict: Verdict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ExpectationPattern {
    /// Passes in every arm: does not measure the skill.
    AlwaysPasses,
    /// Fails in every arm: broken expectation or beyond the model.
    AlwaysFails,
    /// Passes only with the skill: clear value.
    OnlyWithSkill,
    /// Passes only in the baseline: the skill may be hurting.
    OnlyBaseline,
    /// Inconsistent across repeats.
    Flaky,
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ExpectationRow {
    pub case_id: u32,
    pub case_name: String,
    pub text: String,
    /// Pass fraction per arm label.
    pub by_arm: BTreeMap<String, f64>,
    pub pattern: ExpectationPattern,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TriggerSummary {
    pub total: u32,
    pub correct: u32,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    pub results: Vec<TriggerResult>,
}

/// Aggregated, comparative result of a run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Benchmark {
    pub skill_name: String,
    pub skill_hash: String,
    pub executor: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grader: Option<String>,
    pub arms: Vec<ArmSummary>,
    pub baseline: String,
    pub deltas: Vec<ArmDelta>,
    pub expectations: Vec<ExpectationRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<TriggerSummary>,
    pub verdict: Verdict,
    /// Analyst notes: what the aggregates hide.
    pub notes: Vec<String>,
    pub generated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HistoryPoint {
    pub run_id: String,
    pub finished_at: i64,
    pub skill_hash: String,
    pub executor: String,
    pub verdict: Verdict,
    pub with_skill_pass_rate: f64,
    pub baseline_pass_rate: f64,
    pub delta: f64,
    pub with_skill_tokens: f64,
    pub baseline_tokens: f64,
}

/// Detail of a run: the run row plus every case result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EvalRunDetail {
    pub run: EvalRun,
    pub results: Vec<CaseResult>,
}

// ---------------------------------------------------------------------------
// Settings, events, dashboard
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Settings {
    /// Resolved Hive home directory (read-only, informational).
    pub home: String,
    pub default_link_mode: LinkMode,
    /// Budget used when a target does not declare one.
    pub default_description_budget_chars: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_executor_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_grader_id: Option<String>,
    /// Extra directories to include in local scans.
    #[serde(default)]
    pub extra_scan_dirs: Vec<String>,
    /// GitHub token is present in the secret store (raises API rate limits, private repos).
    pub github_token_set: bool,
    pub theme: String,
    /// Similarity threshold (0…1) above which two descriptions count as overlapping.
    pub overlap_threshold: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SettingsPatch {
    #[serde(default)]
    pub default_link_mode: Option<LinkMode>,
    #[serde(default)]
    pub default_description_budget_chars: Option<u32>,
    #[serde(default)]
    pub default_executor_id: Option<Option<String>>,
    #[serde(default)]
    pub default_grader_id: Option<Option<String>>,
    #[serde(default)]
    pub extra_scan_dirs: Option<Vec<String>>,
    /// Set (Some(token)) or clear (Some("")) the GitHub token.
    #[serde(default)]
    pub github_token: Option<String>,
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub overlap_threshold: Option<f64>,
}

/// Progress and lifecycle events pushed to the UI / CLI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
pub enum HiveEvent {
    Progress {
        /// Operation id (scan, import, fetch, run…).
        op: String,
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        done: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        total: Option<u32>,
    },
    RunUpdated {
        run_id: String,
        status: RunStatus,
        progress: RunProgress,
    },
    CaseFinished {
        run_id: String,
        result: CaseResult,
    },
    LibraryChanged,
    DeploymentsChanged,
    Log {
        level: String,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Dashboard {
    pub stats: LibraryStats,
    pub targets: Vec<TargetStatus>,
    pub recent_runs: Vec<EvalRun>,
    pub recent_items: Vec<LibraryItem>,
    pub loadouts: Vec<Loadout>,
    pub providers: Vec<ProviderConfig>,
    pub detected_clis: Vec<DetectedCli>,
}

/// Health report from `hive doctor`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DoctorReport {
    pub home: String,
    pub db_ok: bool,
    pub library_items: u32,
    pub orphan_dirs: Vec<String>,
    pub missing_dirs: Vec<String>,
    pub broken_deployments: Vec<Deployment>,
    pub git_available: bool,
    pub detected_clis: Vec<DetectedCli>,
    pub notes: Vec<String>,
}

/// A repository returned by an online search (GitHub).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RepoSearchResult {
    /// `owner/repo`
    pub full_name: String,
    pub description: String,
    pub url: String,
    pub stars: u32,
    pub topics: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}
