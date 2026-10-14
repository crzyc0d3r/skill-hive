import { useState } from "react";
import { gitBackupStatus } from "@/lib/ipc/actions";
import { t } from "@/lib/i18n";

export function GitStatusButton() {
  const [status, setStatus] = useState<string | null>(null);
  return (
    <span>
      <button
        type="button"
        className="rounded-md border border-border px-2 py-1 text-sm"
        onClick={() => void gitBackupStatus().then((row) => setStatus(row.clean ? row.branch || "clean" : "dirty")).catch((err: Error) => setStatus(err.message))}
      >
        {t("library.gitStatus")}
      </button>
      {status ? <span role="status"> {status}</span> : null}
    </span>
  );
}
