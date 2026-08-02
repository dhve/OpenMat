import { describe, expect, it } from "vitest";
import { InvalidNotebookFileError, parseNotebookFile, serializeNotebook, UnsupportedSchemaVersionError } from "./format";
import { CURRENT_SCHEMA_VERSION } from "./types";

// A representative opaque notebook snapshot, shaped like what
// window.__openmat_get_notebook() is expected to return: cells, slider
// (manipulate) values, an evaluation counter, and a title. This module
// never inspects the shape, it only wraps/unwraps it, so the exact fields
// here are illustrative rather than load-bearing.
const sampleNotebook = {
  title: "Damped Pendulum",
  evaluationCounter: 4,
  cells: [
    { id: "cell-1", kind: "title", text: "Damped Pendulum" },
    {
      id: "cell-2",
      kind: "input",
      latex: "x''(t)+c\\,x'(t)+\\sin(x(t))=0",
      status: "done",
      result: { latex: "x(t)", plot: { curves: [], x_range: [0, 20], y_range: [-1, 1] } },
      manipulate: { name: "c", label: "c", min: 0, max: 2, step: 0.05, value: 0.3 },
    },
  ],
};

describe("serializeNotebook / parseNotebookFile round trip", () => {
  it("parses back an equivalent notebook snapshot", () => {
    const text = serializeNotebook(sampleNotebook);
    expect(parseNotebookFile(text)).toEqual(sampleNotebook);
  });

  it("stamps the current schema_version in the envelope", () => {
    const text = serializeNotebook(sampleNotebook);
    expect(JSON.parse(text).schema_version).toBe(CURRENT_SCHEMA_VERSION);
  });

  it("is byte-stable across a save-load-save round trip", () => {
    const saved = serializeNotebook(sampleNotebook);
    const reloaded = parseNotebookFile(saved);
    const resaved = serializeNotebook(reloaded);
    expect(resaved).toBe(saved);
  });

  it("stays byte-stable across repeated round trips", () => {
    const first = serializeNotebook(parseNotebookFile(serializeNotebook(sampleNotebook)));
    const second = serializeNotebook(parseNotebookFile(first));
    expect(second).toBe(first);
  });

  it("round trips an empty notebook", () => {
    const empty = { title: "Untitled", evaluationCounter: 0, cells: [] };
    const text = serializeNotebook(empty);
    expect(parseNotebookFile(text)).toEqual(empty);
    expect(serializeNotebook(parseNotebookFile(text))).toBe(text);
  });
});

describe("schema_version rejection", () => {
  it("rejects a schema_version this build does not recognize", () => {
    const text = JSON.stringify({ schema_version: 2, notebook: sampleNotebook });
    expect(() => parseNotebookFile(text)).toThrow(UnsupportedSchemaVersionError);
  });

  it("rejects a missing schema_version", () => {
    const text = JSON.stringify({ notebook: sampleNotebook });
    expect(() => parseNotebookFile(text)).toThrow(InvalidNotebookFileError);
  });

  it("rejects malformed JSON", () => {
    expect(() => parseNotebookFile("{not valid json")).toThrow(InvalidNotebookFileError);
  });

  it("rejects a top-level JSON array", () => {
    expect(() => parseNotebookFile("[1,2,3]")).toThrow(InvalidNotebookFileError);
  });

  it("rejects a top-level JSON null", () => {
    expect(() => parseNotebookFile("null")).toThrow(InvalidNotebookFileError);
  });
});
