import { describe, expect, it } from "vitest";
import { createRequestClient } from "./requestClient";
import type { EvaluateFn, KernelResult } from "./types";

function makeResult(requestId: number, latex: string): KernelResult {
  return { request_id: requestId, status: "ok", input_form: latex, displays: [{ kind: "latex", latex }], messages: [] };
}

describe("createRequestClient", () => {
  it("drops a stale response that resolves after a newer request was issued", async () => {
    // Request 1 is issued first but is slow; request 2 is issued second and
    // resolves first. Request 1's response must be discarded once it does
    // land, since a newer request was already in flight when it arrives.
    const resolvers: Array<() => void> = [];
    const raw: EvaluateFn = (_input, _bindings, requestId) =>
      new Promise((resolve) => {
        resolvers.push(() => resolve(makeResult(requestId, `result-${requestId}`)));
      });

    const client = createRequestClient(raw);
    const first = client("input-1", {});
    const second = client("input-2", {});

    // Resolve out of order: the second (newer) request resolves before the first.
    resolvers[1]();
    const secondResult = await second;
    resolvers[0]();
    const firstResult = await first;

    expect(secondResult).not.toBeNull();
    expect(secondResult?.request_id).toBe(2);
    expect(firstResult).toBeNull();
  });

  it("passes through a normal, non-interleaved response", async () => {
    const raw: EvaluateFn = async (_input, _bindings, requestId) => makeResult(requestId, "ok");
    const client = createRequestClient(raw);
    const result = await client("input", {});
    expect(result).not.toBeNull();
    expect(result?.request_id).toBe(1);
  });

  it("assigns a fresh, increasing request id on every call", async () => {
    const seen: number[] = [];
    const raw: EvaluateFn = async (_input, _bindings, requestId) => {
      seen.push(requestId);
      return makeResult(requestId, "ok");
    };
    const client = createRequestClient(raw);
    await client("a", {});
    await client("b", {});
    await client("c", {});
    expect(seen).toEqual([1, 2, 3]);
  });
});
