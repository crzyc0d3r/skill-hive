// React Query hooks over the API, plus event-driven cache invalidation.

import { useEffect } from "react";
import { useMutation, useQuery, useQueryClient, type QueryClient } from "@tanstack/react-query";
import { api, onHiveEvent } from "./client";
import type * as T from "./bindings";

export const keys = {
  app: ["app"] as const,
  dashboard: ["dashboard"] as const,
  settings: ["settings"] as const,
  items: (q: T.LibraryQuery) => ["items", q] as const,
  itemsAll: ["items"] as const,
  item: (id: string) => ["item", id] as const,
  stats: ["stats"] as const,
  targets: ["targets"] as const,
  installedTargets: ["targets", "installed"] as const,
  targetStatus: (id: string, scope: T.Scope, project?: string | null) =>
    ["targets", "status", id, scope, project ?? null] as const,
  projects: ["projects"] as const,
  deployments: ["deployments"] as const,
  loadouts: ["loadouts"] as const,
  loadout: (id: string) => ["loadouts", id] as const,
  providers: ["providers"] as const,
  catalog: ["catalog"] as const,
  runs: (itemId: string | null) => ["runs", itemId] as const,
  run: (id: string) => ["run", id] as const,
  suite: (itemId: string) => ["suite", itemId] as const,
  history: (itemId: string) => ["history", itemId] as const,
  analyze: (ids: readonly string[], targetId?: string | null) => ["analyze", [...ids].sort(), targetId ?? null] as const,
  sourceSpec: (input: string) => ["source", "spec", input] as const,
  sourcePreview: (input: string) => ["source", "preview", input] as const,
  githubSearch: (query: string) => ["github", "search", query] as const,
  doctor: ["doctor"] as const,
};

/** Subscribe once at the app root: backend events refresh the right queries. */
export function useHiveEvents(onEvent?: (e: T.HiveEvent) => void) {
  const qc = useQueryClient();
  useEffect(() => {
    return onHiveEvent((event) => {
      applyEventToCache(qc, event);
      onEvent?.(event);
    });
  }, [qc, onEvent]);
}

export function applyEventToCache(qc: QueryClient, event: T.HiveEvent) {
  switch (event.type) {
    case "library_changed":
      void qc.invalidateQueries({ queryKey: keys.itemsAll });
      void qc.invalidateQueries({ queryKey: ["item"] });
      void qc.invalidateQueries({ queryKey: ["analyze"] });
      void qc.invalidateQueries({ queryKey: keys.stats });
      void qc.invalidateQueries({ queryKey: keys.dashboard });
      break;
    case "deployments_changed":
      void qc.invalidateQueries({ queryKey: keys.deployments });
      void qc.invalidateQueries({ queryKey: keys.targets });
      void qc.invalidateQueries({ queryKey: ["item"] });
      void qc.invalidateQueries({ queryKey: keys.itemsAll });
      void qc.invalidateQueries({ queryKey: keys.dashboard });
      break;
    case "run_updated":
      void qc.invalidateQueries({ queryKey: ["runs"] });
      void qc.invalidateQueries({ queryKey: keys.run(event.run_id) });
      if (event.status === "done" || event.status === "failed" || event.status === "cancelled") {
        void qc.invalidateQueries({ queryKey: ["history"] });
        void qc.invalidateQueries({ queryKey: keys.itemsAll });
        void qc.invalidateQueries({ queryKey: ["item"] });
        void qc.invalidateQueries({ queryKey: keys.dashboard });
      }
      break;
    case "case_finished":
      void qc.invalidateQueries({ queryKey: keys.run(event.run_id) });
      break;
    default:
      break;
  }
}

export const useAppInfo = () => useQuery({ queryKey: keys.app, queryFn: api.appInfo, staleTime: Infinity });
export const useDashboard = () => useQuery({ queryKey: keys.dashboard, queryFn: api.dashboard });
export const useSettings = () => useQuery({ queryKey: keys.settings, queryFn: api.settings, staleTime: 60_000 });
export const useItems = (query: T.LibraryQuery) =>
  useQuery({ queryKey: keys.items(query), queryFn: () => api.listItems(query), placeholderData: (prev) => prev });
