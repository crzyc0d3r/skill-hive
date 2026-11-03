import * as React from "react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { AlertTriangle, BookOpen, Bug, CheckCircle2, Eraser, ExternalLink, GitFork, Hexagon, KeyRound, RefreshCw, XCircle } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation, useAppInfo, useDoctor, useSettings } from "@/lib/api/hooks";
import { cn, displayPath, formatBytes, pluralize } from "@/lib/utils";
import { Badge, Button, Input, Skeleton, openExternal } from "@/components/ui";
import { ErrorNote, InlineCode } from "@/components/app/common";
import { SettingsRow, SettingsSection } from "./bits";
import { DOCS_URL, ISSUES_URL, PROVIDER_KIND_META, REPO_URL, useSettingsPatch } from "./shared";

export function AboutTab() {
  const { data: info } = useAppInfo();
  const { data: settings } = useSettings();
  const doctor = useDoctor();
  const qc = useQueryClient();
  const save = useSettingsPatch();
  const [token, setToken] = React.useState("");

  const reindex = useApiMutation(() => api.doctor(true), [keys.itemsAll, keys.stats, keys.dashboard, keys.deployments], (report) => {
    qc.setQueryData(keys.doctor, report);
    toast.success("Library re-indexed", { description: `${pluralize(report.libraryItems, "item")} in the index` });
  });
  const clearCache = useApiMutation(() => api.clearSourceCache(), [], (bytes) => toast.success("Source cache cleared", { description: `Freed ${formatBytes(bytes)}` }));

  return (
    <div className="flex max-w-3xl flex-col gap-5">
      <section className="surface flex flex-wrap items-center gap-4 px-4 py-4">
        <div className="flex size-10 items-center justify-center rounded-xl bg-accent text-accent-fg shadow-sm">
          <Hexagon className="size-5" strokeWidth={2.4} />
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <h2 className="text-sm font-semibold tracking-[-0.01em]">Skill Hive</h2>
            <Badge tone="neutral" className="num">
              {info ? `v${info.version}` : "…"}
            </Badge>
          </div>
          <p className="mt-0.5 text-xs text-muted">
            One library for every skill, subagent and command you use.{info ? ` ${info.platform} · ${info.arch}.` : ""}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-1.5">
          <Button size="sm" variant="secondary" onClick={() => void openExternal(DOCS_URL)}>
            <BookOpen /> Documentation
          </Button>
          <Button size="sm" variant="secondary" onClick={() => void openExternal(REPO_URL)}>
            <GitFork /> GitHub
          </Button>
          <Button size="sm" variant="ghost" onClick={() => void openExternal(ISSUES_URL)}>
            <Bug /> Report an issue
          </Button>
        </div>
      </section>

      <SettingsSection
        title="Health check"
        description="The same report as hive doctor: database, library files, deployments and tools."
        action={
          <>
            <Button size="sm" variant="ghost" loading={doctor.isFetching && !reindex.isPending} onClick={() => doctor.refetch()}>
              <RefreshCw /> Refresh
            </Button>
            <Button size="sm" variant="secondary" loading={reindex.isPending} onClick={() => reindex.mutate(undefined)}>
              <RefreshCw /> Re-index library
            </Button>
          </>
        }
      >
        {doctor.isLoading ? (
          <div className="flex flex-col gap-2 p-4">
            <Skeleton className="h-4 w-2/3" />
            <Skeleton className="h-4 w-1/2" />
            <Skeleton className="h-4 w-3/5" />
          </div>
        ) : doctor.error ? (
          <div className="p-4">
            <ErrorNote error={doctor.error} />
          </div>
        ) : doctor.data ? (
          <DoctorRows report={doctor.data} />
        ) : null}
        <ErrorNote error={reindex.error} className="m-3" />
      </SettingsSection>

      <SettingsSection title="GitHub" description="A personal access token raises the API rate limit for searches and update checks and allows private repositories.">
        <SettingsRow
          label={
            <span className="flex items-center gap-2">
              Token
              {settings ? <Badge tone={settings.githubTokenSet ? "ok" : "neutral"}>{settings.githubTokenSet ? "set" : "not set"}</Badge> : null}
            </span>
          }
          description="Stored in the OS keychain. Only repo:read scope is needed."
        >
          <div className="relative">
            <KeyRound className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-subtle" />
            <Input
              type="password"
              value={token}
              onChange={(e) => setToken(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && token.trim()) {
                  save.mutate({ patch: { githubToken: token.trim() }, message: "GitHub token saved" });
                  setToken("");
                }
              }}
              placeholder={settings?.githubTokenSet ? "Replace token…" : "ghp_…"}
              aria-label="GitHub token"
              autoComplete="off"
              className="mono w-64 pl-8"
            />
          </div>
          <Button
            size="sm"
            variant="primary"
            disabled={!token.trim()}
            loading={save.isPending && !!save.variables?.patch.githubToken}
            onClick={() => {
              save.mutate({ patch: { githubToken: token.trim() }, message: "GitHub token saved" });
              setToken("");
            }}
          >
            Save
          </Button>
          {settings?.githubTokenSet ? (
            <Button size="sm" variant="ghost" onClick={() => save.mutate({ patch: { githubToken: "" }, message: "GitHub token removed" })}>
              Clear
            </Button>
          ) : null}
        </SettingsRow>
      </SettingsSection>

      <SettingsSection title="Storage">
        <SettingsRow label="Source cache" description={<>Cloned repositories and downloaded archives under <InlineCode>{displayPath(`${settings?.home ?? "~/.skill-hive"}/cache`)}</InlineCode>. Safe to clear; sources are fetched again on demand.</>}>
          <Button size="sm" variant="secondary" loading={clearCache.isPending} onClick={() => clearCache.mutate(undefined)}>
            <Eraser /> Clear source cache
          </Button>
        </SettingsRow>
        <ErrorNote error={clearCache.error ?? save.error} className="m-3" />
      </SettingsSection>
    </div>
  );
}

