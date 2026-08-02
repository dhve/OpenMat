import { describe, expect, it } from "vitest";
import { niceStep, niceTicks } from "./ticks";

describe("niceStep", () => {
  it("picks 5 for a range of 20 over 5 ticks", () => {
    expect(niceStep(20, 5)).toBe(5);
  });

  it("picks 0.2 for a range of 1 over 5 ticks", () => {
    expect(niceStep(1, 5)).toBeCloseTo(0.2, 10);
  });

  it("picks 1 for a range of 6 over 6 ticks", () => {
    expect(niceStep(6, 6)).toBe(1);
  });
});

describe("niceTicks", () => {
  it("generates 0..20 by 5", () => {
    expect(niceTicks(0, 20, 5)).toEqual([0, 5, 10, 15, 20]);
  });

  it("generates a fractional ladder for a 0..1 range", () => {
    const ticks = niceTicks(0, 1, 5);
    expect(ticks[0]).toBeCloseTo(0, 10);
    expect(ticks[ticks.length - 1]).toBeCloseTo(1, 10);
    expect(ticks.length).toBe(6);
  });

  it("handles negative ranges symmetrically", () => {
    expect(niceTicks(-3, 3, 6)).toEqual([-3, -2, -1, 0, 1, 2, 3]);
  });

  it("returns a single value when min equals max", () => {
    expect(niceTicks(4, 4)).toEqual([4]);
  });

  it("returns an empty array for an invalid range", () => {
    expect(niceTicks(5, 2)).toEqual([]);
  });
});
