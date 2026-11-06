//! Standing instruction files: the `CLAUDE.md`, `AGENTS.md`, `GEMINI.md` and rules folders an
//! agent loads into every session, at user scope (the home directory) and project scope.
//!
//! The table below says where each supported agent looks. [`Hive::instruction_files`] lists the
//! locations for a scope, whether or not the files exist, merged by path (several agents read
//! the same `AGENTS.md`). [`Hive::write_instruction_file`] only writes to one of those
//! locations (or a new member of a rules folder) and keeps a copy of what it overwrites under
//! the Hive home, so an edit can be undone.

use std::path::{Path, PathBuf};

use crate::error::{HiveError, IoContext, Result};
use crate::hive::Hive;
use crate::model::{
    HiveEvent, InstructionBackup, InstructionDocument, InstructionFile, InstructionKind, Scope,
};
use crate::util;

/// Longest instruction file read or written, in bytes.
pub const MAX_INSTRUCTION_BYTES: u64 = 2 * 1024 * 1024;
/// Backups kept per file.
pub const BACKUPS_KEPT: usize = 20;

/// Where one agent looks for standing instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructionSpec {
    pub target_id: &'static str,
    pub scope: Scope,
    /// Relative to the home directory (user scope) or the project root (project scope).
    pub rel: &'static str,
    pub label: &'static str,
    pub kind: InstructionKind,
    /// File-name suffix for members of a rules folder.
    pub pattern: Option<&'static str>,
    pub note: Option<&'static str>,
}

const fn file(
    target_id: &'static str,
    scope: Scope,
    rel: &'static str,
    label: &'static str,
    note: Option<&'static str>,
) -> InstructionSpec {
    InstructionSpec {
        target_id,
        scope,
        rel,
        label,
        kind: InstructionKind::File,
        pattern: None,
        note,
    }
}

const fn dir(
    target_id: &'static str,
    scope: Scope,
    rel: &'static str,
    label: &'static str,
    pattern: &'static str,
    note: Option<&'static str>,
) -> InstructionSpec {
    InstructionSpec {
        target_id,
        scope,
        rel,
        label,
        kind: InstructionKind::Directory,
        pattern: Some(pattern),
        note,
    }
}

const LOCAL_NOTE: Option<&str> = Some("Meant for your own machine; usually kept out of git.");
const SHARED_AGENTS_NOTE: Option<&str> =
    Some("The AGENTS.md convention: one file read by several agents.");
const LEGACY_NOTE: Option<&str> = Some("Older single-file format; the rules folder is preferred.");

