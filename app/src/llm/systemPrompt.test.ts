import { describe, expect, it } from "vitest";
import { buildSystemPrompt } from "./systemPrompt";
import { GRAMMAR_SUMMARY } from "./grammar";

describe("buildSystemPrompt", () => {
  const prompt = buildSystemPrompt();

  it("embeds the grammar summary verbatim", () => {
    expect(prompt).toContain(GRAMMAR_SUMMARY);
  });

  it("names every supported function from the grammar summary", () => {
    const functions = [
      "Sin",
      "Cos",
      "Tan",
      "Exp",
      "Log",
      "Sqrt",
      "Abs",
      "D",
      "Integrate",
      "Solve",
      "Expand",
      "Table",
      "Range",
      "Map",
      "NDSolve",
      "Plot",
      "ListPlot",
    ];
    for (const fn of functions) {
      expect(prompt).toContain(fn);
    }
  });

  it("lists the supported operators", () => {
    for (const op of ["+", "-", "*", "/", "^", "==", "->", ":=", "="]) {
      expect(prompt).toContain(op);
    }
  });

  it("mentions patterns, lists, and derivative syntax", () => {
    expect(prompt).toContain("x_");
    expect(prompt).toContain("{a, b, c}");
    expect(prompt).toContain("x'[t]");
  });

  it("demands code only, with no prose or markdown fences", () => {
    expect(prompt).toMatch(/no prose/i);
    expect(prompt).toMatch(/no markdown/i);
  });

  it("is deterministic across calls", () => {
    expect(buildSystemPrompt()).toBe(prompt);
  });
});
