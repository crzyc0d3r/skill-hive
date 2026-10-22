import { middleEllipsis, scanReasonIsError } from "./path-label";

describe("scan path labels", () => {
  it("keeps both ends of a long path", () => {
    const path = "/home/pat/.claude/skills/very-long-skill-name/SKILL.md";
    const label = middleEllipsis(path, 24);
    expect(label.startsWith("/home/pat")).toBe(true);
    expect(label.endsWith("SKILL.md")).toBe(true);
    expect(label.includes("…")).toBe(true);
    expect(label.length).toBeLessThanOrEqual(24);
  });

  it("styles read and parse failures the same way", () => {
    expect(scanReasonIsError("error")).toBe(true);
    expect(scanReasonIsError("parse")).toBe(true);
    expect(scanReasonIsError("")).toBe(false);
  });
});