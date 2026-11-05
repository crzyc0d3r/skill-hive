//! Tauri commands: thin async wrappers over the [`Hive`] facade.
//!
//! Every command is `async` so it runs off the main thread. Blocking file-system work goes
//! through `spawn_blocking`.

use std::sync::Arc;

use hive_core::Hive;
use hive_core::model::*;
use tauri::State;

use crate::error::{CmdResult, CommandError};

type HiveState<'a> = State<'a, Arc<Hive>>;

async fn blocking<T, F>(hive: &Arc<Hive>, f: F) -> CmdResult<T>
where
    T: Send + 'static,
    F: FnOnce(Arc<Hive>) -> hive_core::Result<T> + Send + 'static,
{
    let hive = hive.clone();
    tauri::async_runtime::spawn_blocking(move || f(hive))
        .await
        .map_err(|e| CommandError::from(format!("task failed: {e}")))?
        .map_err(CommandError::from)
}

// ---- app / settings ---------------------------------------------------------------------------

#[tauri::command]
pub async fn app_info() -> CmdResult<serde_json::Value> {
    Ok(serde_json::json!({
        "version": hive_core::version(),
        "platform": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    }))
}

/// Lets the interface put a line in the desktop log: `ready <version>` once it has rendered and
/// the backend answered, or an error when it failed to start. The desktop smoke test reads these.
#[tauri::command]
pub async fn ui_report(level: String, message: String) -> CmdResult<()> {
    match level.as_str() {
        "error" => tracing::error!("ui: {message}"),
        "warn" => tracing::warn!("ui: {message}"),
        _ => tracing::info!("ui: {message}"),
    }
    Ok(())
}

#[tauri::command]
pub async fn dashboard(hive: HiveState<'_>) -> CmdResult<Dashboard> {
    blocking(&hive, |h| h.dashboard()).await
}

#[tauri::command]
pub async fn settings_get(hive: HiveState<'_>) -> CmdResult<Settings> {
    Ok(hive.settings()?)
}

#[tauri::command]
pub async fn settings_update(hive: HiveState<'_>, patch: SettingsPatch) -> CmdResult<Settings> {
    Ok(hive.update_settings(&patch)?)
}

#[tauri::command]
pub async fn doctor(hive: HiveState<'_>, reindex: bool) -> CmdResult<DoctorReport> {
    blocking(&hive, move |h| h.doctor(reindex)).await
}

// ---- library ----------------------------------------------------------------------------------

#[tauri::command]
pub async fn library_list(hive: HiveState<'_>, query: LibraryQuery) -> CmdResult<Vec<LibraryItem>> {
    Ok(hive.list_items(&query)?)
}

#[tauri::command]
pub async fn library_stats(hive: HiveState<'_>) -> CmdResult<LibraryStats> {
    Ok(hive.stats()?)
}

#[tauri::command]
pub async fn library_get(hive: HiveState<'_>, id: String) -> CmdResult<LibraryItemDetail> {
    blocking(&hive, move |h| h.item_detail(&id)).await
}

#[tauri::command]
pub async fn library_read_file(hive: HiveState<'_>, id: String, path: String) -> CmdResult<String> {
    blocking(&hive, move |h| h.read_item_file(&id, &path)).await
}

#[tauri::command]
pub async fn library_import(
    hive: HiveState<'_>,
    request: ImportRequest,
) -> CmdResult<ImportResult> {
    blocking(&hive, move |h| h.import(&request)).await
}

#[tauri::command]
pub async fn library_remove(hive: HiveState<'_>, id: String) -> CmdResult<LibraryItem> {
    blocking(&hive, move |h| h.remove_item(&id)).await
}

#[tauri::command]
pub async fn library_set_tags(
    hive: HiveState<'_>,
    id: String,
    tags: Vec<String>,
) -> CmdResult<LibraryItem> {
    Ok(hive.set_tags(&id, &tags)?)
}

#[tauri::command]
pub async fn library_set_favorite(
    hive: HiveState<'_>,
    id: String,
    favorite: bool,
) -> CmdResult<LibraryItem> {
    Ok(hive.set_favorite(&id, favorite)?)
}

#[tauri::command]
pub async fn library_set_notes(
    hive: HiveState<'_>,
    id: String,
    notes: Option<String>,
) -> CmdResult<LibraryItem> {
    Ok(hive.set_notes(&id, notes.as_deref())?)
}

