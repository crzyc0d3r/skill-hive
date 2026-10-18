// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

//! Compare a skill's central copy with the folder it was installed from.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::updates::{self, UpdateStatus};

pub fn classify_dirs(local: &Path, source: Option<&Path>) -> (String, Option<String>) {
    let Some(source) = source.filter(|path| !path.as_os_str().is_empty()) else {
        return ("source_missing".into(), Some("no source".into()));
    };
    if !source.exists() {
        return (
            "source_missing".into(),
            Some("source folder is missing".into()),
        );
    }
    let check = updates::check_replacement(&read_tree(local), &read_tree(source));
    let status = match check.status {
        UpdateStatus::Current => "current",
        UpdateStatus::UpdateAvailable => "update_available",
        UpdateStatus::PendingRemovals => "pending_removals",
        UpdateStatus::Failed => "failed",
    };
    let detail = if check.pending_removals.is_empty() {
        None
    } else {
        Some(check.pending_removals.join("\n"))
    };
    (status.into(), detail)
}

fn read_tree(dir: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    collect(dir, dir, &mut out);
    out
}

fn collect(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == ".git" || name == ".skill-hive" {
            continue;
        }
        if path.is_dir() {
            collect(root, &path, out);
            continue;
        }
        if !path.is_file() {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        out.insert(rel, text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn a_changed_source_file_is_an_update_and_a_missing_folder_is_not_current() {
        let root = std::env::temp_dir().join(format!("skill-hive-check-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let central = root.join("central");
        let source = root.join("source");
        fs::create_dir_all(&central).unwrap();
        fs::create_dir_all(&source).unwrap();
        fs::write(central.join("SKILL.md"), "---\nname: notes\n---\nsame\n").unwrap();
        fs::write(source.join("SKILL.md"), "---\nname: notes\n---\nsame\n").unwrap();
        assert_eq!(classify_dirs(&central, Some(&source)).0, "current");
        fs::write(source.join("SKILL.md"), "---\nname: notes\n---\nnewer\n").unwrap();
        let (status, _) = classify_dirs(&central, Some(&source));
        assert_eq!(status, "update_available");
        let (missing, detail) = classify_dirs(&central, Some(&root.join("gone")));
        assert_eq!(missing, "source_missing");
        assert!(detail.unwrap().contains("missing"));
        let _ = fs::remove_dir_all(&root);
    }
}
