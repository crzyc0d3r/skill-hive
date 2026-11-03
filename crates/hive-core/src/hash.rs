//! Content hashing for directories and files.
//!
//! The hash covers relative paths and file contents in sorted order, so two copies of a
//! skill compare equal regardless of where they live or when they were copied.

use std::path::Path;

use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::error::{IoContext, Result};

/// Directory names never included in a hash.
pub const IGNORED_DIRS: [&str; 5] = [".git", "node_modules", "__pycache__", ".hive", ".DS_Store"];

pub fn hash_bytes(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

pub fn hash_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).at(path)?;
    Ok(hash_bytes(&bytes))
}

/// Hash a directory tree. Returns `(hash, total_bytes)`.
pub fn hash_dir(dir: &Path) -> Result<(String, u64)> {
    let mut files: Vec<(String, std::path::PathBuf)> = WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_entry(|e| {
            !(e.depth() > 0
                && e.file_type().is_dir()
                && IGNORED_DIRS.contains(&e.file_name().to_string_lossy().as_ref()))
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.file_name() != ".DS_Store")
        .map(|e| {
            let rel = e
                .path()
                .strip_prefix(dir)
                .unwrap_or(e.path())
                .to_string_lossy()
                .replace('\\', "/");
            (rel, e.path().to_path_buf())
        })
        .collect();
    files.sort();
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    for (rel, path) in files {
        let bytes = std::fs::read(&path).at(&path)?;
        total += bytes.len() as u64;
        hasher.update(rel.as_bytes());
        hasher.update([0u8]);
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
        hasher.update([0u8]);
    }
    Ok((hex::encode(hasher.finalize()), total))
}

/// Hash whichever kind of path this is.
pub fn hash_path(path: &Path) -> Result<(String, u64)> {
    if path.is_dir() {
        hash_dir(path)
    } else {
        let bytes = std::fs::read(path).at(path)?;
        Ok((hash_bytes(&bytes), bytes.len() as u64))
    }
}

/// Short prefix for display.
pub fn short(hash: &str) -> &str {
    &hash[..hash.len().min(12)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_content_same_hash() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a");
        let b = tmp.path().join("b");
        for d in [&a, &b] {
            std::fs::create_dir_all(d.join("sub")).unwrap();
            std::fs::write(d.join("SKILL.md"), "x").unwrap();
            std::fs::write(d.join("sub/f.txt"), "y").unwrap();
        }
        std::fs::create_dir_all(a.join(".git")).unwrap();
        std::fs::write(a.join(".git/HEAD"), "ref").unwrap();
        let (ha, size) = hash_dir(&a).unwrap();
        let (hb, _) = hash_dir(&b).unwrap();
        assert_eq!(ha, hb);
        assert_eq!(size, 2);
        std::fs::write(b.join("sub/f.txt"), "z").unwrap();
        assert_ne!(hash_dir(&b).unwrap().0, ha);
    }
}
