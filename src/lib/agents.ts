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

/**
 * Two-letter marks. Claude Code, Codex, and Cursor are CC, CX, and CU
 * so three agents that all start with C stay readable next to each other.
 */
const AGENT_CODES: Record<string, string> = {
  cursor: "CU",
  claude_code: "CC",
  omp_agent: "OA",
  codex: "CX",
  grok: "GK",
  opencode: "OC",
  antigravity: "AG",
  amp: "AM",
  kilo_code: "KC",
  roo_code: "RC",
  goose: "GS",
  gemini_cli: "GM",
  github_copilot: "CP",
  openclaw: "OW",
  droid: "DR",
  windsurf: "WS",
  trae: "TR",
  cline: "CL",
  deepagents: "DA",
  firebender: "FB",
  kimi: "KM",
  replit: "RP",
  warp: "WP",
  augment: "AU",
  bob: "IB",
  codebuddy: "CB",
  command_code: "CD",
  continue: "CN",
  cortex: "CT",
  crush: "CR",
  iflow: "IF",
  junie: "JN",
  kiro: "KI",
  kode: "KD",
  mcpjam: "MJ",
  mistral_vibe: "MV",
  mux: "MX",
  neovate: "NE",
  openhands: "OH",
  pi: "PI",
  pochi: "PO",
  qoder: "QD",
  qwen_code: "QW",
  trae_cn: "TN",
  zencoder: "ZE",
  zcode: "ZC",
  adal: "AD",
  hermes: "HE",
  qclaw: "QC",
  easyclaw: "EY",
  autoclaw: "AC",
  workbuddy: "WB",
  deepseek_harness: "DH",
  gitlab_duo: "GD",
};

/** Featured agents sit on hues far apart from each other. The rest fill the wheel. */
const FEATURED_HUES: Record<string, number> = {
  cursor: 262,
  claude_code: 28,
  codex: 152,
  antigravity: 320,
  amp: 200,
  gemini_cli: 78,
  github_copilot: 285,
  windsurf: 185,
  augment: 110,
  iflow: 345,
};

function hashKey(key: string): number {
  let hash = 0;
  for (const char of key) hash = (hash * 33 + char.charCodeAt(0)) >>> 0;
  return hash;
}

function hueDistance(a: number, b: number): number {
  const delta = Math.abs(a - b) % 360;
  return Math.min(delta, 360 - delta);
}

function placeHue(seed: number, used: number[]): number {
  let hue = Math.round(seed % 360);
  let guard = 0;
  while (used.some((other) => hueDistance(other, hue) < 8) && guard < 45) {
    hue = (hue + 11) % 360;
    guard += 1;
  }
  return hue;
}

function buildHues(): Record<string, number> {
  const hues: Record<string, number> = { ...FEATURED_HUES };
  const used = new Set(Object.values(FEATURED_HUES));
  const rest = Object.keys(AGENT_NAMES).filter((key) => hues[key] == null);
  rest.forEach((key, index) => {
    let hue = Math.round((index * 360) / Math.max(rest.length, 1)) % 360;
    let guard = 0;
    while (used.has(hue) && guard < 360) {
      hue = (hue + 1) % 360;
      guard += 1;
    }
    hues[key] = hue;
    used.add(hue);
  });
  return hues;
}

let agentHues: Record<string, number> | null = null;

function hues(): Record<string, number> {
  agentHues ??= buildHues();
  return agentHues;
}

export function agentCode(key: string): string {
  const known = AGENT_CODES[key];
  if (known) return known;
  const parts = key.split(/[_-]+/).filter(Boolean);
  const raw = (parts.length >= 2 ? `${parts[0][0] ?? ""}${parts[1][0] ?? ""}` : (parts[0] ?? "x").slice(0, 2)).toUpperCase();
  const base = (raw + "X").slice(0, 2);
  const taken = new Set(Object.values(AGENT_CODES));
  if (!taken.has(base)) return base;
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
  const shift = hashKey(key);
  for (let step = 0; step < 26; step += 1) {
    const next = `${base[0]}${alphabet[(shift + step) % 26]}`;
    if (!taken.has(next)) return next;
  }
  return base;
}

/** Stable hue. The same agent always gets the same color, and known agents stay apart. */
export function agentHue(key: string): number {
  const table = hues();
  if (table[key] != null) return table[key];
  return placeHue(hashKey(key) * 137.508, Object.values(table));
}

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
