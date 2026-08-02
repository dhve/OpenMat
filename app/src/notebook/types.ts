import type { EvalResult } from "../engine/types";

export type CellStatus = "idle" | "evaluating" | "done" | "error";

export interface ManipulateConfig {
  /** The WL symbol this slider drives, e.g. "c". Must be a single letter. */
  name: string;
  label: string;
  min: number;
  max: number;
  step: number;
  value: number;
}

/** The three Mathematica-style text cell styles, set with Alt+1 / Alt+4 /
 * Alt+7 (see notebook/notebookDoc.ts, withCellStyle). */
export type TextCellKind = "title" | "section" | "text";

/** A plain editable text cell: Title, Section, or Text. All three share the
 * same shape and only differ in how Cell.tsx styles them. */
export interface TextCellData {
  id: string;
  kind: TextCellKind;
  text: string;
}

export interface InputCell {
  id: string;
  kind: "input";
  latex: string;
  /**
   * How `latex` should be read on evaluation (notebook/buildInput.ts):
   *   - "latex" (default, undefined counts as this): MathLive LaTeX,
   *     translated to WL linear syntax before evaluating.
   *   - "linear": already OpenMat's WL-shaped linear syntax (e.g.
   *     `Sin[x]`, `NDSolve[...]`), used as-is. This is what
   *     `window.__openmat_insert_cell` produces: the LLM workstream's Ask
   *     AI feature (specs/m0-milestone.md M6) generates linear syntax, not
   *     LaTeX, and running it through the LaTeX translator would fail on
   *     every multi-letter function name and square bracket.
   * MathLive still renders `latex` either way; "linear" text just renders
   * as its literal characters rather than typeset math, which is exactly
   * what an editable, review-before-you-run code preview needs.
   */
  sourceKind?: "latex" | "linear";
  status: CellStatus;
  result: EvalResult | null;
  manipulate?: ManipulateConfig;
  /**
   * The shared In[n]/Out[n] number for this cell's last labeled evaluation
   * (the global, notebook-wide counter described in notebookDoc.ts).
   * Undefined until the cell has been evaluated at least once. A Manipulate
   * slider re-evaluate does not change this number: only an explicit
   * evaluation (Shift+Enter, or the flagship demo's evaluate-on-load) does,
   * matching Mathematica where dragging a bound control does not mint new
   * In/Out cells.
   */
  evalNumber?: number;
}

export type NotebookCellData = TextCellData | InputCell;

/**
 * Serializable snapshot of the whole notebook. This is the exact shape
 * returned by `window.__openmat_get_notebook()` and accepted by
 * `window.__openmat_set_notebook()` (see notebook/notebookDoc.ts for the
 * helpers that build and consume it):
 *
 *   - `cells`: every cell's type (`kind`), its editable source
 *     (`text` for Title/Section/Text, `latex` for Input), and its last
 *     outputs (`result`) and Manipulate slider value (`manipulate.value`)
 *     for Input cells.
 *   - `evalCounter`: the global evaluation counter used for In/Out
 *     numbering, so a reload continues numbering where it left off.
 *
 * `schemaVersion` lets a future format change be detected by consumers
 * (persistence, etc.) without guessing.
 */
export interface NotebookDoc {
  schemaVersion: number;
  cells: NotebookCellData[];
  evalCounter: number;
}

/**
 * The window-level integration contract this module owns. Other parts of
 * the app (Save/Open/Settings/Ask AI, persistence, LLM cell insertion) call
 * these through optional chaining, e.g. `window.__openmat_get_notebook?.()`,
 * since they may not be registered yet (or ever, outside the full app).
 */
declare global {
  interface Window {
    __openmat_get_notebook?: () => NotebookDoc;
    __openmat_set_notebook?: (doc: NotebookDoc) => void;
    __openmat_insert_cell?: (source: string) => void;
  }
}
