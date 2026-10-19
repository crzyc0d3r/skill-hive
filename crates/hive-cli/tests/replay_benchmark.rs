use std::fs;

use hive_cli::execute_isolated;
use hive_core::providers::{AnthropicProvider, RunMode, SecretString};
use hive_core::testing::{run_plan, Plan};
use serde_json::Value;

#[path = "wire.rs"]
mod wire;

#[test]
fn replay_benchmark_matches_the_library_run() {
    let tmp = tempfile::tempdir().unwrap();
    let state = tmp.path().join("state");
    let config = tmp.path().join("config");
    fs::create_dir_all(&config).unwrap();
    let skill = tmp.path().join("fixture-skill");
    let skills = tmp.path().join("skills");
    let save = execute_isolated(
        &state,
        &config,
        vec![
            "skill-hive-cli".into(),
            "--json".into(),
            "--skills-root".into(),
            skills.to_string_lossy().into_owned(),
            "test".into(),
            "save-suite".into(),
            "--skill".into(),
            skill.to_string_lossy().into_owned(),
        ],
    );
    assert_eq!(save.code, 0, "{}", save.stdout);
    let mock = wire::spawn_wire();
    let base_url = format!("http://127.0.0.1:{}", mock.port);
    for args in [
        vec![
            "providers".to_string(),
            "add".into(),
            "--id".into(),
            "runner".into(),
            "--name".into(),
            "Runner".into(),
            "--kind".into(),
            "anthropic".into(),
            "--base-url".into(),
            base_url.clone(),
        ],
        vec![
            "providers".into(),
            "set-secret".into(),
            "--id".into(),
            "runner".into(),
            "--secret".into(),
            "sk-replay".into(),
        ],
    ] {
        let mut full = vec![
            "skill-hive-cli".into(),
            "--json".into(),
            "--skills-root".into(),
            skills.to_string_lossy().into_owned(),
        ];
        full.extend(args);
        let output = execute_isolated(&state, &config, full);
        assert_eq!(output.code, 0, "{}", output.stdout);
    }
    let run = execute_isolated(
        &state,
        &config,
        vec![
            "skill-hive-cli".into(),
            "--json".into(),
            "--skills-root".into(),
            skills.to_string_lossy().into_owned(),
            "test".into(),
            "run".into(),
            "--skill".into(),
            skill.to_string_lossy().into_owned(),
            "--mode".into(),
            "offline".into(),
            "--repetitions".into(),
            "1".into(),
        ],
    );
    assert_eq!(run.code, 0, "{}", run.stdout);
    let ran: Value = serde_json::from_str(run.stdout.trim()).unwrap();
    let replay = execute_isolated(
        &state,
        &config,
        vec![
            "skill-hive-cli".into(),
            "--json".into(),
            "--skills-root".into(),
            skills.to_string_lossy().into_owned(),
            "test".into(),
            "run".into(),
            "--skill".into(),
            skill.to_string_lossy().into_owned(),
            "--mode".into(),
            "replay".into(),
        ],
    );
    assert_eq!(replay.code, 0, "{}", replay.stdout);
    let replayed: Value = serde_json::from_str(replay.stdout.trim()).unwrap();
    assert_eq!(replayed["data"]["run_id"], ran["data"]["run_id"]);
    let base = hive_core::library::central_repo::external_base_dir(&skills);
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
        seed: 7,
        repetitions: 1,
        budget_usd: None,
        cancel_after: None,
        crash_after: None,
        recordings: tmp.path().join("recordings"),
        personal_skills: None,
        secret: "sk-replay".into(),
        usd_per_cell: None,
        model: "claude-sonnet-4".into(),
        kind: "anthropic".into(),
    };
    let provider = AnthropicProvider::new("library", base_url, SecretString::new("sk-replay"));
    run_plan(&plan, &provider).unwrap();
    let library: Value = serde_json::from_str(
        &fs::read_to_string(workspace.join("iteration-1").join("benchmark.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        cli["run_summary"]["delta"]["pass_rate"],
        library["run_summary"]["delta"]["pass_rate"]
    );
}
