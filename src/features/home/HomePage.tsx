import { useNavigate } from "react-router-dom";
import { ArrowRight, Compass, Crosshair, FlaskConical, Layers, Monitor, Plus, ScanSearch, Sparkles, TerminalSquare } from "lucide-react";
import { useDashboard } from "@/lib/api/hooks";
import { useUi } from "@/lib/store";
import { cn, formatPercent, pluralize, relativeTime } from "@/lib/utils";
import { Badge, Button, Card, CardContent, CardHeader, CardTitle, EmptyState, SectionTitle, Skeleton, Stat } from "@/components/ui";
import { BudgetMeter, ErrorNote, KindIcon, Page, PageBody, PageHeader, VerdictBadge } from "@/components/app/common";
import { ItemCard } from "@/features/library/ItemCard";
import { RUN_STATUS_META } from "@/features/evals/shared";

export function HomePage() {
  const { data, isLoading, error } = useDashboard();
  const navigate = useNavigate();
  const openItem = useUi((s) => s.openItem);

  if (isLoading || !data) {
    return (
      <Page>
        <PageHeader title="Home" description="Loading your hive…" />
        <PageBody>
          <div className="grid grid-cols-5 gap-3">
            {Array.from({ length: 5 }).map((_, i) => (
              <Skeleton key={i} className="h-20" />
            ))}
          </div>
          {error ? <ErrorNote error={error} className="mt-4" /> : null}
        </PageBody>
      </Page>
    );
  }

  const { stats, targets, recentRuns, recentItems, loadouts, providers, detectedClis } = data;
  const empty = stats.total === 0;
  const conflictsTotal = targets.reduce((n, t) => n + t.conflicts.conflicts.length, 0);
  const highConflicts = targets.reduce((n, t) => n + t.conflicts.conflicts.filter((c) => c.severity === "high").length, 0);

  return (
    <Page>
      <PageHeader
        title="Home"
        description={empty ? "Start by collecting the skills already on this machine." : `${pluralize(stats.total, "item")} in the library, ${pluralize(targets.length, "agent")} detected.`}
        actions={
          <>
            <Button variant="secondary" onClick={() => navigate("/discover?tab=local&scan=1")}>
              <ScanSearch /> Scan this machine
            </Button>
            <Button variant="primary" onClick={() => navigate("/discover?tab=online")}>
              <Plus /> Add a source
            </Button>
          </>
        }
      />
      <PageBody className="flex flex-col gap-6">
        {empty ? (
          <EmptyState
            icon={Sparkles}
            title="Welcome to Skill Hive"
            description="One library for every skill, subagent and command you use. Collect what is already installed, add curated collections from GitHub, deploy conflict-free sets to each agent, and measure whether a skill actually helps."
            action={
              <>
                <Button variant="primary" onClick={() => navigate("/discover?tab=local&scan=1")}>
                  <ScanSearch /> Scan this machine
                </Button>
                <Button onClick={() => navigate("/discover?tab=catalog")}>
                  <Compass /> Browse the catalog
                </Button>
              </>
            }
          />
        ) : null}

        <div className="grid grid-cols-5 gap-3">
          <Stat label="Skills" value={stats.skills} hint={`${pluralize(stats.agents, "agent")} · ${pluralize(stats.commands, "command")}`} />
          <Stat label="Deployed" value={stats.deployed} hint={`across ${pluralize(targets.length, "agent")}`} tone="accent" />
          <Stat label="Measured" value={stats.tested} hint={stats.tested ? "with comparative evals" : "run an eval to start"} />
          <Stat label="Conflicts" value={conflictsTotal} hint={highConflicts ? `${highConflicts} high severity` : "across installed sets"} tone={highConflicts ? "danger" : conflictsTotal ? "warn" : "ok"} />
          <Stat label="Validation issues" value={stats.withErrors} hint={stats.withErrors ? "items with errors" : "all items valid"} tone={stats.withErrors ? "danger" : undefined} />
        </div>

        <div className="grid gap-6 xl:grid-cols-[1.2fr_1fr]">
          <section className="flex flex-col gap-3">
            <SectionTitle
              title="Agents on this machine"
              description="What each agent currently loads, and how much of its context budget that costs."
              action={
                <Button variant="ghost" size="sm" onClick={() => navigate("/agents")}>
                  All agents <ArrowRight />
                </Button>
              }
            />
            {targets.length === 0 ? (
              <EmptyState compact icon={Monitor} title="No agent tools detected" description="Install Claude Code, Codex, Cursor or another supported agent, or add a custom target in Settings." />
            ) : (
              <div className="grid gap-3 md:grid-cols-2">
                {targets.map((t) => (
                  <Card key={t.spec.id} interactive onClick={() => navigate(`/agents/${t.spec.id}`)}>
                    <CardHeader>
                      <div>
                        <CardTitle>{t.spec.name}</CardTitle>
                        <p className="text-xs text-muted">{pluralize(t.assets.length, "asset")} installed</p>
                      </div>
                      {t.conflicts.conflicts.length ? (
                        <Badge tone={t.conflicts.conflicts.some((c) => c.severity === "high") ? "danger" : "warn"}>{pluralize(t.conflicts.conflicts.length, "conflict")}</Badge>
                      ) : (
                        <Badge tone="ok">Clean</Badge>
                      )}
                    </CardHeader>
                    <CardContent>
                      <BudgetMeter chars={t.contextChars} budgetChars={t.spec.descriptionBudgetChars} tokens={t.contextTokens} />
                      <div className="mt-2.5 flex flex-wrap gap-1">
                        {t.assets.slice(0, 6).map((a) => (
                          <span key={a.path} className={cn("inline-flex items-center gap-1 rounded-md bg-sunken px-1.5 py-0.5 text-[11px]", a.state === "unmanaged" && "text-muted")}>
                            <KindIcon kind={a.kind} className="size-3" /> {a.name}
                          </span>
                        ))}
                        {t.assets.length > 6 ? <span className="text-[11px] text-subtle">+{t.assets.length - 6} more</span> : null}
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            )}
          </section>

          <section className="flex flex-col gap-3">
            <SectionTitle
              title="Recent evaluations"
              description="With the skill vs without it."
              action={
                <Button variant="ghost" size="sm" onClick={() => navigate("/evals")}>
                  All runs <ArrowRight />
                </Button>
              }
            />
            {recentRuns.length === 0 ? (
              <EmptyState compact icon={FlaskConical} title="No evaluations yet" description="Pick a skill and run a comparative evaluation to see if it helps." action={<Button size="sm" onClick={() => navigate("/evals")}>Open evals</Button>} />
            ) : (
              <div className="surface divide-y overflow-hidden">
                {recentRuns.slice(0, 6).map((r) => {
                  const w = r.summary?.arms.find((a) => a.label === "with_skill");
                  const b = r.summary?.arms.find((a) => a.label === r.summary?.baseline);
                  return (
                    <button key={r.id} type="button" onClick={() => navigate(`/runs/${r.id}`)} className="flex w-full items-center gap-3 px-3 py-2 text-left text-xs hover:bg-sunken/70">
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-2 text-[13px] font-medium">
                          {r.skillName}
                          <Badge tone={RUN_STATUS_META[r.status].tone}>{RUN_STATUS_META[r.status].label}</Badge>
                        </div>
                        <div className="text-subtle">{relativeTime(r.finishedAt ?? r.createdAt)} · {r.config.repeats}× repeats · {r.config.arms.length} arms</div>
                      </div>
                      {r.summary ? (
                        <div className="text-right">
                          <VerdictBadge verdict={r.summary.verdict} />
                          {w && b ? <div className="mt-0.5 num text-subtle">{formatPercent(w.passRate.mean)} vs {formatPercent(b.passRate.mean)}</div> : null}
                        </div>
                      ) : r.status === "running" || r.status === "grading" ? (
                        <div className="num text-subtle">{r.progress.done}/{r.progress.total}</div>
                      ) : null}
                    </button>
                  );
                })}
              </div>
            )}
          </section>
        </div>

        <section className="flex flex-col gap-3">
          <SectionTitle
            title="Recently updated"
            action={
              <Button variant="ghost" size="sm" onClick={() => navigate("/library")}>
                Open library <ArrowRight />
              </Button>
            }
          />
          {recentItems.length ? (
            <div className="grid gap-3 [grid-template-columns:repeat(auto-fill,minmax(280px,1fr))]">
              {recentItems.slice(0, 6).map((item) => (
                <ItemCard key={item.id} item={item} onOpen={() => openItem(item.id)} dense />
              ))}
            </div>
          ) : null}
        </section>

        <div className="grid gap-6 lg:grid-cols-3">
          <QuickLink icon={Crosshair} title="Find skills for a job" description="Describe the task; get a ranked, conflict-free set within the agent's budget." onClick={() => navigate("/find")} />
          <QuickLink icon={Layers} title={loadouts.length ? `${pluralize(loadouts.length, "loadout")}` : "Create a loadout"} description={loadouts.length ? loadouts.map((l) => l.name).join(" · ") : "Curate sets of skills and apply them to an agent as one unit."} onClick={() => navigate("/loadouts")} />
          <QuickLink
            icon={TerminalSquare}
            title={providers.length ? `${pluralize(providers.length, "provider")} configured` : "Configure a provider"}
            description={detectedClis.length ? `Detected: ${detectedClis.map((c) => `${c.command} ${c.version ?? ""}`.trim()).join(", ")}` : "Needed to run evaluations and re-rank recommendations."}
            onClick={() => navigate("/settings/providers")}
          />
        </div>
      </PageBody>
    </Page>
  );
}

function QuickLink({ icon: Icon, title, description, onClick }: { icon: typeof Crosshair; title: string; description: string; onClick: () => void }) {
  return (
    <button type="button" onClick={onClick} className="surface flex items-start gap-3 p-4 text-left transition-[box-shadow,border-color] hover:border-line-strong hover:shadow-md">
      <span className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-accent-soft text-accent-strong dark:text-accent">
        <Icon className="size-4" />
      </span>
      <span className="min-w-0">
        <span className="block text-[13px] font-semibold">{title}</span>
        <span className="mt-0.5 block text-xs text-muted">{description}</span>
      </span>
      <ArrowRight className="ml-auto size-4 shrink-0 text-subtle" />
    </button>
  );
}
