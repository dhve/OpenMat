// Shared types for the app-to-kernel contract.
// Field names (x_range, y_range) are kept snake_case on purpose: they mirror
// the serde-serialized Rust struct in ARCHITECTURE.md so the mock engine and
// the real Tauri engine can be swapped without touching call sites.

export interface Curve {
  points: [number, number][];
  label?: string;
}

export interface PlotData {
  curves: Curve[];
  x_range: [number, number];
  y_range: [number, number];
}

export interface EvalResult {
  latex: string;
  plot?: PlotData;
  error?: string;
}

export type EvaluateFn = (input: string) => Promise<EvalResult>;
