import { useState } from "react";
import { useSearchParams } from "react-router-dom";
import type { AgentRow } from "@/lib/phase1/client";
import { applyTextSize, t, type TextSize } from "@/lib/i18n";
import { moveById } from "@/lib/phase1/model";
import type { ThemeChoice } from "@/lib/theme";

const TABS = ["general", "appearance", "agents", "about"] as const;

export function SettingsView({
  theme,
  setTheme,
  agents,
  onAddAgent,
  onRepoPath,
  onAgents,
}: {
  theme: ThemeChoice;
  setTheme: (choice: ThemeChoice) => void;
  agents: AgentRow[];
  onAddAgent?: (name: string, path: string) => void;
  onRepoPath?: (path: string) => void;
  onAgents?: (agents: AgentRow[]) => void;
}) {
  const [params, setParams] = useSearchParams();
  const tab = TABS.includes((params.get("tab") ?? "appearance") as (typeof TABS)[number])
    ? ((params.get("tab") ?? "appearance") as (typeof TABS)[number])
    : "appearance";
  const [notice, setNotice] = useState<string | null>(null);
  const [path, setPath] = useState("~/.skill-hive");
  const [agentName, setAgentName] = useState("Field Notes");
  const [agentPath, setAgentPath] = useState("~/.field-notes/skills");
  const [syncMode, setSyncMode] = useState<"symlink" | "copy">("symlink");
  const [updateNote, setUpdateNote] = useState<string | null>(null);
  const [refreshed, setRefreshed] = useState(false);
  const [dragKey, setDragKey] = useState<string | null>(null);
  const [motion, setMotion] = useState(false);
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
        </div>
      ) : null}
      {tab === "general" ? (
        <form
          className="mt-4 space-y-2"
          onSubmit={(event) => {
            event.preventDefault();
            onRepoPath?.(path);
            setNotice(t("settings.restart"));
          }}
        >
          <label className="block text-sm">
            {t("settings.path")}
            <input aria-label={t("settings.path")} value={path} onChange={(event) => setPath(event.target.value)} className="mt-1 block w-full rounded-md border border-input px-2 py-1" />
          </label>
          <div className="flex gap-2 text-sm">
            <button type="button" aria-pressed={syncMode === "symlink"} className="rounded-md border border-border px-2 py-1" onClick={() => setSyncMode("symlink")}>{t("settings.sync.symlink")}</button>
            <button type="button" aria-pressed={syncMode === "copy"} className="rounded-md border border-border px-2 py-1" onClick={() => setSyncMode("copy")}>{t("settings.sync.copy")}</button>
          </div>
          <button type="submit" className="rounded-md bg-primary px-3 py-1 text-sm text-primary-foreground">Save path</button>
          <button
            type="button"
            className="ml-2 rounded-md border border-border px-3 py-1 text-sm"
            onClick={() => setPath("~/.skill-hive")}
          >
            {t("settings.path.reset")}
          </button>
          {notice ? <p role="status">{notice}</p> : null}
        </form>
      ) : null}
      {tab === "agents" ? (
        <div className="mt-4">
          <div className="flex flex-wrap gap-2">
            <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => onAgents?.(agents.map((agent) => ({ ...agent, enabled: true })))}>{t("settings.agents.enableAll")}</button>
            <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => onAgents?.(agents.map((agent) => ({ ...agent, enabled: false })))}>{t("settings.agents.disableAll")}</button>
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
                  onAgents?.(moveById(agents, dragKey, agent.key, (item) => item.key));
                  setDragKey(null);
                }}
                className="rounded-md border border-border p-2 text-sm"
              >
                <span className="font-medium">{agent.name}</span>
                <span className="ml-2 text-muted-foreground">{agent.enabled ? "Enabled" : "Disabled"}</span>
                <span className="ml-2 text-muted-foreground">{agent.installed ? "Detected" : "Other supported"}</span>
                <label className="mt-1 block">
                  {t("settings.agents.reset")}
                  <input aria-label={`${t("settings.agents.reset")} ${agent.name}`} value={agent.path} onChange={(event) => onAgents?.(agents.map((item) => item.key === agent.key ? { ...item, path: event.target.value } : item))} className="ml-2 rounded-md border border-input px-2 py-1" />
                </label>
                <label className="mt-1 block">
                  {t("settings.agents.project")}
                  <input aria-label={`${t("settings.agents.project")} ${agent.name}`} value={agent.projectPath} onChange={(event) => onAgents?.(agents.map((item) => item.key === agent.key ? { ...item, projectPath: event.target.value } : item))} className="ml-2 rounded-md border border-input px-2 py-1" />
                </label>
                <button type="button" className="mt-1 rounded-md border border-border px-2 py-1" onClick={() => onAgents?.(agents.map((item) => item.key === agent.key ? { ...item, path: "~/.claude/skills" } : item))}>{t("settings.agents.reset")}</button>
                {agent.custom ? (
                  <button type="button" className="ml-2 mt-1 rounded-md border border-border px-2 py-1" onClick={() => onAgents?.(agents.filter((item) => item.key !== agent.key))}>{t("settings.agents.remove")} {agent.name}</button>
                ) : null}
              </li>
            ))}
          </ul>
          <form
            className="mt-3 flex flex-wrap gap-2"
            onSubmit={(event) => {
              event.preventDefault();
              onAddAgent?.(agentName, agentPath);
            }}
          >
            <input aria-label="Agent name" value={agentName} onChange={(event) => setAgentName(event.target.value)} className="rounded-md border border-input px-2 py-1 text-sm" />
            <input aria-label="Agent path" value={agentPath} onChange={(event) => setAgentPath(event.target.value)} className="rounded-md border border-input px-2 py-1 text-sm" />
            <button type="submit" className="rounded-md bg-primary px-2 py-1 text-sm text-primary-foreground">{t("settings.agents.new")}</button>
          </form>
        </div>
      ) : null}
      {tab === "about" ? (
        <div className="mt-4 text-sm">
          <p>{t("settings.about.version")}</p>
          <button type="button" className="mt-2 rounded-md border border-border px-2 py-1" onClick={() => setUpdateNote(t("settings.about.latest"))}>{t("settings.about.updates")}</button>
          {updateNote ? <p role="status">{updateNote}</p> : null}
          <p className="mt-2 text-muted-foreground">Update notification only. Installing an update arrives later.</p>
        </div>
      ) : null}
    </section>
  );
}
