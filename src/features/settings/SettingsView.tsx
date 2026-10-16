import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Screen } from "@/components/chrome";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useState } from "react";
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
  setToolOrder,
  removeCustomTool,
} from "@/lib/ipc/actions";
import { downloadAndInstallUpdate, exportDiagnostics } from "@/lib/ipc/actions";
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
  const [dragKey, setDragKey] = useState<string | null>(null);
  const [motion, setMotion] = useState(false);
  const [proxy, setProxy] = useState("");
  const [proxyNote, setProxyNote] = useState<string | null>(null);
  const [closeChoice, setCloseChoice] = useState("ask");
  const [trayOn, setTrayOn] = useState(true);
  const [autoApply, setAutoApply] = useState(false);
  const [exported, setExported] = useState(false);
  const [diagnosticsPath, setDiagnosticsPath] = useState("");
  const [restartArmed, setRestartArmed] = useState(false);
  const crash = typeof localStorage !== "undefined" && localStorage.getItem("skill-hive-panic") === "1";
  const repoWarn = typeof localStorage !== "undefined" && localStorage.getItem("skill-hive-repo-warn") === "1";
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
      {tab === "keyboard" ? (
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
      {tab === "appearance" ? (
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
              <Button key={size} type="button" variant="outline" size="sm" onClick={() => applyTextSize(size)}>
                {size === "xl" ? t("settings.text.xl") : size}
              </Button>
            ))}
          </div>
          <div className="mt-3 flex flex-wrap gap-2">
            <span className="text-sm">{t("settings.language")}</span>
            <Button type="button" variant="outline" size="sm" onClick={() => setLocale("en")}>{t("settings.language.en")}</Button>
            <Button type="button" variant="outline" size="sm" onClick={() => setLocale("zh-CN")}>{t("settings.language.zhCN")}</Button>
            <Button type="button" variant="outline" size="sm" onClick={() => setLocale("zh-TW")}>{t("settings.language.zhTW")}</Button>
          </div>
        </div>
      ) : null}
      {tab === "general" ? (
        <form
          className="mt-4 space-y-2"
          onSubmit={(event) => {
            event.preventDefault();
            void onRepoPath(path);
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
          <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => { setPath("~/.skill-hive"); void onRepoPath("~/.skill-hive"); }}>
            {t("settings.path.reset")}
          </Button>
          {notice ? <p role="status">{notice}</p> : null}
          <div className="mt-4 space-y-2 text-sm">
            <p>{t("settings.close")}</p>
            <Button type="button" aria-pressed={closeChoice === "ask"} variant="outline" size="sm" onClick={() => { setCloseChoice("ask"); void writeSetting("close_behavior", "ask"); }}>{t("close.dialog")}</Button>
            <Button type="button" aria-pressed={closeChoice === "minimize"} variant="outline" size="sm" className="ml-2" onClick={() => { const next = trayOn ? "minimize" : "ask"; setCloseChoice(next); void writeSetting("close_behavior", next); }}>{t("settings.close.minimize")}</Button>
            <Button type="button" aria-pressed={closeChoice === "quit"} variant="outline" size="sm" className="ml-2" onClick={() => { setCloseChoice("quit"); void writeSetting("close_behavior", "quit"); }}>{t("settings.close.quit")}</Button>
            <label className="mt-2 block">
              <input type="checkbox" checked={trayOn} onChange={(event) => { setTrayOn(event.target.checked); void writeSetting("tray", event.target.checked ? "1" : "0"); }} /> {t("settings.tray")}
            </label>
            <p>{t("settings.autoUpdate")}</p>
            <Button type="button" variant="outline" size="sm" onClick={() => setAutoApply(false)}>{t("settings.autoUpdate.off")}</Button>
            <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => setAutoApply(true)}>{t("settings.autoUpdate.daily")}</Button>
            <label className="mt-2 block">
              <input type="checkbox" checked={autoApply} onChange={(event) => setAutoApply(event.target.checked)} /> {t("settings.autoUpdate.apply")}
            </label>
            <div>
              <label className="block">
                {t("settings.proxy")}
                <Input aria-label={t("settings.proxy")} value={proxy} onChange={(event) => setProxy(event.target.value)} className="h-8 ml-2" />
              </label>
              <Button type="button" variant="outline" size="sm" className="mt-2" onClick={() => { const ok = proxyOk(proxy); setProxyNote(ok ? t("settings.proxy.save") : t("settings.proxy.invalid")); if (ok) void writeSetting("proxy", proxy.trim()); }}>{t("settings.proxy.save")}</Button>
              {proxyNote ? <p role="status">{proxyNote}</p> : null}
            </div>
          </div>
        </form>
      ) : null}
      {tab === "agents" ? (
        <div className="mt-4">
          <div className="flex flex-wrap gap-2">
            <Button type="button" variant="outline" size="sm" onClick={() => { void onAgents(agents.map((agent) => ({ ...agent, enabled: true }))); void setAllToolsEnabled(true); }}>{t("settings.agents.enableAll")}</Button>
            <Button type="button" variant="outline" size="sm" onClick={() => { void onAgents(agents.map((agent) => ({ ...agent, enabled: false }))); void setAllToolsEnabled(false); }}>{t("settings.agents.disableAll")}</Button>
            <Button type="button" variant="outline" size="sm" onClick={() => setRefreshed(true)}>{t("settings.agents.refresh")}</Button>
          </div>
          {refreshed ? <p role="status">{t("settings.agents.refreshed")}</p> : null}
          <form
            className="mt-3 flex flex-wrap items-center gap-2"
            onSubmit={(event) => {
              event.preventDefault();
              void onAddAgent(agentName, agentPath);
            }}
          >
            <Input aria-label="Agent name" value={agentName} onChange={(event) => setAgentName(event.target.value)} className="h-8 w-56 shrink-0" />
            <Input aria-label="Agent path" value={agentPath} onChange={(event) => setAgentPath(event.target.value)} className="h-8 w-56 shrink-0" />
            <Button type="button" variant="outline" size="sm" onClick={() => void pickPath(true).then((selected) => selected && setAgentPath(selected))}>Browse</Button>
            <Button type="submit" size="sm">{t("settings.agents.new")}</Button>
          </form>
          <ul data-testid="agent-order" className="mt-3 space-y-2">
            {agents.map((agent) => (
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
                  setDragKey(null);
                }}
                className="rounded-md border border-border p-2 text-sm"
              >
                <span className="font-medium">{agent.name}</span>
                <span className="ml-2 text-muted-foreground">{agent.enabled ? "Enabled" : "Disabled"}</span>
                <span className="ml-2 text-muted-foreground">{agent.installed ? "Detected" : "Other supported"}</span>
                <label className="mt-1 block">
                  {t("settings.agents.reset")}
                  <Input aria-label={`${t("settings.agents.reset")} ${agent.name}`} value={agent.path} onChange={(event) => { const pathValue = event.target.value; void onAgents(agents.map((item) => item.key === agent.key ? { ...item, path: pathValue } : item)); void setCustomToolPath(agent.key, pathValue); }} className="h-8 ml-2" />
                </label>
                <label className="mt-1 block">
                  {t("settings.agents.project")}
                  <Input aria-label={`${t("settings.agents.project")} ${agent.name}`} value={agent.projectPath} onChange={(event) => { const pathValue = event.target.value; void onAgents(agents.map((item) => item.key === agent.key ? { ...item, projectPath: pathValue } : item)); void setCustomToolProjectPath(agent.key, pathValue); }} className="h-8 ml-2" />
                </label>
                <Button type="button" variant="outline" size="sm" className="mt-1" onClick={() => Promise.resolve(onResetPath(agent.key)).then((next) => { if (next) return onAgents(agents.map((item) => item.key === agent.key ? { ...item, path: next } : item)); })}>{t("settings.agents.reset")}</Button>
                {agent.custom ? (
                  <Button type="button" variant="outline" size="sm" className="ml-2 mt-1" onClick={() => { void onAgents(agents.filter((item) => item.key !== agent.key)); void removeCustomTool(agent.key); }}>{t("settings.agents.remove")} {agent.name}</Button>
                ) : null}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
      {tab === "backup" ? (
        <div className="mt-4 space-y-2 text-sm">
          <p>{t("backup.advanced")}</p>
          <label className="block">
            {t("backup.remote")}
            <Input aria-label={t("backup.remote")} className="h-8 ml-2" onBlur={(event) => { if (event.target.value.trim()) void gitBackupSetRemote(event.target.value.trim()); }} />
          </label>
          <Button type="button" variant="outline" size="sm" onClick={() => void gitBackupRemoveRemote()}>{t("backup.disconnect")}</Button>
          <Button type="button" disabled variant="outline" size="sm" className="ml-2">{t("settings.engine.libgit2")}</Button>
          <p className="text-muted-foreground">{t("settings.engine.note")}</p>
        </div>
      ) : null}
      {tab === "about" ? (
        <div className="mt-4 text-sm">
          <p>{t("settings.about.version")}</p>
          <Button type="button" variant="outline" size="sm" className="mt-2" onClick={() => { const remote = localStorage.getItem("skill-hive-update-version"); const runs = Number(localStorage.getItem("skill-hive-active-runs") ?? "0"); const decision = decideUpdate({ remoteVersion: remote, platform: desktopPlatform(navigator.userAgent, localStorage.getItem("skill-hive-platform")), activeRuns: Number.isFinite(runs) ? runs : 0, }); setUpdateNote(decision.state === "available" ? t(decision.messageKey, { version: decision.version }) : t(decision.messageKey)); setUpdateAction(decision.state === "available" ? decision.action : null); void downloadAndInstallUpdate(); if (decision.state === "available" && decision.action === "install") { localStorage.setItem("skill-hive-pending-install", decision.version); } else { localStorage.removeItem("skill-hive-pending-install"); } }}>
            {t("settings.about.updates")}
          </Button>
          {updateNote ? <p role="status">{updateNote}</p> : null}
          {localStorage.getItem("skill-hive-pending-install") && desktopPlatform(navigator.userAgent, localStorage.getItem("skill-hive-platform")) !== "linux" ? (
            <Button type="button" variant="outline" size="sm" className="mt-2" data-proxy={proxyOk(proxy) && proxy.trim() ? proxy.trim() : ""} onClick={() => { localStorage.setItem("skill-hive-restart-now", "1"); setRestartArmed(true); window.dispatchEvent(new Event("skill-hive-restart")); }}>
              {t("settings.about.install")}
            </Button>
          ) : null}
          {updateAction === "download" ? (
            <a className="mt-2 inline-block rounded-md border border-border px-2 py-1" href={LINUX_DOWNLOAD_URL}>{t("settings.about.download")}</a>
          ) : null}
          {crash ? <p role="alert">{t("settings.about.crash")}</p> : null}
          {repoWarn ? <p role="alert">{t("settings.about.repo")}</p> : null}
          <a className="mt-2 inline-block rounded-md border border-border px-2 py-1" href="https://github.com/crzyc0d3r/skill-hive/issues/new" target="_blank" rel="noreferrer">{t("settings.about.report")}</a>
          <label className="mt-2 block">
            Diagnostics path
            <Input aria-label="Diagnostics path" value={diagnosticsPath} onChange={(event) => setDiagnosticsPath(event.target.value)} className="h-8 ml-2" />
          </label>
          <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => { void exportDiagnostics(diagnosticsPath).then(() => setExported(true)); }}>{t("settings.about.export")}</Button>
          {exported ? <p role="status">{t("settings.about.exported")}</p> : null}
          {restartArmed ? <p role="status"><button type="button">{t("settings.about.restart")}</button></p> : null}
          <h2 className="mt-4 text-sm font-medium">{t("settings.about.ack")}</h2>
          <p className="mt-1 text-muted-foreground">{t("settings.about.mit")}</p>
        </div>
      ) : null}
    </Screen>
  );
}
