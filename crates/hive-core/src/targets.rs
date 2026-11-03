//! Agent targets: the tools that consume skills, and where they look for them.
//!
//! The built-in table covers the agents that read the Agent Skills format today. Paths are
//! relative to the home directory (user scope) or a project root (project scope). Users can add
//! custom targets and override the user-scope skills directory of any target in settings.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::model::{AssetKind, Scope, TargetCategory, TargetSpec};

/// Claude Code lists every skill's name and description in its prompt and truncates the list
/// at about this many characters. Other agents are assumed to behave similarly until known.
pub const CLAUDE_CODE_DESCRIPTION_BUDGET_CHARS: u32 = 15_000;

fn spec(
    id: &str,
    name: &str,
    category: TargetCategory,
    user_skills_dir: &str,
    project_skills_dir: Option<&str>,
    detect_dir: &str,
    docs_url: Option<&str>,
) -> TargetSpec {
    TargetSpec {
        id: id.to_string(),
        name: name.to_string(),
        category,
        user_skills_dir: user_skills_dir.to_string(),
        project_skills_dir: project_skills_dir.map(|s| s.to_string()),
        user_agents_dir: None,
        project_agents_dir: None,
        user_commands_dir: None,
        project_commands_dir: None,
        detect_dir: detect_dir.to_string(),
        description_budget_chars: Some(CLAUDE_CODE_DESCRIPTION_BUDGET_CHARS),
        docs_url: docs_url.map(|s| s.to_string()),
        custom: false,
    }
}

/// The built-in targets, most common first.
pub fn builtin_targets() -> Vec<TargetSpec> {
    use TargetCategory::*;
    let mut claude = spec(
        "claude-code",
        "Claude Code",
        Coding,
        ".claude/skills",
        Some(".claude/skills"),
        ".claude",
        Some("https://docs.claude.com/en/docs/claude-code/skills"),
    );
    claude.user_agents_dir = Some(".claude/agents".into());
    claude.project_agents_dir = Some(".claude/agents".into());
    claude.user_commands_dir = Some(".claude/commands".into());
    claude.project_commands_dir = Some(".claude/commands".into());

    vec![
        claude,
        spec(
            "codex",
            "Codex CLI",
            Coding,
            ".codex/skills",
            Some(".codex/skills"),
            ".codex",
            Some("https://developers.openai.com/codex/skills"),
        ),
        spec(
            "cursor",
            "Cursor",
            Coding,
            ".cursor/skills",
            Some(".cursor/skills"),
            ".cursor",
            Some("https://cursor.com/docs/context/skills"),
        ),
        spec(
            "gemini-cli",
            "Gemini CLI",
            Coding,
            ".gemini/skills",
            Some(".gemini/skills"),
            ".gemini",
            Some("https://geminicli.com/docs"),
        ),
        spec(
            "github-copilot",
            "GitHub Copilot",
            Coding,
            ".copilot/skills",
            Some(".github/skills"),
            ".copilot",
            Some("https://docs.github.com/en/copilot"),
        ),
        spec(
            "windsurf",
            "Windsurf",
            Coding,
            ".codeium/windsurf/skills",
            Some(".windsurf/skills"),
            ".codeium/windsurf",
            Some("https://docs.windsurf.com"),
        ),
        spec(
            "opencode",
            "OpenCode",
            Coding,
            ".config/opencode/skills",
            Some(".opencode/skills"),
            ".config/opencode",
            Some("https://opencode.ai/docs/skills"),
        ),
        spec(
            "amp",
            "Amp",
            Coding,
            ".config/amp/skills",
            Some(".agents/skills"),
            ".config/amp",
            Some("https://ampcode.com/manual#skills"),
        ),
        spec(
            "goose",
            "Goose",
            Coding,
            ".config/goose/skills",
            Some(".goose/skills"),
            ".config/goose",
            Some("https://block.github.io/goose/"),
        ),
        spec(
            "kiro",
            "Kiro",
            Coding,
            ".kiro/skills",
            Some(".kiro/skills"),
            ".kiro",
            Some("https://kiro.dev/docs"),
        ),
        spec(
            "cline",
            "Cline",
            Coding,
            ".cline/skills",
            Some(".cline/skills"),
            ".cline",
            Some("https://docs.cline.bot"),
        ),
        spec(
            "roo-code",
            "Roo Code",
            Coding,
            ".roo/skills",
            Some(".roo/skills"),
            ".roo",
            Some("https://docs.roocode.com"),
        ),
        spec(
            "antigravity",
            "Antigravity",
            Coding,
            ".gemini/antigravity/skills",
            Some(".agent/skills"),
            ".gemini/antigravity",
            None,
        ),
        spec(
            "agents-shared",
            "Shared (.agents)",
            Coding,
            ".agents/skills",
            Some(".agents/skills"),
            ".agents",
            Some("https://agentskills.io"),
        ),
        spec(
            "openclaw",
            "OpenClaw",
            Assistant,
            ".openclaw/skills",
            None,
            ".openclaw",
            None,
        ),
        spec(
            "hermes",
            "Hermes Agent",
            Assistant,
            ".hermes/skills",
            None,
            ".hermes",
            None,
        ),
    ]
}

