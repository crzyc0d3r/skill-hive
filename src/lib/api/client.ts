// Typed access to the Rust backend. Inside Tauri every call is an `invoke`; in a plain browser
// (vite dev, vitest, screenshots) the in-memory mock backend answers instead.

import type * as T from "./bindings";

export class ApiError extends Error {
  code: string;
  constructor(code: string, message: string) {
    super(message);
    this.name = "ApiError";
    this.code = code;
  }
}

type Args = Record<string, unknown> | undefined;

export interface Backend {
  call<R>(cmd: string, args?: Args): Promise<R>;
  onEvent(handler: (event: T.HiveEvent) => void): () => void;
}

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

let backendPromise: Promise<Backend> | null = null;

async function tauriBackend(): Promise<Backend> {
  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");
  return {
    async call<R>(cmd: string, args?: Args): Promise<R> {
      try {
        return (await invoke(cmd, args)) as R;
      } catch (err) {
        throw normalizeError(err);
      }
    },
    onEvent(handler) {
      let disposed = false;
      let unlisten: (() => void) | null = null;
      void listen<T.HiveEvent>("hive://event", (e) => handler(e.payload)).then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      });
      return () => {
        disposed = true;
        unlisten?.();
      };
    },
  };
}

async function mockBackend(): Promise<Backend> {
  const { createMockBackend } = await import("./mock");
  return createMockBackend();
}

export function getBackend(): Promise<Backend> {
  if (!backendPromise) {
    backendPromise = isTauri() ? tauriBackend() : mockBackend();
  }
  return backendPromise;
}

/** Test hook: replace the backend. */
export function setBackend(backend: Backend | null) {
  backendPromise = backend ? Promise.resolve(backend) : null;
}

function normalizeError(err: unknown): ApiError {
  if (err instanceof ApiError) return err;
  if (err && typeof err === "object" && "message" in err) {
    const e = err as { code?: string; message: string };
    return new ApiError(e.code ?? "error", String(e.message));
  }
  return new ApiError("error", typeof err === "string" ? err : "Unexpected error");
}

export async function call<R>(cmd: string, args?: Args): Promise<R> {
  const backend = await getBackend();
  try {
    return await backend.call<R>(cmd, args);
  } catch (err) {
    throw normalizeError(err);
  }
}

export function onHiveEvent(handler: (event: T.HiveEvent) => void): () => void {
  let dispose: (() => void) | null = null;
  let cancelled = false;
  void getBackend().then((b) => {
    if (cancelled) return;
    dispose = b.onEvent(handler);
  });
  return () => {
    cancelled = true;
    dispose?.();
  };
}

export interface AppInfo {
  version: string;
  platform: string;
  arch: string;
}

