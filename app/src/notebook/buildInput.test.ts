import { describe, expect, it } from "vitest";
import { bindingsForCell, buildInputForCell } from "./buildInput";
import type { InputCell } from "./types";

function makeCell(latex: string, manipulateValue?: number): InputCell {
  return {
    id: "cell-1",
    kind: "input",
    latex,
    status: "idle",
    result: null,
    manipulate:
      manipulateValue === undefined
        ? undefined
        : { name: "c", label: "c", min: 0, max: 2, step: 0.05, value: manipulateValue },
  };
}

describe("buildInputForCell", () => {
  it("translates a plain equation cell with no manipulate wrapper", () => {
    const cell = makeCell("2+3");
    expect(buildInputForCell(cell)).toBe("2 + 3");
    expect(bindingsForCell(cell)).toEqual({});
  });

  it("keeps the damping coefficient symbolic and wraps in NDSolve for the pendulum cell", () => {
    const latex = "x''\\left(t\\right)+c\\,x'\\left(t\\right)+\\sin\\left(x\\left(t\\right)\\right)=0";
    const cell = makeCell(latex, 0.3);
    expect(buildInputForCell(cell)).toBe(
      "NDSolve[{x''[t] + c x'[t] + Sin[x[t]] == 0, x[0] == 2, x'[0] == 0}, x, {t, 0, 20}]",
    );
    expect(bindingsForCell(cell)).toEqual({ c: 0.3 });
  });

  it("input text stays identical as the slider value changes; only the binding changes", () => {
    const latex = "x''(t)+c\\,x'(t)+\\sin(x(t))=0";
    const low = makeCell(latex, 0);
    const high = makeCell(latex, 2);
    expect(buildInputForCell(low)).toBe(buildInputForCell(high));
    expect(bindingsForCell(low)).toEqual({ c: 0 });
    expect(bindingsForCell(high)).toEqual({ c: 2 });
  });

  it("a linear-source cell (Ask AI insertions) is used as-is, skipping LaTeX translation", () => {
    // Square brackets and multi-letter identifiers like "NDSolve" and
    // "Sin" are not part of the LaTeX subset translateLatexToWL supports,
    // so this would throw a TranslatorParseError if it went through the
    // normal path (see notebook/types.ts, InputCell.sourceKind).
    const cell: InputCell = {
      id: "cell-ai",
      kind: "input",
      latex: "NDSolve[{x''[t] + 0.3 x'[t] + Sin[x[t]] == 0, x[0] == 2, x'[0] == 0}, x, {t, 0, 20}]",
      sourceKind: "linear",
      status: "idle",
      result: null,
    };
    expect(buildInputForCell(cell)).toBe(cell.latex);
  });

  it("trims a linear-source cell's surrounding whitespace", () => {
    const cell: InputCell = {
      id: "cell-ai-2",
      kind: "input",
      latex: "  Sin[x]  ",
      sourceKind: "linear",
      status: "idle",
      result: null,
    };
    expect(buildInputForCell(cell)).toBe("Sin[x]");
  });
});
