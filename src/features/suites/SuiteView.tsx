import { Button, buttonVariants } from "@/components/ui/button";
import { useState } from "react";
import { Link, useParams } from "react-router-dom";
import { cn } from "cn";
import { t } from "../../lib/i18n";
import { applyOptimized, optimizeDescription, useGenerate } from "@/features/generate/store";
import { runEffectivenessSuite, saveSuiteDraft } from "@/lib/ipc/actions";
import { Panel, Screen, StatePanel, TextField } from "@/components/chrome";
import { SuiteSkeleton } from "@/components/skeletons";
import { previewPhase, useOnline, useShellPhase, type ViewPhase } from "@/lib/view-state";

const MODES = ["emulated", "claude-code", "sequential", "static"] as const;
const CONDITIONS = ["with_skill", "without_skill"] as const;

function phaseCopy(phase: ViewPhase): string {
  if (phase === "loading") return t("effect.loading");
  if (phase === "error") return t("effect.error");
  if (phase === "offline") return t("effect.offline");
  return t("effect.empty");
}

export function SuiteView() {
  const forced = previewPhase();
  const shell = useShellPhase();
  const online = useOnline();
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
  const [failed, setFailed] = useState(false);
  const cells = CONDITIONS.flatMap((condition) => Array.from({ length: repetitions }, (_, index) => ({ condition, index: index + 1 })));
  const running = progress === "preparing";
  const finished = progress.length > 0 && !running;

  if (forced && forced !== "loading") {
    return (
      <Screen testId="suite" className="min-h-[calc(100dvh-8.5rem)]" title={t("suite.title")}>
        <StatePanel tone={forced} title={phaseCopy(forced)} />
      </Screen>
    );
  }

  return (
    <Screen testId="suite" className="min-h-[calc(100dvh-8.5rem)]" title={t("suite.title")} lede={t("suite.matrix")}>
      {!online ? <StatePanel tone="offline" title={t("effect.offline")} /> : null}
      <div className="grid flex-1 gap-3 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
        <Panel title={t("suite.cases")} className="h-full">
          <div className="flex flex-col gap-3">
            <label className="text-sm">
              <span className="text-xs font-medium text-muted-foreground">{t("suite.mode")}</span>
              <select aria-label={t("suite.mode")} className="mt-1 h-8 w-full rounded-md border border-input bg-background px-2 text-sm" value={mode} onChange={(event) => setMode(event.target.value as (typeof MODES)[number])}>
                {MODES.map((item) => (
                  <option key={item} value={item}>{item}</option>
                ))}
              </select>
            </label>
            <label className="text-sm">
              <span className="text-xs font-medium text-muted-foreground">{t("suite.cases")}</span>
              <TextField aria-label={t("suite.cases")} className="mt-1" value={prompt} onChange={(event) => setPrompt(event.target.value)} />
            </label>
            <label className="text-sm">
              <span className="text-xs font-medium text-muted-foreground">{t("suite.triggers")}</span>
              <TextField aria-label={t("suite.triggers")} className="mt-1" value={query} onChange={(event) => setQuery(event.target.value)} />
            </label>
            <label className="text-sm">
              <span className="text-xs font-medium text-muted-foreground">{t("suite.checks")}</span>
              <TextField aria-label={t("suite.checks")} className="mt-1" value={check} onChange={(event) => setCheck(event.target.value)} />
            </label>
            <label className="text-sm">
              <span className="text-xs font-medium text-muted-foreground">{t("suite.preset")}</span>
              <select aria-label={t("suite.preset")} className="mt-1 h-8 w-full rounded-md border border-input bg-background px-2 text-sm" value={preset} onChange={(event) => setPreset(event.target.value)}>
                <option>Quick smoke</option>
                <option>Full benchmark</option>
              </select>
            </label>
            <p className="text-sm text-muted-foreground">{t("suite.withSkill")} · {t("suite.withoutSkill")}</p>
          </div>
        </Panel>
        <Panel title={t("suite.matrix")} className="h-full">
          {shell === "loading" ? <SuiteSkeleton /> : (
          <div data-testid="suite-matrix" className="flex h-full flex-col gap-3">
            <p className="text-sm text-muted-foreground">
              {t("suite.matrix")}: with_skill, without_skill · {t("suite.repetitions")} {repetitions} · {mode}
            </p>
            <div className="grid flex-1 gap-3 sm:grid-cols-2">
              {CONDITIONS.map((condition) => (
                <div key={condition} className="rounded-md border border-border bg-muted/20 p-3">
                  <div className="mb-2 font-mono text-xs text-muted-foreground">{condition}</div>
                  <div className="grid grid-cols-3 gap-2">
                    {Array.from({ length: repetitions }, (_, index) => (
                      <div key={`${condition}-${index}`} className="flex h-16 flex-col items-center justify-center rounded-md border border-border bg-card">
                        <span className="font-mono text-lg tabular-nums">{index + 1}</span>
                        <span className="text-[11px] text-muted-foreground">{t("suite.queued")}</span>
                      </div>
                    ))}
                  </div>
                </div>
              ))}
            </div>
            <p className="font-mono text-sm tabular-nums">{t("suite.estimate")}: 2 × 1 × {repetitions}</p>
          </div>
          )}
        </Panel>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <Button type="button" variant="secondary" size="sm" onClick={() => optimizeDescription(id)}>{t("suite.optimize")}</Button>
        <Button type="button" variant="secondary" size="sm" disabled={!wizard.optimized || wizard.applied} onClick={() => applyOptimized()}>{t("suite.apply")}</Button>
        <Button
          type="button"
          size="sm"
          aria-busy={progress === "preparing"}
          disabled={progress === "preparing"}
          onClick={() => {
            setFailed(false);
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
            }).catch(() => {
              setFailed(true);
              setProgress("");
            });
          }}
        >
          {progress === "preparing" ? t("shell.loading") : t("suite.run")}
        </Button>
        <Link to="/effectiveness" data-testid="effectiveness-link" className={buttonVariants({ variant: "secondary", size: "sm" })}>
          {t("suite.effectiveness")}
        </Link>
      </div>
      {failed ? <p role="alert" className="text-sm text-destructive">{t("effect.error")}</p> : null}
      {progress ? (
        <Panel title={t("suite.progress")}>
          <div data-testid="suite-progress" className="flex flex-col gap-3">
            <p className="text-sm">{t("suite.progress")}: {progress}</p>
            <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6">
              {cells.map((cell) => (
                <div
                  key={`${cell.condition}-${cell.index}`}
                  className={cn(
                    "flex h-20 flex-col items-center justify-center rounded-md border px-2 text-center",
                    running && "animate-pulse border-info/40 bg-info-subtle text-info-text",
                    finished && "border-success/40 bg-success-subtle text-success-text",
                  )}
                >
                  <span className="font-mono text-xs">{cell.condition}</span>
                  <span className="font-mono text-xl tabular-nums">{cell.index}</span>
                  <span className="text-[11px]">{running ? t("suite.preparing") : t("suite.done")}</span>
                </div>
              ))}
            </div>
          </div>
        </Panel>
      ) : null}
      {saved ? <p className="text-xs text-muted-foreground">{saved}</p> : null}
      {wizard.optimized ? <p data-testid="optimize-preview">{wizard.optimizePath || t("suite.optimized")}</p> : null}
      {wizard.applied ? <p data-testid="optimize-applied">{t("suite.apply")}</p> : null}
    </Screen>
  );
}
