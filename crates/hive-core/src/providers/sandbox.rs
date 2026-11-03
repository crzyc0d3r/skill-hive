//! Sandbox helpers shared by providers: install a skill into the working directory, keep a
//! transcript under `.hive/`, and work out which files an execution created.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::{HiveError, IoContext, Result};

/// Hive's private directory inside a sandbox (transcripts, bookkeeping).
pub const HIVE_DIR: &str = ".hive";
/// Transcript file name inside [`HIVE_DIR`].
pub const TRANSCRIPT_FILE: &str = "transcript.jsonl";
/// Directories under the sandbox that Hive or the agent tooling manage. They are never
/// reported as created files and never shown to the grader.
pub const MANAGED_DIRS: [&str; 7] = [
    HIVE_DIR,
    ".claude",
    ".codex",
    ".skills",
    ".git",
    "node_modules",
    "__pycache__",
];
/// Directories of a skill that are not installed into a sandbox: the eval suite must not leak
/// to the agent under test.
pub const SKILL_EXCLUDED_DIRS: [&str; 3] = ["evals", ".git", ".hive"];

/// `<cwd>/.hive`, created on demand.
pub fn hive_dir(cwd: &Path) -> Result<PathBuf> {
    let dir = cwd.join(HIVE_DIR);
    std::fs::create_dir_all(&dir).at(&dir)?;
    Ok(dir)
}

/// Where a provider writes its transcript for this sandbox.
pub fn transcript_path(cwd: &Path) -> PathBuf {
    cwd.join(HIVE_DIR).join(TRANSCRIPT_FILE)
}

/// Open the transcript file for appending (truncating any previous content).
pub fn open_transcript(cwd: &Path) -> Result<(PathBuf, std::fs::File)> {
    hive_dir(cwd)?;
    let path = transcript_path(cwd);
    let file = std::fs::File::create(&path).at(&path)?;
    Ok((path, file))
}

/// Write a whole transcript as JSON lines.
pub fn write_transcript(cwd: &Path, lines: &[serde_json::Value]) -> Result<PathBuf> {
    let (path, mut file) = open_transcript(cwd)?;
    for line in lines {
        serde_json::to_writer(&mut file, line)?;
        file.write_all(b"\n").at(&path)?;
    }
    Ok(path)
}

/// Relative paths (with `/` separators) of every file under `cwd`, excluding [`MANAGED_DIRS`].
pub fn snapshot(cwd: &Path) -> BTreeSet<String> {
    walkdir::WalkDir::new(cwd)
        .min_depth(1)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            !(e.file_type().is_dir()
                && MANAGED_DIRS.contains(&e.file_name().to_string_lossy().as_ref()))
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            e.path()
                .strip_prefix(cwd)
                .unwrap_or(e.path())
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect()
}

/// Files present now that were not in `before`, sorted.
pub fn files_created(cwd: &Path, before: &BTreeSet<String>) -> Vec<String> {
    snapshot(cwd).difference(before).cloned().collect()
}

/// A skill's name: the frontmatter `name`, else the directory name.
pub fn skill_name(skill_dir: &Path) -> String {
    crate::skill::read_skill_dir(skill_dir)
        .ok()
        .map(|d| d.name)
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| {
            skill_dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "skill".into())
        })
}

fn copy_skill_filtered(src: &Path, dst: &Path, depth: usize) -> Result<()> {
    std::fs::create_dir_all(dst).at(dst)?;
    for entry in std::fs::read_dir(src).at(src)? {
        let entry = entry.at(src)?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        let from = entry.path();
        let to = dst.join(&name);
        let meta = std::fs::metadata(&from).at(&from)?;
        if meta.is_dir() {
            if depth == 0 && SKILL_EXCLUDED_DIRS.contains(&name_str.as_ref()) {
                continue;
            }
            if name_str == ".git" {
                continue;
            }
            copy_skill_filtered(&from, &to, depth + 1)?;
        } else {
            std::fs::copy(&from, &to).at(&from)?;
        }
    }
    Ok(())
}

