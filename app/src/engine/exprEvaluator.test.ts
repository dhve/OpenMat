import { describe, expect, it } from "vitest";
import { evaluateArithmetic, formatNumber, ExprEvalError } from "./exprEvaluator";

describe("evaluateArithmetic", () => {
  it("adds and subtracts", () => {
    expect(evaluateArithmetic("2 + 3 - 1")).toBe(4);
  });

  it("respects multiplication and division precedence", () => {
    expect(evaluateArithmetic("2 + 3 * 4")).toBe(14);
  });

  it("handles parens", () => {
    expect(evaluateArithmetic("(2 + 3) * 4")).toBe(20);
  });

  it("handles exponentiation right-associatively", () => {
    expect(evaluateArithmetic("2^3^2")).toBe(512); // 2^(3^2) = 2^9
  });

  it("handles implicit multiplication by juxtaposition", () => {
    expect(evaluateArithmetic("2(3 + 4)")).toBe(14);
  });

  it("handles unary minus looser than power", () => {
    expect(evaluateArithmetic("-2^2")).toBe(-4);
  });

  it("evaluates known functions", () => {
    expect(evaluateArithmetic("Sqrt[16]")).toBe(4);
    expect(evaluateArithmetic("Sin[0]")).toBe(0);
  });

  it("evaluates known constants", () => {
    expect(evaluateArithmetic("Pi")).toBeCloseTo(Math.PI, 10);
  });

  it("throws on division by zero", () => {
    expect(() => evaluateArithmetic("1/0")).toThrow(ExprEvalError);
  });

  it("throws on an unknown symbol", () => {
    expect(() => evaluateArithmetic("q + 1")).toThrow(ExprEvalError);
  });

  it("throws on malformed input", () => {
    expect(() => evaluateArithmetic("2 +")).toThrow(ExprEvalError);
  });
});

describe("formatNumber", () => {
  it("prints integers without a decimal point", () => {
    expect(formatNumber(4)).toBe("4");
  });

  it("rounds away floating point noise", () => {
    expect(formatNumber(0.1 + 0.2)).toBe("0.3");
  });
});
