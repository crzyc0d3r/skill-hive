import { Link } from "react-router-dom";
import { t } from "../../lib/i18n";

export function SuiteView() {
  return (
    <section data-testid="suite" className="min-w-0 space-y-3">
      <h1 className="font-mono text-xl font-semibold">{t("suite.title")}</h1>
      <p className="text-sm">{t("suite.withSkill")}</p>
      <p className="text-sm">{t("suite.withoutSkill")}</p>
      <Link to="/runs/run-fixture" className="inline-flex rounded border border-border px-2 py-1 text-sm">
        {t("suite.run")}
      </Link>
    </section>
  );
}
