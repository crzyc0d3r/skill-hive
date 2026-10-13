use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use hive_core::library::central_repo;
use hive_core::util::repo_lock::RepoLock;

#[test]
fn second_process_waits_for_the_repo_lock() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().join("base");
    std::fs::create_dir_all(&base).unwrap();
    central_repo::set_runtime_base_dir_override(Some(base.clone()));

    let bin = PathBuf::from(env!("CARGO_BIN_EXE_lock-holder"));
    let mut child = Command::new(bin)
        .arg(&base)
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn lock-holder");
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(line.contains("held"), "child did not take the lock: {line}");

    let start = Instant::now();
    let waited = RepoLock::acquire_blocking("waiter", Duration::from_secs(20));
    let elapsed = start.elapsed();
    assert!(
        waited.is_ok(),
        "second process should acquire once the holder exits"
    );
    assert!(
        elapsed >= Duration::from_millis(200),
        "waiter should have blocked, elapsed {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(20),
        "wait must stay inside the 20s foreground cap, elapsed {elapsed:?}"
    );
    let _ = child.wait();
    central_repo::set_runtime_base_dir_override(None);
}