/// The built-in table, in the order the agents appear in Hive.
pub const INSTRUCTION_SPECS: &[InstructionSpec] = &[
    // Claude Code
    file(
        "claude-code",
        Scope::User,
        ".claude/CLAUDE.md",
        "User memory",
        Some("Loaded in every project."),
    ),
    file(
        "claude-code",
        Scope::Project,
        "CLAUDE.md",
        "Project instructions",
        Some("Committed with the project; everyone on it sees it."),
    ),
    file(
        "claude-code",
        Scope::Project,
        ".claude/CLAUDE.md",
        "Project instructions (.claude)",
        Some("Alternative location for the project file."),
    ),
    file(
        "claude-code",
        Scope::Project,
        "CLAUDE.local.md",
        "Local instructions",
        LOCAL_NOTE,
    ),
    // Codex CLI
    file(
        "codex",
        Scope::User,
        ".codex/AGENTS.md",
        "Global instructions",
        Some("Loaded in every project."),
    ),
    file(
        "codex",
        Scope::Project,
        "AGENTS.md",
        "Project instructions",
        SHARED_AGENTS_NOTE,
    ),
    // Gemini CLI
    file(
        "gemini-cli",
        Scope::User,
        ".gemini/GEMINI.md",
        "Global context",
        Some("Loaded in every project."),
    ),
    file(
        "gemini-cli",
        Scope::Project,
        "GEMINI.md",
        "Project context",
        None,
    ),
    // Cursor
    dir(
        "cursor",
        Scope::Project,
        ".cursor/rules",
        "Rules",
        ".mdc",
        Some("One rule per file; frontmatter sets when it applies."),
    ),
    file(
        "cursor",
        Scope::Project,
        ".cursorrules",
        "Rules (legacy)",
        LEGACY_NOTE,
    ),
    file(
        "cursor",
        Scope::Project,
        "AGENTS.md",
        "Project instructions",
        SHARED_AGENTS_NOTE,
    ),
    // GitHub Copilot
    file(
        "github-copilot",
        Scope::Project,
        ".github/copilot-instructions.md",
        "Repository instructions",
        None,
    ),
    dir(
        "github-copilot",
        Scope::Project,
        ".github/instructions",
        "Path-specific instructions",
        ".instructions.md",
        Some("Each file names the paths it applies to in its frontmatter."),
    ),
    file(
        "github-copilot",
        Scope::Project,
        "AGENTS.md",
        "Project instructions",
        SHARED_AGENTS_NOTE,
    ),
    // Windsurf
    file(
        "windsurf",
        Scope::User,
        ".codeium/windsurf/memories/global_rules.md",
        "Global rules",
        None,
    ),
    dir(
        "windsurf",
        Scope::Project,
        ".windsurf/rules",
        "Rules",
        ".md",
        None,
    ),
    file(
        "windsurf",
        Scope::Project,
        ".windsurfrules",
        "Rules (legacy)",
        LEGACY_NOTE,
    ),
    // OpenCode
    file(
        "opencode",
        Scope::User,
        ".config/opencode/AGENTS.md",
        "Global instructions",
        None,
    ),
    file(
        "opencode",
        Scope::Project,
        "AGENTS.md",
        "Project instructions",
        SHARED_AGENTS_NOTE,
    ),
    // Amp
    file(
        "amp",
        Scope::User,
        ".config/amp/AGENTS.md",
        "Global instructions",
        None,
    ),
    file(
        "amp",
        Scope::Project,
        "AGENTS.md",
        "Project instructions",
        SHARED_AGENTS_NOTE,
    ),
    // Goose
    file(
        "goose",
        Scope::User,
        ".config/goose/.goosehints",
        "Global hints",
        None,
    ),
    file(
        "goose",
        Scope::Project,
        ".goosehints",
        "Project hints",
        None,
    ),
    // Kiro
    dir(
        "kiro",
        Scope::Project,
        ".kiro/steering",
        "Steering files",
        ".md",
        None,
    ),
    // Cline
    dir(
        "cline",
        Scope::User,
        "Documents/Cline/Rules",
        "Global rules",
        ".md",
        None,
    ),
    file(
        "cline",
        Scope::Project,
        ".clinerules",
        "Rules",
        Some("A single file, or a folder of .md files."),
    ),
    // Roo Code
    dir(
        "roo-code",
        Scope::User,
        ".roo/rules",
        "Global rules",
        ".md",
        None,
    ),
    dir(
        "roo-code",
        Scope::Project,
        ".roo/rules",
        "Rules",
        ".md",
        None,
    ),
    file(
        "roo-code",
        Scope::Project,
        ".roorules",
        "Rules (legacy)",
        LEGACY_NOTE,
    ),
    // The shared convention
    file(
        "agents-shared",
        Scope::Project,
        "AGENTS.md",
        "Project instructions",
        SHARED_AGENTS_NOTE,
    ),
];

/// Specs for one target at one scope.
pub fn specs_for(target_id: &str, scope: Scope) -> Vec<InstructionSpec> {
    INSTRUCTION_SPECS
        .iter()
        .filter(|s| s.target_id == target_id && s.scope == scope)
        .copied()
        .collect()
}

fn file_stats(path: &Path) -> (u64, Option<i64>, u32, u32) {
    let Ok(meta) = std::fs::metadata(path) else {
        return (0, None, 0, 0);
    };
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64);
    let size = meta.len();
    if !meta.is_file() || size > MAX_INSTRUCTION_BYTES {
        return (size, modified, 0, 0);
    }
    match std::fs::read_to_string(path) {
        Ok(text) => (
            size,
            modified,
            util::estimate_tokens(&text),
            text.lines().count() as u32,
        ),
        Err(_) => (size, modified, 0, 0),
    }
}

