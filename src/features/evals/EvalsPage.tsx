// Evals: a rail of skills on the left, and for the selected skill its suite editor and runs.

import * as React from "react";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import { BookOpen, FlaskConical, Play, Search, X } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { useItems, useRuns, useSuite } from "@/lib/api/hooks";
import { useUi } from "@/lib/store";
import { cn, pluralize } from "@/lib/utils";
import { Button, EmptyState, Input, LineTabsList, LineTabsTrigger, Skeleton, Stat, Tabs, TabsContent } from "@/components/ui";
import { ErrorNote, Page, PageBody, PageHeader, VerdictBadge } from "@/components/app/common";
import { SuiteEditor } from "./SuiteEditor";
import { RunDialog } from "./RunDialog";
import { RunsTab, RunsTable } from "./RunsTab";
import { isActiveRun } from "./shared";

const SKILLS_QUERY: T.LibraryQuery = { text: null, kinds: ["skill"], tags: [], sourceKinds: [], favoritesOnly: false, deployedTo: null, sort: "name", limit: null };

export function EvalsPage() {
  const { itemId } = useParams<{ itemId: string }>();
  const navigate = useNavigate();
  const [filter, setFilter] = React.useState("");
  const items = useItems(SKILLS_QUERY);
  const runs = useRuns();

  const runCounts = React.useMemo(() => {
    const out = new Map<string, number>();
    for (const r of runs.data ?? []) if (r.itemId) out.set(r.itemId, (out.get(r.itemId) ?? 0) + 1);
    return out;
  }, [runs.data]);
  const activeByItem = React.useMemo(() => {
    const out = new Set<string>();
    for (const r of runs.data ?? []) if (r.itemId && isActiveRun(r.status)) out.add(r.itemId);
    return out;
  }, [runs.data]);

  const skills = items.data ?? [];
  const q = filter.trim().toLowerCase();
  const visible = q ? skills.filter((s) => s.name.toLowerCase().includes(q) || s.description.toLowerCase().includes(q)) : skills;
  const selected = itemId ? skills.find((s) => s.id === itemId) : undefined;

  return (
    <Page>
      <div className="flex min-h-0 flex-1">
        <aside className="flex w-[272px] shrink-0 flex-col border-r">
          <div className="px-4 pt-5 pb-3">
            <h1 className="text-lg font-semibold tracking-[-0.015em]">Evals</h1>
            <p className="mt-0.5 text-xs text-muted">Measure whether a skill helps.</p>
            <div className="relative mt-3">
              <Search className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-subtle" />
              <Input value={filter} onChange={(e) => setFilter(e.target.value)} placeholder="Filter skills…" aria-label="Filter skills" className="pl-8 pr-7" />
              {filter ? (
                <button type="button" onClick={() => setFilter("")} className="absolute right-2 top-1/2 -translate-y-1/2 text-subtle hover:text-fg" aria-label="Clear filter">
                  <X className="size-3.5" />
                </button>
              ) : null}
            </div>
          </div>
          <nav className="min-h-0 flex-1 scroll-y px-2 pb-3" aria-label="Skills">
            {items.isLoading ? (
              <div className="flex flex-col gap-1 px-1">
                {Array.from({ length: 6 }).map((_, i) => (
                  <Skeleton key={i} className="h-11" />
                ))}
              </div>
            ) : items.isError ? (
              <ErrorNote error={items.error} className="mx-1" />
            ) : visible.length === 0 ? (
              <p className="px-2 py-6 text-center text-xs text-muted">{skills.length === 0 ? "No skills in the library yet." : "No skills match."}</p>
            ) : (
              <ul className="flex flex-col gap-0.5">
                {visible.map((s) => {
                  const count = runCounts.get(s.id) ?? 0;
                  const active = itemId === s.id;
                  return (
                    <li key={s.id}>
                      <button
                        type="button"
                        onClick={() => navigate(`/evals/${s.id}`)}
                        aria-current={active ? "page" : undefined}
                        className={cn(
                          "flex w-full flex-col gap-0.5 rounded-lg px-2.5 py-1.5 text-left transition-colors",
                          active ? "bg-elevated shadow-sm hairline" : "hover:bg-sunken/70",
                        )}
                      >
                        <span className="flex items-center gap-2">
                          <span className="min-w-0 flex-1 truncate text-[13px] font-medium">{s.name}</span>
                          {s.effectiveness ? <VerdictBadge verdict={s.effectiveness.verdict} /> : null}
                        </span>
                        <span className="flex items-center gap-1.5 text-[11px] text-subtle">
                          {activeByItem.has(s.id) ? <span className="inline-block size-1.5 animate-pulse-soft rounded-full bg-info" aria-label="Running" /> : null}
                          {count ? pluralize(count, "run") : "Not measured"}
                          {s.effectiveness ? <span className="num">· {Math.round(s.effectiveness.withSkillPassRate * 100)}% vs {Math.round(s.effectiveness.baselinePassRate * 100)}%</span> : null}
                        </span>
                      </button>
                    </li>
                  );
                })}
              </ul>
            )}
          </nav>
        </aside>

        <div className="flex min-w-0 flex-1 flex-col">
          {itemId ? (
            items.isLoading ? (
              <div className="flex flex-col gap-3 px-6 pt-6">
                <Skeleton className="h-6 w-56" />
                <Skeleton className="h-4 w-full max-w-xl" />
                <Skeleton className="mt-4 h-48 w-full" />
              </div>
            ) : selected ? (
              <SkillEvals key={selected.id} item={selected} runs={(runs.data ?? []).filter((r) => r.itemId === selected.id)} />
            ) : (
              <PageBody className="pt-6">
                <EmptyState icon={FlaskConical} title="Skill not found" description="It may have been removed from the library." action={<Button onClick={() => navigate("/evals")}>Back to evals</Button>} />
              </PageBody>
            )
          ) : (
            <Overview skills={skills} runs={runs.data} loading={runs.isLoading} />
          )}
        </div>
      </div>
    </Page>
  );
}

