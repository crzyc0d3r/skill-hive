import { Button, buttonVariants } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { useState } from "react";
import { Link, useParams } from "react-router-dom";
import { t } from "../../lib/i18n";
import { applyOptimized, optimizeDescription, useGenerate } from "@/features/generate/store";
import { runEffectivenessSuite, saveSuiteDraft } from "@/lib/ipc/actions";

const MODES = ["emulated", "claude-code", "sequential", "static"] as const;

export function SuiteView() {
  const { id = "skill" } = useParams();
  const { wizard } = useGenerate();
  const [mode, setMode] = useState<(typeof MODES)[number]>("emulated");
  const [preset, setPreset] = useState("Full benchmark");
  const repetitions = preset === "Quick smoke" ? 1 : 3;
  const [prompt, setPrompt] = useState("Complete the task");
  const [query, setQuery] = useState("use the skill");
  const [check, setCheck] = useState("answer file exists");
  const [progress, setProgress] = useState("");
  const [saved, setSaved] = useState("");
  return (
    <section data-testid="suite" className="flex w-full min-w-0 flex-col gap-4">
      <h1 className="font-mono text-[22px] font-semibold tracking-tight">{t("suite.title")}</h1>
      <Card className="max-w-xl gap-3 rounded-lg border-border px-4 py-4 shadow-none">
        <p className="text-sm">{t("suite.withSkill")}</p>
        <p className="text-sm text-muted-foreground">{t("suite.withoutSkill")}</p>
        <label className="text-sm">
          {t("suite.mode")}
          <select aria-label={t("suite.mode")} className="ml-2 rounded-md border border-input px-1 py-0.5" value={mode} onChange={(event) => setMode(event.target.value as (typeof MODES)[number])}>
            {MODES.map((item) => (
              <option key={item} value={item}>{item}</option>
            ))}
          </select>
        </label>
        <label className="block text-sm">
          {t("suite.cases")}
          <Input aria-label={t("suite.cases")} className="mt-1 h-8" value={prompt} onChange={(event) => setPrompt(event.target.value)} />
        </label>
        <label className="block text-sm">
          {t("suite.triggers")}
          <Input aria-label={t("suite.triggers")} className="mt-1 h-8" value={query} onChange={(event) => setQuery(event.target.value)} />
        </label>
        <label className="block text-sm">
          {t("suite.checks")}
          <Input aria-label={t("suite.checks")} className="mt-1 h-8" value={check} onChange={(event) => setCheck(event.target.value)} />
        </label>
        <label className="text-sm">
          {t("suite.preset")}
          <select aria-label={t("suite.preset")} className="ml-2 rounded-md border border-input px-1 py-0.5" value={preset} onChange={(event) => setPreset(event.target.value)}>
            <option>Quick smoke</option>
            <option>Full benchmark</option>
          </select>
        </label>
        <p className="text-sm" data-testid="suite-matrix">
          {t("suite.matrix")}: with_skill, without_skill · {t("suite.repetitions")} {repetitions} · {mode}
        </p>
        <p className="text-sm">{t("suite.estimate")}: 2 × 1 × {repetitions}</p>
        <div className="flex flex-wrap items-center gap-2">
          <Button type="button" variant="outline" size="sm" onClick={() => optimizeDescription(id)}>{t("suite.optimize")}</Button>
          <Button type="button" size="sm" disabled={!wizard.optimized || wizard.applied} onClick={() => applyOptimized()}>{t("suite.apply")}</Button>
          <Button
            type="button"
            variant="secondary"
            size="sm"
            onClick={() => {
              setProgress("preparing");
              void saveSuiteDraft(id, prompt, query, check).then((value) => {
                if (typeof value === "string") setSaved(value);
              });
              void runEffectivenessSuite(id, mode, repetitions).then((value) => {
                if (typeof value !== "string") {
                  setProgress("done");
                  return;
                }
                try {
                  const parsed = JSON.parse(value) as { progress?: string[]; matrix?: { models?: string[] } };
                  const steps = parsed.progress?.join(" → ") || "done";
                  const models = parsed.matrix?.models?.join(", ") ?? "";
                  setProgress(models ? `${steps} · ${models}` : steps);
                } catch {
                  setProgress("done");
                }
              });
            }}
          >
            {t("suite.run")}
          </Button>
          <Link to="/effectiveness" data-testid="effectiveness-link" className={buttonVariants({ variant: "outline", size: "sm" })}>
            {t("suite.effectiveness")}
          </Link>
        </div>
        {progress ? <p data-testid="suite-progress">{t("suite.progress")}: {progress}</p> : null}
        {saved ? <p className="text-xs text-muted-foreground">{saved}</p> : null}
        {wizard.optimized ? <p data-testid="optimize-preview">{wizard.optimizePath || t("suite.optimized")}</p> : null}
        {wizard.applied ? <p data-testid="optimize-applied">{t("suite.apply")}</p> : null}
      </Card>
    </section>
  );
}
