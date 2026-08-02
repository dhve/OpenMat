import { describe, expect, it } from "vitest";
import { integratePendulum } from "./rk4";

describe("integratePendulum", () => {
  it("starts at the given initial condition", () => {
    const samples = integratePendulum({ c: 0.3, x0: 2, v0: 0, t0: 0, t1: 20, steps: 400 });
    expect(samples[0]).toEqual({ t: 0, x: 2 });
    expect(samples.length).toBe(401);
  });

  it("matches the small-angle harmonic approximation for a short time with no damping", () => {
    // For small x, sin(x) ~ x, so x'' + x = 0 has solution x(t) = x0 cos(t).
    // Use a small amplitude and a short window so the approximation holds.
    const x0 = 0.05;
    const samples = integratePendulum({ c: 0, x0, v0: 0, t0: 0, t1: 2, steps: 800 });
    const last = samples[samples.length - 1];
    const expected = x0 * Math.cos(2);
    expect(last.x).toBeCloseTo(expected, 3);
  });

  it("dissipates energy with positive damping: amplitude decays over time", () => {
    const samples = integratePendulum({ c: 0.5, x0: 2, v0: 0, t0: 0, t1: 30, steps: 1500 });
    const earlyPeak = Math.max(...samples.slice(0, 300).map((s) => Math.abs(s.x)));
    const latePeak = Math.max(...samples.slice(1200).map((s) => Math.abs(s.x)));
    expect(latePeak).toBeLessThan(earlyPeak);
  });

  it("conserves energy approximately with zero damping over one period", () => {
    // With c = 0, the pendulum is conservative. Energy E = v^2/2 - cos(x)
    // should return close to its initial value after the state returns near
    // its starting point (checked via a bound on drift, not equality).
    const samples = integratePendulum({ c: 0, x0: 1, v0: 0, t0: 0, t1: 10, steps: 2000 });
    const energyAt = (x: number, v: number) => v * v / 2 - Math.cos(x);
    const e0 = energyAt(samples[0].x, 0);
    // Reconstruct v at the end via a finite difference of the last two samples.
    const n = samples.length;
    const h = samples[1].t - samples[0].t;
    const vEnd = (samples[n - 1].x - samples[n - 2].x) / h;
    const eEnd = energyAt(samples[n - 1].x, vEnd);
    expect(Math.abs(eEnd - e0)).toBeLessThan(0.01);
  });

  it("rejects a non-positive step count", () => {
    expect(() => integratePendulum({ c: 0, x0: 0, v0: 0, t0: 0, t1: 1, steps: 0 })).toThrow();
  });
});
