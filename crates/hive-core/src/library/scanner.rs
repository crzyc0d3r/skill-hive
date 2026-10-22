// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

use anyhow::Result;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::db::skill_store::SkillStore;
use crate::library::content_hash::hash_skill_dir;
use crate::library::installer::install_local;
use crate::library::skill_metadata::{find_skill_dirs, read_skill_document, sanitize_skill_name};

#[derive(Debug, Clone, Serialize)]
pub struct ScannedSkill {
    pub tool: String,
    pub path: String,
    pub name: String,
    pub fingerprint: String,
    pub already_managed: bool,
    /// Another copy of this name has different content.
    pub differs: bool,
    /// Empty when the folder read cleanly. Otherwise the IO cause
    /// ("not found", "permission denied", the error message) or
    /// "parse error at line N".
    pub reason: String,
}

pub fn scan_roots(store: &SkillStore, roots: &[(&str, &Path)]) -> Result<Vec<ScannedSkill>> {
    scan_roots_with_central(store, roots, None)
}

/// Flat children, nested `SKILL.md` trees, and symlinks that already point into
/// the central library. A central symlink is managed even when its hash is new.
pub fn scan_roots_with_central(
    store: &SkillStore,
    roots: &[(&str, &Path)],
    central: Option<&Path>,
) -> Result<Vec<ScannedSkill>> {
    let managed: std::collections::HashSet<String> = store
        .get_all_skills()?
        .into_iter()
        .filter_map(|s| s.content_hash)
        .collect();
    let home = central.and_then(|path| path.canonicalize().ok());
    let mut found = Vec::new();
    for (tool, root) in roots {
        if !root.exists() {
            continue;
        }
        let mut dirs = collect_flat(root);
        dirs.extend(collect_recursive(root));
        dirs.extend(collect_central_symlinks(root, home.as_deref()));
        dirs.sort();
        dirs.dedup();
        for dir in dirs {
            let text = match fs::read_to_string(dir.join("SKILL.md")) {
                Ok(text) => text,
                Err(err) => {
                    found.push(failed_skill(tool, &dir, &io_reason(&err)));
                    continue;
                }
            };
            if let Some(reason) = parse_reason(&text) {
                found.push(failed_skill(tool, &dir, &reason));
                continue;
            }
            let doc = match read_skill_document(&dir) {
                Ok(doc) => doc,
                Err(err) => {
                    found.push(failed_skill(tool, &dir, &io_reason(&err)));
                    continue;
                }
            };
            let fingerprint = hash_skill_dir(&dir).unwrap_or_default();
            let linked = home
                .as_ref()
                .is_some_and(|central_dir| symlink_points_inside(&dir, central_dir));
            found.push(ScannedSkill {
                tool: (*tool).to_string(),
                path: dir.to_string_lossy().to_string(),
                name: doc.name,
                fingerprint: fingerprint.clone(),
                already_managed: managed.contains(&fingerprint) || linked,
                differs: false,
                reason: String::new(),
            });
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    let mut grouped = Vec::new();
    for mut skill in found {
        if let Some(existing) = grouped
            .iter_mut()
            .find(|row: &&mut ScannedSkill| row.name.eq_ignore_ascii_case(&skill.name))
        {
            let different = !skill.fingerprint.is_empty()
                && !existing.fingerprint.is_empty()
                && skill.fingerprint != existing.fingerprint;
            if different {
                existing.differs = true;
                skill.differs = true;
                grouped.push(skill);
            } else if skill.already_managed {
                existing.already_managed = true;
            }
            continue;
        }
        grouped.push(skill);
    }
    Ok(grouped)
}

fn io_reason(err: &std::io::Error) -> String {
    match err.kind() {
        std::io::ErrorKind::NotFound => "not found".to_string(),
        std::io::ErrorKind::PermissionDenied => "permission denied".to_string(),
        _ => {
            let message = err.to_string();
            if message.is_empty() {
                "read error".to_string()
            } else {
                message
            }
        }
    }
}

/// Empty files, an unclosed opening fence, and a frontmatter line with no
/// colon are parse failures. The line number is 1-based in the file.
fn parse_reason(text: &str) -> Option<String> {
    parse_error_line(text).map(|line| format!("parse error at line {line}"))
}

fn parse_error_line(text: &str) -> Option<usize> {
    if text.trim().is_empty() {
        return Some(1);
    }
    let rest = text.trim_start_matches('\u{feff}');
    let after = rest.strip_prefix("---")?;
    let after = if let Some(stripped) = after.strip_prefix("\r\n") {
        stripped
    } else if let Some(stripped) = after.strip_prefix('\n') {
        stripped
    } else {
        return Some(1);
    };
    let Some(end) = after.find("\n---") else {
        return Some(1);
    };
    let front = &after[..end];
    for (offset, line) in front.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.split_once(':').is_none() {
            return Some(offset + 2);
        }
    }
    None
}

fn failed_skill(tool: &str, dir: &Path, reason: &str) -> ScannedSkill {
    let folder = dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("skill");
    ScannedSkill {
        tool: tool.to_string(),
        path: dir.to_string_lossy().to_string(),
        name: sanitize_skill_name(folder),
        fingerprint: String::new(),
        already_managed: false,
        differs: false,
        reason: reason.to_string(),
    }
}

pub fn collect_flat(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.join("SKILL.md").is_file() {
            found.push(path);
        }
    }
    found.sort();
    found
}

