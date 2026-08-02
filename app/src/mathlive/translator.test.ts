import { describe, expect, it } from "vitest";
import { translateLatexToWL, TranslatorParseError } from "./translator";

describe("translateLatexToWL", () => {
  it("translates addition", () => {
    expect(translateLatexToWL("2+3")).toBe("2 + 3");
  });

  it("translates a left-associative chain of + and -", () => {
    expect(translateLatexToWL("x+y-z")).toBe("x + y - z");
  });

  it("translates explicit multiplication", () => {
    expect(translateLatexToWL("12.5*3")).toBe("12.5 * 3");
  });

  it("translates \\cdot as explicit multiplication", () => {
    expect(translateLatexToWL("a\\cdot b")).toBe("a * b");
  });

  it("translates implicit multiplication of a number and a symbol", () => {
    expect(translateLatexToWL("2x")).toBe("2 x");
  });

  it("translates implicit multiplication of adjacent symbols", () => {
    expect(translateLatexToWL("xy")).toBe("x y");
  });

  it("translates division", () => {
    expect(translateLatexToWL("a/b")).toBe("a/b");
  });

  it("translates \\frac with atomic parts", () => {
    expect(translateLatexToWL("\\frac{1}{2}")).toBe("1/2");
  });

  it("translates \\frac with a compound numerator, adding parens", () => {
    expect(translateLatexToWL("\\frac{a+b}{c}")).toBe("(a + b)/c");
  });

  it("translates \\frac with a negative numerator", () => {
    expect(translateLatexToWL("\\frac{-a}{b}")).toBe("-a/b");
  });

  it("translates exponents", () => {
    expect(translateLatexToWL("x^2+1")).toBe("x^2 + 1");
  });

  it("keeps unary minus looser than power: -x^2 is -(x^2)", () => {
    expect(translateLatexToWL("-x^2")).toBe("-x^2");
  });

  it("parenthesizes a negative base raised to a power", () => {
    expect(translateLatexToWL("(-x)^2")).toBe("(-x)^2");
  });

  it("re-adds parens that structure requires even without source parens hints", () => {
    expect(translateLatexToWL("a-(b-c)")).toBe("a - (b - c)");
  });

  it("parenthesizes both sides of an implicit multiplication of sums", () => {
    expect(translateLatexToWL("(x+1)(x-1)")).toBe("(x + 1) (x - 1)");
  });

  it("converts known function application to square brackets", () => {
    expect(translateLatexToWL("\\sin(x)")).toBe("Sin[x]");
  });

  it("capitalizes function names and combines with implicit multiplication", () => {
    expect(translateLatexToWL("2\\sin(x)")).toBe("2 Sin[x]");
  });

  it("puts a function call in a fraction", () => {
    expect(translateLatexToWL("\\frac{\\sin(x)}{2}")).toBe("Sin[x]/2");
  });

  it("translates a single prime as a first derivative call", () => {
    expect(translateLatexToWL("x'(t)")).toBe("x'[t]");
  });

  it("translates a double prime as a second derivative call", () => {
    expect(translateLatexToWL("x''(t)")).toBe("x''[t]");
  });

  it("translates equality", () => {
    expect(translateLatexToWL("x=5")).toBe("x == 5");
  });

  it("translates the full damped pendulum equation", () => {
    const latex = "x''\\left(t\\right)+c\\,x'\\left(t\\right)+\\sin\\left(x\\left(t\\right)\\right)=0";
    expect(translateLatexToWL(latex)).toBe("x''[t] + c x'[t] + Sin[x[t]] == 0");
  });

  it("translates \\sqrt", () => {
    expect(translateLatexToWL("\\sqrt{4}")).toBe("Sqrt[4]");
  });

  it("translates \\pi as the WL constant Pi", () => {
    expect(translateLatexToWL("2\\pi")).toBe("2 Pi");
  });

  it("returns an empty string for empty input", () => {
    expect(translateLatexToWL("")).toBe("");
    expect(translateLatexToWL("   ")).toBe("");
  });

  it("throws a TranslatorParseError on a dangling operator", () => {
    expect(() => translateLatexToWL("2+")).toThrow(TranslatorParseError);
  });

  it("throws a TranslatorParseError on mismatched parens", () => {
    expect(() => translateLatexToWL("(2+3")).toThrow(TranslatorParseError);
  });

  it("throws a TranslatorParseError on an unsupported command", () => {
    expect(() => translateLatexToWL("\\vec{x}")).toThrow(TranslatorParseError);
  });
});
