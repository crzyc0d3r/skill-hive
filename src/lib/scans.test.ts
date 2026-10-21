import { groupScanRows } from "./scans";

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
});
