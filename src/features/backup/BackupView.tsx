import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { useEffect, useState } from "react";
import {
  backupSetDeviceName,
  gitBackupFetch,
  gitBackupInit,
  gitBackupRemoveRemote,
  gitBackupResolveConflict,
  gitBackupRestore,
  gitBackupSanitizeRemoteUrl,
  gitBackupSetRemote,
  gitBackupStatus,
  gitBackupSync,
  githubBackupConnect,
  githubDeviceFlowPoll,
  githubDeviceFlowStart,
  silent,
  type SnapshotRow,
} from "@/lib/ipc/actions";
import { restoreBackupRemote, revokeBackupToken, setAsideBackup, storeBackupPat } from "@/lib/ipc/actions";
import { demoEnabled } from "@/lib/demo";
import { backupNow, restoreSnapshot } from "@/features/backup/model";
import { t } from "@/lib/i18n";
import { Screen } from "@/components/chrome";

interface ConflictRow {
  skillId: string;
  theirsCommit: string;
}

function repoNameFrom(url: string): string {
  const trimmed = url.trim().replace(/\.git$/, "");
  const name = trimmed.split("/").filter(Boolean).pop();
  return name || "skill-hive-backup";
}

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
  const [conflicts, setConflicts] = useState<ConflictRow[]>([]);

  useEffect(() => {
    void silent<string>("backup_device_name").then((name) => {
      if (typeof name === "string" && name) setDevice(name);
    });
    void silent<ConflictRow[]>("git_backup_pending_conflicts").then((rows) => {
      if (Array.isArray(rows)) setConflicts(rows.map((row) => ({ skillId: row.skillId, theirsCommit: row.theirsCommit })));
    });
    void silent("git_backup_migrate_credentials");
    void silent("git_backup_fetch");
  }, []);

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
    <Screen testId="backup" title={t("backup.title")}>
      <p className="mt-2 text-sm" role="status">{body}</p>
      <div className="mt-3 flex flex-wrap gap-2 text-sm">
        <Button type="button" data-testid="backup-now" disabled={busy} size="sm" onClick={() =>
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
        </Button>
        <Button type="button" variant="outline" size="sm" onClick={() => void run(async () => { const snap = await gitBackupSync("sync"); setHistory((current) => [snap, ...current]); setNote(snap.label); })}>{t("backup.sync")}</Button>
        <Button type="button" variant="outline" size="sm" onClick={() =>
            void run(async () => {
              const cleaned = remote.trim() ? await gitBackupSanitizeRemoteUrl(remote.trim()) : "";
              const url = cleaned || remote.trim();
              if (url) {
                setRemote(url);
                await gitBackupSetRemote(url);
              }
              const repoName = repoNameFrom(url);
              if (pat.trim()) {
                const connected = await githubBackupConnect(pat.trim(), repoName);
                setNote(connected?.login ? `${connected.login} ${connected.url}` : "Remote saved");
                return;
              }
              const start = await githubDeviceFlowStart();
              if (!start?.userCode) {
                setNote("Remote saved. GitHub device login needs an OAuth client id, which this build does not ship.");
                return;
              }
              setNote(`${start.userCode} ${start.verificationUri}`);
              const deadline = Date.now() + 120000;
              const wait = Math.max(start.interval || 5, 1) * 1000;
              while (Date.now() < deadline) {
                await new Promise((resolve) => setTimeout(resolve, wait));
                const poll = await githubDeviceFlowPoll(start.deviceCode, repoName);
                if (poll?.status === "connected") {
                  setNote(poll.result?.url ?? poll.result?.login ?? "Connected");
                  return;
                }
                if (poll?.status === "denied") {
                  setNote("GitHub denied the device login");
                  return;
                }
              }
            })
          }
        >
          {t("backup.github")}
        </Button>
        <Button type="button" variant="outline" size="sm" onClick={() => void run(async () => { await gitBackupRemoveRemote(); setNote("Remote removed"); })}>{t("backup.disconnect")}</Button>
        <Button type="button" variant="outline" size="sm" onClick={() => { void revokeBackupToken(); setPat(""); setNote("Token cleared"); }}>{t("backup.revoke")}</Button>
        <Button type="button" variant="outline" size="sm" onClick={() => void run(async () => { const snap = await gitBackupSync("retry"); setHistory((current) => [snap, ...current]); })}>{t("backup.retry")}</Button>
        <Button asChild variant="outline" size="sm">
          <a href="https://github.com/settings/applications" target="_blank" rel="noreferrer">{t("backup.deletion")}</a>
        </Button>
        <Button type="button" variant="outline" size="sm" onClick={() => void run(async () => { await gitBackupInit(); const status = await gitBackupStatus(); setNote(status.branch || "repository ready"); })}>{t("backup.setup")}</Button>
        <Button type="button" variant="outline" size="sm" onClick={() => void gitBackupFetch().then(() => gitBackupStatus()).then((status) => setNote(status.head || "no commits yet")).catch((err: Error) => setError(err.message))}>{t("backup.recovery")}</Button>
        <Button type="button" variant="outline" size="sm" onClick={() => { void setAsideBackup(body); setNote("Set aside"); }}>{t("backup.setAside")}</Button>
      </div>
      <p className="mt-2 text-sm">{t("backup.public")}</p>
      <label className="mt-2 block text-sm">
        {t("backup.pat")}
        <Input aria-label={t("backup.pat")} className="ml-2 h-8 w-56" value={pat} onChange={(event) => {
            const next = event.target.value;
            setPat(next);
            if (next.trim()) void storeBackupPat(next.trim());
          }}
        />
      </label>
      <label className="mt-2 block text-sm">
        {t("backup.advanced")}
        <Input aria-label={t("backup.advanced")} className="ml-2 h-8 w-56" value={remote} onChange={(event) => setRemote(event.target.value)} />
      </label>
      <p className="mt-2 text-sm text-muted-foreground">{t("settings.engine.note")}</p>
      <Button type="button" disabled variant="outline" size="sm">{t("settings.engine.libgit2")}</Button>
      <label className="mt-3 block text-sm">
        {t("backup.device")}
        <Input aria-label={t("backup.device")} className="ml-2 h-8 w-56" value={device} onChange={(event) => setDevice(event.target.value)}
          onBlur={() => { if (device.trim()) void backupSetDeviceName(device.trim()); }}
        />
      </label>
      <p className="mt-2 text-sm text-muted-foreground">{t("backup.remote")} {remote}</p>
      <p className="text-sm">{t("backup.scope")}</p>
      {conflicts.length > 0 ? (
        <ul className="mt-2 space-y-1 text-sm">
          {conflicts.map((row) => (
            <li key={row.skillId}>
              {row.skillId}
              <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => void run(async () => { await gitBackupResolveConflict(row.skillId, "keep_local"); setConflicts((current) => current.filter((item) => item.skillId !== row.skillId)); })}>{t("backup.keepMine")}</Button>
              <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => void run(async () => { await gitBackupResolveConflict(row.skillId, "use_remote"); setConflicts((current) => current.filter((item) => item.skillId !== row.skillId)); })}>{t("backup.useRemote")}</Button>
              <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => void run(async () => { await gitBackupResolveConflict(row.skillId, "keep_both"); setConflicts((current) => current.filter((item) => item.skillId !== row.skillId)); })}>{t("backup.keepBoth")}</Button>
            </li>
          ))}
        </ul>
      ) : null}
      {note ? <p role="status">{note}</p> : null}
      {error ? <p role="alert">{error}</p> : null}
      <ul className="mt-2 space-y-1 text-sm">
        {history.map((snap) => (
          <li key={snap.id}>
            {snap.label}
            <Button type="button" data-testid="backup-restore" variant="outline" size="sm" className="ml-2" onClick={() =>
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
            </Button>
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
          <Input name="remote" aria-label="Restore remote" className="mt-1 h-8 max-w-xl" />
        </label>
        <Button type="submit" variant="outline" size="sm" className="ml-2">Use remote library</Button>
      </form>
    </Screen>
  );
}
