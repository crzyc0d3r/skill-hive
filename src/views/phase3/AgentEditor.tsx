import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { t } from "@/lib/i18n";
import { exportStatus, renderAgentFile, renderAgentsCliJson, validateAgentText } from "@/lib/phase3/model";
import { agentState, ensureAgent, exportAgent, fixAgent, saveAgent, useAuthoring } from "@/lib/phase3/store";

const TABS = ["problems", "preview", "export", "versions"] as const;

export function AgentEditor() {
  const { id = "" } = useParams();
  const authoring = useAuthoring();
  const [tab, setTab] = useState<(typeof TABS)[number]>("problems");
  const [view, setView] = useState<"file" | "json" | "delegation">("file");
  const agent = authoring.agents[id] ?? null;

  useEffect(() => {
    ensureAgent(id);
  }, [id]);

  if (!agent) return <p>{t("library.empty")}</p>;
  const findings = validateAgentText(agent.text, agent.plugin);
  const rendered = renderAgentFile(agent.text);
  const status = agentState(id);
  const json = JSON.stringify(renderAgentsCliJson(agent.text));

  return (
    <section data-testid="agent-editor" className="min-w-0">
      <h1 className="font-mono text-lg font-semibold">{agent.name}</h1>
      <div className="mt-2 flex flex-wrap gap-2">
        <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => saveAgent(agent)}>{t("editor.save")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => exportAgent(agent.id)}>{t("agents.user")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => exportAgent(agent.id)}>{t("agents.project")}</button>
        <button type="button" disabled className="rounded-md border border-border px-2 py-1 text-sm opacity-50">{t("editor.unverified")}</button>
        <button type="button" aria-pressed={agent.plugin} className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => saveAgent({ ...agent, plugin: !agent.plugin })}>{t("editor.plugin")}</button>
      </div>
      <p data-testid="export-status">{t(`agents.${status}`)}</p>
      <label className="mt-3 block text-sm">
        {t("editor.body")}
        <textarea aria-label={t("editor.body")} value={agent.text} onChange={(event) => saveAgent({ ...agent, text: event.target.value })} className="mt-1 h-40 w-full rounded-md border border-input p-2 font-mono text-sm" />
      </label>
      <div data-testid="editor-tabs" role="tablist" className="mt-3 flex flex-wrap gap-1">
        {TABS.map((item) => (
          <button key={item} type="button" role="tab" aria-selected={tab === item} className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => setTab(item)}>
            {t(item === "export" ? "editor.export" : `editor.${item}`)}
          </button>
        ))}
      </div>
      {tab === "problems" ? (
        <ul data-testid="problems" className="mt-2 text-sm">
          {findings.map((item) => (
            <li key={`${item.ruleId}-${item.message}`}>
              {item.ruleId} {item.message}
              {item.fixId ? <button type="button" className="ml-2 underline" onClick={() => fixAgent(agent.id)}>{t("editor.fix")}</button> : null}
            </li>
          ))}
        </ul>
      ) : null}
      {tab === "preview" ? (
        <div data-testid="agent-preview" className="mt-2 text-sm">
          <div className="flex gap-1">
            <button type="button" aria-pressed={view === "file"} onClick={() => setView("file")}>{t("editor.previewFile")}</button>
            <button type="button" aria-pressed={view === "json"} onClick={() => setView("json")}>{t("editor.agentsJson")}</button>
            <button type="button" aria-pressed={view === "delegation"} onClick={() => setView("delegation")}>{t("editor.delegation")}</button>
          </div>
          {view === "file" ? <pre data-testid="agent-file">{rendered}</pre> : null}
          {view === "json" ? <pre data-testid="agents-json">{json}</pre> : null}
          {view === "delegation" ? <p>{agent.name}</p> : null}
          <p className="sr-only">{exportStatus(rendered, agent.exported, agent.disk)}</p>
        </div>
      ) : null}
    </section>
  );
}
