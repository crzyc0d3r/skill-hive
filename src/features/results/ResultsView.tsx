import { invoke } from "@tauri-apps/api/core";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { useEffect, useRef, useState } from "react";
import { cn } from "cn";
import { t } from "../../lib/i18n";
import { evidenceAnchor, FIXTURE_CELLS, FIXTURE_STEPS, moveCell, stepDuration, type HeatCell, type TimelineStep } from "@/features/results/model";
import { demoEnabled } from "@/lib/demo";
import { controlMeasuredRun, exportMeasuredRun } from "@/lib/ipc/actions";
import { VerdictChip } from "@/components/app/VerdictChip";
import { EmptyState, ErrorState, Panel, Screen, StatePanel } from "@/components/chrome";
import { Link } from "react-router-dom";
import { ResultsSkeleton } from "@/components/skeletons";
import { previewPhase, useOnline, type ViewPhase } from "@/lib/view-state";

function scrollToId(id: string) {
  const node = document.getElementById(id);
  if (node && typeof node.scrollIntoView === "function") node.scrollIntoView({ block: "nearest" });
}

function rateTone(rate: number): string {
  if (rate >= 0.8) return "border-success/40 bg-success-subtle text-success-subtle-foreground";
  if (rate >= 0.5) return "border-warning/40 bg-warning-subtle text-warning-subtle-foreground";
  return "border-destructive/40 bg-destructive-subtle text-destructive-subtle-foreground";
}

function taskName(id: string): string | null {
  if (id.endsWith("-1")) return t("results.task.summarize");
  if (id.endsWith("-2")) return t("results.task.methods");
  return null;
}

function taskLabel(cell: { id: string; label: string }): string {
  const name = taskName(cell.id);
  if (name) {
    const side = cell.id.startsWith("without") ? t("results.withoutSkill") : t("results.withSkill");
    return `${name} · ${side}`;
  }
  return cell.label;
}

function phaseCopy(phase: ViewPhase): string {
  if (phase === "loading") return t("results.loading");
  if (phase === "error") return t("results.error");
  if (phase === "offline") return t("results.offline");
  return t("results.empty");
}

