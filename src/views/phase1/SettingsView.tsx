import { useState } from "react";
import { useSearchParams } from "react-router-dom";
import type { AgentRow } from "@/lib/phase1/client";
import { applyTextSize, t, type TextSize } from "@/lib/i18n";
import type { ThemeChoice } from "@/lib/theme";

const TABS = ["general", "appearance", "agents", "about"] as const;

export function SettingsView({
  theme,
  setTheme,
  agents,
  onAddAgent,
  onRepoPath,
}: {
  theme: ThemeChoice;
  setTheme: (choice: ThemeChoice) => void;
  agents: AgentRow[];
  onAddAgent?: (name: string, path: string) => void;
  onRepoPath?: (path: string) => void;
}) {
  const [params, setParams] = useSearchParams();
  const tab = TABS.includes((params.get("tab") ?? "appearance") as (typeof TABS)[number])
    ? ((params.get("tab") ?? "appearance") as (typeof TABS)[number])
    : "appearance";
  const [notice, setNotice] = useState<string | null>(null);
  const [path, setPath] = useState("~/.skill-studio");
  const [agentName, setAgentName] = useState("Field Notes");
  const [agentPath, setAgentPath] = useState("~/.field-notes/skills");
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
          <div className="mt-3 flex gap-2">
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
            <span>{t("settings.sync.symlink")}</span>
            <span>{t("settings.sync.copy")}</span>
          </div>
          <button type="submit" className="rounded-md bg-primary px-3 py-1 text-sm text-primary-foreground">Save path</button>
          {notice ? <p role="status">{notice}</p> : null}
        </form>
      ) : null}
      {tab === "agents" ? (
        <div className="mt-4">
          <div className="flex gap-2">
            <button type="button" className="rounded-md border border-border px-2 py-1 text-sm">{t("settings.agents.enableAll")}</button>
            <button type="button" className="rounded-md border border-border px-2 py-1 text-sm">{t("settings.agents.disableAll")}</button>
          </div>
          <ul className="mt-3 space-y-2">
            {agents.map((agent) => (
              <li key={agent.key} className="rounded-md border border-border p-2 text-sm">
                <span className="font-medium">{agent.name}</span>
                <span className="ml-2 text-muted-foreground">{agent.installed ? "Detected" : "Other supported"}</span>
                <div>{agent.path}</div>
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
          <button type="button" className="mt-2 rounded-md border border-border px-2 py-1">{t("settings.about.updates")}</button>
          <p className="mt-2 text-muted-foreground">Update notification only. Installing an update arrives later.</p>
        </div>
      ) : null}
    </section>
  );
}
