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
import { restoreBackupRemote, revokeBackupToken, setAsideBackup, storeBackupPat, type BackupStatus } from "@/lib/ipc/actions";
import { demoEnabled } from "@/lib/demo";
import { backupNow, restoreSnapshot } from "@/features/backup/model";
import { t } from "@/lib/i18n";
import { Screen } from "@/components/chrome";
import { toast } from "sonner";

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
  const [authMethod, setAuthMethod] = useState<"pat" | "device">("pat");
  const [status, setStatus] = useState<BackupStatus | null>(null);
  const [autoBackup, setAutoBackup] = useState(false);
  const [scope, setScope] = useState({ skills: true, presets: true, settings: true });

  useEffect(() => {
    void silent<string>("backup_device_name").then((name) => {
      if (typeof name === "string" && name) setDevice(name);
    });
    void silent<ConflictRow[]>("git_backup_pending_conflicts").then((rows) => {
      if (Array.isArray(rows)) setConflicts(rows.map((row) => ({ skillId: row.skillId, theirsCommit: row.theirsCommit })));
    });
    void silent("git_backup_migrate_credentials");
    void silent("git_backup_fetch");
    void silent<string | null>("get_settings", { key: "github_auth_method" }).then((value) => {
      setAuthMethod(value === "device" ? "device" : "pat");
    });
    void silent<BackupStatus>("git_backup_status").then((row) => {
      if (!row?.branch) return;
      setStatus(row);
      if (row.remote) setRemote(row.remote);
    });
    void silent<string | null>("get_settings", { key: "backup_auto" }).then((value) => {
      if (value === "1") setAutoBackup(true);
    });
    void silent<string | null>("get_settings", { key: "backup_scope" }).then((value) => {
      if (!value) return;
      try {
        const parsed = JSON.parse(value) as { skills?: boolean; presets?: boolean; settings?: boolean };
        setScope({ skills: parsed.skills !== false, presets: parsed.presets !== false, settings: parsed.settings !== false });
      } catch {
        /* keep the default scope */
      }
    });
  }, []);

  function writeScope(next: { skills: boolean; presets: boolean; settings: boolean }) {
    setScope(next);
    void silent("set_settings", { key: "backup_scope", value: JSON.stringify(next) });
  }

  async function run(work: () => Promise<void>) {
    setBusy(true);
    setError(null);
    const toastId = toast.loading(t("backup.progress"));
    try {
      await work();
      toast.success(t("backup.progress"), { id: toastId });
    } catch (err) {
      toast.dismiss(toastId);
      setError(err instanceof Error ? err.message : "Request failed");
    } finally {
      setBusy(false);
    }
  }

  async function startDeviceLogin(url: string) {
    const repoName = repoNameFrom(url);
    const start = await githubDeviceFlowStart();
    if (!start?.userCode) return;
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
  }

  return (
    <Screen testId="backup" title={t("backup.title")}>
      <p className="mt-2 text-sm" role="status">{body}</p>
      {status ? (
        <p data-testid="backup-status" className="text-sm">
          {t("backup.branch")} {status.branch} · {status.clean ? t("backup.clean") : t("backup.dirty")}
          {status.remote ? ` · ${status.remote}` : ""}
        </p>
      ) : null}
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
        {authMethod === "pat" ? (
          <Button type="button" variant="outline" size="sm" onClick={() =>
              void run(async () => {
                const cleaned = remote.trim() ? await gitBackupSanitizeRemoteUrl(remote.trim()) : "";
                const url = cleaned || remote.trim();
                if (url) {
                  setRemote(url);
                  await gitBackupSetRemote(url);
                }
                const repoName = repoNameFrom(url);
                if (!pat.trim()) return;
                const connected = await githubBackupConnect(pat.trim(), repoName);
                setNote(connected?.login ? `${connected.login} ${connected.url}` : "Remote saved");
              })
            }
          >
            {t("backup.github")}
          </Button>
        ) : (
          <Button type="button" variant="outline" size="sm" onClick={() =>
              void run(async () => {
                const cleaned = remote.trim() ? await gitBackupSanitizeRemoteUrl(remote.trim()) : "";
                const url = cleaned || remote.trim();
                if (url) {
                  setRemote(url);
                  await gitBackupSetRemote(url);
                }
                await startDeviceLogin(url);
              })
            }
          >
            {t("backup.deviceStart")}
          </Button>
        )}
        <Button type="button" variant="outline" size="sm" onClick={() => void run(async () => { await gitBackupRemoveRemote(); setNote("Remote removed"); })}>{t("backup.disconnect")}</Button>
        <Button type="button" variant="outline" size="sm" onClick={() => { void revokeBackupToken(); setPat(""); setNote("Token cleared"); }}>{t("backup.revoke")}</Button>
        <Button type="button" variant="outline" size="sm" onClick={() => void run(async () => { const snap = await gitBackupSync("retry"); setHistory((current) => [snap, ...current]); })}>{t("backup.retry")}</Button>
        <Button asChild variant="outline" size="sm">
          <a href={status?.remote || "https://github.com/settings/applications"} target="_blank" rel="noreferrer" onClick={() => { void gitBackupRemoveRemote(); }}>{t("backup.deletion")}</a>
        </Button>
        <Button type="button" variant="outline" size="sm" onClick={() => void run(async () => { await gitBackupInit(); const status = await gitBackupStatus(); setNote(status.branch || "repository ready"); })}>{t("backup.setup")}</Button>
        <Button type="button" variant="outline" size="sm" onClick={() => void gitBackupFetch().then(() => gitBackupStatus()).then((status) => setNote(status.head || "no commits yet")).catch((err: Error) => setError(err.message))}>{t("backup.recovery")}</Button>
        <Button type="button" variant="outline" size="sm" onClick={() => { void setAsideBackup(body); setNote("Set aside"); }}>{t("backup.setAside")}</Button>
      </div>
      <p className="mt-2 text-sm">{t("backup.public")}</p>
      {authMethod === "pat" ? (
        <label className="mt-2 block text-sm">
          {t("backup.pat")}
          <Input aria-label={t("backup.pat")} className="ml-2 h-8 w-56" value={pat} onChange={(event) => {
              const next = event.target.value;
              setPat(next);
              if (next.trim()) void storeBackupPat(next.trim());
            }}
          />
        </label>
      ) : null}
      <label className="mt-2 block text-sm">
        <input type="checkbox" aria-label={t("backup.auto")} checked={autoBackup} onChange={(event) => { const next = event.target.checked; setAutoBackup(next); void silent("set_settings", { key: "backup_auto", value: next ? "1" : "0" }); }} /> {t("backup.auto")}
      </label>
      <fieldset className="mt-2 text-sm">
        <legend>{t("backup.scope")}</legend>
        <label className="mr-3"><input type="checkbox" aria-label={t("backup.scope.skills")} checked={scope.skills} onChange={(event) => writeScope({ ...scope, skills: event.target.checked })} /> {t("backup.scope.skills")}</label>
        <label className="mr-3"><input type="checkbox" aria-label={t("backup.scope.presets")} checked={scope.presets} onChange={(event) => writeScope({ ...scope, presets: event.target.checked })} /> {t("backup.scope.presets")}</label>
        <label><input type="checkbox" aria-label={t("backup.scope.settings")} checked={scope.settings} onChange={(event) => writeScope({ ...scope, settings: event.target.checked })} /> {t("backup.scope.settings")}</label>
      </fieldset>
      <label className="mt-2 block text-sm">
        {t("backup.advanced")}
        <Input aria-label={t("backup.advanced")} className="ml-2 h-8 w-56" value={remote} onChange={(event) => setRemote(event.target.value)} />
      </label>
      <p className="mt-2 text-sm text-muted-foreground">{t("settings.engine.note")}</p>
      <Button type="button" variant="outline" size="sm" onClick={() => { void silent("set_settings", { key: "git_backup_engine", value: "git2" }); }}>{t("settings.engine.libgit2")}</Button>
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