export function ResultsView() {
  const forced = previewPhase();
  const online = useOnline();
  const demo = demoEnabled() && !forced;
  const [cells, setCells] = useState<HeatCell[]>(() => (demo ? FIXTURE_CELLS : []));
  const cellsRef = useRef(cells);
  cellsRef.current = cells;
  const [steps, setSteps] = useState<TimelineStep[]>(() => (demo ? FIXTURE_STEPS : []));
  const [phase, setPhase] = useState<ViewPhase>(() => forced ?? (demo ? "ready" : "loading"));
  const [index, setIndex] = useState(0);
  const [open, setOpen] = useState<string | null>(() => (demoEnabled() && !previewPhase() ? "with-1" : null));
  const [query, setQuery] = useState("");
  const [anchor, setAnchor] = useState<string | null>(null);
  const [searched, setSearched] = useState(false);
  const [budgetOpen, setBudgetOpen] = useState(false);
  const [paused, setPaused] = useState(false);
  const [exported, setExported] = useState("");
  const [notes, setNotes] = useState<string[]>([]);
  const [triggers, setTriggers] = useState<{ query: string; outcome: string }[]>([]);
  const [attempt, setAttempt] = useState(0);
  const [sheetTab, setSheetTab] = useState("transcript");

  useEffect(() => {
    if (forced) {
      setPhase(forced);
      if (forced === "loading" || forced === "empty" || forced === "error" || forced === "offline") {
        setCells([]);
        setSteps([]);
      }
      return;
    }
    if (demoEnabled()) {
      setCells(FIXTURE_CELLS);
      setSteps(FIXTURE_STEPS);
      setPhase("ready");
      return;
    }
    if (!online) {
      setPhase("offline");
      return;
    }
    let cancel = false;
    if (cellsRef.current.length === 0) setPhase("loading");
    void (async () => {
      try {
        const raw = await invoke<string>("get_effectiveness_report");
        let loaded = false;
        if (raw) {
          const parsed = JSON.parse(raw) as {
            cells?: { id: string; configuration: string; pass_rate: number }[];
            analyst_notes?: string[];
            activations?: { query: string; outcome: string }[];
          };
          if (Array.isArray(parsed.cells) && parsed.cells.length) {
            loaded = true;
            setCells(parsed.cells.map((cell) => ({ id: cell.id, label: cell.configuration, passRate: cell.pass_rate })));
          }
          if (Array.isArray(parsed.analyst_notes)) setNotes(parsed.analyst_notes.filter((note) => note.length > 0));
          if (Array.isArray(parsed.activations)) setTriggers(parsed.activations.map((row) => ({ query: row.query, outcome: row.outcome })));
        }
        if (!loaded) {
          const excerpt = await invoke<{ text?: string }>("get_recent_log_excerpt");
          if (excerpt?.text) {
            const parsed = JSON.parse(excerpt.text) as { cells?: HeatCell[]; steps?: TimelineStep[] };
            if (Array.isArray(parsed.cells) && parsed.cells.length) {
              loaded = true;
              setCells(parsed.cells);
            }
            if (Array.isArray(parsed.steps)) setSteps(parsed.steps);
          }
        }
        if (!cancel) setPhase(loaded ? "ready" : "empty");
      } catch {
        if (!cancel) setPhase("error");
      }
    })();
    return () => {
      cancel = true;
    };
  }, [forced, online, attempt]);

  const selected = cells[index];
  const withRates = cells.filter((cell) => cell.id.startsWith("with-"));
  const withoutRates = cells.filter((cell) => cell.id.startsWith("without-"));
  const average = (rows: HeatCell[]) => (rows.length ? rows.reduce((sum, cell) => sum + cell.passRate, 0) / rows.length : 0);
  const verdictDelta = cells.length ? Math.round((average(withRates) - average(withoutRates)) * 100) : 0;
  const lede = paused ? t("results.pause") : phase === "ready" ? t("results.lede") : undefined;
  const hasResults = phase === "ready" && cells.length > 0;
  if (phase === "loading" && cells.length === 0) {
    return (
      <Screen testId="results" className="min-h-[calc(100dvh-8.5rem)]" title={t("results.title")}>
        <ResultsSkeleton />
      </Screen>
    );
  }
  return (
    <Screen testId="results" className="min-h-[calc(100dvh-8.5rem)]" title={t("results.title")} lede={lede} actions={
      <>
        <Button type="button" size="sm" variant={hasResults ? "default" : "outline"} onClick={() => { void exportMeasuredRun().then((value) => { if (typeof value === "string") setExported(value); }); }}>{t("results.export")}</Button>
        <Button type="button" variant="outline" size="sm" className="sr-only" aria-busy={phase === "loading" && cells.length > 0} disabled={phase === "loading" && cells.length > 0} onClick={() => setBudgetOpen(true)}>{t("results.budget")}</Button>
      </>
    }>
      {phase === "error" && cells.length === 0 ? (
        <ErrorState title={t("results.error")} detail={t("shell.renderError")} onRetry={() => setAttempt((value) => value + 1)} />
      ) : null}
      {phase === "offline" && cells.length === 0 ? (
        <StatePanel
          tone="offline"
          title={phaseCopy("offline")}
          action={<Button type="button" size="sm" onClick={() => setAttempt((value) => value + 1)}>{t("banner.retry")}</Button>}
        />
      ) : null}
      {phase === "empty" ? <EmptyState title={t("results.empty")} body={t("results.emptyBody")} action={<Button asChild size="sm"><Link to="/skills/skill-research/tests">{t("results.emptyCta")}</Link></Button>} /> : null}
      {hasResults ? (
        <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-border bg-card p-4" data-testid="verdict-hero">
          <div>
            <VerdictChip delta={verdictDelta} large />
            <p className="mt-2 text-sm text-muted-foreground">{t("verdict.numbers", { with: String(Math.round((cells.find((cell) => cell.id.startsWith("with"))?.passRate ?? 0) * 100)), without: String(Math.round((cells.find((cell) => cell.id.startsWith("without"))?.passRate ?? 0) * 100)) })}</p>
          </div>
        </div>
      ) : null}
      {phase === "ready" || cells.length > 0 ? <div className={cn("grid min-w-0 items-start gap-3", open && "min-[1100px]:grid-cols-[minmax(0,1fr)_18rem] min-[1440px]:grid-cols-[minmax(0,1fr)_400px] min-[1920px]:grid-cols-[minmax(0,1fr)_480px]")}>
        <Panel className="min-w-0 w-full">
          <div
            role="grid"
            tabIndex={0}
            data-testid="heatmap"
            className="grid grid-cols-2 gap-3 outline-none focus-visible:ring-2 focus-visible:ring-ring"
            onKeyDown={(event) => {
              if (event.key === "Enter" && selected) {
                setOpen(selected.id);
                return;
              }
              const next = moveCell(index, event.key, cells.length);
              if (next !== index) {
                event.preventDefault();
                setIndex(next);
              }
            }}
          >
            {cells.map((cell, cellIndex) => (
              <button
                key={cell.id}
                type="button"
                role="gridcell"
                data-selected={cellIndex === index}
                className={cn("flex h-28 flex-col items-start justify-between rounded-lg border p-3 text-left", rateTone(cell.passRate), cellIndex === index && "ring-2 ring-ring")}
                onClick={() => { setIndex(cellIndex); setOpen(cell.id); }}
              >
                <span className="text-sm font-medium">{taskLabel(cell)}</span>
                <span className="text-3xl font-semibold tabular-nums">{Math.round(cell.passRate * 100)}%</span>
              </button>
            ))}
          </div>
        </Panel>
        {open ? (
          <aside data-testid="cell-sheet" className="w-full rounded-lg border border-border bg-card p-4">
            <h2 className="font-sans text-title font-semibold">{t("results.sheet")}</h2>
            <p data-testid="heatmap-open" className="mt-2 text-sm font-medium">{open.startsWith("without") ? t("results.withoutSkill") : open.startsWith("with") ? t("results.withSkill") : open}</p>
            {selected ? <p className="mt-1 text-sm">{taskLabel(selected)} · {Math.round(selected.passRate * 100)}%</p> : null}
            <div className="mt-3 flex flex-nowrap gap-1 overflow-x-auto" role="tablist" aria-label={t("results.sheet")}>
              {(["transcript", "timeline", "outputs", "grading", "checks", "feedback"] as const).map((key) => (
                <button key={key} type="button" role="tab" aria-selected={sheetTab === key} className={sheetTab === key ? "shrink-0 whitespace-nowrap rounded-md bg-muted px-2 py-1 text-xs" : "shrink-0 whitespace-nowrap rounded-md px-2 py-1 text-xs text-muted-foreground"} onClick={() => setSheetTab(key)}>{t(`results.${key}`)}</button>
              ))}
            </div>
            {sheetTab === "transcript" ? (
              <>
                <div data-testid="transcript" className="mt-3 max-h-28 overflow-auto rounded-md border border-border bg-muted/30 p-3 font-mono text-xs">
                  {demo ? (
                    <>
                      <p id="step-2">step 2 wrote outputs/answer.txt</p>
                      <p id="step-3">step 3 read outputs/answer.txt</p>
                      <p id="outputs/answer.txt">PINECONE</p>
                    </>
                  ) : (
                    steps.map((step) => (
                      <p id={`step-${step.id}`} key={step.id}>{step.tool}</p>
                    ))
                  )}
                </div>
                <form
                  className="mt-3 flex flex-wrap items-end gap-2"
                  onSubmit={(event) => {
                    event.preventDefault();
                    const next = evidenceAnchor(query);
                    setSearched(true);
                    setAnchor(next);
                    if (next) scrollToId(next);
                  }}
                >
                  <label className="min-w-0 flex-1 text-sm">
                    <span className="mb-1 block text-xs font-medium text-muted-foreground">{t("results.evidence")}</span>
                    <Input aria-label={t("results.evidence")} className="h-8 w-full" value={query} onChange={(event) => setQuery(event.target.value)} />
                  </label>
                  <Button type="submit" size="sm">{t("results.evidence")}</Button>
                </form>
                {searched ? <p className="mt-2 font-mono text-sm" data-testid="evidence-anchor">{anchor ?? t("results.noAnchor")}</p> : <p className="sr-only" data-testid="evidence-anchor" />}
              </>
            ) : (
              <p className="mt-3 text-sm text-muted-foreground">{t(`results.${sheetTab}`)}</p>
            )}
          </aside>
        ) : null}
      </div> : null}
      {notes.length ? (
        <Panel title={t("results.notes")} testId="analyst-notes">
          <ul className="space-y-2 text-sm">{notes.map((note) => <li key={note}>{note}</li>)}</ul>
        </Panel>
      ) : <div data-testid="analyst-notes" className="hidden" />}
      {triggers.length ? (
        <Panel title={t("results.triggers")} testId="trigger-panel">
          <ul className="space-y-2 text-sm">{triggers.map((row) => <li key={`${row.query}-${row.outcome}`} className="flex justify-between gap-2"><span>{row.query}</span><span className="font-mono text-xs">{row.outcome}</span></li>)}</ul>
        </Panel>
      ) : <div data-testid="trigger-panel" className="hidden" />}
      {phase === "ready" || steps.length > 0 ? <Panel title={t("results.timeline")}>
        <div className="flex h-14 overflow-hidden rounded-md border border-border">
          {steps.map((step) => (
            <button key={step.id} type="button" className="flex min-w-24 flex-1 flex-col items-start justify-center border-r border-border bg-chart-1/15 px-3 text-left last:border-r-0" style={{ flexGrow: Math.max(stepDuration(step), 1) }} onClick={() => scrollToId(`step-${step.id}`)}>
              <span className="text-sm font-medium">{step.tool}</span>
              <span className="font-mono text-xs tabular-nums text-muted-foreground">{stepDuration(step)}ms</span>
            </button>
          ))}
        </div>
      </Panel> : null}
      {budgetOpen ? (
        <div role="dialog" data-testid="budget-dialog" className="rounded-lg border border-border bg-card p-4 shadow-sm">
          <p className="text-sm font-medium">{t("results.budget")}</p>
          <div className="mt-3 flex flex-wrap gap-2">
            <Button type="button" size="sm" onClick={() => { setPaused(true); setBudgetOpen(false); void controlMeasuredRun("continue"); }}>{t("results.confirm")}</Button>
            <Button type="button" variant="secondary" size="sm" onClick={() => { setBudgetOpen(false); void controlMeasuredRun("stop"); }}>{t("results.stop")}</Button>
            <Button type="button" variant="secondary" size="sm" onClick={() => { setBudgetOpen(false); void controlMeasuredRun("cancel"); }}>{t("results.cancel")}</Button>
          </div>
        </div>
      ) : null}
      {exported ? <p data-testid="results-export" className="font-mono text-xs">{exported}</p> : null}
    </Screen>
  );
}
