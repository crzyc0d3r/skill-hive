//! Local discovery: find skills, agents and commands already on this machine.
//!
//! Scans the user-scope directories of every known agent target, registered projects, and any
//! extra directories the user configured. Nested skill trees (a `skills/` folder with categories)
//! are walked a few levels deep; `node_modules`, `.git` and friends are skipped.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use walkdir::WalkDir;

use crate::library::{Library, detect_kind};
use crate::model::{AssetKind, Candidate, ScanReport, SourceKind, SourceRef};
use crate::skill;
use crate::targets::TargetResolver;

const MAX_DEPTH: usize = 4;
const SKIP_DIRS: [&str; 8] = [
    ".git",
    "node_modules",
    "__pycache__",
    "target",
    "dist",
    "build",
    ".venv",
    "venv",
];

/// A directory to scan plus the target it belongs to (if any).
#[derive(Debug, Clone)]
pub struct ScanRoot {
    pub path: PathBuf,
    pub target_id: Option<String>,
    /// Only look for this kind; `None` means skills (dirs) and markdown assets (files).
    pub kind: Option<AssetKind>,
}

/// Collect skill directories under `root` (depth-limited), including `root` itself.
pub fn find_skill_dirs(root: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if skill::is_skill_dir(root) {
        out.push(root.to_path_buf());
        return out;
    }
    let walker = WalkDir::new(root)
        .min_depth(1)
        .max_depth(max_depth)
        .follow_links(true)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| {
            !(e.file_type().is_dir()
                && SKIP_DIRS.contains(&e.file_name().to_string_lossy().as_ref()))
        });
    let mut seen_parents: HashSet<PathBuf> = HashSet::new();
    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_dir() {
            continue;
        }
        let path = entry.path();
        // Do not descend into a skill looking for nested skills.
        if seen_parents.iter().any(|p| path.starts_with(p)) {
            continue;
        }
        if skill::is_skill_dir(path) {
            out.push(path.to_path_buf());
            seen_parents.insert(path.to_path_buf());
        }
    }
    out
}

/// Markdown files directly inside `root` (agents and commands are flat files).
pub fn find_markdown_assets(root: &Path, kind: AssetKind) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("md"))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| !n.eq_ignore_ascii_case("README.md"))
                .unwrap_or(true)
        })
        .filter(|p| match kind {
            AssetKind::Agent => detect_kind(p) == Some(AssetKind::Agent),
            AssetKind::Command => detect_kind(p)
                .map(|k| k != AssetKind::Skill)
                .unwrap_or(false),
            AssetKind::Skill => false,
        })
        .collect();
    out.sort();
    out
}

/// Scan the given roots and classify everything found against the library.
pub fn scan(lib: &Library<'_>, roots: &[ScanRoot]) -> ScanReport {
    let started = Instant::now();
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut errors = Vec::new();
    let mut roots_scanned = Vec::new();

    for root in roots {
        if !root.path.exists() {
            continue;
        }
        roots_scanned.push(root.path.to_string_lossy().to_string());
        let mut source = SourceRef::local(root.path.to_string_lossy());
        if let Some(target) = &root.target_id {
            source.kind = SourceKind::AgentDir;
            source.target_id = Some(target.clone());
        }
        let mut found: Vec<(AssetKind, PathBuf)> = Vec::new();
        match root.kind {
            Some(AssetKind::Skill) | None => {
                for dir in find_skill_dirs(&root.path, MAX_DEPTH) {
                    found.push((AssetKind::Skill, dir));
                }
            }
            Some(kind) => {
                for file in find_markdown_assets(&root.path, kind) {
                    found.push((kind, file));
                }
            }
        }
        if root.kind.is_none() && root.target_id.is_none() {
            // Free-form directory: also pick up loose agent/command files at the top level.
            for file in find_markdown_assets(&root.path, AssetKind::Agent) {
                found.push((AssetKind::Agent, file));
            }
        }
        for (kind, path) in found {
            let key = dunce::canonicalize(&path).unwrap_or_else(|_| path.clone());
            if !seen.insert(key) {
                continue;
            }
            let mut src = source.clone();
            src.locator = path.to_string_lossy().to_string();
            let candidate = lib.candidate_for(kind, &path, src);
            if candidate.content_hash.is_empty()
                && candidate
                    .validation
                    .findings
                    .iter()
                    .any(|f| f.code == "parse.failed")
            {
                errors.push(format!(
                    "{}: {}",
                    path.display(),
                    candidate.validation.findings[0].message
                ));
            }
            candidates.push(candidate);
        }
    }
    candidates.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.path.cmp(&b.path))
    });
    ScanReport {
        candidates,
        roots_scanned,
        errors,
        duration_ms: started.elapsed().as_millis() as u64,
    }
}

