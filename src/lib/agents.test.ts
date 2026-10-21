import { agentDisplayName, detectedAgents, previewAgents } from "./agents";

describe("agent display", () => {
  it("never returns a raw adapter id", () => {
    expect(agentDisplayName("claude_code")).toBe("Claude Code");
    expect(agentDisplayName("iflow")).toBe("iFlow CLI");
    expect(agentDisplayName("adal")).toBe("AdaL");
    expect(agentDisplayName("custom_agent")).toBe("Custom Agent");
    expect(agentDisplayName("claude_code")).not.toContain("_");
  });

  it("hides undetected adapters until show-all is requested", () => {
    const agents = previewAgents();
    expect(agents.length).toBeGreaterThan(40);
    const detected = detectedAgents(agents);
    expect(detected.map((agent) => agent.name)).toEqual([
      "Cursor",
      "Claude Code",
      "Codex",
      "Antigravity",
      "Amp",
      "Gemini CLI",
      "GitHub Copilot",
      "Windsurf",
      "Augment",
      "iFlow CLI",
    ]);
    expect(detected.every((agent) => !agent.name.includes("_"))).toBe(true);
  });
});
