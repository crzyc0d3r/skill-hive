import { Button } from "@/components/ui/button";
import { useState } from "react";
import { t } from "../../lib/i18n";
import { HISTORY_ROWS, type HistoryRow } from "@/features/history/model";
import { writeHistoryBaseline } from "@/lib/ipc/actions";
import { demoEnabled } from "@/lib/demo";
import { Screen } from "@/components/chrome";
import { Table, TableBody, TableCaption, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";

export function HistoryView() {
  const [saved, setSaved] = useState(false);
  const [rows, setRows] = useState<HistoryRow[]>(() => (demoEnabled() ? HISTORY_ROWS : []));
  return (
    <Screen testId="history" title={t("history.title")}>
      <Button type="button" variant="outline" size="sm" className="w-fit" onClick={() => { void writeHistoryBaseline("baseline").then((raw) => { setSaved(true); if (typeof raw !== "string") return; try { const parsed = JSON.parse(raw) as { iterations?: { version: string; parent?: string | null; expectation_pass_rate: number; grading_result: string; is_current_best: boolean }[] }; if (parsed.iterations?.length) { setRows(parsed.iterations.map((row) => ({ version: row.version, parent: row.parent ?? "", rate: String(row.expectation_pass_rate), result: row.grading_result, best: row.is_current_best }))); } } catch { /* the command returns the history document */ } }); }}>{t("history.baseline")}</Button>
      {saved ? <p role="status">{t("history.baseline")}</p> : null}
      {rows.length === 0 ? <p className="text-sm text-muted-foreground">{t("history.empty")}</p> : null}
      <Table>
        <TableCaption className="sr-only">{t("history.table")}</TableCaption>
        <TableHeader>
          <TableRow>
            <TableHead>{t("history.title")}</TableHead>
            <TableHead>{t("history.baseline")}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {rows.map((row) => (
            <TableRow key={row.version} data-testid={`history-${row.version}`}>
              <TableCell className="font-mono">{row.version}</TableCell>
              <TableCell className="tabular-nums">{row.rate}</TableCell>
              <TableCell>{row.best ? t("history.baseline") : row.result}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </Screen>
  );
}
