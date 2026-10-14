import { describe, expect, it } from "vitest";
import { evidenceAnchor, moveCell, publicEndpoint, stepDuration } from "./model";
import { exportEndpoints, resetPhase4, setEndpointSecret } from "./store";

describe("phase 4 results model", () => {
  it("measures a tool from use to result", () => {
    expect(stepDuration({ id: 2, tool: "Write", useTs: 10, resultTs: 40 })).toBe(30);
  });

  it("resolves a step number or an output path", () => {
    expect(evidenceAnchor("2")).toBe("step-2");
    expect(evidenceAnchor("outputs/answer.txt")).toBe("outputs/answer.txt");
    expect(evidenceAnchor("missing")).toBeNull();
  });

  it("moves the heatmap with the arrow keys", () => {
    expect(moveCell(0, "ArrowRight", 4)).toBe(1);
    expect(moveCell(1, "ArrowLeft", 4)).toBe(0);
    expect(moveCell(0, "ArrowDown", 4)).toBe(2);
  });

  it("keeps the saved secret out of the export", () => {
    resetPhase4();
    setEndpointSecret("sk-ant-phase4-fixture-secret");
    const body = JSON.stringify(exportEndpoints());
    expect(body).not.toContain("sk-ant-phase4-fixture-secret");
    expect(body).toContain("hasSecret");
    expect(publicEndpoint(exportEndpoints()[0]).hasSecret).toBe(true);
    resetPhase4();
  });
});
