use crate::deploy::sync_engine::SyncMode;
use crate::library::central_repo::{self};
use crate::library::workspace::{
    delete_managed_target, import_agent_local_skill_to_center, list_agent_skills,
    list_agent_skills_in, pull_center_to_agent, repair_stranded_targets, Adopt, WorkspaceStatus,
};
use std::fs;
use std::path::PathBuf;

use crate::test_support::open_store;

#[test]
fn workspace_upload_adopts_and_pull_returns_to_in_sync() {
    let _guard = central_repo::test_base_dir_lock();
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().join("base");
    let store = open_store(&base);
    let skills = base.join("skills");
    let agent = tmp.path().join("codex").join("skills");
    fs::create_dir_all(&agent).unwrap();
    let local = agent.join("notes");
    fs::create_dir_all(&local).unwrap();
    fs::write(local.join("SKILL.md"), "---\nname: notes\n---\nv1\n").unwrap();
    let listed = list_agent_skills(&store, "codex", &agent).unwrap();
    assert_eq!(listed[0].status, WorkspaceStatus::LocalOnly);
    let err = import_agent_local_skill_to_center(&store, &skills, &local, None).unwrap_err();
    assert!(err.to_string().contains("UserConfirmed"));
    let id =
        import_agent_local_skill_to_center(&store, &skills, &local, Some(Adopt::UserConfirmed))
            .unwrap();
    let listed = list_agent_skills(&store, "codex", &agent).unwrap();
    assert_eq!(listed[0].status, WorkspaceStatus::InSync);
    fs::write(
        PathBuf::from(&store.get_skill_by_id(&id).unwrap().unwrap().central_path).join("SKILL.md"),
        "---\nname: notes\n---\nv2\n",
    )
    .unwrap();
    // A symlink follows the center, so copy-mode is what can drift.
    // Windows DeleteFile refuses a directory symlink; RemoveDirectory does not.
    crate::deploy::sync_engine::remove_deployed_link(&local).unwrap();
    let center = store.get_skill_by_id(&id).unwrap().unwrap().central_path;
    crate::library::installer::copy_dir(std::path::Path::new(&center), &local).unwrap();
    fs::write(local.join("SKILL.md"), "---\nname: notes\n---\nv1\n").unwrap();
    let listed = list_agent_skills(&store, "codex", &agent).unwrap();
    assert_eq!(listed[0].status, WorkspaceStatus::CenterChanged);
    store
        .insert_target(&crate::db::skill_store::SkillTargetRecord {
            id: format!("{id}:codex"),
            skill_id: id.clone(),
            tool: "codex".into(),
            target_path: local.to_string_lossy().to_string(),
            mode: "copy".into(),
            status: "ok".into(),
            synced_at: None,
            last_error: None,
            source_hash: None,
        })
        .unwrap();
    let status = pull_center_to_agent(&store, &id, "codex", &agent, SyncMode::Copy).unwrap();
    assert_eq!(status, WorkspaceStatus::InSync);
    let err = delete_managed_target(&store, &id, &local).unwrap_err();
    assert!(err.to_string().contains("refusing to delete managed skill"));
    assert!(local.join("SKILL.md").exists());
    central_repo::set_test_base_dir_override(None);
}

#[test]
fn stranded_target_is_relinked_to_the_center_copy() {
    let _guard = central_repo::test_base_dir_lock();
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().join("base");
    let store = open_store(&base);
    let skills = base.join("skills");
    let agent = tmp.path().join("codex").join("skills");
    fs::create_dir_all(&agent).unwrap();
    let local = agent.join("notes");
    fs::create_dir_all(&local).unwrap();
    fs::write(local.join("SKILL.md"), "---\nname: notes\n---\nv1\n").unwrap();
    let id =
        import_agent_local_skill_to_center(&store, &skills, &local, Some(Adopt::UserConfirmed))
            .unwrap();
    crate::deploy::sync_engine::remove_deployed_link(&local).unwrap();
    assert!(!local.exists());
    store
        .insert_target(&crate::db::skill_store::SkillTargetRecord {
            id: format!("{id}:codex"),
            skill_id: id,
            tool: "codex".into(),
            target_path: local.to_string_lossy().to_string(),
            mode: "symlink".into(),
            status: "missing".into(),
            synced_at: None,
            last_error: None,
            source_hash: None,
        })
        .unwrap();
    assert_eq!(repair_stranded_targets(&store, "codex", &agent).unwrap(), 1);
    assert!(local.join("SKILL.md").is_file());
    assert_eq!(repair_stranded_targets(&store, "codex", &agent).unwrap(), 0);
    central_repo::set_test_base_dir_override(None);
}

fn write_named(dir: &std::path::Path, name: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: shared\n---\n{name}\n"),
    )
    .unwrap();
}

fn link_skill(target: &std::path::Path, link: &std::path::Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
    #[cfg(windows)]
    {
        if std::os::windows::fs::symlink_dir(target, link).is_err() {
            let status = std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(link)
                .arg(target)
                .status()
                .unwrap();
            assert!(status.success(), "junction was not created");
        }
    }
}

#[test]
fn the_same_skill_in_several_agent_dirs_and_a_link_is_one_card() {
    let tmp = tempfile::tempdir().unwrap();
    let store = crate::db::skill_store::SkillStore::new(&tmp.path().join("hive.db")).unwrap();
    let claude = tmp.path().join("claude").join("skills");
    let codex = tmp.path().join("codex").join("skills");
    let cursor = tmp.path().join("cursor").join("skills");
    let wind = tmp.path().join("windsurf").join("skills");
    write_named(&claude.join("autopilot"), "autopilot");
    write_named(&codex.join("autopilot"), "autopilot");
    write_named(&cursor.join("nested").join("autopilot"), "autopilot");
    fs::create_dir_all(&wind).unwrap();
    link_skill(&claude.join("autopilot"), &wind.join("autopilot"));
    let rows = list_agent_skills_in(&store, "claude_code", &[claude, codex, cursor, wind]).unwrap();
    assert_eq!(rows.len(), 1, "copies and the link stay one card");
    assert_eq!(rows[0].name, "autopilot");
    assert_eq!(rows[0].status, WorkspaceStatus::LocalOnly);
    assert_eq!(rows[0].locations.len(), 4, "{:?}", rows[0].locations);
}
