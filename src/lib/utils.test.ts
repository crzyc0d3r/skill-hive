import { describe, expect, it } from "vitest";
import { cn, displayPath, formatBytes, formatDelta, formatDuration, formatPercent, formatTokens, pluralize, relativeTime, titleCase } from "./utils";

describe("utils", () => {
  it("merges class names with tailwind precedence", () => {
    expect(cn("p-2", "p-4")).toBe("p-4");
    expect(cn("a", false && "b", "c")).toBe("a c");
  });

  it("formats sizes, tokens, percentages and durations", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(2048)).toBe("2.0 KB");
    expect(formatBytes(5 * 1024 * 1024)).toBe("5.0 MB");
    expect(formatTokens(950)).toBe("950");
    expect(formatTokens(4200)).toBe("4.2k");
    expect(formatTokens(42_000)).toBe("42k");
    expect(formatPercent(0.925)).toBe("93%");
    expect(formatDelta(0.42)).toBe("+42%");
    expect(formatDelta(-0.15)).toBe("-15%");
    expect(formatDuration(850)).toBe("850 ms");
    expect(formatDuration(95_000)).toBe("1m 35s");
  });

  it("describes relative time", () => {
    const now = Date.now();
    expect(relativeTime(now - 10_000, now)).toBe("just now");
    expect(relativeTime(now - 5 * 60_000, now)).toBe("5 min ago");
    expect(relativeTime(now - 3 * 3_600_000, now)).toBe("3 h ago");
    expect(relativeTime(now - 2 * 86_400_000, now)).toBe("2 d ago");
  });

  it("pluralises, title-cases and shortens home paths", () => {
    expect(pluralize(1, "skill")).toBe("1 skill");
    expect(pluralize(3, "skill")).toBe("3 skills");
    expect(titleCase("with_skill")).toBe("With Skill");
    expect(displayPath("/Users/sam/.claude/skills/pdf")).toBe("~/.claude/skills/pdf");
    expect(displayPath("/home/sam/x")).toBe("~/x");
  });
});
