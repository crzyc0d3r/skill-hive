import { t } from "../../lib/i18n";
import { HISTORY_ROWS } from "../../lib/phase5/model";

export function HistoryView() {
  return (
    <section data-testid="history" className="space-y-3">
      <h1 className="font-mono text-xl font-semibold">{t("history.title")}</h1>
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
