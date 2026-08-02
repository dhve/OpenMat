// The mock engine implements the same evaluate() contract as the real
// openmat-kernel (see ARCHITECTURE.md, "Kernel API"), so the UI runs
// standalone today and swaps to tauriEngine with no call-site changes once
// the kernel is wired up (see engine/index.ts).
//
// It understands exactly two shapes of input:
//   1. The damped pendulum NDSolve form, matched structurally rather than by
//      exact string equality so small formatting differences from the
//      translator still work. The damping coefficient may be a numeric
//      literal or a bare symbol (e.g. "c"); a symbol is looked up in
//      `bindings`, never text-substituted, matching how the real kernel
//      binds Manipulate parameters.
//   2. Plain arithmetic, handled by exprEvaluator (bindings do not apply).
// Anything else returns a clear, human readable KernelResult error.

import type { Bindings, KernelResult } from "./types";
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
const PENDULUM_NUMERIC_PATTERN = /x''\[t\]\s*\+\s*([+-]?\s*\d*\.?\d*)\s*x'\[t\]\s*\+\s*Sin\[x\[t\]\]\s*==\s*0/;

// Matches the same shape with the damping coefficient left as a bare symbol
// (the pendulum cell's normal form now that c is never text-substituted;
// see notebook/buildInput.ts). Tried only after the numeric pattern fails to
// match, so a numeric coefficient is never misread as a one-letter symbol.
const PENDULUM_SYMBOLIC_PATTERN = /x''\[t\]\s*\+\s*([A-Za-z]\w*)\s*x'\[t\]\s*\+\s*Sin\[x\[t\]\]\s*==\s*0/;

function solvePendulum(c: number, requestId: number): KernelResult {
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
    request_id: requestId,
    status: "ok",
    input_form: `x''[t] + ${cLabel} x'[t] + Sin[x[t]] == 0`,
    displays: [
      { kind: "latex", latex: `x''(t) + ${cLabel}\\,x'(t) + \\sin(x(t)) = 0` },
      {
        kind: "plot",
        curves: [{ points: samples.map((s) => [s.t, s.x] as [number, number]), label: "x(t)" }],
        x_range: [PENDULUM_T0, PENDULUM_T1],
        y_range: [yMin - pad, yMax + pad],
      },
    ],
    messages: [],
  };
}

function errorResult(kind: "parse" | "eval" | "solve", message: string, requestId: number): KernelResult {
  return { request_id: requestId, status: "error", displays: [], messages: [], error: { kind, message } };
}

export async function evaluate(input: string, bindings: Bindings, requestId: number): Promise<KernelResult> {
  const trimmed = input.trim();
  if (trimmed === "") {
    return errorResult("parse", "Nothing to evaluate.", requestId);
  }

  const numericMatch = PENDULUM_NUMERIC_PATTERN.exec(trimmed);
  if (numericMatch) {
    const raw = numericMatch[1].replace(/\s+/g, "");
    const c = raw === "" || raw === "+" ? 1 : raw === "-" ? -1 : parseFloat(raw);
    return solvePendulum(c, requestId);
  }

  const symbolicMatch = PENDULUM_SYMBOLIC_PATTERN.exec(trimmed);
  if (symbolicMatch) {
    const name = symbolicMatch[1];
    const bound = bindings[name];
    if (bound === undefined) {
      return errorResult(
        "solve",
        `'${name}' is not bound to a number; substitute a numeric value for '${name}' before calling NDSolve`,
        requestId,
      );
    }
    return solvePendulum(bound, requestId);
  }

  try {
    const value = evaluateArithmetic(trimmed);
    const text = formatNumber(value);
    return { request_id: requestId, status: "ok", input_form: text, displays: [{ kind: "latex", latex: text }], messages: [] };
  } catch (err) {
    const message = err instanceof ExprEvalError ? err.message : "Could not evaluate this input.";
    return errorResult(
      "eval",
      `${message}. The demo engine currently understands plain arithmetic and the damped pendulum NDSolve form.`,
      requestId,
    );
  }
}
