// Single entry point for the app-to-kernel contract. Picks the real Tauri
// engine when running inside the desktop shell, and the TypeScript mock
// engine everywhere else (vitest, plain `vite dev` in a browser tab), so the
// UI runs standalone and wires up to the real kernel with no call-site
// changes elsewhere.

export type { EvalResult, Curve, PlotData, EvaluateFn, KernelResult, Display, Message, KernelError, KernelStatus, Bindings } from "./types";
export { kernelResultToView } from "./types";

import type { EvaluateFn } from "./types";
import { evaluate as tauriEvaluate } from "./tauriEngine";
import { evaluate as mockEvaluate } from "./mockEngine";

function runningUnderTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export const evaluate: EvaluateFn = runningUnderTauri() ? tauriEvaluate : mockEvaluate;
