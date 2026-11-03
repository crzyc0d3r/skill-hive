// One evaluation run: live progress while it executes, then the comparative benchmark.

import * as React from "react";
import { useNavigate, useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { toast } from "sonner";
import { ArrowLeft, CheckCircle2, ChevronDown, ChevronRight, Copy, FileText, FolderOpen, Info, Loader2, MoreHorizontal, Square, Trash2, XCircle } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation, useProviders, useRun } from "@/lib/api/hooks";
import { cn, formatDate, formatDelta, formatDuration, formatPercent, formatTokens, pluralize } from "@/lib/utils";
import {
  Badge,
  Button,
  ConfirmDialog,
  Dialog,
  DialogBody,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
  EmptyState,
  Progress,
  SectionTitle,
  SimpleSelect,
  Skeleton,
  Tooltip,
  copyText,
  revealPath,
} from "@/components/ui";
import { ErrorNote, InlineCode, Page, PageBody, PageHeader, VerdictBadge } from "@/components/app/common";
import { ArmPassRateChart } from "./charts";
import { ArmSwatch, MiniStat } from "./bits";
import { CASE_STATUS_META, PATTERN_META, RUN_STATUS_META, armColors, armLabel, formatCI, formatPValue, formatRateStats, interpretBenchmark, isActiveRun, passFractionFill } from "./shared";

export function RunPage() {
  const { runId } = useParams<{ runId: string }>();
  const navigate = useNavigate();
  const { data, isLoading, error, refetch } = useRun(runId ?? null);
  const { data: providers } = useProviders();
  const [confirmDelete, setConfirmDelete] = React.useState(false);

  const cancel = useApiMutation(() => api.cancelRun(runId!), [keys.run(runId ?? ""), ["runs"]], () => toast("Cancelling run"));
  const remove = useApiMutation(() => api.deleteRun(runId!), [["runs"], ["history"], keys.itemsAll, keys.dashboard], () => {
    toast.success("Run deleted");
    navigate(data?.run.itemId ? `/evals/${data.run.itemId}?tab=runs` : "/evals");
  });
  const copyMarkdown = useApiMutation(() => api.benchmarkMarkdown(runId!), [], async (md) => {
    await copyText(md);
    toast.success("Benchmark copied as Markdown");
  });

  if (isLoading || (!data && !error)) {
    return (
      <Page>
        <PageHeader title={<Skeleton className="h-6 w-56" />} description={<Skeleton className="h-3.5 w-96" />} />
        <PageBody className="flex flex-col gap-4">
          <Skeleton className="h-28 w-full" />
          <Skeleton className="h-64 w-full" />
        </PageBody>
      </Page>
    );
  }
  if (!data) {
    return (
      <Page>
        <PageHeader title="Run" />
        <PageBody>
          <EmptyState
            title="Could not load this run"
            description={<ErrorNote error={error} />}
            action={
              <>
                <Button onClick={() => refetch()}>Retry</Button>
                <Button variant="ghost" onClick={() => navigate("/evals")}>
                  Back to evals
                </Button>
              </>
            }
          />
        </PageBody>
      </Page>
    );
  }

  const { run, results } = data;
  const bench = run.summary ?? null;
  const active = isActiveRun(run.status);
  const status = RUN_STATUS_META[run.status];
  const providerName = (id: string | null | undefined, fallback: string | null | undefined) => (id ? (providers?.find((p) => p.id === id)?.name ?? fallback ?? id) : (fallback ?? null));
  const executor = providerName(run.config.executorProviderId, bench?.executor);
  const grader = providerName(run.config.graderProviderId, bench?.grader) ?? "checks only";
  const started = run.startedAt ?? run.createdAt;
  const took = run.finishedAt ? formatDuration(run.finishedAt - started) : null;
  const back = () => navigate(run.itemId ? `/evals/${run.itemId}?tab=runs` : "/evals");

  return (
    <Page>
      <PageHeader
        title={
          <span className="flex items-center gap-2">
            <span className="selectable">{run.skillName}</span>
            <Badge tone={status.tone}>
              {active ? <Loader2 className="animate-spin" /> : null}
              {status.label}
            </Badge>
          </span>
        }
        description={
          <span className="selectable">
            Started {formatDate(started)}
            {run.finishedAt ? ` · finished ${formatDate(run.finishedAt)}${took ? ` (${took})` : ""}` : ""} · {run.config.arms.length} arms × {run.config.repeats} {run.config.repeats === 1 ? "repeat" : "repeats"} · executor {executor ?? "—"} · grader {grader}
          </span>
        }
        actions={
          <>
            <Button variant="ghost" onClick={back}>
              <ArrowLeft /> Evals
            </Button>
            {active ? (
              <Button variant="danger-ghost" loading={cancel.isPending} onClick={() => cancel.mutate(undefined)}>
                <Square /> Cancel
              </Button>
            ) : null}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="icon" aria-label="More">
                  <MoreHorizontal />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem disabled={!bench} onSelect={() => copyMarkdown.mutate(undefined)}>
                  <Copy /> Copy benchmark as Markdown
                </DropdownMenuItem>
                <DropdownMenuItem onSelect={() => void revealPath(run.workspace)}>
                  <FolderOpen /> Reveal workspace
                </DropdownMenuItem>
                <DropdownMenuItem onSelect={() => void copyText(run.id).then(() => toast("Run id copied"))}>
                  <Copy /> Copy run id
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem destructive onSelect={() => setConfirmDelete(true)}>
                  <Trash2 /> Delete run
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </>
        }
      />
      <PageBody className="flex flex-col gap-5">
        {run.error ? <ErrorNote error={run.error} /> : null}
        {active ? <LiveCard run={run} /> : null}
        {bench ? (
          <>
            <VerdictCard bench={bench} />
            <section className="surface p-4">
              <SectionTitle title="Arm comparison" description="Mean per arm; error bars and ± are one standard deviation across repeats." />
              <div className="mt-3 grid gap-5 xl:grid-cols-[minmax(0,1fr)_minmax(0,1.8fr)]">
                <ArmPassRateChart arms={bench.arms} />
                <ArmStatsTable bench={bench} />
              </div>
            </section>
            <ExpectationMatrix bench={bench} />
            {bench.trigger ? <TriggerSection trigger={bench.trigger} /> : null}
            {bench.notes.length ? <NotesSection notes={bench.notes} /> : null}
          </>
        ) : !active ? (
          <div className="rounded-lg border border-dashed px-4 py-6 text-center text-xs text-muted">
            No benchmark: the run {run.status === "cancelled" ? "was cancelled" : run.status === "failed" ? "failed" : "has not finished"} before results could be aggregated.
            {results.length ? " Individual case results are below." : ""}
          </div>
        ) : null}
        <CaseResults runId={run.id} results={results} arms={run.config.arms.map((a) => a.label)} live={active} />
      </PageBody>

      <ConfirmDialog
        open={confirmDelete}
        onOpenChange={setConfirmDelete}
        title="Delete this run?"
        description="Case results, transcripts and the benchmark are removed. The skill's effectiveness badge falls back to the previous run."
        confirmLabel="Delete"
        destructive
        loading={remove.isPending}
        onConfirm={() => remove.mutate(undefined)}
      />
    </Page>
  );
}

