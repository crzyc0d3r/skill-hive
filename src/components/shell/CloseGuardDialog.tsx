import { useState } from "react";
import { t } from "@/lib/i18n";

export function CloseGuardDialog({
  onRemember,
  onClose,
}: {
  onRemember: (choice: "minimize" | "quit") => void;
  onClose: () => void;
}) {
  const [remember, setRemember] = useState(false);
  return (
    <div role="dialog" aria-label={t("close.dialog")} className="fixed inset-0 z-[70] flex items-center justify-center bg-[rgb(15_23_42/0.4)]">
      <div className="rounded-lg border border-border bg-popover p-4 text-sm">
        <h2 className="font-medium">{t("close.dialog")}</h2>
        <label className="mt-2 block">
          <input type="checkbox" checked={remember} onChange={(event) => setRemember(event.target.checked)} /> {t("close.remember")}
        </label>
        <div className="mt-3 flex gap-2">
          <button type="button" className="rounded-md border border-border px-2 py-1" onClick={() => { if (remember) onRemember("minimize"); onClose(); }}>{t("settings.close.minimize")}</button>
          <button type="button" className="rounded-md border border-border px-2 py-1" onClick={() => { if (remember) onRemember("quit"); onClose(); }}>{t("settings.close.quit")}</button>
        </div>
      </div>
    </div>
  );
}
