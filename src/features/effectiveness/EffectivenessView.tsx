import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Panel, Screen, Stat, StatePanel } from "@/components/chrome";
import { t } from "@/lib/i18n";
import { demoEnabled } from "@/lib/demo";
import { previewPhase, useOnline, type ViewPhase } from "@/lib/view-state";
import { SAMPLE_REPORT, type EffectivenessReport } from "@/features/effectiveness/model";
import { AxisTable, BisectList, ConflictMatrix, GradeMark, InfluenceMap, Leaderboard, MapTable, RadarFigure, WarningTable } from "@/features/effectiveness/charts";

const VERDICTS = ["hijacked", "cofired", "none", "strong", "value", "install", "healthy"] as const;

function verdictKey(code: string): (typeof VERDICTS)[number] {
  return VERDICTS.find((item) => item === code) ?? "healthy";
}

function verdictText(code: string): string {
  if (code === "hijacked") return t("effect.verdict.hijacked");
  if (code === "cofired") return t("effect.verdict.cofired");
  if (code === "none") return t("effect.verdict.none");
  if (code === "strong") return t("effect.verdict.strong");
  if (code === "value") return t("effect.verdict.value");
  if (code === "install") return t("effect.verdict.install");
  return t("effect.verdict.healthy");
}

function actionText(code: string): string {
  if (code === "hijacked") return t("effect.action.hijacked");
  if (code === "cofired") return t("effect.action.cofired");
  if (code === "none") return t("effect.action.none");
  if (code === "strong") return t("effect.action.strong");
  if (code === "value") return t("effect.action.value");
  if (code === "install") return t("effect.action.install");
  return t("effect.action.healthy");
}

function phaseCopy(phase: ViewPhase): string {
  if (phase === "loading") return t("effect.loading");
  if (phase === "error") return t("effect.error");
  if (phase === "offline") return t("effect.offline");
  return t("effect.empty");
}

