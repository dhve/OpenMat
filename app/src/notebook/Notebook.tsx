import { useEffect, useRef, useState } from "react";
import { Cell } from "./Cell";
import { nextCellId } from "./id";
import { bindingsForCell, buildInputForCell } from "./buildInput";
import { evaluate, kernelResultToView } from "../engine";
import { createRequestClient, type RequestClient } from "../engine/requestClient";
import { TranslatorParseError } from "../mathlive/translator";
import type { MathFieldHandle } from "../mathlive/MathField";
import type { InputCell, NotebookCellData } from "./types";
import "./Notebook.css";

interface NotebookProps {
  initialCells: NotebookCellData[];
}

function isInputCell(cell: NotebookCellData): cell is InputCell {
  return cell.kind === "input";
}

export function Notebook({ initialCells }: NotebookProps) {
  const [cells, setCells] = useState<NotebookCellData[]>(initialCells);
  const fieldHandles = useRef(new Map<string, MathFieldHandle>());
  const pendingFocusId = useRef<string | null>(null);
  const [initialFocusId] = useState(() => initialCells.find(isInputCell)?.id ?? null);

  // One request client per cell, so latest-result-wins tracking (see
  // engine/requestClient.ts) is scoped per cell rather than globally: a slow
  // re-evaluate in one cell must never be able to mark a fast response in an
  // unrelated cell as stale. Dependency tracking is cell-level in M0
  // (ARCHITECTURE.md, "Manipulate: typed bindings, not text substitution").
  const requestClients = useRef(new Map<string, RequestClient>());
  const clientForCell = (id: string): RequestClient => {
    let client = requestClients.current.get(id);
    if (!client) {
      client = createRequestClient(evaluate);
      requestClients.current.set(id, client);
    }
    return client;
  };

  useEffect(() => {
    if (pendingFocusId.current) {
      fieldHandles.current.get(pendingFocusId.current)?.focus();
      pendingFocusId.current = null;
    }
  });

  // Cells that ship with a Manipulate slider (the flagship demo) evaluate
  // once on load, so the plot is already live rather than waiting for the
  // user's first Shift+Enter.
  useEffect(() => {
    initialCells.forEach((c) => {
      if (c.kind === "input" && c.manipulate) void runEvaluate(c.id, c);
    });
    // Mount only.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const updateCell = (id: string, patch: Partial<InputCell>) => {
    setCells((prev) => prev.map((c) => (c.id === id && c.kind === "input" ? { ...c, ...patch } : c)));
  };

  const runEvaluate = async (id: string, overrideCell?: InputCell) => {
    const cell = overrideCell ?? (cells.find((c) => c.id === id) as InputCell | undefined);
    if (!cell || cell.kind !== "input") return;

    updateCell(id, { status: "evaluating" });

    let input: string;
    try {
      input = buildInputForCell(cell);
    } catch (err) {
      const message = err instanceof TranslatorParseError ? err.message : "Could not read this expression.";
      updateCell(id, { status: "error", result: { latex: "", error: message } });
      return;
    }

    if (input === "") {
      updateCell(id, { status: "idle", result: null });
      return;
    }

    const bindings = bindingsForCell(cell);
    const kernelResult = await clientForCell(id)(input, bindings);
    if (kernelResult === null) return; // a newer request for this cell has already landed

    const result = kernelResultToView(kernelResult);
    updateCell(id, { status: result.error ? "error" : "done", result });
  };

  const addCellAfter = (afterId: string | null) => {
    const newCell: InputCell = {
      id: nextCellId(),
      kind: "input",
      latex: "",
      status: "idle",
      result: null,
    };
    setCells((prev) => {
      if (afterId === null) return [...prev, newCell];
      const index = prev.findIndex((c) => c.id === afterId);
      if (index === -1) return [...prev, newCell];
      const next = [...prev];
      next.splice(index + 1, 0, newCell);
      return next;
    });
    pendingFocusId.current = newCell.id;
  };

  const focusNext = (id: string) => {
    const index = cells.findIndex((c) => c.id === id);
    for (let i = index + 1; i < cells.length; i++) {
      if (isInputCell(cells[i])) {
        fieldHandles.current.get(cells[i].id)?.focus();
        return true;
      }
    }
    return false;
  };

  // Enter at the end of the last cell creates a new cell below and focuses
  // it, like a fresh line in a document. In the middle of the notebook,
  // Enter just moves focus to the next cell.
  const handleEnter = (id: string) => {
    const index = cells.findIndex((c) => c.id === id);
    const isLast = index === cells.length - 1;
    if (isLast) {
      addCellAfter(id);
    } else if (!focusNext(id)) {
      addCellAfter(id);
    }
  };

  // Shift+Enter evaluates the cell and moves focus to the next cell, if one
  // exists (it does not create a new one; that is Enter's job).
  const handleShiftEnter = (id: string) => {
    void runEvaluate(id);
    focusNext(id);
  };

  const handleManipulateChange = (id: string, value: number) => {
    // Compute the patched cells up front and evaluate with that exact
    // object, instead of a functional setCells updater: updater functions
    // can run more than once (e.g. under StrictMode), and evaluate() is a
    // side effect that must run exactly once per change.
    const nextCells = cells.map((c) =>
      c.id === id && c.kind === "input" && c.manipulate ? { ...c, manipulate: { ...c.manipulate, value } } : c,
    );
    setCells(nextCells);
    const updated = nextCells.find((c) => c.id === id);
    if (updated && updated.kind === "input") {
      void runEvaluate(id, updated);
    }
  };

  return (
    <div className="notebook">
      {cells.map((cell) => (
        <Cell
          key={cell.id}
          cell={cell}
          autoFocus={cell.id === initialFocusId}
          fieldRef={
            isInputCell(cell)
              ? (handle) => {
                  if (handle) fieldHandles.current.set(cell.id, handle);
                  else fieldHandles.current.delete(cell.id);
                }
              : undefined
          }
          onLatexChange={isInputCell(cell) ? (latex) => updateCell(cell.id, { latex }) : undefined}
          onEvaluate={isInputCell(cell) ? () => handleShiftEnter(cell.id) : undefined}
          onEnter={isInputCell(cell) ? () => handleEnter(cell.id) : undefined}
          onManipulateChange={isInputCell(cell) ? (v) => handleManipulateChange(cell.id, v) : undefined}
        />
      ))}

      <button type="button" className="notebook-add-cell" onClick={() => addCellAfter(cells[cells.length - 1]?.id ?? null)}>
        + New Cell
      </button>
    </div>
  );
}
