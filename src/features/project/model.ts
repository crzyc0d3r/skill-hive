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
