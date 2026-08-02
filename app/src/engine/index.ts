// Single entry point for the app-to-kernel contract. Picks the engine by
// environment, so call sites never change:
//
//   - Tauri desktop shell: the in-process Rust kernel via invoke().
//   - Plain browser (vite dev, a static web deploy): the SAME Rust kernel
//     compiled to wasm32 (crates/openmat-wasm), fetched lazily; one engine
//     everywhere, no JS reimplementation.
//   - vitest/node (no fetch/document): the TypeScript mock engine, kept only
//     so UI unit tests run without a wasm runtime.
//
// If the wasm kernel fails to load in a browser (missing asset), the error
// surfaces in the cell rather than silently downgrading to the mock: the
// mock speaks a tiny demo subset and pretending it is the kernel is exactly
// the confusion this file exists to prevent.

export type { EvalResult, Curve, PlotData, EvaluateFn, KernelResult, Display, Message, KernelError, KernelStatus, Bindings } from "./types";
export { kernelResultToView } from "./types";

import type { EvaluateFn } from "./types";
import { evaluate as tauriEvaluate } from "./tauriEngine";
import { evaluate as mockEvaluate } from "./mockEngine";
import { evaluate as wasmEvaluate, wasmSupported } from "./wasmEngine";

function runningUnderTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export const evaluate: EvaluateFn = runningUnderTauri() ? tauriEvaluate : wasmSupported() ? wasmEvaluate : mockEvaluate;
