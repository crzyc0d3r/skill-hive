import { warmStartWithinBudget, windowRows } from "./perf";

describe("row windows", () => {
  it("windows the library and a 5000-step transcript", () => {
    const skills = Array.from({ length: 1000 }, (_, index) => index);
    expect(windowRows(skills, 100)).toHaveLength(100);
    const steps = Array.from({ length: 5000 }, (_, index) => index);
    expect(windowRows(steps, 100)).toHaveLength(100);
    expect(warmStartWithinBudget(1499)).toBe(true);
    expect(warmStartWithinBudget(1500)).toBe(false);
  });
});
