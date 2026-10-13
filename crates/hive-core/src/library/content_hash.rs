// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Hash of a skill directory's content.
///
/// The hash is scoped to the files inside the skill (relative paths, not the
/// parent folder name) so two copies of the same tree share a hash. Line
/// endings are normalised to LF before hashing, which is the tiebreaker when
/// a Windows checkout and a Unix checkout would otherwise differ.
pub fn hash_skill_dir(dir: &Path) -> std::io::Result<String> {
    let mut files = Vec::new();
    collect(dir, dir, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hasher = Sha256::new();
    for (rel, bytes) in files {
        hasher.update(rel.as_bytes());
        hasher.update([0]);
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == ".git" || name == ".skill-hive" || name == ".skills-manager" {
            continue;
        }
        let meta = fs::symlink_metadata(&path)?;
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            collect(root, &path, out)?;
            continue;
        }
        if !meta.is_file() {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        let raw = fs::read(&path)?;
        out.push((rel, normalise_newlines(&raw)));
    }
    Ok(())
}

fn normalise_newlines(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' {
            out.push(b'\n');
            if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                i += 2;
                continue;
            }
        } else {
            out.push(bytes[i]);
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn shared_scope_and_eol_tiebreaker() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("copy-a").join("alpha");
        let b = tmp.path().join("copy-b").join("alpha");
        fs::create_dir_all(a.join("refs")).unwrap();
        fs::create_dir_all(b.join("refs")).unwrap();
        fs::write(a.join("SKILL.md"), "---\nname: alpha\n---\nline\n").unwrap();
        fs::write(b.join("SKILL.md"), "---\nname: alpha\n---\r\nline\r\n").unwrap();
        fs::write(a.join("refs").join("note.txt"), "same").unwrap();
        fs::write(b.join("refs").join("note.txt"), "same").unwrap();
        assert_eq!(hash_skill_dir(&a).unwrap(), hash_skill_dir(&b).unwrap());
    }
}
