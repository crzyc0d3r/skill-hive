import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { useContext, useMemo, useState } from "react";
import { useSearchParams } from "react-router-dom";
import type { ScanRow } from "@/lib/ipc/catalog";
import { listScanRows, type GitPreview, type GitPreviewSkill } from "@/lib/ipc/actions";
import { pickPath } from "@/lib/ipc/pick";
import { AgentBadge } from "@/components/app/AgentBadge";
import { groupScanRows, isFreshScan } from "@/lib/scans";

import { t } from "@/lib/i18n";
import { EmptyState, ErrorState, Screen } from "@/components/chrome";
import { InstallSkeleton } from "@/components/skeletons";
import { RetryContext, useViewPhase } from "@/lib/view-state";
import { Card } from "@/components/ui/card";
import { MarketplacePanel } from "@/features/install/MarketplacePanel";
import { middleEllipsis, scanReasonIsError } from "@/features/install/path-label";

const TABS = ["leaderboard", "local", "git", "scan"] as const;

export function InstallView({
  scanned,
  onImportAll,
  onImportOne,
  onPreview,
  onImportGit,
  onCancel,
  onInstallLocal,
  onInstallArchive,
  marketplaceReachable = true,
}: {
  scanned: ScanRow[];
  onImportAll: (names: string[]) => void | Promise<void>;
  onImportOne: (row: ScanRow, name: string) => void | Promise<void>;
  onPreview: (url: string, rename: string) => void | Promise<GitPreview | void>;
  onImportGit: (url: string, rename: string, previewId: string, relative: string) => void | Promise<void>;
  onCancel: (previewId: string) => void | Promise<void>;
  onInstallLocal: (path: string) => void | Promise<void>;
  onInstallArchive: (path: string) => void | Promise<void>;
  marketplaceReachable?: boolean;
}) {
  const phase = useViewPhase();
  const retry = useContext(RetryContext);
  const [params, setParams] = useSearchParams();
  const tab = params.get("tab") ?? "local";
  const [url, setUrl] = useState("");
  const [rename, setRename] = useState("");
  const [preview, setPreview] = useState(false);
  const [previewId, setPreviewId] = useState("");
  const [previewSkills, setPreviewSkills] = useState<GitPreviewSkill[]>([]);
  const [chosen, setChosen] = useState<string[]>([]);
  const [relative, setRelative] = useState(".");
  const [cancelled, setCancelled] = useState(false);
  const [folder, setFolder] = useState("");
  const [archive, setArchive] = useState("");
  const [installed, setInstalled] = useState<string | null>(null);
  const [imported, setImported] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [scanNames, setScanNames] = useState<Record<string, string>>({});
  const [picked, setPicked] = useState<string[]>([]);
  const [rescanned, setRescanned] = useState<ScanRow[] | null>(null);
  const [scanQuery, setScanQuery] = useState("");
  const [onlyNew, setOnlyNew] = useState(false);
  const scanRows = rescanned ?? scanned;
  const groupedScan = useMemo(() => groupScanRows(scanRows), [scanRows]);
  const freshCount = groupedScan.filter((row) => isFreshScan(row)).length;
  const visibleScan = groupedScan.filter((row) => {
    if (onlyNew && row.alreadyManaged) return false;
    const needle = scanQuery.trim().toLowerCase();
    if (!needle) return true;
    return row.name.toLowerCase().includes(needle) || row.paths.some((item) => item.toLowerCase().includes(needle));
  });
  async function act(work: () => Promise<void>, done?: () => void) {
    setBusy(true);
    setError(null);
    try {
      await work();
      done?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Request failed");
    } finally {
      setBusy(false);
    }
  }
  const reachable = marketplaceReachable !== false;
  const tabs = (reachable ? TABS : TABS.filter((item) => item !== "leaderboard")) as readonly string[];
  const active = useMemo(() => (tabs.includes(tab) ? tab : "local"), [tab, tabs]);

  return (
    <Screen
      testId="install"
      title={t("install.title")}
      lede={t("install.lede")}
      actions={active === "scan" ? (
        <Button type="button" disabled={busy || picked.length === 0} aria-busy={busy} size="sm" onClick={() => void act(() => Promise.resolve(onImportAll(picked)))}>
          {t("install.scan.importSelected", { count: String(picked.length) })}
        </Button>
      ) : undefined}
    >
      <div role="tablist" aria-label={t("install.title")} className="inline-flex max-w-full flex-wrap rounded-lg bg-muted p-0.5">
        {tabs.map((item) => (
          <Button key={item} type="button" role="tab" aria-selected={active === item} data-testid={`tab-${item}`} variant="ghost" size="sm" className={`h-7 rounded-md px-3 shadow-none ${active === item ? "bg-background text-foreground shadow-sm" : "text-muted-foreground hover:bg-transparent"}`} onClick={() => setParams({ tab: item })}>
            {item === "scan" ? (
              <span className="inline-flex items-center gap-1.5">
                {t("install.scan.found")}
                <span className="rounded-full bg-foreground/10 px-1.5 text-[10px] tabular-nums">{freshCount}</span>
              </span>
            ) : t(`install.tab.${item}`)}
          </Button>
        ))}
      </div>
      {phase === "loading" ? (
        <>
          <p role="status" className="text-sm text-muted-foreground">{t("install.scan.scanning", { done: "3", total: "12" })}</p>
          <InstallSkeleton />
        </>
      ) : null}
      {phase === "error" && active === "scan" ? (
        <ul className="space-y-2">
          {["Cursor", "Claude Code"].map((agent) => (
            <li key={agent} className="rounded-md border border-warning/40 bg-warning-subtle px-3 py-2 text-sm text-warning-subtle-foreground">{t("install.scan.warning", { agent })}</li>
          ))}
        </ul>
      ) : null}
      {phase === "error" && active !== "scan" ? <ErrorState title={t("shell.error")} detail={t("shell.renderError")} onRetry={retry} /> : null}
      {phase === "offline" && active !== "leaderboard" ? (
        <div data-testid="install-offline" role="alert" className="mt-4 flex min-h-40 flex-col items-center justify-center gap-3 rounded-lg border border-dashed border-destructive/40 px-6 py-10 text-center">
          <p className="max-w-sm text-sm text-destructive">{t("install.leaderboard.offline")}</p>
          <Button type="button" size="sm" variant="outline" onClick={retry}>{t("banner.retry")}</Button>
        </div>
      ) : null}
      {phase === "empty" ? <EmptyState title={t("install.scan.empty")} body={t("install.lede")} /> : null}
      {(phase === "ready" || phase === "offline") && active === "leaderboard" ? (
        <MarketplacePanel reachable={reachable} mode={phase === "offline" || params.get("market") === "offline" ? "offline" : params.get("market") === "initial" ? "initial" : null} />
      ) : null}
      {phase === "ready" && active === "local" ? (
        <form
          className="max-w-xl"
          onSubmit={(event) => {
            event.preventDefault();
            const label = folder.split(/[/\\]/).filter(Boolean).pop() ?? folder;
            return act(async () => {
              await onInstallLocal(folder);
            }, () => setInstalled(label));
          }}
        >
          <Card className="gap-4 rounded-lg border-border py-4 shadow-none">
            <div className="space-y-4 px-4">
              <label className="block text-sm">
                <span className="mb-1.5 block text-muted-foreground">{t("install.local.folder")}</span>
                <span className="flex gap-2">
                  <Input aria-label={t("install.local.folder")} value={folder} onChange={(event) => setFolder(event.target.value)} placeholder="~/.claude/skills" className="h-8" />
                  <Button type="button" variant="outline" size="sm" onClick={() => void pickPath(true).then((path) => path && setFolder(path))}>Browse</Button>
                </span>
              </label>
              <label className="block text-sm">
                <span className="mb-1.5 block text-muted-foreground">{t("install.local.archive")}</span>
                <span className="flex gap-2">
                  <Input aria-label={t("install.local.archive")} value={archive} onChange={(event) => setArchive(event.target.value)} placeholder="skill.tgz" className="h-8" />
                  <Button type="button" variant="outline" size="sm" onClick={() => void pickPath(false).then((path) => path && setArchive(path))}>Browse</Button>
                </span>
              </label>
              <div className="flex gap-2">
                <Button type="submit" disabled={busy} aria-busy={busy} size="sm">{busy ? t("shell.loading") : t("install.local.submit")}</Button>
                <Button type="button" disabled={busy} variant="outline" size="sm" onClick={() => { const label = archive.split(/[/\\]/).filter(Boolean).pop() ?? archive; void act(async () => { await onInstallArchive(archive); }, () => setInstalled(label)); }}>
                  {t("install.local.archiveSubmit")}
                </Button>
              </div>
              {installed ? <p role="status">{t("install.git.imported")} {installed}</p> : null}
            </div>
          </Card>
        </form>
      ) : null}
      {phase === "ready" && active === "git" ? (
        <form
          className="max-w-xl space-y-3 rounded-lg border border-border bg-card p-4"
          onSubmit={(event) => {
            event.preventDefault();
            return act(async () => {
              const result = await onPreview(url, rename);
              setPreview(true);
              setCancelled(false);
              if (result?.id) {
                setPreviewId(result.id);
                setPreviewSkills(result.skills);
                setChosen(result.skills.map((skill) => skill.relative));
                setRelative(result.skills[0]?.relative ?? ".");
              }
            });
          }}
        >
          <div>
            <h2 className="font-sans text-title font-semibold">{t("install.git.title")}</h2>
            <p className="mt-1 text-sm text-muted-foreground">{t("install.git.lede")}</p>
          </div>
          <label className="block text-sm">
            <span className="mb-1.5 block text-muted-foreground">{t("install.git.url")}</span>
            <Input aria-label="Git URL" value={url} onChange={(event) => setUrl(event.target.value)} placeholder="https://github.com/org/skills" className="h-8 max-w-xl" />
            <span className="mt-1.5 block text-xs text-muted-foreground">{t("install.git.helper")}</span>
          </label>
          <Button type="submit" size="sm" disabled={busy} aria-busy={busy}>{t("install.git.preview")}</Button>
          {cancelled ? <p role="status">{t("install.git.removed")}</p> : null}
          {preview && !cancelled ? (
            <div data-testid="git-preview" className="rounded-md border border-border p-3">
              <label className="text-sm">
                Rename
                <Input aria-label="Rename skill" value={rename} onChange={(event) => setRename(event.target.value)} className="h-8 ml-2" />
              </label>
              <ul className="mt-2 space-y-1">
                {(previewSkills.length ? previewSkills : [{ relative, name: rename || "skill" }]).map((skill) => (
                  <li key={skill.relative}>
                    <label className="text-sm">
                      <input
                        type="checkbox"
                        aria-label={t("install.git.select", { name: skill.name })}
                        checked={chosen.includes(skill.relative)}
                        onChange={() => setChosen((current) => current.includes(skill.relative) ? current.filter((item) => item !== skill.relative) : [...current, skill.relative])}
                      /> {skill.name}
                    </label>
                  </li>
                ))}
              </ul>
              <div className="mt-2 flex gap-2">
                <Button type="button" variant="outline" size="sm" disabled={busy} aria-busy={busy} onClick={() => { const label = rename || "skill"; const picked = chosen.length ? chosen : [relative]; void act(async () => { for (const item of picked) await onImportGit(url, rename, previewId, item); }, () => setImported(label)); }}>
                  {t("install.git.import")}
                </Button>
                <Button type="button" variant="outline" size="sm" onClick={() => { setCancelled(true); void onCancel(previewId); }}>
                  {t("install.git.cancel")}
                </Button>
              </div>
            </div>
          ) : null}
        </form>
      ) : null}
      {phase === "ready" && active === "scan" && scanRows.length === 0 ? <EmptyState title={t("install.scan.empty")} /> : null}
      {(phase === "ready" || phase === "error") && active === "scan" && scanRows.length > 0 ? (
        <div className="mt-4 space-y-3">
          <p className="text-sm text-muted-foreground">{t("install.scan.summary", { skills: String(groupedScan.length), agents: String(new Set(scanRows.map((row) => row.tool)).size), fresh: String(freshCount) })}</p>
          <div className="flex flex-wrap items-end gap-2">
            <label className="block text-sm">
              <span className="mb-1.5 block text-muted-foreground">{t("install.scan.search")}</span>
              <Input aria-label={t("install.scan.search")} value={scanQuery} onChange={(event) => setScanQuery(event.target.value)} className="h-8 w-56" />
            </label>
            <Button type="button" variant="outline" size="sm" aria-pressed={onlyNew} onClick={() => setOnlyNew((current) => !current)}>{t("install.scan.onlyNew")}</Button>
            <Button type="button" variant="outline" size="sm" onClick={() => setPicked(groupedScan.filter((row) => isFreshScan(row)).map((row) => row.name))}>{t("install.scan.selectNew")}</Button>
            <Button type="button" variant="ghost" size="sm" className="sr-only" disabled={busy} aria-busy={busy} onClick={() => void act(() => Promise.resolve(onImportAll(scanRows.map((row) => row.name))))}>{t("install.scan.all")}</Button>
            <Button type="button" variant="ghost" size="sm" disabled={busy} aria-busy={busy} onClick={() => void act(async () => { setRescanned(await listScanRows()); })}>{t("install.scan.rescan")}</Button>
          </div>
          <div className="overflow-hidden rounded-lg border border-border">
            <table className="w-full table-fixed text-sm">
              <colgroup>
                <col className="w-10" />
                <col className="w-[34%]" />
                <col className="w-[40%]" />
                <col className="w-[22%]" />
              </colgroup>
              <thead className="bg-muted/50 text-left text-xs text-muted-foreground">
                <tr>
                  <th className="px-3 py-2" />
                  <th className="px-3 py-2">{t("install.scan.skill")}</th>
                  <th className="px-3 py-2">{t("install.scan.foundIn")}</th>
                  <th className="px-3 py-2">{t("install.scan.reason")}</th>
                </tr>
              </thead>
              <tbody>
            {visibleScan.map((row) => {
              const name = scanNames[row.name] ?? row.name;
              const checked = picked.includes(row.name);
              const reason = scanReasonIsError(row.reason)
                ? row.reason === "parse" ? t("install.scan.parseError") : t("install.scan.readError")
                : row.differs ? t("install.scan.differs") : row.alreadyManaged ? t("install.scan.inLibrary") : t("install.scan.new");
              const fullPath = row.paths.filter(Boolean).join(" · ");
              return (
                <tr key={row.name} className="h-10 border-t border-border">
                  <td className="px-3">
                    <input type="checkbox" aria-label={`${t("install.git.select", { name: row.name })}`} checked={checked} disabled={Boolean(row.alreadyManaged) || scanReasonIsError(row.reason)} onChange={() => setPicked((current) => current.includes(row.name) ? current.filter((item) => item !== row.name) : [...current, row.name])} />
                  </td>
                  <td className="px-3">
                    <span className="flex min-w-0 items-center gap-2">
                      <span className="min-w-0 flex-1 truncate font-medium">{row.name}</span>
                      <span className="flex shrink-0 items-center gap-1">
                        {row.tools.slice(0, 3).map((tool) => <AgentBadge key={tool} agent={tool} />)}
                      </span>
                    </span>
                    <span className="sr-only">
                      <Input aria-label={`${t("install.scan.rename")} ${row.name}`} value={name} onChange={(event) => setScanNames((current) => ({ ...current, [row.name]: event.target.value }))} />
                      <Button type="button" disabled={busy} aria-busy={busy} onClick={() => { void act(async () => { await onImportOne(row, name); }, () => setImported(name)); }}>
                        {t("install.scan.one")} {row.name}
                      </Button>
                    </span>
                  </td>
                  <td className="px-3">
                    <span className="block truncate font-mono text-xs text-muted-foreground" title={fullPath}>{middleEllipsis(fullPath)}</span>
                  </td>
                  <td className={scanReasonIsError(row.reason) ? "px-3 text-destructive" : "px-3 text-muted-foreground"}>
                    <span className="block truncate">{reason}</span>
                  </td>
                </tr>
              );
            })}
              </tbody>
            </table>
          </div>
        </div>
      ) : null}
      {error ? <p role="alert">{error}</p> : null}
      {imported ? <p>{t("install.git.imported")} {imported}</p> : null}
    </Screen>
  );
}
