use crate::library::central_repo::{self, sidecar_read_dir};
use std::fs;

#[test]
fn sidecar_reader_falls_back_to_skills_manager_name() {
    let _guard = central_repo::test_base_dir_lock();
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().join("base");
    central_repo::set_test_base_dir_override(Some(base.clone()));
    fs::create_dir_all(base.join("skills").join(".skills-manager")).unwrap();
    let found = sidecar_read_dir();
    assert!(
        found.ends_with(".skills-manager"),
        "expected legacy sidecar, got {}",
        found.display()
    );
    fs::create_dir_all(base.join("skills").join(".skill-hive")).unwrap();
    let modern = sidecar_read_dir();
    assert!(modern.ends_with(".skill-hive"));
    central_repo::set_test_base_dir_override(None);
}
