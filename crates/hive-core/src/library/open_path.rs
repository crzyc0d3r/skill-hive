use super::content_hash::hash_skill_dir;
use super::scanner::scan_roots;
use super::skill_check::classify_dirs;
use super::skill_metadata::read_skill_document;
use crate::db::skill_store::{SkillRecord, SkillStore};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SlimCard {
    id: String,
    name: String,
    description: String,
    source: String,
    tags: Vec<String>,
    enabled: bool,
    body: String,
    source_body: String,
    agents: Vec<String>,
    files: Vec<String>,
    updated_at: i64,
    update_status: String,
    locations: Vec<String>,
}

fn ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

#[test]
fn profile_library_open_steps() {
    let root = std::env::temp_dir().join(format!(
        "skill-hive-open-profile-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis())
            .unwrap_or(0)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    let mut agent_dirs = Vec::new();
    let mut skill_dirs = Vec::new();
    let mut names = Vec::new();
    for agent in 0..10 {
        let agent_dir = root.join(format!("agent-{agent}"));
        fs::create_dir_all(&agent_dir).unwrap();
        agent_dirs.push(agent_dir.clone());
        for skill in 0..50 {
            let dir = agent_dir.join(format!("skill-{skill}"));
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join("SKILL.md"),
                format!(
                    "---\nname: skill-{skill}\ndescription: Keep a long task moving across the library index.\n---\nbody {agent}\n"
                ),
            )
            .unwrap();
            names.push(format!("skill-{skill}"));
            skill_dirs.push(dir);
        }
    }

    let detect_started = Instant::now();
    let mut detected = 0usize;
    for dir in &agent_dirs {
        if dir.exists() {
            detected += 1;
        }
    }
    for index in 0..54 {
        if root.join(format!("missing-adapter-{index}")).exists() {
            detected += 1;
        }
    }
    let detect_ms = ms(detect_started);

    let read_started = Instant::now();
    for dir in &skill_dirs {
        read_skill_document(dir).unwrap();
    }
    let read_ms = ms(read_started);

    let hash_started = Instant::now();
    for dir in &skill_dirs {
        hash_skill_dir(dir).unwrap();
    }
    let hash_ms = ms(hash_started);

    let classify_started = Instant::now();
    let mut updates = 0usize;
    for dir in &skill_dirs {
        let (status, _) = classify_dirs(dir, Some(dir));
        if status == "update_available" {
            updates += 1;
        }
    }
    let classify_ms = ms(classify_started);

    let dedupe_slow_started = Instant::now();
    let mut quadratic = Vec::new();
    for name in &names {
        if quadratic
            .iter()
            .any(|row: &String| row.eq_ignore_ascii_case(name))
        {
            continue;
        }
        quadratic.push(name.clone());
    }
    let dedupe_slow_ms = ms(dedupe_slow_started);

    let dedupe_fast_started = Instant::now();
    let mut seen = std::collections::HashMap::<String, usize>::new();
    let mut mapped = Vec::new();
    for name in &names {
        let key = name.to_ascii_lowercase();
        if seen.contains_key(&key) {
            continue;
        }
        seen.insert(key, mapped.len());
        mapped.push(name.clone());
    }
    let dedupe_fast_ms = ms(dedupe_fast_started);

    let store = SkillStore::new(&root.join("hive.db")).unwrap();
    let owned: Vec<(String, PathBuf)> = agent_dirs
        .iter()
        .enumerate()
        .map(|(index, dir)| (format!("agent-{index}"), dir.clone()))
        .collect();
    let refs: Vec<(&str, &Path)> = owned
        .iter()
        .map(|(key, dir)| (key.as_str(), dir.as_path()))
        .collect();
    let scan_started = Instant::now();
    let scanned = scan_roots(&store, &refs).unwrap();
    let scan_ms = ms(scan_started);

    let git_ms = time_git_status(&root.join("git-repo"), &skill_dirs);

    let insert_started = Instant::now();
    for (index, dir) in skill_dirs.iter().enumerate() {
        store
            .insert_skill(&SkillRecord {
                id: format!("skill-{index}"),
                name: names[index].clone(),
                description: Some("Keep a long task moving across the library index.".into()),
                source_type: "local".into(),
                source_ref: Some(dir.to_string_lossy().to_string()),
                source_ref_resolved: None,
                source_subpath: None,
                source_branch: None,
                source_revision: None,
                remote_revision: None,
                central_path: dir.to_string_lossy().to_string(),
                content_hash: None,
                enabled: true,
                created_at: 1,
                updated_at: 1,
                status: "managed".into(),
                update_status: "unknown".into(),
                last_checked_at: None,
                last_check_error: None,
            })
            .unwrap();
    }
    let insert_ms = ms(insert_started);

    let per_row_started = Instant::now();
    for index in 0..skill_dirs.len() {
        store
            .update_skill_check_state(&format!("skill-{index}"), None, "current", None)
            .unwrap();
    }
    let per_row_ms = ms(per_row_started);

    let batch: Vec<(String, String, Option<String>)> = (0..skill_dirs.len())
        .map(|index| (format!("skill-{index}"), "current".into(), None))
        .collect();
    let batch_started = Instant::now();
    store.update_skill_check_states(&batch).unwrap();
    let batch_ms = ms(batch_started);

    let cards: Vec<SlimCard> = (0..skill_dirs.len())
        .map(|index| SlimCard {
            id: format!("skill-{index}"),
            name: names[index].clone(),
            description: "Keep a long task moving across the library index.".into(),
            source: "local".into(),
            tags: vec![],
            enabled: true,
            body: String::new(),
            source_body: String::new(),
            agents: vec![format!("agent-{}", index % 10)],
            files: vec![],
            updated_at: 1,
            update_status: "current".into(),
            locations: vec![skill_dirs[index].to_string_lossy().to_string()],
        })
        .collect();
    let json_started = Instant::now();
    let json = serde_json::to_string(&cards).unwrap();
    let json_ms = ms(json_started);
    let parse_started = Instant::now();
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
    let parse_ms = ms(parse_started);

    let report = format!(
        "library open profile: 10 agent dirs x 50 skills = {}\n\
         detect exists (10 roots + 54 missing adapters): {detect_ms:.1} ms ({detected} hits)\n\
         read SKILL.md: {read_ms:.1} ms\n\
         hash_skill_dir: {hash_ms:.1} ms\n\
         classify_dirs (central == source): {classify_ms:.1} ms ({updates} updates)\n\
         name dedupe linear search: {dedupe_slow_ms:.1} ms -> {}\n\
         name dedupe map: {dedupe_fast_ms:.1} ms -> {}\n\
         scan_roots (read + hash + dedupe, not on library navigation): {scan_ms:.1} ms -> {}\n\
         git status --porcelain (click only, not on navigation): {git_ms}\n\
         sqlite insert 500 skills, one commit each: {insert_ms:.1} ms\n\
         sqlite update 500 checks, one commit each: {per_row_ms:.1} ms\n\
         sqlite update 500 checks, one transaction: {batch_ms:.1} ms\n\
         slim IPC JSON: {} bytes, serialize {json_ms:.1} ms, parse {parse_ms:.1} ms -> {}\n",
        skill_dirs.len(),
        quadratic.len(),
        mapped.len(),
        scanned.len(),
        json.len(),
        parsed.len(),
    );
    let _ = fs::write("/tmp/library-open-profile.txt", &report);
    assert!(
        batch_ms < 200.0,
        "cached status write should stay under 200 ms\n{report}"
    );
    assert!(
        parse_ms < 200.0,
        "reading the slim index should stay under 200 ms\n{report}"
    );
    assert_eq!(mapped.len(), 50);
    assert_eq!(
        store
            .get_skill_by_id("skill-0")
            .unwrap()
            .unwrap()
            .update_status,
        "current"
    );
    let _ = fs::remove_dir_all(&root);
}

fn time_git_status(repo: &Path, skills: &[PathBuf]) -> String {
    if fs::create_dir_all(repo).is_err() {
        return "unavailable".into();
    }
    let init = Command::new("git").arg("init").current_dir(repo).output();
    if init.is_err() || !init.unwrap().status.success() {
        return "git init failed".into();
    }
    for (index, dir) in skills.iter().enumerate() {
        let _ = fs::copy(dir.join("SKILL.md"), repo.join(format!("skill-{index}.md")));
    }
    let started = Instant::now();
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo)
        .output();
    match status {
        Ok(output) if output.status.success() => format!("{:.1} ms", ms(started)),
        _ => "git status failed".into(),
    }
}
