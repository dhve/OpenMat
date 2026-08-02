import type { NotebookCellData } from "../notebook/types";

// Kept in this exact form because it is covered by
// notebook/buildInput.test.ts ("translates the full damped pendulum
// equation"), so we know it round-trips through the translator correctly.
const PENDULUM_LATEX = "x''\\left(t\\right)+c\\,x'\\left(t\\right)+\\sin\\left(x\\left(t\\right)\\right)=0";

/**
 * The flagship demo notebook (ARCHITECTURE.md): a slider-driven damped
 * pendulum. Title cell, one 2D-input equation cell, and a Manipulate slider
 * for the damping coefficient c. The pendulum cell evaluates once on load
 * (see Notebook.tsx) so the plot is already showing.
 */
export function createDefaultNotebook(): NotebookCellData[] {
  return [
    { id: "demo-title", kind: "title", text: "Damped Pendulum" },
    {
      id: "demo-pendulum",
      kind: "input",
      latex: PENDULUM_LATEX,
      status: "idle",
      result: null,
      manipulate: { name: "c", label: "c", min: 0, max: 2, step: 0.05, value: 0.3 },
    },
  ];
}
