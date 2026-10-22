import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { EmptyState, ErrorState, Screen } from "@/components/chrome";
import { AgentListSkeleton } from "@/components/skeletons";
import { RetryContext, useViewPhase } from "@/lib/view-state";
import { useContext, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { t } from "@/lib/i18n";
import { demoEnabled } from "@/lib/demo";
import { ensureAgent, importAgent, useAuthoring } from "@/lib/authoring/store";

export function AgentsView() {
  const phase = useViewPhase();
  const retry = useContext(RetryContext);
  const authoring = useAuthoring();
  const [importOpen, setImportOpen] = useState(false);
  const [importName, setImportName] = useState("reviewer");
  const [collision, setCollision] = useState(false);
  const [query, setQuery] = useState("");
  const [tag, setTag] = useState("");
  const agents = Object.values(authoring.agents).filter((agent) => {
    const matchesQuery = agent.name.toLowerCase().includes(query.trim().toLowerCase());
    const matchesTag = tag.trim() === "" || tag.trim().toLowerCase() === "draft";
    return matchesQuery && matchesTag;
  });
  useEffect(() => {
    if (phase !== "ready") return;
    if (demoEnabled() && Object.keys(authoring.agents).length === 0) ensureAgent("reviewer");
  }, [phase, authoring.agents]);

  function submitImport() {
    if (Object.values(authoring.agents).some((agent) => agent.name === importName)) setCollision(true);
    else {
      importAgent(importName, "replace");
      setImportOpen(false);
    }
  }

  return (
    <Screen
      testId="agents"
      title={t("agents.title")}
      lede={t("agents.lede")}
      actions={
        <>
          <Button type="button" size="sm" onClick={() => ensureAgent(`profile-${Object.keys(authoring.agents).length + 1}`)}>{t("agents.new")}</Button>
          <Button type="button" variant="outline" size="sm" onClick={() => { setImportOpen(true); submitImport(); }}>{t("agents.import")}</Button>
        </>
      }
    >
      {importOpen && !collision ? (
        <form
          className="max-w-xl space-y-3 rounded-lg border border-border bg-card p-4"
          onSubmit={(event) => {
            event.preventDefault();
            submitImport();
          }}
        >
          <label className="block text-sm">
            <span className="mb-1.5 block text-muted-foreground">{t("agents.field.search")}</span>
            <Input aria-label={t("agents.search")} value={query} onChange={(event) => setQuery(event.target.value)} className="h-8" />
          </label>
          <label className="block text-sm">
            <span className="mb-1.5 block text-muted-foreground">{t("agents.field.tag")}</span>
            <Input aria-label={t("agents.tag")} value={tag} onChange={(event) => setTag(event.target.value)} className="h-8" />
          </label>
          <label className="block text-sm">
            <span className="mb-1.5 block text-muted-foreground">{t("agents.field.import")}</span>
            <Input aria-label={t("agents.import")} value={importName} onChange={(event) => setImportName(event.target.value)} className="h-8" />
          </label>
          <Button type="submit" size="sm">{t("agents.import")}</Button>
        </form>
      ) : null}
      {phase === "loading" ? <AgentListSkeleton /> : null}
      {phase === "error" ? <ErrorState title={t("shell.error")} detail={t("shell.renderError")} onRetry={retry} /> : null}
      {phase === "empty" ? <EmptyState title={t("agents.emptyTitle")} body={t("agents.lede")} action={<Button type="button" size="sm" onClick={() => ensureAgent("profile-1")}>{t("agents.new")}</Button>} /> : null}
      {phase === "ready" && agents.length === 0 ? <EmptyState title={t("agents.emptyTitle")} body={t("agents.lede")} /> : null}
      {phase === "ready" && agents.length > 0 ? (
        <div className="overflow-hidden rounded-lg border border-border">
          <table className="w-full text-sm">
            <thead className="bg-muted/40 text-left text-xs text-muted-foreground">
              <tr>
                <th className="px-3 py-2 font-medium">{t("agents.col.name")}</th>
                <th className="px-3 py-2 font-medium">{t("agents.col.usedBy")}</th>
                <th className="px-3 py-2 font-medium">{t("agents.col.edited")}</th>
                <th className="px-3 py-2 font-medium">{t("agents.col.status")}</th>
              </tr>
            </thead>
            <tbody>
              {agents.map((agent) => (
                <tr key={agent.id} className="border-t border-border">
                  <td className="px-3 py-3"><Link to={`/agents/${agent.id}/edit`} className="font-medium">{agent.name}</Link></td>
                  <td className="px-3 py-3 text-muted-foreground">{t("agents.usedBy.draft")}</td>
                  <td className="px-3 py-3 text-muted-foreground">{t("agents.edited.today")}</td>
                  <td className="px-3 py-3"><Badge variant="secondary">{t("library.badge.draft")}</Badge></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}
      {collision ? (
        <div role="dialog" aria-label={t("agents.collision")} className="mt-3 rounded-md border border-border p-3">
          <p>{t("agents.collision")}</p>
          <button type="button" className="mr-2 text-sm" onClick={() => { importAgent(importName, "replace"); setCollision(false); setImportOpen(false); }}>{t("agents.replace")}</button>
          <button type="button" className="mr-2 text-sm" onClick={() => { importAgent(importName, "copy"); setCollision(false); setImportOpen(false); }}>{t("agents.copy")}</button>
          <button type="button" className="text-sm" onClick={() => setCollision(false)}>{t("agents.cancel")}</button>
        </div>
      ) : null}
    </Screen>
  );
}
