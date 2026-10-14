use crate::deploy::scenario_service::{add_membership, apply_preset, sync_one, DeployIntent};
use crate::deploy::sync_engine::{repoint_symlinks, undeploy_target, NativeBackend, SyncMode};
use crate::library::central_repo::{self};
use crate::library::installer::install_local;
use crate::library::scanner::{import_all, scan_roots};
use std::collections::HashMap;
use std::fs;

use crate::test_support::open_store;

#[test]
fn install_deploy_preset_undeploy_scan_and_import() {
    let _guard = central_repo::test_base_dir_lock();
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().join("base");
    let store = open_store(&base);
    let skills = base.join("skills");
    let source = tmp.path().join("incoming").join("alpha");
    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("SKILL.md"),
        "---\nname: Alpha\ndescription: one\n---\n",
    )
    .unwrap();
    let installed = install_local(&store, &skills, &source).unwrap();

    let claude = tmp.path().join("agents").join("claude");
    let codex = tmp.path().join("agents").join("codex");
    fs::create_dir_all(&claude).unwrap();
    fs::create_dir_all(&codex).unwrap();
    let mut roots = HashMap::new();
    roots.insert("claude_code".into(), claude.clone());
    roots.insert("codex".into(), codex.clone());

    store
        .insert_scenario(&crate::db::skill_store::ScenarioRecord {
            id: "research".into(),
            name: "Research".into(),
            description: None,
            icon: None,
            sort_order: 0,
            created_at: 0,
            updated_at: 0,
        })
        .unwrap();
    add_membership(
        &store,
        "research",
        &installed.record.id,
        &["claude_code".into(), "codex".into()],
    )
    .unwrap();
    let before = fs::read_dir(&claude).unwrap().count();
    assert_eq!(before, 0, "membership edits must not deploy");
    apply_preset(
        &store,
        "research",
        &roots,
        SyncMode::Symlink,
        crate::deploy::sync_engine::current_os(),
    )
    .unwrap();
    assert!(claude.join("alpha").exists() || claude.join("Alpha").exists());
    let deployed = claude.join(&installed.record.name);
    assert!(deployed.exists(), "deployed {}", deployed.display());

    sync_one(
        &store,
        &crate::deploy::scenario_service::SyncOne {
            skill_id: installed.record.id.clone(),
            tool: "claude_code".into(),
            root: claude.clone(),
            intent: DeployIntent::Remove,
            mode: SyncMode::Symlink,
            os: crate::deploy::sync_engine::current_os(),
        },
        &NativeBackend,
    )
    .unwrap();
    assert!(!deployed.exists());

    let loose = tmp.path().join("scan-root").join("beta");
    fs::create_dir_all(&loose).unwrap();
    fs::write(loose.join("SKILL.md"), "---\nname: Beta\n---\n").unwrap();
    let scan_root = tmp.path().join("scan-root");
    let scanned = scan_roots(&store, &[("cursor", scan_root.as_path())]).unwrap();
    let imported = import_all(&store, &skills, &scanned).unwrap();
    assert_eq!(imported, vec!["beta".to_string()]);
    assert_eq!(store.get_all_skills().unwrap().len(), 2);
    central_repo::set_test_base_dir_override(None);
}

#[test]
fn preset_b_after_a_unsyncs_obsolete_targets_and_keeps_shared_ones() {
    let _guard = central_repo::test_base_dir_lock();
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().join("base");
    let store = open_store(&base);
    let skills = base.join("skills");
    let mut ids = Vec::new();
    for name in ["one", "two"] {
        let dir = tmp.path().join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), format!("---\nname: {name}\n---\n")).unwrap();
        ids.push(install_local(&store, &skills, &dir).unwrap().record.id);
    }
    let agent = tmp.path().join("claude");
    fs::create_dir_all(&agent).unwrap();
    let roots = HashMap::from([("claude_code".into(), agent.clone())]);
    for (id, preset) in [("a", "A"), ("b", "B")] {
        store
            .insert_scenario(&crate::db::skill_store::ScenarioRecord {
                id: id.into(),
                name: preset.into(),
                description: None,
                icon: None,
                sort_order: 0,
                created_at: 0,
                updated_at: 0,
            })
            .unwrap();
    }
    add_membership(&store, "a", &ids[0], &["claude_code".into()]).unwrap();
    add_membership(&store, "a", &ids[1], &["claude_code".into()]).unwrap();
    add_membership(&store, "b", &ids[1], &["claude_code".into()]).unwrap();
    apply_preset(
        &store,
        "a",
        &roots,
        SyncMode::Symlink,
        crate::deploy::sync_engine::current_os(),
    )
    .unwrap();
    assert!(agent.join("one").exists());
    assert!(agent.join("two").exists());
    apply_preset(
        &store,
        "b",
        &roots,
        SyncMode::Symlink,
        crate::deploy::sync_engine::current_os(),
    )
    .unwrap();
    assert!(!agent.join("one").exists());
    assert!(agent.join("two").exists());
    central_repo::set_test_base_dir_override(None);
}

