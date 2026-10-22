import { useContext, useState } from "react";
import { Link } from "react-router-dom";
import { ActivityIcon, BotIcon, FlaskConicalIcon, LibraryIcon } from "lucide-react";
import type { SkillCard } from "@/features/library/model";
import { t } from "@/lib/i18n";
import { RetryContext, useViewPhase } from "@/lib/view-state";
import { useGenerate } from "@/features/generate/store";
import { installManageSkills } from "@/lib/ipc/actions";
import { AgentBadge } from "@/components/app/AgentBadge";
import { SourceBadge } from "@/components/app/SourceBadge";
import { ErrorState, Panel, PrimaryLink, QuietLink, Screen, Stat } from "@/components/chrome";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { DashboardSkeleton } from "@/components/skeletons";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";

export function DashboardView({ skills }: { skills: SkillCard[] }) {
  const phase = useViewPhase();
  const retry = useContext(RetryContext);
  const { seed } = useGenerate();
  const [setupNote, setSetupNote] = useState<string | null>(null);
  const synced = skills.filter((skill) => skill.agents.length > 0).length;
  const connected = new Set(skills.flatMap((skill) => skill.agents)).size;
  const coverage = skills.length === 0 ? 0 : Math.round((synced / skills.length) * 100);
  const recent = [...skills].sort((left, right) => (right.updatedAt ?? 0) - (left.updatedAt ?? 0));
  const installed = skills.some((skill) => skill.name === "manage-skills");
  const uninstalled = skills.filter((skill) => skill.agents.length === 0).length;
  const regressions = seed.regressions.length;
  return (
    <Screen
      testId="dashboard"
      className="min-h-[calc(100dvh-8.5rem)]"
      title={t("dashboard.greeting")}
      lede={phase === "empty" || skills.length === 0 ? t("dashboard.empty") : t("dashboard.lede", { skills: String(skills.length), agents: String(connected) })}
      actions={
        <>
          <PrimaryLink to="/install?tab=local">{t("dashboard.quick.install")}</PrimaryLink>
          <Link to="/generate/skill" className="sr-only">{t("dashboard.generate")}</Link>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button type="button" variant="outline" size="sm">{t("shell.more")}</Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem asChild><Link to="/install?tab=scan">{t("dashboard.scan")}</Link></DropdownMenuItem>
              <DropdownMenuItem asChild><Link to="/my-skills">{t("dashboard.quick.library")}</Link></DropdownMenuItem>
              <DropdownMenuItem asChild><Link to="/generate/skill">{t("dashboard.generate")}</Link></DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </>
      }
    >
      {phase === "loading" ? <DashboardSkeleton /> : null}
      {phase === "error" && skills.length === 0 ? <ErrorState title={t("shell.error")} detail={t("shell.renderError")} onRetry={retry} /> : null}
      {phase === "empty" || (phase === "ready" && skills.length === 0) ? (
        <div className="rounded-lg border border-dashed border-border bg-card px-4 py-6">
          <h2 className="font-sans text-title font-semibold">{t("dashboard.onboard.title")}</h2>
          <ol className="mt-4 grid gap-3 sm:grid-cols-3">
            <li className="rounded-md border border-border p-3 text-sm"><span className="text-muted-foreground">1</span> <Link to="/install?tab=scan">{t("dashboard.scan")}</Link></li>
            <li className="rounded-md border border-border p-3 text-sm"><span className="text-muted-foreground">2</span> {t("dashboard.onboard.install")}</li>
            <li className="rounded-md border border-border p-3 text-sm"><span className="text-muted-foreground">3</span> {t("dashboard.onboard.test")}</li>
          </ol>
        </div>
      ) : null}
      {(phase === "ready" || phase === "error") && skills.length > 0 ? <>
      {uninstalled > 0 || regressions > 0 ? (
        <div className="space-y-2" data-testid="attention-strip">
          {uninstalled > 0 ? (
            <Alert className="border-warning/40 bg-warning-subtle text-warning-subtle-foreground">
              <AlertTitle>{uninstalled === 1 ? t("dashboard.attention.one") : t("dashboard.attention.uninstalled", { count: String(uninstalled) })}</AlertTitle>
              <AlertDescription>
                <QuietLink to="/my-skills">{t("dashboard.attention.review")}</QuietLink>
              </AlertDescription>
            </Alert>
          ) : null}
          {regressions > 0 ? (
            <Alert className="border-warning/40 bg-warning-subtle text-warning-subtle-foreground" data-testid="regressions">
              <AlertTitle>{t("dashboard.attention.regressions", { count: String(regressions) })}</AlertTitle>
              <AlertDescription data-testid="regression-row">
                {seed.regressions.map((row) => <span key={`${row.skillId}-${row.rule}`}>{row.rule} {row.message} </span>)}
                <QuietLink to="/effectiveness">{t("dashboard.attention.review")}</QuietLink>
              </AlertDescription>
            </Alert>
          ) : null}
        </div>
      ) : null}
      <div className="grid grid-cols-2 gap-3 min-[1100px]:grid-cols-4">
        <Stat label={t("dashboard.stat.skills")} value={skills.length} icon={<LibraryIcon className="size-4" aria-hidden="true" />} />
        <Stat label={t("dashboard.stat.coverage")} value={`${coverage}%`} hint={<><span>{t("dashboard.coverage.meaning", { synced: String(synced), total: String(skills.length) })}</span><Progress value={coverage} className="mt-2" /></>} icon={<ActivityIcon className="size-4" aria-hidden="true" />} />
        <Stat label={t("dashboard.stat.agents")} value={connected} icon={<BotIcon className="size-4" aria-hidden="true" />} />
        <Stat label={t("dashboard.stat.tested")} value={<span data-testid="tested-count">{seed.tested.length}</span>} icon={<FlaskConicalIcon className="size-4" aria-hidden="true" />} />
      </div>
      <div className="grid min-h-0 flex-1 gap-3 min-[1440px]:grid-cols-[minmax(0,1.45fr)_minmax(16rem,1fr)]">
        <Panel title={t("dashboard.recent")} className="h-full">
          <ul data-testid="recent-activity" className="divide-y divide-border">
            {recent.slice(0, 6).map((skill) => (
              <li key={skill.id}>
                <Link to={`/my-skills?skill=${skill.id}`} className="flex items-center gap-3 py-2.5 hover:text-foreground">
                  <span className="flex size-8 shrink-0 items-center justify-center rounded-md bg-muted text-sm font-semibold uppercase">
                    {skill.name.slice(0, 1)}
                  </span>
                  <span className="min-w-0 flex-1">
                    <span className="flex items-center gap-2">
                      <span className="truncate text-sm font-medium">{skill.name}</span>
                      <SourceBadge source={skill.source} />
                    </span>
                    <span className="block truncate text-xs text-muted-foreground">{skill.description}</span>
                  </span>
                  <span className="hidden flex-wrap justify-end gap-1 sm:flex">
                    {skill.agents.slice(0, 3).map((agent) => <AgentBadge key={agent} agent={agent} />)}
                  </span>
                </Link>
              </li>
            ))}
          </ul>
        </Panel>
        <div className="flex h-full flex-col gap-3">
          <Panel title={t("dashboard.setup")} testId="manage-skills">
            <p className="text-sm text-muted-foreground">{t("dashboard.setup.body")}</p>
            <Button
              type="button"
              size="sm"
              variant="outline"
              className="mt-3"
              disabled={installed}
              onClick={() => {
                void installManageSkills().then((row) => setSetupNote(row?.name ?? t("dashboard.setup.install")));
              }}
            >
              {installed ? t("library.checked") : t("dashboard.setup.install")}
            </Button>
            {setupNote ? <p role="status" className="mt-2 text-sm">{setupNote}</p> : null}
          </Panel>
          {phase === "error" ? <ErrorState title={t("shell.error")} detail={t("shell.renderError")} onRetry={retry} /> : <Panel title={t("dashboard.health")} testId="test-health" className="flex-1">
            <div className="flex h-3 overflow-hidden rounded-full bg-muted" aria-hidden="true">
              <span className="h-full bg-[var(--verdict-helps-lot)]" style={{ width: `${skills.length ? (Math.max(synced, 1) / skills.length) * 40 : 0}%` }} />
              <span className="h-full bg-[var(--verdict-neutral)]" style={{ width: "35%" }} />
              <span className="h-full bg-[var(--verdict-worse)]" style={{ width: `${regressions ? 25 : 8}%` }} />
            </div>
            <ul className="mt-3 space-y-1 text-sm">
              <li>{t("dashboard.health.helps")}</li>
              <li>{t("dashboard.health.neutral")}</li>
              <li>{t("dashboard.health.worse")} · {regressions}</li>
            </ul>
          </Panel>}
        </div>
      </div>
      </> : null}
    </Screen>
  );
}
