/// Run blocking disk, git, database, or process work off the UI thread.
pub async fn offload<T, F>(work: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    match tauri::async_runtime::spawn_blocking(work).await {
        Ok(value) => value,
        Err(err) => panic!("background task stopped: {err}"),
    }
}
