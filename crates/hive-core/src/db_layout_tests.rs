use crate::db::migrations::run_migrations;
use crate::db::skill_store::SkillStore;
use crate::library::central_repo::{self, ensure_central_repo};

#[test]

fn hive_db_migrates_to_v12_and_layout_is_created() {
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
    assert_eq!(version, 12, "hive.db must migrate to schema v12");
    run_migrations(&conn).unwrap();
    let again: u32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(again, 12);

    let store = SkillStore::new(&db_path).unwrap();
    assert_eq!(
        store.get_setting("theme").unwrap().as_deref(),
        Some("dark"),
        "theme survives reopening the database"
    );

    central_repo::set_test_base_dir_override(None);
}
