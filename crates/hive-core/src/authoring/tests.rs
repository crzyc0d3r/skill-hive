use std::fs;
use std::path::Path;
use std::process::Command;

use super::agents::{
    apply_agent_fix, export_agent, export_status, render_agent_file, render_agents_cli_json,
    AgentExportRequest,
};
use super::frontmatter::{parse_document, Style};
use super::skill::{
    deployment_status, instantiate_template, plan_publish, publish_draft, rename_skill,
    resolve_inside, template_ids, BuiltInTemplate, Deployment,
};
use super::validator::{
    first_qv_error, validate_agent, validate_command_file, validate_skill, Profile, Severity,
};
use super::versions::{
    export_history_json, hide_version, restore_version, save_autosave, VersionStore,
};
use crate::db::skill_store::SkillStore;
use crate::util::error::ErrorKind;

fn write_skill(root: &Path, name: &str, files: &[(&str, &str)]) {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    for (rel, body) in files {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, body).unwrap();
    }
}

fn skill(name: &str, description: &str, extra: &str, body: &str) -> String {
    format!("---\nname: {name}\ndescription: {description}\n{extra}---\n{body}")
}

#[test]
fn every_rule_id_has_a_function_path_and_the_suite_agrees_with_quick_validate() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let long = "x".repeat(1025);
    let cases = [
        (
            "ok-blank",
            skill("ok-blank", "Use this skill for notes.", "", "Hello\n"),
            true,
        ),
        (
            "ok-workflow",
            skill(
                "ok-workflow",
                "Use this skill for a procedure.",
                "",
                "1. Read.\n",
            ),
            true,
        ),
        (
            "ok-meta",
            "---\nname: ok-meta\ndescription: Use metadata.\nmetadata:\n  owner: ada\n---\n"
                .to_string(),
            true,
        ),
        (
            "ok-tools",
            skill(
                "ok-tools",
                "Use tools.",
                "allowed-tools: Bash Read\n",
                "Run it.\n",
            ),
            true,
        ),
        (
            "ok-quoted",
            "---\nname: ok-quoted\ndescription: \"Find papers\"\n---\nBody\n".into(),
            true,
        ),
        (
            "ok-fold",
            "---\nname: ok-fold\ndescription: >\n  Find papers\n  quickly.\n---\nBody\n".into(),
            true,
        ),
        (
            "ok-literal",
            "---\nname: ok-literal\ndescription: |\n  line one\n  line two\n---\n".into(),
            true,
        ),
        (
            "ok-fold-strip",
            "---\nname: ok-fold-strip\ndescription: >-\n  folded line\n---\n".into(),
            true,
        ),
        (
            "ok-lit-strip",
            "---\nname: ok-lit-strip\ndescription: |-\n  literal line\n---\n".into(),
            true,
        ),
        (
            "ok-license",
            skill(
                "ok-license",
                "Use the license.",
                "license: MIT\ncompatibility: macOS\n",
                "Ok\n",
            ),
            true,
        ),
        (
            "ui-styling",
            skill(
                "ui-styling",
                "Style components with semantic tokens.",
                "argument-hint: \"[component or layout]\"\n",
                "Use tokens.\n",
            ),
            false,
        ),
        ("missing-marker", "README.md only".into(), false),
        ("legacy-marker", "legacy".into(), false),
        (
            "nested-skill",
            skill("nested-skill", "Has a nested skill.", "", "Root\n"),
            false,
        ),
        (
            "late-marker",
            "Note\n---\nname: late-marker\ndescription: Too late.\n---\n".into(),
            false,
        ),
        (
            "unclosed",
            "---\nname: unclosed\ndescription: No closer.\nBody\n".into(),
            false,
        ),
        ("not-a-map", "---\n- just\n- a\n- list\n---\n".into(), false),
        (
            "bad-name",
            skill("Bad_Name", "Uppercase is rejected.", "", "No\n"),
            false,
        ),
        (
            "angle-desc",
            skill("angle-desc", "Use <tags> here.", "", "No\n"),
            false,
        ),
        ("long-desc", skill("long-desc", &long, "", "No\n"), false),
    ];
    assert_eq!(cases.len(), 20);
    for (name, body, qv_ok) in cases {
        let dir = root.join(name);
        fs::create_dir_all(&dir).unwrap();
        match name {
            "missing-marker" => fs::write(dir.join("README.md"), body).unwrap(),
            "legacy-marker" => fs::write(
                dir.join("skill.md"),
                "---\nname: legacy-marker\ndescription: Old marker.\n---\n",
            )
            .unwrap(),
            "nested-skill" => {
                fs::write(dir.join("SKILL.md"), body).unwrap();
                fs::create_dir_all(dir.join("nested")).unwrap();
                fs::write(
                    dir.join("nested").join("SKILL.md"),
                    "---\nname: nested\ndescription: Inner.\n---\n",
                )
                .unwrap();
            }
            "bad-name" => fs::write(dir.join("SKILL.md"), body).unwrap(),
            _ => fs::write(dir.join("SKILL.md"), body).unwrap(),
        }
        let doc = if dir.join("SKILL.md").is_file() {
            Some(parse_document(
                &fs::read_to_string(dir.join("SKILL.md")).unwrap(),
            ))
        } else {
            None
        };
        if let Some(doc) = &doc {
            if doc.yaml_error.is_none() && doc.closed && doc.opened_on_line_1 {
                assert_eq!(
                    doc.serialize(),
                    fs::read_to_string(dir.join("SKILL.md")).unwrap(),
                    "{name}"
                );
            }
        }
        let findings = validate_skill(&dir, Profile::Portable);
        let qv = quick_validate(&dir);
        let first = first_qv_error(&findings).map(|finding| finding.rule_id.as_str());
        assert_eq!(qv.ok, qv_ok, "{name} python={qv:?}");
        if qv.ok {
            assert!(
                first.is_none(),
                "{name} portable QV error {first:?} but quick_validate passed"
            );
        } else {
            assert_eq!(
                first,
                Some(qv.rule.as_str()),
                "{name} findings={findings:?}"
            );
        }
        if name == "ui-styling" {
            assert!(findings
                .iter()
                .any(|f| f.rule_id == "S-23" && f.severity == Severity::Error));
            let claude = validate_skill(&dir, Profile::ClaudeCode);
            assert!(
                claude
                    .iter()
                    .all(|finding| finding.severity != Severity::Error),
                "{claude:?}"
            );
        }
    }
}

