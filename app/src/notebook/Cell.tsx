import type { Ref } from "react";
import { MathField, type MathFieldHandle } from "../mathlive/MathField";
import { Slider } from "../manipulate/Slider";
import { OutputView } from "../output/OutputView";
import type { NotebookCellData } from "./types";
import "./Cell.css";

interface CellProps {
  cell: NotebookCellData;
  fieldRef?: Ref<MathFieldHandle>;
  autoFocus?: boolean;
  onLatexChange?: (latex: string) => void;
  onEvaluate?: () => void;
  onEnter?: () => void;
  onManipulateChange?: (value: number) => void;
}

export function Cell({ cell, fieldRef, autoFocus, onLatexChange, onEvaluate, onEnter, onManipulateChange }: CellProps) {
  if (cell.kind === "title") {
    return (
      <div className="cell cell-title">
        <h1>{cell.text}</h1>
      </div>
    );
  }

  return (
    <div className="cell cell-input">
      <div className="cell-input-row">
        <span className="cell-prompt" aria-hidden="true">
          In
        </span>
        <MathField
          ref={fieldRef}
          value={cell.latex}
          onChange={(latex) => onLatexChange?.(latex)}
          onEvaluate={onEvaluate}
          onEnter={onEnter}
          autoFocus={autoFocus}
          placeholder="Type an expression…"
        />
      </div>

      {cell.manipulate && (
        <Slider
          name={cell.manipulate.name}
          label={cell.manipulate.label}
          min={cell.manipulate.min}
          max={cell.manipulate.max}
          step={cell.manipulate.step}
          value={cell.manipulate.value}
          onChange={(v) => onManipulateChange?.(v)}
        />
      )}

      <OutputView result={cell.result} status={cell.status} />
    </div>
  );
}
