// Single entry point for the app-to-kernel contract. The whole app runs on
// mockEngine today; switching to the real kernel is this one line.

export type { EvalResult, Curve, PlotData, EvaluateFn } from "./types";
export { evaluate } from "./mockEngine";
// Swap the line above for the one below once openmat-kernel is wired up:
// export { evaluate } from "./tauriEngine";
