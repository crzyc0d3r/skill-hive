import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { useEffect, useState } from "react";
import { t } from "../../lib/i18n";
import { evidenceAnchor, FIXTURE_CELLS, FIXTURE_STEPS, moveCell, stepDuration, type HeatCell, type TimelineStep } from "@/features/results/model";
import { demoEnabled } from "@/lib/demo";
import { silent } from "@/lib/ipc/actions";

function scrollToId(id: string) {
  const node = document.getElementById(id);
  if (node && typeof node.scrollIntoView === "function") node.scrollIntoView({ block: "nearest" });
}

export function ResultsView() {
  const demo = demoEnabled();
  const [cells, setCells] = useState<HeatCell[]>(() => (demo ? FIXTURE_CELLS : []));
  const [steps, setSteps] = useState<TimelineStep[]>(() => (demo ? FIXTURE_STEPS : []));
  const [index, setIndex] = useState(0);
  const [open, setOpen] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [anchor, setAnchor] = useState<string | null>(null);
  const [budgetOpen, setBudgetOpen] = useState(false);
  const [paused, setPaused] = useState(false);
  useEffect(() => {
    if (demo) return;
    void silent<{ text?: string }>("get_recent_log_excerpt").then((excerpt) => {
      if (!excerpt?.text) return;
      try {
        const parsed = JSON.parse(excerpt.text) as { cells?: HeatCell[]; steps?: TimelineStep[] };
        if (Array.isArray(parsed.cells) && parsed.cells.length) setCells(parsed.cells);
        if (Array.isArray(parsed.steps)) setSteps(parsed.steps);
      } catch {
        /* logs that are not a run payload stay on the empty state */
      }
    });
  }, [demo]);
  const selected = cells[index];
  return (
    <section data-testid="results" className="min-w-0 space-y-3">
      <h1 className="font-mono text-xl font-semibold">{t("results.title")}</h1>
      <p className="text-sm">{paused ? t("results.pause") : cells.length === 0 ? t("results.empty") : t("results.heatmap")}</p>
      <div
        role="grid"
        tabIndex={0}
        data-testid="heatmap"
        className="grid grid-cols-2 gap-2"
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
          <Button key={cell.id} type="button" role="gridcell" data-selected={cellIndex === index} variant="outline" size="sm" className="text-left" onClick={() => setIndex(cellIndex)}>
            {cell.label} {cell.passRate}
          </Button>
        ))}
      </div>
      {open ? <p data-testid="heatmap-open">{open}</p> : null}
      <h2 className="text-sm font-semibold">{t("results.timeline")}</h2>
      <ul className="flex gap-2">
        {steps.map((step) => (
          <li key={step.id}>
            <Button type="button" variant="outline" size="sm" onClick={() => scrollToId(`step-${step.id}`)}>
              {step.tool} {stepDuration(step)}ms
            </Button>
          </li>
        ))}
      </ul>
      <div data-testid="transcript" className="max-h-24 overflow-auto rounded border border-border p-2 text-xs">
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
        className="flex flex-wrap gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          const next = evidenceAnchor(query);
          setAnchor(next);
          if (next) scrollToId(next);
        }}
      >
        <label className="text-sm">
          {t("results.evidence")}
          <Input className="h-8 ml-2" value={query} onChange={(event) => setQuery(event.target.value)} />
        </label>
        <Button type="submit" variant="outline" size="sm">
          {t("results.evidence")}
        </Button>
      </form>
      <p data-testid="evidence-anchor">{anchor ?? t("results.noAnchor")}</p>
      <Button type="button" variant="outline" size="sm" onClick={() => setBudgetOpen(true)}>
        {t("results.budget")}
      </Button>
      {budgetOpen ? (
        <div role="dialog" data-testid="budget-dialog" className="rounded border border-border p-3">
          <p className="text-sm">{t("results.budget")}</p>
          <Button type="button" variant="outline" size="sm" className="mr-2" onClick={() => { setPaused(true); setBudgetOpen(false); }}>
            {t("results.confirm")}
          </Button>
          <Button type="button" variant="outline" size="sm" onClick={() => setBudgetOpen(false)}>
            {t("results.cancel")}
          </Button>
        </div>
      ) : null}
    </section>
  );
}
