import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { t } from "../../lib/i18n";
import { recordVerdict, useGenerate } from "@/features/generate/store";

export function CompareView() {
  const { wizard } = useGenerate();
  const revealed = wizard.verdict.length > 0;
  return (
    <section data-testid="compare" className="flex w-full flex-col gap-4">
      <h1 className="font-mono text-[22px] font-semibold tracking-tight">{t("compare.title")}</h1>
      <div className="grid gap-3 md:grid-cols-2">
        <Card className="gap-2 rounded-lg border-border px-4 py-4 shadow-none">
          <h2 className="text-sm font-medium">{revealed ? "security-reviewer v2" : t("compare.outputA")}</h2>
          <p className="text-sm text-muted-foreground">Findings list the seeded vulnerability.</p>
        </Card>
        <Card className="gap-2 rounded-lg border-border px-4 py-4 shadow-none">
          <h2 className="text-sm font-medium">{revealed ? "security-reviewer v1" : t("compare.outputB")}</h2>
          <p className="text-sm text-muted-foreground">No findings.</p>
        </Card>
      </div>
      <Button type="button" variant="outline" size="sm" onClick={() => recordVerdict("Output A")}>{t("compare.verdict")}</Button>
      {revealed ? <p data-testid="compare-unblind">{t("compare.unblind")}: {wizard.verdict}</p> : null}
    </section>
  );
}
