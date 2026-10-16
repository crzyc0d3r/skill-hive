import { Button } from "@/components/ui/button";
import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { t } from "@/lib/i18n";
import { exportStatus, renderAgentFile, renderAgentsCliJson, validateAgentText } from "@/lib/authoring/model";
import { agentState, ensureAgent, exportAgent, fixAgent, saveAgent, useAuthoring } from "@/lib/authoring/store";

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
        <Button type="button" variant="outline" size="sm" onClick={() => saveAgent(agent)}>{t("editor.save")}</Button>
        <Button type="button" variant="outline" size="sm" onClick={() => exportAgent(agent.id)}>{t("agents.user")}</Button>
        <Button type="button" variant="outline" size="sm" onClick={() => exportAgent(agent.id)}>{t("agents.project")}</Button>
        <Button type="button" disabled variant="outline" size="sm">{t("editor.unverified")}</Button>
        <Button type="button" aria-pressed={agent.plugin} variant="outline" size="sm" onClick={() => saveAgent({ ...agent, plugin: !agent.plugin })}>{t("editor.plugin")}</Button>
      </div>
      <p data-testid="export-status">{t(`agents.${status}`)}</p>
      <label className="mt-3 block text-sm">
        {t("editor.body")}
        <textarea aria-label={t("editor.body")} value={agent.text} onChange={(event) => saveAgent({ ...agent, text: event.target.value })} className="mt-1 h-40 w-full rounded-md border border-input p-2 font-mono text-sm" />
      </label>
      <div data-testid="editor-tabs" role="tablist" className="mt-3 flex flex-wrap gap-1">
        {TABS.map((item) => (
          <Button key={item} type="button" role="tab" aria-selected={tab === item} variant={tab === item ? "default" : "outline"} size="sm" onClick={() => setTab(item)}>
            {t(item === "export" ? "editor.export" : `editor.${item}`)}
          </Button>
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
