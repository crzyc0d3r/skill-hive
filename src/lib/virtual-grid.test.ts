import { designColumns } from "./shell-width";
import { columnCount, windowSlice } from "./virtual-grid";

describe("virtual grid", () => {
  it("recomputes columns from the container width", () => {
    expect(columnCount(0)).toBe(1);
    expect(columnCount(496)).toBe(1);
    expect(columnCount(796)).toBe(2);
    expect(columnCount(1136)).toBe(3);
    expect(designColumns(800)).toBe(1);
    expect(designColumns(1100)).toBe(2);
    expect(designColumns(1440)).toBe(3);
    expect(designColumns(1920)).toBe(4);
  });

  it("caps an unmeasured window so a large library does not mount every card", () => {
    const items = Array.from({ length: 520 }, (_, index) => index);
    expect(windowSlice(items, 0, 0, 196, 1)).toHaveLength(24);
    expect(windowSlice(items, 0, 600, 196, 2).length).toBeLessThan(40);
  });
});
