use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use crate::db::skill_store::SkillStore;
use crate::deploy::scenario_service::{add_membership, apply_preset, sync_one, DeployIntent};
use crate::deploy::sync_engine::{repoint_symlinks, undeploy_target, NativeBackend, SyncMode};
use crate::deploy::tool_adapters::{CustomToolDef, ToolCategory};
use crate::deploy::tool_service::{
    add_custom_tool, builtin_catalog, global_and_project_dirs, installed_keys, reorder_tools,
    reset_tool_path, set_project_path, set_tool_path, tool_statuses, CustomAdd,
    BUILTIN_ADAPTER_COUNT,
};
use crate::library::central_repo::{self, ensure_central_repo};
use crate::library::git_fetcher::{
    cancel_git_preview, confirm_git_install, parse_git_input, preview_local_repo, resolve_tree_url,
};
use crate::library::index::{self, SkillMetaFile};
use crate::library::installer::{archive_and_directory_hash_equal, install_local};
use crate::library::scanner::{import_all, scan_roots};
use crate::library::skill_metadata::EmptySkillMetadata;
use crate::library::sync_metadata::{ScenarioMetaFile, ScenarioSkillMetaFile};
use crate::library::workspace::{
    delete_managed_target, import_agent_local_skill_to_center, list_agent_skills,
    pull_center_to_agent, Adopt, WorkspaceStatus,
};
use std::collections::{BTreeMap, HashMap};

fn open_store(base: &std::path::Path) -> SkillStore {
    central_repo::set_test_base_dir_override(Some(base.to_path_buf()));
    ensure_central_repo(false).unwrap();
    SkillStore::new(&base.join("hive.db")).unwrap()
}

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
    let store = open_store(&base);
    set_tool_path(&store, "codex", "/var/codex-skills").unwrap();
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
    assert_ne!(reset.skills_dir, "/var/codex-skills");
    central_repo::set_test_base_dir_override(None);
}

#[test]
fn skills_manager_import_reproduces_counts_and_order() {
    let _guard = central_repo::test_base_dir_lock();
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("legacy").join("skills");
    fs::create_dir_all(&source).unwrap();
    let sidecar = source.join(".skills-manager");
    let mut names = Vec::new();
    for index in 0..12 {
        let name = format!("skill-{index:02}");
        names.push(name.clone());
        let dir = source.join(&name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: {name}\n---\nbody\n"),
        )
        .unwrap();
        let meta = SkillMetaFile {
            schema_version: 1,
            id: format!("id-{index:02}"),
            name: name.clone(),
            description: None,
            source_type: "local".into(),
            source_ref: None,
            central_dirname: name,
            content_hash: None,
            tags: if index < 4 {
                vec!["research".into()]
            } else {
                vec![]
            },
            enabled: true,
        };
        let skills_meta = sidecar.join("skills");
        fs::create_dir_all(&skills_meta).unwrap();
        fs::write(
            skills_meta.join(format!("id-{index:02}.json")),
            serde_json::to_vec_pretty(&meta).unwrap(),
        )
        .unwrap();
    }
    for (order, preset) in ["Alpha", "Beta", "Gamma"].iter().enumerate() {
        let meta = ScenarioMetaFile {
            schema_version: 1,
            scenario_id: format!("preset-{order}"),
            name: (*preset).into(),
            description: None,
            icon: None,
            sort_order: order as i32,
        };
        fs::create_dir_all(sidecar.join("scenarios")).unwrap();
        fs::write(
            sidecar
                .join("scenarios")
                .join(format!("preset-{order}.json")),
            serde_json::to_vec_pretty(&meta).unwrap(),
        )
        .unwrap();
    }
    for (preset, skill_indexes) in [(0, vec![0, 1]), (1, vec![2, 3, 4]), (2, vec![5])] {
        for (position, skill_index) in skill_indexes.into_iter().enumerate() {
            let member = ScenarioSkillMetaFile {
                schema_version: 1,
                scenario_id: format!("preset-{preset}"),
                skill_id: format!("id-{skill_index:02}"),
                sort_order: position as i32,
                tools: BTreeMap::from([("claude_code".into(), true)]),
            };
            let dir = sidecar.join("memberships").join(format!("preset-{preset}"));
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join(format!("id-{skill_index:02}.json")),
                serde_json::to_vec_pretty(&member).unwrap(),
            )
            .unwrap();
        }
    }
    let base = tmp.path().join("base");
    let store = open_store(&base);
    let dest = base.join("skills");
    let report = index::import_skills_manager_library(&store, &source, &dest).unwrap();
    assert_eq!(report.skills, 12);
    assert_eq!(report.presets, 3);
    assert_eq!(report.tag_assignments, 4);
    assert_eq!(
        report.membership_order,
        vec![
            ("Alpha".into(), vec!["skill-00".into(), "skill-01".into()]),
            (
                "Beta".into(),
                vec!["skill-02".into(), "skill-03".into(), "skill-04".into()]
            ),
            ("Gamma".into(), vec!["skill-05".into()]),
        ]
    );
    central_repo::set_test_base_dir_override(None);
}

#[test]
fn sidecar_less_library_is_refused_and_rebuilt_from_the_database() {
    let _guard = central_repo::test_base_dir_lock();
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().join("base");
    let store = open_store(&base);
    let skills = base.join("skills");
    let source = tmp.path().join("one");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("SKILL.md"), "---\nname: kept\n---\n").unwrap();
    install_local(&store, &skills, &source).unwrap();
    fs::create_dir_all(skills.join("orphan")).unwrap();
    fs::write(
        skills.join("orphan").join("SKILL.md"),
        "---\nname: orphan\n---\n",
    )
    .unwrap();
    let sidecar = skills.join(".skill-hive");
    fs::remove_dir_all(&sidecar).unwrap();
    let err = index::reindex_from_metadata(&store, &skills).unwrap_err();
    assert!(err.downcast_ref::<EmptySkillMetadata>().is_some());
    assert_eq!(store.get_all_skills().unwrap().len(), 1);
    index::write_all_from_db(&store, &skills).unwrap();
    assert!(sidecar.join("skills").exists());
    assert_eq!(store.get_all_skills().unwrap().len(), 1);
    central_repo::set_test_base_dir_override(None);
}

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
