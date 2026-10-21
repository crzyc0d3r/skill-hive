import { DEMO } from "@/lib/demo";
import type { SkillCard } from "@/features/library/model";

export function previewLibrary(): SkillCard[] {
  const autopilot: SkillCard = {
    id: "skill-autopilot",
    name: "Autopilot",
    description: "Keep a task moving",
    source: "local",
    tags: ["tools"],
    enabled: true,
    body: "Autopilot local body",
    sourceBody: "Autopilot source body",
    agents: ["claude_code", "codex", "cursor"],
    locations: [
      "~/.claude/skills/autopilot",
      "~/.codex/skills/autopilot",
      "~/.cursor/skills/autopilot",
    ],
  };
  return [autopilot, ...DEMO.skills];
}

const DUPLICATES = ["autopilot", "azure-outage", "bigquery-sql", "breakit"] as const;
const DUPLICATE_DESCRIPTIONS: Record<(typeof DUPLICATES)[number], string> = {
  autopilot: "Keep a long task moving without stopping for every confirmation.",
  "azure-outage": "Check Azure service health and draft an incident note.",
  "bigquery-sql": "Write and explain BigQuery SQL for a question.",
  breakit: "Find a small breaking case for the change under review.",
};
const HOSTS = ["claude_code", "codex", "cursor", "gemini_cli", "github_copilot", "windsurf", "amp", "antigravity", "augment", "iflow"];

export function crowdEnabled(): boolean {
  return typeof localStorage !== "undefined" && localStorage.getItem("skill-hive-crowd") === "1";
}

/** 520 library rows: four names repeated across agent dirs, plus unique skills. */
export function crowdSkills(): SkillCard[] {
  const rows: SkillCard[] = [];
  for (const name of DUPLICATES) {
    HOSTS.slice(0, 4).forEach((agent, index) => {
      rows.push({
        id: `${name}-${agent}`,
        name,
        description: DUPLICATE_DESCRIPTIONS[name],
        source: "local",
        tags: index === 0 ? ["tools"] : [],
        enabled: true,
        body: `${name} body`,
        sourceBody: `${name} body`,
        agents: [agent],
        locations: [`~/.${agent}/skills/${name}`],
      });
    });
  }
  const target = 520;
  let n = 0;
  while (rows.length < target) {
    const agent = HOSTS[n % HOSTS.length];
    const name = `crowd-skill-${n}`;
    rows.push({
      id: name,
      name,
      description: `Generated skill ${n}`,
      source: n % 5 === 0 ? "git" : "local",
      tags: [],
      enabled: n % 3 !== 0,
      body: `body ${n}`,
      sourceBody: `body ${n}`,
      agents: [agent],
      locations: [`~/.${agent}/skills/${name}`],
    });
    n += 1;
  }
  return rows;
}
