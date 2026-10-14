import { Link } from "react-router-dom";
import { t } from "../../lib/i18n";
import { applyOptimized, optimizeDescription, usePhase5 } from "../../lib/phase5/store";

export function SuiteView() {
  const { wizard } = usePhase5();
  return (
    <section data-testid="suite" className="min-w-0 space-y-3">
      <h1 className="font-mono text-xl font-semibold">{t("suite.title")}</h1>
      <p className="text-sm">{t("suite.withSkill")}</p>
      <p className="text-sm">{t("suite.withoutSkill")}</p>
      <button type="button" className="rounded border border-border px-2 py-1 text-sm" onClick={() => optimizeDescription()}>{t("suite.optimize")}</button>
      {wizard.optimized ? <p data-testid="optimize-preview">{t("suite.optimized")}</p> : null}
      <button type="button" className="rounded border border-border px-2 py-1 text-sm" disabled={!wizard.optimized || wizard.applied} onClick={() => applyOptimized()}>{t("suite.apply")}</button>
      {wizard.applied ? <p data-testid="optimize-applied">{t("suite.apply")}</p> : null}
      <Link to="/runs/run-fixture" className="inline-flex rounded border border-border px-2 py-1 text-sm">
        {t("suite.run")}
      </Link>
    </section>
  );
}
