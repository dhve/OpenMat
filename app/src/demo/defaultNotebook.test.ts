import { describe, expect, it } from "vitest";
import { createDefaultNotebook } from "./defaultNotebook";
import { bindingsForCell, buildInputForCell } from "../notebook/buildInput";
import { evaluate, kernelResultToView } from "../engine";
import type { InputCell } from "../notebook/types";

describe("default demo notebook", () => {
  it("has a title cell and one pendulum input cell with a Manipulate slider for c", () => {
    const cells = createDefaultNotebook();
    expect(cells[0]).toMatchObject({ kind: "title", text: "Damped Pendulum" });
    const pendulum = cells[1] as InputCell;
    expect(pendulum.kind).toBe("input");
    expect(pendulum.manipulate).toMatchObject({ name: "c", min: 0, max: 2 });
  });

  it("goes end to end: MathLive latex -> WL input (c symbolic) -> typed binding -> mock engine -> plot", async () => {
    const cells = createDefaultNotebook();
    const pendulum = cells[1] as InputCell;

    const input = buildInputForCell(pendulum);
    expect(input).toBe("NDSolve[{x''[t] + c x'[t] + Sin[x[t]] == 0, x[0] == 2, x'[0] == 0}, x, {t, 0, 20}]");
    const bindings = bindingsForCell(pendulum);
    expect(bindings).toEqual({ c: 0.3 });

    const kernelResult = await evaluate(input, bindings, 1);
    expect(kernelResult.status).toBe("ok");
    const result = kernelResultToView(kernelResult);
    expect(result.error).toBeUndefined();
    expect(result.plot).toBeDefined();
    expect(result.plot!.curves[0].points.length).toBeGreaterThan(100);
    expect(result.plot!.curves[0].points[0]).toEqual([0, 2]);
  });
});
