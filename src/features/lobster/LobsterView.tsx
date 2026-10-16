import { useState } from "react";
import { listScanRows } from "@/lib/ipc/actions";
import { t } from "@/lib/i18n";
import { Button } from "@/components/ui/button";
import { Screen } from "@/components/chrome";

export function LobsterView({ agentKey }: { agentKey?: string }) {
  const [count, setCount] = useState<number | null>(null);
  return (
    <Screen testId="lobster" title={t("lobster.title")} lede={agentKey ? agentKey : t("lobster.all")}>
      <Button type="button" variant="outline" size="sm" onClick={() => void listScanRows().then((rows) => setCount(rows.length)).catch(() => setCount(0))}>
        {t("dashboard.scan")}
      </Button>
      {count !== null ? <p role="status">{count}</p> : null}
    </Screen>
  );
}
