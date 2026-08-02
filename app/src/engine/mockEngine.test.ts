import { describe, expect, it } from "vitest";
import { evaluate } from "./mockEngine";
import { kernelResultToView } from "./types";

describe("mockEngine.evaluate", () => {
  it("evaluates plain arithmetic", async () => {
    const result = await evaluate("2 + 3 * 4", {}, 1);
    expect(result.status).toBe("ok");
    expect(result.request_id).toBe(1);
    const view = kernelResultToView(result);
    expect(view.error).toBeUndefined();
    expect(view.latex).toBe("14");
    expect(view.plot).toBeUndefined();
  });

  it("solves the damped pendulum with a bound coefficient and returns a plot", async () => {
    const input = "NDSolve[{x''[t] + c x'[t] + Sin[x[t]] == 0, x[0] == 2, x'[0] == 0}, x, {t, 0, 20}]";
    const result = await evaluate(input, { c: 0.3 }, 1);
    expect(result.status).toBe("ok");
    const view = kernelResultToView(result);
    expect(view.error).toBeUndefined();
    expect(view.plot).toBeDefined();
    expect(view.plot!.curves).toHaveLength(1);
    expect(view.plot!.curves[0].points[0]).toEqual([0, 2]);
    expect(view.plot!.x_range).toEqual([0, 20]);
    expect(view.latex).toContain("0.3");
  });

  it("recognizes the bare pendulum equation without the NDSolve wrapper", async () => {
    const result = await evaluate("x''[t] + 1.2 x'[t] + Sin[x[t]] == 0", {}, 1);
    expect(kernelResultToView(result).plot).toBeDefined();
  });

  it("treats a coefficient-free damping term as c = 1", async () => {
    const result = await evaluate("x''[t] + x'[t] + Sin[x[t]] == 0", {}, 1);
    const view = kernelResultToView(result);
    expect(view.plot).toBeDefined();
    expect(view.latex).toContain("1");
  });

  it("damps more with a larger coefficient: late amplitude shrinks faster", async () => {
    const low = kernelResultToView(await evaluate("x''[t] + 0.1 x'[t] + Sin[x[t]] == 0", {}, 1));
    const high = kernelResultToView(await evaluate("x''[t] + 1.5 x'[t] + Sin[x[t]] == 0", {}, 2));
    const lastPoints = (r: typeof low) => r.plot!.curves[0].points.slice(-20).map((p) => Math.abs(p[1]));
    const lowTail = Math.max(...lastPoints(low));
    const highTail = Math.max(...lastPoints(high));
    expect(highTail).toBeLessThan(lowTail);
  });

  it("solves for the same equation text using different bindings, unchanged text", async () => {
    const input = "NDSolve[{x''[t] + c x'[t] + Sin[x[t]] == 0, x[0] == 2, x'[0] == 0}, x, {t, 0, 20}]";
    const low = kernelResultToView(await evaluate(input, { c: 0.1 }, 1));
    const high = kernelResultToView(await evaluate(input, { c: 1.5 }, 2));
    const lastPoints = (r: typeof low) => r.plot!.curves[0].points.slice(-20).map((p) => Math.abs(p[1]));
    expect(Math.max(...lastPoints(high))).toBeLessThan(Math.max(...lastPoints(low)));
  });

  it("returns a solve error naming the symbol when the damping coefficient is an unbound binding", async () => {
    const input = "NDSolve[{x''[t] + c x'[t] + Sin[x[t]] == 0, x[0] == 2, x'[0] == 0}, x, {t, 0, 20}]";
    const result = await evaluate(input, {}, 1);
    expect(result.status).toBe("error");
    expect(result.error?.message).toContain("c");
  });

  it("returns a human readable error for unsupported input", async () => {
    const result = await evaluate("Integrate[x, x]", {}, 1);
    expect(result.status).toBe("error");
    expect(result.error?.message).not.toMatch(/undefined|NaN/);
  });

  it("returns an error for empty input", async () => {
    const result = await evaluate("   ", {}, 1);
    expect(result.status).toBe("error");
  });

  it("echoes the request id", async () => {
    const result = await evaluate("1 + 1", {}, 77);
    expect(result.request_id).toBe(77);
  });
});
