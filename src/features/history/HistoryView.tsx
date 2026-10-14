import { useState } from "react";
import { t } from "../../lib/i18n";
import { HISTORY_ROWS } from "@/features/history/model";
import { writeHistoryBaseline } from "@/lib/ipc/actions";

export function HistoryView() {
  const [saved, setSaved] = useState(false);
  return (
    <section data-testid="history" className="space-y-3">
      <h1 className="font-mono text-xl font-semibold">{t("history.title")}</h1>
      <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => { void writeHistoryBaseline("baseline"); setSaved(true); }}>{t("history.baseline")}</button>
      {saved ? <p role="status">{t("history.baseline")}</p> : null}
      <table className="w-full text-sm">
        <caption className="sr-only">{t("history.table")}</caption>
        <thead>
          <tr>
            <th className="text-left">{t("history.title")}</th>
            <th className="text-left">{t("history.baseline")}</th>
          </tr>
        </thead>
        <tbody>
          {HISTORY_ROWS.map((row) => (
            <tr key={row.version} data-testid={`history-${row.version}`}>
              <td>{row.version}</td>
              <td>{row.rate}</td>
              <td>{row.best ? t("history.baseline") : row.result}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}
