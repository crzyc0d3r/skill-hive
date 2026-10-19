use std::fs;
use std::path::PathBuf;

use hive_cli::{command_ids, execute_isolated, validate};

#[path = "wire.rs"]
mod wire;
use serde_json::Value;

fn run(state: &std::path::Path, config: &std::path::Path, args: &[&str]) -> (i32, Value) {
    let mut full = vec![
        "skill-hive-cli".to_string(),
        "--json".to_string(),
        "--skills-root".to_string(),
        state.join("skills").to_string_lossy().into_owned(),
    ];
    full.extend(args.iter().map(|arg| (*arg).to_string()));
    let output = execute_isolated(&state.join("state"), config, full);
    let parsed: Value = serde_json::from_str(output.stdout.trim()).unwrap_or_else(|err| {
        panic!("stdout was not JSON ({err}): {}", output.stdout);
    });
    (output.code, parsed)
}

fn data_of(parsed: &Value) -> &Value {
    parsed
        .get("data")
        .unwrap_or_else(|| panic!("missing data in {parsed}"))
}

#[test]
fn contract_suite_exercises_every_subcommand() {
    let tmp = tempfile::tempdir().unwrap();
    let state = tmp.path();
    let config = state.join("config");
    fs::create_dir_all(&config).unwrap();
    let mut seen = Vec::new();

    let (code, parsed) = run(state, &config, &["repo", "status"]);
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));

    let skill_src = state.join("incoming");
    fs::create_dir_all(&skill_src).unwrap();
    fs::write(skill_src.join("SKILL.md"), "---\nname: demo\n---\nbody\n").unwrap();
    let (code, parsed) = run(
        state,
        &config,
        &["skills", "install", "--path", skill_src.to_str().unwrap()],
    );
    assert_eq!(code, 0, "{parsed}");
    let id = data_of(&parsed)["id"].as_str().unwrap().to_string();
    seen.push(record(state, &parsed));

    for args in [
        vec!["skills", "list"],
        vec!["skills", "show", "--id", &id],
        vec!["skills", "search", "--query", "demo"],
        vec!["skills", "enable", "--id", &id],
        vec!["skills", "disable", "--id", &id],
        vec!["skills", "enable", "--id", &id],
        vec!["skills", "set-source", "--id", &id, "--source", "local"],
        vec!["skills", "check", "--id", &id],
        vec!["skills", "update", "--id", &id],
        vec!["skills", "status"],
        vec!["skills", "sync"],
        vec!["skills", "tag", "add", "--id", &id, "--name", "tools"],
        vec!["skills", "tag", "list", "--id", &id],
        vec![
            "skills", "tag", "set", "--id", &id, "--name", "tools", "--name", "kit",
        ],
        vec![
            "skills", "tag", "rename", "--from", "tools", "--to", "renamed",
        ],
        vec!["skills", "tag", "delete", "--name", "kit", "--yes"],
        vec!["skills", "tag", "remove", "--id", &id, "--name", "renamed"],
    ] {
        let (code, parsed) = run(state, &config, &args);
        assert_eq!(code, 0, "{args:?} {parsed}");
        seen.push(record(state, &parsed));
    }

    let export_to = state.join("export");
    let (code, parsed) = run(
        state,
        &config,
        &[
            "skills",
            "export",
            "--id",
            &id,
            "--to",
            export_to.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));

    let adopted = state.join("adopted.md");
    fs::write(&adopted, "---\nname: demo\n---\nadopted\n").unwrap();
    let (code, parsed) = run(
        state,
        &config,
        &[
            "skills",
            "adopt",
            "--id",
            &id,
            "--path",
            adopted.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));

    let deploy_to = state.join("deploy-target");
    let (code, parsed) = run(
        state,
        &config,
        &[
            "skills",
            "deploy",
            "--id",
            &id,
            "--to",
            deploy_to.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    let (code, parsed) = run(
        state,
        &config,
        &["skills", "undeploy", "--path", deploy_to.to_str().unwrap()],
    );
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));

    let (code, parsed) = run(state, &config, &["agents", "list"]);
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    let (code, parsed) = run(
        state,
        &config,
        &["agents", "disable", "--key", "claude_code"],
    );
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    let (code, parsed) = run(
        state,
        &config,
        &["agents", "enable", "--key", "claude_code"],
    );
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    let custom = state.join("custom-skills");
    fs::create_dir_all(&custom).unwrap();
    let (code, parsed) = run(
        state,
        &config,
        &[
            "agents",
            "add-custom",
            "--key",
            "field_notes",
            "--name",
            "Field Notes",
            "--path",
            custom.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));

    let (code, parsed) = run(state, &config, &["presets", "create", "--name", "Research"]);
    assert_eq!(code, 0, "{parsed}");
    let preset = data_of(&parsed)["id"].as_str().unwrap().to_string();
    seen.push(record(state, &parsed));
    for args in [
        vec!["presets", "list"],
        vec!["presets", "add-skill", "--id", &preset, "--skill", &id],
        vec!["presets", "show", "--id", &preset],
        vec!["presets", "preview", "--id", &preset],
        vec!["presets", "update", "--id", &preset, "--name", "Research 2"],
        vec!["presets", "apply", "--id", &preset],
        vec!["presets", "current"],
        vec!["presets", "status", "--id", &preset],
        vec!["presets", "deactivate"],
    ] {
        let (code, parsed) = run(state, &config, &args);
        assert_eq!(code, 0, "{args:?} {parsed}");
        seen.push(record(state, &parsed));
    }
    let preset_root = state.join("preset-root");
    fs::create_dir_all(&preset_root).unwrap();
    let (code, parsed) = run(
        state,
        &config,
        &["presets", "add-skill", "--id", &preset, "--skill", &id],
    );
    assert_eq!(code, 0, "{parsed}");
    let (code, parsed) = run(
        state,
        &config,
        &[
            "presets",
            "deploy",
            "--id",
            &preset,
            "--tool",
            "claude_code",
            "--root",
            preset_root.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    let (code, parsed) = run(
        state,
        &config,
        &[
            "presets",
            "undeploy",
            "--id",
            &preset,
            "--tool",
            "claude_code",
            "--root",
            preset_root.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    let (code, parsed) = run(
        state,
        &config,
        &["presets", "remove-skill", "--id", &preset, "--skill", &id],
    );
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    let (code, parsed) = run(state, &config, &["presets", "delete", "--id", &preset]);
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));

    let bare = state.join("remote.git");
    let output = std::process::Command::new("git")
        .args(["init", "--bare", bare.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let (code, parsed) = run(state, &config, &["git", "init"]);
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    let (code, parsed) = run(
        state,
        &config,
        &["git", "commit", "--message", "v1", "--tag", "snap-1"],
    );
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    let (code, parsed) = run(
        state,
        &config,
        &["git", "set-remote", "--url", bare.to_str().unwrap()],
    );
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    let (code, parsed) = run(state, &config, &["git", "push"]);
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    let (code, parsed) = run(state, &config, &["git", "status"]);
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    let (code, parsed) = run(state, &config, &["git", "pull"]);
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    let (code, parsed) = run(state, &config, &["git", "versions"]);
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    let snapshot = data_of(&parsed)["tags"][0]
        .as_str()
        .expect("snapshot tag")
        .to_string();
    let (code, parsed) = run(state, &config, &["git", "commit", "--message", "v2"]);
    assert_eq!(code, 0, "{parsed}");
    let (code, parsed) = run(state, &config, &["git", "restore", "--tag", &snapshot]);
    assert_eq!(code, 0, "{parsed}");
    assert!(data_of(&parsed)["safety"]
        .as_str()
        .unwrap()
        .starts_with("sm-v-"));
    seen.push(record(state, &parsed));
    let (code, parsed) = run(state, &config, &["git", "init"]);
    assert_eq!(code, 0, "{parsed}");
    let backup = PathBuf::from(data_of(&parsed)["path"].as_str().unwrap());
    let tagged = std::process::Command::new("git")
        .args(["tag", "sync-leaked"])
        .current_dir(&backup)
        .status()
        .unwrap();
    assert!(tagged.success());
    let (code, parsed) = run(state, &config, &["git", "prune-sync-refs"]);
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    let clone_to = state.join("clone");
    let (code, parsed) = run(
        state,
        &config,
        &[
            "git",
            "clone",
            "--url",
            bare.to_str().unwrap(),
            "--to",
            clone_to.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));

    let next = state.join("next-base");
    fs::create_dir_all(&next).unwrap();
    let (code, parsed) = run(
        state,
        &config,
        &["repo", "set-path", "--path", next.to_str().unwrap()],
    );
    assert_eq!(code, 0, "{parsed}");
    assert_eq!(data_of(&parsed)["pending"], true);
    seen.push(record(state, &parsed));
    let (code, parsed) = run(state, &config, &["repo", "reset-path"]);
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));

    let (code, parsed) = run(
        state,
        &config,
        &[
            "skills",
            "validate",
            "--path",
            skill_src.to_str().unwrap(),
            "--profile",
            "portable",
        ],
    );
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    let agent = state.join("reviewer.md");
    fs::write(
        &agent,
        "---\nname: reviewer\ndescription: Reviews code.\nmodel: sonnet\n---\nReview.\n",
    )
    .unwrap();
    let (code, parsed) = run(
        state,
        &config,
        &[
            "agents",
            "def",
            "validate",
            "--path",
            agent.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));

    let (code, parsed) = run(state, &config, &["skills", "remove", "--id", &id]);
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));

    let secret = "sk-test-secret-value";
    let (code, parsed) = run(
        state,
        &config,
        &[
            "providers",
            "add",
            "--id",
            "local",
            "--name",
            "Local",
            "--kind",
            "anthropic",
        ],
    );
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    let (code, parsed) = run(
        state,
        &config,
        &[
            "providers",
            "set-secret",
            "--id",
            "local",
            "--secret",
            secret,
        ],
    );
    assert_eq!(code, 0, "{parsed}");
    assert!(!parsed.to_string().contains(secret), "{parsed}");
    seen.push(record(state, &parsed));
    for args in [
        vec!["providers", "has-secret", "--id", "local"],
        vec!["providers", "list"],
        vec!["providers", "probe", "--id", "local", "--offline"],
    ] {
        let (code, parsed) = run(state, &config, &args);
        assert_eq!(code, 0, "{args:?} {parsed}");
        assert!(!parsed.to_string().contains(secret), "{parsed}");
        seen.push(record(state, &parsed));
    }
    let skill = state.join("fixture-skill");
    let (code, parsed) = run(
        state,
        &config,
        &["test", "save-suite", "--skill", skill.to_str().unwrap()],
    );
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    let mock = wire::spawn_wire();
    let (code, parsed) = run(
        state,
        &config,
        &[
            "providers",
            "add",
            "--id",
            "local",
            "--name",
            "Local",
            "--kind",
            "anthropic",
            "--base-url",
            &format!("http://127.0.0.1:{}", mock.port),
        ],
    );
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    let (code, parsed) = run(
        state,
        &config,
        &[
            "test",
            "run",
            "--skill",
            skill.to_str().unwrap(),
            "--mode",
            "offline",
            "--repetitions",
            "1",
        ],
    );
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    let (code, parsed) = run(state, &config, &["test", "show"]);
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    let (code, parsed) = run(
        state,
        &config,
        &[
            "test",
            "run",
            "--skill",
            skill.to_str().unwrap(),
            "--mode",
            "replay",
        ],
    );
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    for args in [
        vec!["test", "suites"],
        vec!["test", "status"],
        vec!["test", "results"],
        vec!["test", "export"],
        vec!["providers", "test", "--id", "local"],
        vec!["providers", "models", "--id", "local"],
        vec![
            "generate",
            "skill",
            "--name",
            "invoice-extractor",
            "--replay",
        ],
        vec![
            "generate",
            "agent",
            "--name",
            "security-reviewer",
            "--replay",
        ],
        vec![
            "generate",
            "evals",
            "--name",
            "invoice-extractor",
            "--replay",
        ],
        vec!["agents", "def", "list"],
        vec!["agents", "def", "show", "--name", "security-reviewer"],
        vec!["agents", "def", "deploy", "--name", "security-reviewer"],
        vec!["agents", "def", "undeploy", "--name", "security-reviewer"],
    ] {
        let (code, parsed) = run(state, &config, &args);
        assert_eq!(code, 0, "{args:?} {parsed}");
        seen.push(record(state, &parsed));
    }
    let (code, parsed) = run(
        state,
        &config,
        &[
            "providers",
            "add",
            "--id",
            "local",
            "--name",
            "Local",
            "--kind",
            "anthropic",
        ],
    );
    assert_eq!(code, 0, "{parsed}");
    seen.push(record(state, &parsed));
    let (code, parsed) = run(
        state,
        &config,
        &["generate", "skill", "--name", "invoice-extractor"],
    );
    assert_ne!(code, 0, "{parsed}");
    assert!(parsed.to_string().contains("API key"), "{parsed}");

    seen.sort();
    seen.dedup();
    let mut expected = command_ids();
    expected.sort();
    assert_eq!(
        seen,
        expected,
        "missing {}",
        expected
            .iter()
            .filter(|id| !seen.contains(id))
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );
}

