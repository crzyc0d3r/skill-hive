//! Where Hive keeps its data.
//!
//! Layout under the Hive home (default `~/.skill-hive`, override with `HIVE_HOME`):
//!
//! ```text
//! hive.db              SQLite index (rebuildable from the library directory)
//! library/skills/      one directory per skill
//! library/agents/      one markdown file per subagent definition
//! library/commands/    one markdown file per slash command
//! library/meta/        provenance sidecars, one JSON file per library item
//! cache/git/           shallow clones of remote sources
//! workspaces/          evaluation sandboxes and reports
//! logs/
//! secrets.json         only when the OS keychain is unavailable (0600)
//! ```

use std::path::{Path, PathBuf};

use crate::error::{IoContext, Result};

pub const HOME_ENV: &str = "HIVE_HOME";
pub const DEFAULT_DIR_NAME: &str = ".skill-hive";

#[derive(Debug, Clone)]
pub struct HivePaths {
    pub home: PathBuf,
}

impl HivePaths {
    /// Resolve from `HIVE_HOME` or the user's home directory.
    pub fn resolve() -> Self {
        if let Some(dir) = std::env::var_os(HOME_ENV).filter(|v| !v.is_empty()) {
            return Self::new(PathBuf::from(dir));
        }
        let base = dirs::home_dir().unwrap_or_else(std::env::temp_dir);
        Self::new(base.join(DEFAULT_DIR_NAME))
    }

    pub fn new(home: impl Into<PathBuf>) -> Self {
        Self { home: home.into() }
    }

    pub fn db_path(&self) -> PathBuf {
        self.home.join("hive.db")
    }

    pub fn library_dir(&self) -> PathBuf {
        self.home.join("library")
    }

    pub fn skills_dir(&self) -> PathBuf {
        self.library_dir().join("skills")
    }

    pub fn agents_dir(&self) -> PathBuf {
        self.library_dir().join("agents")
    }

    pub fn commands_dir(&self) -> PathBuf {
        self.library_dir().join("commands")
    }

    pub fn meta_dir(&self) -> PathBuf {
        self.library_dir().join("meta")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.home.join("cache")
    }

    pub fn git_cache_dir(&self) -> PathBuf {
        self.cache_dir().join("git")
    }

    pub fn workspaces_dir(&self) -> PathBuf {
        self.home.join("workspaces")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.home.join("logs")
    }

    pub fn secrets_file(&self) -> PathBuf {
        self.home.join("secrets.json")
    }

    /// Copies kept before an instruction file is overwritten or deleted.
    pub fn instruction_backups_dir(&self) -> PathBuf {
        self.home.join("backups").join("instructions")
    }

    /// Create every directory Hive needs.
    pub fn ensure(&self) -> Result<()> {
        for dir in [
            self.home.clone(),
            self.skills_dir(),
            self.agents_dir(),
            self.commands_dir(),
            self.meta_dir(),
            self.git_cache_dir(),
            self.workspaces_dir(),
            self.logs_dir(),
        ] {
            std::fs::create_dir_all(&dir).at(&dir)?;
        }
        Ok(())
    }

    /// True when `path` lives inside the library directory (after resolving symlinks where possible).
    pub fn is_in_library(&self, path: &Path) -> bool {
        let lib = canonicalize_lenient(&self.library_dir());
        canonicalize_lenient(path).starts_with(&lib)
    }
}

/// Canonicalize a path even when its tail does not exist yet: the nearest existing ancestor is
/// resolved (so `/var` → `/private/var` on macOS) and the remaining components are re-appended.
pub fn canonicalize_lenient(path: &Path) -> PathBuf {
    if let Ok(p) = dunce::canonicalize(path) {
        return p;
    }
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    let mut current = path.to_path_buf();
    while let Some(parent) = current.parent() {
        if let Some(name) = current.file_name() {
            tail.push(name.to_os_string());
        }
        if let Ok(base) = dunce::canonicalize(parent) {
            let mut out = base;
            for part in tail.iter().rev() {
                out.push(part);
            }
            return out;
        }
        current = parent.to_path_buf();
    }
    path.to_path_buf()
}

/// Expand a leading `~` to the user's home directory.
pub fn expand_tilde(input: &str) -> PathBuf {
    if let Some(rest) = input.strip_prefix("~") {
        if rest.is_empty() {
            return dirs::home_dir().unwrap_or_default();
        }
        if let Some(stripped) = rest.strip_prefix('/').or_else(|| rest.strip_prefix('\\')) {
            return dirs::home_dir().unwrap_or_default().join(stripped);
        }
    }
    PathBuf::from(input)
}

/// Render a path with the home directory collapsed to `~` for display.
pub fn display_path(path: &Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(rest) = path.strip_prefix(&home) {
            if rest.as_os_str().is_empty() {
                return "~".to_string();
            }
            return format!("~/{}", rest.to_string_lossy().replace('\\', "/"));
        }
    }
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_creates_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = HivePaths::new(tmp.path().join("hive"));
        paths.ensure().unwrap();
        assert!(paths.skills_dir().is_dir());
        assert!(paths.meta_dir().is_dir());
        assert!(paths.workspaces_dir().is_dir());
        assert!(paths.is_in_library(&paths.skills_dir().join("x")));
        assert!(!paths.is_in_library(tmp.path()));
    }

    #[cfg(unix)]
    #[test]
    fn library_membership_survives_symlinked_homes() {
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("real-home");
        std::fs::create_dir_all(&real).unwrap();
        let link = tmp.path().join("linked-home");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let paths = HivePaths::new(link.join("hive"));
        paths.ensure().unwrap();
        // A not-yet-existing path under the (symlinked) library still counts as inside it.
        assert!(paths.is_in_library(&link.join("hive/library/skills/new-skill")));
        assert!(paths.is_in_library(&real.join("hive/library/skills/new-skill")));
        assert!(!paths.is_in_library(&link.join("elsewhere")));
        assert_eq!(
            canonicalize_lenient(&link.join("hive/library/skills/a/b")),
            dunce::canonicalize(&real)
                .unwrap()
                .join("hive/library/skills/a/b")
        );
    }

    #[test]
    fn tilde_expansion() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(expand_tilde("~"), home);
        assert_eq!(expand_tilde("~/x"), home.join("x"));
        assert_eq!(expand_tilde("/abs"), PathBuf::from("/abs"));
    }
}
