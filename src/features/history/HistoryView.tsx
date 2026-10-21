import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { useEffect, useRef, useState } from "react";
import { t } from "../../lib/i18n";
import { HISTORY_ROWS, type HistoryRow } from "@/features/history/model";
import { writeHistoryBaseline } from "@/lib/ipc/actions";
import { demoEnabled } from "@/lib/demo";
import { Panel, Screen, StatePanel } from "@/components/chrome";
import { HistorySkeleton } from "@/components/skeletons";
import { Table, TableBody, TableCaption, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { previewPhase, useOnline, type ViewPhase } from "@/lib/view-state";

function phaseCopy(phase: ViewPhase): string {
  if (phase === "loading") return t("history.loading");
  if (phase === "error") return t("history.error");
  if (phase === "offline") return t("history.offline");
  return t("history.empty");
}

function readIterations(raw: string): HistoryRow[] {
  const parsed = JSON.parse(raw) as { iterations?: { version: string; parent?: string | null; expectation_pass_rate: number; grading_result: string; is_current_best: boolean }[]; history?: { iterations?: { version: string; parent?: string | null; expectation_pass_rate: number; grading_result: string; is_current_best: boolean }[] }; alerts?: { rule: string; message: string }[] };
  const iterations = parsed.iterations ?? parsed.history?.iterations ?? [];
  return iterations.map((row) => ({ version: row.version, parent: row.parent ?? "", rate: String(row.expectation_pass_rate), result: row.grading_result, best: row.is_current_best }));
}

export function HistoryView() {
  const forced = previewPhase();
  const online = useOnline();
  const [saved, setSaved] = useState(false);
  const [rows, setRows] = useState<HistoryRow[]>(() => (demoEnabled() && !previewPhase() ? HISTORY_ROWS : []));
  const rowsRef = useRef(rows);
  rowsRef.current = rows;
  const [alerts, setAlerts] = useState<string[]>([]);
  const [phase, setPhase] = useState<ViewPhase>(() => previewPhase() ?? (demoEnabled() ? "ready" : "loading"));
  const [refreshing, setRefreshing] = useState(false);
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    if (forced) {
      setPhase(forced);
      setRefreshing(false);
      setRows([]);
      return;
    }
    if (demoEnabled()) {
      setRows(HISTORY_ROWS);
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
    if (rowsRef.current.length > 0) setRefreshing(true);
    else setPhase("loading");
    void invoke<string>("get_effectiveness_report")
      .then((raw) => {
        if (cancel) return;
        setRefreshing(false);
        if (!raw) {
          setPhase(rowsRef.current.length ? "ready" : "empty");
          return;
        }
        try {
          const parsed = JSON.parse(raw) as { history?: { iterations?: { version: string; parent?: string | null; expectation_pass_rate: number; grading_result: string; is_current_best: boolean }[] }; alerts?: { rule: string; message: string }[] };
          if (parsed.history?.iterations?.length) setRows(readIterations(raw));
          if (parsed.alerts?.length) setAlerts(parsed.alerts.map((alert) => `${alert.rule} ${alert.message}`));
          setPhase(parsed.history?.iterations?.length || rowsRef.current.length ? "ready" : "empty");
        } catch {
          if (!rowsRef.current.length) setPhase("error");
          else setPhase("ready");
        }
      })
      .catch(() => {
        if (cancel) return;
        setRefreshing(false);
        if (!rowsRef.current.length) setPhase("error");
      });
    return () => {
      cancel = true;
    };
  }, [forced, online, attempt]);

  const points = rows.map((row) => Number(row.rate)).filter((value) => Number.isFinite(value));
  const spark = points.map((value, index) => {
    const x = points.length === 1 ? 160 : (index / (points.length - 1)) * 300 + 10;
    const y = 70 - ((value - 0.5) / 0.5) * 55;
    return `${x},${Math.max(8, Math.min(72, y))}`;
  }).join(" ");

  return (
    <Screen
      testId="history"
      className="min-h-[calc(100dvh-8.5rem)]"
      title={t("history.title")}
      actions={
        <Button type="button" size="sm" aria-busy={refreshing || phase === "loading"} disabled={phase === "loading"} onClick={() => { void writeHistoryBaseline("baseline").then((raw) => { setSaved(true); if (typeof raw !== "string") return; try { const next = readIterations(raw); if (next.length) { setRows(next); setPhase("ready"); } } catch { /* the command returns the history document */ } }); }}>
          {t("history.baseline")}
        </Button>
      }
    >
      {phase === "loading" && rows.length === 0 ? <HistorySkeleton /> : null}
      {phase !== "ready" && phase !== "loading" && rows.length === 0 ? (
        <StatePanel tone={phase} title={phaseCopy(phase)} action={phase === "error" || phase === "offline" ? <Button type="button" size="sm" onClick={() => setAttempt((value) => value + 1)}>{t("banner.retry")}</Button> : null} />
      ) : null}
      {saved ? <p role="status">{t("history.baseline")}</p> : null}
      {phase === "ready" && rows.length === 0 ? <p className="text-sm text-muted-foreground">{t("history.empty")}</p> : null}
      {phase === "ready" && rows.length > 0 ? <p className="text-sm text-muted-foreground">{t("history.trend")}</p> : null}
      {alerts.length ? <p data-testid="history-alerts" className="rounded-md border border-warning/40 bg-warning-subtle px-3 py-2 text-sm text-warning-text">{t("history.alerts")}: {alerts.join("; ")}</p> : null}
      {rows.length > 0 ? (
        <Panel title={t("history.trend")} className="min-w-0">
          <svg viewBox="0 0 320 80" role="img" aria-label={t("history.trend")} className="h-24 w-full">
            <line x1="10" y1="70" x2="310" y2="70" stroke="var(--color-border)" />
            <polyline points={spark} fill="none" stroke="var(--color-chart-1)" strokeWidth="2.5" />
            {spark.split(" ").map((pair) => {
              const [x, y] = pair.split(",");
              return <circle key={pair} cx={x} cy={y} r="4" fill="var(--color-chart-1)" />;
            })}
          </svg>
        </Panel>
      ) : null}
      {phase === "loading" && rows.length === 0 ? null : <div className="overflow-hidden rounded-md border border-border">
        <Table>
          <TableCaption className="sr-only">{t("history.table")}</TableCaption>
          <TableHeader>
            <TableRow>
              <TableHead>{t("history.version")}</TableHead>
              <TableHead>{t("history.rate")}</TableHead>
              <TableHead>{t("history.result")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map((row) => (
              <TableRow key={row.version} data-testid={`history-${row.version}`} className={row.best ? "bg-success-subtle" : "odd:bg-muted/30"}>
                <TableCell className="font-mono">{row.version}</TableCell>
                <TableCell className="font-mono tabular-nums">{row.rate}</TableCell>
                <TableCell>{row.best ? t("history.baseline") : row.result}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>}
    </Screen>
  );
}