#[test]
fn description_edit_changes_only_that_key_and_block_scalars_round_trip() {
    let original = "---\nname: demo\ndescription: Hello\nlicense: MIT\n---\nBody\n";
    let mut doc = parse_document(original);
    assert!(doc.set_string("description", "World"));
    assert_eq!(
        doc.serialize(),
        "---\nname: demo\ndescription: World\nlicense: MIT\n---\nBody\n"
    );
    let block = "---\nname: demo\ndescription: >\n  Find papers\n  quickly.\n---\nBody\n";
    let parsed = parse_document(block);
    assert_eq!(parsed.serialize(), block);
    assert!(matches!(
        parsed.get("description"),
        Some(super::frontmatter::Scalar::Str {
            style: Style::Fold,
            ..
        })
    ));
}

#[test]
fn rename_refuses_unsafe_names_and_updates_evals() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("demo");
    fs::create_dir_all(dir.join("evals")).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        skill("demo", "A skill.", "", "Body\n"),
    )
    .unwrap();
    fs::write(
        dir.join("evals").join("evals.json"),
        r#"{"skill_name":"demo","evals":[]}"#,
    )
    .unwrap();
    let err = rename_skill(&dir, "Bad Name").unwrap_err();
    assert!(matches!(err.kind, ErrorKind::InvalidInput));
    let moved = rename_skill(&dir, "demo-renamed").unwrap();
    let text = fs::read_to_string(moved.join("SKILL.md")).unwrap();
    assert!(text.contains("name: demo-renamed"));
    assert!(fs::read_to_string(moved.join("evals").join("evals.json"))
        .unwrap()
        .contains("demo-renamed"));
}

#[test]
fn templates_have_zero_errors_and_paths_stay_inside() {
    let tmp = tempfile::tempdir().unwrap();
    for id in template_ids() {
        let dest = tmp.path().join(id);
        instantiate_template(BuiltInTemplate::parse(id).unwrap(), &dest, id).unwrap();
        for profile in [Profile::Portable, Profile::ClaudeCode] {
            let errors: Vec<_> = validate_skill(&dest, profile)
                .into_iter()
                .filter(|f| f.severity == Severity::Error)
                .collect();
            assert!(errors.is_empty(), "{id} {profile:?} {errors:?}");
        }
    }
    let err = resolve_inside(tmp.path(), "../outside").unwrap_err();
    assert!(matches!(err.kind, ErrorKind::InvalidInput));
}

