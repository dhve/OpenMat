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

  it("keeps consecutive letters as one symbol, as Mathematica does", () => {
    expect(translateLatexToWL("xy")).toBe("xy");
  });

  it("translates implicit multiplication across a spacing boundary", () => {
    expect(translateLatexToWL("c\\,x")).toBe("c x");
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

  it("translates = with an assignable left side as Set", () => {
    expect(translateLatexToWL("x=5")).toBe("x = 5");
  });

  it("translates = with a non-assignable left side as Equal", () => {
    expect(translateLatexToWL("x^2+y^2=4")).toBe("x^2 + y^2 == 4");
  });

  it("translates == as Equal regardless of the left side", () => {
    expect(translateLatexToWL("x==5")).toBe("x == 5");
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

  it("translates a bracketed call with a typed-brace list, MathLive style", () => {
    expect(translateLatexToWL("Plot\\lbrack\\sin\\left(x\\right),\\lbrace x,0,10\\rbrace\\rbrack")).toBe(
      "Plot[Sin[x], {x, 0, 10}]",
    );
  });

  it("translates a bracketed call typed with literal characters", () => {
    expect(translateLatexToWL("Plot[\\sin(x),\\{x,0,10\\}]")).toBe("Plot[Sin[x], {x, 0, 10}]");
  });

  it("translates a multi-letter head applied with brackets", () => {
    expect(translateLatexToWL("Expand[(x+1)^2]")).toBe("Expand[(x + 1)^2]");
  });

  it("translates a known head applied with parens", () => {
    expect(translateLatexToWL("Plot(\\sin(x),\\lbrace x,0,10\\rbrace)")).toBe("Plot[Sin[x], {x, 0, 10}]");
  });

  it("normalizes a lowercase known head", () => {
    expect(translateLatexToWL("sin[x]")).toBe("Sin[x]");
  });

  it("repairs MathLive's mid-word \\in shortcut firing", () => {
    expect(translateLatexToWL("S\\in[x]")).toBe("Sin[x]");
  });

  it("treats an unknown multi-letter symbol before parens as multiplication", () => {
    expect(translateLatexToWL("ab(x+1)")).toBe("ab (x + 1)");
  });

  it("translates a nested NDSolve call", () => {
    const latex = "NDSolve[\\lbrace x''(t)+cx'(t)+\\sin(x(t))==0,x(0)==2,x'(0)==0\\rbrace,x,\\lbrace t,0,20\\rbrace]";
    expect(translateLatexToWL(latex)).toBe(
      "NDSolve[{x''[t] + cx'[t] + Sin[x[t]] == 0, x[0] == 2, x'[0] == 0}, x, {t, 0, 20}]",
    );
  });

  it("translates a rule arrow", () => {
    expect(translateLatexToWL("x\\to2")).toBe("x -> 2");
  });

  it("translates a bounded integral with \\differentialD", () => {
    expect(translateLatexToWL("\\int_0^1x^2\\differentialD x")).toBe("Integrate[x^2, {x, 0, 1}]");
  });

  it("translates an indefinite integral typed with plain dx", () => {
    expect(translateLatexToWL("\\int x^2dx")).toBe("Integrate[x^2, x]");
  });

  it("translates a sum with bounds", () => {
    expect(translateLatexToWL("\\sum_{n=1}^{10}n^2")).toBe("Sum[n^2, {n, 1, 10}]");
  });

  it("translates an empty list", () => {
    expect(translateLatexToWL("\\lbrace\\rbrace")).toBe("{}");
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
