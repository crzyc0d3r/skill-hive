import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { useMemo, useState } from "react";
import { useSearchParams } from "react-router-dom";
import type { ScanRow } from "@/lib/ipc/catalog";
import type { GitPreview } from "@/lib/ipc/actions";
import { pickPath } from "@/lib/ipc/pick";
import { t } from "@/lib/i18n";
import { Screen } from "@/components/chrome";
import { Card } from "@/components/ui/card";
import { MarketplacePanel } from "@/features/install/MarketplacePanel";

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
  const [params, setParams] = useSearchParams();
  const tab = params.get("tab") ?? "local";
  const [url, setUrl] = useState("");
  const [rename, setRename] = useState("");
  const [preview, setPreview] = useState(false);
  const [previewId, setPreviewId] = useState("");
  const [relative, setRelative] = useState(".");
  const [cancelled, setCancelled] = useState(false);
  const [folder, setFolder] = useState("");
  const [archive, setArchive] = useState("");
  const [installed, setInstalled] = useState<string | null>(null);
  const [imported, setImported] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [scanNames, setScanNames] = useState<Record<string, string>>({});
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
    <Screen testId="install" title={t("install.title")}>
      <div role="tablist" aria-label={t("install.title")} className="inline-flex max-w-full flex-wrap rounded-lg bg-muted p-0.5">
        {tabs.map((item) => (
          <Button key={item} type="button" role="tab" aria-selected={active === item} data-testid={`tab-${item}`} variant="ghost" size="sm" className={`h-7 rounded-md px-3 shadow-none ${active === item ? "bg-background text-foreground shadow-sm" : "text-muted-foreground hover:bg-transparent"}`} onClick={() => setParams({ tab: item })}>
            {item === "scan" ? t("dashboard.scan") : t(`install.tab.${item}`)}
          </Button>
        ))}
      </div>
      {active === "leaderboard" ? <MarketplacePanel reachable={reachable} /> : null}
      {active === "local" ? (
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
                <Button type="submit" disabled={busy} size="sm">{t("install.local.submit")}</Button>
                <Button type="button" disabled={busy} variant="outline" size="sm" onClick={() => { const label = archive.split(/[/\\]/).filter(Boolean).pop() ?? archive; void act(async () => { await onInstallArchive(archive); }, () => setInstalled(label)); }}>
                  {t("install.local.archiveSubmit")}
                </Button>
              </div>
              {installed ? <p role="status">{t("install.git.imported")} {installed}</p> : null}
            </div>
          </Card>
        </form>
      ) : null}
      {active === "git" ? (
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
                setRelative(result.skills[0]?.relative ?? ".");
              }
            });
          }}
        >
          <Input aria-label="Git URL" value={url} onChange={(event) => setUrl(event.target.value)} placeholder="https://github.com/org/skills" className="h-8 max-w-xl" />
          <Button type="submit" size="sm">{t("install.git.preview")}</Button>
          {cancelled ? <p role="status">{t("install.git.removed")}</p> : null}
          {preview && !cancelled ? (
            <div data-testid="git-preview" className="rounded-md border border-border p-3">
              <label className="text-sm">
                Rename
                <Input aria-label="Rename skill" value={rename} onChange={(event) => setRename(event.target.value)} className="h-8 ml-2" />
              </label>
              <div className="mt-2 flex gap-2">
                <Button type="button" variant="outline" size="sm" onClick={() => { const label = rename || "skill"; void act(async () => { await onImportGit(url, rename, previewId, relative); }, () => setImported(label)); }}>
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
      {active === "scan" ? (
        <div className="mt-4">
          <ul>
            {scanned.map((row) => {
              const name = scanNames[row.name] ?? row.name;
              return (
                <li key={`${row.tool}-${row.name}`} className="mt-2 flex flex-wrap items-center gap-2 text-sm">
                  <span>{row.tool}</span>
                  <Input aria-label={`${t("install.scan.rename")} ${row.name}`} value={name} onChange={(event) => setScanNames((current) => ({ ...current, [row.name]: event.target.value }))} className="h-8" />
                  <Button type="button" variant="outline" size="sm" onClick={() => { void act(async () => { await onImportOne(row, name); }, () => setImported(name)); }}>
                    {t("install.scan.one")} {row.name}
                  </Button>
                </li>
              );
            })}
          </ul>
          <Button type="button" disabled={busy} variant="default" size="sm" className="mt-2" onClick={() => void act(() => Promise.resolve(onImportAll(scanned.map((row) => row.name))))}>
            {t("install.scan.all")}
          </Button>
        </div>
      ) : null}
      {error ? <p role="alert">{error}</p> : null}
      {imported ? <p>{t("install.git.imported")} {imported}</p> : null}
    </Screen>
  );
}
