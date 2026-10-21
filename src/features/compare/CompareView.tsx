import { Button } from "@/components/ui/button";
import { t } from "../../lib/i18n";
import { recordVerdict, useGenerate } from "@/features/generate/store";
import { demoEnabled } from "@/lib/demo";
import { DeltaBar, VerdictChip } from "@/components/app/VerdictChip";
import { ErrorState, Panel, Screen, StatePanel } from "@/components/chrome";
import { CompareSkeleton } from "@/components/skeletons";
import { previewPhase, useOnline, useShellPhase } from "@/lib/view-state";

export function CompareView() {
  const { wizard } = useGenerate();
  const forced = previewPhase();
  const shell = useShellPhase();
  const online = useOnline();
  const demo = demoEnabled() && !forced;
  const revealed = wizard.verdict.length > 0;
  if (forced === "error") {
    return (
      <Screen testId="compare" className="min-h-[calc(100dvh-8.5rem)]" title={t("compare.title")}>
        <ErrorState title={t("compare.error")} detail={t("compare.error")} />
      </Screen>
    );
  }
  if (forced && forced !== "loading") {
    const title = forced === "offline" ? t("compare.offline") : t("compare.empty");
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
      lede={demo ? t("compare.summary", { delta: "12" }) : undefined}
      actions={<Button type="button" size="sm" onClick={() => recordVerdict("Output A")}>{t("compare.verdict")}</Button>}
    >
      {!online ? <StatePanel tone="offline" title={t("compare.offline")} /> : null}
      {shell === "loading" ? <CompareSkeleton /> : null}
      {shell !== "loading" && demo ? (
        <div className="space-y-4">
          <div className="grid gap-3 md:grid-cols-2">
            <label className="block text-sm">
              <span className="mb-1.5 block text-muted-foreground">{t("compare.pickerA")}</span>
              <select aria-label={t("compare.outputA")} className="h-8 w-full rounded-md border border-input bg-background px-2">
                <option>{revealed ? "security-reviewer v2" : t("compare.outputA")}</option>
              </select>
            </label>
            <label className="block text-sm">
              <span className="mb-1.5 block text-muted-foreground">{t("compare.pickerB")}</span>
              <select aria-label={t("compare.outputB")} className="h-8 w-full rounded-md border border-input bg-background px-2">
                <option>{revealed ? "security-reviewer v1" : t("compare.outputB")}</option>
              </select>
            </label>
          </div>
          <p className="text-sm">{t("compare.summary", { delta: "12" })}</p>
          <div className="grid gap-3 md:grid-cols-2">
            <DeltaBar delta={12} />
            <DeltaBar delta={12} />
          </div>
          <table className="w-full text-sm">
            <thead className="text-left text-xs text-muted-foreground">
              <tr>
                <th className="py-2">{t("compare.axis")}</th>
                <th>{t("compare.pickerA")}</th>
                <th>{t("compare.pickerB")}</th>
                <th>{t("compare.difference")}</th>
              </tr>
            </thead>
            <tbody>
              <tr className="border-t border-border">
                <td className="py-2">{t("suite.withSkill")}</td>
                <td>75%</td>
                <td>63%</td>
                <td><VerdictChip delta={12} /></td>
              </tr>
              <tr className="border-t border-border text-muted-foreground">
                <td className="py-2">{t("history.rate")}</td>
                <td>80%</td>
                <td>78%</td>
                <td><VerdictChip delta={2} /></td>
              </tr>
            </tbody>
          </table>
          {revealed ? <p className="text-sm">{t("compare.unblind")}</p> : null}
        </div>
      ) : shell !== "loading" && wizard.compareText ? (
        <Panel>
          <p data-testid="compare-live">{wizard.compareText}</p>
        </Panel>
      ) : (
        shell === "loading" ? null : <StatePanel tone="empty" title={t("compare.empty")} />
      )}
      {demo ? <VerdictChip delta={12} /> : null}
      {revealed && demo ? (
        <p data-testid="compare-unblind" className="rounded-md border border-success/40 bg-success-subtle px-3 py-2 text-sm text-success-text">
          {t("compare.unblind")}: {wizard.verdict}
        </p>
      ) : null}
    </Screen>
  );
}
