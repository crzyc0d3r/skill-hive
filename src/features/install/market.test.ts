import { describe, expect, it } from "vitest";
import { filterMarket, MARKET_FIXTURE } from "./market";
import { applyRemoval, removalPlan } from "@/features/library/updates";

describe("marketplace", () => {
  it("filters marketplace rows and applies a removal token", () => {
    expect(filterMarket(MARKET_FIXTURE, "rev", "acme").map((row) => row.name)).toEqual(["review"]);
    const plan = removalPlan(["notes.txt"]);
    expect(applyRemoval(plan, null)).toBe(false);
    expect(applyRemoval(plan, plan.token)).toBe(true);
  });
});
