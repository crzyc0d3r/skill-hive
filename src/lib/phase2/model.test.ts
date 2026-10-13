import { describe, expect, it } from "vitest";
import {
  addSkillToAgents,
  applyRemoval,
  backupNow,
  dotColor,
  effectiveClose,
  filterMarket,
  lobsterAgents,
  MARKET_FIXTURE,
  proxyOk,
  pullCenter,
  removalPlan,
  restoreSnapshot,
  toggleSkill,
} from "./model";

const skill = {
  id: "notes",
  name: "notes",
  agents: [],
  enabled: true,
  center: "v2",
  local: "v1",
};

describe("phase 2 model", () => {
  it("colours a project dot and pulls the center copy", () => {
    expect(dotColor(skill)).toBe("red");
    expect(dotColor(toggleSkill(skill, false))).toBe("grey");
    const shared = addSkillToAgents(skill, ["claude_code", "codex"]);
    expect(shared.agents).toEqual(["claude_code", "codex"]);
    expect(dotColor(pullCenter(shared))).toBe("green");
  });

  it("filters marketplace rows and applies a removal token", () => {
    expect(filterMarket(MARKET_FIXTURE, "rev", "acme").map((row) => row.name)).toEqual(["review"]);
    const plan = removalPlan(["notes.txt"]);
    expect(applyRemoval(plan, null)).toBe(false);
    expect(applyRemoval(plan, plan.token)).toBe(true);
  });

  it("hides lobster agents until the home marker exists", () => {
    expect(lobsterAgents([])).toEqual([]);
    expect(lobsterAgents([".openclaw"]).map((row) => row.name)).toEqual(["OpenClaw"]);
  });

  it("restores a snapshot behind a safety tag and checks proxy URLs", () => {
    const history = backupNow("library v1", []);
    expect(restoreSnapshot(history, "snap-1")?.safety).toBe("safety-snap-1");
    expect(proxyOk("")).toBe(true);
    expect(proxyOk("http://127.0.0.1:8888")).toBe(true);
    expect(proxyOk("ftp://example")).toBe(false);
    expect(effectiveClose("minimize", false)).toBe("ask");
  });
});
