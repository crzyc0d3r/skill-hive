use crate::db::skill_store::SkillStore;
use crate::library::central_repo::{self, ensure_central_repo};
use crate::util::file_watcher::start_file_watcher;
use crate::util::notifier::RecordingNotifier;
use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};

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