#[tauri::command]
pub async fn library_create_skill(
    hive: HiveState<'_>,
    name: String,
    description: String,
    body: String,
    tags: Vec<String>,
) -> CmdResult<LibraryItem> {
    blocking(&hive, move |h| {
        h.create_skill(&name, &description, &body, &tags)
    })
    .await
}

#[tauri::command]
pub async fn library_write_document(
    hive: HiveState<'_>,
    id: String,
    text: String,
) -> CmdResult<LibraryItem> {
    blocking(&hive, move |h| h.write_document(&id, &text)).await
}

#[tauri::command]
pub async fn library_refresh(hive: HiveState<'_>, id: String) -> CmdResult<LibraryItem> {
    blocking(&hive, move |h| h.refresh_item(&id)).await
}

#[tauri::command]
pub async fn library_analyze(
    hive: HiveState<'_>,
    ids: Vec<String>,
    target_id: Option<String>,
) -> CmdResult<ConflictReport> {
    blocking(&hive, move |h| h.analyze_items(&ids, target_id.as_deref())).await
}

// ---- discovery & sources ----------------------------------------------------------------------

#[tauri::command]
pub async fn scan_local(hive: HiveState<'_>, extra: Vec<String>) -> CmdResult<ScanReport> {
    blocking(&hive, move |h| h.scan_local(&extra)).await
}

#[tauri::command]
pub async fn scan_paths(hive: HiveState<'_>, paths: Vec<String>) -> CmdResult<ScanReport> {
    blocking(&hive, move |h| h.scan_paths(&paths)).await
}

#[tauri::command]
pub async fn source_parse(hive: HiveState<'_>, input: String) -> CmdResult<SourceSpec> {
    Ok(hive.parse_source(&input)?)
}

#[tauri::command]
pub async fn source_preview(hive: HiveState<'_>, input: String) -> CmdResult<SourcePreview> {
    Ok(hive.preview_source(&input).await?)
}

#[tauri::command]
pub async fn source_import(
    hive: HiveState<'_>,
    input: String,
    paths: Option<Vec<String>>,
    tags: Vec<String>,
    overwrite: bool,
) -> CmdResult<ImportResult> {
    Ok(hive
        .import_from_source(&input, paths.as_deref(), &tags, overwrite)
        .await?)
}

#[tauri::command]
pub async fn source_check_updates(
    hive: HiveState<'_>,
    ids: Option<Vec<String>>,
) -> CmdResult<Vec<UpdateCheck>> {
    Ok(hive.check_updates(ids.as_deref()).await?)
}

#[tauri::command]
pub async fn source_update_item(hive: HiveState<'_>, id: String) -> CmdResult<LibraryItem> {
    Ok(hive.update_item_from_source(&id).await?)
}

#[tauri::command]
pub async fn catalog_list(hive: HiveState<'_>, refresh: bool) -> CmdResult<Vec<CatalogEntry>> {
    Ok(hive.catalog(refresh).await?)
}

#[tauri::command]
pub async fn github_search(
    hive: HiveState<'_>,
    query: String,
    limit: Option<u32>,
) -> CmdResult<Vec<RepoSearchResult>> {
    Ok(hive.search_github(&query, limit.unwrap_or(20)).await?)
}

#[tauri::command]
pub async fn source_clear_cache(hive: HiveState<'_>) -> CmdResult<u64> {
    blocking(&hive, |h| h.clear_source_cache()).await
}

// ---- targets, projects, deployments -----------------------------------------------------------

#[tauri::command]
pub async fn targets_list(hive: HiveState<'_>) -> CmdResult<Vec<TargetSpec>> {
    Ok(hive.targets())
}

#[tauri::command]
pub async fn targets_installed(hive: HiveState<'_>) -> CmdResult<Vec<TargetStatus>> {
    blocking(&hive, |h| h.installed_target_statuses()).await
}

#[tauri::command]
pub async fn target_status(
    hive: HiveState<'_>,
    target_id: String,
    scope: Scope,
    project_path: Option<String>,
) -> CmdResult<TargetStatus> {
    blocking(&hive, move |h| {
        h.target_status(&target_id, scope, project_path.as_deref())
    })
    .await
}

#[tauri::command]
pub async fn target_add_custom(hive: HiveState<'_>, spec: TargetSpec) -> CmdResult<TargetSpec> {
    Ok(hive.add_custom_target(spec)?)
}