function SkillEvals({ item, runs }: { item: T.LibraryItem; runs: T.EvalRun[] }) {
  const [params, setParams] = useSearchParams();
  const tab = params.get("tab") === "runs" ? "runs" : "suite";
  const [runOpen, setRunOpen] = React.useState(false);
  const openItem = useUi((s) => s.openItem);
  const suite = useSuite(item.id);

  const setTab = (t: string) => {
    const next = new URLSearchParams(params);
    if (t === "suite") next.delete("tab");
    else next.set("tab", t);
    setParams(next, { replace: true });
  };

  return (
    <Tabs value={tab} onValueChange={setTab} className="flex min-h-0 flex-1 flex-col">
      <PageHeader
        title={
          <span className="flex items-center gap-2">
            <span className="selectable">{item.name}</span>
            {item.effectiveness ? <VerdictBadge verdict={item.effectiveness.verdict} delta={item.effectiveness.delta} /> : null}
          </span>
        }
        description={<span className="selectable">{item.description}</span>}
        actions={
          <>
            <Button variant="ghost" onClick={() => openItem(item.id)}>
              <BookOpen /> Open in library
            </Button>
            <Button variant="primary" onClick={() => setRunOpen(true)}>
              <Play /> Run evaluation…
            </Button>
          </>
        }
        className="pb-0"
      >
        <LineTabsList>
          <LineTabsTrigger value="suite">
            Suite
            {suite.data ? <span className="num text-subtle">{suite.data.cases.length}</span> : null}
          </LineTabsTrigger>
          <LineTabsTrigger value="runs">
            Runs
            <span className="num text-subtle">{runs.length}</span>
          </LineTabsTrigger>
        </LineTabsList>
      </PageHeader>

      <TabsContent value="suite" className="mt-0 flex min-h-0 flex-1 flex-col data-[state=inactive]:hidden">
        {suite.isLoading ? (
          <div className="flex flex-col gap-3 px-6 pt-4">
            <Skeleton className="h-4 w-80" />
            <Skeleton className="h-28 w-full" />
            <Skeleton className="h-28 w-full" />
          </div>
        ) : suite.isError ? (
          <div className="px-6 pt-4">
            <ErrorNote error={suite.error} />
            <Button className="mt-3" onClick={() => suite.refetch()}>
              Retry
            </Button>
          </div>
        ) : suite.data ? (
          <SuiteEditor key={item.id} suite={suite.data} />
        ) : null}
      </TabsContent>
      <TabsContent value="runs" className="mt-0 flex min-h-0 flex-1 flex-col data-[state=inactive]:hidden">
        <PageBody>
          <RunsTab itemId={item.id} onRun={() => setRunOpen(true)} />
        </PageBody>
      </TabsContent>

      <RunDialog open={runOpen} onOpenChange={setRunOpen} itemId={item.id} skillName={item.name} suite={suite.data} runs={runs} />
    </Tabs>
  );
}

function Overview({ skills, runs, loading }: { skills: T.LibraryItem[]; runs: T.EvalRun[] | undefined; loading: boolean }) {
  const navigate = useNavigate();
  const list = runs ?? [];
  const measured = skills.filter((s) => s.effectiveness).length;
  const helping = skills.filter((s) => s.effectiveness && (s.effectiveness.verdict === "helps" || s.effectiveness.verdict === "helps_a_lot")).length;
  const running = list.filter((r) => isActiveRun(r.status)).length;
  return (
    <>
      <PageHeader title="Evals" description="Pick a skill on the left to edit its eval suite and run it with and without the skill. Every run grades the same prompts in each arm and reports a verdict with a confidence interval." />
      <PageBody className="flex flex-col gap-5">
        {skills.length === 0 && !loading ? (
          <EmptyState
            icon={FlaskConical}
            title="No skills to evaluate"
            description="Collect or write a skill first; evaluations measure whether a skill's instructions help an agent."
            action={
              <Button variant="primary" onClick={() => navigate("/discover")}>
                Collect skills
              </Button>
            }
          />
        ) : (
          <>
            <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
              <Stat label="Skills" value={skills.length} hint="in the library" />
              <Stat label="Measured" value={measured} hint={measured ? `${helping} helping` : "none evaluated yet"} tone={measured ? "accent" : undefined} />
              <Stat label="Runs" value={list.length} hint="all skills" />
              <Stat label="Running now" value={running} hint={running ? "updating live" : "nothing in progress"} tone={running ? "ok" : undefined} />
            </div>
            <section className="flex flex-col gap-3">
              <div>
                <h2 className="text-[15px] font-semibold tracking-[-0.01em]">Recent runs</h2>
                <p className="mt-0.5 text-xs text-muted">Across every skill, newest first.</p>
              </div>
              {loading ? (
                <Skeleton className="h-40 w-full" />
              ) : list.length === 0 ? (
                <EmptyState compact icon={FlaskConical} title="No runs yet" description="Select a skill to draft a suite and start the first evaluation." />
              ) : (
                <RunsTable runs={list.slice(0, 20)} showSkill />
              )}
            </section>
          </>
        )}
      </PageBody>
    </>
  );
}