fn display_for(scope: Scope, rel: &str) -> String {
    match scope {
        Scope::User => format!("~/{rel}"),
        Scope::Project => rel.to_string(),
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Does `name` fit a rules folder's suffix and sit directly inside it (no separators)?
fn member_name_ok(name: &str, pattern: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && !name.contains('/')
        && !name.contains('\\')
        && name.ends_with(pattern)
        && name.len() > pattern.len()
}

/// `(target id, target name)` pairs that read one location.
type Targets = Vec<(String, String)>;

/// The location a path belongs to, when it is one Hive knows.
#[derive(Debug, Clone)]
struct Resolved {
    spec: InstructionSpec,
    scope: Scope,
    project: Option<PathBuf>,
    /// For a rules-folder member: the folder.
    parent: Option<PathBuf>,
    targets: Targets,
}

/// A known location, as the listing and the write path describe it.
struct Location<'a> {
    spec: &'a InstructionSpec,
    scope: Scope,
    project: Option<&'a Path>,
    targets: &'a [(String, String)],
}

/// Describe one path at a location. `parent` is set for a member of a rules folder.
fn entry(
    loc: &Location<'_>,
    path: PathBuf,
    display: String,
    parent: Option<&Path>,
) -> InstructionFile {
    let spec = loc.spec;
    let is_dir = path.is_dir();
    let kind = if parent.is_none() && (spec.kind == InstructionKind::Directory || is_dir) {
        InstructionKind::Directory
    } else {
        InstructionKind::File
    };
    let exists = if kind == InstructionKind::Directory {
        is_dir
    } else {
        path.is_file()
    };
    let (size_bytes, modified_at, tokens, lines) = if exists && kind == InstructionKind::File {
        file_stats(&path)
    } else if exists {
        let (_, modified, _, _) = file_stats(&path);
        (0, modified, 0, 0)
    } else {
        (0, None, 0, 0)
    };
    InstructionFile {
        path: path_string(&path),
        display,
        scope: loc.scope,
        project_path: loc.project.map(path_string),
        target_ids: loc.targets.iter().map(|(id, _)| id.clone()).collect(),
        target_names: loc.targets.iter().map(|(_, n)| n.clone()).collect(),
        label: spec.label.to_string(),
        kind,
        pattern: if kind == InstructionKind::Directory {
            Some(spec.pattern.unwrap_or(".md").to_string())
        } else {
            None
        },
        parent: parent.map(path_string),
        exists,
        size_bytes,
        modified_at,
        tokens,
        lines,
        note: spec.note.map(str::to_string),
    }
}

impl Hive {
    fn instruction_root(&self, scope: Scope, project: Option<&Path>) -> Result<PathBuf> {
        match scope {
            Scope::User => Ok(self.resolver().home().to_path_buf()),
            Scope::Project => {
                let p = project
                    .ok_or_else(|| HiveError::invalid("project scope needs a project path"))?;
                let p = crate::paths::expand_tilde(&p.to_string_lossy());
                if !p.is_dir() {
                    return Err(HiveError::invalid(format!(
                        "{} is not a directory",
                        p.display()
                    )));
                }
                Ok(dunce::canonicalize(&p).unwrap_or(p))
            }
        }
    }

    /// Every instruction location at `scope`, for the agents detected on this machine plus any
    /// agent whose file exists. One entry per path, naming every agent that reads it; a rules
    /// folder is followed by its member files.
    pub fn instruction_files(
        &self,
        scope: Scope,
        project: Option<&str>,
    ) -> Result<Vec<InstructionFile>> {
        let root = self.instruction_root(scope, project.map(Path::new))?;
        let resolver = self.resolver();
        let project_root = (scope == Scope::Project).then_some(root.as_path());
        // Group specs by path, keeping the table's order.
        let mut groups: Vec<(PathBuf, InstructionSpec, Targets)> = Vec::new();
        for spec in INSTRUCTION_SPECS.iter().filter(|s| s.scope == scope) {
            let Some(target) = resolver.get(spec.target_id) else {
                continue;
            };
            let path = root.join(spec.rel);
            if !resolver.is_installed(target) && !path.exists() {
                continue;
            }
            let who = (target.id.clone(), target.name.clone());
            match groups.iter_mut().find(|(p, _, _)| *p == path) {
                Some((_, _, targets)) => targets.push(who),
                None => groups.push((path, *spec, vec![who])),
            }
        }
        let mut out = Vec::new();
        for (path, spec, targets) in groups {
            let display = display_for(scope, spec.rel);
            let loc = Location {
                spec: &spec,
                scope,
                project: project_root,
                targets: &targets,
            };
            let top = entry(&loc, path.clone(), display.clone(), None);
            let is_dir = top.kind == InstructionKind::Directory;
            out.push(top);
            if is_dir && path.is_dir() {
                let pattern = spec.pattern.unwrap_or(".md");
                let mut names: Vec<String> = std::fs::read_dir(&path)
                    .map(|rd| {
                        rd.filter_map(|e| e.ok())
                            .filter(|e| e.path().is_file())
                            .map(|e| e.file_name().to_string_lossy().to_string())
                            .filter(|n| member_name_ok(n, pattern))
                            .collect()
                    })
                    .unwrap_or_default();
                names.sort();
                for name in names {
                    let member = path.join(&name);
                    out.push(entry(
                        &loc,
                        member,
                        format!("{display}/{name}"),
                        Some(&path),
                    ));
                }
            }
        }
        Ok(out)
    }

    /// Match an absolute path against the known locations (user scope first, then the given
    /// project, then every registered project).
    fn resolve_instruction_path(&self, path: &Path, project: Option<&str>) -> Result<Resolved> {
        let wanted = crate::paths::canonicalize_lenient(path);
        let resolver = self.resolver();
        let mut roots: Vec<(Scope, Option<PathBuf>)> = vec![(Scope::User, None)];
        if let Some(p) = project {
            roots.push((
                Scope::Project,
                Some(self.instruction_root(Scope::Project, Some(Path::new(p)))?),
            ));
        }
        for p in self.db.list_projects()? {
            let root = PathBuf::from(&p.path);
            if !roots.iter().any(|(_, r)| r.as_ref() == Some(&root)) {
                roots.push((Scope::Project, Some(root)));
            }
        }
        for (scope, project_root) in roots {
            let root = match scope {
                Scope::User => resolver.home().to_path_buf(),
                Scope::Project => project_root.clone().unwrap_or_default(),
            };
            let mut hit: Option<Resolved> = None;
            for spec in INSTRUCTION_SPECS.iter().filter(|s| s.scope == scope) {
                let Some(target) = resolver.get(spec.target_id) else {
                    continue;
                };
                let base = crate::paths::canonicalize_lenient(&root.join(spec.rel));
                let who = (target.id.clone(), target.name.clone());
                // A single-file location that is a folder on disk (`.clinerules/`) is treated
                // like a rules folder, as the listing does.
                let is_dir_spec = spec.kind == InstructionKind::Directory || base.is_dir();
                let parent = if base == wanted {
                    if is_dir_spec {
                        return Err(HiveError::invalid(format!(
                            "{} is a rules folder; write one of its files instead",
                            spec.rel
                        )));
                    }
                    None
                } else if is_dir_spec
                    && wanted.parent() == Some(base.as_path())
                    && wanted
                        .file_name()
                        .map(|n| {
                            member_name_ok(&n.to_string_lossy(), spec.pattern.unwrap_or(".md"))
                        })
                        .unwrap_or(false)
                {
                    Some(base.clone())
                } else {
                    continue;
                };
                match &mut hit {
                    Some(h) => h.targets.push(who),
                    None => {
                        hit = Some(Resolved {
                            spec: *spec,
                            scope,
                            project: project_root.clone(),
                            parent,
                            targets: vec![who],
                        })
                    }
                }
            }
            if let Some(h) = hit {
                return Ok(h);
            }
        }
        Err(HiveError::invalid(format!(
            "{} is not an instruction file of a known agent (add the project under Agents first, or pass its path)",
            path.display()
        )))
    }

    fn resolved_entry(&self, path: &Path, r: &Resolved) -> InstructionFile {
        let display = match r.scope {
            Scope::User => format!("~/{}", r.spec.rel),
            Scope::Project => r.spec.rel.to_string(),
        };
        let display = match &r.parent {
            Some(_) => format!(
                "{display}/{}",
                path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default()
            ),
            None => display,
        };
        let loc = Location {
            spec: &r.spec,
            scope: r.scope,
            project: r.project.as_deref(),
            targets: &r.targets,
        };
        entry(&loc, path.to_path_buf(), display, r.parent.as_deref())
    }

    fn is_backup_path(&self, path: &Path) -> bool {
        crate::paths::canonicalize_lenient(path).starts_with(crate::paths::canonicalize_lenient(
            &self.paths.instruction_backups_dir(),
        ))
    }

    /// Read an instruction file (or one of its backups). A known location that does not exist
    /// yet comes back with empty text and `exists: false`.
    pub fn read_instruction_file(
        &self,
        path: &str,
        project: Option<&str>,
    ) -> Result<InstructionDocument> {
        let p = instruction_path(path, project);
        if self.is_backup_path(&p) {
            let text = read_text(&p)?;
            let (size_bytes, modified_at, tokens, lines) = file_stats(&p);
            return Ok(InstructionDocument {
                file: InstructionFile {
                    path: path_string(&p),
                    display: p
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    scope: Scope::User,
                    project_path: None,
                    target_ids: vec![],
                    target_names: vec![],
                    label: "Backup".into(),
                    kind: InstructionKind::File,
                    pattern: None,
                    parent: None,
                    exists: true,
                    size_bytes,
                    modified_at,
                    tokens,
                    lines,
                    note: None,
                },
                text,
            });
        }
        let r = self.resolve_instruction_path(&p, project)?;
        let file = self.resolved_entry(&p, &r);
        let text = if file.exists {
            read_text(&p)?
        } else {
            String::new()
        };
        Ok(InstructionDocument { file, text })
    }

    /// Keep a copy of `path` under the Hive home before it changes. Oldest copies beyond
    /// [`BACKUPS_KEPT`] are removed.
    fn backup_instruction_file(&self, path: &Path) -> Result<Option<InstructionBackup>> {
        if !path.is_file() {
            return Ok(None);
        }
        let dir = self.paths.instruction_backups_dir().join(backup_key(path));
        std::fs::create_dir_all(&dir).at(&dir)?;
        let now = util::now_ms();
        let ext = path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_else(|| ".txt".into());
        // Two saves within one millisecond get distinct names: `<ms>.md`, `<ms>-1.md`, …
        let mut dest = dir.join(format!("{now}{ext}"));
        let mut n = 0;
        while dest.exists() {
            n += 1;
            dest = dir.join(format!("{now}-{n}{ext}"));
        }
        std::fs::copy(path, &dest).at(path)?;
        let size_bytes = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
        let mut all = list_backups(&dir);
        while all.len() > BACKUPS_KEPT {
            let oldest = all.remove(0);
            let _ = std::fs::remove_file(&oldest.path);
        }
        Ok(Some(InstructionBackup {
            path: path_string(&dest),
            saved_at: now,
            size_bytes,
        }))
    }

    /// Backups of an instruction file, oldest first.
    pub fn instruction_backups(&self, path: &str) -> Result<Vec<InstructionBackup>> {
        let p = crate::paths::expand_tilde(path);
        Ok(list_backups(
            &self.paths.instruction_backups_dir().join(backup_key(&p)),
        ))
    }

    /// Write (or create) an instruction file at a known location, keeping a backup of the
    /// previous content. A symlinked file (`~/.claude/CLAUDE.md -> ~/dotfiles/CLAUDE.md`)
    /// is written through: the link stays and its target changes.
    pub fn write_instruction_file(
        &self,
        path: &str,
        text: &str,
        project: Option<&str>,
    ) -> Result<InstructionDocument> {
        let p = instruction_path(path, project);
        let r = self.resolve_instruction_path(&p, project)?;
        if text.len() as u64 > MAX_INSTRUCTION_BYTES {
            return Err(HiveError::invalid("the file is larger than 2 MB"));
        }
        if p.is_dir() {
            return Err(HiveError::invalid(format!(
                "{} is a folder; write one of its files instead",
                p.display()
            )));
        }
        let target = link_target(&p);
        let backup = self.backup_instruction_file(&target)?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).at(parent)?;
        }
        // The atomic write renames a fresh temp file into place; keep the mode the file had.
        let permissions = std::fs::metadata(&target).ok().map(|m| m.permissions());
        util::write_atomic(&target, text.as_bytes())?;
        if let Some(perms) = permissions {
            let _ = std::fs::set_permissions(&target, perms);
        }
        self.db.audit(
            "instructions.write",
            &serde_json::json!({
                "path": path_string(&p),
                "bytes": text.len(),
                "backup": backup.as_ref().map(|b| b.path.clone()),
            }),
        )?;
        self.emit(HiveEvent::InstructionsChanged);
        let file = self.resolved_entry(&p, &r);
        Ok(InstructionDocument {
            file,
            text: text.to_string(),
        })
    }

    /// Delete an instruction file (a backup is kept). Rules folders themselves are not deleted.
    pub fn delete_instruction_file(
        &self,
        path: &str,
        project: Option<&str>,
    ) -> Result<InstructionFile> {
        let p = instruction_path(path, project);
        let r = self.resolve_instruction_path(&p, project)?;
        if !p.is_file() {
            return Err(HiveError::not_found(format!(
                "{} does not exist",
                p.display()
            )));
        }
        let target = link_target(&p);
        self.backup_instruction_file(&target)?;
        std::fs::remove_file(&target).at(&target)?;
        if target != p {
            // Deleting through a symlink removes the link as well, so nothing dangles.
            let _ = std::fs::remove_file(&p);
        }
        self.db.audit(
            "instructions.delete",
            &serde_json::json!({"path": path_string(&p)}),
        )?;
        self.emit(HiveEvent::InstructionsChanged);
        Ok(self.resolved_entry(&p, &r))
    }
}

