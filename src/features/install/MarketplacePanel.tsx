import { useState } from "react";
import { filterMarket, MARKET_FIXTURE, type MarketSkill } from "@/features/install/market";
import { t } from "@/lib/i18n";

export function MarketplacePanel({ reachable = true, skills = MARKET_FIXTURE }: { reachable?: boolean; skills?: MarketSkill[] }) {
  const [query, setQuery] = useState("");
  const [owner, setOwner] = useState<string | null>(null);
  const [installed, setInstalled] = useState<string | null>(null);
  const [pending, setPending] = useState<string | null>(null);
  const rows = filterMarket(skills, query, owner);
  if (!reachable) return <p className="mt-4 text-sm text-muted-foreground">{t("install.leaderboard.empty")}</p>;
  return (
    <div className="mt-4 space-y-2" data-testid="marketplace">
      <input aria-label={t("install.search")} value={query} onChange={(event) => setQuery(event.target.value)} className="rounded-md border border-input px-2 py-1 text-sm" />
      <div className="flex flex-wrap gap-2">
        {["acme", "beta"].map((item) => (
          <button key={item} type="button" aria-pressed={owner === item} className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => setOwner(owner === item ? null : item)}>
            {t("install.owner")} {item}
          </button>
        ))}
      </div>
      <ul className="space-y-1 text-sm">
        {rows.map((skill) => (
          <li key={`${skill.owner}/${skill.name}`} className="flex items-center gap-2">
            <span>{skill.owner}/{skill.name}</span>
            <button type="button" className="rounded-md bg-primary px-2 py-1 text-primary-foreground" onClick={() => { setPending(skill.name); setInstalled(skill.name); }}>
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
      <button type="button" className="rounded-md border border-border px-2 py-1 text-sm">{t("install.loadMore")}</button>
    </div>
  );
}
