import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { EmptyState, Screen, StatePanel } from "@/components/chrome";
import { AgentListSkeleton } from "@/components/skeletons";
import { RetryContext, useShellPhase } from "@/lib/view-state";
import { useContext, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { t } from "@/lib/i18n";
import { demoEnabled } from "@/lib/demo";
import { agentBudget, ensureAgent, importAgent, useAuthoring } from "@/lib/authoring/store";

export function AgentsView() {
  const phase = useShellPhase();
  const retry = useContext(RetryContext);
  const authoring = useAuthoring();
  const [importName, setImportName] = useState("reviewer");
  const [collision, setCollision] = useState(false);
  const [query, setQuery] = useState("");
  const [tag, setTag] = useState("");
  const agents = Object.values(authoring.agents).filter((agent) => {
    const matchesQuery = agent.name.toLowerCase().includes(query.trim().toLowerCase());
    const matchesTag = tag.trim() === "" || tag.trim().toLowerCase() === "draft";
    return matchesQuery && matchesTag;
  });
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
        <label className="text-sm">
          <span className="mb-1 block text-xs text-muted-foreground">{t("agents.field.search")}</span>
          <Input aria-label={t("agents.search")} value={query} onChange={(event) => setQuery(event.target.value)} className="h-8 w-40" />
        </label>
        <label className="text-sm">
          <span className="mb-1 block text-xs text-muted-foreground">{t("agents.field.tag")}</span>
          <Input aria-label={t("agents.tag")} value={tag} onChange={(event) => setTag(event.target.value)} className="h-8 w-28" />
        </label>
        <label className="text-sm">
          <span className="mb-1 block text-xs text-muted-foreground">{t("agents.field.import")}</span>
          <Input aria-label={t("agents.import")} value={importName} onChange={(event) => setImportName(event.target.value)} className="h-8 w-56" />
        </label>
        <Button type="submit" size="sm">{t("agents.import")}</Button>
      </form>
      {phase === "loading" ? <AgentListSkeleton /> : null}
      {phase === "error" ? <StatePanel tone="error" title={t("shell.error")} action={<Button type="button" size="sm" onClick={retry}>{t("banner.retry")}</Button>} /> : null}
      {phase === "ready" && agents.length === 0 ? <EmptyState title={t("agents.title")} /> : null}
      {phase === "ready" ? <div className="overflow-hidden rounded-lg border border-border">
        <table className="w-full text-sm">
          <tbody>
        {agents.map((agent) => (
          <tr key={agent.id} className="border-t border-border first:border-t-0">
            <td className="px-3 py-3"><Link to={`/agents/${agent.id}/edit`} className="font-medium">{agent.name}</Link></td>
            <td className="px-3 py-3 text-right"><Badge variant="secondary">{t("library.badge.draft")}</Badge></td>
          </tr>
        ))}
          </tbody>
        </table>
      </div> : null}
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
