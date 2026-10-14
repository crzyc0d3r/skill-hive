import { t } from "../../lib/i18n";
import { recordVerdict, useGenerate } from "@/features/generate/store";

export function CompareView() {
  const { wizard } = useGenerate();
  const revealed = wizard.verdict.length > 0;
  return (
    <section data-testid="compare" className="space-y-3">
      <h1 className="font-mono text-xl font-semibold">{t("compare.title")}</h1>
      <div className="grid grid-cols-2 gap-3">
        <article className="rounded-md border border-border p-3">
          <h2 className="text-sm font-medium">{revealed ? "security-reviewer v2" : t("compare.outputA")}</h2>
          <p>Findings list the seeded vulnerability.</p>
        </article>
        <article className="rounded-md border border-border p-3">
          <h2 className="text-sm font-medium">{revealed ? "security-reviewer v1" : t("compare.outputB")}</h2>
          <p>No findings.</p>
        </article>
      </div>
      <button type="button" className="rounded-md border border-border px-3 py-1 text-sm" onClick={() => recordVerdict("Output A")}>{t("compare.verdict")}</button>
      {revealed ? <p data-testid="compare-unblind">{t("compare.unblind")}: {wizard.verdict}</p> : null}
    </section>
  );
}
