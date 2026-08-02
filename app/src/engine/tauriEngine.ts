// Real engine: calls the openmat-kernel through Tauri's invoke bridge. Kept
// behind the same evaluate() signature as mockEngine so swapping engines in
// index.ts is a one-line change with no call-site impact elsewhere.

import { invoke } from "@tauri-apps/api/core";
import type { EvalResult } from "./types";

export async function evaluate(input: string): Promise<EvalResult> {
  return invoke<EvalResult>("evaluate", { input });
}
