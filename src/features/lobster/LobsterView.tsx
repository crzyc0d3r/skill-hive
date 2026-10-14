import { useState } from "react";
import { listScanRows } from "@/lib/ipc/actions";
import { t } from "@/lib/i18n";

export function LobsterView({ agentKey }: { agentKey?: string }) {
  const [count, setCount] = useState<number | null>(null);
  return (
    <section data-testid="lobster">
      <h1 className="font-mono text-[22px] font-semibold">{t("lobster.title")}</h1>
      <p className="mt-2 text-sm">{agentKey ? agentKey : t("lobster.all")}</p>
      <button type="button" className="mt-3 rounded-md border border-border px-2 py-1 text-sm" onClick={() => void listScanRows().then((rows) => setCount(rows.length)).catch(() => setCount(0))}>
        {t("dashboard.scan")}
      </button>
      {count !== null ? <p role="status">{count}</p> : null}
    </section>
  );
}
