import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { useContext, useEffect, useState } from "react";
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
import { EmptyState, ErrorState, Screen } from "@/components/chrome";
import { BackupSkeleton } from "@/components/skeletons";
import { RetryContext, useViewPhase } from "@/lib/view-state";
import { toast } from "sonner";
import { MoreHorizontalIcon } from "lucide-react";

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
  const phase = useViewPhase();
  const retry = useContext(RetryContext);
  const [body, setBody] = useState("library v1");
  const [history, setHistory] = useState<SnapshotRow[]>(() => (demoEnabled() ? [
    { id: "2026-10-21", label: "2026-10-21" },
    { id: "2026-10-18", label: "2026-10-18" },
  ] : []));
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
  const [menuId, setMenuId] = useState<string | null>(null);

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

  if (phase === "loading" || phase === "error" || phase === "empty") {
    return (
      <Screen testId="backup" title={t("backup.title")} lede={t("backup.lede")} actions={
        <Button type="button" data-testid="backup-now" size="sm" disabled={phase !== "empty"} aria-busy={phase === "loading"}>
          {t("backup.now")}
        </Button>
      }>
        {phase === "loading" ? <BackupSkeleton /> : null}
        {phase === "error" ? <ErrorState title={t("shell.error")} detail={t("shell.renderError")} onRetry={retry} /> : null}
        {phase === "empty" ? <EmptyState title={t("backup.snapshots")} body={t("backup.lede")} /> : null}
      </Screen>
    );
  }

  return (
    <Screen testId="backup" title={t("backup.title")} lede={t("backup.lede")} actions={
      <Button type="button" data-testid="backup-now" disabled={busy} aria-busy={busy} size="sm" onClick={() =>
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
    }>
      <p className="mt-2 text-sm" role="status">{body}</p>
      <div className="overflow-hidden rounded-lg border border-border">
        <table className="w-full text-sm">
          <caption className="px-3 py-2 text-left text-sm font-medium">{t("backup.snapshots")}</caption>
          <thead className="text-left text-xs text-muted-foreground">
            <tr>
              <th className="px-3 py-2">{t("history.date")}</th>
              <th className="px-3 py-2">{t("backup.size")}</th>
              <th className="px-3 py-2">{t("backup.scope.skills")}</th>
              <th className="px-3 py-2 text-right">{t("backup.restore")}</th>
            </tr>
          </thead>
          <tbody>
            {history.map((snap, index) => (
              <tr key={snap.id} className="border-t border-border">
                <td className="px-3 py-2">{snap.label}</td>
                <td className="px-3 py-2">{index === 0 ? "1.2 MB" : "1.1 MB"}</td>
                <td className="px-3 py-2">{index === 0 ? "12" : "11"}</td>
                <td className="relative px-3 py-2 text-right">
                  <Button type="button" variant="ghost" size="icon-sm" data-testid="backup-row-menu" aria-label={`${t("backup.rowMenu")} ${snap.label}`} aria-expanded={menuId === snap.id} onClick={() => setMenuId(menuId === snap.id ? null : snap.id)}>
                    <MoreHorizontalIcon aria-hidden="true" />
                  </Button>
                  {menuId === snap.id ? (
                    <div role="menu" className="absolute right-3 z-20 mt-1 w-36 rounded-md border border-border bg-popover p-1 text-left shadow-md">
                      <button type="button" role="menuitem" data-testid="backup-restore" className="block w-full rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent" onClick={() => {
                        setMenuId(null);
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
                        });
                      }}>
                        {t("backup.restore")}
                      </button>
                    </div>
                  ) : null}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {status ? (
        <p data-testid="backup-status" className="text-sm">
          {t("backup.branch")} {status.branch} · {status.clean ? t("backup.clean") : t("backup.dirty")}
          {status.remote ? ` · ${status.remote}` : ""}
        </p>
      ) : null}
      <details className="rounded-lg border border-border bg-card px-3 py-2 text-sm">
        <summary className="cursor-pointer">{t("shell.more")}</summary>
        <div className="mt-2 flex flex-wrap gap-2">
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
        <Button type="button" variant="outline" size="sm" className="w-fit" onClick={() => { void setAsideBackup(body); setNote("Set aside"); }}>{t("backup.setAside")}</Button>
        </div>
      </details>
      <div className="grid items-start gap-3 lg:grid-cols-3">
        <section className="space-y-3 rounded-lg border border-border bg-card p-3">
          <h2 className="text-sm font-medium">{t("backup.section.token")}</h2>
          <p className="text-sm text-muted-foreground">{t("backup.public")}</p>
          {authMethod === "pat" ? (
            <label className="block text-sm">
              <span className="mb-1.5 block text-muted-foreground">{t("backup.pat")}</span>
              <Input aria-label={t("backup.pat")} className="h-8 w-full" value={pat} onChange={(event) => {
                  const next = event.target.value;
                  setPat(next);
                  if (next.trim()) void storeBackupPat(next.trim());
                }}
              />
            </label>
          ) : null}
        </section>
        <section className="space-y-3 rounded-lg border border-border bg-card p-3">
          <h2 className="text-sm font-medium">{t("backup.section.engine")}</h2>
          <p className="text-sm text-muted-foreground">{t("settings.engine.note")}</p>
          <Button type="button" variant="outline" size="sm" onClick={() => { void silent("set_settings", { key: "git_backup_engine", value: "git2" }); }}>{t("settings.engine.libgit2")}</Button>
          <label className="block text-sm">
            <input type="checkbox" aria-label={t("backup.auto")} checked={autoBackup} onChange={(event) => { const next = event.target.checked; setAutoBackup(next); void silent("set_settings", { key: "backup_auto", value: next ? "1" : "0" }); }} /> {t("backup.auto")}
          </label>
          <fieldset className="text-sm">
            <legend>{t("backup.scope")}</legend>
            <label className="mr-3"><input type="checkbox" aria-label={t("backup.scope.skills")} checked={scope.skills} onChange={(event) => writeScope({ ...scope, skills: event.target.checked })} /> {t("backup.scope.skills")}</label>
            <label className="mr-3"><input type="checkbox" aria-label={t("backup.scope.presets")} checked={scope.presets} onChange={(event) => writeScope({ ...scope, presets: event.target.checked })} /> {t("backup.scope.presets")}</label>
            <label><input type="checkbox" aria-label={t("backup.scope.settings")} checked={scope.settings} onChange={(event) => writeScope({ ...scope, settings: event.target.checked })} /> {t("backup.scope.settings")}</label>
          </fieldset>
        </section>
        <section className="space-y-3 rounded-lg border border-border bg-card p-3">
          <h2 className="text-sm font-medium">{t("backup.section.remote")}</h2>
          <label className="block text-sm">
            <span className="mb-1.5 block text-muted-foreground">{t("backup.advanced")}</span>
            <Input aria-label={t("backup.advanced")} className="h-8 w-full" value={remote} onChange={(event) => setRemote(event.target.value)} />
          </label>
          <label className="block text-sm">
            <span className="mb-1.5 block text-muted-foreground">{t("backup.device")}</span>
            <Input aria-label={t("backup.device")} className="h-8 w-full" value={device} onChange={(event) => setDevice(event.target.value)}
              onBlur={() => { if (device.trim()) void backupSetDeviceName(device.trim()); }}
            />
          </label>
          <p className="text-sm text-muted-foreground"><span className="block text-xs">{t("backup.remote")}</span>{remote}</p>
          <form
            className="space-y-2 text-sm"
            data-testid="first-run-restore"
            onSubmit={(event) => {
              event.preventDefault();
              const data = new FormData(event.currentTarget);
              const remoteValue = String(data.get("remote") ?? "");
              void restoreBackupRemote(remoteValue);
            }}
          >
            <p>{t("backup.firstRun")}</p>
            <label className="block">
              <span className="mb-1.5 block text-muted-foreground">{t("backup.remote")}</span>
              <Input name="remote" aria-label="Restore remote" className="h-8 w-full" />
            </label>
            <Button type="submit" variant="outline" size="sm">Use remote library</Button>
          </form>
        </section>
      </div>
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
      {safety ? <p role="status">{t("backup.history")} {safety}</p> : null}
    </Screen>
  );
}
