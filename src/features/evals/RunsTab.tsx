// Runs tab for one skill (history chart + runs table) and the runs table used by the overview.

import * as React from "react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { FlaskConical, Loader2, Play, Trash2 } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation, useHistory, useProviders, useRuns } from "@/lib/api/hooks";
import { cn, formatDate, formatDelta, formatPercent, pluralize, relativeTime } from "@/lib/utils";
import { Badge, Button, ConfirmDialog, EmptyState, Progress, SectionTitle, Skeleton, Tooltip } from "@/components/ui";
import { ErrorNote, VerdictBadge } from "@/components/app/common";
import { HistoryChart } from "./charts";
import { ArmSwatch } from "./bits";
import { RUN_STATUS_META, armColors, armLabel, isActiveRun } from "./shared";

export function RunsTab({ itemId, onRun }: { itemId: string; onRun: () => void }) {
  const navigate = useNavigate();
  const history = useHistory(itemId);
  const runs = useRuns(itemId);

  if (runs.isLoading) {
    return (
      <div className="flex flex-col gap-4 pt-4">
        <Skeleton className="h-56 w-full" />
        <Skeleton className="h-40 w-full" />
      </div>
    );
  }
  if (runs.isError) return <ErrorNote error={runs.error} className="mt-4" />;
  const list = runs.data ?? [];
  if (list.length === 0) {
    return (
      <div className="pt-4">
        <EmptyState
          icon={FlaskConical}
          title="No runs yet"
          description="Run the suite with and without the skill to find out whether these instructions help. Results, deltas and a verdict appear here."
          action={
            <Button variant="primary" onClick={onRun}>
              <Play /> Run evaluation…
            </Button>
          }
        />
      </div>
    );
  }
  const points = history.data ?? [];
  return (
    <div className="flex flex-col gap-5 pt-4">
      <section className="surface p-4">
        <SectionTitle title="Pass rate over time" description={points.length ? `${pluralize(points.length, "finished run")}; click a point to open the run.` : "Appears after the first finished run."} />
        <div className="mt-3">
          {points.length ? <HistoryChart points={points} onSelect={(id) => navigate(`/runs/${id}`)} /> : <div className="rounded-lg border border-dashed px-3 py-8 text-center text-xs text-muted">No finished runs to chart yet.</div>}
        </div>
      </section>
      <section className="flex flex-col gap-3">
        <SectionTitle title={`Runs (${list.length})`} description="Newest first. Live runs update as cases finish." />
        <RunsTable runs={list} />
      </section>
    </div>
  );
}

