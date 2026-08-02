// Pure notebook-document logic, kept separate from Notebook.tsx so it is
// testable without rendering React (matching the rest of this package:
// buildInput.ts, id.ts). Notebook.tsx wires these into state and the
// window integration contract (notebook/types.ts, NotebookDoc).

import { nextCellId } from "./id";
import type { InputCell, NotebookCellData, NotebookDoc, TextCellData, TextCellKind } from "./types";

export const NOTEBOOK_SCHEMA_VERSION = 1;

export function createInputCell(source = "", sourceKind?: InputCell["sourceKind"]): InputCell {
  return { id: nextCellId(), kind: "input", latex: source, sourceKind, status: "idle", result: null };
}

export function createTextCell(kind: TextCellKind, text = ""): TextCellData {
  return { id: nextCellId(), kind, text };
}

/** A fresh, empty notebook: one blank Input cell, nothing evaluated yet. */
export function blankNotebookCells(): NotebookCellData[] {
  return [createInputCell()];
}

export function toNotebookDoc(cells: NotebookCellData[], evalCounter: number): NotebookDoc {
  return { schemaVersion: NOTEBOOK_SCHEMA_VERSION, cells, evalCounter };
}

/**
 * Normalizes a loaded NotebookDoc defensively: a doc with no cells (or a
 * missing/non-finite counter) still leaves the notebook usable rather than
 * blank and broken.
 */
export function cellsFromDoc(doc: NotebookDoc): { cells: NotebookCellData[]; evalCounter: number } {
  const cells = doc.cells && doc.cells.length > 0 ? doc.cells : blankNotebookCells();
  const evalCounter = Number.isFinite(doc.evalCounter) ? doc.evalCounter : 0;
  return { cells, evalCounter };
}

function isTextCell(cell: NotebookCellData): cell is TextCellData {
  return cell.kind !== "input";
}

/**
 * Sets the cell's style to one of Title/Section/Text (Alt+1/Alt+4/Alt+7).
 * Converting between text styles keeps the text; converting from an Input
 * cell carries its LaTeX source over as a starting point rather than
 * discarding it (imperfect, but round-trips cleanly back to Input via
 * withCellConvertedToInput, and never silently loses the user's work).
 */
export function withCellStyle(cells: NotebookCellData[], id: string, kind: TextCellKind): NotebookCellData[] {
  return cells.map((c) => {
    if (c.id !== id || c.kind === kind) return c;
    const text = isTextCell(c) ? c.text : c.latex;
    return { id: c.id, kind, text };
  });
}

/** Converts a Title/Section/Text cell to a fresh Input cell, seeded with its
 * text as a starting point for editing. */
export function withCellConvertedToInput(cells: NotebookCellData[], id: string): NotebookCellData[] {
  return cells.map((c) => {
    if (c.id !== id || c.kind === "input") return c;
    return { id: c.id, kind: "input", latex: c.text, status: "idle", result: null } satisfies InputCell;
  });
}

/** Inserts a fresh Input cell at `index` (clamped to the cell list bounds),
 * optionally seeded with `source` text. Returns the new cell list and the
 * new cell's id, so the caller can focus it. */
export function insertInputCellAt(
  cells: NotebookCellData[],
  index: number,
  source = "",
  sourceKind?: InputCell["sourceKind"],
): { cells: NotebookCellData[]; id: string } {
  const cell = createInputCell(source, sourceKind);
  const clamped = Math.max(0, Math.min(index, cells.length));
  return { cells: [...cells.slice(0, clamped), cell, ...cells.slice(clamped)], id: cell.id };
}