/// The path a command names: `~` expanded, and a relative path taken from the project root
/// when one is given (so a name copied from `hive instructions list --project` works
/// wherever the command runs).
fn instruction_path(path: &str, project: Option<&str>) -> PathBuf {
    let p = crate::paths::expand_tilde(path);
    match project {
        Some(root) if p.is_relative() => crate::paths::expand_tilde(root).join(p),
        _ => p,
    }
}

/// The file a path really names: the target of a symlinked file, else the path itself.
fn link_target(path: &Path) -> PathBuf {
    match std::fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => {
            dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
        }
        _ => path.to_path_buf(),
    }
}

fn read_text(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).at(path)?;
    if bytes.len() as u64 > MAX_INSTRUCTION_BYTES {
        return Err(HiveError::invalid("the file is larger than 2 MB"));
    }
    String::from_utf8(bytes).map_err(|_| HiveError::invalid("the file is not UTF-8 text"))
}

/// A folder name for a path's backups: the path with every separator and odd character
/// replaced, plus a short hash so two paths never share a folder.
fn backup_key(path: &Path) -> String {
    let canonical = crate::paths::canonicalize_lenient(path);
    let text = canonical.to_string_lossy().replace('\\', "/");
    let mut key: String = text
        .trim_start_matches('/')
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if key.len() > 80 {
        key = key[key.len() - 80..].to_string();
    }
    format!("{}-{}", key, &crate::hash::hash_bytes(text.as_bytes())[..8])
}

