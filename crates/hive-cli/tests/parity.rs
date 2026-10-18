//! Disk and database effects for the parity rows. No vendor network.

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use hive_cli::execute_isolated;
use hive_core::db::skill_store::SkillStore;
use hive_core::providers::list_named_providers;
use serde_json::Value;

fn run(state: &Path, config: &Path, args: &[&str]) -> (i32, Value) {
    let mut full = vec![
        "skill-hive-cli".to_string(),
        "--json".to_string(),
        "--skills-root".to_string(),
        state.join("skills").to_string_lossy().into_owned(),
    ];
    full.extend(args.iter().copied().map(str::to_string));
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

fn ok(state: &Path, config: &Path, args: &[&str]) -> Value {
    let (code, parsed) = run(state, config, args);
    assert_eq!(code, 0, "{args:?} {parsed}");
    data_of(&parsed).clone()
}

fn fresh() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let state = tmp.path().to_path_buf();
    let config = state.join("config");
    fs::create_dir_all(&config).unwrap();
    (tmp, state, config)
}

fn base_of(state: &Path, config: &Path) -> PathBuf {
    PathBuf::from(
        ok(state, config, &["repo", "status"])["base"]
            .as_str()
            .unwrap(),
    )
}

fn store_at(base: &Path) -> SkillStore {
    SkillStore::new(&base.join("hive.db")).unwrap()
}

fn write_skill(dir: &Path, name: &str, body: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: Describe {name}.\n---\n{body}\n"),
    )
    .unwrap();
}

fn git(dir: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .env_remove("GIT_AUTHOR_NAME")
        .env_remove("GIT_AUTHOR_EMAIL")
        .env_remove("GIT_COMMITTER_NAME")
        .env_remove("GIT_COMMITTER_EMAIL")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn init_repo(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    git(dir, &["init", "-b", "main"]);
    git(dir, &["config", "user.email", "test@example.com"]);
    git(dir, &["config", "user.name", "Test"]);
}

fn commit_all(dir: &Path, message: &str) {
    git(dir, &["add", "-A"]);
    git(
        dir,
        &[
            "-c",
            "user.email=test@example.com",
            "-c",
            "user.name=Test",
            "commit",
            "-m",
            message,
        ],
    );
}

fn install(state: &Path, config: &Path, dir: &Path) -> String {
    ok(
        state,
        config,
        &["skills", "install", "--path", dir.to_str().unwrap()],
    )["id"]
        .as_str()
        .unwrap()
        .to_string()
}

struct Mock {
    port: u16,
    hits: Arc<Mutex<Vec<String>>>,
}

fn spawn_mock(body: &str) -> Mock {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let hits = Arc::new(Mutex::new(Vec::new()));
    let recorded = hits.clone();
    let body = body.to_string();
    thread::spawn(move || {
        for _ in 0..8 {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let mut collected = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        collected.extend_from_slice(&buf[..n]);
                        let text = String::from_utf8_lossy(&collected);
                        if let Some((head, rest)) = text.split_once("\r\n\r\n") {
                            let length = head.lines().find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().ok())
                                    .flatten()
                            });
                            if rest.len() >= length.unwrap_or(0) {
                                break;
                            }
                        }
                    }
                }
            }
            recorded
                .lock()
                .unwrap()
                .push(String::from_utf8_lossy(&collected).into_owned());
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    Mock { port, hits }
}

