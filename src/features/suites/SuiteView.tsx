import { Button, buttonVariants } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { useState } from "react";
import { Link } from "react-router-dom";
import { t } from "../../lib/i18n";
import { applyOptimized, optimizeDescription, useGenerate } from "@/features/generate/store";
import { runEffectivenessSuite } from "@/lib/ipc/actions";

export function SuiteView() {
  const { wizard } = useGenerate();
  const [endpoint, setEndpoint] = useState("http://127.0.0.1:9");
  const [goal, setGoal] = useState("classify the dock sample");
  const [measured, setMeasured] = useState(false);
  return (
    <section data-testid="suite" className="flex w-full min-w-0 flex-col gap-4">
      <h1 className="font-mono text-[22px] font-semibold tracking-tight">{t("suite.title")}</h1>
      <Card className="max-w-xl gap-3 rounded-lg border-border px-4 py-4 shadow-none">
        <p className="text-sm">{t("suite.withSkill")}</p>
        <p className="text-sm text-muted-foreground">{t("suite.withoutSkill")}</p>
        <form
          className="flex flex-col gap-2"
          onSubmit={(event) => {
            event.preventDefault();
            void runEffectivenessSuite(endpoint, "", goal).then((raw) => {
              if (typeof raw === "string" && raw.includes("with_skill")) setMeasured(true);
            });
          }}
        >
          <label className="text-sm" htmlFor="suite-endpoint">
            {t("suite.endpoint")}
            <Input id="suite-endpoint" className="mt-1 h-8" value={endpoint} onChange={(event) => setEndpoint(event.target.value)} />
          </label>
          <label className="text-sm" htmlFor="suite-goal">
            {t("suite.goal")}
            <Input id="suite-goal" className="mt-1 h-8" value={goal} onChange={(event) => setGoal(event.target.value)} />
          </label>
          <div className="flex flex-wrap items-center gap-2">
            <Button type="submit" size="sm">{t("suite.measure")}</Button>
          </div>
        </form>
        {measured ? <p data-testid="suite-measured">{t("suite.measured")}</p> : null}
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
