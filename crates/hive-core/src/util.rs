//! Small helpers: ids, time, slugs, token estimates, atomic writes, directory copies.

use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{HiveError, IoContext, Result};

/// Milliseconds since the Unix epoch.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Time-ordered unique id.
pub fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// ISO-8601 timestamp for reports.
pub fn iso_now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn iso_from_ms(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_default()
}

/// Lowercase kebab-case slug: letters, digits and single hyphens.
pub fn slugify(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut last_dash = true;
    for ch in input.chars() {
        let ch = ch.to_ascii_lowercase();
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "item".to_string()
    } else {
        trimmed
    }
}

/// Rough token estimate (≈ 1 token per 4 characters, with a word-based floor).
/// Good enough for budgets; exact counts depend on the model tokenizer.
pub fn estimate_tokens(text: &str) -> u32 {
    if text.is_empty() {
        return 0;
    }
    let chars = text.chars().count() as f64;
    let words = text.split_whitespace().count() as f64;
    ((chars / 4.0).max(words * 0.75)).ceil() as u32
}

/// Write a file atomically (temp file in the same directory, then rename).
pub fn write_atomic(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).at(parent)?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent).at(parent)?;
    std::io::Write::write_all(&mut tmp, contents).at(path)?;
    tmp.persist(path)
        .map_err(|e| HiveError::io(path, e.error))?;
    Ok(())
}

pub fn write_json_atomic<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let text = serde_json::to_string_pretty(value)?;
    write_atomic(path, text.as_bytes())
}

pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let text = std::fs::read_to_string(path).at(path)?;
    Ok(serde_json::from_str(&text)?)
}

/// Recursively copy a directory. Symlinks are followed; `.git` directories are skipped.
pub fn copy_dir(src: &Path, dst: &Path) -> Result<u64> {
    let mut bytes = 0u64;
    std::fs::create_dir_all(dst).at(dst)?;
    for entry in std::fs::read_dir(src).at(src)? {
        let entry = entry.at(src)?;
        let name = entry.file_name();
        if name == ".git" {
            continue;
        }
        let from = entry.path();
        let to = dst.join(&name);
        let meta = std::fs::metadata(&from).at(&from)?;
        if meta.is_dir() {
            bytes += copy_dir(&from, &to)?;
        } else {
            std::fs::copy(&from, &to).at(&from)?;
            bytes += meta.len();
        }
    }
    Ok(bytes)
}

/// Remove a path whether it is a file, a directory, a symlink or a junction.
pub fn remove_path(path: &Path) -> Result<()> {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return Ok(());
    };
    if meta.file_type().is_symlink() {
        #[cfg(windows)]
        {
            // Directory symlinks and junctions both report `is_symlink()`, but `is_dir()` is
            // false for symlink metadata, so ask the file type directly and use `remove_dir`,
            // which unlinks the reparse point without touching what it points at.
            use std::os::windows::fs::FileTypeExt;
            if meta.file_type().is_symlink_dir() {
                return std::fs::remove_dir(path).at(path);
            }
        }
        return std::fs::remove_file(path).at(path);
    }
    #[cfg(windows)]
    {
        if junction::exists(path).unwrap_or(false) {
            return junction::delete(path)
                .at(path)
                .and_then(|_| std::fs::remove_dir(path).at(path));
        }
    }
    if meta.is_dir() {
        std::fs::remove_dir_all(path).at(path)
    } else {
        std::fs::remove_file(path).at(path)
    }
}

/// Reject relative paths that escape their root (`..`) or are absolute.
pub fn is_safe_relative(path: &str) -> bool {
    let p = Path::new(path);
    if p.is_absolute() || path.contains('\0') {
        return false;
    }
    !p.components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::Prefix(_) | Component::RootDir
        )
    })
}

/// Total size of a directory tree in bytes.
pub fn dir_size(path: &Path) -> u64 {
    walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

/// Human readable byte size.
pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// Resolve `.` and `..` lexically without touching the file system.
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Windows gives every child of a windowed process a console of its own, so each `git` or
/// agent CLI the desktop app spawns would flash a black window over it. `CREATE_NO_WINDOW`
/// prevents that; the flag is meaningless elsewhere.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Keep a child process from opening a console window (Windows only; no-op elsewhere).
pub fn hide_console(cmd: &mut tokio::process::Command) -> &mut tokio::process::Command {
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// [`hide_console`] for a blocking `std::process::Command`.
pub fn hide_console_std(cmd: &mut std::process::Command) -> &mut std::process::Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Truncate a string to at most `max` characters, appending an ellipsis.
pub fn truncate_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_rules() {
        assert_eq!(slugify("Hello World!"), "hello-world");
        assert_eq!(slugify("--a__b--"), "a-b");
        assert_eq!(slugify("***"), "item");
        assert_eq!(slugify("pdf"), "pdf");
    }

    #[test]
    fn token_estimate_is_monotonic() {
        assert_eq!(estimate_tokens(""), 0);
        assert!(estimate_tokens("hello world") < estimate_tokens("hello world, this is longer"));
    }

    #[test]
    fn safe_relative() {
        assert!(is_safe_relative("a/b.txt"));
        assert!(!is_safe_relative("../x"));
        assert!(!is_safe_relative("/etc/passwd"));
        assert!(!is_safe_relative("a/../../b"));
    }

    #[test]
    fn atomic_write_and_copy_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("a/b/c.txt");
        write_atomic(&file, b"hi").unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "hi");
        let dst = tmp.path().join("copy");
        let bytes = copy_dir(&tmp.path().join("a"), &dst).unwrap();
        assert_eq!(bytes, 2);
        assert!(dst.join("b/c.txt").is_file());
        remove_path(&dst).unwrap();
        assert!(!dst.exists());
    }

    #[test]
    fn bytes_and_truncate() {
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(2048), "2.0 KB");
        assert_eq!(truncate_chars("abcdef", 4), "abc…");
        assert_eq!(truncate_chars("abc", 4), "abc");
    }
}
