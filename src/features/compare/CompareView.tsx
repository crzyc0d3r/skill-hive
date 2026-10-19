import { LockIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { t } from "../../lib/i18n";
import { recordVerdict, useGenerate } from "@/features/generate/store";
import { demoEnabled } from "@/lib/demo";
import { Panel, Screen, StatePanel } from "@/components/chrome";
import { previewPhase, useOnline } from "@/lib/view-state";

function Bars() {
  return (
    <div className="mt-4 space-y-2" aria-hidden="true">
      <div className="h-3 w-full rounded-full bg-muted" />
      <div className="h-3 w-11/12 rounded-full bg-muted" />
      <div className="h-3 w-4/5 rounded-full bg-muted" />
      <div className="h-3 w-2/3 rounded-full bg-muted" />
    </div>
  );
}

export function CompareView() {
  const { wizard } = useGenerate();
  const forced = previewPhase();
  const online = useOnline();
  const demo = demoEnabled() && !forced;
  const revealed = wizard.verdict.length > 0;
  if (forced) {
    const title = forced === "loading" ? t("compare.loading") : forced === "error" ? t("compare.error") : forced === "offline" ? t("compare.offline") : t("compare.empty");
    return (
      <Screen testId="compare" className="min-h-[calc(100dvh-8.5rem)]" title={t("compare.title")}>
        <StatePanel tone={forced} title={title} />
      </Screen>
    );
  }
  return (
    <Screen
      testId="compare"
      className="min-h-[calc(100dvh-8.5rem)]"
      title={t("compare.title")}
      lede={demo ? t("compare.sealed") : undefined}
      actions={<Button type="button" size="sm" onClick={() => recordVerdict("Output A")}>{t("compare.verdict")}</Button>}
    >
      {!online ? <StatePanel tone="offline" title={t("compare.offline")} /> : null}
      {demo ? (
        <div className="grid flex-1 gap-3 md:grid-cols-2">
          <Panel className="min-h-64">
            <div className="flex items-center gap-2">
              <span className="flex size-8 items-center justify-center rounded-md bg-muted text-muted-foreground"><LockIcon className="size-4" aria-hidden="true" /></span>
              <h2 className="font-mono text-base font-medium">{revealed ? "security-reviewer v2" : t("compare.outputA")}</h2>
            </div>
            <p className="mt-3 text-sm text-muted-foreground">{t("compare.sealed")}</p>
            {revealed ? <p className="mt-4 text-sm">{t("compare.unblind")}</p> : <Bars />}
          </Panel>
          <Panel className="min-h-64">
            <div className="flex items-center gap-2">
              <span className="flex size-8 items-center justify-center rounded-md bg-muted text-muted-foreground"><LockIcon className="size-4" aria-hidden="true" /></span>
              <h2 className="font-mono text-base font-medium">{revealed ? "security-reviewer v1" : t("compare.outputB")}</h2>
            </div>
            <p className="mt-3 text-sm text-muted-foreground">{t("compare.sealed")}</p>
            {revealed ? <p className="mt-4 text-sm">{t("compare.verdict")}</p> : <Bars />}
          </Panel>
        </div>
      ) : wizard.compareText ? (
        <Panel>
          <p data-testid="compare-live">{wizard.compareText}</p>
        </Panel>
      ) : (
        <StatePanel tone="empty" title={t("compare.empty")} />
      )}
      {revealed && demo ? (
        <p data-testid="compare-unblind" className="rounded-md border border-success/40 bg-success-subtle px-3 py-2 text-sm text-success-text">
          {t("compare.unblind")}: {wizard.verdict}
        </p>
      ) : null}
    </Screen>
  );
}