#[test]
fn publish_confirms_symlinks_and_marks_copies_center_changed() {
    let tmp = tempfile::tempdir().unwrap();
    let draft = tmp.path().join("draft");
    let center = tmp.path().join("center");
    let copy = tmp.path().join("copy");
    fs::create_dir_all(&draft).unwrap();
    fs::create_dir_all(&copy).unwrap();
    fs::write(draft.join("SKILL.md"), "new").unwrap();
    fs::write(copy.join("SKILL.md"), "old").unwrap();
    let deployments = vec![
        Deployment {
            agent: "claude".into(),
            mode: "symlink".into(),
            target: tmp.path().join("link"),
        },
        Deployment {
            agent: "codex".into(),
            mode: "copy".into(),
            target: copy.clone(),
        },
    ];
    assert_eq!(plan_publish(&deployments), vec!["claude".to_string()]);
    let held = publish_draft(&draft, &center, &deployments, false).unwrap();
    assert!(!held.wrote);
    assert!(!center.exists());
    let wrote = publish_draft(&draft, &center, &deployments, true).unwrap();
    assert!(wrote.wrote);
    assert_eq!(wrote.center_changed, vec!["codex".to_string()]);
    assert_eq!(deployment_status(&center, &copy), "center_changed");
    assert_eq!(fs::read_to_string(copy.join("SKILL.md")).unwrap(), "old");
}

#[test]
fn versions_dedup_pin_restore_and_history_schema() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("demo");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("SKILL.md"), "one").unwrap();
    let first = save_autosave(&dir).unwrap();
    let again = save_autosave(&dir).unwrap();
    assert_eq!(first.id, again.id);
    fs::write(dir.join("SKILL.md"), "two").unwrap();
    let second = save_autosave(&dir).unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(second.parent_id.as_deref(), Some(first.id.as_str()));
    let store = VersionStore::open(&dir);
    store.pin(&first.id, "run-1").unwrap();
    let err = hide_version(&dir, &first.id).unwrap_err();
    assert!(matches!(err.kind, ErrorKind::InvalidInput));
    assert!(err.message.contains("run-1"));
    fs::write(dir.join("SKILL.md"), "draft-before-restore").unwrap();
    let restored = restore_version(&dir, &first.id).unwrap();
    assert_eq!(restored.parent_id.as_deref(), Some(first.id.as_str()));
    assert_eq!(fs::read_to_string(dir.join("SKILL.md")).unwrap(), "one");
    let versions = VersionStore::open(&dir).versions().unwrap();
    assert!(versions
        .iter()
        .any(|version| version.content_hash != first.content_hash
            && version.source == super::versions::VersionSource::Autosave));
    let history = export_history_json(&dir, &tmp.path().join("history.json")).unwrap();
    let iterations = history["iterations"].as_array().unwrap();
    assert_eq!(
        iterations
            .iter()
            .filter(|item| item["is_current_best"] == true)
            .count(),
        1
    );
    for item in iterations {
        assert!(
            ["baseline", "won", "lost", "tie"].contains(&item["grading_result"].as_str().unwrap())
        );
    }
    let best = iterations
        .iter()
        .find(|item| item["is_current_best"] == true)
        .unwrap();
    assert_eq!(history["current_best"], best["version"]);
}

#[test]
fn agent_rules_export_and_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp
        .path()
        .join(".claude")
        .join("agents")
        .join("reviewer.md");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "---\nname: reviewer\ndescription: Reviews code.\ndisallowed_tools: Bash\ntools: Read, Skill\nmodel: sonnet\n---\nYou review.\n").unwrap();
    let findings = validate_agent(&path, false);
    assert!(findings.iter().any(|f| f.rule_id == "A-05"));
    assert!(findings.iter().any(|f| f.rule_id == "A-06"));
    assert!(apply_agent_fix(&path, "rename-key").unwrap());
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("disallowedTools: Bash"));
    assert!(!text.contains("disallowed_tools"));
    let rendered = render_agent_file(&text).unwrap();
    let target = tmp.path().join("user").join("reviewer.md");
    let exported = export_agent(AgentExportRequest {
        source: &path,
        target: &target,
        expected_hash: None,
    })
    .unwrap();
    assert_eq!(exported.bytes, rendered.into_bytes());
    assert_eq!(
        export_status(&exported.hash, Some(&exported.hash), Some(&exported.hash)),
        "current"
    );
    assert_eq!(
        export_status("new", Some(&exported.hash), Some(&exported.hash)),
        "stale"
    );
    let foreign = tmp.path().join("foreign.md");
    fs::write(&foreign, "not ours").unwrap();
    let err = export_agent(AgentExportRequest {
        source: &path,
        target: &foreign,
        expected_hash: Some("nope"),
    })
    .unwrap_err();
    assert!(matches!(err.kind, ErrorKind::TargetConflict));
    assert_eq!(fs::read_to_string(&foreign).unwrap(), "not ours");
    let json = render_agents_cli_json(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(json["reviewer"]["model"], "sonnet");
    assert!(json["reviewer"]["prompt"]
        .as_str()
        .unwrap()
        .contains("You review."));
    let late = tmp.path().join("late.md");
    fs::write(&late, "hello\n---\nname: late\ndescription: Late.\n---\n").unwrap();
    let late_findings = validate_agent(&late, false);
    assert!(late_findings
        .iter()
        .any(|f| f.severity == Severity::Error && f.message.contains("---")));
}

