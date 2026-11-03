import * as React from "react";
import { useMutation } from "@tanstack/react-query";
import { useSearchParams } from "react-router-dom";
import { AlertTriangle, Crosshair, Info, Search, Sparkles, X } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { useAnalyze, useItems, useProviders, useTargets } from "@/lib/api/hooks";
import { useUi } from "@/lib/store";
import { cn, modKey, pluralize } from "@/lib/utils";
import { Badge, Button, EmptyState, Kbd, Progress, SimpleSelect, Skeleton, Switch, Textarea, Tooltip } from "@/components/ui";
import { EffectivenessBadge, ErrorNote, InlineCode, KIND_LABEL, KindIcon, Page, PageBody, PageHeader, ValidationBadge } from "@/components/app/common";
import { ALL_ITEMS_QUERY, estimateDescriptionChars, sameSet } from "@/features/agents/queries";
import { SetPanel } from "./SetPanel";

const EXAMPLES = [
  "pull the tables out of a PDF invoice and build a spreadsheet",
  "review a pull request for security issues and write commit messages",
  "debug a failing test and add a regression test",
  "build a slide deck summarising a quarterly report",
];

/** Find: describe a job, get a ranked and conflict-free set; or analyse a set given as `?ids=`. */
export function FindPage() {
  const [params, setParams] = useSearchParams();
  const ids = React.useMemo(
    () =>
      (params.get("ids") ?? "")
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean),
    [params],
  );
  const { data: targets } = useTargets();
  const [targetId, setTargetId] = React.useState("claude-code");
  const targetOptions = (targets ?? []).map((t) => ({ value: t.id, label: t.name }));

  if (ids.length) {
    return (
      <Page>
        <PageHeader
          title="Find"
          description={`Checking ${pluralize(ids.length, "item")} for conflicts and context cost on ${targets?.find((t) => t.id === targetId)?.name ?? targetId}.`}
          actions={
            <>
              <SimpleSelect size="sm" className="w-44" value={targetId} onChange={setTargetId} options={targetOptions} />
              <Button variant="secondary" onClick={() => setParams({})}>
                <Search /> New search
              </Button>
            </>
          }
        />
        <PageBody>
          <AnalysisView ids={ids} targetId={targetId} onRemove={(id) => setParams(ids.filter((x) => x !== id).length ? { ids: ids.filter((x) => x !== id).join(",") } : {})} />
        </PageBody>
      </Page>
    );
  }

  return (
    <Page>
      <PageHeader title="Find" description="Describe the job. Skill Hive ranks your library, drops overlapping skills and keeps the set inside the agent's context budget." />
      <PageBody className="flex flex-col gap-4">
        <SearchForm targetId={targetId} onTargetChange={setTargetId} targetOptions={targetOptions} />
      </PageBody>
    </Page>
  );
}

