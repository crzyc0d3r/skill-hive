import { Link } from "react-router-dom";
import { ActivityIcon, AlertTriangleIcon, BotIcon, FlaskConicalIcon, LibraryIcon, SparklesIcon } from "lucide-react";
import type { SkillCard } from "@/features/library/model";
import { t } from "@/lib/i18n";
import { useGenerate } from "@/features/generate/store";
import { Panel, PrimaryLink, QuietLink, Screen, Stat } from "@/components/chrome";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";

export function DashboardView({ skills }: { skills: SkillCard[] }) {
  const { seed } = useGenerate();
  const enabled = skills.filter((skill) => skill.enabled).length;
  const synced = skills.filter((skill) => skill.agents.length > 0).length;
  const coverage = skills.length === 0 ? 0 : Math.round((synced / skills.length) * 100);
  return (
    <Screen
      testId="dashboard"
      className="min-h-[calc(100dvh-8.5rem)]"
      title={t("dashboard.greeting")}
      lede={skills.length === 0 ? t("dashboard.empty") : `${skills.length} skills`}
      actions={
        <>
          <PrimaryLink to="/install?tab=local">{t("dashboard.quick.install")}</PrimaryLink>
          <QuietLink to="/install?tab=scan">{t("dashboard.scan")}</QuietLink>
          <QuietLink to="/my-skills">{t("dashboard.quick.library")}</QuietLink>
          <QuietLink to="/generate/skill">{t("dashboard.generate")}</QuietLink>
        </>
      }
    >
      <div className="grid grid-cols-2 gap-3 xl:grid-cols-4">
        <Stat label={t("dashboard.stat.skills")} value={skills.length} icon={<LibraryIcon className="size-4" aria-hidden="true" />} />
        <Stat label={t("dashboard.stat.enabled")} value={enabled} icon={<ActivityIcon className="size-4" aria-hidden="true" />} />
        <Stat label={t("dashboard.stat.agents")} value={2} icon={<BotIcon className="size-4" aria-hidden="true" />} />
        <Stat label={t("dashboard.stat.tested")} value={<span data-testid="tested-count">{seed.tested.length}</span>} icon={<FlaskConicalIcon className="size-4" aria-hidden="true" />} />
      </div>
      <div className="grid min-h-0 flex-1 gap-3 lg:grid-cols-[minmax(0,1.45fr)_minmax(16rem,1fr)]">
        <Panel title={t("dashboard.recent")} className="h-full">
          {skills.length === 0 ? (
            <div className="flex min-h-48 flex-col items-start justify-center gap-3 rounded-md border border-dashed border-border bg-muted/30 px-4 py-8">
              <SparklesIcon className="size-5 text-muted-foreground" aria-hidden="true" />
              <p className="text-sm text-muted-foreground">{t("dashboard.recent")}</p>
            </div>
          ) : (
            <ul data-testid="recent-activity" className="divide-y divide-border">
              {skills.slice(0, 6).map((skill) => (
                <li key={skill.id}>
                  <Link to={`/my-skills?skill=${skill.id}`} className="flex items-center gap-3 py-2.5 hover:text-foreground">
                    <span className="flex size-8 shrink-0 items-center justify-center rounded-md bg-muted font-mono text-xs font-semibold uppercase">
                      {skill.name.slice(0, 1)}
                    </span>
                    <span className="min-w-0 flex-1">
                      <span className="flex items-center gap-2">
                        <span className="truncate text-sm font-medium">{skill.name}</span>
                        <Badge variant="outline" className="font-normal">{skill.source}</Badge>
                      </span>
                      <span className="block truncate text-xs text-muted-foreground">{skill.description}</span>
                    </span>
                    <span className="hidden font-mono text-[11px] text-muted-foreground sm:inline">
                      {skill.agents.length > 0 ? skill.agents.join(", ") : t("dashboard.stat.enabled")}
                    </span>
                  </Link>
                </li>
              ))}
            </ul>
          )}
        </Panel>
        <div className="flex h-full flex-col gap-3">
          <Panel title={t("dashboard.stat.enabled")}>
            <div className="flex items-end justify-between gap-3">
              <div className="font-mono text-2xl font-semibold tabular-nums">{synced}<span className="text-base text-muted-foreground">/{skills.length}</span></div>
              <span className="text-xs text-muted-foreground">{coverage}%</span>
            </div>
            <Progress value={coverage} className="mt-3" />
            <p className="mt-3 text-xs text-muted-foreground">{t("dashboard.stat.skills")} {skills.length}</p>
          </Panel>
          <Panel title={t("dashboard.regressions")} testId="regressions" className="flex-1">
            {seed.regressions.length === 0 ? (
              <p className="flex min-h-24 items-center gap-2 text-sm text-muted-foreground">
                <SparklesIcon className="size-3.5" aria-hidden="true" />
                {t("dashboard.regressions.empty")}
              </p>
            ) : (
              <ul className="space-y-2">
                {seed.regressions.map((row) => (
                  <li key={`${row.skillId}-${row.rule}`} data-testid="regression-row" className="flex items-start gap-2 text-sm">
                    <AlertTriangleIcon className="mt-0.5 size-3.5 text-warning-text" aria-hidden="true" />
                    <span>{row.rule} {row.skill}: {row.message}</span>
                  </li>
                ))}
              </ul>
            )}
          </Panel>
        </div>
      </div>
    </Screen>
  );
}