export function RunsTable({ runs, showSkill = false }: { runs: T.EvalRun[]; showSkill?: boolean }) {
  const navigate = useNavigate();
  const { data: providers } = useProviders();
  const [pendingDelete, setPendingDelete] = React.useState<T.EvalRun | null>(null);
  const remove = useApiMutation((id: string) => api.deleteRun(id), [["runs"], ["history"], keys.itemsAll, keys.dashboard], () => {
    toast.success("Run deleted");
    setPendingDelete(null);
  });
  const providerName = (id: string | null | undefined) => (id ? (providers?.find((p) => p.id === id)?.name ?? id) : null);

  // Header and rows share one minimum width, so the surface scrolls sideways instead of the columns collapsing.
  const cols = cn(
    "grid items-center gap-3 px-3",
    showSkill
      ? "min-w-[820px] grid-cols-[minmax(0,1.5fr)_minmax(0,0.9fr)_auto_minmax(0,1.2fr)_auto_minmax(0,1fr)_minmax(0,1.2fr)_minmax(0,1fr)_auto_auto]"
      : "min-w-[700px] grid-cols-[minmax(0,1fr)_auto_minmax(0,1.2fr)_auto_minmax(0,1fr)_minmax(0,1.2fr)_minmax(0,1fr)_auto_auto]",
  );

  return (
    <>
      <div className="surface overflow-x-auto">
        <div className={cn(cols, "border-b bg-sunken/60 py-1.5 text-[11px] font-medium uppercase tracking-wide text-subtle")}>
          {showSkill ? <span>Skill</span> : null}
          <span>Created</span>
          <span>Status</span>
          <span>Arms</span>
          <span className="text-right">Repeats</span>
          <span>Executor</span>
          <span>Verdict</span>
          <span className="whitespace-nowrap text-right">With / baseline</span>
          <span className="w-14 text-right">Δ</span>
          <span className="w-7" />
        </div>
        {runs.map((r) => {
          const bench = r.summary;
          const w = bench?.arms.find((a) => a.label === "with_skill");
          const b = bench?.arms.find((a) => a.label === bench.baseline);
          const delta = bench?.deltas.find((d) => d.arm === "with_skill") ?? bench?.deltas[0];
          const active = isActiveRun(r.status);
          const colors = armColors(r.config.arms.map((a) => a.label));
          const status = RUN_STATUS_META[r.status];
          return (
            <div
              key={r.id}
              role="button"
              tabIndex={0}
              onClick={() => navigate(`/runs/${r.id}`)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  navigate(`/runs/${r.id}`);
                }
              }}
              className={cn(cols, "border-b py-2 text-xs last:border-b-0 hover:bg-sunken/60 focus-visible:bg-sunken/60")}
            >
              {showSkill ? (
                <span className="truncate text-[13px] font-medium" title={r.skillName}>
                  {r.skillName}
                </span>
              ) : null}
              <Tooltip content={formatDate(r.createdAt)}>
                <span className="truncate text-fg">{relativeTime(r.createdAt)}</span>
              </Tooltip>
              <span>
                <Badge tone={status.tone}>
                  {active ? <Loader2 className="animate-spin" /> : null}
                  {status.label}
                </Badge>
              </span>
              <span className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-0.5 text-muted">
                {r.config.arms.map((a) => (
                  <span key={a.label} className="inline-flex items-center gap-1 truncate">
                    <ArmSwatch color={colors[a.label]} /> {armLabel(a.label)}
                  </span>
                ))}
              </span>
              <span className="num text-right">{r.config.repeats}×</span>
              <span className="truncate text-muted" title={bench?.executor ?? undefined}>
                {providerName(r.config.executorProviderId) ?? bench?.executor ?? "—"}
              </span>
              {active ? (
                <span className="col-span-3 flex min-w-0 items-center gap-2">
                  <Progress value={r.progress.total ? r.progress.done / r.progress.total : 0} size="sm" className="w-24 shrink-0" tone="info" />
                  <span className="num shrink-0 text-muted">
                    {r.progress.done}/{r.progress.total || "?"}
                  </span>
                  <span className="truncate text-subtle">{r.progress.message}</span>
                </span>
              ) : bench ? (
                <>
                  <span>
                    <VerdictBadge verdict={bench.verdict} />
                  </span>
                  <span className="num text-right">
                    {w ? formatPercent(w.passRate.mean) : "—"} <span className="text-subtle">/</span> {b ? formatPercent(b.passRate.mean) : "—"}
                  </span>
                  <span className={cn("num w-14 text-right font-medium", delta && delta.passRateDelta > 0.02 ? "text-ok" : delta && delta.passRateDelta < -0.02 ? "text-danger" : "text-muted")}>{delta ? formatDelta(delta.passRateDelta) : "—"}</span>
                </>
              ) : (
                <span className="col-span-3 truncate text-muted">{r.error ? r.error : r.status === "cancelled" ? "Cancelled before a benchmark was produced" : "No benchmark"}</span>
              )}
              <span className="flex w-7 justify-end">
                <Tooltip content="Delete run">
                  <Button
                    variant="danger-ghost"
                    size="icon-sm"
                    aria-label={`Delete run ${r.id}`}
                    onClick={(e) => {
                      e.stopPropagation();
                      setPendingDelete(r);
                    }}
                  >
                    <Trash2 />
                  </Button>
                </Tooltip>
              </span>
            </div>
          );
        })}
      </div>
      <ConfirmDialog
        open={!!pendingDelete}
        onOpenChange={(o) => !o && setPendingDelete(null)}
        title="Delete this run?"
        description={pendingDelete ? `${pendingDelete.skillName} · ${formatDate(pendingDelete.createdAt)}. Case results and the benchmark are removed; the sandbox workspace is deleted too.` : undefined}
        confirmLabel="Delete"
        destructive
        loading={remove.isPending}
        onConfirm={() => {
          if (pendingDelete) remove.mutate(pendingDelete.id);
        }}
      />
    </>
  );
}
