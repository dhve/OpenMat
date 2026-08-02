// Browser engine: the real openmat-kernel compiled to wasm32 (see
// crates/openmat-wasm), so `vite dev` in a plain browser tab runs the exact
// same engine as the Tauri desktop shell. Loaded lazily on the first
// evaluate() call; if instantiation fails (no fetch in node/vitest, missing
// asset), callers fall back to the mock engine via engine/index.ts.
//
// Memory contract (crates/openmat-wasm/src/lib.rs): strings cross as
// om_alloc'd (pointer, length) pairs, which om_evaluate consumes; the result
// comes back as one [4-byte LE length][KernelResult JSON] buffer that we
// free with om_free after reading.

import type { Bindings, KernelResult } from "./types";

interface KernelExports {
  memory: WebAssembly.Memory;
  om_alloc(len: number): number;
  om_free(ptr: number, len: number): void;
  om_evaluate(inputPtr: number, inputLen: number, bindingsPtr: number, bindingsLen: number, requestId: bigint): number;
}

let exportsPromise: Promise<KernelExports> | null = null;

function loadKernel(): Promise<KernelExports> {
  if (!exportsPromise) {
    exportsPromise = WebAssembly.instantiateStreaming(fetch("/openmat_kernel.wasm"), {}).then(
      ({ instance }) => instance.exports as unknown as KernelExports,
    );
  }
  return exportsPromise;
}

/** Whether the wasm kernel can even be attempted in this environment: a
 * real browser with fetch and WebAssembly, and not vitest (whose DOM shim
 * has both, but whose node fetch cannot resolve the relative asset URL). */
export function wasmSupported(): boolean {
  return typeof fetch === "function" && typeof WebAssembly !== "undefined" && typeof document !== "undefined" && !import.meta.env?.TEST;
}

function writeString(kernel: KernelExports, text: string): [number, number] {
  const bytes = new TextEncoder().encode(text);
  const ptr = kernel.om_alloc(bytes.length);
  new Uint8Array(kernel.memory.buffer, ptr, bytes.length).set(bytes);
  return [ptr, bytes.length];
}

export async function evaluate(input: string, bindings: Bindings, requestId: number): Promise<KernelResult> {
  const kernel = await loadKernel();

  const [inputPtr, inputLen] = writeString(kernel, input);
  const [bindingsPtr, bindingsLen] = writeString(kernel, JSON.stringify(bindings));

  const resultPtr = kernel.om_evaluate(inputPtr, inputLen, bindingsPtr, bindingsLen, BigInt(requestId));

  const lengthView = new DataView(kernel.memory.buffer, resultPtr, 4);
  const jsonLen = lengthView.getUint32(0, true);
  const jsonBytes = new Uint8Array(kernel.memory.buffer, resultPtr + 4, jsonLen);
  const json = new TextDecoder().decode(jsonBytes);
  kernel.om_free(resultPtr, 4 + jsonLen);

  return JSON.parse(json) as KernelResult;
}
