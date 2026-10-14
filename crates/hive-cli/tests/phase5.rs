use std::fs;

use hive_cli::execute_isolated;
use hive_core::providers::{RunMode, ScriptedProvider};
use hive_core::testing::{run_plan, Plan};
use serde_json::Value;

#[test]
fn replay_benchmark_matches_the_library_run() {
    let tmp = tempfile::tempdir().unwrap();
    let state = tmp.path().join("state");
    let config = tmp.path().join("config");
    fs::create_dir_all(&config).unwrap();
    let skill = tmp.path().join("fixture-skill");
    let save = execute_isolated(
        &state,
        &config,
        vec![
            "skill-hive-cli".into(),
            "--json".into(),
            "--skills-root".into(),
            tmp.path().join("skills").to_string_lossy().into_owned(),
            "test".into(),
            "save-suite".into(),
            "--skill".into(),
            skill.to_string_lossy().into_owned(),
        ],
    );
    assert_eq!(save.code, 0, "{}", save.stdout);
    let replay = execute_isolated(
        &state,
        &config,
        vec![
            "skill-hive-cli".into(),
            "--json".into(),
            "--skills-root".into(),
            tmp.path().join("skills").to_string_lossy().into_owned(),
            "test".into(),
            "run".into(),
            "--skill".into(),
            skill.to_string_lossy().into_owned(),
            "--mode".into(),
            "replay".into(),
        ],
    );
    assert_eq!(replay.code, 0, "{}", replay.stdout);
    let base = hive_core::library::central_repo::external_base_dir(&tmp.path().join("skills"));
    let cli: Value = serde_json::from_str(
        &fs::read_to_string(
            base.join("test-runs")
                .join("workspace")
                .join("iteration-1")
                .join("benchmark.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let workspace = tmp.path().join("library-workspace");
    let plan = Plan {
        skill_dir: skill,
        workspace: workspace.clone(),
        db_path: tmp.path().join("library.db"),
        mode: RunMode::Live,
        seed: 4,
        repetitions: 1,
        budget_usd: None,
        cancel_after: None,
        crash_after: None,
        recordings: tmp.path().join("recordings"),
        personal_skills: None,
        secret: String::new(),
        usd_per_cell: None,
        model: "fixture-model".into(),
        kind: "scripted".into(),
    };
    run_plan(&plan, &ScriptedProvider::new("offline")).unwrap();
    let library: Value = serde_json::from_str(
        &fs::read_to_string(workspace.join("iteration-1").join("benchmark.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(cli["run_summary"], library["run_summary"]);
}
