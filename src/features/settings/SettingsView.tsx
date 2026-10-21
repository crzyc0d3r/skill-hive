import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { AgentName } from "@/components/app/AgentBadge";
import { Screen, StatePanel } from "@/components/chrome";
import { SettingsSkeleton } from "@/components/skeletons";
import { RetryContext, useShellPhase } from "@/lib/view-state";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useContext, useEffect, useState } from "react";
import { useSearchParams } from "react-router-dom";
import type { AgentRow } from "@/lib/ipc/catalog";
import { applyTextSize, setLocale, t, type TextSize } from "@/lib/i18n";
import { moveById } from "@/features/library/model";
import { EDITOR_SHORTCUTS } from "@/lib/authoring/model";
import { proxyOk } from "@/features/settings/proxy";
import { pickPath } from "@/lib/ipc/pick";
import { writeSetting } from "@/lib/ipc/catalog";
import {
  gitBackupRemoveRemote,
  gitBackupSetRemote,
  setAllToolsEnabled,
  setCustomToolPath,
  setCustomToolProjectPath,
  setToolEnabled,
  setToolOrder,
  removeCustomTool,
} from "@/lib/ipc/actions";
import { downloadAndInstallUpdate, exportDiagnostics, silent } from "@/lib/ipc/actions";
import type { ThemeChoice } from "@/lib/theme";
import { decideUpdate, desktopPlatform, LINUX_DOWNLOAD_URL } from "@/lib/updater";

const TABS = ["general", "appearance", "agents", "backup", "keyboard", "about"] as const;

