// The mock engine implements the same evaluate() contract as the real
// openmat-kernel (see ARCHITECTURE.md), so the UI runs standalone today and
// swaps to tauriEngine with no call-site changes once the kernel is wired.
//
// It understands exactly two shapes of input:
//   1. The damped pendulum NDSolve form, matched structurally rather than by
//      exact string equality so small formatting differences from the
//      translator still work.
//   2. Plain arithmetic, handled by exprEvaluator.
// Anything else returns a clear, human readable error.

import type { EvalResult } from "./types";
import { integratePendulum } from "./rk4";
import { evaluateArithmetic, formatNumber, ExprEvalError } from "./exprEvaluator";

const PENDULUM_T0 = 0;
const PENDULUM_T1 = 20;
const PENDULUM_STEPS = 400;
const PENDULUM_X0 = 2;
const PENDULUM_V0 = 0;

// Matches the damping term "<number> x'[t]" (or "-<number> x'[t]", or a bare
// "x'[t]" meaning coefficient 1) inside an NDSolve pendulum equation of the
// form: x''[t] + c x'[t] + Sin[x[t]] == 0. Whitespace is tolerant.
const PENDULUM_PATTERN =
  /x''\[t\]\s*\+\s*([+-]?\s*\d*\.?\d*)\s*x'\[t\]\s*\+\s*Sin\[x\[t\]\]\s*==\s*0/;

function extractDampingCoefficient(input: string): number | null {
  const match = PENDULUM_PATTERN.exec(input);
  if (!match) return null;
  const raw = match[1].replace(/\s+/g, "");
  if (raw === "" || raw === "+") return 1;
  if (raw === "-") return -1;
  return parseFloat(raw);
}

function solvePendulum(c: number): EvalResult {
  const samples = integratePendulum({
    c,
    x0: PENDULUM_X0,
    v0: PENDULUM_V0,
    t0: PENDULUM_T0,
    t1: PENDULUM_T1,
    steps: PENDULUM_STEPS,
  });

  const xValues = samples.map((s) => s.x);
  const yMin = Math.min(...xValues);
  const yMax = Math.max(...xValues);
  const pad = Math.max(0.2, (yMax - yMin) * 0.1);

  const cLabel = formatNumber(Math.round(c * 1000) / 1000);
  return {
    latex: `x''(t) + ${cLabel}\\,x'(t) + \\sin(x(t)) = 0`,
    plot: {
      curves: [
        {
          points: samples.map((s) => [s.t, s.x] as [number, number]),
          label: "x(t)",
        },
      ],
      x_range: [PENDULUM_T0, PENDULUM_T1],
      y_range: [yMin - pad, yMax + pad],
    },
  };
}

export async function evaluate(input: string): Promise<EvalResult> {
  const trimmed = input.trim();
  if (trimmed === "") {
    return { latex: "", error: "Nothing to evaluate." };
  }

  const c = extractDampingCoefficient(trimmed);
  if (c !== null) {
    return solvePendulum(c);
  }

  try {
    const value = evaluateArithmetic(trimmed);
    return { latex: formatNumber(value) };
  } catch (err) {
    const message = err instanceof ExprEvalError ? err.message : "Could not evaluate this input.";
    return {
      latex: "",
      error: `${message}. The demo engine currently understands plain arithmetic and the damped pendulum NDSolve form.`,
    };
  }
}
