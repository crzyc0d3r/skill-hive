use crate::library::central_repo::{self};
use crate::library::index::{self, SkillMetaFile};
use crate::library::installer::install_local;
use crate::library::skill_metadata::EmptySkillMetadata;
use crate::library::sync_metadata::{ScenarioMetaFile, ScenarioSkillMetaFile};
use std::collections::BTreeMap;
use std::fs;

use crate::test_support::open_store;

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
