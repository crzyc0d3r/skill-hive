use std::path::PathBuf;
use std::thread;
use std::time::Duration;

fn main() {
    let base = PathBuf::from(std::env::args().nth(1).expect("base dir"));
    studio_core::library::central_repo::set_runtime_base_dir_override(Some(base));
    let _lock = studio_core::util::repo_lock::RepoLock::acquire("holder").expect("lock");
    println!("held");
    let _ = std::io::Write::flush(&mut std::io::stdout());
    thread::sleep(Duration::from_millis(700));
}