export function SettingsView({
  theme,
  setTheme,
  agents,
  onAddAgent,
  onRepoPath,
  onAgents,
  onResetPath,
}: {
  theme: ThemeChoice;
  setTheme: (choice: ThemeChoice) => void;
  agents: AgentRow[];
  onAddAgent: (name: string, path: string) => void | Promise<void>;
  onRepoPath: (path: string) => void | Promise<void>;
  onAgents: (agents: AgentRow[]) => void | Promise<void>;
  onResetPath: (key: string) => Promise<string> | string;
}) {
  const phase = useShellPhase();
  const retry = useContext(RetryContext);
  const [params, setParams] = useSearchParams();
  const tab = TABS.includes((params.get("tab") ?? "appearance") as (typeof TABS)[number])
    ? ((params.get("tab") ?? "appearance") as (typeof TABS)[number])
    : "appearance";
  const [notice, setNotice] = useState<string | null>(null);
  const [path, setPath] = useState("~/.skill-hive");
  const [agentName, setAgentName] = useState("");
  const [agentPath, setAgentPath] = useState("");
  const [syncMode, setSyncMode] = useState<"symlink" | "copy">("symlink");
  const [updateNote, setUpdateNote] = useState<string | null>(null);
  const [updateAction, setUpdateAction] = useState<"install" | "download" | null>(null);
  const [refreshed, setRefreshed] = useState(false);
  const [agentRefresh, setAgentRefresh] = useState(false);
  const [showAllAgents, setShowAllAgents] = useState(false);
  const [dragKey, setDragKey] = useState<string | null>(null);
  const [motion, setMotion] = useState(false);
  const [proxy, setProxy] = useState("");
  const [proxyNote, setProxyNote] = useState<string | null>(null);
  const [closeChoice, setCloseChoice] = useState("ask");
  const [trayOn, setTrayOn] = useState(true);
  const [autoApply, setAutoApply] = useState(false);
  const [authMethod, setAuthMethod] = useState("device");
  const [mergeEngine, setMergeEngine] = useState("object");
  const [checkTtl, setCheckTtl] = useState("60");
  const [gitEngine, setGitEngine] = useState("git");
  const [exported, setExported] = useState(false);
  const [diagnosticsPath, setDiagnosticsPath] = useState("");
  const [restartArmed, setRestartArmed] = useState(false);
  const crash = typeof localStorage !== "undefined" && localStorage.getItem("skill-hive-panic") === "1";
  const repoWarn = typeof localStorage !== "undefined" && localStorage.getItem("skill-hive-repo-warn") === "1";
  const [versionLine, setVersionLine] = useState("");
  const [diagnostics, setDiagnostics] = useState<string | null>(null);
  const [panicExcerpt, setPanicExcerpt] = useState<string | null>(null);
  const [repoWarnings, setRepoWarnings] = useState<string[]>([]);
  useEffect(() => {
    void silent<string | null>("get_settings", { key: "ui_text_size" }).then((value) => {
      if (value === "small" || value === "medium" || value === "large" || value === "xl") applyTextSize(value);
    });
    void silent<string | null>("get_settings", { key: "ui_language" }).then((value) => {
      if (value) setLocale(value);
    });
    void silent<string>("check_app_update").then((value) => {
      if (typeof value === "string") setVersionLine(value);
    });
  }, []);
  useEffect(() => {
    if (tab !== "about") return;
    void silent<{ version: string; os: string; libraryPath: string }>("get_diagnostic_info").then((info) => {
      if (info?.version) setDiagnostics(`${info.version} · ${info.os} · ${info.libraryPath}`);
    });
    void silent<{ excerpt: string } | null>("check_last_panic").then((panic) => {
      if (panic?.excerpt) setPanicExcerpt(panic.excerpt);
    });
    void silent<string[]>("get_central_repo_warnings").then((warnings) => {
      if (Array.isArray(warnings)) setRepoWarnings(warnings);
    });
  }, [tab]);
  const choices: ThemeChoice[] = ["light", "dark", "system"];
  const sizes: TextSize[] = ["small", "medium", "large", "xl"];

  return (
    <Screen testId="settings" title="Settings">
      <div role="tablist" className="inline-flex max-w-full flex-wrap rounded-lg bg-muted p-0.5">
        {TABS.map((item) => (
          <Button key={item} type="button" role="tab" aria-selected={tab === item} variant="ghost" size="sm" className={`h-7 rounded-md px-3 shadow-none ${tab === item ? "bg-background text-foreground shadow-sm" : "text-muted-foreground hover:bg-transparent"}`} onClick={() => setParams({ tab: item })}>
            {t(`settings.${item}`)}
          </Button>
        ))}
      </div>
      {phase === "loading" ? <SettingsSkeleton /> : null}
      {phase === "error" ? <StatePanel tone="error" title={t("shell.error")} action={<Button type="button" size="sm" onClick={retry}>{t("banner.retry")}</Button>} /> : null}
      {phase === "ready" && tab === "keyboard" ? (
        <div className="mt-4">
          <h2 className="text-sm font-medium">{t("shortcuts.editor")}</h2>
          <Table className="mt-2">
            <TableHeader>
              <TableRow>
                <TableHead>Shortcut</TableHead>
                <TableHead>Action</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {EDITOR_SHORTCUTS.map((row) => (
                <TableRow key={row.id}>
                  <TableCell className="font-mono">{row.keys}</TableCell>
                  <TableCell>{row.action}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      ) : null}
      {phase === "ready" && tab === "appearance" ? (
        <div className="mt-4">
          <h2 className="text-sm font-medium">{t("settings.appearance")}</h2>
          <div role="group" aria-label="Theme" className="mt-2 flex gap-2">
            {choices.map((choice) => (
              <Button key={choice} type="button" size="sm" aria-pressed={theme === choice} variant={theme === choice ? "default" : "outline"} onClick={() => setTheme(choice)}>
                {t(`settings.theme.${choice}`)}
              </Button>
            ))}
          </div>
          <div className="mt-3 flex flex-wrap gap-2">
            <Button type="button" aria-pressed={motion} variant="outline" size="sm" onClick={() => { const next = !motion; setMotion(next); document.documentElement.dataset.reduceMotion = next ? "on" : "off"; }}>
              {t("settings.motion")}
            </Button>
            {sizes.map((size) => (
              <Button key={size} type="button" variant="outline" size="sm" onClick={() => { applyTextSize(size); void writeSetting("ui_text_size", size); }}>
                {t(`settings.text.${size}`)}
              </Button>
            ))}
          </div>
          <div className="mt-3 flex flex-wrap gap-2">
            <span className="text-sm">{t("settings.language")}</span>
            <Button type="button" variant="outline" size="sm" onClick={() => { setLocale("en"); void writeSetting("ui_language", "en"); }}>{t("settings.language.en")}</Button>
            <Button type="button" variant="outline" size="sm" onClick={() => { setLocale("zh-CN"); void writeSetting("ui_language", "zh-CN"); }}>{t("settings.language.zhCN")}</Button>
            <Button type="button" variant="outline" size="sm" onClick={() => { setLocale("zh-TW"); void writeSetting("ui_language", "zh-TW"); }}>{t("settings.language.zhTW")}</Button>
          </div>
        </div>
      ) : null}
      {phase === "ready" && tab === "general" ? (
        <form
          className="mt-4 space-y-2"
          onSubmit={(event) => {
            event.preventDefault();
            void onRepoPath(path);
            void silent<string[]>("get_central_repo_warnings").then((warnings) => {
              if (Array.isArray(warnings) && warnings.length) setRepoWarnings(warnings);
            });
            setNotice(t("settings.restart"));
          }}
        >
          <label className="block text-sm">
            {t("settings.path")}
            <Input aria-label={t("settings.path")} value={path} onChange={(event) => setPath(event.target.value)} className="h-8 mt-1 block w-full" />
          </label>
          <div className="flex gap-2 text-sm">
            <Button type="button" aria-pressed={syncMode === "symlink"} variant="outline" size="sm" onClick={() => { setSyncMode("symlink"); void writeSetting("sync_mode", "symlink"); }}>{t("settings.sync.symlink")}</Button>
            <Button type="button" aria-pressed={syncMode === "copy"} variant="outline" size="sm" onClick={() => { setSyncMode("copy"); void writeSetting("sync_mode", "copy"); }}>{t("settings.sync.copy")}</Button>
          </div>
          <Button type="submit" size="sm">Save path</Button>
          <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => { void silent("open_central_repo_folder"); }}>{t("settings.path.open")}</Button>
          <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => { setPath("~/.skill-hive"); void onRepoPath("~/.skill-hive"); }}>
            {t("settings.path.reset")}
          </Button>
          {notice ? <p role="status">{notice}</p> : null}
          <div className="mt-4 space-y-2 text-sm">
            <p>{t("settings.close")}</p>
            <Button type="button" aria-pressed={closeChoice === "ask"} variant="outline" size="sm" onClick={() => { setCloseChoice("ask"); void writeSetting("close_behavior", "ask"); void writeSetting("closeBehaviour", "ask"); }}>{t("close.dialog")}</Button>
            <Button type="button" aria-pressed={closeChoice === "minimize"} variant="outline" size="sm" className="ml-2" onClick={() => { const next = trayOn ? "minimize" : "ask"; setCloseChoice(next); void writeSetting("close_behavior", next); void writeSetting("closeBehaviour", next); }}>{t("settings.close.minimize")}</Button>
            <Button type="button" aria-pressed={closeChoice === "quit"} variant="outline" size="sm" className="ml-2" onClick={() => { setCloseChoice("quit"); void writeSetting("close_behavior", "quit"); void writeSetting("closeBehaviour", "quit"); }}>{t("settings.close.quit")}</Button>
            <label className="mt-2 block">
              <input type="checkbox" checked={trayOn} onChange={(event) => { setTrayOn(event.target.checked); void writeSetting("tray", event.target.checked ? "1" : "0"); void writeSetting("trayIcon", event.target.checked ? "on" : "off"); }} /> {t("settings.tray")}
            </label>
            <p>{t("settings.autoUpdate")}</p>
            <Button type="button" variant="outline" size="sm" onClick={() => { setAutoApply(false); void writeSetting("auto_update_interval", "off"); void writeSetting("auto_update_apply", "0"); }}>{t("settings.autoUpdate.off")}</Button>
            <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => { setAutoApply(true); void writeSetting("auto_update_interval", "daily"); }}>{t("settings.autoUpdate.daily")}</Button>
            <label className="mt-2 block">
              <input type="checkbox" checked={autoApply} onChange={(event) => { setAutoApply(event.target.checked); void writeSetting("auto_update_apply", event.target.checked ? "1" : "0"); }} /> {t("settings.autoUpdate.apply")}
            </label>
            <label className="mt-2 block">
              {t("settings.ttl")}
              <Input aria-label={t("settings.ttl")} value={checkTtl} onChange={(event) => setCheckTtl(event.target.value)} onBlur={() => { const minutes = Number(checkTtl); if (Number.isFinite(minutes) && minutes > 0) void writeSetting("update_check_ttl_minutes", String(Math.round(minutes))); }} className="mt-1 h-8 max-w-32" />
            </label>
            <div>
              <label className="block">
                {t("settings.proxy")}
                <Input aria-label={t("settings.proxy")} value={proxy} onChange={(event) => setProxy(event.target.value)} className="h-8 ml-2" />
              </label>
              <Button type="button" variant="outline" size="sm" className="mt-2" onClick={() => { const ok = proxyOk(proxy); setProxyNote(ok ? t("settings.proxy.save") : t("settings.proxy.invalid")); if (ok) { void writeSetting("proxy_url", proxy.trim()); void writeSetting("proxy", proxy.trim()); } }}>{t("settings.proxy.save")}</Button>
              {proxyNote ? <p role="status">{proxyNote}</p> : null}
            </div>
          </div>
        </form>
      ) : null}
      {phase === "ready" && tab === "agents" ? (
        <div className="mt-4">
          <div className="flex flex-wrap gap-2">
            <Button type="button" variant="outline" size="sm" onClick={() => { void onAgents(agents.map((agent) => ({ ...agent, enabled: true }))); void setAllToolsEnabled(true); }}>{t("settings.agents.enableAll")}</Button>
            <Button type="button" variant="outline" size="sm" onClick={() => { void onAgents(agents.map((agent) => ({ ...agent, enabled: false }))); void setAllToolsEnabled(false); }}>{t("settings.agents.disableAll")}</Button>
            <Button type="button" variant="outline" size="sm" disabled={agentRefresh} aria-busy={agentRefresh} onClick={() => { setAgentRefresh(true); setRefreshed(true); void silent<string[]>("get_tool_order_cmd").finally(() => setAgentRefresh(false)); }}>{t("settings.agents.refresh")}</Button>
          </div>
          {refreshed ? <p role="status">{t("settings.agents.refreshed")}</p> : null}
          <form
            className="mt-3 flex flex-wrap items-end gap-2"
            onSubmit={(event) => {
              event.preventDefault();
              void onAddAgent(agentName, agentPath);
            }}
          >
            <label className="block text-sm">
              {t("settings.agents.name")}
              <Input aria-label={t("settings.agents.name")} placeholder={t("settings.agents.namePlaceholder")} value={agentName} onChange={(event) => setAgentName(event.target.value)} className="mt-1 h-8 w-56" />
            </label>
            <label className="block text-sm">
              {t("settings.agents.path")}
              <Input aria-label={t("settings.agents.path")} placeholder={t("settings.agents.pathPlaceholder")} value={agentPath} onChange={(event) => setAgentPath(event.target.value)} className="mt-1 h-8 w-56" />
            </label>
            <Button type="button" variant="outline" size="sm" onClick={() => void pickPath(true).then((selected) => selected && setAgentPath(selected))}>Browse</Button>
            <Button type="submit" size="sm">{t("settings.agents.new")}</Button>
          </form>
          <Button type="button" variant="outline" size="sm" className="mt-3" aria-pressed={showAllAgents} onClick={() => setShowAllAgents((current) => !current)}>{t("settings.agents.showAll")}</Button>
          <p className="mt-2 text-sm text-muted-foreground">{showAllAgents ? t("settings.agents.showingAll") : t("settings.agents.detected")}</p>
          <ul data-testid="agent-order" className="mt-3 space-y-2">
            {(showAllAgents ? agents : agents.filter((agent) => agent.installed)).map((agent) => (
              <li
                key={agent.key}
                draggable
                data-agent={agent.key}
                onDragStart={() => setDragKey(agent.key)}
                onDragOver={(event) => event.preventDefault()}
                onDrop={() => {
                  if (!dragKey) return;
                  const next = moveById(agents, dragKey, agent.key, (item) => item.key);
                  void onAgents(next);
                  void setToolOrder(next.map((item) => item.key));
                  void writeSetting("agent_order", next.map((item) => item.key).join(","));
                  setDragKey(null);
                }}
                className="rounded-md border border-border p-2 text-sm"
              >
                <AgentName agent={agent.key} displayName={agent.name} />
                <label className="ml-2 text-sm">
                  <input type="checkbox" aria-label={`${t("settings.agents.enable")} ${agent.name}`} checked={agent.enabled} onChange={(event) => { const enabled = event.target.checked; void onAgents(agents.map((item) => item.key === agent.key ? { ...item, enabled } : item)); void setToolEnabled(agent.key, enabled); }} /> {t("settings.agents.enable")}
                </label>
                <span className="ml-2 text-muted-foreground">{agent.enabled ? "Enabled" : "Disabled"}</span>
                <span className="ml-2 text-muted-foreground">{agent.installed ? "Detected" : "Other supported"}</span>
                <div className="mt-2 grid gap-2">
                <label className="grid grid-cols-[7.5rem_minmax(0,1fr)] items-center gap-2">
                  <span>{t("settings.agents.global")}</span>
                  <Input aria-label={`${t("settings.agents.global")} ${agent.name}`} value={agent.path} onChange={(event) => { const pathValue = event.target.value; void onAgents(agents.map((item) => item.key === agent.key ? { ...item, path: pathValue } : item)); void setCustomToolPath(agent.key, pathValue); }} className="h-8 w-full" />
                </label>
                <label className="grid grid-cols-[7.5rem_minmax(0,1fr)] items-center gap-2">
                  <span>{t("settings.agents.project")}</span>
                  <Input aria-label={`${t("settings.agents.project")} ${agent.name}`} value={agent.projectPath} onChange={(event) => { const pathValue = event.target.value; void onAgents(agents.map((item) => item.key === agent.key ? { ...item, projectPath: pathValue } : item)); void setCustomToolProjectPath(agent.key, pathValue); }} className="h-8 w-full" />
                </label>
                </div>
                <Button type="button" variant="outline" size="sm" className="mt-1" onClick={() => { void silent("reset_custom_tool_project_path", { key: agent.key }); Promise.resolve(onResetPath(agent.key)).then((next) => { if (next) return onAgents(agents.map((item) => item.key === agent.key ? { ...item, path: next } : item)); }); }}>{t("settings.agents.reset")}</Button>
                {agent.custom ? (
                  <Button type="button" variant="outline" size="sm" className="ml-2 mt-1" onClick={() => { void onAgents(agents.filter((item) => item.key !== agent.key)); void removeCustomTool(agent.key); }}>{t("settings.agents.remove")} {agent.name}</Button>
                ) : null}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
      {phase === "ready" && tab === "backup" ? (
        <div className="mt-4 space-y-2 text-sm">
          <p>{t("backup.advanced")}</p>
          <label className="block">
            {t("backup.remote")}
            <Input aria-label={t("backup.remote")} className="h-8 ml-2" onBlur={(event) => { if (event.target.value.trim()) void gitBackupSetRemote(event.target.value.trim()); }} />
          </label>
          <Button type="button" variant="outline" size="sm" onClick={() => void gitBackupRemoveRemote()}>{t("backup.disconnect")}</Button>
          <Button type="button" aria-pressed={gitEngine === "git2"} variant="outline" size="sm" className="ml-2" onClick={() => { setGitEngine("git2"); void writeSetting("git_backup_engine", "git2"); }}>{t("settings.engine.libgit2")}</Button>
          <Button type="button" aria-pressed={gitEngine === "git"} variant="outline" size="sm" className="ml-2" onClick={() => { setGitEngine("git"); void writeSetting("git_backup_engine", "git"); }}>{t("settings.engine.git")}</Button>
          <div className="flex flex-wrap gap-2">
            <Button type="button" aria-pressed={authMethod === "device"} variant="outline" size="sm" onClick={() => { setAuthMethod("device"); void writeSetting("github_auth_method", "device"); }}>{t("settings.auth.device")}</Button>
            <Button type="button" aria-pressed={authMethod === "pat"} variant="outline" size="sm" onClick={() => { setAuthMethod("pat"); void writeSetting("github_auth_method", "pat"); }}>{t("settings.auth.pat")}</Button>
          </div>
          <div className="flex flex-wrap gap-2">
            <span>{t("settings.merge")}</span>
            <Button type="button" aria-pressed={mergeEngine === "object"} variant="outline" size="sm" onClick={() => { setMergeEngine("object"); void writeSetting("merge_engine", "object"); }}>{t("settings.merge.object")}</Button>
            <Button type="button" aria-pressed={mergeEngine === "system"} variant="outline" size="sm" onClick={() => { setMergeEngine("system"); void writeSetting("merge_engine", "system"); }}>{t("settings.engine.git")}</Button>
          </div>
          <p className="text-muted-foreground">{t("settings.engine.note")}</p>
        </div>
      ) : null}
      {phase === "ready" && tab === "about" ? (
        <div className="mt-4 text-sm">
          <p>{t("settings.about.version")} {versionLine}</p>
          {diagnostics ? <p>{diagnostics}</p> : null}
          <Button type="button" variant="outline" size="sm" className="mt-2" onClick={() => { const remote = localStorage.getItem("skill-hive-update-version"); const runs = Number(localStorage.getItem("skill-hive-active-runs") ?? "0"); const decision = decideUpdate({ remoteVersion: remote, platform: desktopPlatform(navigator.userAgent, localStorage.getItem("skill-hive-platform")), activeRuns: Number.isFinite(runs) ? runs : 0, }); setUpdateNote(decision.state === "available" ? t(decision.messageKey, { version: decision.version }) : t(decision.messageKey)); setUpdateAction(decision.state === "available" ? decision.action : null); void downloadAndInstallUpdate(); if (decision.state === "available" && decision.action === "install") { localStorage.setItem("skill-hive-pending-install", decision.version); } else { localStorage.removeItem("skill-hive-pending-install"); } }}>
            {t("settings.about.updates")}
          </Button>
          {updateNote ? <p role="status">{updateNote}</p> : null}
          {localStorage.getItem("skill-hive-pending-install") && desktopPlatform(navigator.userAgent, localStorage.getItem("skill-hive-platform")) !== "linux" ? (
            <Button type="button" variant="outline" size="sm" className="mt-2" data-proxy={proxyOk(proxy) && proxy.trim() ? proxy.trim() : ""} onClick={() => { localStorage.setItem("skill-hive-restart-now", "1"); setRestartArmed(true); window.dispatchEvent(new Event("skill-hive-restart")); void silent("restart_app"); }}>
              {t("settings.about.install")}
            </Button>
          ) : null}
          {updateAction === "download" ? (
            <a className="mt-2 inline-block rounded-md border border-border px-2 py-1" href={LINUX_DOWNLOAD_URL}>{t("settings.about.download")}</a>
          ) : null}
          {crash || panicExcerpt ? <p role="alert">{panicExcerpt ?? t("settings.about.crash")}</p> : null}
          {repoWarn || repoWarnings.length > 0 ? <p role="alert">{repoWarnings[0] ?? t("settings.about.repo")}</p> : null}
          <a className="mt-2 inline-block rounded-md border border-border px-2 py-1" href={`https://github.com/crzyc0d3r/skill-hive/issues/new?body=${encodeURIComponent(diagnostics ?? diagnosticsPath)}`} target="_blank" rel="noreferrer" onClick={() => { const path = diagnostics ?? diagnosticsPath; if (path && navigator.clipboard) void navigator.clipboard.writeText(path); }}>{t("settings.about.report")}</a>
          <label className="mt-2 block">
            Diagnostics path
            <Input aria-label="Diagnostics path" value={diagnosticsPath} onChange={(event) => setDiagnosticsPath(event.target.value)} className="h-8 ml-2" />
          </label>
          <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => { const logs = diagnosticsPath ? `${diagnosticsPath}.logs.zip` : ""; void silent("export_logs_zip", { path: logs }); void exportDiagnostics(diagnosticsPath).then(() => setExported(true)); }}>{t("settings.about.export")}</Button>
          {exported ? <p role="status">{t("settings.about.exported")}</p> : null}
          {restartArmed ? <p role="status"><button type="button">{t("settings.about.restart")}</button></p> : null}
          <h2 className="mt-4 text-sm font-medium">{t("settings.about.ack")}</h2>
          <p className="mt-1 text-muted-foreground">{t("settings.about.mit")}</p>
        </div>
      ) : null}
    </Screen>
  );
}
