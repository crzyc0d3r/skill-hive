import { Button } from "@/components/ui/button";
import { useState } from "react";
import { t } from "../../lib/i18n";
import { HISTORY_ROWS } from "@/features/history/model";
import { writeHistoryBaseline } from "@/lib/ipc/actions";
import { Screen } from "@/components/chrome";
import { Table, TableBody, TableCaption, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";

export function HistoryView() {
  const [saved, setSaved] = useState(false);
  return (
    <Screen testId="history" title={t("history.title")}>
      <Button type="button" variant="outline" size="sm" className="w-fit" onClick={() => { void writeHistoryBaseline("baseline"); setSaved(true); }}>{t("history.baseline")}</Button>
      {saved ? <p role="status">{t("history.baseline")}</p> : null}
      <Table>
        <TableCaption className="sr-only">{t("history.table")}</TableCaption>
        <TableHeader>
          <TableRow>
            <TableHead>{t("history.title")}</TableHead>
            <TableHead>{t("history.baseline")}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {HISTORY_ROWS.map((row) => (
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