fn list_backups(dir: &Path) -> Vec<InstructionBackup> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    // `<ms>.ext` or `<ms>-<n>.ext`; the sequence number orders copies saved in the same
    // millisecond.
    let mut found: Vec<(i64, i64, InstructionBackup)> = rd
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let stem = name.split('.').next()?;
            let (ms, seq) = match stem.split_once('-') {
                Some((ms, n)) => (ms, n.parse::<i64>().ok()?),
                None => (stem, 0),
            };
            let saved_at: i64 = ms.parse().ok()?;
            Some((
                saved_at,
                seq,
                InstructionBackup {
                    path: path_string(&e.path()),
                    saved_at,
                    size_bytes: e.metadata().map(|m| m.len()).unwrap_or(0),
                },
            ))
        })
        .collect();
    found.sort_by_key(|(ms, seq, _)| (*ms, *seq));
    found.into_iter().map(|(_, _, b)| b).collect()
}

/// Which targets read `AGENTS.md`-style shared files; handy for docs and tests.
pub fn targets_reading(rel: &str, scope: Scope) -> Vec<&'static str> {
    INSTRUCTION_SPECS
        .iter()
        .filter(|s| s.scope == scope && s.rel == rel)
        .map(|s| s.target_id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn temp_hive() -> (tempfile::TempDir, Arc<Hive>, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let hive = Hive::open_temp(tmp.path()).unwrap();
        let home = tmp.path().join("home");
        std::fs::create_dir_all(home.join(".claude")).unwrap();
        std::fs::create_dir_all(home.join(".codex")).unwrap();
        hive.set_target_home(home.clone()).unwrap();
        (tmp, hive, home)
    }

    #[test]
    fn table_is_consistent() {
        for spec in INSTRUCTION_SPECS {
            assert!(
                crate::targets::builtin_targets()
                    .iter()
                    .any(|t| t.id == spec.target_id),
                "{}",
                spec.target_id
            );
            assert!(
                !spec.rel.starts_with('/') && !spec.rel.starts_with('~'),
                "{}",
                spec.rel
            );
            if spec.kind == InstructionKind::Directory {
                assert!(spec.pattern.is_some(), "{}", spec.rel);
            }
        }
        let readers = targets_reading("AGENTS.md", Scope::Project);
        assert!(
            readers.contains(&"codex")
                && readers.contains(&"opencode")
                && readers.contains(&"cursor")
        );
    }

    #[test]
    fn lists_user_files_for_detected_agents_and_merges_shared_paths() {
        let (_tmp, hive, home) = temp_hive();
        let files = hive.instruction_files(Scope::User, None).unwrap();
        let claude = files
            .iter()
            .find(|f| f.path.ends_with(".claude/CLAUDE.md"))
            .expect("claude user memory");
        assert!(!claude.exists);
        assert_eq!(claude.display, "~/.claude/CLAUDE.md");
        assert_eq!(claude.target_ids, vec!["claude-code"]);
        assert!(files.iter().any(|f| f.path.ends_with(".codex/AGENTS.md")));
        // Cursor is not detected and has no user file: nothing listed. Gemini is not detected
        // either, but its file exists, so it is listed.
        assert!(
            !files
                .iter()
                .any(|f| f.target_ids.contains(&"cursor".to_string()))
        );
        std::fs::create_dir_all(home.join(".gemini")).unwrap();
        std::fs::write(home.join(".gemini/GEMINI.md"), "# Gemini\n\nBe brief.\n").unwrap();
        let files = hive.instruction_files(Scope::User, None).unwrap();
        let gemini = files
            .iter()
            .find(|f| f.path.ends_with("GEMINI.md"))
            .unwrap();
        assert!(gemini.exists);
        assert_eq!(gemini.lines, 3);
        assert!(gemini.tokens > 0);
        assert!(gemini.modified_at.is_some());

        // Project scope: AGENTS.md is one entry naming every agent that reads it.
        let project = home.join("proj");
        std::fs::create_dir_all(project.join(".cursor/rules")).unwrap();
        std::fs::write(
            project.join(".cursor/rules/style.mdc"),
            "---\nalwaysApply: true\n---\nUse tabs.\n",
        )
        .unwrap();
        std::fs::write(project.join(".cursor/rules/notes.txt"), "ignored").unwrap();
        std::fs::write(project.join("AGENTS.md"), "# Agents\n").unwrap();
        let files = hive
            .instruction_files(Scope::Project, Some(&project.to_string_lossy()))
            .unwrap();
        let agents = files.iter().find(|f| f.display == "AGENTS.md").unwrap();
        assert!(agents.exists);
        assert!(agents.target_ids.contains(&"codex".to_string()));
        assert!(
            agents.target_ids.contains(&"cursor".to_string()),
            "{:?}",
            agents.target_ids
        );
        assert_eq!(files.iter().filter(|f| f.display == "AGENTS.md").count(), 1);
        let rules = files.iter().find(|f| f.display == ".cursor/rules").unwrap();
        assert_eq!(rules.kind, InstructionKind::Directory);
        assert_eq!(rules.pattern.as_deref(), Some(".mdc"));
        let member = files
            .iter()
            .find(|f| f.display == ".cursor/rules/style.mdc")
            .unwrap();
        assert_eq!(member.kind, InstructionKind::File);
        assert_eq!(member.parent.as_deref(), Some(rules.path.as_str()));
        assert!(!files.iter().any(|f| f.display.ends_with("notes.txt")));
        assert!(files.iter().any(|f| f.display == "CLAUDE.md" && !f.exists));
    }

    #[test]
    fn writes_with_backups_and_refuses_unknown_paths() {
        let (_tmp, hive, home) = temp_hive();
        let path = home.join(".claude/CLAUDE.md");
        let mut rx = hive.subscribe();
        let doc = hive
            .read_instruction_file(&path.to_string_lossy(), None)
            .unwrap();
        assert!(!doc.file.exists);
        assert_eq!(doc.text, "");
        let doc = hive
            .write_instruction_file(&path.to_string_lossy(), "# Memory\n\nv1\n", None)
            .unwrap();
        assert!(doc.file.exists);
        assert_eq!(doc.file.lines, 3);
        assert!(matches!(rx.try_recv(), Ok(HiveEvent::InstructionsChanged)));
        assert!(
            hive.instruction_backups(&path.to_string_lossy())
                .unwrap()
                .is_empty()
        );
        hive.write_instruction_file(&path.to_string_lossy(), "# Memory\n\nv2\n", None)
            .unwrap();
        let backups = hive.instruction_backups(&path.to_string_lossy()).unwrap();
        assert_eq!(backups.len(), 1);
        let old = hive.read_instruction_file(&backups[0].path, None).unwrap();
        assert_eq!(old.text, "# Memory\n\nv1\n");
        assert_eq!(old.file.label, "Backup");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "# Memory\n\nv2\n");

        // Unknown locations are refused; so are rules folders themselves and wrong suffixes.
        assert!(
            hive.write_instruction_file(&home.join("random.md").to_string_lossy(), "x", None)
                .is_err()
        );
        assert!(
            hive.write_instruction_file(&home.join(".ssh/config").to_string_lossy(), "x", None)
                .is_err()
        );
        let project = home.join("proj");
        std::fs::create_dir_all(&project).unwrap();
        let rules = project.join(".cursor/rules");
        assert!(
            hive.write_instruction_file(
                &rules.to_string_lossy(),
                "x",
                Some(&project.to_string_lossy())
            )
            .is_err()
        );
        assert!(
            hive.write_instruction_file(
                &rules.join("x.txt").to_string_lossy(),
                "x",
                Some(&project.to_string_lossy())
            )
            .is_err()
        );
        // An unregistered project needs its path passed; registered ones are found on their own.
        let member = rules.join("style.mdc");
        assert!(
            hive.write_instruction_file(&member.to_string_lossy(), "rule", None)
                .is_err()
        );
        let doc = hive
            .write_instruction_file(
                &member.to_string_lossy(),
                "rule",
                Some(&project.to_string_lossy()),
            )
            .unwrap();
        assert_eq!(doc.file.display, ".cursor/rules/style.mdc");
        assert_eq!(doc.file.scope, Scope::Project);
        hive.add_project(&project.to_string_lossy()).unwrap();
        assert!(
            hive.write_instruction_file(&member.to_string_lossy(), "rule 2", None)
                .is_ok()
        );
        let deleted = hive
            .delete_instruction_file(&member.to_string_lossy(), None)
            .unwrap();
        assert!(!deleted.exists);
        assert!(!member.exists());
        assert_eq!(
            hive.instruction_backups(&member.to_string_lossy())
                .unwrap()
                .len(),
            2
        );
        assert!(
            hive.delete_instruction_file(&member.to_string_lossy(), None)
                .is_err()
        );
    }

    #[test]
    fn backups_are_capped() {
        let (_tmp, hive, home) = temp_hive();
        let path = home.join(".codex/AGENTS.md");
        for i in 0..(BACKUPS_KEPT + 5) {
            // No pause between writes: several land in the same millisecond and must
            // still be kept apart.
            hive.write_instruction_file(&path.to_string_lossy(), &format!("v{i}"), None)
                .unwrap();
        }
        let backups = hive.instruction_backups(&path.to_string_lossy()).unwrap();
        assert_eq!(backups.len(), BACKUPS_KEPT);
        assert!(backups.windows(2).all(|w| w[0].saved_at <= w[1].saved_at));
        // The newest backup holds the previous version, the oldest kept the earliest survivor.
        let newest = hive
            .read_instruction_file(&backups[BACKUPS_KEPT - 1].path, None)
            .unwrap();
        assert_eq!(newest.text, format!("v{}", BACKUPS_KEPT + 3));
        let oldest = hive.read_instruction_file(&backups[0].path, None).unwrap();
        assert_eq!(oldest.text, "v4");
    }

    #[test]
    fn relative_paths_follow_the_project_and_folder_form_locations_work() {
        let (_tmp, hive, home) = temp_hive();
        let project = home.join("proj");
        std::fs::create_dir_all(project.join(".clinerules")).unwrap();
        std::fs::write(project.join(".clinerules/coding.md"), "# Coding\n").unwrap();
        let proj = project.to_string_lossy().to_string();

        // A name copied from the listing works from any directory.
        let doc = hive
            .write_instruction_file("CLAUDE.md", "# Project\n", Some(&proj))
            .unwrap();
        assert_eq!(doc.file.display, "CLAUDE.md");
        assert_eq!(
            std::fs::read_to_string(project.join("CLAUDE.md")).unwrap(),
            "# Project\n"
        );
        assert_eq!(
            hive.read_instruction_file("CLAUDE.md", Some(&proj))
                .unwrap()
                .text,
            "# Project\n"
        );

        // `.clinerules/` as a folder: listed with its members, which can be read and written.
        let files = hive.instruction_files(Scope::Project, Some(&proj)).unwrap();
        let folder = files.iter().find(|f| f.display == ".clinerules").unwrap();
        assert_eq!(folder.kind, InstructionKind::Directory);
        assert!(
            files
                .iter()
                .any(|f| f.display == ".clinerules/coding.md" && f.exists)
        );
        let doc = hive
            .read_instruction_file(".clinerules/coding.md", Some(&proj))
            .unwrap();
        assert_eq!(doc.text, "# Coding\n");
        assert_eq!(doc.file.parent.as_deref(), Some(folder.path.as_str()));
        hive.write_instruction_file(".clinerules/testing.md", "# Tests\n", Some(&proj))
            .unwrap();
        assert!(project.join(".clinerules/testing.md").is_file());
        assert!(
            hive.write_instruction_file(".clinerules", "x", Some(&proj))
                .is_err()
        );
        assert!(
            hive.write_instruction_file(".clinerules/notes.txt", "x", Some(&proj))
                .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn writes_through_symlinks_and_keeps_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let (_tmp, hive, home) = temp_hive();
        let real = home.join("dotfiles/CLAUDE.md");
        std::fs::create_dir_all(real.parent().unwrap()).unwrap();
        std::fs::write(&real, "# Dotfiles\n").unwrap();
        std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o644)).unwrap();
        let link = home.join(".claude/CLAUDE.md");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        hive.write_instruction_file(&link.to_string_lossy(), "# Edited\n", None)
            .unwrap();
        assert!(
            std::fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(std::fs::read_to_string(&real).unwrap(), "# Edited\n");
        assert_eq!(
            std::fs::metadata(&real).unwrap().permissions().mode() & 0o777,
            0o644
        );
        let backups = hive.instruction_backups(&link.to_string_lossy()).unwrap();
        assert_eq!(backups.len(), 1);
        assert_eq!(
            hive.read_instruction_file(&backups[0].path, None)
                .unwrap()
                .text,
            "# Dotfiles\n"
        );

        hive.delete_instruction_file(&link.to_string_lossy(), None)
            .unwrap();
        assert!(!real.exists());
        assert!(std::fs::symlink_metadata(&link).is_err());
    }
}