/// Copy a skill into `<root>/<name>` (replacing any previous copy), skipping its `evals/`
/// directory. Returns the skill name and the installed path.
pub fn install_skill(skill_dir: &Path, root: &Path) -> Result<(String, PathBuf)> {
    if !crate::skill::is_skill_dir(skill_dir) {
        return Err(HiveError::invalid(format!(
            "{} is not a skill directory",
            skill_dir.display()
        )));
    }
    let name = skill_name(skill_dir);
    let dest = root.join(&name);
    crate::util::remove_path(&dest)?;
    copy_skill_filtered(skill_dir, &dest, 0)?;
    Ok((name, dest))
}

/// The sentence prepended to a task prompt so the agent knows where the skill is. Mirrors the
/// skill-creator evaluation loop.
pub fn skill_notice(rel_path: &str) -> String {
    format!(
        "A skill is available at {rel_path}. Read its SKILL.md first and follow it where relevant.\n\n"
    )
}

/// System prompt for API providers: the skill's `SKILL.md` body with a short framing.
pub fn skill_system_prompt(skill_dir: &Path) -> Result<String> {
    let doc = crate::skill::read_skill_dir(skill_dir)?;
    Ok(format!(
        "You have access to the following skill. Follow its instructions wherever they apply to the task. Files bundled with the skill are not available in this conversation; rely on the instructions below.\n\n# Skill: {}\n\n{}\n\n{}",
        doc.name,
        doc.description,
        doc.body.trim()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::test_support::write_skill;

    #[test]
    fn snapshot_and_created_files_ignore_managed_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let cwd = tmp.path();
        std::fs::write(cwd.join("input.txt"), "in").unwrap();
        let before = snapshot(cwd);
        assert_eq!(before.len(), 1);
        std::fs::create_dir_all(cwd.join("out/sub")).unwrap();
        std::fs::write(cwd.join("out/sub/a.md"), "a").unwrap();
        std::fs::create_dir_all(cwd.join(".claude/skills/x")).unwrap();
        std::fs::write(cwd.join(".claude/skills/x/SKILL.md"), "s").unwrap();
        write_transcript(cwd, &[serde_json::json!({"type": "x"})]).unwrap();
        assert_eq!(files_created(cwd, &before), vec!["out/sub/a.md"]);
        assert!(transcript_path(cwd).is_file());
    }

    #[test]
    fn install_skips_evals_and_names_from_frontmatter() {
        let tmp = tempfile::tempdir().unwrap();
        let skill = write_skill(
            tmp.path(),
            "pdf-tools",
            "Extract tables from PDFs when asked about a PDF",
            "# Body",
        );
        std::fs::create_dir_all(skill.join("evals")).unwrap();
        std::fs::write(skill.join("evals/evals.json"), "{}").unwrap();
        std::fs::create_dir_all(skill.join("scripts")).unwrap();
        std::fs::write(skill.join("scripts/run.py"), "print(1)").unwrap();
        let root = tmp.path().join("sandbox/.claude/skills");
        let (name, dest) = install_skill(&skill, &root).unwrap();
        assert_eq!(name, "pdf-tools");
        assert!(dest.join("SKILL.md").is_file());
        assert!(dest.join("scripts/run.py").is_file());
        assert!(!dest.join("evals").exists());
        // Re-installing replaces the previous copy.
        std::fs::write(dest.join("stale.txt"), "x").unwrap();
        install_skill(&skill, &root).unwrap();
        assert!(!dest.join("stale.txt").exists());
        assert!(install_skill(tmp.path(), &root).is_err());
        let sys = skill_system_prompt(&skill).unwrap();
        assert!(sys.contains("# Skill: pdf-tools"));
        assert!(sys.contains("# Body"));
        assert!(skill_notice("./.claude/skills/pdf-tools").starts_with("A skill is available at"));
    }
}
