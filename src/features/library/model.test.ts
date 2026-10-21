import { fileDiffs, filterSkills, groupSkillCards, lcsDiff, moveById, presetFill, presetLabel, type SkillCard } from "./model";

const skills: SkillCard[] = [
  {
    id: "a",
    name: "Alpha",
    description: "one",
    source: "git",
    tags: ["research"],
    enabled: true,
    body: "same\nold",
    sourceBody: "same\nnew",
    agents: [],
  },
  {
    id: "b",
    name: "Beta",
    description: "two",
    source: "local",
    tags: [],
    enabled: false,
    body: "",
    sourceBody: "",
    agents: [],
  },
];

describe("library filter and diff", () => {
  it("filters by source, tag, and enabled state", () => {
    expect(filterSkills(skills, "", "enabled", "git", "research").map((skill) => skill.id)).toEqual(["a"]);
    expect(filterSkills(skills, "beta", "all", null, "untagged").map((skill) => skill.id)).toEqual(["b"]);
  });

  it("builds an LCS line diff", () => {
    expect(lcsDiff("same\nold", "same\nnew")).toEqual([
      { kind: "same", text: "same" },
      { kind: "del", text: "old" },
      { kind: "add", text: "new" },
    ]);
  });

  it("reorders by id and diffs each source file", () => {
    expect(moveById(["a", "b", "c"], "c", "a", (item) => item)).toEqual(["c", "a", "b"]);
    expect(fileDiffs([{ path: "SKILL.md", local: "left", remote: "right" }])[0].rows).toEqual([
      { kind: "del", text: "left" },
      { kind: "add", text: "right" },
    ]);
  });

  it("collapses the same skill name into one card", () => {
    const copies: SkillCard[] = [
      { ...skills[0], id: "one", name: "Autopilot", agents: ["claude_code"], locations: ["/a"] },
      { ...skills[0], id: "two", name: "autopilot", agents: ["codex"], locations: ["/b"] },
      { ...skills[1], id: "three", name: "Other", agents: [], locations: [] },
    ];
    const grouped = groupSkillCards(copies);
    expect(grouped.map((skill) => skill.id)).toEqual(["one", "three"]);
    expect(grouped[0].agents).toEqual(["claude_code", "codex"]);
    expect(grouped[0].locations).toEqual(["/a", "/b"]);
  });

  it("keeps one description when copies were stamped copy N", () => {
    const copies: SkillCard[] = [
      { ...skills[0], id: "one", name: "autopilot", description: "autopilot copy 1", agents: ["claude_code"] },
      { ...skills[0], id: "two", name: "autopilot", description: "Keep a long task moving", agents: ["codex"] },
    ];
    expect(groupSkillCards(copies)).toHaveLength(1);
    expect(groupSkillCards(copies)[0].description).toBe("Keep a long task moving");
  });

  it("labels preset fill", () => {
    expect(presetFill(2, 2)).toBe("active");
    expect(presetFill(1, 2)).toBe("partial");
    expect(presetLabel("Research", 2, 2)).toBe("Research check");
  });
});
