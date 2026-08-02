// Latest-result-wins wrapper around a raw EvaluateFn (ARCHITECTURE.md,
// "Manipulate: typed bindings, not text substitution"; specs/m0-milestone.md
// rows 3 and 7). The transport, not the caller, assigns monotonically
// increasing request IDs and drops any response whose request_id is not the
// newest one issued, so an out-of-order resolution during a fast slider
// drag can never flicker the UI back to a stale frame.

import type { Bindings, EvaluateFn, KernelResult } from "./types";

export type RequestClient = (input: string, bindings: Bindings) => Promise<KernelResult | null>;

/**
 * Wrap `rawEvaluate` so callers do not manage request IDs themselves. The
 * returned function resolves to `null` instead of a `KernelResult` when a
 * newer call (issued after this one) has already been made by the time this
 * one's response lands; the caller should treat `null` as "ignore, do
 * nothing" rather than as an error.
 */
export function createRequestClient(rawEvaluate: EvaluateFn): RequestClient {
  let nextId = 1;
  let latestIssued = 0;

  return async function evaluate(input, bindings) {
    const requestId = nextId++;
    latestIssued = requestId;
    const result = await rawEvaluate(input, bindings, requestId);
    if (result.request_id < latestIssued) {
      return null;
    }
    return result;
  };
}
