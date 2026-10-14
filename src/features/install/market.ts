export interface MarketSkill {
  owner: string;
  name: string;
  installs: number;
}

export const MARKET_FIXTURE: MarketSkill[] = [
  { owner: "acme", name: "review", installs: 12 },
  { owner: "acme", name: "notes", installs: 4 },
  { owner: "beta", name: "lint", installs: 9 },
];

export function filterMarket(skills: MarketSkill[], query: string, owner: string | null): MarketSkill[] {
  const needle = query.trim().toLowerCase();
  return skills.filter((skill) => {
    if (owner && skill.owner !== owner) return false;
    if (!needle) return true;
    return skill.name.includes(needle) || skill.owner.includes(needle);
  });
}