import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card } from "@/components/ui/card";
import { EmptyState, Screen } from "@/components/chrome";
import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { t } from "@/lib/i18n";
import { demoEnabled } from "@/lib/demo";
import { agentBudget, ensureAgent, importAgent, useAuthoring } from "@/lib/authoring/store";

export function AgentsView() {
  const authoring = useAuthoring();
  const [importName, setImportName] = useState("reviewer");
  const [collision, setCollision] = useState(false);
  const agents = Object.values(authoring.agents);
  const budget = agentBudget();
  useEffect(() => {
    if (demoEnabled() && agents.length === 0) ensureAgent("reviewer");
  }, [agents.length]);

  return (
    <Screen
      testId="agents"
      title={t("agents.title")}
      lede={<span data-testid="description-budget" data-amber={budget.amber ? "true" : "false"} className={budget.amber ? "text-warning-text" : undefined}>{t("agents.budget")} {budget.tokens}</span>}
    >
      <form
        className="flex max-w-xl flex-wrap items-center gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          if (agents.some((agent) => agent.name === importName)) setCollision(true);
          else importAgent(importName, "replace");
        }}
      >
        <Input aria-label={t("agents.import")} value={importName} onChange={(event) => setImportName(event.target.value)} className="h-8 w-56" />
        <Button type="submit" size="sm">{t("agents.import")}</Button>
      </form>
      {agents.length === 0 ? <EmptyState title={t("agents.title")} /> : null}
      <ul className="grid gap-2 sm:grid-cols-2">
        {agents.map((agent) => (
          <li key={agent.id}>
            <Card className="flex-row items-center justify-between rounded-lg border-border px-3 py-3 shadow-none">
              <Link to={`/agents/${agent.id}/edit`} className="text-sm font-medium">{agent.name}</Link>
              <Badge variant="secondary">{t("library.badge.draft")}</Badge>
            </Card>
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
    </Screen>
  );
}