pub fn collect_recursive(root: &Path) -> Vec<PathBuf> {
    find_skill_dirs(root, 6)
}

pub fn collect_central_symlinks(root: &Path, central: Option<&Path>) -> Vec<PathBuf> {
    let Some(central) = central else {
        return Vec::new();
    };
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if symlink_points_inside(&path, central) {
            found.push(path);
        }
    }
    found.sort();
    found
}

fn symlink_points_inside(path: &Path, central: &Path) -> bool {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return false;
    };
    if !meta.file_type().is_symlink() {
        return false;
    }
    let Ok(target) = fs::read_link(path) else {
        return false;
    };
    let absolute = if target.is_absolute() {
        target
    } else {
        path.parent().unwrap_or(path).join(target)
    };
    absolute
        .canonicalize()
        .ok()
        .is_some_and(|real| real.starts_with(central))
}

pub fn import_scanned(
    store: &SkillStore,
    skills_root: &Path,
    scanned: &ScannedSkill,
    rename: Option<&str>,
) -> Result<String> {
    let installed = install_local(store, skills_root, Path::new(&scanned.path))?;
    if let Some(name) = rename {
        let name = crate::library::skill_metadata::sanitize_skill_name(name);
        store.update_skill_after_install(
            &installed.record.id,
            &name,
            installed.record.description.as_deref(),
            None,
            None,
            installed.record.content_hash.as_deref(),
            "unknown",
        )?;
        return Ok(name);
    }
    Ok(installed.record.name)
}

