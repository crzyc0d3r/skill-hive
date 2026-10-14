import { useState } from "react";
import {
  backupSetDeviceName,
  gitBackupRemoveRemote,
  gitBackupRestore,
  gitBackupSetRemote,
  gitBackupStatus,
  gitBackupSync,
  type SnapshotRow,
} from "@/lib/ipc/actions";
import { restoreBackupRemote, revokeBackupToken, setAsideBackup, storeBackupPat } from "@/lib/ipc/actions";
import { demoEnabled } from "@/lib/demo";
import { backupNow, restoreSnapshot } from "@/features/backup/model";
import { t } from "@/lib/i18n";

export function BackupView() {
  const [body, setBody] = useState("library v1");
  const [history, setHistory] = useState<SnapshotRow[]>([]);
  const [safety, setSafety] = useState<string | null>(null);
  const [device, setDevice] = useState("");
  const [remote, setRemote] = useState("");
  const [pat, setPat] = useState("");
  const [note, setNote] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function run(work: () => Promise<void>) {
    setBusy(true);
    setError(null);
    try {
      await work();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Request failed");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section data-testid="backup">
      <h1 className="font-mono text-[22px] font-semibold">{t("backup.title")}</h1>
      <p className="mt-2 text-sm" role="status">{body}</p>
      <div className="mt-3 flex flex-wrap gap-2 text-sm">
        <button
          type="button"
          data-testid="backup-now"
          disabled={busy}
          className="rounded-md bg-primary px-2 py-1 text-primary-foreground"
          onClick={() =>
            void run(async () => {
              if (demoEnabled()) {
                const snap = backupNow(body, [])[0];
                setHistory((current) => [{ id: snap.id, label: snap.label }, ...current]);
                setBody(snap.label);
                return;
              }
              const snap = await gitBackupSync(body);
              setHistory((current) => [snap, ...current]);
              setBody(snap.label);
            })
          }
        >
          {t("backup.now")}
        </button>
        <button type="button" className="rounded-md border border-border px-2 py-1" onClick={() => void run(async () => { const snap = await gitBackupSync("sync"); setHistory((current) => [snap, ...current]); setNote(snap.label); })}>{t("backup.sync")}</button>
        <button
          type="button"
          className="rounded-md border border-border px-2 py-1"
          onClick={() =>
            void run(async () => {
              if (remote.trim()) await gitBackupSetRemote(remote.trim());
              setNote("Remote saved. GitHub device login needs an OAuth client id, which this build does not ship.");
            })
          }
        >
          {t("backup.github")}
        </button>
        <button type="button" className="rounded-md border border-border px-2 py-1" onClick={() => void run(async () => { await gitBackupRemoveRemote(); setNote("Remote removed"); })}>{t("backup.disconnect")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1" onClick={() => { void revokeBackupToken(); setPat(""); setNote("Token cleared"); }}>{t("backup.revoke")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1" onClick={() => void run(async () => { const snap = await gitBackupSync("retry"); setHistory((current) => [snap, ...current]); })}>{t("backup.retry")}</button>
        <a className="rounded-md border border-border px-2 py-1" href="https://github.com/settings/applications" target="_blank" rel="noreferrer">{t("backup.deletion")}</a>
        <button type="button" className="rounded-md border border-border px-2 py-1" onClick={() => void run(async () => { const status = await gitBackupStatus(); setNote(status.branch || "repository ready"); })}>{t("backup.setup")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1" onClick={() => void gitBackupStatus().then((status) => setNote(status.head || "no commits yet")).catch((err: Error) => setError(err.message))}>{t("backup.recovery")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1" onClick={() => { void setAsideBackup(body); setNote("Set aside"); }}>{t("backup.setAside")}</button>
      </div>
      <p className="mt-2 text-sm">{t("backup.public")}</p>
      <label className="mt-2 block text-sm">
        {t("backup.pat")}
        <input
          aria-label={t("backup.pat")}
          value={pat}
          onChange={(event) => {
            const next = event.target.value;
            setPat(next);
            if (next.trim()) void storeBackupPat(next.trim());
          }}
          className="ml-2 rounded-md border border-input px-2 py-1"
        />
      </label>
      <label className="mt-2 block text-sm">
        {t("backup.advanced")}
        <input aria-label={t("backup.advanced")} value={remote} onChange={(event) => setRemote(event.target.value)} className="ml-2 rounded-md border border-input px-2 py-1" />
      </label>
      <p className="mt-2 text-sm text-muted-foreground">{t("settings.engine.note")}</p>
      <button type="button" disabled className="rounded-md border border-border px-2 py-1 text-sm">{t("settings.engine.libgit2")}</button>
      <label className="mt-3 block text-sm">
        {t("backup.device")}
        <input
          aria-label={t("backup.device")}
          value={device}
          onChange={(event) => setDevice(event.target.value)}
          onBlur={() => { if (device.trim()) void backupSetDeviceName(device.trim()); }}
          className="ml-2 rounded-md border border-input px-2 py-1"
        />
      </label>
      <p className="mt-2 text-sm text-muted-foreground">{t("backup.remote")} {remote}</p>
      <p className="text-sm">{t("backup.scope")}</p>
      {note ? <p role="status">{note}</p> : null}
      {error ? <p role="alert">{error}</p> : null}
      <ul className="mt-2 space-y-1 text-sm">
        {history.map((snap) => (
          <li key={snap.id}>
            {snap.label}
            <button
              type="button"
              data-testid="backup-restore"
              className="ml-2 rounded-md border border-border px-2 py-1"
              onClick={() =>
                void run(async () => {
                  if (demoEnabled()) {
                    const restored = restoreSnapshot([{ id: snap.id, label: snap.label, body }], snap.id);
                    setBody(restored?.body ?? snap.label);
                    setSafety(restored?.safety ?? null);
                    return;
                  }
                  const restored = await gitBackupRestore(snap.id);
                  setBody(snap.label);
                  setSafety(restored);
                })
              }
            >
              {t("backup.restore")}
            </button>
          </li>
        ))}
      </ul>
      {safety ? <p role="status">{t("backup.history")} {safety}</p> : null}
      <form
        className="mt-3 space-y-2 text-sm"
        data-testid="first-run-restore"
        onSubmit={(event) => {
          event.preventDefault();
          const data = new FormData(event.currentTarget);
          const remote = String(data.get("remote") ?? "");
          void restoreBackupRemote(remote);
        }}
      >
        <p>{t("backup.firstRun")}</p>
        <label>
          Remote
          <input name="remote" aria-label="Restore remote" className="ml-2 rounded-md border border-input px-2 py-1" />
        </label>
        <button type="submit" className="ml-2 rounded-md border border-border px-2 py-1">Use remote library</button>
      </form>
    </section>
  );
}
