use std::fs;
use std::time::{Duration, Instant};

use crate::db::migrations::run_migrations;
use crate::db::skill_store::SkillStore;
use crate::library::central_repo::{self, ensure_central_repo, sidecar_read_dir};
use crate::util::file_watcher::start_file_watcher;
use crate::util::notifier::RecordingNotifier;
use std::sync::Arc;

#[test]
fn hive_db_migrates_to_v8_and_layout_is_created() {
    let _guard = central_repo::test_base_dir_lock();
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().join("base");
    central_repo::set_test_base_dir_override(Some(base.clone()));

    ensure_central_repo(false).unwrap();

    assert!(base.join("skills").is_dir());
    assert!(base.join("skills").join(".skill-hive").is_dir());
    assert!(base.join("cache").join("repos").is_dir());
    assert!(base.join("logs").is_dir());
    assert!(base.join(".skill-hive.lock").is_file());

    let db_path = base.join("hive.db");
    let store = SkillStore::new(&db_path).unwrap();
    store.set_setting("theme", "dark").unwrap();
    drop(store);

    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let version: u32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 8, "hive.db must migrate to schema v8");
    run_migrations(&conn).unwrap();
    let again: u32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(again, 8);

    let store = SkillStore::new(&db_path).unwrap();
    assert_eq!(
        store.get_setting("theme").unwrap().as_deref(),
        Some("dark"),
        "theme survives reopening the database"
    );

    central_repo::set_test_base_dir_override(None);
}

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

#[test]
fn file_watcher_emits_app_files_changed() {
    let _guard = central_repo::test_base_dir_lock();
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().join("base");
    central_repo::set_test_base_dir_override(Some(base.clone()));
    ensure_central_repo(false).unwrap();
    let db_path = base.join("hive.db");
    let store = Arc::new(SkillStore::new(&db_path).unwrap());
    let notifier = Arc::new(RecordingNotifier::default());
    start_file_watcher(notifier.clone(), store);

    let before = notifier.events.lock().unwrap().len();
    fs::create_dir_all(base.join("skills").join("from-outside")).unwrap();
    let start = Instant::now();
    let mut saw = false;
    while start.elapsed() < Duration::from_secs(4) {
        let events = notifier.events.lock().unwrap();
        if events.len() > before && events.iter().any(|(name, _)| name == "app-files-changed") {
            saw = true;
            break;
        }
        drop(events);
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(saw, "watcher should emit app-files-changed within 4s");
    central_repo::set_test_base_dir_override(None);
}
