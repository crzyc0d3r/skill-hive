//! Safe extraction of downloaded `.zip` and `.tar.gz` archives.

use std::io::Read;
use std::path::{Component, Path, PathBuf};

use crate::error::{HiveError, IoContext, Result};

pub const MAX_ARCHIVE_BYTES: u64 = 200 * 1024 * 1024;
pub const MAX_ENTRIES: usize = 20_000;

fn safe_join(root: &Path, name: &str) -> Option<PathBuf> {
    let rel = Path::new(name);
    if rel.is_absolute() {
        return None;
    }
    let mut out = root.to_path_buf();
    for c in rel.components() {
        match c {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            _ => return None,
        }
    }
    Some(out)
}

pub fn extract_zip(bytes: &[u8], dest: &Path) -> Result<()> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| HiveError::invalid(format!("not a zip archive: {e}")))?;
    if archive.len() > MAX_ENTRIES {
        return Err(HiveError::invalid("archive has too many entries"));
    }
    let mut total = 0u64;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| HiveError::invalid(format!("zip entry error: {e}")))?;
        // `enclosed_name` already rejects absolute paths and `..` components.
        let Some(path) = entry
            .enclosed_name()
            .and_then(|rel| safe_join(dest, &rel.to_string_lossy()))
        else {
            continue;
        };
        if entry.is_dir() {
            std::fs::create_dir_all(&path).at(&path)?;
            continue;
        }
        total += entry.size();
        if total > MAX_ARCHIVE_BYTES {
            return Err(HiveError::invalid("archive is too large"));
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).at(parent)?;
        }
        let mut out = std::fs::File::create(&path).at(&path)?;
        std::io::copy(&mut entry, &mut out).at(&path)?;
    }
    Ok(())
}

pub fn extract_tar_gz(bytes: &[u8], dest: &Path) -> Result<()> {
    let gz = flate2::read::GzDecoder::new(bytes);
    let mut archive = tar::Archive::new(gz);
    let mut total = 0u64;
    let mut count = 0usize;
    for entry in archive
        .entries()
        .map_err(|e| HiveError::invalid(format!("not a tar archive: {e}")))?
    {
        let mut entry = entry.map_err(|e| HiveError::invalid(format!("tar entry error: {e}")))?;
        count += 1;
        if count > MAX_ENTRIES {
            return Err(HiveError::invalid("archive has too many entries"));
        }
        let name = entry
            .path()
            .map_err(|e| HiveError::invalid(e.to_string()))?
            .to_string_lossy()
            .to_string();
        let Some(path) = safe_join(dest, &name) else {
            continue;
        };
        let kind = entry.header().entry_type();
        if kind.is_dir() {
            std::fs::create_dir_all(&path).at(&path)?;
            continue;
        }
        if !kind.is_file() {
            continue; // symlinks and specials are skipped
        }
        total += entry.header().size().unwrap_or(0);
        if total > MAX_ARCHIVE_BYTES {
            return Err(HiveError::invalid("archive is too large"));
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).at(parent)?;
        }
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).at(&path)?;
        std::fs::write(&path, buf).at(&path)?;
    }
    Ok(())
}

/// Detect the archive type from the URL/file name and the magic bytes, then extract.
pub fn extract(bytes: &[u8], name_hint: &str, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest).at(dest)?;
    let lower = name_hint.to_lowercase();
    if lower.ends_with(".zip") || bytes.starts_with(b"PK\x03\x04") {
        extract_zip(bytes, dest)
    } else if lower.ends_with(".tar.gz")
        || lower.ends_with(".tgz")
        || bytes.starts_with(&[0x1f, 0x8b])
    {
        extract_tar_gz(bytes, dest)
    } else {
        Err(HiveError::invalid(
            "unsupported archive type (expected .zip or .tar.gz)",
        ))
    }
}

/// GitHub tarballs and many zips wrap everything in one top-level directory; return it.
pub fn single_root(dest: &Path) -> PathBuf {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dest)
        .map(|rd| rd.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    if entries.len() == 1 && entries[0].is_dir() {
        entries.remove(0)
    } else {
        dest.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn zip_round_trip_rejects_escapes() {
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default();
            w.start_file("repo/skills/a/SKILL.md", opts).unwrap();
            w.write_all(b"---\nname: a\ndescription: d\n---\nb")
                .unwrap();
            w.start_file("../evil.txt", opts).unwrap();
            w.write_all(b"x").unwrap();
            w.finish().unwrap();
        }
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("out");
        extract(&buf, "x.zip", &dest).unwrap();
        assert!(dest.join("repo/skills/a/SKILL.md").is_file());
        assert!(!tmp.path().join("evil.txt").exists());
        assert_eq!(single_root(&dest), dest.join("repo"));
    }

    #[test]
    fn tar_gz_round_trip() {
        let mut tar_bytes = Vec::new();
        {
            let mut builder = tar::Builder::new(&mut tar_bytes);
            let data = b"---\nname: t\ndescription: d\n---\nbody";
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, "root/t/SKILL.md", &data[..])
                .unwrap();
            builder.finish().unwrap();
        }
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(&tar_bytes).unwrap();
        let bytes = gz.finish().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        extract(&bytes, "x.tar.gz", tmp.path()).unwrap();
        assert!(tmp.path().join("root/t/SKILL.md").is_file());
        assert!(extract(b"nonsense", "x.bin", tmp.path()).is_err());
    }
}