#[test]
fn command_rules_and_agent_rows() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp
        .path()
        .join(".claude")
        .join("commands")
        .join("docs")
        .join("review.md");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        "---\nname: review\ndescription: Review the diff.\n---\n",
    )
    .unwrap();
    let findings = validate_command_file(&path, &["docs:review".into()]);
    assert!(findings
        .iter()
        .any(|f| f.rule_id == "C-01" && f.severity == Severity::Error));
    assert!(findings.iter().any(|f| f.rule_id == "C-02"));
    let store = SkillStore::new(&tmp.path().join("hive.db")).unwrap();
    store
        .upsert_agent(
            "reviewer",
            "reviewer",
            Some("Reviews"),
            "[]",
            "agents/reviewer.json",
            1,
        )
        .unwrap();
    let rows = store.list_agents().unwrap();
    assert_eq!(rows[0].id, "reviewer");
}

#[test]
fn body_gauge_references_and_silent_skip_reasons() {
    let tmp = tempfile::tempdir().unwrap();
    write_skill(
        tmp.path(),
        "gauge",
        &[("SKILL.md", &{
            let mut body = "---\nname: gauge\ndescription: A long body.\n---\n".to_string();
            for index in 0..500 {
                body.push_str(&format!("line {index}\n"));
            }
            body
        })],
    );
    let findings = validate_skill(&tmp.path().join("gauge"), Profile::Portable);
    assert!(findings
        .iter()
        .any(|f| f.rule_id == "S-27" && f.severity == Severity::Warning));
    write_skill(
        tmp.path(),
        "refs",
        &[(
            "SKILL.md",
            "---\nname: refs\ndescription: Missing file.\n---\nSee scripts/missing.py now.\n",
        )],
    );
    let findings = validate_skill(&tmp.path().join("refs"), Profile::Portable);
    assert!(findings.iter().any(|f| f.rule_id == "S-30"));
    write_skill(
        tmp.path(),
        "evals-name",
        &[
            (
                "SKILL.md",
                "---\nname: evals-name\ndescription: Evals.\n---\n",
            ),
            (
                "evals/evals.json",
                r#"{"skill_name":"other","evals":[{"id":1,"prompt":"Go","expectations":["done"]}]}"#,
            ),
        ],
    );
    assert!(
        validate_skill(&tmp.path().join("evals-name"), Profile::Portable)
            .iter()
            .any(|f| f.rule_id == "S-33")
    );
    write_skill(
        tmp.path(),
        "graded",
        &[
            ("SKILL.md", "---\nname: graded\ndescription: Graded.\n---\n"),
            (
                "grading.json",
                r#"{"expectations":[{"text":"ok","passed":true}]}"#,
            ),
        ],
    );
    assert!(
        validate_skill(&tmp.path().join("graded"), Profile::Portable)
            .iter()
            .any(|f| f.rule_id == "S-34")
    );
}

#[derive(Debug)]
struct Qv {
    ok: bool,
    rule: String,
}

fn quick_validate(dir: &Path) -> Qv {
    let python = python_bin();
    let script =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/authoring/quick_validate.py");
    let output = Command::new(&python.0)
        .args(&python.1)
        .arg(&script)
        .arg(dir)
        .output()
        .expect("python");
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        panic!(
            "quick_validate stdout: {} stderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    Qv {
        ok: parsed["ok"].as_bool().unwrap_or(false),
        rule: parsed["rule"].as_str().unwrap_or("").to_string(),
    }
}

fn python_bin() -> (String, Vec<String>) {
    if Command::new("python3").arg("--version").output().is_ok() {
        return ("python3".into(), Vec::new());
    }
    if cfg!(windows) {
        if Command::new("py")
            .arg("-3")
            .arg("--version")
            .output()
            .is_ok()
        {
            return ("py".into(), vec!["-3".into()]);
        }
        if Command::new("python").arg("--version").output().is_ok() {
            return ("python".into(), Vec::new());
        }
        return ("python".into(), Vec::new());
    }
    ("python3".into(), Vec::new())
}