function SearchForm({ targetId, onTargetChange, targetOptions }: { targetId: string; onTargetChange: (id: string) => void; targetOptions: { value: string; label: string }[] }) {
  const [task, setTask] = React.useState("");
  const [kinds, setKinds] = React.useState<T.AssetKind[]>([]);
  const [useLlm, setUseLlm] = React.useState(false);
  const { data: providers } = useProviders();
  const recommender = providers?.find((p) => p.roles.includes("recommender"));
  const recommend = useMutation({ mutationFn: (req: T.RecommendRequest) => api.recommend(req) });

  const submit = (text = task) => {
    const t = text.trim();
    if (!t) return;
    recommend.mutate({ task: t, kinds, limit: 12, targetId, excludeIds: [], useLlm });
  };
  const toggleKind = (k: T.AssetKind) => setKinds((prev) => (prev.includes(k) ? prev.filter((x) => x !== k) : [...prev, k]));

  return (
    <>
      <div className="surface flex flex-col gap-3 p-4">
        <Textarea
          autoFocus
          value={task}
          onChange={(e) => setTask(e.target.value)}
          onKeyDown={(e) => {
            if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
              e.preventDefault();
              submit();
            }
          }}
          rows={3}
          placeholder="Describe the job, e.g. 'pull the tables out of a PDF invoice and build a spreadsheet'"
          aria-label="Task description"
          className="min-h-[76px] text-sm leading-6"
        />
        <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
          <label className="flex items-center gap-2 text-xs text-muted">
            Budget for
            <SimpleSelect size="sm" className="w-40" value={targetId} onChange={onTargetChange} options={targetOptions} />
          </label>
          <div className="flex items-center gap-0.5 rounded-lg bg-sunken p-0.5" role="group" aria-label="Kinds">
            {(["skill", "agent", "command"] as T.AssetKind[]).map((k) => (
              <button
                key={k}
                type="button"
                onClick={() => toggleKind(k)}
                aria-pressed={kinds.includes(k)}
                className={cn("h-7 rounded-md px-2.5 text-xs font-medium transition-colors", kinds.includes(k) ? "bg-elevated text-fg shadow-sm" : "text-muted hover:text-fg")}
              >
                {KIND_LABEL[k]}s
              </button>
            ))}
          </div>
          <label className="flex items-center gap-2 text-xs">
            <Switch checked={useLlm} onCheckedChange={setUseLlm} aria-label="Ask a model to re-rank" />
            <span className="flex items-center gap-1">
              Ask a model to re-rank
              <Tooltip content="Sends the task and the top candidates to a provider with the recommender role, which re-orders them and explains why.">
                <Info className="size-3.5 text-subtle" />
              </Tooltip>
            </span>
          </label>
          <span className="ml-auto flex items-center gap-2">
            <span className="hidden items-center gap-1 text-[11px] text-subtle sm:flex">
              <Kbd>{modKey()}</Kbd>
              <Kbd>↵</Kbd>
            </span>
            <Button variant="primary" onClick={() => submit()} loading={recommend.isPending} disabled={!task.trim()}>
              <Crosshair /> Find
            </Button>
          </span>
        </div>
        {useLlm ? (
          recommender ? (
            <p className="flex items-center gap-1.5 text-[11px] text-subtle">
              <Sparkles className="size-3" /> {recommender.name}
              {recommender.model ? ` (${recommender.model})` : ""} re-ranks and explains the top candidates.
            </p>
          ) : (
            <p className="flex items-center gap-1.5 text-[11px] text-warn">
              <AlertTriangle className="size-3" /> No provider has the recommender role. Add one under Settings → Providers; until then the built-in ranking is used.
            </p>
          )
        ) : null}
      </div>

      {recommend.isPending ? (
        <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(300px,360px)]" aria-busy="true" aria-label="Finding skills">
          <div className="surface flex flex-col gap-2 p-3">
            {Array.from({ length: 5 }).map((_, i) => (
              <Skeleton key={i} className="h-14" />
            ))}
          </div>
          <Skeleton className="h-64" />
        </div>
      ) : recommend.isError ? (
        <ErrorNote error={recommend.error} />
      ) : recommend.data ? (
        <ResultsView key={recommend.submittedAt} response={recommend.data} targetId={targetId} />
      ) : (
        <EmptyState
          icon={Crosshair}
          title="What are you about to do?"
          description="Try one of these, or describe your own job above."
          action={
            <div className="flex flex-wrap justify-center gap-1.5">
              {EXAMPLES.map((e) => (
                <button
                  key={e}
                  type="button"
                  onClick={() => {
                    setTask(e);
                    submit(e);
                  }}
                  className="rounded-full bg-sunken px-2.5 py-1 text-[11px] text-muted hover:text-fg"
                >
                  {e}
                </button>
              ))}
            </div>
          }
        />
      )}
    </>
  );
}

