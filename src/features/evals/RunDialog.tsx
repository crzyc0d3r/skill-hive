// "Run evaluation…" dialog: arms, case subset, repeats, providers, timeout and trigger eval,
// starting from the backend's default config for the skill.

import * as React from "react";
import { useNavigate } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { toast } from "sonner";
import { AlertTriangle, FolderPlus, Play, Settings2, X } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation, useProviders } from "@/lib/api/hooks";
import { cn, displayPath, formatDuration, pluralize } from "@/lib/utils";
import { Badge, Button, Checkbox, Dialog, DialogBody, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Field, Input, Label, SimpleSelect, Skeleton, Switch, pickDirectory } from "@/components/ui";
import { ErrorNote } from "@/components/app/common";
import { Segmented } from "@/features/settings/bits";
import { ArmSwatch } from "./bits";
import { armColors, armLabel } from "./shared";

const NONE = "__none__";

export function RunDialog({
  open,
  onOpenChange,
  itemId,
  skillName,
  suite,
  runs,
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  itemId: string;
  skillName: string;
  suite: T.EvalSuite | undefined;
  runs: T.EvalRun[] | undefined;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="lg">
        <DialogHeader>
          <DialogTitle>Run evaluation · {skillName}</DialogTitle>
          <DialogDescription>Each case runs in every arm, with repeats, in an isolated sandbox. Results are graded against the suite's expectations and compared against the baseline.</DialogDescription>
        </DialogHeader>
        {open ? <RunDialogBody itemId={itemId} suite={suite} runs={runs} onClose={() => onOpenChange(false)} /> : null}
      </DialogContent>
    </Dialog>
  );
}

function RunDialogBody({ itemId, suite, runs, onClose }: { itemId: string; suite: T.EvalSuite | undefined; runs: T.EvalRun[] | undefined; onClose: () => void }) {
  const cfg = useQuery({ queryKey: ["eval-default-config", itemId], queryFn: () => api.defaultRunConfig(itemId), staleTime: 0, gcTime: 0 });
  if (cfg.isLoading || !cfg.data) {
    return (
      <>
        <DialogBody className="flex flex-col gap-3 pb-5">
          <Skeleton className="h-8 w-full" />
          <Skeleton className="h-24 w-full" />
          <Skeleton className="h-8 w-2/3" />
          <ErrorNote error={cfg.error} />
        </DialogBody>
        {cfg.error ? (
          <DialogFooter>
            <Button variant="ghost" onClick={onClose}>
              Close
            </Button>
            <Button onClick={() => cfg.refetch()}>Retry</Button>
          </DialogFooter>
        ) : null}
      </>
    );
  }
  return <RunForm initial={cfg.data} suite={suite} runs={runs} onClose={onClose} />;
}

