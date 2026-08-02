import { translateLatexToWL } from "../mathlive/translator";
import type { InputCell } from "./types";
import type { Bindings } from "../engine/types";

const PENDULUM_X0 = 2;
const PENDULUM_V0 = 0;
const PENDULUM_T1 = 20;

/**
 * Build the string passed to evaluate() for a notebook input cell: translate
 * the MathLive LaTeX to WL linear syntax, and if the cell carries a
 * Manipulate slider, wrap the equation in the flagship demo's NDSolve call
 * (fixed initial conditions and t range, per ARCHITECTURE.md's example).
 *
 * The slider's symbol (e.g. "c") is left symbolic in the returned text; its
 * current value travels separately as a typed binding (see
 * bindingsForCell), never substituted into the string. This is what lets
 * the kernel parse the cell once per edit and skip parsing on every slider
 * tick (ARCHITECTURE.md, "Manipulate: typed bindings, not text
 * substitution"; specs/m0-milestone.md row 3).
 *
 * Throws TranslatorParseError if the LaTeX does not parse.
 *
 * A cell with sourceKind "linear" (inserted via window.__openmat_insert_cell
 * by the Ask AI feature) already holds WL linear syntax, not LaTeX, and
 * skips translation entirely; see InputCell.sourceKind in notebook/types.ts.
 */
export function buildInputForCell(cell: InputCell): string {
  const core = cell.sourceKind === "linear" ? cell.latex.trim() : translateLatexToWL(cell.latex);
  if (!cell.manipulate) return core;
  // Linear-syntax cells with a slider (generated notebooks) already carry
  // the complete expression: the slider's symbol is simply bound at
  // evaluation time. The NDSolve wrapping below is only for the flagship
  // demo's 2D equation cell, whose source is just the equation itself.
  if (cell.sourceKind === "linear") return core;
  return `NDSolve[{${core}, x[0] == ${PENDULUM_X0}, x'[0] == ${PENDULUM_V0}}, x, {t, 0, ${PENDULUM_T1}}]`;
}

/**
 * The typed bindings to send alongside buildInputForCell's result: the
 * Manipulate slider's current value keyed by its symbol, or empty for a
 * cell with no slider.
 */
export function bindingsForCell(cell: InputCell): Bindings {
  return cell.manipulate ? { [cell.manipulate.name]: cell.manipulate.value } : {};
}
