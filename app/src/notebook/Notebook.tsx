import { Fragment, useEffect, useRef, useState } from "react";
import { Cell, type CellFieldHandle } from "./Cell";
import { InsertBar } from "./InsertBar";
import { bindingsForCell, buildInputForCell } from "./buildInput";
import { evaluate, kernelResultToView } from "../engine";
import { createRequestClient, type RequestClient } from "../engine/requestClient";
import { TranslatorParseError } from "../mathlive/translator";
import {
  blankNotebookCells,
  cellsFromDoc,
  insertInputCellAt,
  toNotebookDoc,
  withCellConvertedToInput,
  withCellStyle,
} from "./notebookDoc";
import type { InputCell, NotebookCellData, TextCellKind } from "./types";
import "./Notebook.css";

interface NotebookProps {
  initialCells: NotebookCellData[];
}

function isInputCell(cell: NotebookCellData): cell is InputCell {
  return cell.kind === "input";
}

const STYLE_SHORTCUTS: Record<string, TextCellKind> = { "1": "title", "4": "section", "7": "text" };

export function Notebook({ initialCells }: NotebookProps) {
  const [cells, setCells] = useState<NotebookCellData[]>(initialCells);
  const [evalCounter, setEvalCounter] = useState(0);
  const [selectedId, setSelectedId] = useState<string | null>(() => initialCells.find(isInputCell)?.id ?? initialCells[0]?.id ?? null);

  const fieldHandles = useRef(new Map<string, CellFieldHandle>());
  const pendingFocusId = useRef<string | null>(null);
  const pendingFocusPosition = useRef<"start" | "end" | undefined>(undefined);
  const [initialFocusId] = useState(() => initialCells.find(isInputCell)?.id ?? initialCells[0]?.id ?? null);

  // Kept in sync with state on every render so the window-level integration
  // contract (get/set notebook) and the global Alt+1/4/7/9 style-shortcut
  // listener, both registered once on mount, never read stale state.
  const cellsRef = useRef(cells);
  const evalCounterRef = useRef(evalCounter);
  const selectedIdRef = useRef(selectedId);
  useEffect(() => {
    cellsRef.current = cells;
    evalCounterRef.current = evalCounter;
    selectedIdRef.current = selectedId;
  });

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
      fieldHandles.current.get(pendingFocusId.current)?.focus(pendingFocusPosition.current);
      pendingFocusId.current = null;
      pendingFocusPosition.current = undefined;
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

  // The window integration contract other agents' UI (Save/Open/Settings,
  // Ask AI cell insertion, persistence) calls through optional chaining.
  // Registered once; reads/writes go through the refs above so this always
  // sees the latest notebook regardless of when it was mounted.
  useEffect(() => {
    window.__openmat_get_notebook = () => toNotebookDoc(cellsRef.current, evalCounterRef.current);

    window.__openmat_set_notebook = (doc) => {
      const { cells: nextCells, evalCounter: nextCounter } = cellsFromDoc(doc);
      requestClients.current.clear();
      cellsRef.current = nextCells;
      evalCounterRef.current = nextCounter;
      setCells(nextCells);
      setEvalCounter(nextCounter);
      setSelectedId(nextCells[0]?.id ?? null);
    };

    window.__openmat_insert_cell = (source: string) => {
      // Ask AI (the only current caller) hands us OpenMat linear syntax,
      // not MathLive LaTeX; see InputCell.sourceKind in notebook/types.ts.
      const { cells: next, id } = insertInputCellAt(cellsRef.current, cellsRef.current.length, source, "linear");
      cellsRef.current = next;
      setCells(next);
      setSelectedId(id);
      pendingFocusId.current = id;
    };

    return () => {
      delete window.__openmat_get_notebook;
      delete window.__openmat_set_notebook;
      delete window.__openmat_insert_cell;
    };
  }, []);

  // Mathematica-style cell style shortcuts: Alt+1 Title, Alt+4 Section,
  // Alt+7 Text, Alt+9 back to Input. Global (not per-field) so it works no
  // matter which cell currently has focus.
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (!e.altKey || e.metaKey || e.ctrlKey) return;
      const id = selectedIdRef.current;
      if (!id) return;
      const styleKind = STYLE_SHORTCUTS[e.key];
      if (styleKind) {
        e.preventDefault();
        setCells((prev) => withCellStyle(prev, id, styleKind));
        pendingFocusId.current = id;
        return;
      }
      if (e.key === "9") {
        e.preventDefault();
        setCells((prev) => withCellConvertedToInput(prev, id));
        pendingFocusId.current = id;
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const updateCell = (id: string, patch: Partial<InputCell>) => {
    setCells((prev) => prev.map((c) => (c.id === id && c.kind === "input" ? { ...c, ...patch } : c)));
  };

  const updateTextCell = (id: string, text: string) => {
    setCells((prev) => prev.map((c) => (c.id === id && c.kind !== "input" ? { ...c, text } : c)));
  };

  const nextEvalNumber = () => {
    const n = evalCounterRef.current + 1;
    evalCounterRef.current = n;
    setEvalCounter(n);
    return n;
  };

  const runEvaluate = async (id: string, overrideCell?: InputCell, options: { assignNumber?: boolean } = {}) => {
    const assignNumber = options.assignNumber ?? true;
    const cell = overrideCell ?? (cells.find((c) => c.id === id) as InputCell | undefined);
    if (!cell || cell.kind !== "input") return;

    // A Manipulate drag re-solves under the *same* In/Out number: only an
    // explicit evaluation mints a new one, matching Mathematica (dragging a
    // bound control never creates new In/Out cells).
    const evalNumber = assignNumber ? nextEvalNumber() : cell.evalNumber;
    updateCell(id, { status: "evaluating", ...(assignNumber ? { evalNumber } : {}) });

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

  const insertCellAt = (index: number, source = "") => {
    const { cells: next, id } = insertInputCellAt(cells, index, source);
    setCells(next);
    setSelectedId(id);
    pendingFocusId.current = id;
  };

  const focusNeighbor = (id: string, direction: 1 | -1, position: "start" | "end") => {
    const index = cells.findIndex((c) => c.id === id);
    const target = cells[index + direction];
    if (!target) return false;
    fieldHandles.current.get(target.id)?.focus(position);
    setSelectedId(target.id);
    return true;
  };

  const focusNext = (id: string) => focusNeighbor(id, 1, "start");
  const focusPrev = (id: string) => focusNeighbor(id, -1, "end");

  // Enter at the end of the last cell creates a new cell below and focuses
  // it, like a fresh line in a document. In the middle of the notebook,
  // Enter just moves focus to the next cell. Shared by every cell kind: for
  // Title/Section/Text there is nothing to evaluate, so "commit" and "move
  // on" is their whole story.
  const handleEnter = (id: string) => {
    const index = cells.findIndex((c) => c.id === id);
    const isLast = index === cells.length - 1;
    if (isLast) {
      insertCellAt(cells.length);
    } else if (!focusNext(id)) {
      insertCellAt(cells.length);
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
      void runEvaluate(id, updated, { assignNumber: false });
    }
  };

  const setSelectedStyle = (kind: TextCellKind) => {
    if (!selectedId) return;
    setCells((prev) => withCellStyle(prev, selectedId, kind));
    pendingFocusId.current = selectedId;
  };

  const convertSelectedToInput = () => {
    if (!selectedId) return;
    setCells((prev) => withCellConvertedToInput(prev, selectedId));
    pendingFocusId.current = selectedId;
  };

  const clearNotebook = () => {
    const fresh = blankNotebookCells();
    requestClients.current.clear();
    setCells(fresh);
    setEvalCounter(0);
    evalCounterRef.current = 0;
    setSelectedId(fresh[0]?.id ?? null);
    pendingFocusId.current = fresh[0]?.id ?? null;
  };

  return (
    <div className="notebook">
      <div className="notebook-toolbar">
        <div className="notebook-toolbar-group" role="group" aria-label="Cell style">
          <button type="button" title="Title (Alt+1)" onClick={() => setSelectedStyle("title")}>
            Title
          </button>
          <button type="button" title="Section (Alt+4)" onClick={() => setSelectedStyle("section")}>
            Section
          </button>
          <button type="button" title="Text (Alt+7)" onClick={() => setSelectedStyle("text")}>
            Text
          </button>
          <button type="button" title="Input (Alt+9)" onClick={convertSelectedToInput}>
            Input
          </button>
        </div>
        <button type="button" className="notebook-toolbar-clear" onClick={clearNotebook}>
          Clear Notebook
        </button>
      </div>

      <InsertBar onInsert={() => insertCellAt(0)} />
      {cells.map((cell, index) => (
        <Fragment key={cell.id}>
          <Cell
            cell={cell}
            selected={cell.id === selectedId}
            autoFocus={cell.id === initialFocusId}
            fieldRef={(handle) => {
              if (handle) fieldHandles.current.set(cell.id, handle);
              else fieldHandles.current.delete(cell.id);
            }}
            onSelect={() => setSelectedId(cell.id)}
            onLatexChange={isInputCell(cell) ? (latex) => updateCell(cell.id, { latex }) : undefined}
            onTextChange={!isInputCell(cell) ? (text) => updateTextCell(cell.id, text) : undefined}
            onEvaluate={isInputCell(cell) ? () => handleShiftEnter(cell.id) : undefined}
            onCommit={() => handleEnter(cell.id)}
            onNavigateUp={() => focusPrev(cell.id)}
            onNavigateDown={() => focusNext(cell.id)}
            onManipulateChange={isInputCell(cell) ? (v) => handleManipulateChange(cell.id, v) : undefined}
          />
          <InsertBar onInsert={() => insertCellAt(index + 1)} alwaysVisible={index === cells.length - 1} />
        </Fragment>
      ))}
    </div>
  );
}
