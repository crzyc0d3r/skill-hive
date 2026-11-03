import * as React from "react";
import { useMutation } from "@tanstack/react-query";
import { toast } from "sonner";
import { AlertTriangle, Check, ListChecks, Minus, Plus, Rocket } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation, useProjects, useSettings, useTargets } from "@/lib/api/hooks";
import { cn, displayPath, pluralize } from "@/lib/utils";
import { Button, Field, SimpleSelect } from "@/components/ui";
import { BudgetMeter, ConflictList, ErrorNote, KindIcon } from "@/components/app/common";
import { ScopePicker } from "@/features/agents/ScopePicker";

const STRATEGIES: { value: T.ApplyStrategy; label: string; hint: string }[] = [
  { value: "exclusive", label: "Exclusive", hint: "The agent's managed set becomes exactly this loadout; other managed items are removed. Unmanaged files stay." },
  { value: "additive", label: "Additive", hint: "Add the loadout's items on top of whatever is already installed." },
];

/** Choose where and how to apply a loadout, plan it, then apply. */
export function ApplyLoadoutCard({ loadout, targetId, onTargetChange }: { loadout: T.Loadout; targetId: string; onTargetChange: (id: string) => void }) {
  const { data: targets } = useTargets();
  const { data: projects } = useProjects();
  const { data: settings } = useSettings();
  const [scope, setScope] = React.useState<T.Scope>("user");
  const [projectOverride, setProjectOverride] = React.useState<string | null>(null);
  const projectPath = projectOverride ?? projects?.[0]?.path ?? null;
  const [strategy, setStrategy] = React.useState<T.ApplyStrategy>("exclusive");
  const [mode, setMode] = React.useState<T.LinkMode | null>(null);
  const target = targets?.find((t) => t.id === targetId);
  const effectiveMode = mode ?? settings?.defaultLinkMode ?? "link";

  const request: T.ApplyLoadoutRequest = { loadoutId: loadout.id, targetId, scope, projectPath: scope === "project" ? projectPath : null, strategy, mode };
  const requestKey = JSON.stringify({ request, items: loadout.itemIds, updatedAt: loadout.updatedAt });
  const [plannedKey, setPlannedKey] = React.useState<string | null>(null);
  const plan = useMutation({ mutationFn: (req: T.ApplyLoadoutRequest) => api.planLoadout(req) });
  const current = plan.data && plannedKey === requestKey ? plan.data : null;
  const stale = !!plan.data && !current && !plan.isPending;

  const apply = useApiMutation(
    (req: T.ApplyLoadoutRequest) => api.applyLoadout(req),
    [keys.deployments, keys.targets, keys.itemsAll, ["item"], keys.dashboard, keys.loadouts],
    (res) => {
      const parts = [res.deployed.length ? `${pluralize(res.deployed.length, "item")} deployed` : null, res.removed.length ? `${pluralize(res.removed.length, "item")} removed` : null].filter(Boolean);
      if (res.failed.length) toast.warning(`Applied with ${pluralize(res.failed.length, "failure")}`, { description: res.failed[0].reason });
      else toast.success(`Applied ${loadout.name} to ${target?.name ?? targetId}`, { description: parts.join(", ") || "Nothing needed to change" });
    },
  );

  const ready = scope === "user" || !!projectPath;

  return (
    <div className="surface flex flex-col gap-4 p-4 xl:sticky xl:top-0">
      <div>
        <div className="text-[13px] font-semibold">Apply to an agent</div>
        <p className="text-xs text-muted">Plan first to see exactly what changes, then apply.</p>
      </div>
      <Field label="Agent">
        <SimpleSelect value={targetId} onChange={onTargetChange} options={(targets ?? []).map((t) => ({ value: t.id, label: t.name }))} />
      </Field>
      <Field label="Scope">
        <ScopePicker scope={scope} onScopeChange={setScope} projectPath={projectPath} onProjectChange={setProjectOverride} projectDisabled={!target?.projectSkillsDir} />
      </Field>
      <Field label="Strategy">
        <div role="radiogroup" aria-label="Strategy" className="grid grid-cols-2 gap-2">
          {STRATEGIES.map((s) => (
            <button
              key={s.value}
              type="button"
              role="radio"
              aria-checked={strategy === s.value}
              onClick={() => setStrategy(s.value)}
              className={cn("flex flex-col gap-0.5 rounded-lg px-3 py-2 text-left text-xs transition-colors hairline", strategy === s.value ? "bg-accent-soft/60 ring-1 ring-accent/50" : "bg-elevated hover:bg-sunken")}
            >
              <span className="font-medium">{s.label}</span>
              <span className="text-[11px] text-muted">{s.hint}</span>
            </button>
          ))}
        </div>
      </Field>
      <Field label="Mode">
        <SimpleSelect
          value={effectiveMode}
          onChange={(v) => setMode(v as T.LinkMode)}
          options={[
            { value: "link", label: "Link (follows the library)" },
            { value: "copy", label: "Copy (independent)" },
          ]}
        />
      </Field>
      <div className="flex gap-2">
        <Button
          variant="secondary"
          className="flex-1"
          disabled={!ready}
          loading={plan.isPending}
          onClick={() => {
            setPlannedKey(requestKey);
            plan.mutate(request);
          }}
        >
          <ListChecks /> Plan
        </Button>
        <Button variant="primary" className="flex-1" disabled={!current || !ready} loading={apply.isPending} onClick={() => apply.mutate(request)}>
          <Rocket /> Apply
        </Button>
      </div>
      <ErrorNote error={plan.error ?? apply.error} />
      {stale ? (
        <p className="flex items-center gap-1.5 text-[11px] text-warn">
          <AlertTriangle className="size-3" /> Settings changed since the last plan. Plan again before applying.
        </p>
      ) : null}
      {current ? <PlanView plan={current} budgetChars={target?.descriptionBudgetChars} /> : null}
      {apply.data ? <ApplyResultView result={apply.data} /> : null}
    </div>
  );
}

