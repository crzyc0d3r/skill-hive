// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::Command;

use crate::library::content_hash::hash_skill_dir;
use crate::library::installer::copy_dir;
use crate::util::error::{AppError, TargetConflictDetail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    Symlink,
    Copy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsKind {
    Unix,
    Windows,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetClass {
    Missing,
    Symlink,
    Junction,
    RealDirectory,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployOutcome {
    pub mode: &'static str,
    pub noop: bool,
}

pub fn current_os() -> OsKind {
    if cfg!(windows) {
        OsKind::Windows
    } else {
        OsKind::Unix
    }
}

pub fn is_unc_path(path: &Path) -> bool {
    let text = path.to_string_lossy();
    text.starts_with(r"\\") || text.starts_with("//")
}

/// Fallback chain the reference uses: symlink, then a Windows junction, then a copy.
/// UNC targets skip link creation and copy.
pub fn fallback_chain(preference: SyncMode, unc: bool, os: OsKind) -> Vec<&'static str> {
    match preference {
        SyncMode::Copy => vec!["copy"],
        SyncMode::Symlink if unc => vec!["copy"],
        SyncMode::Symlink if os == OsKind::Windows => vec!["symlink", "junction", "copy"],
        SyncMode::Symlink => vec!["symlink", "copy"],
    }
}

pub trait LinkBackend {
    fn symlink(&self, source: &Path, target: &Path) -> io::Result<()>;
    fn junction(&self, source: &Path, target: &Path) -> io::Result<()>;
    fn copy_tree(&self, source: &Path, target: &Path) -> io::Result<()>;
}

pub struct NativeBackend;

impl LinkBackend for NativeBackend {
    fn symlink(&self, source: &Path, target: &Path) -> io::Result<()> {
        native_symlink(source, target)
    }

    fn junction(&self, source: &Path, target: &Path) -> io::Result<()> {
        native_junction(source, target)
    }

    fn copy_tree(&self, source: &Path, target: &Path) -> io::Result<()> {
        if target.exists() {
            remove_target(target)?;
        }
        copy_dir(source, target)
    }
}

pub fn classify_target(path: &Path) -> TargetClass {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return TargetClass::Missing;
    };
    if meta.file_type().is_symlink() {
        return TargetClass::Symlink;
    }
    #[cfg(windows)]
    if is_junction(path) {
        return TargetClass::Junction;
    }
    if meta.is_dir() {
        TargetClass::RealDirectory
    } else {
        TargetClass::File
    }
}

pub fn deploy_skill(
    source: &Path,
    target: &Path,
    preference: SyncMode,
    os: OsKind,
    backend: &dyn LinkBackend,
) -> Result<DeployOutcome, AppError> {
    match classify_target(target) {
        TargetClass::RealDirectory | TargetClass::File => {
            return Err(conflict(target));
        }
        TargetClass::Symlink | TargetClass::Junction if is_current(source, target, preference) => {
            return Ok(DeployOutcome {
                mode: if preference == SyncMode::Copy {
                    "copy"
                } else {
                    "symlink"
                },
                noop: true,
            });
        }
        TargetClass::Missing | TargetClass::Symlink | TargetClass::Junction => {}
    }
    if matches!(
        classify_target(target),
        TargetClass::Symlink | TargetClass::Junction
    ) {
        remove_target(target).map_err(AppError::io)?;
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(AppError::io)?;
    }
    let chain = fallback_chain(preference, is_unc_path(target), os);
    let mut last_error = None;
    for step in chain {
        let result = match step {
            "symlink" => backend.symlink(source, target),
            "junction" => backend.junction(source, target),
            "copy" => backend.copy_tree(source, target),
            _ => Err(io::Error::other(format!("unknown step {step}"))),
        };
        match result {
            Ok(()) => {
                return Ok(DeployOutcome {
                    mode: step,
                    noop: false,
                });
            }
            Err(err) => {
                let _ = remove_target(target);
                last_error = Some(err);
            }
        }
    }
    Err(AppError::io(
        last_error.unwrap_or_else(|| io::Error::other("deploy failed")),
    ))
}

pub fn is_current(source: &Path, target: &Path, preference: SyncMode) -> bool {
    if preference == SyncMode::Copy {
        return hash_skill_dir(source).ok() == hash_skill_dir(target).ok() && target.exists();
    }
    link_points_at(target, source)
}

pub fn link_points_at(link: &Path, source: &Path) -> bool {
    let Ok(raw) = fs::read_link(link) else {
        return false;
    };
    let absolute = if raw.is_absolute() {
        raw
    } else {
        link.parent().unwrap_or(Path::new(".")).join(raw)
    };
    paths_equal(&absolute, source)
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

/// Remove a link or a copy this app created. A real directory is a conflict
/// and is left untouched. When `still_claimed` is set, the path stays because
/// another tool (amp/replit or cline/warp, or a second preset) still wants it.
pub fn undeploy_target(target: &Path, still_claimed: bool) -> Result<bool, AppError> {
    if still_claimed || !target.exists() && fs::symlink_metadata(target).is_err() {
        return Ok(false);
    }
    match classify_target(target) {
        TargetClass::Missing => Ok(false),
        TargetClass::RealDirectory | TargetClass::File => Err(conflict(target)),
        TargetClass::Symlink | TargetClass::Junction => {
            remove_target(target).map_err(AppError::io)?;
            Ok(true)
        }
    }
}

pub fn remove_copy_target(target: &Path) -> Result<(), AppError> {
    match classify_target(target) {
        TargetClass::Missing => Ok(()),
        TargetClass::RealDirectory | TargetClass::Symlink | TargetClass::Junction => {
            fs::remove_dir_all(target).map_err(AppError::io)
        }
        TargetClass::File => fs::remove_file(target).map_err(AppError::io),
    }
}

fn conflict(path: &Path) -> AppError {
    let text = path.to_string_lossy().to_string();
    AppError::target_conflict(
        format!("Refused to replace {text}. The content was left untouched."),
        vec![TargetConflictDetail {
            path: text,
            reason: "foreign directory".into(),
        }],
    )
}

fn remove_target(path: &Path) -> io::Result<()> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };
    if meta.file_type().is_symlink() {
        return fs::remove_file(path);
    }
    #[cfg(windows)]
    if is_junction(path) {
        // remove_dir drops the junction itself. remove_dir_all can walk into the target.
        return fs::remove_dir(path);
    }
    if meta.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// Rewrite symlink targets that still point into `from` so they point into `to`.
pub fn repoint_symlinks(links: &[PathBuf], from: &Path, to: &Path) -> io::Result<usize> {
    let mut rewritten = 0;
    for link in links {
        let Ok(raw) = fs::read_link(link) else {
            continue;
        };
        let absolute = if raw.is_absolute() {
            raw
        } else if let Some(parent) = link.parent() {
            parent.join(&raw)
        } else {
            raw.clone()
        };
        let Ok(suffix) = absolute.strip_prefix(from) else {
            continue;
        };
        let next = to.join(suffix);
        remove_target(link)?;
        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent)?;
        }
        native_symlink(&next, link)?;
        rewritten += 1;
    }
    Ok(rewritten)
}

