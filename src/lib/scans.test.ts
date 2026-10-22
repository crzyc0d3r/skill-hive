import { groupScanRows, isFreshScan } from "./scans";

describe("scan grouping", () => {
  it("marks a skill when two copies have different fingerprints", () => {
    const rows = groupScanRows([
      { name: "autopilot", tool: "claude_code", path: "/a", fingerprint: "same" },
      { name: "Autopilot", tool: "cursor", path: "/b", fingerprint: "other" },
      { name: "notes", tool: "codex", path: "/c", fingerprint: "one" },
    ]);
    expect(rows).toHaveLength(2);
    expect(rows[0]?.differs).toBe(true);
    expect(rows[0]?.tools).toEqual(["claude_code", "cursor"]);
    expect(rows[1]?.differs).toBe(false);
  });

  it("keeps identical copies as one row without a differs badge", () => {
    const rows = groupScanRows([
      { name: "autopilot", tool: "claude_code", path: "/a", fingerprint: "same" },
      { name: "autopilot", tool: "codex", path: "/b", fingerprint: "same" },
    ]);
    expect(rows).toHaveLength(1);
    expect(rows[0]?.differs).toBe(false);
    expect(rows[0]?.paths).toHaveLength(2);
  });

  it("keeps a read error ahead of a parse error", () => {
    const rows = groupScanRows([
      { name: "broken", tool: "cursor", path: "/a/SKILL.md", reason: "parse error at line 3" },
      { name: "broken", tool: "codex", path: "/b/SKILL.md", reason: "permission denied" },
    ]);
    expect(rows[0]?.reason).toBe("permission denied");
    const legacy = groupScanRows([
      { name: "broken", tool: "cursor", path: "/a/SKILL.md", reason: "parse" },
      { name: "broken", tool: "codex", path: "/b/SKILL.md", reason: "not found" },
    ]);
    expect(legacy[0]?.reason).toBe("not found");
  });

  it("does not count a read or parse failure as new", () => {
    expect(isFreshScan({ reason: "not found" })).toBe(false);
    expect(isFreshScan({ reason: "permission denied" })).toBe(false);
    expect(isFreshScan({ reason: "parse error at line 2" })).toBe(false);
    expect(isFreshScan({ reason: "error" })).toBe(false);
    expect(isFreshScan({ reason: "parse" })).toBe(false);
    expect(isFreshScan({ alreadyManaged: true })).toBe(false);
    expect(isFreshScan({})).toBe(true);
  });
});