/** Every backend command, typed. Names mirror the Rust command names. */
export const api = {
  appInfo: () => call<AppInfo>("app_info"),
  dashboard: () => call<T.Dashboard>("dashboard"),
  settings: () => call<T.Settings>("settings_get"),
  updateSettings: (patch: T.SettingsPatch) => call<T.Settings>("settings_update", { patch }),
  doctor: (reindex = false) => call<T.DoctorReport>("doctor", { reindex }),

  // library
  listItems: (query: T.LibraryQuery) => call<T.LibraryItem[]>("library_list", { query }),
  stats: () => call<T.LibraryStats>("library_stats"),
  itemDetail: (id: string) => call<T.LibraryItemDetail>("library_get", { id }),
  readItemFile: (id: string, path: string) => call<string>("library_read_file", { id, path }),
  importItems: (request: T.ImportRequest) => call<T.ImportResult>("library_import", { request }),
  removeItem: (id: string) => call<T.LibraryItem>("library_remove", { id }),
  setTags: (id: string, tags: string[]) => call<T.LibraryItem>("library_set_tags", { id, tags }),
  setFavorite: (id: string, favorite: boolean) => call<T.LibraryItem>("library_set_favorite", { id, favorite }),
  setNotes: (id: string, notes: string | null) => call<T.LibraryItem>("library_set_notes", { id, notes }),
  createSkill: (name: string, description: string, body: string, tags: string[]) =>
    call<T.LibraryItem>("library_create_skill", { name, description, body, tags }),
  writeDocument: (id: string, text: string) => call<T.LibraryItem>("library_write_document", { id, text }),
  refreshItem: (id: string) => call<T.LibraryItem>("library_refresh", { id }),
  analyzeItems: (ids: string[], targetId?: string | null) =>
    call<T.ConflictReport>("library_analyze", { ids, targetId: targetId ?? null }),

  // discovery & sources
  scanLocal: (extra: string[] = []) => call<T.ScanReport>("scan_local", { extra }),
  scanPaths: (paths: string[]) => call<T.ScanReport>("scan_paths", { paths }),
  parseSource: (input: string) => call<T.SourceSpec>("source_parse", { input }),
  previewSource: (input: string) => call<T.SourcePreview>("source_preview", { input }),
  importSource: (input: string, paths: string[] | null, tags: string[], overwrite: boolean) =>
    call<T.ImportResult>("source_import", { input, paths, tags, overwrite }),
  checkUpdates: (ids: string[] | null = null) => call<T.UpdateCheck[]>("source_check_updates", { ids }),
  updateItem: (id: string) => call<T.LibraryItem>("source_update_item", { id }),
  catalog: (refresh = false) => call<T.CatalogEntry[]>("catalog_list", { refresh }),
  searchGithub: (query: string, limit = 20) => call<T.RepoSearchResult[]>("github_search", { query, limit }),
  clearSourceCache: () => call<number>("source_clear_cache"),

  // targets & deployments
  targets: () => call<T.TargetSpec[]>("targets_list"),
  installedTargets: () => call<T.TargetStatus[]>("targets_installed"),
  targetStatus: (targetId: string, scope: T.Scope, projectPath?: string | null) =>
    call<T.TargetStatus>("target_status", { targetId, scope, projectPath: projectPath ?? null }),
  addCustomTarget: (spec: T.TargetSpec) => call<T.TargetSpec>("target_add_custom", { spec }),
  removeCustomTarget: (id: string) => call<void>("target_remove_custom", { id }),
  setTargetSkillsDir: (id: string, dir: string | null) => call<void>("target_set_skills_dir", { id, dir }),
  projects: () => call<T.ProjectRef[]>("projects_list"),
  addProject: (path: string) => call<T.ProjectRef>("project_add", { path }),
  removeProject: (path: string) => call<void>("project_remove", { path }),
  deploy: (request: T.DeployRequest) => call<T.DeployResult>("deploy", { request }),
  undeploy: (deploymentId: string) => call<T.Deployment>("undeploy", { deploymentId }),
  undeployItem: (itemId: string) => call<T.Deployment[]>("undeploy_item", { itemId }),
  resync: (deploymentId: string) => call<T.Deployment>("resync", { deploymentId }),
  deployments: () => call<T.Deployment[]>("deployments_list"),
  removeInstalledPath: (path: string) => call<void>("remove_installed_path", { path }),

  // loadouts & recommendations
  loadouts: () => call<T.Loadout[]>("loadouts_list"),
  loadout: (id: string) => call<T.Loadout>("loadout_get", { id }),
  createLoadout: (input: T.LoadoutInput) => call<T.Loadout>("loadout_create", { input }),
  updateLoadout: (id: string, input: T.LoadoutInput) => call<T.Loadout>("loadout_update", { id, input }),
  deleteLoadout: (id: string) => call<void>("loadout_delete", { id }),
  planLoadout: (request: T.ApplyLoadoutRequest) => call<T.LoadoutPlan>("loadout_plan", { request }),
  applyLoadout: (request: T.ApplyLoadoutRequest) => call<T.ApplyLoadoutResult>("loadout_apply", { request }),
  recommend: (request: T.RecommendRequest) => call<T.RecommendResponse>("recommend", { request }),

  // providers
  providers: () => call<T.ProviderConfig[]>("providers_list"),
  saveProvider: (id: string | null, input: T.ProviderInput) => call<T.ProviderConfig>("provider_save", { id, input }),
  deleteProvider: (id: string) => call<void>("provider_delete", { id }),
  probeProvider: (id: string) => call<T.ProviderProbe>("provider_probe", { id }),
  detectClis: () => call<T.DetectedCli[]>("providers_detect"),
  ensureDefaultProviders: () => call<T.ProviderConfig[]>("providers_ensure_defaults"),

  // evaluations
  evalSuite: (itemId: string) => call<T.EvalSuite>("eval_suite_get", { itemId }),
  saveEvalSuite: (suite: T.EvalSuite) => call<T.EvalSuite>("eval_suite_save", { suite }),
  draftEvalSuite: (itemId: string) => call<T.EvalSuite>("eval_suite_draft", { itemId }),
  defaultRunConfig: (itemId: string) => call<T.RunConfig>("eval_default_config", { itemId }),
  startRun: (config: T.RunConfig) => call<T.EvalRun>("eval_run_start", { config }),
  cancelRun: (runId: string) => call<void>("eval_run_cancel", { runId }),
  runs: (itemId: string | null = null, limit = 50) => call<T.EvalRun[]>("eval_runs_list", { itemId, limit }),
  runDetail: (runId: string) => call<T.EvalRunDetail>("eval_run_detail", { runId }),
  deleteRun: (runId: string) => call<void>("eval_run_delete", { runId }),
  history: (itemId: string) => call<T.HistoryPoint[]>("eval_history", { itemId }),
  benchmarkMarkdown: (runId: string) => call<string>("eval_benchmark_markdown", { runId }),
  caseTranscript: (runId: string, caseResultId: string) =>
    call<string>("eval_case_transcript", { runId, caseResultId }),
};

export type Api = typeof api;
