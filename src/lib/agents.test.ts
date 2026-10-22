import { agentCode, agentDisplayName, agentHue, detectedAgents, previewAgents, AGENT_NAMES } from "./agents";

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

  it("gives every agent a distinct two-letter mark and separates the C agents", () => {
    expect(agentCode("claude_code")).toBe("CC");
    expect(agentCode("codex")).toBe("CX");
    expect(agentCode("cursor")).toBe("CU");
    const codes = Object.keys(AGENT_NAMES).map((key) => agentCode(key));
    expect(codes.every((code) => /^[A-Z]{2}$/.test(code))).toBe(true);
    expect(new Set(codes).size).toBe(codes.length);
    const hues = [agentHue("claude_code"), agentHue("codex"), agentHue("cursor")];
    const distance = (a: number, b: number) => {
      const delta = Math.abs(a - b) % 360;
      return Math.min(delta, 360 - delta);
    };
    expect(distance(hues[0], hues[1])).toBeGreaterThanOrEqual(80);
    expect(distance(hues[0], hues[2])).toBeGreaterThanOrEqual(80);
    expect(distance(hues[1], hues[2])).toBeGreaterThanOrEqual(80);
    const all = Object.keys(AGENT_NAMES).map((key) => agentHue(key));
    expect(new Set(all).size).toBe(all.length);
  });
});
