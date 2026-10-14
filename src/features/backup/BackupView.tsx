import { useState } from "react";
import { backupNow, restoreSnapshot } from "@/features/backup/model";
import { t } from "@/lib/i18n";

export function BackupView() {
  const [body, setBody] = useState("library v1");
  const [history, setHistory] = useState<{ id: string; label: string; body: string }[]>([]);
  const [safety, setSafety] = useState<string | null>(null);
  const [device, setDevice] = useState("this-machine");
  return (
    <section data-testid="backup">
      <h1 className="font-mono text-[22px] font-semibold">{t("backup.title")}</h1>
      <p className="mt-2 text-sm" role="status">{body}</p>
      <div className="mt-3 flex flex-wrap gap-2 text-sm">
        <button type="button" data-testid="backup-now" className="rounded-md bg-primary px-2 py-1 text-primary-foreground" onClick={() => setHistory(backupNow(body, history))}>{t("backup.now")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1">{t("backup.sync")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1">{t("backup.github")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1">{t("backup.disconnect")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1">{t("backup.revoke")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1">{t("backup.retry")}</button>
        <a className="rounded-md border border-border px-2 py-1" href="https://github.com/settings/applications" target="_blank" rel="noreferrer">{t("backup.deletion")}</a>
        <button type="button" className="rounded-md border border-border px-2 py-1">{t("backup.setup")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1">{t("backup.recovery")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1">{t("backup.setAside")}</button>
      </div>
      <p className="mt-2 text-sm">{t("backup.public")}</p>
      <label className="mt-2 block text-sm">
        {t("backup.pat")}
        <input aria-label={t("backup.pat")} className="ml-2 rounded-md border border-input px-2 py-1" />
      </label>
      <label className="mt-2 block text-sm">
        {t("backup.advanced")}
        <input aria-label={t("backup.advanced")} defaultValue="git@localhost:hive.git" className="ml-2 rounded-md border border-input px-2 py-1" />
      </label>
      <p className="mt-2 text-sm text-muted-foreground">{t("settings.engine.note")}</p>
      <button type="button" disabled className="rounded-md border border-border px-2 py-1 text-sm">{t("settings.engine.libgit2")}</button>
      <label className="mt-3 block text-sm">
        {t("backup.device")}
        <input aria-label={t("backup.device")} value={device} onChange={(event) => setDevice(event.target.value)} className="ml-2 rounded-md border border-input px-2 py-1" />
      </label>
      <p className="mt-2 text-sm text-muted-foreground">{t("backup.remote")} git@localhost:hive.git</p>
      <p className="text-sm">{t("backup.scope")}</p>
      <ul className="mt-2 space-y-1 text-sm">
        {history.map((snap) => (
          <li key={snap.id}>
            {snap.label}
            <button
              type="button"
              data-testid="backup-restore"
              className="ml-2 rounded-md border border-border px-2 py-1"
              onClick={() => {
                const restored = restoreSnapshot(history, snap.id);
                if (!restored) return;
                setBody(restored.body);
                setSafety(restored.safety);
              }}
            >
              {t("backup.restore")}
            </button>
          </li>
        ))}
      </ul>
      {safety ? <p role="status">{t("backup.history")} {safety}</p> : null}
      <p className="mt-2 text-sm">{t("backup.firstRun")}</p>
    </section>
  );
}
