import { describe, expect, it } from "vitest";
import { evaluate } from "./mockEngine";

describe("mockEngine.evaluate", () => {
  it("evaluates plain arithmetic", async () => {
    const result = await evaluate("2 + 3 * 4");
    expect(result.error).toBeUndefined();
    expect(result.latex).toBe("14");
    expect(result.plot).toBeUndefined();
  });

  it("solves the damped pendulum and returns a plot", async () => {
    const input = "NDSolve[{x''[t] + 0.3 x'[t] + Sin[x[t]] == 0, x[0] == 2, x'[0] == 0}, x, {t, 0, 20}]";
    const result = await evaluate(input);
    expect(result.error).toBeUndefined();
    expect(result.plot).toBeDefined();
    expect(result.plot!.curves).toHaveLength(1);
    expect(result.plot!.curves[0].points[0]).toEqual([0, 2]);
    expect(result.plot!.x_range).toEqual([0, 20]);
    expect(result.latex).toContain("0.3");
  });

  it("recognizes the bare pendulum equation without the NDSolve wrapper", async () => {
    const result = await evaluate("x''[t] + 1.2 x'[t] + Sin[x[t]] == 0");
    expect(result.plot).toBeDefined();
  });

  it("treats a coefficient-free damping term as c = 1", async () => {
    const result = await evaluate("x''[t] + x'[t] + Sin[x[t]] == 0");
    expect(result.plot).toBeDefined();
    expect(result.latex).toContain("1");
  });

  it("damps more with a larger coefficient: late amplitude shrinks faster", async () => {
    const low = await evaluate("x''[t] + 0.1 x'[t] + Sin[x[t]] == 0");
    const high = await evaluate("x''[t] + 1.5 x'[t] + Sin[x[t]] == 0");
    const lastPoints = (r: typeof low) => r.plot!.curves[0].points.slice(-20).map((p) => Math.abs(p[1]));
    const lowTail = Math.max(...lastPoints(low));
    const highTail = Math.max(...lastPoints(high));
    expect(highTail).toBeLessThan(lowTail);
  });

  it("returns a human readable error for unsupported input", async () => {
    const result = await evaluate("Integrate[x, x]");
    expect(result.error).toBeDefined();
    expect(result.error).not.toMatch(/undefined|NaN/);
  });

  it("returns an error for empty input", async () => {
    const result = await evaluate("   ");
    expect(result.error).toBeDefined();
  });
});
