import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { filterMarket, MARKET_FIXTURE, type MarketSkill } from "@/features/install/market";
import { fetchLeaderboard, installFromSkillsSh } from "@/lib/ipc/actions";
import { demoEnabled } from "@/lib/demo";
import { t } from "@/lib/i18n";

export function MarketplacePanel({ reachable = true, skills }: { reachable?: boolean; skills?: MarketSkill[] }) {
  const [query, setQuery] = useState("");
  const [owner, setOwner] = useState<string | null>(null);
  const [installed, setInstalled] = useState<string | null>(null);
  const [pending, setPending] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
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
  const source = skills ?? leaderboard.data ?? [];
  const rows = filterMarket(source, query, owner);
  const owners = [...new Set(source.map((skill) => skill.owner))];
  if (!reachable) return <p className="mt-4 text-sm text-muted-foreground">{t("install.leaderboard.empty")}</p>;
  return (
    <div className="mt-4 space-y-2" data-testid="marketplace">
      <input aria-label={t("install.search")} value={query} onChange={(event) => setQuery(event.target.value)} className="rounded-md border border-input px-2 py-1 text-sm" />
      {leaderboard.isError ? <p role="alert">{leaderboard.error instanceof Error ? leaderboard.error.message : "Marketplace unavailable"}</p> : null}
      {error ? <p role="alert">{error}</p> : null}
      <div className="flex flex-wrap gap-2">
        {owners.map((item) => (
          <button key={item} type="button" aria-pressed={owner === item} className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => setOwner(owner === item ? null : item)}>
            {t("install.owner")} {item}
          </button>
        ))}
      </div>
      <ul className="space-y-1 text-sm">
        {rows.map((skill) => (
          <li key={`${skill.owner}/${skill.name}`} className="flex items-center gap-2">
            <span>{skill.owner}/{skill.name}</span>
            <button
              type="button"
              className="rounded-md bg-primary px-2 py-1 text-primary-foreground"
              onClick={() => {
                setPending(skill.name);
                setInstalled(skill.name);
                if (demoEnabled()) return;
                const sourceUrl = "source" in skill && typeof skill.source === "string" ? skill.source : `${skill.owner}/${skill.name}`;
                void installFromSkillsSh(sourceUrl, skill.name).catch((err: Error) => setError(err.message));
              }}
            >
              {t("install.install")}
            </button>
          </li>
        ))}
      </ul>
      {pending ? (
        <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => { setPending(null); setInstalled(null); }}>
          {t("install.cancelInstall")}
        </button>
      ) : null}
      {installed ? <p role="status">{t("install.installed")} {installed}</p> : null}
      <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => void leaderboard.refetch()}>{t("install.loadMore")}</button>
    </div>
  );
}
