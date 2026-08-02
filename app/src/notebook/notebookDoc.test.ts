import { describe, expect, it } from "vitest";
import {
  blankNotebookCells,
  cellsFromDoc,
  createInputCell,
  createTextCell,
  insertInputCellAt,
  NOTEBOOK_SCHEMA_VERSION,
  toNotebookDoc,
  withCellConvertedToInput,
  withCellStyle,
} from "./notebookDoc";
import type { InputCell, NotebookCellData, TextCellData } from "./types";

describe("createInputCell / createTextCell", () => {
  it("creates a blank, idle input cell by default", () => {
    const cell = createInputCell();
    expect(cell.kind).toBe("input");
    expect(cell.latex).toBe("");
    expect(cell.sourceKind).toBeUndefined();
    expect(cell.status).toBe("idle");
    expect(cell.result).toBeNull();
    expect(cell.id).toBeTruthy();
  });

  it("seeds source and sourceKind when given", () => {
    const cell = createInputCell("Sin[x]", "linear");
    expect(cell.latex).toBe("Sin[x]");
    expect(cell.sourceKind).toBe("linear");
  });

  it("creates a text cell of the requested kind", () => {
    const cell = createTextCell("section", "Overview");
    expect(cell).toMatchObject({ kind: "section", text: "Overview" });
  });

  it("gives every cell a distinct id", () => {
    const a = createInputCell();
    const b = createInputCell();
    expect(a.id).not.toBe(b.id);
  });
});

describe("toNotebookDoc / cellsFromDoc round trip", () => {
  it("round-trips cells and the eval counter", () => {
    const cells: NotebookCellData[] = [createTextCell("title", "Demo"), createInputCell("2 + 2")];
    const doc = toNotebookDoc(cells, 5);
    expect(doc.schemaVersion).toBe(NOTEBOOK_SCHEMA_VERSION);
    expect(doc.evalCounter).toBe(5);

    const restored = cellsFromDoc(doc);
    expect(restored.cells).toEqual(cells);
    expect(restored.evalCounter).toBe(5);
  });

  it("falls back to a blank notebook for an empty cell list", () => {
    const restored = cellsFromDoc(toNotebookDoc([], 3));
    expect(restored.cells).toHaveLength(1);
    expect(restored.cells[0].kind).toBe("input");
  });

  it("falls back to counter 0 for a missing or non-finite evalCounter", () => {
    const cells = [createInputCell()];
    expect(cellsFromDoc({ schemaVersion: 1, cells, evalCounter: NaN }).evalCounter).toBe(0);
    expect(cellsFromDoc({ schemaVersion: 1, cells, evalCounter: undefined as unknown as number }).evalCounter).toBe(0);
  });
});

describe("blankNotebookCells", () => {
  it("is a single empty input cell", () => {
    const cells = blankNotebookCells();
    expect(cells).toHaveLength(1);
    expect(cells[0]).toMatchObject({ kind: "input", latex: "" });
  });
});

describe("withCellStyle", () => {
  it("converts an input cell to a text cell, carrying its latex over as starting text", () => {
    const input = createInputCell("x^2");
    const next = withCellStyle([input], input.id, "title") as TextCellData[];
    expect(next[0]).toMatchObject({ id: input.id, kind: "title", text: "x^2" });
  });

  it("converts between text styles, keeping the text", () => {
    const text = createTextCell("text", "Some notes");
    const next = withCellStyle([text], text.id, "section") as TextCellData[];
    expect(next[0]).toMatchObject({ kind: "section", text: "Some notes" });
  });

  it("is a no-op for a cell already in the requested style", () => {
    const text = createTextCell("section", "Overview");
    const next = withCellStyle([text], text.id, "section");
    expect(next[0]).toBe(text);
  });

  it("leaves other cells untouched", () => {
    const a = createInputCell("1");
    const b = createInputCell("2");
    const next = withCellStyle([a, b], a.id, "text");
    expect(next[1]).toBe(b);
  });
});

describe("withCellConvertedToInput", () => {
  it("converts a text cell to a fresh input cell, seeding latex from its text", () => {
    const text = createTextCell("text", "NDSolve[...]");
    const next = withCellConvertedToInput([text], text.id) as InputCell[];
    expect(next[0]).toMatchObject({ id: text.id, kind: "input", latex: "NDSolve[...]", status: "idle", result: null });
  });

  it("is a no-op for a cell that is already input", () => {
    const input = createInputCell("2+2");
    const next = withCellConvertedToInput([input], input.id);
    expect(next[0]).toBe(input);
  });
});

describe("insertInputCellAt", () => {
  it("inserts at a given index without disturbing the rest of the list", () => {
    const a = createInputCell("a");
    const b = createInputCell("b");
    const { cells, id } = insertInputCellAt([a, b], 1, "new");
    expect(cells.map((c) => c.id)).toEqual([a.id, id, b.id]);
    expect(cells[1]).toMatchObject({ latex: "new" });
  });

  it("clamps an out-of-range index to the list bounds", () => {
    const a = createInputCell("a");
    const { cells, id } = insertInputCellAt([a], 99);
    expect(cells.map((c) => c.id)).toEqual([a.id, id]);

    const { cells: cells2, id: id2 } = insertInputCellAt([a], -5);
    expect(cells2.map((c) => c.id)).toEqual([id2, a.id]);
  });

  it("tags the new cell's sourceKind when given", () => {
    const { cells } = insertInputCellAt([], 0, "Sin[x]", "linear");
    expect(cells[0]).toMatchObject({ latex: "Sin[x]", sourceKind: "linear" });
  });
});
