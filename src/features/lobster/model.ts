export const LOBSTER_KEYS = ["openclaw"] as const;

export function isLobsterKey(key: string): boolean {
  return (LOBSTER_KEYS as readonly string[]).includes(key);
}

export function lobsterAgents(homeEntries: string[]): { key: string; name: string }[] {
  return homeEntries.includes(".openclaw") ? [{ key: "openclaw", name: "OpenClaw" }] : [];
}
