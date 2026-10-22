import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { DeltaBar, VerdictChip } from "@/components/app/VerdictChip";
import { EmptyState, ErrorState, Panel, Screen, StatePanel } from "@/components/chrome";
import { scoreDelta } from "@/lib/verdict";
import { EffectivenessSkeleton } from "@/components/skeletons";
import { t } from "@/lib/i18n";
import { demoEnabled } from "@/lib/demo";
import { previewPhase, useOnline, type ViewPhase } from "@/lib/view-state";
import { SAMPLE_REPORT, shortAxis, type EffectivenessReport } from "@/features/effectiveness/model";
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
  const reportRef = useRef(report);
  reportRef.current = report;
  const [phase, setPhase] = useState<ViewPhase>(() => previewPhase() ?? (demoEnabled() ? "ready" : "loading"));
  const [refreshing, setRefreshing] = useState(false);
  const [open, setOpen] = useState(false);
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    if (forced) {
      setPhase(forced);
      setRefreshing(false);
      setReport(null);
      return;
    }
    if (demoEnabled()) {
      setReport(SAMPLE_REPORT);
      setPhase("ready");
      setRefreshing(false);
      return;
    }
    if (!online) {
      setPhase("offline");
      setRefreshing(false);
      return;
    }
    let cancel = false;
    if (reportRef.current) setRefreshing(true);
    else setPhase("loading");
    void invoke<string>("get_effectiveness_report")
      .then((raw) => {
        if (cancel) return;
        setRefreshing(false);
        if (!raw) {
          if (!reportRef.current) setReport(null);
          setPhase(reportRef.current ? "ready" : "empty");
          return;
        }
        try {
          const parsed = JSON.parse(raw) as EffectivenessReport;
          if (!parsed.grade) {
            if (!reportRef.current) setReport(null);
            setPhase(reportRef.current ? "ready" : "empty");
            return;
          }
          setReport(parsed);
          setPhase("ready");
        } catch {
          if (!reportRef.current) setPhase("error");
          else setPhase("ready");
        }
      })
      .catch(() => {
        if (cancel) return;
        setRefreshing(false);
        if (!reportRef.current) setPhase("error");
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

  if (!report) {
    return (
      <Screen
        testId="effectiveness"
        className="min-h-[calc(100dvh-8.5rem)]"
        title={t("effect.title")}
        actions={phase === "loading" ? <Button type="button" size="sm" disabled aria-busy>{t("effect.runAgain")}</Button> : undefined}
      >
        {phase === "loading" ? <EffectivenessSkeleton /> : phase === "error" ? <ErrorState title={t("effect.error")} detail={t("shell.renderError")} onRetry={() => setAttempt((value) => value + 1)} /> : phase === "ready" || phase === "empty" ? <EmptyState title={phaseCopy("empty")} action={<Button type="button" size="sm" onClick={() => setAttempt((value) => value + 1)}>{t("effect.runAgain")}</Button>} /> : <StatePanel tone={phase} title={phaseCopy(phase)} action={retry} />}
      </Screen>
    );
  }

  const code = verdictKey(report.verdict);
  const scores = scoreDelta(report.axes);
  return (
    <Screen
      testId="effectiveness"
      className="min-h-[calc(100dvh-8.5rem)]"
      title={t("effect.title")}
      actions={
        <>
          <Button type="button" size="sm" aria-busy={refreshing} disabled={refreshing} onClick={() => setAttempt((value) => value + 1)}>{t("effect.runAgain")}</Button>
          <Button type="button" variant="outline" size="sm" aria-busy={refreshing} disabled={refreshing} onClick={() => { if (!refreshing) setOpen((value) => !value); }}>
            {open ? t("effect.hide") : t("effect.show")}
          </Button>
        </>
      }
    >
      <div className="rounded-lg border border-border bg-card p-4 shadow-sm" data-testid="verdict-hero">
        <VerdictChip delta={scores.delta} large />
        <p className="mt-2 text-sm">{t("effect.deltaLine", { delta: String(Math.round(scores.delta ?? 0)) })}</p>
        <div className="mt-3 max-w-xl"><DeltaBar delta={scores.delta} /></div>
        {scores.withScore !== null && scores.withoutScore !== null ? (
          <p className="mt-2 text-xs text-muted-foreground">{t("verdict.numbers", { with: String(Math.round(scores.withScore)), without: String(Math.round(scores.withoutScore)) })} · {report.grade} ({report.grade_score})</p>
        ) : null}
      </div>
      <Panel title={t("effect.goingOn")}>
        <p className="text-sm" data-testid="effect-verdict">{verdictText(code)}</p>
        <p className="mt-1 text-sm text-muted-foreground" data-testid="effect-action">{actionText(code)}</p>
      </Panel>
      <Panel title={t("effect.grade")}>
        <div className="flex flex-wrap items-center gap-6">
          <GradeMark letter={report.grade} score={report.grade_score} hero testId="effect-grade" />
          <div className="min-w-0 max-w-prose flex-1">
            <p className="text-sm font-medium">{verdictText(code)}</p>
            <p className="mt-1 text-sm text-muted-foreground">{actionText(code)}</p>
          </div>
        </div>
      </Panel>
      <Panel title={t("effect.radar")} className="min-w-0">
        <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
          {open ? <p className="text-sm text-muted-foreground">{t("effect.series.skill")}</p> : <p className="text-sm text-muted-foreground">{t("effect.caption.radar", { axis: shortAxis(report.weakest_axis) })}</p>}
          <Button type="button" size="sm" variant="secondary" onClick={() => document.getElementById("effect-radar-table")?.focus()}>
            {t("effect.showTable")}
          </Button>
        </div>
        <AxisTable report={report} showOverlay={open} />
      </Panel>
      <details className="rounded-lg border border-border bg-card px-4 py-3">
        <summary className="cursor-pointer text-sm font-medium">{t("effect.advanced")}</summary>
        <div className="mt-3 space-y-2 text-sm text-muted-foreground">
          <p>{t("effect.explain.overlay")}</p>
          <p>{t("effect.explain.board")}</p>
          <p>{t("effect.explain.map")}</p>
        </div>
      </details>
      {open ? (
        <>
          <div className="grid min-w-0 items-start gap-3 xl:grid-cols-[minmax(0,1.45fr)_minmax(18rem,0.7fr)]">
            <Panel title={t("effect.radar")} className="min-w-0">
              <p className="text-sm text-muted-foreground">{t("effect.caption.radar", { axis: shortAxis(report.weakest_axis) })}</p>
              <div className="mt-3">
                <RadarFigure report={report} />
              </div>
            </Panel>
            <div className="flex min-w-0 flex-col gap-3">
              <Panel>
                <GradeMark letter={report.grade} score={report.grade_score} testId="effect-grade" />
                <p className="mt-3 text-sm">{verdictText(code)}</p>
                <p className="mt-1 text-sm text-muted-foreground">{actionText(code)}</p>
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
      ) : null}
    </Screen>
  );
}