/** Keyed per response: the selection starts as the suggested set. */
function ResultsView({ response, targetId }: { response: T.RecommendResponse; targetId: string }) {
  const openItem = useUi((s) => s.openItem);
  const { data: targets } = useTargets();
  const [selected, setSelected] = React.useState<string[]>(response.suggestedIds);
  const results = response.results;
  const byId = React.useMemo(() => new Map(results.map((r) => [r.item.id, r])), [results]);
  const selectedItems = selected.map((id) => byId.get(id)?.item).filter((i): i is T.LibraryItem => !!i);
  const suggested = sameSet(selected, response.suggestedIds);
  const analysis = useAnalyze(selected, targetId);
  const budgetChars = targets?.find((t) => t.id === targetId)?.descriptionBudgetChars ?? response.budget.budgetChars;
  const chars = suggested ? response.budget.descriptionChars : estimateDescriptionChars(selectedItems);
  const tokens = suggested ? response.budget.descriptionTokens : selectedItems.reduce((n, i) => n + i.descriptionTokens, 0);
  const toggle = (id: string) => setSelected((s) => (s.includes(id) ? s.filter((x) => x !== id) : [...s, id]));

  if (results.length === 0) {
    return <EmptyState icon={Search} title="Nothing in the library matches" description="Try different words, or collect more skills from Discover." />;
  }

  return (
    <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(300px,360px)]">
      <div className="surface min-w-0 overflow-hidden">
        <div className="flex items-center gap-2 border-b bg-sunken/60 px-3 py-1.5 text-[11px]">
          <span className="font-medium">{pluralize(results.length, "match")}</span>
          <span className="text-subtle">· {response.suggestedIds.length} suggested</span>
          {!suggested ? (
            <Button size="sm" variant="ghost" className="ml-auto" onClick={() => setSelected(response.suggestedIds)}>
              Reset to suggestion
            </Button>
          ) : null}
        </div>
        <ol className="divide-y">
          {results.map((r, i) => {
            const on = selected.includes(r.item.id);
            const conflictNames = r.conflictsWith.map((id) => byId.get(id)?.item.name ?? id);
            return (
              <li key={r.item.id} className={cn("flex items-start gap-3 px-3 py-2.5 text-xs transition-colors", on && "bg-accent-soft/30")}>
                <span className="num w-4 pt-1 text-right text-[11px] text-subtle">{i + 1}</span>
                <Tooltip content={`Score ${Math.round(r.score * 100)} of 100`}>
                  <span className="flex w-14 shrink-0 flex-col gap-1 pt-1.5">
                    <Progress value={r.score} size="sm" tone={on ? "accent" : "info"} />
                    <span className="num text-[10.5px] text-subtle">{Math.round(r.score * 100)}</span>
                  </span>
                </Tooltip>
                <span className="flex size-6 shrink-0 items-center justify-center rounded-md bg-sunken text-muted">
                  <KindIcon kind={r.item.kind} className="size-3.5" />
                </span>
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-1.5">
                    <button type="button" onClick={() => openItem(r.item.id)} className="truncate text-[13px] font-medium hover:underline">
                      {r.item.name}
                    </button>
                    <ValidationBadge status={r.item.validation} />
                    <EffectivenessBadge e={r.item.effectiveness} />
                    {conflictNames.length ? (
                      <Tooltip content="Overlapping descriptions: an agent will not reliably pick the right one">
                        <span>
                          <Badge tone="warn">
                            <AlertTriangle /> conflicts with {conflictNames.join(", ")}
                          </Badge>
                        </span>
                      </Tooltip>
                    ) : null}
                  </div>
                  <p className="line-clamp-2 text-muted">{r.item.description}</p>
                  {r.reasons.length || r.matchedTerms.length ? (
                    <div className="mt-1 flex flex-wrap items-center gap-1">
                      {r.reasons.map((reason) => (
                        <Badge key={reason} tone="neutral">
                          {reason}
                        </Badge>
                      ))}
                      {r.matchedTerms.map((t) => (
                        <InlineCode key={t} className="text-[10.5px]">
                          {t}
                        </InlineCode>
                      ))}
                    </div>
                  ) : null}
                </div>
                <label className="flex shrink-0 flex-col items-center gap-1 pt-0.5 text-[10.5px] text-subtle">
                  <Switch checked={on} onCheckedChange={() => toggle(r.item.id)} aria-label={`Include ${r.item.name}`} />
                  {on ? "in set" : "excluded"}
                </label>
              </li>
            );
          })}
        </ol>
      </div>
      <SetPanel
        items={selectedItems}
        chars={chars}
        budgetChars={budgetChars}
        tokens={tokens}
        estimated={!suggested}
        conflicts={analysis.data}
        conflictsLoading={analysis.isFetching}
        conflictsError={analysis.error}
        onRemove={toggle}
        targetId={targetId}
        llmNotes={response.llmNotes}
      />
    </div>
  );
}