fn record(_state: &std::path::Path, parsed: &Value) -> String {
    let data = data_of(parsed);
    let command = data["command"].as_str().unwrap();
    validate(command, data).unwrap_or_else(|err| panic!("{err} in {data}"));
    command.to_string()
}

#[test]
fn usage_error_is_exit_2_with_invalid_argument() {
    let tmp = tempfile::tempdir().unwrap();
    let output = execute_isolated(
        &tmp.path().join("state"),
        &tmp.path().join("config"),
        vec![
            "skill-hive-cli".into(),
            "--json".into(),
            "--skills-root".into(),
            tmp.path().join("skills").to_string_lossy().into_owned(),
            "skills".into(),
            "show".into(),
        ],
    );
    assert_eq!(output.code, 2, "{}", output.stdout);
    let parsed: Value = serde_json::from_str(output.stdout.trim()).unwrap();
    assert_eq!(parsed["ok"], false);
    assert_eq!(parsed["error"]["code"], "INVALID_ARGUMENT");
}

#[test]
fn deploy_into_an_occupied_target_returns_target_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    let state = tmp.path();
    let config = state.join("config");
    fs::create_dir_all(&config).unwrap();
    let skill_src = state.join("incoming");
    fs::create_dir_all(&skill_src).unwrap();
    fs::write(skill_src.join("SKILL.md"), "---\nname: demo\n---\nbody\n").unwrap();
    let (code, parsed) = run(
        state,
        &config,
        &["skills", "install", "--path", skill_src.to_str().unwrap()],
    );
    assert_eq!(code, 0, "{parsed}");
    let id = data_of(&parsed)["id"].as_str().unwrap().to_string();
    let occupied = state.join("occupied");
    fs::create_dir_all(&occupied).unwrap();
    fs::write(occupied.join("foreign.txt"), "leave me").unwrap();
    let (code, parsed) = run(
        state,
        &config,
        &[
            "skills",
            "deploy",
            "--id",
            &id,
            "--to",
            occupied.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 1, "{parsed}");
    assert_eq!(parsed["error"]["code"], "TARGET_CONFLICT");
    let conflicts = parsed["error"]["details"]["conflicts"].as_array().unwrap();
    assert!(!conflicts.is_empty());
    assert!(conflicts[0]["path"].as_str().unwrap().contains("occupied"));
    assert_eq!(
        fs::read_to_string(occupied.join("foreign.txt")).unwrap(),
        "leave me"
    );
}