/// Resolves target directories against a home directory and user overrides.
#[derive(Debug, Clone)]
pub struct TargetResolver {
    home: PathBuf,
    specs: Vec<TargetSpec>,
    /// `target_id -> absolute user-scope skills directory`
    overrides: BTreeMap<String, PathBuf>,
}

impl TargetResolver {
    pub fn new(
        home: PathBuf,
        custom: Vec<TargetSpec>,
        overrides: BTreeMap<String, PathBuf>,
    ) -> Self {
        let mut specs = builtin_targets();
        for mut c in custom {
            c.custom = true;
            specs.retain(|s| s.id != c.id);
            specs.push(c);
        }
        Self {
            home,
            specs,
            overrides,
        }
    }

    pub fn with_home(home: PathBuf) -> Self {
        Self::new(home, Vec::new(), BTreeMap::new())
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn specs(&self) -> &[TargetSpec] {
        &self.specs
    }

    pub fn get(&self, id: &str) -> Option<&TargetSpec> {
        self.specs.iter().find(|s| s.id == id)
    }

    /// Directory for `kind` at `scope`. `None` when the target has no such directory.
    pub fn asset_dir(
        &self,
        spec: &TargetSpec,
        kind: AssetKind,
        scope: Scope,
        project: Option<&Path>,
    ) -> Option<PathBuf> {
        match (scope, kind) {
            (Scope::User, AssetKind::Skill) => Some(
                self.overrides
                    .get(&spec.id)
                    .cloned()
                    .unwrap_or_else(|| self.home.join(&spec.user_skills_dir)),
            ),
            (Scope::User, AssetKind::Agent) => {
                spec.user_agents_dir.as_ref().map(|d| self.home.join(d))
            }
            (Scope::User, AssetKind::Command) => {
                spec.user_commands_dir.as_ref().map(|d| self.home.join(d))
            }
            (Scope::Project, AssetKind::Skill) => {
                let root = project?;
                spec.project_skills_dir.as_ref().map(|d| root.join(d))
            }
            (Scope::Project, AssetKind::Agent) => {
                let root = project?;
                spec.project_agents_dir.as_ref().map(|d| root.join(d))
            }
            (Scope::Project, AssetKind::Command) => {
                let root = project?;
                spec.project_commands_dir.as_ref().map(|d| root.join(d))
            }
        }
    }

    pub fn skills_dir(
        &self,
        spec: &TargetSpec,
        scope: Scope,
        project: Option<&Path>,
    ) -> Option<PathBuf> {
        self.asset_dir(spec, AssetKind::Skill, scope, project)
    }

    /// A target counts as installed when its detect directory exists (or it has an override).
    pub fn is_installed(&self, spec: &TargetSpec) -> bool {
        spec.custom
            || self.overrides.contains_key(&spec.id)
            || self.home.join(&spec.detect_dir).exists()
    }

    /// Every user-scope directory worth scanning for assets, with the owning target id.
    pub fn user_scan_roots(&self) -> Vec<(String, AssetKind, PathBuf)> {
        let mut out = Vec::new();
        for spec in &self.specs {
            for kind in [AssetKind::Skill, AssetKind::Agent, AssetKind::Command] {
                if let Some(dir) = self.asset_dir(spec, kind, Scope::User, None) {
                    if dir.is_dir()
                        && !out
                            .iter()
                            .any(|(_, k, d): &(String, AssetKind, PathBuf)| *k == kind && d == &dir)
                    {
                        out.push((spec.id.clone(), kind, dir));
                    }
                }
            }
        }
        out
    }

    /// Every project-scope directory worth scanning under `project`.
    pub fn project_scan_roots(&self, project: &Path) -> Vec<(String, AssetKind, PathBuf)> {
        let mut out = Vec::new();
        for spec in &self.specs {
            for kind in [AssetKind::Skill, AssetKind::Agent, AssetKind::Command] {
                if let Some(dir) = self.asset_dir(spec, kind, Scope::Project, Some(project)) {
                    if dir.is_dir()
                        && !out
                            .iter()
                            .any(|(_, k, d): &(String, AssetKind, PathBuf)| *k == kind && d == &dir)
                    {
                        out.push((spec.id.clone(), kind, dir));
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_ids_are_unique_and_kebab() {
        let specs = builtin_targets();
        let mut ids: Vec<&str> = specs.iter().map(|s| s.id.as_str()).collect();
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n);
        assert!(ids.iter().all(|id| crate::skill::is_valid_name(id)));
    }

    #[test]
    fn resolves_paths_and_overrides() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().to_path_buf();
        let resolver = TargetResolver::with_home(home.clone());
        let claude = resolver.get("claude-code").unwrap();
        assert_eq!(
            resolver.skills_dir(claude, Scope::User, None).unwrap(),
            home.join(".claude/skills")
        );
        assert_eq!(
            resolver
                .skills_dir(claude, Scope::Project, Some(Path::new("/p")))
                .unwrap(),
            PathBuf::from("/p/.claude/skills")
        );
        assert!(
            resolver
                .asset_dir(claude, AssetKind::Agent, Scope::User, None)
                .is_some()
        );
        let codex = resolver.get("codex").unwrap();
        assert!(
            resolver
                .asset_dir(codex, AssetKind::Agent, Scope::User, None)
                .is_none()
        );
        assert!(!resolver.is_installed(claude));
        std::fs::create_dir_all(home.join(".claude/skills")).unwrap();
        assert!(resolver.is_installed(claude));
        let roots = resolver.user_scan_roots();
        assert!(
            roots
                .iter()
                .any(|(id, k, _)| id == "claude-code" && *k == AssetKind::Skill)
        );

        let mut overrides = BTreeMap::new();
        overrides.insert("codex".to_string(), home.join("custom-codex"));
        let resolver = TargetResolver::new(home.clone(), vec![], overrides);
        let codex = resolver.get("codex").unwrap();
        assert_eq!(
            resolver.skills_dir(codex, Scope::User, None).unwrap(),
            home.join("custom-codex")
        );
        assert!(resolver.is_installed(codex));
    }

    #[test]
    fn custom_targets_replace_builtins() {
        let custom = TargetSpec {
            id: "cursor".into(),
            name: "Cursor (custom)".into(),
            category: TargetCategory::Coding,
            user_skills_dir: "x/skills".into(),
            project_skills_dir: None,
            user_agents_dir: None,
            project_agents_dir: None,
            user_commands_dir: None,
            project_commands_dir: None,
            detect_dir: "x".into(),
            description_budget_chars: None,
            docs_url: None,
            custom: false,
        };
        let resolver = TargetResolver::new(PathBuf::from("/h"), vec![custom], BTreeMap::new());
        let cursor = resolver.get("cursor").unwrap();
        assert!(cursor.custom);
        assert_eq!(
            resolver.specs().iter().filter(|s| s.id == "cursor").count(),
            1
        );
    }
}