/** `?ids=` mode: conflicts and budget for a given set, with the same actions. */
function AnalysisView({ ids, targetId, onRemove }: { ids: string[]; targetId: string; onRemove: (id: string) => void }) {
  const openItem = useUi((s) => s.openItem);
  const { data: all, isLoading, error } = useItems(ALL_ITEMS_QUERY);
  const { data: targets } = useTargets();
  const analysis = useAnalyze(ids, targetId);
  const items = ids.map((id) => all?.find((i) => i.id === id)).filter((i): i is T.LibraryItem => !!i);
  const missing = all ? ids.length - items.length : 0;
  const budgetChars = targets?.find((t) => t.id === targetId)?.descriptionBudgetChars;

  if (isLoading && !all) {
    return (
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(300px,360px)]" aria-busy="true">
        <Skeleton className="h-48" />
        <Skeleton className="h-64" />
      </div>
    );
  }
  if (error) return <ErrorNote error={error} />;

  return (
    <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(300px,360px)]">
      <div className="surface min-w-0 overflow-hidden">
        <div className="flex items-center gap-2 border-b bg-sunken/60 px-3 py-1.5 text-[11px]">
          <span className="font-medium">Items being checked</span>
          <span className="text-subtle">· {ids.length}</span>
          {missing ? <Badge tone="warn">{pluralize(missing, "id")} not in the library</Badge> : null}
        </div>
        {items.length === 0 ? (
          <div className="px-3 py-8 text-center text-xs text-muted">None of these ids are in the library any more.</div>
        ) : (
          <ul className="divide-y">
            {items.map((it) => (
              <li key={it.id} className="flex items-center gap-3 px-3 py-2 text-xs">
                <span className="flex size-6 shrink-0 items-center justify-center rounded-md bg-sunken text-muted">
                  <KindIcon kind={it.kind} className="size-3.5" />
                </span>
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-1.5">
                    <button type="button" onClick={() => openItem(it.id)} className="truncate text-[13px] font-medium hover:underline">
                      {it.name}
                    </button>
                    <ValidationBadge status={it.validation} />
                  </div>
                  <p className="line-clamp-1 text-muted">{it.description}</p>
                </div>
                <span className="num shrink-0 text-[11px] text-subtle">{it.descriptionTokens} tok</span>
                <Tooltip content="Leave out of this check">
                  <Button size="icon-sm" variant="ghost" aria-label={`Remove ${it.name}`} onClick={() => onRemove(it.id)}>
                    <X />
                  </Button>
                </Tooltip>
              </li>
            ))}
          </ul>
        )}
      </div>
      <SetPanel
        title="This set"
        items={items}
        chars={estimateDescriptionChars(items)}
        budgetChars={budgetChars}
        tokens={items.reduce((n, i) => n + i.descriptionTokens, 0)}
        estimated
        conflicts={analysis.data}
        conflictsLoading={analysis.isFetching}
        conflictsError={analysis.error}
        onRemove={onRemove}
        targetId={targetId}
      />
    </div>
  );
}
