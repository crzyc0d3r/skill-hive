import { Button, buttonVariants } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Link } from "react-router-dom";
import { t } from "../../lib/i18n";
import { applyOptimized, optimizeDescription, useGenerate } from "@/features/generate/store";

export function SuiteView() {
  const { wizard } = useGenerate();
  return (
    <section data-testid="suite" className="flex w-full min-w-0 flex-col gap-4">
      <h1 className="font-mono text-[22px] font-semibold tracking-tight">{t("suite.title")}</h1>
      <Card className="max-w-xl gap-3 rounded-lg border-border px-4 py-4 shadow-none">
        <p className="text-sm">{t("suite.withSkill")}</p>
        <p className="text-sm text-muted-foreground">{t("suite.withoutSkill")}</p>
        <div className="flex flex-wrap items-center gap-2">
          <Button type="button" variant="outline" size="sm" onClick={() => optimizeDescription()}>{t("suite.optimize")}</Button>
          <Button type="button" size="sm" disabled={!wizard.optimized || wizard.applied} onClick={() => applyOptimized()}>{t("suite.apply")}</Button>
          <Link to="/runs/run-fixture" className={buttonVariants({ variant: "secondary", size: "sm" })}>
            {t("suite.run")}
          </Link>
        </div>
        {wizard.optimized ? <p data-testid="optimize-preview">{wizard.optimizePath || t("suite.optimized")}</p> : null}
        {wizard.applied ? <p data-testid="optimize-applied">{t("suite.apply")}</p> : null}
      </Card>
    </section>
  );
}
