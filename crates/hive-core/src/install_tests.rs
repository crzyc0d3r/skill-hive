use crate::library::central_repo::{self};
use crate::library::git_fetcher::{
    cancel_git_preview, confirm_git_install, parse_git_input, preview_local_repo, resolve_tree_url,
};
use crate::library::installer::archive_and_directory_hash_equal;
use std::fs;
use std::sync::atomic::AtomicBool;

use crate::test_support::open_store;

#[test]
fn archive_hash_matches_unpacked_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("demo");
    fs::create_dir_all(dir.join("refs")).unwrap();
    fs::write(dir.join("SKILL.md"), "---\nname: demo\n---\nbody\n").unwrap();
    fs::write(dir.join("refs").join("a.txt"), "alpha").unwrap();
    assert!(archive_and_directory_hash_equal(&dir).unwrap());
}

#[test]
fn slash_branch_resolves_offline_and_online() {
    let url = "https://github.com/acme/skills/tree/feature/slash/tools/demo";
    let refs = vec!["main".into(), "feature".into(), "feature/slash".into()];
    let offline = parse_git_input(url, Some(&refs)).unwrap();
    assert_eq!(offline.branch.as_deref(), Some("feature/slash"));
    assert_eq!(offline.subpath.as_deref(), Some("tools/demo"));

    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    fs::create_dir_all(repo.join("tools").join("demo")).unwrap();
    fs::write(
        repo.join("tools").join("demo").join("SKILL.md"),
        "---\nname: demo\n---\n",
    )
    .unwrap();
    let git = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(&repo)
            .status()
            .unwrap();
        assert!(status.success());
    };
    git(&["init", "-b", "main"]);
    git(&["config", "user.email", "dev@example.com"]);
    git(&["config", "user.name", "Dev"]);
    git(&["add", "."]);
    git(&["commit", "-m", "init"]);
    git(&["checkout", "-b", "feature/slash"]);
    let online = resolve_tree_url(url, Some(&repo)).unwrap();
    assert_eq!(online.branch.as_deref(), Some("feature/slash"));
    assert_eq!(online.subpath.as_deref(), Some("tools/demo"));
}

#[test]
fn cancelled_preview_removes_its_temp_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    fs::create_dir_all(repo.join("nested")).unwrap();
    fs::write(
        repo.join("nested").join("SKILL.md"),
        "---\nname: nested\n---\n",
    )
    .unwrap();
    let cancel = AtomicBool::new(true);
    let err = preview_local_repo(&repo, None, &cancel).unwrap_err();
    assert!(err.to_string().to_ascii_lowercase().contains("cancelled"));
    let cancel = AtomicBool::new(false);
    let preview = preview_local_repo(&repo, None, &cancel).unwrap();
    assert!(preview.temp_dir.exists());
    assert!(cancel_git_preview(&preview.id).unwrap());
    assert!(!preview.temp_dir.exists());
}

#[test]
fn git_preview_rename_imports_the_selected_skill() {
    let _guard = central_repo::test_base_dir_lock();
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().join("base");
    let store = open_store(&base);
    let repo = tmp.path().join("repo");
    fs::create_dir_all(repo.join("tools").join("demo")).unwrap();
    fs::write(
        repo.join("tools").join("demo").join("SKILL.md"),
        "---\nname: demo\n---\n",
    )
    .unwrap();
    let cancel = AtomicBool::new(false);
    let preview = preview_local_repo(&repo, Some("tools/demo"), &cancel).unwrap();
    let relative = preview.skills[0].relative_dir.clone();
    let names = confirm_git_install(
        &store,
        &base.join("skills"),
        &preview.id,
        &[(relative, Some("Renamed Demo".into()))],
    )
    .unwrap();
    assert_eq!(names, vec!["renamed-demo".to_string()]);
    central_repo::set_test_base_dir_override(None);
}
