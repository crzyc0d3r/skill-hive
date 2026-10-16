import { Button } from "@/components/ui/button";
import { Link } from "react-router-dom";
import { t } from "../../lib/i18n";
import { applyOptimized, optimizeDescription, useGenerate } from "@/features/generate/store";

export function SuiteView() {
  const { wizard } = useGenerate();
  return (
    <section data-testid="suite" className="min-w-0 space-y-3">
      <h1 className="font-mono text-xl font-semibold">{t("suite.title")}</h1>
      <p className="text-sm">{t("suite.withSkill")}</p>
      <p className="text-sm">{t("suite.withoutSkill")}</p>
      <Button type="button" variant="outline" size="sm" onClick={() => optimizeDescription()}>{t("suite.optimize")}</Button>
      {wizard.optimized ? <p data-testid="optimize-preview">{t("suite.optimized")}</p> : null}
      <Button type="button" variant="outline" size="sm" disabled={!wizard.optimized || wizard.applied} onClick={() => applyOptimized()}>{t("suite.apply")}</Button>
      {wizard.applied ? <p data-testid="optimize-applied">{t("suite.apply")}</p> : null}
      <Link to="/runs/run-fixture" className="inline-flex rounded border border-border px-2 py-1 text-sm">
        {t("suite.run")}
      </Link>
    </section>
  );
}
