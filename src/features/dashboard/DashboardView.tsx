import { Link } from "react-router-dom";
import { ActivityIcon, AlertTriangleIcon, SparklesIcon } from "lucide-react";
import type { SkillCard } from "@/features/library/model";
import { t } from "@/lib/i18n";
import { useGenerate } from "@/features/generate/store";
import { Panel, PrimaryLink, QuietLink, Screen, Stat } from "@/components/chrome";

export function DashboardView({ skills }: { skills: SkillCard[] }) {
  const { seed } = useGenerate();
  const enabled = skills.filter((skill) => skill.enabled).length;
  return (
    <Screen
      testId="dashboard"
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
        <Stat label={t("dashboard.stat.skills")} value={skills.length} />
        <Stat label={t("dashboard.stat.enabled")} value={enabled} />
        <Stat label={t("dashboard.stat.agents")} value={2} />
        <Stat label={t("dashboard.stat.tested")} value={<span data-testid="tested-count">{seed.tested.length}</span>} />
      </div>
      <div className="grid gap-3 lg:grid-cols-2">
        <Panel title={t("dashboard.recent")}>
          {skills.length === 0 ? (
            <p className="text-sm text-muted-foreground">{t("dashboard.recent")}</p>
          ) : (
            <ul data-testid="recent-activity" className="divide-y divide-border">
              {skills.slice(0, 5).map((skill) => (
                <li key={skill.id}>
                  <Link to={`/my-skills?skill=${skill.id}`} className="flex items-center gap-2 py-2 text-sm hover:text-foreground">
                    <ActivityIcon className="size-3.5 text-muted-foreground" aria-hidden="true" />
                    <span className="font-medium">{skill.name}</span>
                    <span className="ml-auto font-mono text-xs text-muted-foreground">{skill.source}</span>
                  </Link>
                </li>
              ))}
            </ul>
          )}
        </Panel>
        <Panel title={t("dashboard.regressions")} testId="regressions">
          {seed.regressions.length === 0 ? (
            <p className="flex items-center gap-2 text-sm text-muted-foreground">
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
    </Screen>
  );
}
