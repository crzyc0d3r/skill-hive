import { useMemo, useState } from "react";
import { useSearchParams } from "react-router-dom";
import type { ScanRow } from "@/lib/ipc/catalog";
import type { GitPreview } from "@/lib/ipc/actions";
import { pickPath } from "@/lib/ipc/pick";
import { t } from "@/lib/i18n";
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
    <section data-testid="install">
      <h1 className="font-mono text-[22px] font-semibold">{t("install.title")}</h1>
      <div role="tablist" aria-label={t("install.title")} className="mt-3 flex gap-2">
        {tabs.map((item) => (
          <button
            key={item}
            type="button"
            role="tab"
            aria-selected={active === item}
            data-testid={`tab-${item}`}
            className="rounded-md border border-border px-2 py-1 text-sm"
            onClick={() => setParams({ tab: item })}
          >
            {item === "scan" ? t("dashboard.scan") : t(`install.tab.${item}`)}
          </button>
        ))}
      </div>
      {active === "leaderboard" ? <MarketplacePanel reachable={reachable} /> : null}
      {active === "local" ? (
        <form
          className="mt-4 space-y-2"
          onSubmit={(event) => {
            event.preventDefault();
            const label = folder.split(/[/\\]/).filter(Boolean).pop() ?? folder;
            return act(async () => {
              await onInstallLocal(folder);
            }, () => setInstalled(label));
          }}
        >
          <label className="block text-sm">
            {t("install.local.folder")}
            <input aria-label={t("install.local.folder")} value={folder} onChange={(event) => setFolder(event.target.value)} className="ml-2 rounded-md border border-input px-2 py-1" />
            <button type="button" className="ml-2 rounded-md border border-border px-2 py-1" onClick={() => void pickPath(true).then((path) => path && setFolder(path))}>Browse</button>
          </label>
          <label className="block text-sm">
            {t("install.local.archive")}
            <input aria-label={t("install.local.archive")} value={archive} onChange={(event) => setArchive(event.target.value)} className="ml-2 rounded-md border border-input px-2 py-1" />
            <button type="button" className="ml-2 rounded-md border border-border px-2 py-1" onClick={() => void pickPath(false).then((path) => path && setArchive(path))}>Browse</button>
          </label>
          <button type="submit" disabled={busy} className="rounded-md bg-primary px-3 py-1 text-sm text-primary-foreground">{t("install.local.submit")}</button>
          <button
            type="button"
            disabled={busy}
            className="ml-2 rounded-md border border-border px-3 py-1 text-sm"
            onClick={() => {
              const label = archive.split(/[/\\]/).filter(Boolean).pop() ?? archive;
              void act(async () => {
                await onInstallArchive(archive);
              }, () => setInstalled(label));
            }}
          >
            {t("install.local.archiveSubmit")}
          </button>
          {installed ? <p role="status">{t("install.git.imported")} {installed}</p> : null}
        </form>
      ) : null}
      {active === "git" ? (
        <form
          className="mt-4 space-y-2"
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
          <input aria-label="Git URL" value={url} onChange={(event) => setUrl(event.target.value)} className="w-full rounded-md border border-input px-2 py-1 text-sm" />
          <button type="submit" className="rounded-md bg-primary px-3 py-1 text-sm text-primary-foreground">{t("install.git.preview")}</button>
          {cancelled ? <p role="status">{t("install.git.removed")}</p> : null}
          {preview && !cancelled ? (
            <div data-testid="git-preview" className="rounded-md border border-border p-3">
              <label className="text-sm">
                Rename
                <input aria-label="Rename skill" value={rename} onChange={(event) => setRename(event.target.value)} className="ml-2 rounded-md border border-input px-2 py-1" />
              </label>
              <div className="mt-2 flex gap-2">
                <button
                  type="button"
                  className="rounded-md border border-border px-2 py-1 text-sm"
                  onClick={() => {
                    const label = rename || "skill";
                    void act(async () => {
                      await onImportGit(url, rename, previewId, relative);
                    }, () => setImported(label));
                  }}
                >
                  {t("install.git.import")}
                </button>
                <button
                  type="button"
                  className="rounded-md border border-border px-2 py-1 text-sm"
                  onClick={() => {
                    setCancelled(true);
                    void onCancel(previewId);
                  }}
                >
                  {t("install.git.cancel")}
                </button>
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
                  <input
                    aria-label={`${t("install.scan.rename")} ${row.name}`}
                    value={name}
                    onChange={(event) => setScanNames((current) => ({ ...current, [row.name]: event.target.value }))}
                    className="rounded-md border border-input px-2 py-1"
                  />
                  <button
                    type="button"
                    className="rounded-md border border-border px-2 py-1"
                    onClick={() => {
                      void act(async () => {
                        await onImportOne(row, name);
                      }, () => setImported(name));
                    }}
                  >
                    {t("install.scan.one")} {row.name}
                  </button>
                </li>
              );
            })}
          </ul>
          <button type="button" disabled={busy} className="mt-2 rounded-md bg-primary px-3 py-1 text-sm text-primary-foreground" onClick={() => void act(() => Promise.resolve(onImportAll(scanned.map((row) => row.name))))}>
            {t("install.scan.all")}
          </button>
        </div>
      ) : null}
      {error ? <p role="alert">{error}</p> : null}
      {imported ? <p>{t("install.git.imported")} {imported}</p> : null}
    </section>
  );
}