export function EffectivenessView() {
  const forced = previewPhase();
  const online = useOnline();
  const [report, setReport] = useState<EffectivenessReport | null>(() => (demoEnabled() && !previewPhase() ? SAMPLE_REPORT : null));
  const [phase, setPhase] = useState<ViewPhase>(() => previewPhase() ?? (demoEnabled() ? "ready" : "loading"));
  const [open, setOpen] = useState(false);
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    if (forced) {
      setPhase(forced);
      setReport(null);
      return;
    }
    if (demoEnabled()) {
      setReport(SAMPLE_REPORT);
      setPhase("ready");
      return;
    }
    if (!online) {
      setReport(null);
      setPhase("offline");
      return;
    }
    let cancel = false;
    setPhase("loading");
    void invoke<string>("get_effectiveness_report")
      .then((raw) => {
        if (cancel) return;
        if (!raw) {
          setReport(null);
          setPhase("empty");
          return;
        }
        try {
          const parsed = JSON.parse(raw) as EffectivenessReport;
          if (!parsed.grade) {
            setReport(null);
            setPhase("empty");
            return;
          }
          setReport(parsed);
          setPhase("ready");
        } catch {
          setPhase("error");
        }
      })
      .catch(() => {
        if (!cancel) setPhase("error");
      });
    return () => {
      cancel = true;
    };
  }, [forced, online, attempt]);

  const retry =
    phase === "error" || phase === "offline" ? (
      <Button type="button" size="sm" onClick={() => setAttempt((value) => value + 1)}>
        {t("banner.retry")}
      </Button>
    ) : null;

  if (phase !== "ready" || !report) {
    return (
      <Screen testId="effectiveness" className="min-h-[calc(100dvh-8.5rem)]" title={t("effect.title")}>
        <StatePanel tone={phase === "ready" ? "empty" : phase} title={phaseCopy(phase === "ready" ? "empty" : phase)} action={retry} />
      </Screen>
    );
  }

  const code = verdictKey(report.verdict);
  return (
    <Screen
      testId="effectiveness"
      className="min-h-[calc(100dvh-8.5rem)]"
      title={t("effect.title")}
      lede={open ? undefined : verdictText(code)}
      actions={
        <Button type="button" size="sm" onClick={() => setOpen((value) => !value)}>
          {open ? t("effect.hide") : t("effect.show")}
        </Button>
      }
    >
      {open ? (
        <>
          <div className="grid min-w-0 items-start gap-3 xl:grid-cols-[minmax(0,1.45fr)_minmax(18rem,0.7fr)]">
            <Panel title={t("effect.radar")} className="min-w-0">
              <p className="text-sm text-muted-foreground">{t("effect.caption.radar", { axis: report.weakest_axis })}</p>
              <div className="mt-3">
                <RadarFigure report={report} />
              </div>
            </Panel>
            <div className="flex min-w-0 flex-col gap-3">
              <Panel>
                <GradeMark letter={report.grade} score={report.grade_score} testId="effect-grade" />
                <p className="mt-3 text-sm" data-testid="effect-verdict">{verdictText(code)}</p>
                <p className="mt-1 text-sm text-muted-foreground" data-testid="effect-action">{actionText(code)}</p>
              </Panel>
              <Panel title={t("effect.map")}>
                <p className="mb-2 text-sm text-muted-foreground">{t("effect.caption.map", { arrow: report.thickest_arrow })}</p>
                <InfluenceMap report={report} />
              </Panel>
              <Panel title={t("effect.board")}>
                <p className="mb-2 text-sm text-muted-foreground">{t("effect.caption.board", { skill: report.leader_name, factor: report.leader_factor })}</p>
                <Leaderboard report={report} />
              </Panel>
            </div>
          </div>
          <Panel title={t("effect.conflicts")}>
            <p className="mb-2 text-sm text-muted-foreground">
              {t("effect.caption.conflicts", { other: report.conflicts?.[0]?.other ?? "—", owner: report.conflicts?.[0]?.owner ?? "—" })}
            </p>
            <ConflictMatrix report={report} />
          </Panel>
          <div className="grid gap-3 lg:grid-cols-2">
            <Panel title={t("effect.bisect")}>
              <p className="mb-2 text-sm text-muted-foreground">{t("effect.caption.bisect", { skills: report.interference?.bisect.join(", ") || "—" })}</p>
              <BisectList report={report} />
            </Panel>
            <Panel title={t("effect.warnings")}>
              <p className="mb-2 text-sm text-muted-foreground">{t("effect.caption.warnings", { skill: report.interference?.warnings[0]?.suspect ?? "—" })}</p>
              <WarningTable report={report} />
            </Panel>
          </div>
          <Panel title={t("effect.map")}>
            <MapTable report={report} />
          </Panel>
        </>
      ) : (
        <div className="grid gap-3 lg:grid-cols-[minmax(18rem,0.85fr)_minmax(0,1.15fr)]">
          <Panel className="h-full">
            <GradeMark letter={report.grade} score={report.grade_score} hero testId="effect-grade" />
            <p className="mt-4 text-sm" data-testid="effect-verdict">{verdictText(code)}</p>
            <p className="mt-2 text-sm text-muted-foreground" data-testid="effect-action">{actionText(code)}</p>
            {report.omitted_axes?.length ? <p className="mt-3 text-xs text-muted-foreground">{t("effect.omitted", { axes: report.omitted_axes.join(", ") })}</p> : null}
          </Panel>
          <div className="grid grid-cols-2 gap-3">
            <Stat label={t("effect.radar")} value={report.axes.length} hint={report.weakest_axis} />
            <Stat label={t("effect.board")} value={report.leader_name} hint={report.leader_factor} />
            <Stat label={t("effect.overlay")} value={report.overlay?.name ?? "—"} />
            <Stat label={t("effect.map")} value={report.influence?.edges.length ?? 0} hint={report.thickest_arrow} />
          </div>
        </div>
      )}
      <Panel title={t("effect.radar")} className="min-w-0">
        <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
          {open ? <p className="text-sm text-muted-foreground">{t("effect.series.skill")}</p> : <p className="text-sm text-muted-foreground">{t("effect.caption.radar", { axis: report.weakest_axis })}</p>}
          <Button type="button" size="sm" variant="secondary" onClick={() => document.getElementById("effect-radar-table")?.focus()}>
            {t("effect.showTable")}
          </Button>
        </div>
        <AxisTable report={report} />
      </Panel>
    </Screen>
  );
}
