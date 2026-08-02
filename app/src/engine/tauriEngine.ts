// Real engine: calls the openmat-kernel through Tauri's invoke bridge. Kept
// behind the same evaluate() signature as mockEngine so swapping engines in
// index.ts is a one-line change with no call-site impact elsewhere.
//
// Tauri camelCases Rust command parameter names by default, so the Rust
// side's `request_id: u64` is reached as `requestId` here; see
// src-tauri/src/lib.rs.

import { invoke } from "@tauri-apps/api/core";
import type { Bindings, KernelResult } from "./types";

export async function evaluate(input: string, bindings: Bindings, requestId: number): Promise<KernelResult> {
  return invoke<KernelResult>("evaluate", { input, bindings, requestId });
}