function Row({ ok, label, value, detail, warn }: { ok: boolean; label: string; value: React.ReactNode; detail?: React.ReactNode; warn?: boolean }) {
  return (
    <div className="flex items-start gap-3 px-4 py-2.5 text-xs">
      {ok ? <CheckCircle2 className="mt-0.5 size-3.5 shrink-0 text-ok" /> : warn ? <AlertTriangle className="mt-0.5 size-3.5 shrink-0 text-warn" /> : <XCircle className="mt-0.5 size-3.5 shrink-0 text-danger" />}
      <span className="w-40 shrink-0 text-muted">{label}</span>
      <span className="min-w-0 flex-1">
        <span className={cn("font-medium", !ok && !warn && "text-danger")}>{value}</span>
        {detail ? <span className="mt-1 block text-subtle">{detail}</span> : null}
      </span>
    </div>
  );
}

function DoctorRows({ report }: { report: T.DoctorReport }) {
  const list = (items: string[]) => (items.length ? <span className="mono selectable block break-all">{items.map(displayPath).join(", ")}</span> : null);
  return (
    <>
      <Row ok={report.dbOk} label="Database" value={report.dbOk ? "OK" : "Not readable"} detail={<span className="mono">{displayPath(report.home)}/hive.db</span>} />
      <Row ok label="Library items" value={report.libraryItems.toLocaleString()} />
      <Row ok={report.orphanDirs.length === 0} warn label="Orphan directories" value={report.orphanDirs.length ? `${report.orphanDirs.length} in the library but not in the index` : "None"} detail={list(report.orphanDirs)} />
      <Row ok={report.missingDirs.length === 0} warn label="Missing directories" value={report.missingDirs.length ? `${report.missingDirs.length} indexed but gone from disk` : "None"} detail={list(report.missingDirs)} />
      <Row
        ok={report.brokenDeployments.length === 0}
        warn
        label="Broken deployments"
        value={report.brokenDeployments.length ? pluralize(report.brokenDeployments.length, "broken link") : "None"}
        detail={report.brokenDeployments.length ? <span className="mono selectable block break-all">{report.brokenDeployments.map((d) => `${d.itemName} → ${displayPath(d.path)}`).join(", ")}</span> : null}
      />
      <Row ok={report.gitAvailable} warn label="Git" value={report.gitAvailable ? "Available" : "Not found"} detail={report.gitAvailable ? null : "Needed to collect from git repositories."} />
      <Row
        ok={report.detectedClis.length > 0}
        warn
        label="Agent CLIs"
        value={report.detectedClis.length ? report.detectedClis.map((c) => `${PROVIDER_KIND_META[c.kind].label}${c.version ? ` ${c.version}` : ""}`).join(", ") : "None detected"}
        detail={report.detectedClis.length ? <span className="mono">{report.detectedClis.map((c) => c.path).join(", ")}</span> : "Install Claude Code or Codex to run evaluations."}
      />
      {report.notes.length ? (
        <div className="flex items-start gap-3 px-4 py-2.5 text-xs">
          <ExternalLink className="mt-0.5 size-3.5 shrink-0 text-subtle" />
          <span className="w-40 shrink-0 text-muted">Notes</span>
          <ul className="min-w-0 flex-1 list-disc pl-4 text-muted">
            {report.notes.map((n, i) => (
              <li key={i} className="selectable">
                {n}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </>
  );
}
