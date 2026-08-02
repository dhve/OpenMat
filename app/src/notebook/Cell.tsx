import type { Ref } from "react";
import { MathField } from "../mathlive/MathField";
import { CodeInputField } from "./CodeInputField";
import { TextCellView } from "./TextCellView";
import { Slider } from "../manipulate/Slider";
import { OutputView } from "../output/OutputView";
import type { NotebookCellData } from "./types";
import "./Cell.css";

/** The shape both MathField and TextCellView expose via ref: enough for
 * Notebook.tsx's focus-navigation to treat every cell kind alike. */
export interface CellFieldHandle {
  focus: (position?: "start" | "end") => void;
}

interface CellProps {
  cell: NotebookCellData;
  selected: boolean;
  fieldRef?: Ref<CellFieldHandle>;
  autoFocus?: boolean;
  onSelect?: () => void;
  onLatexChange?: (latex: string) => void;
  onTextChange?: (text: string) => void;
  /** Shift+Enter on an Input cell: evaluate, then move on. */
  onEvaluate?: () => void;
  /** Enter on an Input/Title/Section cell, or Shift+Enter on a Text cell:
   * no evaluation, just move to (or create) the next cell. */
  onCommit?: () => void;
  onNavigateUp?: () => void;
  onNavigateDown?: () => void;
  onManipulateChange?: (value: number) => void;
}

export function Cell({
  cell,
  selected,
  fieldRef,
  autoFocus,
  onSelect,
  onLatexChange,
  onTextChange,
  onEvaluate,
  onCommit,
  onNavigateUp,
  onNavigateDown,
  onManipulateChange,
}: CellProps) {
  if (cell.kind !== "input") {
    return (
      <div className={`cell cell-${cell.kind}${selected ? " cell-selected" : ""}`} onClick={onSelect}>
        <div className="cell-row cell-row-text">
          <TextCellView
            ref={fieldRef}
            cell={cell}
            autoFocus={autoFocus}
            onChange={(text) => onTextChange?.(text)}
            onFocus={onSelect}
            onCommit={onCommit}
            onNavigateUp={onNavigateUp}
            onNavigateDown={onNavigateDown}
          />
        </div>
        <span className="cell-bracket cell-bracket-outer" aria-hidden="true" />
      </div>
    );
  }

  const inLabel = cell.evalNumber != null ? `In[${cell.evalNumber}]:=` : "";
  const outLabel = cell.status === "done" && cell.evalNumber != null ? `Out[${cell.evalNumber}]=` : "";
  const showOutputRow = cell.status !== "idle";

  return (
    <div className={`cell cell-input${selected ? " cell-selected" : ""}`} onClick={onSelect}>
      <div className="cell-row cell-row-input">
        <span className="cell-label cell-label-in" aria-hidden="true">
          {inLabel}
        </span>
        {cell.sourceKind === "linear" ? (
          <CodeInputField
            ref={fieldRef}
            value={cell.latex}
            onChange={(latex) => onLatexChange?.(latex)}
            onEvaluate={onEvaluate}
            onCommit={onCommit}
            onFocus={onSelect}
            onNavigateUp={onNavigateUp}
            onNavigateDown={onNavigateDown}
            autoFocus={autoFocus}
            placeholder="OpenMat code…"
          />
        ) : (
          <MathField
            ref={fieldRef}
            value={cell.latex}
            onChange={(latex) => onLatexChange?.(latex)}
            onEvaluate={onEvaluate}
            onEnter={onCommit}
            onFocus={onSelect}
            onNavigateUp={onNavigateUp}
            onNavigateDown={onNavigateDown}
            autoFocus={autoFocus}
            placeholder="Type an expression…"
          />
        )}
        <span className="cell-bracket cell-bracket-row" aria-hidden="true" />
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

      {showOutputRow && (
        <div className="cell-row cell-row-output">
          <span className="cell-label cell-label-out" aria-hidden="true">
            {outLabel}
          </span>
          <OutputView result={cell.result} status={cell.status} />
          <span className="cell-bracket cell-bracket-row" aria-hidden="true" />
        </div>
      )}

      <span className="cell-bracket cell-bracket-outer" aria-hidden="true" />
    </div>
  );
}
