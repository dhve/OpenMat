import { translateLatexToWL } from "../mathlive/translator";
import { formatNumber } from "../engine/exprEvaluator";
import type { InputCell } from "./types";

/**
 * Substitute a bare symbol with a numeric value in WL linear syntax. The
 * translator always separates implicit-multiplication factors with a space
 * (see translator.ts), so a single-letter symbol is always bounded by a
 * word boundary; a plain \b regex is safe here.
 */
function substituteSymbol(wl: string, name: string, value: number): string {
  const pattern = new RegExp(`\\b${name}\\b`, "g");
  return wl.replace(pattern, formatNumber(value));
}

const PENDULUM_X0 = 2;
const PENDULUM_V0 = 0;
const PENDULUM_T1 = 20;

/**
 * Build the string passed to evaluate() for a notebook input cell: translate
 * the MathLive LaTeX to WL linear syntax, and if the cell carries a
 * Manipulate slider, substitute its current value and wrap the equation in
 * the flagship demo's NDSolve call (fixed initial conditions and t range,
 * per ARCHITECTURE.md's example).
 *
 * Throws TranslatorParseError if the LaTeX does not parse.
 */
export function buildInputForCell(cell: InputCell): string {
  const core = translateLatexToWL(cell.latex);
  if (!cell.manipulate) return core;

  const substituted = substituteSymbol(core, cell.manipulate.name, cell.manipulate.value);
  return `NDSolve[{${substituted}, x[0] == ${PENDULUM_X0}, x'[0] == ${PENDULUM_V0}}, x, {t, 0, ${PENDULUM_T1}}]`;
}