/// Default roots: every installed target's user directories, registered projects, extra dirs.
pub fn default_roots(
    resolver: &TargetResolver,
    projects: &[PathBuf],
    extra: &[PathBuf],
) -> Vec<ScanRoot> {
    let mut roots = Vec::new();
    for (target_id, kind, dir) in resolver.user_scan_roots() {
        roots.push(ScanRoot {
            path: dir,
            target_id: Some(target_id),
            kind: Some(kind),
        });
    }
    for project in projects {
        for (target_id, kind, dir) in resolver.project_scan_roots(project) {
            roots.push(ScanRoot {
                path: dir,
                target_id: Some(target_id),
                kind: Some(kind),
            });
        }
    }
    for dir in extra {
        roots.push(ScanRoot {
            path: dir.clone(),
            target_id: None,
            kind: None,
        });
    }
    roots
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::library::test_support::write_skill;
    use crate::model::CandidateStatus;
    use crate::paths::HivePaths;

    #[test]
    fn finds_nested_skills_and_skips_noise() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write_skill(root, "top", "Top level skill for testing scans", "x");
        write_skill(
            &root.join("category/sub"),
            "deep",
            "Deep skill for testing scans",
            "x",
        );
        write_skill(
            &root.join("node_modules/pkg"),
            "ignored",
            "Should be skipped by the scanner",
            "x",
        );
        // nested skill inside a skill is not a separate skill
        write_skill(
            &root.join("top/inner"),
            "inner",
            "Nested inside top, should be ignored",
            "x",
        );
        let dirs = find_skill_dirs(root, 4);
        let mut names: Vec<String> = dirs
            .iter()
            .map(|d| d.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        names.sort();
        assert_eq!(names, vec!["deep", "top"]);
    }

    #[test]
    fn scan_classifies_against_library() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = HivePaths::new(tmp.path().join("hive"));
        paths.ensure().unwrap();
        let db = Database::open_memory().unwrap();
        let lib = Library::new(&db, &paths);
        let home = tmp.path().join("home");
        let claude_skills = home.join(".claude/skills");
        write_skill(
            &claude_skills,
            "alpha",
            "Alpha skill for testing discovery",
            "a",
        );
        let beta = write_skill(
            &claude_skills,
            "beta",
            "Beta skill for testing discovery",
            "b",
        );
        std::fs::create_dir_all(home.join(".claude/agents")).unwrap();
        std::fs::write(
            home.join(".claude/agents/reviewer.md"),
            "---\nname: reviewer\ndescription: Reviews code carefully\n---\nbody",
        )
        .unwrap();
        lib.import_path(&beta, SourceRef::local("x"), &[], false)
            .unwrap();

        let resolver = TargetResolver::with_home(home.clone());
        let roots = default_roots(&resolver, &[], &[]);
        let report = scan(&lib, &roots);
        assert_eq!(report.candidates.len(), 3, "{report:?}");
        let alpha = report
            .candidates
            .iter()
            .find(|c| c.name == "alpha")
            .unwrap();
        assert_eq!(alpha.status, CandidateStatus::New);
        assert_eq!(alpha.source.kind, SourceKind::AgentDir);
        assert_eq!(alpha.source.target_id.as_deref(), Some("claude-code"));
        let beta = report.candidates.iter().find(|c| c.name == "beta").unwrap();
        assert_eq!(beta.status, CandidateStatus::InLibrary);
        let reviewer = report
            .candidates
            .iter()
            .find(|c| c.name == "reviewer")
            .unwrap();
        assert_eq!(reviewer.kind, AssetKind::Agent);
        assert!(report.errors.is_empty());
    }

    #[test]
    fn extra_dirs_pick_up_loose_assets() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = HivePaths::new(tmp.path().join("hive"));
        paths.ensure().unwrap();
        let db = Database::open_memory().unwrap();
        let lib = Library::new(&db, &paths);
        let extra = tmp.path().join("downloads");
        write_skill(&extra, "gamma", "Gamma skill for testing extra dirs", "g");
        std::fs::write(extra.join("broken.md"), "---\nname: x\n").unwrap();
        std::fs::write(
            extra.join("helper.md"),
            "---\nname: helper\ndescription: A helper agent\n---\nbody",
        )
        .unwrap();
        let roots = vec![ScanRoot {
            path: extra.clone(),
            target_id: None,
            kind: None,
        }];
        let report = scan(&lib, &roots);
        let names: Vec<&str> = report.candidates.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"gamma"));
        assert!(names.contains(&"helper"));
        assert!(!names.contains(&"broken"));
    }
}