function RunForm({ initial, suite, runs, onClose }: { initial: T.RunConfig; suite: T.EvalSuite | undefined; runs: T.EvalRun[] | undefined; onClose: () => void }) {
  const navigate = useNavigate();
  const { data: providers } = useProviders();
  const [cfg, setCfg] = React.useState<T.RunConfig>(initial);
  const [caseMode, setCaseMode] = React.useState<"all" | "subset">(initial.caseIds.length ? "subset" : "all");

  const executors = (providers ?? []).filter((p) => p.roles.includes("executor"));
  const graders = (providers ?? []).filter((p) => p.roles.includes("grader"));
  const cases = suite?.cases ?? [];
  const triggerCount = suite?.triggerCases.length ?? 0;
  const hasBaseline = cfg.arms.some((a) => a.kind !== "with_skill");
  const withoutSkill = cfg.arms.some((a) => a.kind === "without_skill");
  const variants = cfg.arms.filter((a) => a.kind === "variant");
  const colors = armColors(cfg.arms.map((a) => a.label));
  const baseline = cfg.baselineLabel ?? cfg.arms.find((a) => a.kind !== "with_skill")?.label ?? null;

  const selectedCases = caseMode === "all" ? cases.length : cases.filter((c) => cfg.caseIds.includes(c.id)).length;
  const totalRuns = selectedCases * cfg.arms.length * cfg.repeats;
  const triggerQueries = cfg.includeTrigger ? triggerCount : 0;

  // Rough time/cost from this skill's previous finished runs, else from the timeout.
  const prior = React.useMemo(() => {
    let dur = 0;
    let cost = 0;
    let n = 0;
    for (const r of runs ?? []) {
      for (const a of r.summary?.arms ?? []) {
        dur += a.durationMs.mean * a.runs;
        cost += a.costUsd ?? 0;
        n += a.runs;
      }
    }
    return n ? { perRunMs: dur / n, perRunUsd: cost / n } : null;
  }, [runs]);
  const concurrency = Math.max(1, cfg.concurrency);
  const estimateMs = prior ? (totalRuns * prior.perRunMs) / concurrency : (totalRuns * cfg.timeoutSecs * 1000) / concurrency;
  const estimateUsd = prior ? totalRuns * prior.perRunUsd : null;

  const set = (patch: Partial<T.RunConfig>) => setCfg((c) => ({ ...c, ...patch }));
  const setArms = (fn: (arms: T.ArmSpec[]) => T.ArmSpec[]) => setCfg((c) => ({ ...c, arms: fn(c.arms) }));

  const addVariant = async () => {
    const path = await pickDirectory("Choose a skill directory to compare as a variant");
    if (!path) return;
    const n = variants.length + 1;
    let label = `variant-${n}`;
    while (cfg.arms.some((a) => a.label === label)) label = `${label}b`;
    setArms((arms) => [...arms, { label, kind: "variant", path }]);
  };

  const start = useApiMutation(
    (c: T.RunConfig) => api.startRun(c),
    [["runs"], keys.dashboard],
    (run) => {
      toast.success("Evaluation started", { description: `${pluralize(totalRuns, "agent run")}${triggerQueries ? ` + ${pluralize(triggerQueries, "trigger query", "trigger queries")}` : ""}` });
      onClose();
      navigate(`/runs/${run.id}`);
    },
  );

  const problems: string[] = [];
  if (!suite?.exists || cases.length === 0) problems.push("This skill has no saved eval suite yet. Save a suite first.");
  if (caseMode === "subset" && selectedCases === 0) problems.push("Choose at least one case.");
  if (!cfg.executorProviderId) problems.push("Pick an executor provider.");
  if (executors.length && !executors.some((p) => p.id === cfg.executorProviderId)) problems.push("The selected executor no longer exists.");
  const canStart = problems.length === 0 && !start.isPending;

  const submit = () => {
    const config: T.RunConfig = {
      ...cfg,
      caseIds: caseMode === "all" ? [] : cfg.caseIds,
      graderProviderId: cfg.graderProviderId || null,
      baselineLabel: null,
      concurrency: Math.max(1, Math.min(16, cfg.concurrency || 1)),
      timeoutSecs: Math.max(30, cfg.timeoutSecs || 600),
    };
    start.mutate(config);
  };

  return (
    <>
      <DialogBody className="flex flex-col gap-5 pb-4">
        <section className="flex flex-col gap-2">
          <Label>Arms</Label>
          <div className="surface divide-y overflow-hidden">
            <ArmRow color={colors.with_skill} title="With skill" description="The skill is installed in the sandbox; the agent decides whether to use it." right={<Badge tone="accent">Always</Badge>} />
            <ArmRow
              color={colors.without_skill ?? "var(--chart-2)"}
              title="Without skill"
              description="The same prompts with no skill installed. The baseline every other arm is compared against."
              right={
                <Switch
                  checked={withoutSkill}
                  aria-label="Include the without-skill baseline"
                  onCheckedChange={(v) => setArms((arms) => (v ? [...arms.filter((a) => a.kind !== "without_skill"), { label: "without_skill", kind: "without_skill" }] : arms.filter((a) => a.kind !== "without_skill")))}
                />
              }
            />
            {variants.map((v) => (
              <ArmRow
                key={v.label}
                color={colors[v.label]}
                title={
                  <Input
                    value={v.label}
                    onChange={(e) => {
                      const label = e.target.value.replace(/[^a-z0-9_-]/gi, "").toLowerCase();
                      setArms((arms) => arms.map((a) => (a === v ? { ...a, label } : a)));
                    }}
                    aria-label="Variant label"
                    className="mono h-6 w-40 text-xs"
                  />
                }
                description={<span className="mono">{displayPath((v as Extract<T.ArmSpec, { kind: "variant" }>).path)}</span>}
                right={
                  <Button variant="ghost" size="icon-sm" aria-label="Remove variant" onClick={() => setArms((arms) => arms.filter((a) => a !== v))}>
                    <X />
                  </Button>
                }
              />
            ))}
            <div className="flex items-center justify-between gap-3 px-3 py-2">
              <span className="text-xs text-muted">Variants compare another version of the skill (an edited copy, a fork) in the same run.</span>
              <Button size="sm" variant="ghost" onClick={() => void addVariant()}>
                <FolderPlus /> Add variant…
              </Button>
            </div>
          </div>
          {!hasBaseline ? (
            <p className="flex items-center gap-1.5 text-xs text-warn">
              <AlertTriangle className="size-3.5" /> Without a baseline arm there is nothing to compare against; the run will report pass rates but no verdict.
            </p>
          ) : (
            <p className="text-xs text-subtle">
              Deltas are computed against <span className="font-medium text-muted">{armLabel(baseline ?? "")}</span>.
            </p>
          )}
        </section>

        <section className="flex flex-col gap-2">
          <div className="flex items-center justify-between">
            <Label>Cases</Label>
            <Segmented<"all" | "subset">
              ariaLabel="Cases"
              value={caseMode}
              onChange={setCaseMode}
              options={[
                { value: "all", label: `All ${cases.length}` },
                { value: "subset", label: "Choose" },
              ]}
            />
          </div>
          {cases.length === 0 ? (
            <p className="rounded-lg border border-dashed px-3 py-3 text-xs text-muted">No cases in the suite. Draft or write a suite in the Suite tab and save it first.</p>
          ) : caseMode === "subset" ? (
            <div className="surface divide-y overflow-hidden">
              {cases.map((c) => {
                const checked = cfg.caseIds.includes(c.id);
                return (
                  <label key={c.id} className="flex cursor-default items-center gap-3 px-3 py-1.5 text-xs hover:bg-sunken/60">
                    <Checkbox checked={checked} onCheckedChange={(v) => set({ caseIds: v ? [...cfg.caseIds, c.id] : cfg.caseIds.filter((id) => id !== c.id) })} aria-label={c.name} />
                    <span className="mono font-medium">{c.name}</span>
                    <span className="ml-auto text-subtle">
                      {pluralize(c.expectations.length, "expectation")} · {pluralize(c.checks.length, "check")}
                    </span>
                  </label>
                );
              })}
            </div>
          ) : (
            <p className="text-xs text-subtle">{cases.map((c) => c.name).join(" · ")}</p>
          )}
        </section>

        <div className="grid gap-4 sm:grid-cols-2">
          <Field label="Repeats per arm" hint="Agents are nondeterministic; 3 or more repeats give a usable confidence interval.">
            <div className="flex items-center gap-0.5 rounded-lg bg-sunken p-0.5" role="radiogroup" aria-label="Repeats">
              {[1, 2, 3, 4, 5].map((n) => (
                <button key={n} type="button" role="radio" aria-checked={cfg.repeats === n} onClick={() => set({ repeats: n })} className={cn("h-7 flex-1 rounded-md text-xs font-medium num transition-colors", cfg.repeats === n ? "bg-elevated text-fg shadow-sm" : "text-muted hover:text-fg")}>
                  {n}×
                </button>
              ))}
            </div>
          </Field>
          <Field label="Concurrency" hint="Sandboxes run in parallel, bounded by the executor's limit.">
            <Input type="number" min={1} max={16} value={cfg.concurrency} onChange={(e) => set({ concurrency: Number(e.target.value) })} aria-label="Concurrency" />
          </Field>
          <Field
            label="Executor"
            hint={
              executors.length === 0 ? (
                <span className="flex items-center gap-1 text-warn">
                  <AlertTriangle className="size-3" /> No provider can execute tasks.
                  <Button
                    variant="link"
                    size="sm"
                    onClick={() => {
                      onClose();
                      navigate("/settings/providers");
                    }}
                  >
                    <Settings2 /> Configure providers
                  </Button>
                </span>
              ) : (
                "The agent that performs each task."
              )
            }
          >
            <SimpleSelect value={cfg.executorProviderId || undefined} onChange={(v) => set({ executorProviderId: v })} options={executors.map((p) => ({ value: p.id, label: `${p.name}${p.model ? ` · ${p.model}` : ""}` }))} placeholder={executors.length ? "Choose an executor…" : "No executor providers"} disabled={executors.length === 0} />
          </Field>
          <Field label="Grader" hint={cfg.graderProviderId ? "Judges each expectation from the transcript and outputs, citing evidence." : "Without a grader only deterministic checks are evaluated; expectations without a check are skipped."}>
            <SimpleSelect value={cfg.graderProviderId ?? NONE} onChange={(v) => set({ graderProviderId: v === NONE ? null : v })} options={[{ value: NONE, label: <span className="text-muted">None — checks only</span> }, ...graders.map((p) => ({ value: p.id, label: `${p.name}${p.model ? ` · ${p.model}` : ""}` }))]} />
          </Field>
          <Field label="Timeout per run (seconds)" hint="A run that exceeds it is marked as timed out and counts as failed.">
            <Input type="number" min={30} step={30} value={cfg.timeoutSecs} onChange={(e) => set({ timeoutSecs: Number(e.target.value) })} aria-label="Timeout seconds" />
          </Field>
          <Field label="Trigger evaluation" hint={triggerCount ? `Asks the agent which skill it would pick for ${pluralize(triggerCount, "query", "queries")}; reports precision and recall of the description.` : "The suite has no trigger queries."}>
            <label className="flex h-8 items-center gap-2 text-xs">
              <Switch checked={cfg.includeTrigger && triggerCount > 0} disabled={triggerCount === 0} onCheckedChange={(v) => set({ includeTrigger: v })} aria-label="Include trigger evaluation" />
              <span className="text-muted">{cfg.includeTrigger && triggerCount > 0 ? "Included" : "Skipped"}</span>
            </label>
          </Field>
        </div>

        <div className="rounded-lg bg-sunken px-3 py-2.5 text-xs">
          <div className="flex flex-wrap items-baseline gap-x-2">
            <span className="text-[15px] font-semibold num">{totalRuns}</span>
            <span className="text-muted">
              agent runs = {selectedCases} {selectedCases === 1 ? "case" : "cases"} × {cfg.arms.length} {cfg.arms.length === 1 ? "arm" : "arms"} × {cfg.repeats} {cfg.repeats === 1 ? "repeat" : "repeats"}
              {triggerQueries ? ` + ${pluralize(triggerQueries, "trigger query", "trigger queries")}` : ""}
            </span>
          </div>
          <div className="mt-1 text-muted">
            {prior ? (
              <>
                Roughly <span className="font-medium text-fg">{formatDuration(estimateMs)}</span> at ~{formatDuration(prior.perRunMs)} per run with concurrency {concurrency}
                {estimateUsd ? (
                  <>
                    {" "}
                    · about <span className="font-medium text-fg">${estimateUsd.toFixed(2)}</span> based on previous runs
                  </>
                ) : null}
                .
              </>
            ) : (
              <>
                Up to <span className="font-medium text-fg">{formatDuration(estimateMs)}</span> at the {cfg.timeoutSecs} s timeout with concurrency {concurrency}; usually much less.
              </>
            )}
          </div>
        </div>

        {problems.length ? (
          <ul className="flex flex-col gap-1 text-xs text-warn">
            {problems.map((p) => (
              <li key={p} className="flex items-center gap-1.5">
                <AlertTriangle className="size-3.5 shrink-0" /> {p}
              </li>
            ))}
          </ul>
        ) : null}
        <ErrorNote error={start.error} />
      </DialogBody>
      <DialogFooter>
        <Button variant="ghost" onClick={onClose}>
          Cancel
        </Button>
        <Button variant="primary" disabled={!canStart} loading={start.isPending} onClick={submit}>
          <Play /> Start {totalRuns ? `${totalRuns} runs` : ""}
        </Button>
      </DialogFooter>
    </>
  );
}

function ArmRow({ color, title, description, right }: { color: string; title: React.ReactNode; description: React.ReactNode; right: React.ReactNode }) {
  return (
    <div className="flex items-center gap-3 px-3 py-2">
      <ArmSwatch color={color} />
      <div className="min-w-0 flex-1">
        <div className="text-[13px] font-medium">{title}</div>
        <div className="truncate text-xs text-muted">{description}</div>
      </div>
      <div className="shrink-0">{right}</div>
    </div>
  );
}