function LiveCard({ run }: { run: T.EvalRun }) {
  const frac = run.progress.total ? run.progress.done / run.progress.total : 0;
  return (
    <section className="surface p-4" aria-live="polite">
      <div className="flex items-center gap-3">
        <Loader2 className="size-4 animate-spin text-info" />
        <div className="min-w-0 flex-1">
          <div className="flex items-center justify-between gap-3 text-xs">
            <span className="truncate font-medium">{run.progress.message ?? (run.status === "queued" ? "Waiting to start…" : run.status === "grading" ? "Grading…" : "Running…")}</span>
            <span className="num shrink-0 text-muted">
              {run.progress.done}/{run.progress.total || "?"}
              {run.progress.failed ? <span className="text-danger"> · {run.progress.failed} failed</span> : null}
            </span>
          </div>
          <Progress value={frac} tone="info" className="mt-2" />
        </div>
      </div>
      <p className="mt-2 text-[11px] text-subtle">Case results stream in below as each sandbox finishes. The benchmark is computed when every arm is graded.</p>
    </section>
  );
}

function VerdictCard({ bench }: { bench: T.Benchmark }) {
  const w = bench.arms.find((a) => a.label === "with_skill");
  const b = bench.arms.find((a) => a.label === bench.baseline);
  const main = bench.deltas.find((d) => d.arm === "with_skill" && d.baseline === bench.baseline) ?? bench.deltas[0];
  const others = bench.deltas.filter((d) => d !== main);
  const significant = main?.pValue !== null && main?.pValue !== undefined && main.pValue < 0.05;
  return (
    <section className="surface p-4">
      <div className="flex flex-wrap items-start gap-5">
        <div className="min-w-0 flex-1 basis-80">
          <div className="flex flex-wrap items-center gap-2">
            <VerdictBadge verdict={bench.verdict} delta={main?.passRateDelta} />
            <span className="text-xs text-muted">
              vs {armLabel(bench.baseline)}
              {main ? (
                <>
                  {" "}
                  · {pluralize(w?.runs ?? 0, "run")} per arm
                </>
              ) : null}
            </span>
          </div>
          <p className="mt-2 text-[15px] font-semibold leading-snug tracking-[-0.01em] text-balance selectable">{interpretBenchmark(bench)}</p>
          {main ? (
            <p className="mt-1.5 text-xs text-muted selectable">
              Pass-rate delta <span className="num font-medium text-fg">{formatDelta(main.passRateDelta)}</span>, 95 % CI <span className="num">{formatCI(main.ciLow, main.ciHigh)}</span>,{" "}
              <span className={cn("num", significant ? "text-fg" : "")}>{formatPValue(main.pValue)}</span>
              {main.ciLow <= 0 && main.ciHigh >= 0 ? " · the interval includes zero" : ""}
            </p>
          ) : null}
        </div>
        <div className="grid shrink-0 grid-cols-2 gap-2 sm:grid-cols-4">
          <MiniStat label="With skill" value={w ? formatPercent(w.passRate.mean) : "—"} hint={w ? `± ${Math.round(w.passRate.stddev * 100)}` : undefined} />
          <MiniStat label="Baseline" value={b ? formatPercent(b.passRate.mean) : "—"} hint={b ? `± ${Math.round(b.passRate.stddev * 100)}` : undefined} />
          <MiniStat label="Tokens" value={main ? `${main.tokensDelta > 0 ? "+" : main.tokensDelta < 0 ? "−" : ""}${formatTokens(Math.abs(main.tokensDelta))}` : "—"} hint="per run vs baseline" />
          <MiniStat label="Duration" value={main ? `${main.durationDeltaMs > 0 ? "+" : main.durationDeltaMs < 0 ? "−" : ""}${formatDuration(Math.abs(main.durationDeltaMs))}` : "—"} hint="per run vs baseline" />
        </div>
      </div>
      {others.length ? (
        <ul className="mt-3 flex flex-wrap gap-2 border-t pt-3 text-xs">
          {others.map((d) => (
            <li key={`${d.arm}-${d.baseline}`} className="flex items-center gap-2 rounded-lg bg-sunken px-2.5 py-1.5">
              <span className="font-medium">{armLabel(d.arm)}</span>
              <VerdictBadge verdict={d.verdict} delta={d.passRateDelta} />
              <span className="num text-muted">
                CI {formatCI(d.ciLow, d.ciHigh)} · {formatPValue(d.pValue)}
              </span>
            </li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}

function ArmStatsTable({ bench }: { bench: T.Benchmark }) {
  const colors = armColors(bench.arms.map((a) => a.label));
  const cell = "px-2 py-1.5 text-right num whitespace-nowrap";
  const head = "px-2 py-1.5 text-right text-[11px] font-medium uppercase tracking-wide text-subtle whitespace-nowrap";
  return (
    <div className="overflow-x-auto">
      <table className="w-full text-xs">
        <thead>
          <tr className="border-b">
            <th className={cn(head, "text-left")}>Arm</th>
            <th className={head}>Runs</th>
            <th className={head}>Errors</th>
            <th className={head}>Pass rate</th>
            <th className={head}>Tokens</th>
            <th className={head}>Duration</th>
            <th className={head}>Tool calls</th>
            <th className={head}>Cost</th>
          </tr>
        </thead>
        <tbody>
          {bench.arms.map((a) => (
            <tr key={a.label} className="border-b last:border-b-0">
              <td className="px-2 py-1.5">
                <span className="inline-flex flex-wrap items-center gap-x-1.5 font-medium">
                  <span className="inline-flex items-center gap-1.5 whitespace-nowrap">
                    <ArmSwatch color={colors[a.label]} /> {armLabel(a.label)}
                  </span>
                  {a.label === bench.baseline ? <span className="text-[11px] font-normal text-subtle">baseline</span> : null}
                </span>
              </td>
              <td className={cell}>{a.runs}</td>
              <td className={cn(cell, a.errors ? "text-danger" : "text-muted")}>{a.errors}</td>
              <td className={cn(cell, "font-medium")}>{formatRateStats(a.passRate)}</td>
              <td className={cell}>
                {formatTokens(Math.round(a.tokens.mean))} <span className="text-subtle">± {formatTokens(Math.round(a.tokens.stddev))}</span>
              </td>
              <td className={cell}>
                {formatDuration(a.durationMs.mean)} <span className="text-subtle">± {formatDuration(a.durationMs.stddev)}</span>
              </td>
              <td className={cell}>
                {a.toolCalls.mean.toFixed(1)} <span className="text-subtle">± {a.toolCalls.stddev.toFixed(1)}</span>
              </td>
              <td className={cell}>{a.costUsd !== null && a.costUsd !== undefined ? `$${a.costUsd.toFixed(2)}` : "—"}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function ExpectationMatrix({ bench }: { bench: T.Benchmark }) {
  const arms = bench.arms.map((a) => a.label);
  const colors = armColors(arms);
  const groups = React.useMemo(() => {
    const map = new Map<number, { name: string; rows: T.ExpectationRow[] }>();
    for (const row of bench.expectations) {
      const g = map.get(row.caseId) ?? { name: row.caseName, rows: [] };
      g.rows.push(row);
      map.set(row.caseId, g);
    }
    return [...map.entries()];
  }, [bench.expectations]);
  const counts = React.useMemo(() => {
    const out: Partial<Record<T.ExpectationPattern, number>> = {};
    for (const r of bench.expectations) out[r.pattern] = (out[r.pattern] ?? 0) + 1;
    return out;
  }, [bench.expectations]);

  return (
    <section className="surface p-4">
      <SectionTitle
        title="Expectation matrix"
        description="Pass fraction of every expectation in each arm, with the pattern it follows. Green passes, red fails; the pattern says what that means for the skill."
        action={
          <div className="flex flex-wrap items-center justify-end gap-1.5">
            {(Object.keys(PATTERN_META) as T.ExpectationPattern[])
              .filter((p) => counts[p])
              .map((p) => (
                <Tooltip key={p} content={PATTERN_META[p].explain}>
                  <span>
                    <Badge tone={PATTERN_META[p].tone}>
                      {counts[p]} {PATTERN_META[p].label.toLowerCase()}
                    </Badge>
                  </span>
                </Tooltip>
              ))}
          </div>
        }
      />
      <div className="mt-3 overflow-x-auto">
        <table className="w-full text-xs" aria-label="Expectation matrix">
          <thead>
            <tr className="border-b">
              <th className="px-2 py-1.5 text-left text-[11px] font-medium uppercase tracking-wide text-subtle">Expectation</th>
              {arms.map((a) => (
                <th key={a} className="w-28 whitespace-nowrap px-2 py-1.5 text-center text-[11px] font-medium uppercase tracking-wide text-subtle">
                  <span className="inline-flex items-center gap-1.5">
                    <ArmSwatch color={colors[a]} /> {armLabel(a)}
                  </span>
                </th>
              ))}
              <th className="w-36 px-2 py-1.5 text-left text-[11px] font-medium uppercase tracking-wide text-subtle">Pattern</th>
            </tr>
          </thead>
          <tbody>
            {groups.map(([caseId, g]) => (
              <React.Fragment key={caseId}>
                <tr className="bg-sunken/60">
                  <td colSpan={arms.length + 2} className="mono px-2 py-1 text-[11px] text-muted">
                    {g.name}
                  </td>
                </tr>
                {g.rows.map((row, i) => {
                  const meta = PATTERN_META[row.pattern];
                  return (
                    <tr key={`${caseId}-${i}`} className="border-b last:border-b-0">
                      <td className="px-2 py-1 selectable">{row.text}</td>
                      {arms.map((a) => {
                        const f = row.byArm[a];
                        return (
                          <td key={a} className="px-1 py-1">
                            {f === undefined ? (
                              <div className="rounded-md px-2 py-1 text-center text-subtle">—</div>
                            ) : (
                              <div className="num rounded-md px-2 py-1 text-center font-medium" style={{ background: passFractionFill(f) }} title={`${armLabel(a)}: passed ${formatPercent(f)} of repeats`}>
                                {formatPercent(f)}
                              </div>
                            )}
                          </td>
                        );
                      })}
                      <td className="px-2 py-1">
                        <Tooltip content={meta.explain}>
                          <span>
                            <Badge tone={meta.tone}>{meta.label}</Badge>
                          </span>
                        </Tooltip>
                      </td>
                    </tr>
                  );
                })}
              </React.Fragment>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function TriggerSection({ trigger }: { trigger: T.TriggerSummary }) {
  const pct = (v: number) => `${Math.round(v * 100)}%`;
  return (
    <section className="surface p-4">
      <SectionTitle title="Trigger evaluation" description="Does the description alone make the agent pick this skill? Precision: when it triggered, was it right. Recall: of the queries that should trigger, how many did." />
      <div className="mt-3 grid grid-cols-2 gap-2 sm:grid-cols-4">
        <MiniStat label="Precision" value={pct(trigger.precision)} tone={trigger.precision >= 0.8 ? "ok" : trigger.precision >= 0.5 ? "warn" : "danger"} />
        <MiniStat label="Recall" value={pct(trigger.recall)} tone={trigger.recall >= 0.8 ? "ok" : trigger.recall >= 0.5 ? "warn" : "danger"} />
        <MiniStat label="F1" value={trigger.f1.toFixed(2)} />
        <MiniStat label="Correct" value={`${trigger.correct}/${trigger.total}`} hint="queries" />
      </div>
      <div className="mt-3 overflow-x-auto">
        <table className="w-full text-xs" aria-label="Trigger results">
          <thead>
            <tr className="border-b">
              <th className="px-2 py-1.5 text-left text-[11px] font-medium uppercase tracking-wide text-subtle">Query</th>
              <th className="px-2 py-1.5 text-left text-[11px] font-medium uppercase tracking-wide text-subtle">Expected</th>
              <th className="px-2 py-1.5 text-left text-[11px] font-medium uppercase tracking-wide text-subtle">Chosen</th>
              <th className="px-2 py-1.5 text-left text-[11px] font-medium uppercase tracking-wide text-subtle">Result</th>
            </tr>
          </thead>
          <tbody>
            {trigger.results.map((r, i) => (
              <tr key={i} className="border-b last:border-b-0">
                <td className="px-2 py-1.5 selectable">{r.query}</td>
                <td className="px-2 py-1.5">
                  <Badge tone={r.shouldTrigger ? "accent" : "neutral"}>{r.shouldTrigger ? "should trigger" : "should not"}</Badge>
                </td>
                <td className="px-2 py-1.5">{r.chosen ? <InlineCode>{r.chosen}</InlineCode> : <span className="text-subtle">no skill</span>}</td>
                <td className="px-2 py-1.5">
                  <span className={cn("inline-flex items-center gap-1", r.correct ? "text-ok" : "text-danger")}>
                    {r.correct ? <CheckCircle2 className="size-3.5" /> : <XCircle className="size-3.5" />}
                    {r.correct ? "correct" : r.shouldTrigger ? "missed" : "false positive"}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function NotesSection({ notes }: { notes: string[] }) {
  return (
    <section className="surface p-4">
      <SectionTitle title="Analyst notes" description="What the aggregates hide." />
      <ul className="mt-3 flex flex-col gap-1.5">
        {notes.map((n, i) => (
          <li key={i} className="flex items-start gap-2 rounded-lg bg-sunken px-3 py-2 text-xs selectable">
            <Info className="mt-0.5 size-3.5 shrink-0 text-info" />
            <span>{n}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}

const ALL = "__all__";

function CaseResults({ runId, results, arms, live }: { runId: string; results: T.CaseResult[]; arms: string[]; live: boolean }) {
  const [arm, setArm] = React.useState(ALL);
  const [caseName, setCaseName] = React.useState(ALL);
  const [expanded, setExpanded] = React.useState<Set<string>>(() => new Set());
  const [transcriptFor, setTranscriptFor] = React.useState<T.CaseResult | null>(null);
  const colors = armColors(arms);
  const caseNames = React.useMemo(() => [...new Set(results.map((r) => r.caseName))], [results]);
  const armOptions = React.useMemo(() => [...new Set([...arms, ...results.map((r) => r.arm)])], [arms, results]);
  const sorted = React.useMemo(() => {
    const order = new Map(armOptions.map((a, i) => [a, i]));
    return [...results]
      .filter((r) => (arm === ALL || r.arm === arm) && (caseName === ALL || r.caseName === caseName))
      .sort((a, b) => a.caseId - b.caseId || (order.get(a.arm) ?? 0) - (order.get(b.arm) ?? 0) || a.repeat - b.repeat);
  }, [results, arm, caseName, armOptions]);
  const toggle = (id: string) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const cols = "grid grid-cols-[auto_minmax(0,1.4fr)_minmax(0,1fr)_auto_auto_minmax(0,0.9fr)_auto_auto_auto] items-center gap-3 px-3";

  return (
    <section className="flex flex-col gap-3">
      <SectionTitle
        title={`Case results (${results.length})`}
        description={live ? "Streaming in as sandboxes finish." : "Every (case, arm, repeat) execution. Expand a row for the graded expectations, output and transcript."}
        action={
          results.length ? (
            <div className="flex items-center gap-2">
              <SimpleSelect size="sm" className="w-40" value={arm} onChange={setArm} options={[{ value: ALL, label: "All arms" }, ...armOptions.map((a) => ({ value: a, label: armLabel(a) }))]} />
              <SimpleSelect size="sm" className="w-48" value={caseName} onChange={setCaseName} options={[{ value: ALL, label: "All cases" }, ...caseNames.map((c) => ({ value: c, label: c }))]} />
            </div>
          ) : null
        }
      />
      {results.length === 0 ? (
        <div className="flex items-center justify-center gap-2 rounded-lg border border-dashed px-3 py-8 text-xs text-muted">
          {live ? (
            <>
              <Loader2 className="size-3.5 animate-spin" /> Waiting for the first case to finish…
            </>
          ) : (
            "No case results were recorded."
          )}
        </div>
      ) : (
        <div className="surface overflow-hidden">
          <div className={cn(cols, "border-b bg-sunken/60 py-1.5 text-[11px] font-medium uppercase tracking-wide text-subtle")}>
            <span className="w-5" />
            <span>Case</span>
            <span>Arm</span>
            <span className="text-right">Repeat</span>
            <span>Status</span>
            <span>Passed</span>
            <span className="text-right">Tokens</span>
            <span className="text-right">Duration</span>
            <span className="text-right">Tools</span>
          </div>
          {sorted.length === 0 ? <div className="px-3 py-6 text-center text-xs text-muted">Nothing matches these filters.</div> : null}
          {sorted.map((r) => {
            const open = expanded.has(r.id);
            const st = CASE_STATUS_META[r.status];
            return (
              <div key={r.id} className="border-b last:border-b-0">
                <button type="button" onClick={() => toggle(r.id)} aria-expanded={open} className={cn(cols, "w-full py-1.5 text-left text-xs hover:bg-sunken/60")}>
                  <span className="flex w-5 justify-center text-muted">{open ? <ChevronDown className="size-4" /> : <ChevronRight className="size-4" />}</span>
                  <span className="mono truncate font-medium">{r.caseName}</span>
                  <span className="inline-flex min-w-0 items-center gap-1.5 truncate">
                    <ArmSwatch color={colors[r.arm] ?? "var(--chart-5)"} /> {armLabel(r.arm)}
                  </span>
                  <span className="num text-right text-muted">{r.repeat}</span>
                  <span>
                    <Badge tone={st.tone}>{st.label}</Badge>
                  </span>
                  <span className="flex items-center gap-2">
                    <Progress value={r.total ? r.passed / r.total : 0} size="sm" tone={r.total && r.passed === r.total ? "ok" : r.passed === 0 ? "danger" : "warn"} className="w-14 shrink-0" />
                    <span className="num text-muted">
                      {r.passed}/{r.total}
                    </span>
                  </span>
                  <span className="num text-right text-muted">{formatTokens(r.inputTokens + r.outputTokens)}</span>
                  <span className="num text-right text-muted">{formatDuration(r.durationMs)}</span>
                  <span className="num text-right text-muted">{r.toolCalls}</span>
                </button>
                {open ? <CaseDetail result={r} onTranscript={() => setTranscriptFor(r)} /> : null}
              </div>
            );
          })}
        </div>
      )}
      <TranscriptDialog runId={runId} result={transcriptFor} onClose={() => setTranscriptFor(null)} />
    </section>
  );
}

function CaseDetail({ result: r, onTranscript }: { result: T.CaseResult; onTranscript: () => void }) {
  return (
    <div className="grid gap-4 border-t bg-sunken/40 px-4 py-3 text-xs lg:grid-cols-[minmax(0,1.3fr)_minmax(0,1fr)]">
      <div className="flex flex-col gap-1.5">
        <div className="text-[11px] font-medium uppercase tracking-wide text-subtle">Expectations</div>
        {r.expectations.length === 0 ? <p className="text-muted">No expectations were graded.</p> : null}
        {r.expectations.map((e, i) => (
          <div key={i} className="flex items-start gap-2 rounded-lg bg-elevated px-2.5 py-1.5 hairline">
            {e.passed ? <CheckCircle2 className="mt-0.5 size-3.5 shrink-0 text-ok" /> : <XCircle className="mt-0.5 size-3.5 shrink-0 text-danger" />}
            <div className="min-w-0 flex-1">
              <div className="flex items-start justify-between gap-2">
                <span className="font-medium selectable">{e.text}</span>
                <Badge tone="outline">{e.source}</Badge>
              </div>
              <p className="mt-0.5 text-muted selectable">{e.evidence}</p>
            </div>
          </div>
        ))}
      </div>
      <div className="flex min-w-0 flex-col gap-3">
        <div>
          <div className="mb-1 text-[11px] font-medium uppercase tracking-wide text-subtle">Final answer (excerpt)</div>
          <pre className="mono selectable max-h-32 overflow-auto whitespace-pre-wrap rounded-lg bg-elevated p-2.5 text-[11.5px] leading-5 hairline">{r.outputExcerpt || "(empty)"}</pre>
        </div>
        {r.graderNotes ? (
          <div>
            <div className="mb-1 text-[11px] font-medium uppercase tracking-wide text-subtle">Grader notes</div>
            <p className="selectable text-muted">{r.graderNotes}</p>
          </div>
        ) : null}
        {r.error ? <ErrorNote error={r.error} /> : null}
        <div className="flex flex-wrap items-center gap-1.5">
          <Button size="sm" variant="secondary" onClick={onTranscript} disabled={!r.transcriptPath && r.status === "queued"}>
            <FileText /> Transcript
          </Button>
          <Button size="sm" variant="ghost" onClick={() => void revealPath(r.sandboxPath)}>
            <FolderOpen /> Reveal sandbox
          </Button>
          <Tooltip content={r.sandboxPath}>
            <Button size="sm" variant="ghost" onClick={() => void copyText(r.sandboxPath).then(() => toast("Sandbox path copied"))}>
              <Copy /> Copy path
            </Button>
          </Tooltip>
          {r.costUsd ? <span className="ml-auto num text-subtle">${r.costUsd.toFixed(2)}</span> : null}
        </div>
      </div>
    </div>
  );
}

function TranscriptDialog({ runId, result, onClose }: { runId: string; result: T.CaseResult | null; onClose: () => void }) {
  const q = useQuery({ queryKey: ["transcript", runId, result?.id ?? ""], queryFn: () => api.caseTranscript(runId, result!.id), enabled: !!result, staleTime: Infinity });
  const lines = q.data ? q.data.split("\n").filter(Boolean).length : 0;
  return (
    <Dialog open={!!result} onOpenChange={(o) => !o && onClose()}>
      <DialogContent size="xl">
        <DialogHeader>
          <DialogTitle>
            Transcript · {result?.caseName} · {result ? armLabel(result.arm) : ""} · repeat {result?.repeat}
          </DialogTitle>
          <DialogDescription>{result?.transcriptPath ? <span className="mono">{result.transcriptPath}</span> : "Raw agent transcript (JSONL)."}</DialogDescription>
        </DialogHeader>
        <DialogBody>
          {q.isLoading ? (
            <div className="flex flex-col gap-2">
              <Skeleton className="h-4 w-full" />
              <Skeleton className="h-4 w-5/6" />
              <Skeleton className="h-4 w-2/3" />
            </div>
          ) : q.error ? (
            <ErrorNote error={q.error} />
          ) : (
            <pre className="mono selectable max-h-[60vh] overflow-auto whitespace-pre-wrap break-all rounded-lg bg-sunken p-3 text-[11.5px] leading-5">{q.data}</pre>
          )}
        </DialogBody>
        <DialogFooter>
          <span className="mr-auto text-xs text-subtle">{lines ? pluralize(lines, "line") : null}</span>
          <Button variant="ghost" onClick={onClose}>
            Close
          </Button>
          <Button disabled={!q.data} onClick={() => q.data && void copyText(q.data).then(() => toast.success("Transcript copied"))}>
            <Copy /> Copy
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
