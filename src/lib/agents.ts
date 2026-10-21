import { Bot, Code2, Gem, Terminal, Wind, type LucideIcon } from "lucide-react";
import type { AgentRow } from "@/lib/ipc/catalog";

const ICONS: Record<string, LucideIcon> = {
  claude_code: Bot,
  codex: Terminal,
  cursor: Code2,
  gemini_cli: Gem,
  github_copilot: Bot,
  windsurf: Wind,
};

/** Display names for the built-in adapters. Visible UI never prints the key. */
export const AGENT_NAMES: Record<string, string> = {
  cursor: "Cursor",
  claude_code: "Claude Code",
  omp_agent: "OMP Agent",
  codex: "Codex",
  grok: "Grok",
  opencode: "OpenCode",
  antigravity: "Antigravity",
  amp: "Amp",
  kilo_code: "Kilo Code",
  roo_code: "Roo Code",
  goose: "Goose",
  gemini_cli: "Gemini CLI",
  github_copilot: "GitHub Copilot",
  openclaw: "OpenClaw",
  droid: "Droid",
  windsurf: "Windsurf",
  trae: "TRAE IDE",
  cline: "Cline",
  deepagents: "Deep Agents",
  firebender: "Firebender",
  kimi: "Kimi Code CLI",
  replit: "Replit",
  warp: "Warp",
  augment: "Augment",
  bob: "IBM Bob",
  codebuddy: "CodeBuddy",
  command_code: "Command Code",
  continue: "Continue",
  cortex: "Cortex Code",
  crush: "Crush",
  iflow: "iFlow CLI",
  junie: "Junie",
  kiro: "Kiro CLI",
  kode: "Kode",
  mcpjam: "MCPJam",
  mistral_vibe: "Mistral Vibe",
  mux: "Mux",
  neovate: "Neovate",
  openhands: "OpenHands",
  pi: "Pi",
  pochi: "Pochi",
  qoder: "Qoder",
  qwen_code: "Qwen Code",
  trae_cn: "TRAE CN",
  zencoder: "Zencoder",
  zcode: "ZCode",
  adal: "AdaL",
  hermes: "Hermes Agent",
  qclaw: "QClaw",
  easyclaw: "EasyClaw",
  autoclaw: "AutoClaw",
  workbuddy: "WorkBuddy",
  deepseek_harness: "DeepSeek Harness",
  gitlab_duo: "GitLab Duo",
};

const INSTALLED_ON_THIS_MACHINE = new Set([
  "claude_code",
  "codex",
  "cursor",
  "gemini_cli",
  "github_copilot",
  "windsurf",
  "amp",
  "antigravity",
  "augment",
  "iflow",
]);

export function agentIcon(key: string): LucideIcon {
  return ICONS[key] ?? Bot;
}

export function agentDisplayName(key: string, agents: Pick<AgentRow, "key" | "name">[] = []): string {
  const known = agents.find((agent) => agent.key === key)?.name;
  if (known && known.trim() && known !== key) return known;
  if (AGENT_NAMES[key]) return AGENT_NAMES[key];
  return key
    .split(/[_-]+/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

export function detectedAgents(agents: AgentRow[]): AgentRow[] {
  return agents.filter((agent) => agent.installed);
}

export function previewAgentsEnabled(): boolean {
  return typeof localStorage !== "undefined" && (localStorage.getItem("skill-hive-preview-agents") === "1" || localStorage.getItem("skill-hive-crowd") === "1");
}

export function previewAgents(): AgentRow[] {
  return Object.entries(AGENT_NAMES).map(([key, name]) => {
    const folder = name.replace(/\s+/g, "-");
    return {
      key,
      name,
      installed: INSTALLED_ON_THIS_MACHINE.has(key),
      enabled: true,
      path: `~/agents/${folder}/skills`,
      projectPath: `.agents/${folder}/skills`,
      custom: false,
    };
  });
}
