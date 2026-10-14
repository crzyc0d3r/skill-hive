import { useState } from "react";
import { Link } from "react-router-dom";
import { t } from "@/lib/i18n";
import { agentBudget, importAgent, useAuthoring } from "@/lib/authoring/store";

export function AgentsView() {
  const authoring = useAuthoring();
  const [importName, setImportName] = useState("reviewer");
  const [collision, setCollision] = useState(false);
  const agents = Object.values(authoring.agents);
  const budget = agentBudget();

  return (
    <section data-testid="agents">
      <h1 className="font-mono text-[22px] font-semibold">{t("agents.title")}</h1>
      <p data-testid="description-budget" data-amber={budget.amber ? "true" : "false"} className={budget.amber ? "text-warning-text" : "text-muted-foreground"}>
        {t("agents.budget")} {budget.tokens}
      </p>
      <form
        className="mt-3 flex gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          if (agents.some((agent) => agent.name === importName)) setCollision(true);
          else importAgent(importName, "replace");
        }}
      >
        <input aria-label={t("agents.import")} value={importName} onChange={(event) => setImportName(event.target.value)} className="rounded-md border border-input px-2 py-1 text-sm" />
        <button type="submit" className="rounded-md border border-border px-2 py-1 text-sm">{t("agents.import")}</button>
      </form>
      <ul className="mt-4 space-y-2">
        {agents.map((agent) => (
          <li key={agent.id} className="rounded-md border border-border p-3">
            <Link to={`/agents/${agent.id}/edit`}>{agent.name}</Link>
            <span className="ml-2 rounded-full border border-border px-2 text-xs">{t("library.badge.draft")}</span>
          </li>
        ))}
      </ul>
      {collision ? (
        <div role="dialog" aria-label={t("agents.collision")} className="mt-3 rounded-md border border-border p-3">
          <p>{t("agents.collision")}</p>
          <button type="button" className="mr-2 text-sm" onClick={() => { importAgent(importName, "replace"); setCollision(false); }}>{t("agents.replace")}</button>
          <button type="button" className="mr-2 text-sm" onClick={() => { importAgent(importName, "copy"); setCollision(false); }}>{t("agents.copy")}</button>
          <button type="button" className="text-sm" onClick={() => setCollision(false)}>{t("agents.cancel")}</button>
        </div>
      ) : null}
    </section>
  );
}
