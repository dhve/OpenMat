// Parses the model's reply to a free-form request into typed cell specs.
// The contract (see systemPrompt.ts) is a JSON object {"cells": [...]}, but
// model output is treated as untrusted text: anything that does not parse
// and validate falls back to "the whole reply is one expression", which is
// exactly the old single-expression behavior, so a model that ignores the
// JSON contract still produces a working cell rather than an error.

import type { ManipulateConfig } from "../notebook/types";

export interface GeneratedInputCell {
  kind: "input";
  code: string;
  manipulate?: ManipulateConfig;
}

export interface GeneratedTextCell {
  kind: "title" | "section" | "text";
  text: string;
}

export type GeneratedCellSpec = GeneratedInputCell | GeneratedTextCell;

/** Ceiling on generated cells, so a runaway reply cannot flood the
 * notebook. Generous: a rich topic notebook is well under this. */
const MAX_CELLS = 24;

function asManipulate(value: unknown): ManipulateConfig | undefined {
  if (typeof value !== "object" || value === null) return undefined;
  const m = value as Record<string, unknown>;
  const name = typeof m.name === "string" ? m.name.trim() : "";
  const min = typeof m.min === "number" && Number.isFinite(m.min) ? m.min : null;
  const max = typeof m.max === "number" && Number.isFinite(m.max) ? m.max : null;
  if (name === "" || min === null || max === null || !(min < max)) return undefined;
  const step = typeof m.step === "number" && Number.isFinite(m.step) && m.step > 0 ? m.step : (max - min) / 100;
  const rawValue = typeof m.value === "number" && Number.isFinite(m.value) ? m.value : (min + max) / 2;
  const value_ = Math.min(max, Math.max(min, rawValue));
  const label = typeof m.label === "string" && m.label.trim() !== "" ? m.label : name;
  return { name, label, min, max, step, value: value_ };
}

function asCell(value: unknown): GeneratedCellSpec | null {
  if (typeof value !== "object" || value === null) return null;
  const c = value as Record<string, unknown>;
  if (c.kind === "title" || c.kind === "section" || c.kind === "text") {
    if (typeof c.text !== "string" || c.text.trim() === "") return null;
    return { kind: c.kind, text: c.text.trim() };
  }
  if (c.kind === "input") {
    if (typeof c.code !== "string" || c.code.trim() === "") return null;
    return { kind: "input", code: c.code.trim(), manipulate: asManipulate(c.manipulate) };
  }
  return null;
}

/** Extracts one brace-balanced {...} span starting at `start`, tracking
 * JSON string literals so braces inside strings do not count. Returns the
 * end index (inclusive), or -1 if the span never balances. */
function balancedSpanEnd(raw: string, start: number): number {
  let depth = 0;
  let inString = false;
  let escaped = false;
  for (let j = start; j < raw.length; j++) {
    const ch = raw[j];
    if (inString) {
      if (escaped) escaped = false;
      else if (ch === "\\") escaped = true;
      else if (ch === '"') inString = false;
    } else if (ch === '"') {
      inString = true;
    } else if (ch === "{") {
      depth++;
    } else if (ch === "}") {
      depth--;
      if (depth === 0) return j;
    }
  }
  return -1;
}

/** Yields each brace-balanced {...} span in `raw`, in order. Small models
 * often emit one valid object followed by babbled extra ones; scanning
 * balanced spans lets the first VALID object win instead of gluing them
 * together. */
function* balancedObjects(raw: string): Generator<string> {
  let i = raw.indexOf("{");
  while (i !== -1) {
    const end = balancedSpanEnd(raw, i);
    if (end === -1) return;
    yield raw.slice(i, end + 1);
    i = raw.indexOf("{", end + 1);
  }
}

/** Salvage pass for structurally broken replies (an outer object that never
 * balances, invented cell kinds, syntax errors between cells): pull out
 * every individually balanced object that LOOKS like a cell ({"kind": ...})
 * and keep the ones that validate. A weak local model's mangled notebook
 * still yields its good cells this way. */
function salvageCells(raw: string): GeneratedCellSpec[] {
  const cells: GeneratedCellSpec[] = [];
  const kindMarker = /\{\s*"kind"/g;
  let match: RegExpExecArray | null;
  while ((match = kindMarker.exec(raw)) !== null && cells.length < MAX_CELLS) {
    const end = balancedSpanEnd(raw, match.index);
    if (end === -1) continue;
    try {
      const cell = asCell(JSON.parse(raw.slice(match.index, end + 1)));
      if (cell) cells.push(cell);
    } catch {
      // skip this span; later cells may still parse
    }
    kindMarker.lastIndex = end + 1;
  }
  return cells;
}

/** How many candidate {...} spans to try before giving up: enough to skip
 * leading junk objects, small enough to stay O(reply length). */
const MAX_CANDIDATE_OBJECTS = 8;

/**
 * Parse a model reply into cell specs. Never throws: an unparseable or
 * invalid reply becomes a single input cell holding the raw text, whose
 * kernel evaluation will surface any real problem to the user.
 */
export function parseGeneratedNotebook(raw: string): GeneratedCellSpec[] {
  const fallback: GeneratedCellSpec[] = [{ kind: "input", code: raw.trim() }];
  if (raw.trim() === "") return [];

  let attempts = 0;
  for (const candidate of balancedObjects(raw)) {
    if (++attempts > MAX_CANDIDATE_OBJECTS) break;

    let parsed: unknown;
    try {
      parsed = JSON.parse(candidate);
    } catch {
      continue;
    }
    if (typeof parsed !== "object" || parsed === null) continue;

    // Some models double-encode: {"cells": "[{\"kind\": ...}]"} with the
    // array serialized as a string. Unwrap it before validating.
    let cellsValue = (parsed as Record<string, unknown>).cells;
    if (typeof cellsValue === "string") {
      try {
        cellsValue = JSON.parse(cellsValue);
      } catch {
        continue;
      }
    }
    if (!Array.isArray(cellsValue)) continue;

    const cells = (cellsValue as unknown[])
      .slice(0, MAX_CELLS)
      .map(asCell)
      .filter((c): c is GeneratedCellSpec => c !== null);
    if (cells.length > 0) return cells;
  }

  const salvaged = salvageCells(raw);
  if (salvaged.length > 0) return salvaged;

  return fallback;
}