pub fn import_all(
    store: &SkillStore,
    skills_root: &Path,
    scanned: &[ScannedSkill],
) -> Result<Vec<String>> {
    let mut names = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for skill in scanned.iter().filter(|s| !s.already_managed) {
        if !seen.insert(skill.name.to_ascii_lowercase()) {
            continue;
        }
        names.push(import_scanned(store, skills_root, skill, None)?);
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::skill_store::SkillStore;

    fn write_skill(dir: &Path, name: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: d\n---\n{name}\n"),
        )
        .unwrap();
    }

    #[test]
    fn flat_recursive_and_central_symlink_collectors_split_the_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("agent");
        write_skill(&root.join("top"), "top");
        write_skill(&root.join("nested").join("deep"), "deep");
        let central = tmp.path().join("central");
        write_skill(&central.join("linked"), "linked");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(central.join("linked"), root.join("linked")).unwrap();
        }
        assert_eq!(collect_flat(&root).len(), if cfg!(unix) { 2 } else { 1 });
        assert!(collect_recursive(&root)
            .iter()
            .any(|path| path.ends_with("deep")));
        let store = SkillStore::new(&tmp.path().join("hive.db")).unwrap();
        let rows = scan_roots_with_central(
            &store,
            &[("codex", root.as_path())],
            Some(central.as_path()),
        )
        .unwrap();
        assert!(rows
            .iter()
            .any(|row| row.name == "top" && !row.already_managed && row.reason.is_empty()));
        assert!(rows.iter().any(|row| row.name == "deep"));
        #[cfg(unix)]
        assert!(rows
            .iter()
            .any(|row| row.name == "linked" && row.already_managed));
        let imported = import_all(&store, &tmp.path().join("library"), &rows).unwrap();
        assert!(imported.iter().any(|name| name == "top"));
        #[cfg(unix)]
        assert!(!imported.iter().any(|name| name == "linked"));
        assert!(tmp
            .path()
            .join("library")
            .join("top")
            .join("SKILL.md")
            .is_file());
    }

    #[test]
    fn unreadable_and_empty_skills_share_error_reasons() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("agent");
        fs::create_dir_all(root.join("blank")).unwrap();
        fs::write(root.join("blank").join("SKILL.md"), "   ").unwrap();
        fs::create_dir_all(root.join("locked")).unwrap();
        let store = SkillStore::new(&tmp.path().join("hive.db")).unwrap();
        let rows = scan_roots_with_central(&store, &[("cursor", root.as_path())], None).unwrap();
        let blank = rows.iter().find(|row| row.name == "blank").unwrap();
        assert_eq!(blank.reason, "parse error at line 1");
        assert!(blank.path.contains("blank"));
        fs::create_dir_all(root.join("bad-line")).unwrap();
        fs::write(
            root.join("bad-line").join("SKILL.md"),
            "---\nname: bad-line\nnot a field\n---\nbody\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("open-fence")).unwrap();
        fs::write(
            root.join("open-fence").join("SKILL.md"),
            "---\nname: open-fence\n",
        )
        .unwrap();
        let rows = scan_roots_with_central(&store, &[("cursor", root.as_path())], None).unwrap();
        assert_eq!(
            rows.iter()
                .find(|row| row.name == "bad-line")
                .unwrap()
                .reason,
            "parse error at line 3"
        );
        assert_eq!(
            rows.iter()
                .find(|row| row.name == "open-fence")
                .unwrap()
                .reason,
            "parse error at line 1"
        );
    }

    #[test]
    fn io_reasons_use_the_error_kind_and_message() {
        assert_eq!(
            io_reason(&std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "missing"
            )),
            "not found"
        );
        assert_eq!(
            io_reason(&std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "no"
            )),
            "permission denied"
        );
        let message = io_reason(&std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "stream did not contain valid UTF-8",
        ));
        assert!(message.contains("UTF-8"), "{message}");
    }

    #[cfg(unix)]
    #[test]
    fn permission_denied_reports_the_os_error() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("agent");
        let file = root.join("locked").join("SKILL.md");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, "---\nname: locked\ndescription: d\n---\nbody\n").unwrap();
        let mut blocked = fs::metadata(&file).unwrap().permissions();
        blocked.set_mode(0o000);
        fs::set_permissions(&file, blocked).unwrap();
        struct Restore(std::path::PathBuf);
        impl Drop for Restore {
            fn drop(&mut self) {
                let mut perms = fs::metadata(&self.0).unwrap().permissions();
                perms.set_mode(0o644);
                fs::set_permissions(&self.0, perms).ok();
            }
        }
        let _restore = Restore(file.clone());
        let store = SkillStore::new(&tmp.path().join("hive.db")).unwrap();
        let rows = scan_roots_with_central(&store, &[("cursor", root.as_path())], None).unwrap();
        assert_eq!(
            rows.iter().find(|row| row.name == "locked").unwrap().reason,
            "permission denied"
        );
    }

    #[test]
    fn copies_with_different_content_stay_visible() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("codex");
        write_skill(&root.join("shared"), "shared");
        let other = tmp.path().join("cursor");
        let copy = other.join("shared");
        fs::create_dir_all(&copy).unwrap();
        fs::write(
            copy.join("SKILL.md"),
            "---\nname: shared\ndescription: d\n---\nother body\n",
        )
        .unwrap();
        let store = SkillStore::new(&tmp.path().join("hive.db")).unwrap();
        let rows = scan_roots_with_central(
            &store,
            &[("codex", root.as_path()), ("cursor", other.as_path())],
            None,
        )
        .unwrap();
        let shared: Vec<_> = rows.iter().filter(|row| row.name == "shared").collect();
        assert_eq!(shared.len(), 2);
        assert!(shared.iter().all(|row| row.differs));
    }
}