#[cfg(unix)]
fn native_symlink(source: &Path, target: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(source, target)
}

#[cfg(windows)]
fn native_symlink(source: &Path, target: &Path) -> io::Result<()> {
    std::os::windows::fs::symlink_dir(source, target)
}

#[cfg(windows)]
fn native_junction(source: &Path, target: &Path) -> io::Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let status = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(target)
        .arg(source)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other("mklink /J failed"))
    }
}

#[cfg(not(windows))]
fn native_junction(_source: &Path, _target: &Path) -> io::Result<()> {
    Err(io::Error::other("junctions are a Windows link"))
}

#[cfg(windows)]
fn is_junction(path: &Path) -> bool {
    use std::os::windows::fs::MetadataExt;
    // Junctions are mount-point reparse points. Rust reports only IO_REPARSE_TAG_SYMLINK
    // as a symlink, so a junction would otherwise look like a real directory and the
    // next deploy would refuse it with TARGET_CONFLICT.
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    let Ok(meta) = fs::symlink_metadata(path) else {
        return false;
    };
    meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
        && !meta.file_type().is_symlink()
        && meta.is_dir()
}

/// Tools that deploy into one shared directory. Undeploying one of them must
/// leave the directory in place while the other still claims it.
pub fn shared_directory_groups() -> Vec<Vec<&'static str>> {
    vec![vec!["amp", "replit"], vec!["cline", "warp"]]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Recording {
        symlink: AtomicUsize,
        junction: AtomicUsize,
        copy: AtomicUsize,
        fail_symlink: bool,
        fail_junction: bool,
    }

    impl LinkBackend for Recording {
        fn symlink(&self, source: &Path, target: &Path) -> io::Result<()> {
            self.symlink.fetch_add(1, Ordering::SeqCst);
            if self.fail_symlink {
                return Err(io::Error::other("symlink denied"));
            }
            native_symlink(source, target)
        }
        fn junction(&self, _source: &Path, _target: &Path) -> io::Result<()> {
            self.junction.fetch_add(1, Ordering::SeqCst);
            if self.fail_junction {
                return Err(io::Error::other("junction denied"));
            }
            Ok(())
        }
        fn copy_tree(&self, source: &Path, target: &Path) -> io::Result<()> {
            self.copy.fetch_add(1, Ordering::SeqCst);
            if is_unc_path(target) {
                return Ok(());
            }
            NativeBackend.copy_tree(source, target)
        }
    }

    #[test]
    fn unc_fallback_copies_without_trying_a_link() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("skill");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), "---\nname: demo\n---\n").unwrap();
        let backend = Recording {
            symlink: AtomicUsize::new(0),
            junction: AtomicUsize::new(0),
            copy: AtomicUsize::new(0),
            fail_symlink: true,
            fail_junction: true,
        };
        let target = PathBuf::from(r"\\server\share\skills\demo");
        let outcome = deploy_skill(
            &source,
            &target,
            SyncMode::Symlink,
            OsKind::Windows,
            &backend,
        )
        .unwrap();
        assert_eq!(outcome.mode, "copy");
        assert_eq!(backend.symlink.load(Ordering::SeqCst), 0);
        assert_eq!(backend.junction.load(Ordering::SeqCst), 0);
        assert_eq!(backend.copy.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn symlink_then_junction_then_copy() {
        assert_eq!(
            fallback_chain(SyncMode::Symlink, false, OsKind::Windows),
            vec!["symlink", "junction", "copy"]
        );
    }

    #[test]
    fn repeated_symlink_deploy_is_a_noop_and_a_real_directory_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("center").join("demo");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), "hello").unwrap();
        let target = tmp.path().join("agent").join("demo");
        let backend = NativeBackend;
        let first =
            deploy_skill(&source, &target, SyncMode::Symlink, current_os(), &backend).unwrap();
        assert!(!first.noop);
        let second =
            deploy_skill(&source, &target, SyncMode::Symlink, current_os(), &backend).unwrap();
        assert!(second.noop);
        let foreign = tmp.path().join("foreign");
        fs::create_dir_all(&foreign).unwrap();
        fs::write(foreign.join("keep.txt"), "mine").unwrap();
        let err =
            deploy_skill(&source, &foreign, SyncMode::Symlink, current_os(), &backend).unwrap_err();
        assert!(matches!(
            err.kind,
            crate::util::error::ErrorKind::TargetConflict
        ));
        assert!(err.message.contains(foreign.to_str().unwrap()));
        assert!(err.message.contains("left untouched"));
        assert_eq!(
            fs::read_to_string(foreign.join("keep.txt")).unwrap(),
            "mine"
        );
        assert!(err.details.expect("conflicts").conflicts.len() == 1);
    }

    #[cfg(windows)]
    #[test]
    fn junction_path_is_exercised() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("src");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), "via junction").unwrap();
        let target = tmp.path().join("linked");
        native_junction(&source, &target).unwrap();
        assert_eq!(
            fs::read_to_string(target.join("SKILL.md")).unwrap(),
            "via junction"
        );
    }
}
