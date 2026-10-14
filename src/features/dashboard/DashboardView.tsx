import { Link } from "react-router-dom";
import type { SkillCard } from "@/features/library/model";
import { t } from "@/lib/i18n";
import { useGenerate } from "@/features/generate/store";

export function DashboardView({ skills }: { skills: SkillCard[] }) {
  const { seed } = useGenerate();
  const enabled = skills.filter((skill) => skill.enabled).length;
  return (
    <section data-testid="dashboard">
      <h1 className="font-mono text-[22px] font-semibold tracking-tight">{t("dashboard.greeting")}</h1>
      <p className="mt-2 text-sm text-muted-foreground">{skills.length === 0 ? t("dashboard.empty") : `${skills.length} skills`}</p>
      <ul className="mt-4 grid grid-cols-2 gap-3">
        <li className="rounded-lg border border-border p-3"><span className="text-xs text-muted-foreground">{t("dashboard.stat.skills")}</span><div>{skills.length}</div></li>
        <li className="rounded-lg border border-border p-3"><span className="text-xs text-muted-foreground">{t("dashboard.stat.enabled")}</span><div>{enabled}</div></li>
        <li className="rounded-lg border border-border p-3"><span className="text-xs text-muted-foreground">{t("dashboard.stat.agents")}</span><div>2</div></li>
        <li className="rounded-lg border border-border p-3"><span className="text-xs text-muted-foreground">{t("dashboard.stat.tested")}</span><div data-testid="tested-count">{seed.tested.length}</div></li>
      </ul>
      <section data-testid="regressions" className="mt-4 rounded-lg border border-border p-3">
        <h2 className="text-sm font-medium">{t("dashboard.regressions")}</h2>
        {seed.regressions.length === 0 ? <p className="text-sm text-muted-foreground">{t("dashboard.regressions.empty")}</p> : (
          <ul>
            {seed.regressions.map((row) => (
              <li key={`${row.skillId}-${row.rule}`} data-testid="regression-row">{row.rule} {row.skill}: {row.message}</li>
            ))}
          </ul>
        )}
      </section>
      <div className="mt-4 flex gap-2">
        <Link to="/install?tab=scan" className="rounded-md bg-primary px-3 py-1 text-sm text-primary-foreground">{t("dashboard.scan")}</Link>
        <Link to="/install?tab=local" className="rounded-md border border-border px-3 py-1 text-sm">{t("dashboard.quick.install")}</Link>
        <Link to="/my-skills" className="rounded-md border border-border px-3 py-1 text-sm">{t("dashboard.quick.library")}</Link>
        <Link to="/generate/skill" className="rounded-md border border-border px-3 py-1 text-sm">{t("dashboard.generate")}</Link>
      </div>
      <h2 className="mt-6 text-sm font-medium">{t("dashboard.recent")}</h2>
      <ul data-testid="recent-activity">
        {skills.slice(0, 5).map((skill) => (
          <li key={skill.id}>
            <Link to={`/my-skills?skill=${skill.id}`} className="text-sm">{skill.name}</Link>
          </li>
        ))}
      </ul>
    </section>
  );
}