#[test]
fn shared_directory_undeploy_keeps_the_path_while_another_tool_claims_it() {
    let _guard = central_repo::test_base_dir_lock();
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().join("base");
    let store = open_store(&base);
    let skills = base.join("skills");
    let dir = tmp.path().join("shared-skill");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("SKILL.md"), "---\nname: shared\n---\n").unwrap();
    let installed = install_local(&store, &skills, &dir).unwrap();
    let shared = tmp.path().join("agents-skills");
    fs::create_dir_all(&shared).unwrap();
    let roots = HashMap::from([
        ("amp".into(), shared.clone()),
        ("replit".into(), shared.clone()),
    ]);
    store
        .insert_scenario(&crate::db::skill_store::ScenarioRecord {
            id: "both".into(),
            name: "Both".into(),
            description: None,
            icon: None,
            sort_order: 0,
            created_at: 0,
            updated_at: 0,
        })
        .unwrap();
    add_membership(
        &store,
        "both",
        &installed.record.id,
        &["amp".into(), "replit".into()],
    )
    .unwrap();
    apply_preset(
        &store,
        "both",
        &roots,
        SyncMode::Symlink,
        crate::deploy::sync_engine::current_os(),
    )
    .unwrap();
    let link = shared.join("shared");
    assert!(link.exists());
    sync_one(
        &store,
        &crate::deploy::scenario_service::SyncOne {
            skill_id: installed.record.id.clone(),
            tool: "amp".into(),
            root: shared.clone(),
            intent: DeployIntent::Remove,
            mode: SyncMode::Symlink,
            os: crate::deploy::sync_engine::current_os(),
        },
        &NativeBackend,
    )
    .unwrap();
    assert!(link.exists(), "replit still claims the shared directory");
    sync_one(
        &store,
        &crate::deploy::scenario_service::SyncOne {
            skill_id: installed.record.id.clone(),
            tool: "replit".into(),
            root: shared.clone(),
            intent: DeployIntent::Remove,
            mode: SyncMode::Symlink,
            os: crate::deploy::sync_engine::current_os(),
        },
        &NativeBackend,
    )
    .unwrap();
    assert!(!link.exists());
    central_repo::set_test_base_dir_override(None);
}

#[test]
fn foreign_directory_conflict_names_the_path() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("SKILL.md"), "x").unwrap();
    let foreign = tmp.path().join("real");
    fs::create_dir_all(&foreign).unwrap();
    fs::write(foreign.join("data.txt"), "keep").unwrap();
    let err = undeploy_target(&foreign, false).unwrap_err();
    assert!(err.message.contains(&foreign.to_string_lossy().to_string()));
    assert!(err.message.contains("left untouched"));
    assert_eq!(
        fs::read_to_string(foreign.join("data.txt")).unwrap(),
        "keep"
    );
}

#[test]
fn next_launch_moves_the_library_and_symlinks_still_resolve() {
    let tmp = tempfile::tempdir().unwrap();
    let source_base = tmp.path().join("old");
    let skill = source_base.join("skills").join("demo");
    fs::create_dir_all(&skill).unwrap();
    fs::write(skill.join("SKILL.md"), "moved").unwrap();
    let link_dir = tmp.path().join("codex");
    fs::create_dir_all(&link_dir).unwrap();
    let link = link_dir.join("demo");
    crate::deploy::sync_engine::deploy_skill(
        &skill,
        &link,
        SyncMode::Symlink,
        crate::deploy::sync_engine::current_os(),
        &NativeBackend,
    )
    .unwrap();
    let target_base = tmp.path().join("new");
    fs::rename(&source_base, &target_base).unwrap();
    let rewritten =
        repoint_symlinks(std::slice::from_ref(&link), &source_base, &target_base).unwrap();
    assert_eq!(rewritten, 1);
    assert_eq!(fs::read_to_string(link.join("SKILL.md")).unwrap(), "moved");
}
