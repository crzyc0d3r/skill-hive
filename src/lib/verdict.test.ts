import { scoreDelta, verdictFromDelta } from "./verdict";

describe("verdict cutoffs", () => {
  it("uses plus or minus 5 and 20 points", () => {
    expect(verdictFromDelta(20)).toBe("helps-lot");
    expect(verdictFromDelta(19)).toBe("helps");
    expect(verdictFromDelta(5)).toBe("helps");
    expect(verdictFromDelta(4.9)).toBe("neutral");
    expect(verdictFromDelta(0)).toBe("neutral");
    expect(verdictFromDelta(-4.9)).toBe("neutral");
    expect(verdictFromDelta(-5)).toBe("bit-worse");
    expect(verdictFromDelta(-19)).toBe("bit-worse");
    expect(verdictFromDelta(-20)).toBe("much-worse");
    expect(verdictFromDelta(null)).toBe("neutral");
  });

  it("reads the value-added axis as the point delta", () => {
    const scores = scoreDelta([
      { name: "With-skill task pass rate", value: 100 },
      { name: "Value added over without_skill", value: 75 },
    ]);
    expect(scores.delta).toBe(75);
    expect(scores.withScore).toBe(100);
    expect(scores.withoutScore).toBe(25);
    expect(verdictFromDelta(scores.delta)).toBe("helps-lot");
  });
});
