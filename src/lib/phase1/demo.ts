import type { SkillCard } from "./model";

export interface DemoState {
  skills: SkillCard[];
  presets: { id: string; name: string; skillIds: string[] }[];
  agents: { key: string; name: string; installed: boolean; enabled: boolean; path: string }[];
  scanned: { name: string; tool: string }[];
}

export const DEMO: DemoState = {
  skills: [
    {
      id: "skill-research",
      name: "research-notes",
      description: "Collect sources",
      source: "local",
      tags: ["research"],
      enabled: true,
      body: "Local body\nline two",
      sourceBody: "Source body\nline two",
      agents: ["claude_code", "codex"],
    },
    {
      id: "skill-review",
      name: "review-diff",
      description: "Review a change",
      source: "git",
      tags: [],
      enabled: false,
      body: "Review locally",
      sourceBody: "Review at source",
      agents: [],
    },
    {
      id: "skill-scan",
      name: "cursor-skill",
      description: "Imported from a scan",
      source: "scan",
      tags: ["tools"],
      enabled: true,
      body: "Scan body",
      sourceBody: "Scan body",
      agents: ["claude_code"],
    },
  ],
  presets: [
    { id: "default", name: "Default", skillIds: [] },
    { id: "research", name: "Research", skillIds: ["skill-research", "skill-review"] },
  ],
  agents: [
    { key: "claude_code", name: "Claude Code", installed: true, enabled: true, path: "~/.claude/skills" },
    { key: "codex", name: "Codex", installed: true, enabled: true, path: "~/.codex/skills" },
    { key: "cursor", name: "Cursor", installed: false, enabled: true, path: "~/.cursor/skills" },
  ],
  scanned: [
    { name: "claude-skill", tool: "claude_code" },
    { name: "cursor-skill", tool: "cursor" },
  ],
};

export function demoEnabled(): boolean {
  return typeof localStorage !== "undefined" && localStorage.getItem("skill-studio-demo") === "1";
}
