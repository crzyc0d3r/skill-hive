import { Button } from "@/components/ui/button";
import { useState } from "react";
import { gitBackupStatus } from "@/lib/ipc/actions";
import { t } from "@/lib/i18n";

export function GitStatusButton() {
  const [status, setStatus] = useState<string | null>(null);
  return (
    <span>
      <Button type="button" variant="ghost" size="sm" className="text-muted-foreground" onClick={() => void gitBackupStatus().then((row) => setStatus(row.clean ? row.branch || "clean" : "dirty")).catch((err: Error) => setStatus(err.message))}
      >
        {t("library.gitStatus")}
      </Button>
      {status ? <span role="status"> {status}</span> : null}
    </span>
  );
}