export const useItem = (id: string | null) =>
  useQuery({ queryKey: keys.item(id ?? ""), queryFn: () => api.itemDetail(id!), enabled: !!id });
export const useStats = () => useQuery({ queryKey: keys.stats, queryFn: api.stats });
export const useTargets = () => useQuery({ queryKey: keys.targets, queryFn: api.targets, staleTime: 60_000 });
export const useInstalledTargets = () => useQuery({ queryKey: keys.installedTargets, queryFn: api.installedTargets });
export const useTargetStatus = (id: string | null, scope: T.Scope, project?: string | null) =>
  useQuery({
    queryKey: keys.targetStatus(id ?? "", scope, project),
    queryFn: () => api.targetStatus(id!, scope, project),
    enabled: !!id && (scope === "user" || !!project),
  });
export const useProjects = () => useQuery({ queryKey: keys.projects, queryFn: api.projects });
export const useDeployments = () => useQuery({ queryKey: keys.deployments, queryFn: api.deployments });
export const useLoadouts = () => useQuery({ queryKey: keys.loadouts, queryFn: api.loadouts });
export const useProviders = () => useQuery({ queryKey: keys.providers, queryFn: api.providers });
export const useCatalog = () => useQuery({ queryKey: keys.catalog, queryFn: () => api.catalog(false), staleTime: 300_000 });
export const useRuns = (itemId: string | null = null) =>
  useQuery({ queryKey: keys.runs(itemId), queryFn: () => api.runs(itemId) });
export const useRun = (runId: string | null) =>
  useQuery({
    queryKey: keys.run(runId ?? ""),
    queryFn: () => api.runDetail(runId!),
    enabled: !!runId,
    refetchInterval: (q) => {
      const status = q.state.data?.run.status;
      return status === "running" || status === "grading" || status === "queued" ? 1500 : false;
    },
  });
export const useSuite = (itemId: string | null) =>
  useQuery({ queryKey: keys.suite(itemId ?? ""), queryFn: () => api.evalSuite(itemId!), enabled: !!itemId });
export const useHistory = (itemId: string | null) =>
  useQuery({ queryKey: keys.history(itemId ?? ""), queryFn: () => api.history(itemId!), enabled: !!itemId });
export const useDoctor = () => useQuery({ queryKey: keys.doctor, queryFn: () => api.doctor(false), staleTime: 60_000 });
export const useLoadout = (id: string | null) =>
  useQuery({ queryKey: keys.loadout(id ?? ""), queryFn: () => api.loadout(id!), enabled: !!id });
/** Conflicts among a set of library items, optionally against a target's budget. */
export const useAnalyze = (ids: readonly string[], targetId?: string | null) =>
  useQuery({
    queryKey: keys.analyze(ids, targetId),
    queryFn: () => api.analyzeItems([...ids], targetId),
    enabled: ids.length > 0,
    placeholderData: (prev) => prev,
  });
/** Parses what the user typed into a source spec; errors are the feedback, so no retries. */
export const useSourceSpec = (input: string) =>
  useQuery({
    queryKey: keys.sourceSpec(input),
    queryFn: () => api.parseSource(input),
    enabled: input.trim().length > 0,
    retry: false,
    staleTime: 60_000,
  });
export const useSourcePreview = (input: string | null) =>
  useQuery({
    queryKey: keys.sourcePreview(input ?? ""),
    queryFn: () => api.previewSource(input!),
    enabled: !!input,
    retry: false,
    staleTime: Infinity,
  });
export const useGithubSearch = (query: string) =>
  useQuery({
    queryKey: keys.githubSearch(query),
    queryFn: () => api.searchGithub(query),
    enabled: query.trim().length > 0,
    retry: false,
    staleTime: 60_000,
  });

/** Generic mutation that invalidates a set of keys on success. */
export function useApiMutation<TArgs, TResult>(
  fn: (args: TArgs) => Promise<TResult>,
  invalidate: readonly (readonly unknown[])[] = [],
  onSuccess?: (result: TResult, args: TArgs) => void,
) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: fn,
    onSuccess: (result, args) => {
      for (const key of invalidate) void qc.invalidateQueries({ queryKey: key as unknown[] });
      onSuccess?.(result, args);
    },
  });
}
