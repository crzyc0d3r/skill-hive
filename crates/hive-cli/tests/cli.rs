//! End-to-end tests of the `hive` binary in a temporary home.

use std::path::{Path, PathBuf};
use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;

struct Env {
    _tmp: tempfile::TempDir,
    home: PathBuf,
    hive_home: PathBuf,
}

impl Env {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        std::fs::create_dir_all(home.join(".claude")).unwrap();
        std::fs::create_dir_all(home.join(".codex")).unwrap();
        Self {
            hive_home: tmp.path().join("hive"),
            home,
            _tmp: tmp,
        }
    }

    fn cmd(&self) -> Command {
        let mut c = Command::cargo_bin("hive").unwrap();
        c.env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("HIVE_TARGET_HOME", &self.home)
            .env("HIVE_HOME", &self.hive_home)
            .env("HIVE_NO_KEYRING", "1")
            .env("NO_COLOR", "1");
        c
    }

    fn root(&self) -> &Path {
        self.home.parent().unwrap()
    }
}

fn write_skill(root: &Path, name: &str, description: &str, body: &str) -> PathBuf {
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: {description}\n---\n\n{body}\n"),
    )
    .unwrap();
    dir
}

#[test]
fn add_list_show_deploy_loadout_remove() {
    let env = Env::new();
    let src = env.root().join("src");
    write_skill(
        &src,
        "pdf-extract",
        "Extract text and tables from PDF documents. Use when the user shares a PDF.",
        "Run extract.py",
    );
    write_skill(
        &src,
        "git-commit",
        "Write conventional commit messages. Use when committing changes.",
        "git log",
    );
    write_skill(
        &src,
        "xlsx-build",
        "Build spreadsheets with formulas. Use when the user asks for an Excel file.",
        "openpyxl",
    );

    env.cmd()
        .args(["add", src.to_str().unwrap(), "--tag", "test"])
        .assert()
        .success()
        .stdout(predicate::str::contains("imported skill pdf-extract"))
        .stdout(predicate::str::contains("imported skill git-commit"));

    env.cmd()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pdf-extract"))
        .stdout(predicate::str::contains("xlsx-build"));

    let json = env.cmd().args(["--json", "list"]).output().unwrap();
    let items: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(items.as_array().unwrap().len(), 3);
    assert_eq!(items[0]["tags"][0], "test");

    env.cmd()
        .args(["show", "pdf-extract"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Extract text and tables"))
        .stdout(predicate::str::contains("validation"));

    env.cmd()
        .args(["search", "spreadsheet formulas"])
        .assert()
        .success()
        .stdout(predicate::str::contains("xlsx-build"));

    env.cmd()
        .args([
            "find",
            "pull",
            "the",
            "tables",
            "out",
            "of",
            "this",
            "pdf",
            "--target",
            "claude-code",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("pdf-extract"))
        .stdout(predicate::str::contains("suggested set"));

    env.cmd()
        .args(["deploy", "pdf-extract", "git-commit", "--to", "claude-code"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pdf-extract →"));
    assert!(
        env.home
            .join(".claude/skills/pdf-extract/SKILL.md")
            .exists()
    );

    env.cmd()
        .args(["targets", "claude-code"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pdf-extract"))
        .stdout(predicate::str::contains("Linked"));

    env.cmd()
        .args([
            "loadout",
            "create",
            "Docs",
            "--item",
            "pdf-extract",
            "--item",
            "xlsx-build",
        ])
        .assert()
        .success();
    env.cmd()
        .args([
            "loadout",
            "plan",
            "docs",
            "--to",
            "claude-code",
            "--exclusive",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("xlsx-build"))
        .stdout(predicate::str::contains("git-commit"));
    env.cmd()
        .args([
            "loadout",
            "apply",
            "docs",
            "--to",
            "claude-code",
            "--exclusive",
        ])
        .assert()
        .success();
    assert!(env.home.join(".claude/skills/xlsx-build").exists());
    assert!(!env.home.join(".claude/skills/git-commit").exists());

    env.cmd()
        .args(["conflicts", "pdf-extract", "xlsx-build"])
        .assert()
        .success()
        .stdout(predicate::str::contains("no conflicts"));

    env.cmd()
        .args(["scan"])
        .assert()
        .success()
        .stdout(predicate::str::contains("managed"));

    env.cmd()
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("database"));

    env.cmd()
        .args(["remove", "pdf-extract", "--yes"])
        .assert()
        .success();
    assert!(!env.home.join(".claude/skills/pdf-extract").exists());
    env.cmd()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pdf-extract").not());
}

#[test]
fn scan_imports_from_agent_directories_and_reports_conflicts() {
    let env = Env::new();
    write_skill(
        &env.home.join(".claude/skills"),
        "commit-helper",
        "Write conventional commit messages for git. Use when committing changes.",
        "a",
    );
    write_skill(
        &env.home.join(".codex/skills"),
        "git-commits",
        "Write conventional commit messages for git. Use when committing changes.",
        "b",
    );

    env.cmd()
        .args(["scan"])
        .assert()
        .success()
        .stdout(predicate::str::contains("commit-helper"))
        .stdout(predicate::str::contains("new"));
    env.cmd()
        .args(["scan", "--import"])
        .assert()
        .success()
        .stdout(predicate::str::contains("imported commit-helper"))
        .stdout(predicate::str::contains("imported git-commits"));

    // Both describe the same thing: the analyser flags the pair.
    env.cmd()
        .args(["conflicts", "commit-helper", "git-commits"])
        .assert()
        .success()
        .stdout(predicate::str::contains("DescriptionOverlap"));

    // The imported items came from agent directories.
    let json = env
        .cmd()
        .args(["--json", "show", "commit-helper"])
        .output()
        .unwrap();
    let detail: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(detail["item"]["source"]["kind"], "agent_dir");
    assert_eq!(detail["item"]["source"]["targetId"], "claude-code");
}

#[test]
fn scripted_evaluation_end_to_end() {
    let env = Env::new();
    let src = env.root().join("src");
    let skill = write_skill(
        &src,
        "report-writer",
        "Write a project report file when the user asks for a report.",
        "Write report.md with a total.",
    );
    std::fs::create_dir_all(skill.join("evals")).unwrap();
    std::fs::write(
        skill.join("evals/evals.json"),
        r#"{"skill_name":"report-writer","evals":[{"id":1,"name":"write-report","prompt":"Write the quarterly report.","expected_output":"report.md","files":[],"expectations":["report.md exists"]}]}"#,
    )
    .unwrap();
    std::fs::write(
        skill.join("evals/checks.json"),
        r#"{"evals":{"1":[{"id":"report","expectation":"report.md exists","kind":"file_exists","path":"report.md"}]}}"#,
    )
    .unwrap();
    env.cmd()
        .args(["add", skill.to_str().unwrap()])
        .assert()
        .success();

    let script = env.root().join("script.json");
    std::fs::write(
        &script,
        r##"{"rules":[{"match":"(?i)skill is available","with_skill":true,"output":"Wrote report.md","files":{"report.md":"# Report\n"},"tool_calls":2,"duration_ms":5}],"default":{"output":"I do not know how to write that."}}"##,
    )
    .unwrap();
    let out = env
        .cmd()
        .args([
            "--json",
            "provider",
            "add",
            "scripted",
            "--kind",
            "scripted",
            "--command",
            script.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let provider: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let provider_id = provider["id"].as_str().unwrap().to_string();

    env.cmd()
        .args(["eval", "suite", "report-writer"])
        .assert()
        .success()
        .stdout(predicate::str::contains("write-report"))
        .stdout(predicate::str::contains("file_exists"));

    env.cmd()
        .args([
            "eval",
            "run",
            "report-writer",
            "--executor",
            &provider_id,
            "--grader",
            "none",
            "--repeats",
            "2",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("verdict:"))
        .stdout(predicate::str::contains("with_skill"))
        .stdout(predicate::str::contains("without_skill"));

    let json = env
        .cmd()
        .args(["--json", "eval", "runs", "report-writer"])
        .output()
        .unwrap();
    let runs: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    let run = &runs[0];
    assert_eq!(run["status"], "done");
    let arms = run["summary"]["arms"].as_array().unwrap();
    let with = arms.iter().find(|a| a["label"] == "with_skill").unwrap();
    let without = arms.iter().find(|a| a["label"] == "without_skill").unwrap();
    assert_eq!(with["passRate"]["mean"].as_f64().unwrap(), 1.0);
    assert_eq!(without["passRate"]["mean"].as_f64().unwrap(), 0.0);
    let run_id = run["id"].as_str().unwrap();

    env.cmd()
        .args(["eval", "report", run_id, "--markdown"])
        .assert()
        .success()
        .stdout(predicate::str::contains("with_skill"));
    env.cmd()
        .args(["eval", "history", "report-writer"])
        .assert()
        .success()
        .stdout(predicate::str::contains("100%"));

    // The library item now carries the measurement.
    env.cmd()
        .args(["show", "report-writer"])
        .assert()
        .success()
        .stdout(predicate::str::contains("measured"));
}

#[test]
fn config_and_projects() {
    let env = Env::new();
    env.cmd()
        .args(["config", "set", "link-mode", "copy"])
        .assert()
        .success();
    env.cmd()
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("copy"))
        .stdout(predicate::str::contains("auto-update-check      true"));
    env.cmd()
        .args(["config", "set", "auto-update-check", "off"])
        .assert()
        .success();
    env.cmd()
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("auto-update-check      false"));
    env.cmd()
        .args(["config", "set", "auto-update-check", "maybe"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("expected true or false"));
    env.cmd()
        .args(["config", "set", "nonsense", "x"])
        .assert()
        .failure();
    let proj = env.root().join("proj");
    std::fs::create_dir_all(&proj).unwrap();
    env.cmd()
        .args(["project", "add", proj.to_str().unwrap()])
        .assert()
        .success();
    env.cmd()
        .args(["project", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("proj"));
    env.cmd()
        .args(["catalog"])
        .assert()
        .success()
        .stdout(predicate::str::contains("anthropics/skills"));
    env.cmd()
        .args(["preview", "/definitely/missing/path"])
        .assert()
        .failure();
}
