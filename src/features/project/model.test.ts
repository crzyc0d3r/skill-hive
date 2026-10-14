import { describe, expect, it } from "vitest";
import { addSkillToAgents, dotColor, pullCenter, toggleSkill } from "./model";

const skill = {
  id: "notes",
  name: "notes",
  agents: [],
  enabled: true,
  center: "v2",
  local: "v1",
};

describe("project skill", () => {
  it("colours a project dot and pulls the center copy", () => {
    expect(dotColor(skill)).toBe("red");
    expect(dotColor(toggleSkill(skill, false))).toBe("grey");
    const shared = addSkillToAgents(skill, ["claude_code", "codex"]);
    expect(shared.agents).toEqual(["claude_code", "codex"]);
    expect(dotColor(pullCenter(shared))).toBe("green");
  });
});