export function PlanView({ plan, budgetChars }: { plan: T.LoadoutPlan; budgetChars?: number | null }) {
  const sections = [
    { key: "adds", title: "Add", items: plan.adds, tone: "text-ok", icon: Plus },
    { key: "removes", title: "Remove", items: plan.removes, tone: "text-danger", icon: Minus },
    { key: "keeps", title: "Keep", items: plan.keeps, tone: "text-muted", icon: Check },
    { key: "unmanaged", title: "Unmanaged", items: plan.unmanaged, tone: "text-warn", icon: AlertTriangle },
  ];
  const nothing = plan.adds.length === 0 && plan.removes.length === 0;
  return (
    <div className="flex flex-col gap-3 border-t pt-3" aria-label="Plan">
      <div className="grid grid-cols-4 gap-1.5 text-center text-[11px]">
        {sections.map((s) => (
          <div key={s.key} className="rounded-md bg-sunken px-1 py-1.5">
            <div className={cn("num text-base font-semibold", s.items.length ? s.tone : "text-subtle")}>{s.items.length}</div>
            <div className="text-subtle">{s.title}</div>
          </div>
        ))}
      </div>
      {nothing ? <p className="text-xs text-muted">Already applied: nothing to add or remove.</p> : null}
      {sections
        .filter((s) => s.items.length)
        .map((s) => (
          <section key={s.key} aria-label={s.title}>
            <div className="mb-1 flex items-center gap-1 text-[11px] font-medium uppercase tracking-wide text-subtle">
              <s.icon className={cn("size-3", s.tone)} /> {s.title}
              <span className="num">({s.items.length})</span>
              {s.key === "unmanaged" ? <span className="ml-1 normal-case tracking-normal">stay in place, may collide</span> : null}
            </div>
            <ul className="flex flex-col gap-0.5">
              {s.items.map((it) => (
                <li key={`${s.key}:${it.path ?? it.name}`} className="flex items-center gap-2 rounded-md bg-sunken/70 px-2 py-1 text-xs">
                  <KindIcon kind={it.kind} className="size-3.5 shrink-0 text-muted" />
                  <span className="truncate font-medium">{it.name}</span>
                  {it.path ? (
                    <span className="mono min-w-0 truncate text-[10.5px] text-subtle" title={it.path}>
                      {displayPath(it.path)}
                    </span>
                  ) : null}
                  <span className="num ml-auto shrink-0 text-[11px] text-subtle">{it.descriptionTokens} tok</span>
                </li>
              ))}
            </ul>
          </section>
        ))}
      <BudgetMeter chars={plan.budget.descriptionChars} budgetChars={plan.budget.budgetChars ?? budgetChars} tokens={plan.budget.descriptionTokens} label="Budget after applying" />
      <div>
        <div className="mb-1 text-[11px] font-medium uppercase tracking-wide text-subtle">Resulting conflicts</div>
        <ConflictList report={plan.conflicts} compact />
      </div>
    </div>
  );
}

function ApplyResultView({ result }: { result: T.ApplyLoadoutResult }) {
  return (
    <div className="flex flex-col gap-1.5 border-t pt-3 text-xs" role="status">
      <div className="flex items-center gap-1.5 font-medium">
        {result.failed.length ? <AlertTriangle className="size-3.5 text-warn" /> : <Check className="size-3.5 text-ok" />}
        Applied: {pluralize(result.deployed.length, "item")} deployed, {result.removed.length} removed
        {result.failed.length ? `, ${result.failed.length} failed` : ""}
      </div>
      {result.failed.length ? (
        <ul className="flex flex-col gap-1">
          {result.failed.map((f) => (
            <li key={f.itemId} className="rounded-md bg-danger-soft px-2 py-1 text-danger">
              {f.reason}
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