#[tauri::command]
pub async fn target_remove_custom(hive: HiveState<'_>, id: String) -> CmdResult<()> {
    Ok(hive.remove_custom_target(&id)?)
}

#[tauri::command]
pub async fn target_set_skills_dir(
    hive: HiveState<'_>,
    id: String,
    dir: Option<String>,
) -> CmdResult<()> {
    Ok(hive.set_target_skills_dir(&id, dir.as_deref())?)
}

#[tauri::command]
pub async fn projects_list(hive: HiveState<'_>) -> CmdResult<Vec<ProjectRef>> {
    Ok(hive.projects()?)
}

#[tauri::command]
pub async fn project_add(hive: HiveState<'_>, path: String) -> CmdResult<ProjectRef> {
    Ok(hive.add_project(&path)?)
}

#[tauri::command]
pub async fn project_remove(hive: HiveState<'_>, path: String) -> CmdResult<()> {
    Ok(hive.remove_project(&path)?)
}

#[tauri::command]
pub async fn deploy(hive: HiveState<'_>, request: DeployRequest) -> CmdResult<DeployResult> {
    blocking(&hive, move |h| h.deploy(&request)).await
}

#[tauri::command]
pub async fn undeploy(hive: HiveState<'_>, deployment_id: String) -> CmdResult<Deployment> {
    blocking(&hive, move |h| h.undeploy(&deployment_id)).await
}

#[tauri::command]
pub async fn undeploy_item(hive: HiveState<'_>, item_id: String) -> CmdResult<Vec<Deployment>> {
    blocking(&hive, move |h| h.undeploy_item(&item_id)).await
}

#[tauri::command]
pub async fn resync(hive: HiveState<'_>, deployment_id: String) -> CmdResult<Deployment> {
    blocking(&hive, move |h| h.resync(&deployment_id)).await
}

#[tauri::command]
pub async fn deployments_list(hive: HiveState<'_>) -> CmdResult<Vec<Deployment>> {
    blocking(&hive, |h| h.deployments()).await
}

#[tauri::command]
pub async fn remove_installed_path(hive: HiveState<'_>, path: String) -> CmdResult<()> {
    blocking(&hive, move |h| h.remove_installed_path(&path)).await
}

// ---- loadouts & recommendations ---------------------------------------------------------------

#[tauri::command]
pub async fn loadouts_list(hive: HiveState<'_>) -> CmdResult<Vec<Loadout>> {
    Ok(hive.loadouts()?)
}

#[tauri::command]
pub async fn loadout_get(hive: HiveState<'_>, id: String) -> CmdResult<Loadout> {
    Ok(hive.loadout(&id)?)
}

#[tauri::command]
pub async fn loadout_create(hive: HiveState<'_>, input: LoadoutInput) -> CmdResult<Loadout> {
    Ok(hive.create_loadout(&input)?)
}

#[tauri::command]
pub async fn loadout_update(
    hive: HiveState<'_>,
    id: String,
    input: LoadoutInput,
) -> CmdResult<Loadout> {
    Ok(hive.update_loadout(&id, &input)?)
}

#[tauri::command]
pub async fn loadout_delete(hive: HiveState<'_>, id: String) -> CmdResult<()> {
    Ok(hive.delete_loadout(&id)?)
}

#[tauri::command]
pub async fn loadout_plan(
    hive: HiveState<'_>,
    request: ApplyLoadoutRequest,
) -> CmdResult<LoadoutPlan> {
    blocking(&hive, move |h| h.plan_loadout(&request)).await
}

#[tauri::command]
pub async fn loadout_apply(
    hive: HiveState<'_>,
    request: ApplyLoadoutRequest,
) -> CmdResult<ApplyLoadoutResult> {
    blocking(&hive, move |h| h.apply_loadout(&request)).await
}

#[tauri::command]
pub async fn recommend(
    hive: HiveState<'_>,
    request: RecommendRequest,
) -> CmdResult<RecommendResponse> {
    Ok(hive.recommend(&request).await?)
}

// ---- merging skills ---------------------------------------------------------------------------

#[tauri::command]
pub async fn merge_propose(hive: HiveState<'_>, request: MergeRequest) -> CmdResult<MergeProposal> {
    Ok(hive.propose_merge(&request).await?)
}

