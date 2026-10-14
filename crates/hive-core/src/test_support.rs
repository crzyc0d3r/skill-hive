use crate::db::skill_store::SkillStore;
use crate::library::central_repo::{self, ensure_central_repo};

pub(crate) fn open_store(base: &std::path::Path) -> SkillStore {
    central_repo::set_test_base_dir_override(Some(base.to_path_buf()));
    ensure_central_repo(false).unwrap();
    SkillStore::new(&base.join("hive.db")).unwrap()
}
