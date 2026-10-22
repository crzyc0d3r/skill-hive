import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { filterMarket, MARKET_FIXTURE, type MarketSkill } from "@/features/install/market";
import { beginMarketInstall, cancelMarketInstall, fetchLeaderboard, marketInstallResult, searchSkillsSh } from "@/lib/ipc/actions";
import { demoEnabled } from "@/lib/demo";
import { t } from "@/lib/i18n";

export function MarketplacePanel({ reachable = true, skills }: { reachable?: boolean; skills?: MarketSkill[] }) {
  const [query, setQuery] = useState("");
  const [owner, setOwner] = useState<string | null>(null);
  const [installed, setInstalled] = useState<string | null>(null);
  const [pending, setPending] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [remote, setRemote] = useState<MarketSkill[] | null>(null);
  useEffect(() => {
    if (demoEnabled() || !query.trim()) return;
    const handle = window.setTimeout(() => {
      void searchSkillsSh(query.trim()).then((rows) => {
        if (Array.isArray(rows)) setRemote(rows);
      });
    }, 300);
    return () => window.clearTimeout(handle);
  }, [query]);
  const leaderboard = useQuery({
    queryKey: ["leaderboard", demoEnabled()],
    queryFn: () =>
      demoEnabled()
        ? Promise.resolve(
            MARKET_FIXTURE.map((skill) => ({
              owner: skill.owner,
              name: skill.name,
              installs: skill.installs,
              source: `${skill.owner}/${skill.name}`,
            })),
          )
        : fetchLeaderboard(),
    enabled: reachable && !skills,
  });
  const source = remote ?? skills ?? leaderboard.data ?? [];
  const rows = filterMarket(source, query, owner);
  const owners = [...new Set(source.map((skill) => skill.owner))];
  if (!reachable) return <p className="mt-4 text-sm text-muted-foreground">{t("install.leaderboard.empty")}</p>;
  return (
    <div className="mt-4 space-y-2" data-testid="marketplace">
      <label className="block text-sm">
        <span className="mb-1.5 block text-muted-foreground">{t("install.search")}</span>
        <Input aria-label={t("install.search")} value={query} onChange={(event) => setQuery(event.target.value)} className="h-8 max-w-sm" />
      </label>
      {leaderboard.isError ? <p role="alert">{leaderboard.error instanceof Error ? leaderboard.error.message : "Marketplace unavailable"}</p> : null}
      {error ? <p role="alert">{error}</p> : null}
      <div className="flex flex-wrap gap-2">
        {owners.map((item) => (
          <Button key={item} type="button" aria-pressed={owner === item} variant="outline" size="sm" onClick={() => setOwner(owner === item ? null : item)}>
            {t("install.owner")} {item}
          </Button>
        ))}
      </div>
      <ul className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
        {rows.map((skill) => (
          <li key={`${skill.owner}/${skill.name}`} className="flex flex-col gap-2 rounded-lg border border-border bg-card p-3">
            <div className="min-w-0">
              <p className="font-medium">{skill.name}</p>
              <p className="text-xs text-muted-foreground">{t("install.market.author", { owner: skill.owner })}</p>
              <p className="text-xs tabular-nums text-muted-foreground">{t("install.market.installs", { count: String(skill.installs) })}</p>
            </div>
            <Button type="button" variant="outline" size="sm" className="w-fit" onClick={() => { setPending(skill.name); if (demoEnabled()) { setInstalled(skill.name); return; } const sourceUrl = "source" in skill && typeof skill.source === "string" ? skill.source : `${skill.owner}/${skill.name}`; void (async () => { try { await beginMarketInstall(sourceUrl, skill.name); const deadline = Date.now() + 120000; while (Date.now() < deadline) { const status = (await marketInstallResult()) ?? ""; if (status === "running" || status === "") { await new Promise((resolve) => setTimeout(resolve, 200)); continue; } if (status.startsWith("installed:")) { setInstalled(skill.name); return; } setError(status.replace(/^error:/, "")); setPending(null); return; } setError("marketplace install timed out"); setPending(null); } catch (err) { setError(err instanceof Error ? err.message : "marketplace install failed"); setPending(null); } })(); }}>
              {t("install.install")}
            </Button>
          </li>
        ))}
      </ul>
      {pending ? (
        <Button type="button" variant="outline" size="sm" onClick={() => { setPending(null); setInstalled(null); void cancelMarketInstall(); }}>
          {t("install.cancelInstall")}
        </Button>
      ) : null}
      {installed ? <p role="status">{t("install.installed")} {installed}</p> : null}
      <Button type="button" variant="outline" size="sm" onClick={() => void leaderboard.refetch()}>{t("install.loadMore")}</Button>
    </div>
  );
}