#[tauri::command]
pub async fn merge_apply(hive: HiveState<'_>, commit: MergeCommit) -> CmdResult<MergeResult> {
    blocking(&hive, move |h| h.apply_merge(&commit)).await
}

// ---- providers --------------------------------------------------------------------------------

#[tauri::command]
pub async fn providers_list(hive: HiveState<'_>) -> CmdResult<Vec<ProviderConfig>> {
    Ok(hive.list_providers()?)
}

#[tauri::command]
pub async fn provider_save(
    hive: HiveState<'_>,
    id: Option<String>,
    input: ProviderInput,
) -> CmdResult<ProviderConfig> {
    Ok(hive.save_provider(id.as_deref(), &input)?)
}

#[tauri::command]
pub async fn provider_delete(hive: HiveState<'_>, id: String) -> CmdResult<()> {
    Ok(hive.delete_provider(&id)?)
}

#[tauri::command]
pub async fn provider_probe(hive: HiveState<'_>, id: String) -> CmdResult<ProviderProbe> {
    Ok(hive.probe_provider(&id).await?)
}

#[tauri::command]
pub async fn providers_detect() -> CmdResult<Vec<DetectedCli>> {
    tauri::async_runtime::spawn_blocking(hive_core::providers::detect_clis)
        .await
        .map_err(|e| CommandError::from(format!("task failed: {e}")))
}

#[tauri::command]
pub async fn providers_ensure_defaults(hive: HiveState<'_>) -> CmdResult<Vec<ProviderConfig>> {
    blocking(&hive, |h| h.ensure_default_providers()).await
}

// ---- evaluations ------------------------------------------------------------------------------

#[tauri::command]
pub async fn eval_suite_get(hive: HiveState<'_>, item_id: String) -> CmdResult<EvalSuite> {
    blocking(&hive, move |h| h.eval_suite(&item_id)).await
}

#[tauri::command]
pub async fn eval_suite_save(hive: HiveState<'_>, suite: EvalSuite) -> CmdResult<EvalSuite> {
    blocking(&hive, move |h| h.save_eval_suite(&suite)).await
}

#[tauri::command]
pub async fn eval_suite_draft(hive: HiveState<'_>, item_id: String) -> CmdResult<EvalSuite> {
    blocking(&hive, move |h| h.draft_eval_suite(&item_id)).await
}

#[tauri::command]
pub async fn eval_default_config(hive: HiveState<'_>, item_id: String) -> CmdResult<RunConfig> {
    Ok(hive.default_run_config(&item_id)?)
}

#[tauri::command]
pub async fn eval_run_start(hive: HiveState<'_>, config: RunConfig) -> CmdResult<EvalRun> {
    let hive: Arc<Hive> = hive.inner().clone();
    Ok(hive.start_run(config).await?)
}

#[tauri::command]
pub async fn eval_run_cancel(hive: HiveState<'_>, run_id: String) -> CmdResult<()> {
    Ok(hive.cancel_run(&run_id)?)
}

#[tauri::command]
pub async fn eval_runs_list(
    hive: HiveState<'_>,
    item_id: Option<String>,
    limit: Option<u32>,
) -> CmdResult<Vec<EvalRun>> {
    Ok(hive.list_runs(item_id.as_deref(), limit.unwrap_or(50))?)
}

#[tauri::command]
pub async fn eval_run_detail(hive: HiveState<'_>, run_id: String) -> CmdResult<EvalRunDetail> {
    Ok(hive.run_detail(&run_id)?)
}

#[tauri::command]
pub async fn eval_run_delete(hive: HiveState<'_>, run_id: String) -> CmdResult<()> {
    blocking(&hive, move |h| h.delete_run(&run_id)).await
}

#[tauri::command]
pub async fn eval_history(hive: HiveState<'_>, item_id: String) -> CmdResult<Vec<HistoryPoint>> {
    Ok(hive.history(&item_id)?)
}

#[tauri::command]
pub async fn eval_benchmark_markdown(hive: HiveState<'_>, run_id: String) -> CmdResult<String> {
    Ok(hive.benchmark_markdown(&run_id)?)
}

#[tauri::command]
pub async fn eval_case_transcript(
    hive: HiveState<'_>,
    run_id: String,
    case_result_id: String,
) -> CmdResult<String> {
    blocking(&hive, move |h| {
        h.read_case_transcript(&run_id, &case_result_id)
    })
    .await
}