fn agent_dir(state: &Path, config: &Path, key: &str) -> PathBuf {
    let base = base_of(state, config);
    fs::create_dir_all(base.join("home").join(match key {
        "claude_code" => ".claude",
        "codex" => ".codex",
        other => panic!("unsupported test key {other}"),
    }))
    .unwrap();
    let listed = ok(state, config, &["agents", "list"]);
    let agent = listed["agents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["key"] == key)
        .unwrap();
    assert_eq!(agent["installed"], true, "{agent}");
    PathBuf::from(agent["skills_dir"].as_str().unwrap())
}

#[test]
fn agents_list_reports_install_state_and_paths() {
    let (_tmp, state, config) = fresh();
    let skills_dir = agent_dir(&state, &config, "claude_code");
    assert!(skills_dir.ends_with(".claude/skills"), "{skills_dir:?}");
    assert!(skills_dir.starts_with(base_of(&state, &config).join("home")));
}

#[test]
fn git_clone_and_pull_reindex_a_local_repo() {
    let (_tmp, state, config) = fresh();
    let source = state.join("source");
    init_repo(&source);
    write_skill(&source, "cloned-skill", "version-one");
    fs::create_dir_all(source.join("notes")).unwrap();
    fs::write(source.join("notes").join("extra.txt"), "nested").unwrap();
    commit_all(&source, "v1");

    let cloned = ok(
        &state,
        &config,
        &["git", "clone", "--url", source.to_str().unwrap()],
    );
    let backup = PathBuf::from(cloned["path"].as_str().unwrap());
    assert!(backup.join(".git").is_dir(), "{backup:?}");
    let base = base_of(&state, &config);
    let index: Value =
        serde_json::from_str(&fs::read_to_string(base.join("library-index.json")).unwrap())
            .unwrap();
    assert!(
        !index["skills"].as_array().unwrap().is_empty(),
        "index missing skill row: {index}"
    );
    let listed = ok(&state, &config, &["skills", "list"]);
    let row = listed["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "cloned-skill")
        .expect("skill row");
    let id = row["id"].as_str().unwrap();
    let central = PathBuf::from(
        ok(&state, &config, &["skills", "show", "--id", id])["path"]
            .as_str()
            .unwrap(),
    );
    assert!(central.join("SKILL.md").is_file());

    fs::write(
        source.join("SKILL.md"),
        "---\nname: cloned-skill\ndescription: Describe cloned-skill.\n---\nversion-two\n",
    )
    .unwrap();
    commit_all(&source, "v2");
    git(&backup, &["config", "user.email", "test@example.com"]);
    git(&backup, &["config", "user.name", "Test"]);
    let pulled = ok(&state, &config, &["git", "pull"]);
    assert!(!pulled["branch"].as_str().unwrap().is_empty());
    let central_body = fs::read_to_string(central.join("SKILL.md")).unwrap();
    assert!(
        central_body.contains("version-two"),
        "library was not reconciled: {central_body}"
    );
    let backup_body = fs::read_to_string(backup.join("SKILL.md")).unwrap();
    assert!(backup_body.contains("version-two"), "{backup_body}");
}

#[test]
fn agent_def_show_returns_file_text() {
    let (_tmp, state, config) = fresh();
    let base = base_of(&state, &config);
    let defs = base.join("agent-defs");
    fs::create_dir_all(&defs).unwrap();
    fs::write(defs.join("reviewer.md"), "agent body for the reviewer\n").unwrap();
    let shown = ok(
        &state,
        &config,
        &["agents", "def", "show", "--name", "reviewer"],
    );
    assert!(
        shown["text"].as_str().unwrap().contains("agent body"),
        "{shown}"
    );
}

#[test]
fn providers_add_persists_model_and_caller_roles() {
    let (_tmp, state, config) = fresh();
    let added = ok(
        &state,
        &config,
        &[
            "providers",
            "add",
            "--id",
            "openai-local",
            "--name",
            "OpenAI",
            "--kind",
            "openai",
            "--model",
            "gpt-test",
            "--role",
            "generator",
        ],
    );
    assert_eq!(added["model"], "gpt-test");
    assert_eq!(added["roles"], serde_json::json!(["generator"]));
    let base = base_of(&state, &config);
    let rows = list_named_providers(&base.join("hive.db")).unwrap();
    let row = rows.iter().find(|row| row.id == "openai-local").unwrap();
    assert_eq!(row.default_model.as_deref(), Some("gpt-test"));
    assert!(row.role_generator);
    assert!(!row.role_grader);
    assert!(!row.role_executor);

    let defaults = ok(
        &state,
        &config,
        &[
            "providers",
            "add",
            "--id",
            "claude",
            "--name",
            "Claude",
            "--kind",
            "anthropic",
        ],
    );
    assert_eq!(defaults["model"], "claude-sonnet-4");
    assert_eq!(
        defaults["roles"],
        serde_json::json!(["generator", "grader"])
    );
}

#[test]
fn providers_probe_posts_to_local_base_and_refuses_when_absent() {
    let mock = spawn_mock(r#"{"ok":true,"stages":3,"marker":"probe-from-mock"}"#);
    let (_tmp, state, config) = fresh();
    ok(
        &state,
        &config,
        &[
            "providers",
            "add",
            "--id",
            "local",
            "--name",
            "Local",
            "--kind",
            "openai",
            "--base-url",
            &format!("http://127.0.0.1:{}/v1/messages", mock.port),
        ],
    );
    let live = ok(
        &state,
        &config,
        &["providers", "probe", "--id", "local", "--live"],
    );
    assert_eq!(live["live"], true, "{live}");
    assert_eq!(live["ok"], true);
    assert_eq!(live["body"]["marker"], "probe-from-mock");
    let hit = mock.hits.lock().unwrap().join("\n");
    assert!(hit.starts_with("POST "), "{hit}");
    assert!(!hit.contains("api.openai.com"), "{hit}");

    let closed = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        port
    };
    ok(
        &state,
        &config,
        &[
            "providers",
            "add",
            "--id",
            "missing",
            "--name",
            "Missing",
            "--kind",
            "openai",
            "--base-url",
            &format!("http://127.0.0.1:{closed}/probe"),
        ],
    );
    let absent = ok(
        &state,
        &config,
        &["providers", "probe", "--id", "missing", "--live"],
    );
    assert_eq!(absent["live"], false, "{absent}");
    assert_eq!(absent["refused"], true, "{absent}");
    assert!(absent["stages"].as_i64().unwrap() > 0);
}

#[test]
fn test_run_uses_scripted_plan_until_a_base_url_is_set() {
    let (_tmp, state, config) = fresh();
    let skill = state.join("suite");
    ok(
        &state,
        &config,
        &["test", "save-suite", "--skill", skill.to_str().unwrap()],
    );
    let scripted = ok(
        &state,
        &config,
        &[
            "test",
            "run",
            "--skill",
            skill.to_str().unwrap(),
            "--mode",
            "offline",
        ],
    );
    let base = base_of(&state, &config);
    let first = fs::read_to_string(base.join("test-runs").join("last.json")).unwrap();
    assert!(first.contains(scripted["run_id"].as_str().unwrap()));
    assert!(!first.contains("mock-run"));

    let mock = spawn_mock(r#"{"run_id":"mock-run","status":"passed","cells":2,"passed":2}"#);
    ok(
        &state,
        &config,
        &[
            "providers",
            "add",
            "--id",
            "runner",
            "--name",
            "Runner",
            "--kind",
            "openai",
            "--base-url",
            &format!("http://127.0.0.1:{}/chat", mock.port),
        ],
    );
    let live = ok(
        &state,
        &config,
        &[
            "test",
            "run",
            "--skill",
            skill.to_str().unwrap(),
            "--mode",
            "offline",
        ],
    );
    assert_eq!(live["run_id"], "mock-run");
    let last = fs::read_to_string(base.join("test-runs").join("last.json")).unwrap();
    assert!(last.contains("mock-run"), "{last}");
    let hit = mock.hits.lock().unwrap().join("\n");
    assert!(hit.starts_with("POST "), "{hit}");
    let posted = hit
        .split("\r\n\r\n")
        .nth(1)
        .and_then(|body| serde_json::from_str::<Value>(body.trim()).ok())
        .unwrap_or_else(|| panic!("request body was not JSON: {hit}"));
    assert_eq!(posted["mode"], "offline");
    assert_eq!(posted["skill"].as_str(), skill.to_str(), "{hit}");
}

#[test]
fn generate_skill_and_agent_write_the_caller_task() {
    let (_tmp, state, config) = fresh();
    let task = "classify seaweed samples from the dock";
    ok(
        &state,
        &config,
        &[
            "generate", "skill", "--name", "seaweed", "--task", task, "--replay",
        ],
    );
    let base = base_of(&state, &config);
    let skill =
        fs::read_to_string(base.join("generations").join("seaweed").join("SKILL.md")).unwrap();
    assert!(skill.contains(task), "{skill}");
    assert!(!skill.contains("supplier invoice"), "{skill}");

    let agent_task = "watch the tide gauges";
    ok(
        &state,
        &config,
        &[
            "generate", "agent", "--name", "tide", "--task", agent_task, "--replay",
        ],
    );
    let agent = fs::read_to_string(base.join("agent-defs").join("tide.md")).unwrap();
    assert!(agent.contains(agent_task), "{agent}");
    let shown = ok(
        &state,
        &config,
        &["agents", "def", "show", "--name", "tide"],
    );
    assert!(shown["text"].as_str().unwrap().contains(agent_task));

    let mock = spawn_mock(r#"{"text":"MOCK-GENERATED-BODY"}"#);
    ok(
        &state,
        &config,
        &[
            "providers",
            "add",
            "--id",
            "gen",
            "--name",
            "Gen",
            "--kind",
            "anthropic",
            "--base-url",
            &format!("http://127.0.0.1:{}/v1/messages", mock.port),
        ],
    );
    let live_task = "draft a kelp inventory";
    ok(
        &state,
        &config,
        &["generate", "skill", "--name", "kelp", "--task", live_task],
    );
    let live = fs::read_to_string(base.join("generations").join("kelp").join("SKILL.md")).unwrap();
    assert!(live.contains(live_task), "{live}");
    assert!(live.contains("MOCK-GENERATED-BODY"), "{live}");
    let hit = mock.hits.lock().unwrap().join("\n");
    assert!(hit.starts_with("POST "), "{hit}");
    assert!(hit.contains(live_task), "{hit}");
    assert!(!hit.contains("supplier invoice"), "{hit}");
}

#[test]
fn presets_keep_fields_preview_and_deploy_to_disk() {
    let (_tmp, state, config) = fresh();
    let skill_dir = state.join("incoming");
    write_skill(&skill_dir, "preset-skill", "body");
    let id = install(&state, &config, &skill_dir);
    let preset = ok(
        &state,
        &config,
        &["presets", "create", "--name", "Research"],
    )["id"]
        .as_str()
        .unwrap()
        .to_string();
    ok(
        &state,
        &config,
        &["presets", "add-skill", "--id", &preset, "--skill", &id],
    );
    let base = base_of(&state, &config);
    store_at(&base)
        .update_scenario(&preset, "Research", Some("keep-desc"), Some("keep-icon"))
        .unwrap();
    ok(
        &state,
        &config,
        &["presets", "update", "--id", &preset, "--name", "Renamed"],
    );
    let saved = store_at(&base)
        .get_all_scenarios()
        .unwrap()
        .into_iter()
        .find(|row| row.id == preset)
        .unwrap();
    assert_eq!(saved.name, "Renamed");
    assert_eq!(saved.description.as_deref(), Some("keep-desc"));
    assert_eq!(saved.icon.as_deref(), Some("keep-icon"));

    let custom = state.join("custom-root");
    fs::create_dir_all(&custom).unwrap();
    ok(
        &state,
        &config,
        &[
            "presets",
            "deploy",
            "--id",
            &preset,
            "--tool",
            "claude_code",
            "--root",
            custom.to_str().unwrap(),
        ],
    );
    assert!(custom.join("preset-skill").join("SKILL.md").is_file());
    let _skills_dir = agent_dir(&state, &config, "claude_code");
    let preview = ok(&state, &config, &["presets", "preview", "--id", &preset]);
    let deploy = preview["deploy"].as_array().unwrap();
    let undeploy = preview["undeploy"].as_array().unwrap();
    assert!(
        deploy.iter().any(|row| row["action"] == "deploy"
            && row["path"].as_str().unwrap().contains("preset-skill")),
        "{preview}"
    );
    assert!(
        undeploy.iter().any(|row| row["action"] == "undeploy"
            && row["path"].as_str().unwrap().contains("custom-root")),
        "{preview}"
    );

    ok(&state, &config, &["presets", "apply", "--id", &preset]);
    let home_skill = base
        .join("home")
        .join(".claude")
        .join("skills")
        .join("preset-skill")
        .join("SKILL.md");
    assert!(home_skill.is_file(), "apply did not write {home_skill:?}");
    let status = ok(&state, &config, &["presets", "status", "--id", &preset]);
    assert_eq!(status["members"][0]["deployed"], true, "{status}");

    ok(&state, &config, &["presets", "deactivate"]);
    let current = ok(&state, &config, &["presets", "current"]);
    assert_eq!(current["id"], "");
    assert!(
        !home_skill.exists(),
        "deactivate left {}",
        home_skill.display()
    );
}

#[test]
fn presets_bare_deploy_and_undeploy_use_a_resolved_root() {
    let (_tmp, state, config) = fresh();
    let skill_dir = state.join("incoming");
    write_skill(&skill_dir, "bare-skill", "body");
    let id = install(&state, &config, &skill_dir);
    let preset = ok(&state, &config, &["presets", "create", "--name", "Bare"])["id"]
        .as_str()
        .unwrap()
        .to_string();
    ok(
        &state,
        &config,
        &["presets", "add-skill", "--id", &preset, "--skill", &id],
    );
    let (code, missing) = run(&state, &config, &["presets", "deploy", "--id", &preset]);
    assert_ne!(code, 0, "{missing}");
    assert!(missing.to_string().contains("root"), "{missing}");

    let root = state.join("default-root");
    fs::create_dir_all(&root).unwrap();
    ok(
        &state,
        &config,
        &[
            "presets",
            "deploy",
            "--id",
            &preset,
            "--tool",
            "codex",
            "--root",
            root.to_str().unwrap(),
        ],
    );
    let written = root.join("bare-skill").join("SKILL.md");
    assert!(written.is_file());
    fs::remove_dir_all(root.join("bare-skill")).unwrap();
    assert!(!written.exists());
    ok(&state, &config, &["presets", "deploy", "--id", &preset]);
    assert!(
        written.is_file(),
        "bare deploy did not rewrite the saved root"
    );
    ok(&state, &config, &["presets", "undeploy", "--id", &preset]);
    assert!(!written.exists(), "bare undeploy left the deployment");
}

#[test]
fn repo_status_includes_counts_and_pending_path() {
    let (_tmp, state, config) = fresh();
    let skill_dir = state.join("incoming");
    write_skill(&skill_dir, "counted", "body");
    install(&state, &config, &skill_dir);
    let next = state.join("next-base");
    fs::create_dir_all(&next).unwrap();
    ok(
        &state,
        &config,
        &["repo", "set-path", "--path", next.to_str().unwrap()],
    );
    let status = ok(&state, &config, &["repo", "status"]);
    assert!(
        status["counts"]["skills"].as_i64().unwrap() >= 1,
        "{status}"
    );
    assert_eq!(status["pending"], next.to_str().unwrap());
}

#[test]
fn skills_list_show_export_and_install_cover_metadata() {
    let (_tmp, state, config) = fresh();
    let skill_dir = state.join("incoming");
    write_skill(&skill_dir, "listed", "body");
    fs::create_dir_all(skill_dir.join("references")).unwrap();
    fs::write(skill_dir.join("references").join("note.md"), "nested note").unwrap();
    let id = install(&state, &config, &skill_dir);
    ok(
        &state,
        &config,
        &["skills", "tag", "add", "--id", &id, "--name", "tools"],
    );
    ok(
        &state,
        &config,
        &[
            "skills",
            "set-source",
            "--id",
            &id,
            "--source",
            "local-tree",
        ],
    );
    let listed = ok(&state, &config, &["skills", "list", "--tag", "tools"]);
    assert_eq!(listed["skills"].as_array().unwrap().len(), 1, "{listed}");
    let row = &listed["skills"][0];
    assert_eq!(row["tags"][0], "tools");
    assert!(row.get("source").is_some(), "{row}");
    assert!(row.get("deployed").is_some(), "{row}");
    let hidden = ok(&state, &config, &["skills", "list", "--tag", "missing-tag"]);
    assert!(hidden["skills"].as_array().unwrap().is_empty(), "{hidden}");

    let shown = ok(&state, &config, &["skills", "show", "--id", &id]);
    assert_eq!(shown["description"], "Describe listed.");
    assert!(Path::new(shown["path"].as_str().unwrap())
        .join("SKILL.md")
        .is_file());
    assert_eq!(shown["tags"][0], "tools");
    assert_eq!(shown["source"], "local-tree");

    let export_to = state.join("exported");
    ok(
        &state,
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
    assert_eq!(
        fs::read_to_string(export_to.join("references").join("note.md")).unwrap(),
        "nested note"
    );
    let (code, refused) = run(
        &state,
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
    assert_ne!(code, 0, "{refused}");
    assert_eq!(refused["error"]["code"], "TARGET_CONFLICT");
    fs::write(
        Path::new(shown["path"].as_str().unwrap()).join("SKILL.md"),
        "---\nname: listed\ndescription: Describe listed.\n---\nforced\n",
    )
    .unwrap();
    ok(
        &state,
        &config,
        &[
            "skills",
            "export",
            "--id",
            &id,
            "--to",
            export_to.to_str().unwrap(),
            "--force",
        ],
    );
    assert!(fs::read_to_string(export_to.join("SKILL.md"))
        .unwrap()
        .contains("forced"));

    let git_dir = state.join("git-skill");
    init_repo(&git_dir);
    write_skill(&git_dir, "from-git", "git-body");
    commit_all(&git_dir, "skill");
    let git_id = install(&state, &config, &git_dir);
    let base = base_of(&state, &config);
    let row = store_at(&base).get_skill_by_id(&git_id).unwrap().unwrap();
    assert_eq!(row.source_type, "git");
    assert!(Path::new(&row.central_path).join("SKILL.md").is_file());

    let resolved = ok(
        &state,
        &config,
        &[
            "skills",
            "install",
            "--path",
            "https://skills.sh/acme/widgets/helper",
        ],
    );
    assert_eq!(resolved["resolved"], "https://github.com/acme/widgets");
    assert_eq!(resolved["subpath"], "helper");
    let file = fs::read_to_string(base.join("install-resolutions.json")).unwrap();
    assert!(file.contains("https://github.com/acme/widgets"), "{file}");
    assert!(file.contains("helper"), "{file}");
}

#[test]
fn skills_update_check_remove_disable_and_sync() {
    let (_tmp, state, config) = fresh();
    let skill_dir = state.join("incoming");
    write_skill(&skill_dir, "mutable", "original");
    let id = install(&state, &config, &skill_dir);
    let shown = ok(&state, &config, &["skills", "show", "--id", &id]);
    let central = PathBuf::from(shown["path"].as_str().unwrap());

    let checkout = state.join("checkout");
    init_repo(&checkout);
    write_skill(&checkout, "mutable", "updated-from-git");
    fs::create_dir_all(checkout.join("notes")).unwrap();
    fs::write(checkout.join("notes").join("a.md"), "copied").unwrap();
    commit_all(&checkout, "update");
    let updated = ok(
        &state,
        &config,
        &[
            "skills",
            "update",
            "--id",
            &id,
            "--from",
            checkout.to_str().unwrap(),
        ],
    );
    assert_eq!(updated["applied"], true);
    assert!(fs::read_to_string(central.join("SKILL.md"))
        .unwrap()
        .contains("updated-from-git"));
    assert_eq!(
        fs::read_to_string(central.join("notes").join("a.md")).unwrap(),
        "copied"
    );

    let fake = state.join("fake-remote");
    write_skill(&fake, "mutable", "updated-from-git");
    fs::create_dir_all(fake.join(".git")).unwrap();
    fs::write(fake.join(".git").join("HEAD"), "abc123deadbeef\n").unwrap();
    let checked = ok(
        &state,
        &config,
        &[
            "skills",
            "check",
            "--id",
            &id,
            "--from",
            fake.to_str().unwrap(),
        ],
    );
    assert_eq!(checked["remoteRevision"], "abc123deadbeef");
    let base = base_of(&state, &config);
    let row = store_at(&base).get_skill_by_id(&id).unwrap().unwrap();
    assert_eq!(row.remote_revision.as_deref(), Some("abc123deadbeef"));

    let elsewhere = state.join("elsewhere");
    write_skill(&elsewhere, "mutable", "different body");
    let compared = ok(
        &state,
        &config,
        &[
            "skills",
            "set-source",
            "--id",
            &id,
            "--source",
            elsewhere.to_str().unwrap(),
        ],
    );
    assert_eq!(compared["status"], "update");
    assert!(
        fs::read_to_string(central.join("SKILL.md"))
            .unwrap()
            .contains("updated-from-git"),
        "set-source copied the tree"
    );
    let row = store_at(&base).get_skill_by_id(&id).unwrap().unwrap();
    assert_eq!(row.source_ref.as_deref(), Some(elsewhere.to_str().unwrap()));
    assert_eq!(row.update_status, "update");

    let skills_dir = agent_dir(&state, &config, "codex");
    let preset = ok(&state, &config, &["presets", "create", "--name", "Sync"])["id"]
        .as_str()
        .unwrap()
        .to_string();
    ok(
        &state,
        &config,
        &["presets", "add-skill", "--id", &preset, "--skill", &id],
    );
    let dry = ok(
        &state,
        &config,
        &["skills", "sync", "--preset", &preset, "--dry-run"],
    );
    assert_eq!(dry["synced"], 0);
    let paths = dry["paths"].as_array().unwrap();
    assert!(!paths.is_empty(), "{dry}");
    for value in paths {
        let path = PathBuf::from(value.as_str().unwrap());
        assert!(!path.exists(), "dry-run wrote {path:?}");
    }
    let synced = ok(&state, &config, &["skills", "sync", "--preset", &preset]);
    assert!(synced["synced"].as_i64().unwrap() >= 1, "{synced}");
    assert!(
        paths.iter().any(|value| {
            Path::new(value.as_str().unwrap())
                .join("SKILL.md")
                .is_file()
        }),
        "sync did not write reported paths: {synced}"
    );

    ok(
        &state,
        &config,
        &["skills", "deploy", "--id", &id, "--to", "codex"],
    );
    let deployed = skills_dir.join("mutable");
    assert!(
        deployed.join("SKILL.md").is_file(),
        "agent deploy missing {deployed:?}"
    );
    let explicit = state.join("explicit-target");
    ok(
        &state,
        &config,
        &[
            "skills",
            "deploy",
            "--id",
            &id,
            "--to",
            explicit.to_str().unwrap(),
        ],
    );
    assert!(explicit.join("SKILL.md").is_file() || explicit.is_symlink());
    ok(&state, &config, &["skills", "undeploy", "--agent", "codex"]);
    assert!(!deployed.exists(), "undeploy left {deployed:?}");

    ok(&state, &config, &["skills", "disable", "--id", &id]);
    let status = ok(&state, &config, &["skills", "status"]);
    let skill_status = status["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == id)
        .unwrap();
    assert_eq!(skill_status["enabled"], false, "{skill_status}");
    assert!(
        store_at(&base)
            .get_targets_for_skill(&id)
            .unwrap()
            .is_empty(),
        "disable left deployment rows"
    );

    ok(&state, &config, &["skills", "enable", "--id", &id]);
    ok(
        &state,
        &config,
        &["skills", "deploy", "--id", &id, "--to", "claude_code"],
    );
    let claude = agent_dir(&state, &config, "claude_code").join("mutable");
    assert!(claude.join("SKILL.md").is_file() || claude.is_symlink());
    ok(&state, &config, &["skills", "remove", "--id", &id]);
    assert!(store_at(&base).get_skill_by_id(&id).unwrap().is_none());
    assert!(!central.exists(), "central files remained");
    assert!(!claude.exists(), "deployment remained");
}

#[test]
fn test_measure_does_not_fall_back_to_the_scripted_provider() {
    let (_tmp, state, config) = fresh();
    let skill = state.join("pine");
    fs::create_dir_all(&skill).unwrap();
    fs::write(
        skill.join("SKILL.md"),
        "---\nname: pine\ndescription: File the weekly pine report\n---\n",
    )
    .unwrap();
    let (code, parsed) = run(
        &state,
        &config,
        &["test", "measure", "--skill", skill.to_str().unwrap()],
    );
    assert_ne!(code, 0, "{parsed}");
    let message = parsed["error"]["message"].as_str().unwrap_or("");
    assert!(
        message.contains("base URL") || message.contains("provider"),
        "{parsed}"
    );
    let base = base_of(&state, &config);
    assert!(
        !base.join("test-runs").join("last.json").is_file(),
        "measure wrote a scripted last.json"
    );
}

#[test]
fn test_blind_does_not_fall_back_to_the_scripted_provider() {
    let (_tmp, state, config) = fresh();
    let (code, parsed) = run(
        &state,
        &config,
        &[
            "test",
            "blind",
            "--prompt",
            "file the token",
            "--output-a",
            "alpha",
            "--output-b",
            "beta",
        ],
    );
    assert_ne!(code, 0, "{parsed}");
    let message = parsed["error"]["message"].as_str().unwrap_or("");
    assert!(
        message.contains("base URL") || message.contains("provider"),
        "{parsed}"
    );
    let base = base_of(&state, &config);
    assert!(
        !base.join("test-runs").join("blind.json").is_file(),
        "blind wrote a scripted comparison"
    );
}
