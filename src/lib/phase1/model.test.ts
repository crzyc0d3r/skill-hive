import { fileDiffs, filterSkills, lcsDiff, moveById, presetFill, presetLabel, type SkillCard } from "./model";

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

  it("labels preset fill", () => {
    expect(presetFill(2, 2)).toBe("active");
    expect(presetFill(1, 2)).toBe("partial");
    expect(presetLabel("Research", 2, 2)).toBe("Research check");
  });
});
