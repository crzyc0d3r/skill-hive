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
    <section data-testid="settings">
      <h1 className="font-mono text-[22px] font-semibold">Settings</h1>
      <div role="tablist" className="mt-3 flex gap-2">
        {TABS.map((item) => (
          <button key={item} type="button" role="tab" aria-selected={tab === item} className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => setParams({ tab: item })}>
            {t(`settings.${item}`)}
          </button>
        ))}
      </div>
      {tab === "keyboard" ? (
        <div className="mt-4">
          <h2 className="text-sm font-medium">{t("shortcuts.editor")}</h2>
          <table className="mt-2 text-sm">
            <tbody>
              {EDITOR_SHORTCUTS.map((row) => (
                <tr key={row.id}>
                  <td className="pr-4">{row.keys}</td>
                  <td>{row.action}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}
      {tab === "appearance" ? (
        <div className="mt-4">
          <h2 className="text-sm font-medium">{t("settings.appearance")}</h2>
          <div role="group" aria-label="Theme" className="mt-2 flex gap-2">
            {choices.map((choice) => (
              <button
                key={choice}
                type="button"
                aria-pressed={theme === choice}
                className={`rounded-md border px-3 py-1 text-sm ${theme === choice ? "border-ring bg-primary text-primary-foreground" : "border-border bg-secondary text-secondary-foreground"}`}
                onClick={() => setTheme(choice)}
              >
                {t(`settings.theme.${choice}`)}
              </button>
            ))}
          </div>
          <div className="mt-3 flex flex-wrap gap-2">
            <button
              type="button"
              aria-pressed={motion}
              className="rounded-md border border-border px-2 py-1 text-sm"
              onClick={() => {
                const next = !motion;
                setMotion(next);
                document.documentElement.dataset.reduceMotion = next ? "on" : "off";
              }}
            >
              {t("settings.motion")}
            </button>
            {sizes.map((size) => (
              <button
                key={size}
                type="button"
                className="rounded-md border border-border px-2 py-1 text-sm"
                onClick={() => applyTextSize(size)}
              >
                {size === "xl" ? t("settings.text.xl") : size}
              </button>
            ))}
          </div>
          <div className="mt-3 flex flex-wrap gap-2">
            <span className="text-sm">{t("settings.language")}</span>
            <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => setLocale("en")}>{t("settings.language.en")}</button>
            <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => setLocale("zh-CN")}>{t("settings.language.zhCN")}</button>
            <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => setLocale("zh-TW")}>{t("settings.language.zhTW")}</button>
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
            <input aria-label={t("settings.path")} value={path} onChange={(event) => setPath(event.target.value)} className="mt-1 block w-full rounded-md border border-input px-2 py-1" />
          </label>
          <div className="flex gap-2 text-sm">
            <button type="button" aria-pressed={syncMode === "symlink"} className="rounded-md border border-border px-2 py-1" onClick={() => { setSyncMode("symlink"); void writeSetting("sync_mode", "symlink"); }}>{t("settings.sync.symlink")}</button>
            <button type="button" aria-pressed={syncMode === "copy"} className="rounded-md border border-border px-2 py-1" onClick={() => { setSyncMode("copy"); void writeSetting("sync_mode", "copy"); }}>{t("settings.sync.copy")}</button>
          </div>
          <button type="submit" className="rounded-md bg-primary px-3 py-1 text-sm text-primary-foreground">Save path</button>
          <button
            type="button"
            className="ml-2 rounded-md border border-border px-3 py-1 text-sm"
            onClick={() => {
              setPath("~/.skill-hive");
              void onRepoPath("~/.skill-hive");
            }}
          >
            {t("settings.path.reset")}
          </button>
          {notice ? <p role="status">{notice}</p> : null}
          <div className="mt-4 space-y-2 text-sm">
            <p>{t("settings.close")}</p>
            <button type="button" aria-pressed={closeChoice === "ask"} className="rounded-md border border-border px-2 py-1" onClick={() => { setCloseChoice("ask"); void writeSetting("close_behavior", "ask"); }}>{t("close.dialog")}</button>
            <button type="button" aria-pressed={closeChoice === "minimize"} className="ml-2 rounded-md border border-border px-2 py-1" onClick={() => { const next = trayOn ? "minimize" : "ask"; setCloseChoice(next); void writeSetting("close_behavior", next); }}>{t("settings.close.minimize")}</button>
            <button type="button" aria-pressed={closeChoice === "quit"} className="ml-2 rounded-md border border-border px-2 py-1" onClick={() => { setCloseChoice("quit"); void writeSetting("close_behavior", "quit"); }}>{t("settings.close.quit")}</button>
            <label className="mt-2 block">
              <input type="checkbox" checked={trayOn} onChange={(event) => { setTrayOn(event.target.checked); void writeSetting("tray", event.target.checked ? "1" : "0"); }} /> {t("settings.tray")}
            </label>
            <p>{t("settings.autoUpdate")}</p>
            <button type="button" className="rounded-md border border-border px-2 py-1" onClick={() => setAutoApply(false)}>{t("settings.autoUpdate.off")}</button>
            <button type="button" className="ml-2 rounded-md border border-border px-2 py-1" onClick={() => setAutoApply(true)}>{t("settings.autoUpdate.daily")}</button>
            <label className="mt-2 block">
              <input type="checkbox" checked={autoApply} onChange={(event) => setAutoApply(event.target.checked)} /> {t("settings.autoUpdate.apply")}
            </label>
            <div>
              <label className="block">
                {t("settings.proxy")}
                <input aria-label={t("settings.proxy")} value={proxy} onChange={(event) => setProxy(event.target.value)} className="ml-2 rounded-md border border-input px-2 py-1" />
              </label>
              <button type="button" className="mt-2 rounded-md border border-border px-2 py-1" onClick={() => { const ok = proxyOk(proxy); setProxyNote(ok ? t("settings.proxy.save") : t("settings.proxy.invalid")); if (ok) void writeSetting("proxy", proxy.trim()); }}>{t("settings.proxy.save")}</button>
              {proxyNote ? <p role="status">{proxyNote}</p> : null}
            </div>
          </div>
        </form>
      ) : null}
      {tab === "agents" ? (
        <div className="mt-4">
          <div className="flex flex-wrap gap-2">
            <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => { void onAgents(agents.map((agent) => ({ ...agent, enabled: true }))); void setAllToolsEnabled(true); }}>{t("settings.agents.enableAll")}</button>
            <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => { void onAgents(agents.map((agent) => ({ ...agent, enabled: false }))); void setAllToolsEnabled(false); }}>{t("settings.agents.disableAll")}</button>
            <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => setRefreshed(true)}>{t("settings.agents.refresh")}</button>
          </div>
          {refreshed ? <p role="status">{t("settings.agents.refreshed")}</p> : null}
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
                  <input aria-label={`${t("settings.agents.reset")} ${agent.name}`} value={agent.path} onChange={(event) => { const pathValue = event.target.value; void onAgents(agents.map((item) => item.key === agent.key ? { ...item, path: pathValue } : item)); void setCustomToolPath(agent.key, pathValue); }} className="ml-2 rounded-md border border-input px-2 py-1" />
                </label>
                <label className="mt-1 block">
                  {t("settings.agents.project")}
                  <input aria-label={`${t("settings.agents.project")} ${agent.name}`} value={agent.projectPath} onChange={(event) => { const pathValue = event.target.value; void onAgents(agents.map((item) => item.key === agent.key ? { ...item, projectPath: pathValue } : item)); void setCustomToolProjectPath(agent.key, pathValue); }} className="ml-2 rounded-md border border-input px-2 py-1" />
                </label>
                <button type="button" className="mt-1 rounded-md border border-border px-2 py-1" onClick={() => Promise.resolve(onResetPath(agent.key)).then((next) => { if (next) return onAgents(agents.map((item) => item.key === agent.key ? { ...item, path: next } : item)); })}>{t("settings.agents.reset")}</button>
                {agent.custom ? (
                  <button type="button" className="ml-2 mt-1 rounded-md border border-border px-2 py-1" onClick={() => { void onAgents(agents.filter((item) => item.key !== agent.key)); void removeCustomTool(agent.key); }}>{t("settings.agents.remove")} {agent.name}</button>
                ) : null}
              </li>
            ))}
          </ul>
          <form
            className="mt-3 flex flex-wrap gap-2"
            onSubmit={(event) => {
              event.preventDefault();
              void onAddAgent(agentName, agentPath);
            }}
          >
            <input aria-label="Agent name" value={agentName} onChange={(event) => setAgentName(event.target.value)} className="rounded-md border border-input px-2 py-1 text-sm" />
            <input aria-label="Agent path" value={agentPath} onChange={(event) => setAgentPath(event.target.value)} className="rounded-md border border-input px-2 py-1 text-sm" />
            <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => void pickPath(true).then((selected) => selected && setAgentPath(selected))}>Browse</button>
            <button type="submit" className="rounded-md bg-primary px-2 py-1 text-sm text-primary-foreground">{t("settings.agents.new")}</button>
          </form>
        </div>
      ) : null}
      {tab === "backup" ? (
        <div className="mt-4 space-y-2 text-sm">
          <p>{t("backup.advanced")}</p>
          <label className="block">
            {t("backup.remote")}
            <input aria-label={t("backup.remote")} className="ml-2 rounded-md border border-input px-2 py-1" onBlur={(event) => { if (event.target.value.trim()) void gitBackupSetRemote(event.target.value.trim()); }} />
          </label>
          <button type="button" className="rounded-md border border-border px-2 py-1" onClick={() => void gitBackupRemoveRemote()}>{t("backup.disconnect")}</button>
          <button type="button" disabled className="ml-2 rounded-md border border-border px-2 py-1">{t("settings.engine.libgit2")}</button>
          <p className="text-muted-foreground">{t("settings.engine.note")}</p>
        </div>
      ) : null}
      {tab === "about" ? (
        <div className="mt-4 text-sm">
          <p>{t("settings.about.version")}</p>
          <button
            type="button"
            className="mt-2 rounded-md border border-border px-2 py-1"
            onClick={() => {
              const remote = localStorage.getItem("skill-hive-update-version");
              const runs = Number(localStorage.getItem("skill-hive-active-runs") ?? "0");
              const decision = decideUpdate({
                remoteVersion: remote,
                platform: desktopPlatform(navigator.userAgent, localStorage.getItem("skill-hive-platform")),
                activeRuns: Number.isFinite(runs) ? runs : 0,
              });
              setUpdateNote(decision.state === "available" ? t(decision.messageKey, { version: decision.version }) : t(decision.messageKey));
              setUpdateAction(decision.state === "available" ? decision.action : null);
              void downloadAndInstallUpdate();
              if (decision.state === "available" && decision.action === "install") {
                localStorage.setItem("skill-hive-pending-install", decision.version);
              } else {
                localStorage.removeItem("skill-hive-pending-install");
              }
            }}
          >
            {t("settings.about.updates")}
          </button>
          {updateNote ? <p role="status">{updateNote}</p> : null}
          {localStorage.getItem("skill-hive-pending-install") && desktopPlatform(navigator.userAgent, localStorage.getItem("skill-hive-platform")) !== "linux" ? (
            <button
              type="button"
              className="mt-2 rounded-md border border-border px-2 py-1"
              data-proxy={proxyOk(proxy) && proxy.trim() ? proxy.trim() : ""}
              onClick={() => {
                localStorage.setItem("skill-hive-restart-now", "1");
                setRestartArmed(true);
                window.dispatchEvent(new Event("skill-hive-restart"));
              }}
            >
              {t("settings.about.install")}
            </button>
          ) : null}
          {updateAction === "download" ? (
            <a className="mt-2 inline-block rounded-md border border-border px-2 py-1" href={LINUX_DOWNLOAD_URL}>{t("settings.about.download")}</a>
          ) : null}
          {crash ? <p role="alert">{t("settings.about.crash")}</p> : null}
          {repoWarn ? <p role="alert">{t("settings.about.repo")}</p> : null}
          <a className="mt-2 inline-block rounded-md border border-border px-2 py-1" href="https://github.com/crzyc0d3r/skill-hive/issues/new" target="_blank" rel="noreferrer">{t("settings.about.report")}</a>
          <label className="mt-2 block">
            Diagnostics path
            <input aria-label="Diagnostics path" value={diagnosticsPath} onChange={(event) => setDiagnosticsPath(event.target.value)} className="ml-2 rounded-md border border-input px-2 py-1" />
          </label>
          <button type="button" className="ml-2 rounded-md border border-border px-2 py-1" onClick={() => { void exportDiagnostics(diagnosticsPath).then(() => setExported(true)); }}>{t("settings.about.export")}</button>
          {exported ? <p role="status">{t("settings.about.exported")}</p> : null}
          {restartArmed ? <p role="status"><button type="button">{t("settings.about.restart")}</button></p> : null}
          <h2 className="mt-4 text-sm font-medium">{t("settings.about.ack")}</h2>
          <p className="mt-1 text-muted-foreground">{t("settings.about.mit")}</p>
        </div>
      ) : null}
    </section>
  );
}
