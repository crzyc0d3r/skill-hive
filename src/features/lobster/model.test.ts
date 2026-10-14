import { describe, expect, it } from "vitest";
import { lobsterAgents } from "./model";

describe("lobster agents", () => {
  it("hides lobster agents until the home marker exists", () => {
    expect(lobsterAgents([])).toEqual([]);
    expect(lobsterAgents([".openclaw"]).map((row) => row.name)).toEqual(["OpenClaw"]);
  });
});
