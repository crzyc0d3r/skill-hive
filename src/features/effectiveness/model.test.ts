import { describe, expect, it } from "vitest";
import { RADAR, SAMPLE_REPORT, axisLabelLines, radarLabelAnchor, shortAxis } from "@/features/effectiveness/model";

describe("radar axis labels", () => {
  it("wraps multi-word labels and keeps every word inside the viewBox", () => {
    const axes = SAMPLE_REPORT.axes;
    expect(axisLabelLines(shortAxis("With-skill task pass rate"))).toEqual(["With", "skill"]);
    expect(axisLabelLines(shortAxis("Weak-model performance"))).toEqual(["Smaller", "model"]);
    expect(axisLabelLines(shortAxis("Survival in a crowded library"))).toEqual(["Survival"]);

    for (const [index, axis] of axes.entries()) {
      const lines = axisLabelLines(shortAxis(axis.name));
      const place = radarLabelAnchor(index, axes.length, lines.length);
      const width = Math.max(...lines.map((line) => line.length * RADAR.fontSize * 0.72));
      const top = place.y - RADAR.fontSize * 0.85;
      const bottom = place.y + (lines.length - 1) * RADAR.lineHeight + RADAR.fontSize * 0.28;
      const left = place.anchor === "start" ? place.x : place.anchor === "end" ? place.x - width : place.x - width / 2;
      const right = place.anchor === "start" ? place.x + width : place.anchor === "end" ? place.x : place.x + width / 2;
      expect(left, axis.name).toBeGreaterThanOrEqual(8);
      expect(top, axis.name).toBeGreaterThanOrEqual(8);
      expect(right, axis.name).toBeLessThanOrEqual(RADAR.size - 8);
      expect(bottom, axis.name).toBeLessThanOrEqual(RADAR.size - 8);
      if (place.anchor === "start") expect(place.x, axis.name).toBeGreaterThan(RADAR.center);
      if (place.anchor === "end") expect(place.x, axis.name).toBeLessThan(RADAR.center);
    }
  });
});
