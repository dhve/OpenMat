import { describe, expect, it } from "vitest";
import { parseGeneratedNotebook } from "./notebookSpec";

describe("parseGeneratedNotebook", () => {
  it("parses a full notebook of mixed cells", () => {
    const raw = JSON.stringify({
      cells: [
        { kind: "title", text: "Damped Oscillator" },
        { kind: "text", text: "The equation of motion." },
        { kind: "input", code: "NDSolve[{x''[t] + c x'[t] + x[t] == 0, x[0] == 1, x'[0] == 0}, x, {t, 0, 20}]", manipulate: { name: "c", min: 0, max: 2, step: 0.1, value: 0.5 } },
        { kind: "input", code: "Plot[Sin[x], {x, 0, 10}]" },
      ],
    });
    const cells = parseGeneratedNotebook(raw);
    expect(cells).toHaveLength(4);
    expect(cells[0]).toEqual({ kind: "title", text: "Damped Oscillator" });
    expect(cells[2].kind).toBe("input");
    if (cells[2].kind === "input") {
      expect(cells[2].manipulate).toMatchObject({ name: "c", label: "c", min: 0, max: 2, step: 0.1, value: 0.5 });
    }
  });

  it("tolerates prose around the JSON object", () => {
    const raw = 'Here you go:\n{"cells": [{"kind": "input", "code": "1 + 1"}]}\nEnjoy!';
    expect(parseGeneratedNotebook(raw)).toEqual([{ kind: "input", code: "1 + 1", manipulate: undefined }]);
  });

  it("takes the first valid object when a small model babbles extra JSON after it", () => {
    const raw = '{"cells": [{"kind": "input", "code": "Plot[Sin[x], {x, 0, 10}]"}]}\n\n{"cells": [{"kind": "output", "bogus": true}]}\n{"cells": []}';
    expect(parseGeneratedNotebook(raw)).toEqual([{ kind: "input", code: "Plot[Sin[x], {x, 0, 10}]", manipulate: undefined }]);
  });

  it("skips a leading invalid object in favor of a later valid one", () => {
    const raw = '{"note": "hi"} {"cells": [{"kind": "input", "code": "1 + 1"}]}';
    expect(parseGeneratedNotebook(raw)).toEqual([{ kind: "input", code: "1 + 1", manipulate: undefined }]);
  });

  it("falls back to a single expression when the reply is not JSON", () => {
    expect(parseGeneratedNotebook("Integrate[x^2, x]")).toEqual([{ kind: "input", code: "Integrate[x^2, x]" }]);
  });

  it("falls back when the JSON has no usable cells", () => {
    expect(parseGeneratedNotebook('{"cells": [{"kind": "input"}]}')).toEqual([{ kind: "input", code: '{"cells": [{"kind": "input"}]}' }]);
  });

  it("drops an invalid manipulate but keeps the cell", () => {
    const raw = JSON.stringify({ cells: [{ kind: "input", code: "Plot[a x, {x, 0, 1}]", manipulate: { name: "a", min: 5, max: 1 } }] });
    const cells = parseGeneratedNotebook(raw);
    expect(cells).toHaveLength(1);
    if (cells[0].kind === "input") expect(cells[0].manipulate).toBeUndefined();
  });

  it("defaults slider step, value, and label sensibly", () => {
    const raw = JSON.stringify({ cells: [{ kind: "input", code: "a x", manipulate: { name: "a", min: 0, max: 10 } }] });
    const cells = parseGeneratedNotebook(raw);
    if (cells[0].kind === "input") {
      expect(cells[0].manipulate).toMatchObject({ label: "a", step: 0.1, value: 5 });
    }
  });

  it("returns no cells for an empty reply", () => {
    expect(parseGeneratedNotebook("  ")).toEqual([]);
  });

  it("unwraps a double-encoded cells array", () => {
    const raw = JSON.stringify({ cells: JSON.stringify([{ kind: "input", code: "1 + 1" }]) });
    expect(parseGeneratedNotebook(raw)).toEqual([{ kind: "input", code: "1 + 1", manipulate: undefined }]);
  });

  it("salvages valid cells from a structurally broken outer object", () => {
    // Shaped like a real small-model failure: doubled closing brace after
    // the first cell breaks the outer object, later entries invent kinds.
    const raw = `{"cells": [
      {"kind": "title", "text": "Damped Harmonic Oscillator"}},
      {"kind": "text", "text": "How damping affects the motion."},
      {"kind": "input", "code": ""},
      {"kind": "table", "cells": []},
      {"kind": "input", "code": "Plot[Exp[-0.2 t] Cos[t], {t, 0, 20}]"}
    ]`;
    const cells = parseGeneratedNotebook(raw);
    expect(cells).toEqual([
      { kind: "title", text: "Damped Harmonic Oscillator" },
      { kind: "text", text: "How damping affects the motion." },
      { kind: "input", code: "Plot[Exp[-0.2 t] Cos[t], {t, 0, 20}]", manipulate: undefined },
    ]);
  });
});
