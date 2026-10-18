use crate::db::skill_store::SkillStore;
use crate::deploy::tool_adapters::{CustomToolDef, ToolCategory};
use crate::deploy::tool_service::{
    add_custom_tool, builtin_catalog, global_and_project_dirs, installed_keys, reorder_tools,
    reset_tool_path, set_project_path, set_tool_path, tool_statuses, CustomAdd,
    BUILTIN_ADAPTER_COUNT,
};
use crate::library::central_repo::{self};
use std::fs;

use crate::test_support::open_store;

#[test]
fn fifty_four_adapters_have_global_and_project_directories() {
    let catalog = builtin_catalog();
    assert_eq!(catalog.len(), BUILTIN_ADAPTER_COUNT);
    let dirs = global_and_project_dirs();
    assert_eq!(dirs.len(), BUILTIN_ADAPTER_COUNT);
    for adapter in catalog {
        let (global, project) = dirs.get(&adapter.key).unwrap();
        assert!(!global.is_empty(), "{}", adapter.key);
        assert!(!project.is_empty(), "{}", adapter.key);
    }
}

#[test]
fn tool_status_reports_only_claude_and_codex_when_those_homes_exist() {
    let _guard = central_repo::test_base_dir_lock();
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().join("base");
    let home = tmp.path().join("home");
    fs::create_dir_all(home.join(".claude")).unwrap();
    fs::create_dir_all(home.join(".codex")).unwrap();
    let store = open_store(&base);
    let installed = installed_keys(&store, &home).unwrap();
    assert_eq!(
        installed,
        vec!["claude_code".to_string(), "codex".to_string()]
    );
    central_repo::set_test_base_dir_override(None);
}

#[test]
fn custom_agent_collision_is_skipped_and_order_persists() {
    let _guard = central_repo::test_base_dir_lock();
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().join("base");
    let store = open_store(&base);
    let skipped = add_custom_tool(
        &store,
        CustomToolDef {
            key: "claude_code".into(),
            display_name: "Not Claude".into(),
            skills_dir: tmp.path().join("custom").to_string_lossy().into_owned(),
            project_relative_skills_dir: None,
            category: ToolCategory::Coding,
        },
    )
    .unwrap();
    assert_eq!(skipped, CustomAdd::SkippedCollision);
    let added = add_custom_tool(
        &store,
        CustomToolDef {
            key: "field_notes".into(),
            display_name: "Field Notes".into(),
            skills_dir: tmp.path().join("notes").to_string_lossy().into_owned(),
            project_relative_skills_dir: Some(".notes/skills".into()),
            category: ToolCategory::Coding,
        },
    )
    .unwrap();
    assert_eq!(added, CustomAdd::Added);
    reorder_tools(
        &store,
        &["field_notes".into(), "codex".into(), "claude_code".into()],
    )
    .unwrap();
    drop(store);
    let store = SkillStore::new(&base.join("hive.db")).unwrap();
    let statuses = tool_statuses(&store, tmp.path()).unwrap();
    assert_eq!(statuses[0].key, "field_notes");
    assert!(statuses
        .iter()
        .any(|row| row.key == "field_notes" && row.has_path_override));
    central_repo::set_test_base_dir_override(None);
}

#[test]
fn path_overrides_round_trip_and_reset() {
    let _guard = central_repo::test_base_dir_lock();
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().join("base");
    let skills = tmp.path().join("codex-skills");
    let skills_text = skills.to_string_lossy().into_owned();
    let store = open_store(&base);
    set_tool_path(&store, "codex", &skills_text).unwrap();
    set_project_path(&store, "codex", "custom/skills").unwrap();
    let row = tool_statuses(&store, tmp.path())
        .unwrap()
        .into_iter()
        .find(|row| row.key == "codex")
        .unwrap();
    assert!(row.has_path_override);
    assert!(row.skills_dir.ends_with("codex-skills"));
    assert_eq!(row.project_dir, "custom/skills");
    reset_tool_path(&store, "codex").unwrap();
    let reset = tool_statuses(&store, tmp.path())
        .unwrap()
        .into_iter()
        .find(|row| row.key == "codex")
        .unwrap();
    assert!(!reset.has_path_override);
    assert_ne!(reset.skills_dir, skills_text);
    central_repo::set_test_base_dir_override(None);
}
