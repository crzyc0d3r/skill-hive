import { useMemo, useState } from "react";
import { useSearchParams } from "react-router-dom";
import type { ScanRow } from "@/lib/phase1/client";
import { t } from "@/lib/i18n";

const TABS = ["leaderboard", "local", "git", "scan"] as const;

export function InstallView({
  scanned,
  onImportAll,
  onImportOne,
  onPreview,
  onCancel,
  onInstallLocal,
  onInstallArchive,
}: {
  scanned: ScanRow[];
  onImportAll?: (names: string[]) => void;
  onImportOne?: (name: string) => void;
  onPreview?: (url: string, rename: string) => void;
  onCancel?: () => void;
  onInstallLocal?: (path: string) => void;
  onInstallArchive?: (path: string) => void;
}) {
  const [params, setParams] = useSearchParams();
  const tab = params.get("tab") ?? "local";
  const [url, setUrl] = useState("https://github.com/acme/skills/tree/feature/slash/tools/demo");
  const [rename, setRename] = useState("renamed-demo");
  const [preview, setPreview] = useState(false);
  const [cancelled, setCancelled] = useState(false);
  const [folder, setFolder] = useState("/path/to/demo");
  const [archive, setArchive] = useState("/path/to/demo.skill");
  const [installed, setInstalled] = useState<string | null>(null);
  const [imported, setImported] = useState<string | null>(null);
  const [scanNames, setScanNames] = useState<Record<string, string>>({});
  const active = useMemo(() => (TABS.includes(tab as (typeof TABS)[number]) ? tab : "local"), [tab]);

  return (
    <section data-testid="install">
      <h1 className="font-mono text-[22px] font-semibold">{t("install.title")}</h1>
      <div role="tablist" aria-label={t("install.title")} className="mt-3 flex gap-2">
        {TABS.map((item) => (
          <button
            key={item}
            type="button"
            role="tab"
            aria-selected={active === item}
            className="rounded-md border border-border px-2 py-1 text-sm"
            onClick={() => setParams({ tab: item })}
          >
            {item === "scan" ? t("dashboard.scan") : t(`install.tab.${item}`)}
          </button>
        ))}
      </div>
      {active === "leaderboard" ? <p className="mt-4 text-sm text-muted-foreground">{t("install.leaderboard.empty")}</p> : null}
      {active === "local" ? (
        <form
          className="mt-4 space-y-2"
          onSubmit={(event) => {
            event.preventDefault();
            onInstallLocal?.(folder);
            setInstalled(folder.split("/").filter(Boolean).pop() ?? folder);
          }}
        >
          <label className="block text-sm">
            {t("install.local.folder")}
            <input aria-label={t("install.local.folder")} value={folder} onChange={(event) => setFolder(event.target.value)} className="ml-2 rounded-md border border-input px-2 py-1" />
          </label>
          <label className="block text-sm">
            {t("install.local.archive")}
            <input aria-label={t("install.local.archive")} value={archive} onChange={(event) => setArchive(event.target.value)} className="ml-2 rounded-md border border-input px-2 py-1" />
          </label>
          <button type="submit" className="rounded-md bg-primary px-3 py-1 text-sm text-primary-foreground">{t("install.local.submit")}</button>
          <button
            type="button"
            className="ml-2 rounded-md border border-border px-3 py-1 text-sm"
            onClick={() => {
              onInstallArchive?.(archive);
              setInstalled(archive.split("/").filter(Boolean).pop() ?? archive);
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
            setPreview(true);
            setCancelled(false);
            onPreview?.(url, rename);
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
                    onPreview?.(url, rename);
                    setImported(rename);
                  }}
                >
                  {t("install.git.import")}
                </button>
                <button
                  type="button"
                  className="rounded-md border border-border px-2 py-1 text-sm"
                  onClick={() => {
                    setCancelled(true);
                    onCancel?.();
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
                      onImportOne?.(name);
                      setImported(name);
                    }}
                  >
                    {t("install.scan.one")} {row.name}
                  </button>
                </li>
              );
            })}
          </ul>
          <button type="button" className="mt-2 rounded-md bg-primary px-3 py-1 text-sm text-primary-foreground" onClick={() => onImportAll?.(scanned.map((row) => row.name))}>
            {t("install.scan.all")}
          </button>
        </div>
      ) : null}
      {imported ? <p>{t("install.git.imported")} {imported}</p> : null}
    </section>
  );
}
