// Shared types for the app-to-kernel contract (ARCHITECTURE.md, "Kernel
// API"). Two layers on purpose:
//
//   - KernelResult (and Display/Message/KernelError) mirror the kernel's
//     serde-serialized Rust structs field for field, including the "kind"
//     tag on Display. This is what both engines return.
//   - EvalResult is the UI-facing view model Cell/OutputView already render:
//     a flat latex/plot/error shape. kernelResultToView() bridges the two so
//     the UI layer never has to know about displays/messages/status.

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

export type KernelStatus = "ok" | "error";

export type Display =
  | { kind: "latex"; latex: string }
  | { kind: "plot"; curves: Curve[]; x_range: [number, number]; y_range: [number, number] };

export interface Message {
  severity: "warning" | "note";
  text: string;
}

export interface KernelError {
  kind: "parse" | "eval" | "solve";
  message: string;
  position?: number;
}

export interface KernelResult {
  request_id: number;
  status: KernelStatus;
  input_form?: string;
  displays: Display[];
  messages: Message[];
  error?: KernelError;
}

/** Typed Manipulate slider values, keyed by WL symbol name (e.g. "c").
 * Never substituted into source text; see ARCHITECTURE.md, "Manipulate:
 * typed bindings, not text substitution". */
export type Bindings = Record<string, number>;

export type EvaluateFn = (input: string, bindings: Bindings, requestId: number) => Promise<KernelResult>;

/**
 * Bridge the kernel's structured KernelResult down to the UI's view model:
 * pull the first latex/plot display out of `displays`, and turn a typed
 * KernelError into the plain error string OutputView already renders.
 */
export function kernelResultToView(result: KernelResult): EvalResult {
  if (result.status === "error") {
    return { latex: "", error: result.error?.message ?? "Evaluation failed." };
  }

  const latexDisplay = result.displays.find((d): d is Extract<Display, { kind: "latex" }> => d.kind === "latex");
  const plotDisplay = result.displays.find((d): d is Extract<Display, { kind: "plot" }> => d.kind === "plot");

  return {
    latex: latexDisplay?.latex ?? "",
    plot: plotDisplay ? { curves: plotDisplay.curves, x_range: plotDisplay.x_range, y_range: plotDisplay.y_range } : undefined,
  };
}
