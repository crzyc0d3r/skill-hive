export interface ProjectSkill {
  id: string;
  name: string;
  agents: string[];
  enabled: boolean;
  center: string;
  local: string;
}

export type DotColor = "green" | "amber" | "red" | "grey";

export function dotColor(skill: ProjectSkill): DotColor {
  if (!skill.enabled) return "grey";
  if (skill.center === skill.local) return "green";
  if (skill.center && skill.local) return "red";
  return "amber";
}

export function worstDot(skills: ProjectSkill[]): DotColor {
  const rank: Record<DotColor, number> = { green: 0, grey: 1, amber: 2, red: 3 };
  return skills.reduce<DotColor>((worst, skill) => (rank[dotColor(skill)] > rank[worst] ? dotColor(skill) : worst), "green");
}

export function addSkillToAgents(skill: ProjectSkill, agents: string[]): ProjectSkill {
  return { ...skill, agents: [...new Set([...skill.agents, ...agents])], enabled: true };
}

export function toggleSkill(skill: ProjectSkill, enabled: boolean): ProjectSkill {
  return { ...skill, enabled };
}

export function pullCenter(skill: ProjectSkill): ProjectSkill {
  return { ...skill, local: skill.center };
}

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

export interface RemovalPlan {
  paths: string[];
  token: string;
}

export function removalPlan(paths: string[]): RemovalPlan {
  return { paths, token: paths.join("|") };
}

export function applyRemoval(plan: RemovalPlan, token: string | null): boolean {
  return token === plan.token;
}

export type CloseChoice = "ask" | "minimize" | "quit";

export function effectiveClose(choice: CloseChoice, trayEnabled: boolean): CloseChoice {
  if (!trayEnabled && choice === "minimize") return "ask";
  return choice;
}

export const LOBSTER_KEYS = ["openclaw"] as const;

export function isLobsterKey(key: string): boolean {
  return (LOBSTER_KEYS as readonly string[]).includes(key);
}

export function lobsterAgents(homeEntries: string[]): { key: string; name: string }[] {
  return homeEntries.includes(".openclaw") ? [{ key: "openclaw", name: "OpenClaw" }] : [];
}

export const HELP_CARDS = [
  "help.card.library",
  "help.card.install",
  "help.card.workspace",
  "help.card.project",
  "help.card.backup",
  "help.card.updates",
  "help.card.tray",
  "help.card.cli",
] as const;

export interface BackupSnapshot {
  id: string;
  label: string;
  body: string;
}

export function backupNow(body: string, history: BackupSnapshot[]): BackupSnapshot[] {
  return [{ id: `snap-${history.length + 1}`, label: `Snapshot ${history.length + 1}`, body }, ...history];
}

export function proxyOk(value: string): boolean {
  const trimmed = value.trim();
  if (!trimmed) return true;
  try {
    const url = new URL(trimmed);
    return url.protocol === "http:" || url.protocol === "https:";
  } catch {
    return false;
  }
}

export function restoreSnapshot(history: BackupSnapshot[], id: string): { body: string; safety: string } | null {
  const snap = history.find((item) => item.id === id);
  if (!snap) return null;
  return { body: snap.body, safety: `safety-${id}` };
}
