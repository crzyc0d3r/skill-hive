export interface EndpointRow {
  id: string;
  name: string;
  kind: string;
  hasSecret: boolean;
}

export const FIXTURE_ENDPOINTS: EndpointRow[] = [
  { id: "anthropic", name: "Anthropic", kind: "anthropic", hasSecret: false },
  { id: "ollama", name: "Ollama", kind: "openai_compat", hasSecret: false },
  { id: "claude-cli", name: "Claude Code", kind: "local_cli", hasSecret: false },
];
export function publicEndpoint(row: EndpointRow): Omit<EndpointRow, never> {
  return { id: row.id, name: row.name, kind: row.kind, hasSecret: row.hasSecret };
}
