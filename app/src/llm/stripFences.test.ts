import { describe, expect, it } from "vitest";
import { stripCodeFences } from "./stripFences";

describe("stripCodeFences", () => {
  it("passes plain code through unchanged", () => {
    expect(stripCodeFences("D[x^2, x]")).toBe("D[x^2, x]");
  });

  it("trims surrounding whitespace even with no fence", () => {
    expect(stripCodeFences("  Sin[x]  \n")).toBe("Sin[x]");
  });

  it("strips a fence with no language tag", () => {
    expect(stripCodeFences("```\nIntegrate[x^2, x]\n```")).toBe("Integrate[x^2, x]");
  });

  it("strips a fence with a language tag", () => {
    expect(stripCodeFences("```wolfram\nSolve[x^2 - 5x + 6 == 0, x]\n```")).toBe("Solve[x^2 - 5x + 6 == 0, x]");
  });

  it("strips a fence on a single line", () => {
    expect(stripCodeFences("```Plot[Sin[x], {x, 0, 10}]```")).toBe("Plot[Sin[x], {x, 0, 10}]");
  });

  it("strips a single pair of backticks", () => {
    expect(stripCodeFences("`Sin[x]`")).toBe("Sin[x]");
  });

  it("strips a dangling opening fence with no closing fence", () => {
    expect(stripCodeFences("```wolfram\nExpand[(x+y)^2]")).toBe("Expand[(x+y)^2]");
  });

  it("strips a dangling closing fence with no opening fence", () => {
    expect(stripCodeFences("Table[i^2, {i, 1, 10}]\n```")).toBe("Table[i^2, {i, 1, 10}]");
  });

  it("preserves multiline content inside a fence", () => {
    const raw = "```\nNDSolve[{y'[t] == -y[t], y[0] == 1}, y, {t, 0, 10}]\n```";
    expect(stripCodeFences(raw)).toBe("NDSolve[{y'[t] == -y[t], y[0] == 1}, y, {t, 0, 10}]");
  });

  it("returns an empty string unchanged", () => {
    expect(stripCodeFences("")).toBe("");
    expect(stripCodeFences("   ")).toBe("");
  });

  it("does not strip backticks that are not a matched wrapping pair", () => {
    expect(stripCodeFences("x` + `y")).toBe("x` + `y");
  });
});
