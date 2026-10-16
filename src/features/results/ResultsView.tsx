import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { useState } from "react";
import { t } from "../../lib/i18n";
import { evidenceAnchor, FIXTURE_CELLS, FIXTURE_STEPS, moveCell, stepDuration } from "@/features/results/model";

function scrollToId(id: string) {
  const node = document.getElementById(id);
  if (node && typeof node.scrollIntoView === "function") node.scrollIntoView({ block: "nearest" });
}

export function ResultsView() {
  const [index, setIndex] = useState(0);
  const [open, setOpen] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [anchor, setAnchor] = useState<string | null>(null);
  const [budgetOpen, setBudgetOpen] = useState(false);
  const [paused, setPaused] = useState(false);
  const selected = FIXTURE_CELLS[index];
  return (
    <section data-testid="results" className="min-w-0 space-y-3">
      <h1 className="font-mono text-xl font-semibold">{t("results.title")}</h1>
      <p className="text-sm">{paused ? t("results.pause") : t("results.heatmap")}</p>
      <div
        role="grid"
        tabIndex={0}
        data-testid="heatmap"
        className="grid grid-cols-2 gap-2"
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            setOpen(selected.id);
            return;
          }
          const next = moveCell(index, event.key, FIXTURE_CELLS.length);
          if (next !== index) {
            event.preventDefault();
            setIndex(next);
          }
        }}
      >
        {FIXTURE_CELLS.map((cell, cellIndex) => (
          <Button key={cell.id} type="button" role="gridcell" data-selected={cellIndex === index} variant="outline" size="sm" className="text-left" onClick={() => setIndex(cellIndex)}>
            {cell.label} {cell.passRate}
          </Button>
        ))}
      </div>
      {open ? <p data-testid="heatmap-open">{open}</p> : null}
      <h2 className="text-sm font-semibold">{t("results.timeline")}</h2>
      <ul className="flex gap-2">
        {FIXTURE_STEPS.map((step) => (
          <li key={step.id}>
            <Button type="button" variant="outline" size="sm" onClick={() => scrollToId(`step-${step.id}`)}>
              {step.tool} {stepDuration(step)}ms
            </Button>
          </li>
        ))}
      </ul>
      <div data-testid="transcript" className="max-h-24 overflow-auto rounded border border-border p-2 text-xs">
        <p id="step-2">step 2 wrote outputs/answer.txt</p>
        <p id="step-3">step 3 read outputs/answer.txt</p>
        <p id="outputs/answer.txt">PINECONE</p>
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
