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

export interface TitleCell {
  id: string;
  kind: "title";
  text: string;
}

export interface InputCell {
  id: string;
  kind: "input";
  latex: string;
  status: CellStatus;
  result: EvalResult | null;
  manipulate?: ManipulateConfig;
}

export type NotebookCellData = TitleCell | InputCell;
