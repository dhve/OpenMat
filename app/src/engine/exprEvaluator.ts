// A small evaluator for WL-shaped linear syntax arithmetic: numbers,
// + - * / ^, parens, implicit multiplication by juxtaposition (spaces), and
// a handful of known functions/constants applied to numeric arguments.
// This is the mock engine's stand-in for real symbolic evaluation; it only
// ever produces a number.

export class ExprEvalError extends Error {}

type TokenType = "num" | "ident" | "lparen" | "rparen" | "lbracket" | "rbracket" | "plus" | "minus" | "star" | "slash" | "caret" | "comma" | "eof";

interface Token {
  type: TokenType;
  value: string;
}

const CONSTANTS: Record<string, number> = {
  Pi: Math.PI,
  E: Math.E,
};

const FUNCTIONS: Record<string, (...args: number[]) => number> = {
  Sin: Math.sin,
  Cos: Math.cos,
  Tan: Math.tan,
  ArcSin: Math.asin,
  ArcCos: Math.acos,
  ArcTan: Math.atan,
  Sinh: Math.sinh,
  Cosh: Math.cosh,
  Tanh: Math.tanh,
  Sqrt: Math.sqrt,
  Exp: Math.exp,
  Log: (x, base) => (base === undefined ? Math.log(x) : Math.log(x) / Math.log(base)),
  Abs: Math.abs,
};

function tokenize(input: string): Token[] {
  const tokens: Token[] = [];
  let i = 0;
  const n = input.length;
  while (i < n) {
    const c = input[i];
    if (/\s/.test(c)) {
      // A run of whitespace between two value-starting tokens means
      // implicit multiplication; emit a synthetic space token only when
      // useful, handled by the parser via lookahead instead. Just skip here
      // but remember whitespace occurred by not merging tokens.
      i++;
      continue;
    }
    if (/[0-9]/.test(c)) {
      let start = i;
      while (i < n && /[0-9]/.test(input[i])) i++;
      if (input[i] === "." && /[0-9]/.test(input[i + 1] ?? "")) {
        i++;
        while (i < n && /[0-9]/.test(input[i])) i++;
      }
      tokens.push({ type: "num", value: input.slice(start, i) });
      continue;
    }
    if (/[A-Za-z]/.test(c)) {
      let start = i;
      while (i < n && /[A-Za-z0-9]/.test(input[i])) i++;
      tokens.push({ type: "ident", value: input.slice(start, i) });
      continue;
    }
    switch (c) {
      case "(":
        tokens.push({ type: "lparen", value: c });
        i++;
        continue;
      case ")":
        tokens.push({ type: "rparen", value: c });
        i++;
        continue;
      case "[":
        tokens.push({ type: "lbracket", value: c });
        i++;
        continue;
      case "]":
        tokens.push({ type: "rbracket", value: c });
        i++;
        continue;
      case "+":
        tokens.push({ type: "plus", value: c });
        i++;
        continue;
      case "-":
        tokens.push({ type: "minus", value: c });
        i++;
        continue;
      case "*":
        tokens.push({ type: "star", value: c });
        i++;
        continue;
      case "/":
        tokens.push({ type: "slash", value: c });
        i++;
        continue;
      case "^":
        tokens.push({ type: "caret", value: c });
        i++;
        continue;
      case ",":
        tokens.push({ type: "comma", value: c });
        i++;
        continue;
      default:
        throw new ExprEvalError(`Unexpected character "${c}"`);
    }
  }
  tokens.push({ type: "eof", value: "" });
  return tokens;
}

class Parser {
  private tokens: Token[];
  private pos = 0;
  constructor(tokens: Token[]) {
    this.tokens = tokens;
  }
  private peek(): Token {
    return this.tokens[this.pos];
  }
  private advance(): Token {
    const t = this.tokens[this.pos];
    if (this.pos < this.tokens.length - 1) this.pos++;
    return t;
  }
  private expect(type: TokenType): Token {
    const t = this.peek();
    if (t.type !== type) {
      throw new ExprEvalError(`Expected ${type} but found ${t.type === "eof" ? "end of input" : `"${t.value}"`}`);
    }
    return this.advance();
  }

  parseTop(): number {
    const v = this.parseAddSub();
    if (this.peek().type !== "eof") {
      throw new ExprEvalError(`Unexpected trailing input near "${this.peek().value}"`);
    }
    return v;
  }

  private parseAddSub(): number {
    let left = this.parseMulDiv();
    while (this.peek().type === "plus" || this.peek().type === "minus") {
      const isPlus = this.advance().type === "plus";
      const right = this.parseMulDiv();
      left = isPlus ? left + right : left - right;
    }
    return left;
  }

  private startsFactor(): boolean {
    const t = this.peek().type;
    return t === "num" || t === "ident" || t === "lparen";
  }

  private parseMulDiv(): number {
    let left = this.parseUnary();
    while (true) {
      if (this.peek().type === "star") {
        this.advance();
        left *= this.parseUnary();
      } else if (this.peek().type === "slash") {
        this.advance();
        const divisor = this.parseUnary();
        if (divisor === 0) throw new ExprEvalError("Division by zero");
        left /= divisor;
      } else if (this.startsFactor()) {
        left *= this.parseUnary();
      } else {
        break;
      }
    }
    return left;
  }

  private parseUnary(): number {
    if (this.peek().type === "minus") {
      this.advance();
      return -this.parseUnary();
    }
    if (this.peek().type === "plus") {
      this.advance();
      return this.parseUnary();
    }
    return this.parsePow();
  }

  private parsePow(): number {
    const base = this.parseAtom();
    if (this.peek().type === "caret") {
      this.advance();
      const exponent = this.parseUnary();
      return Math.pow(base, exponent);
    }
    return base;
  }

  private parseArgList(): number[] {
    const args = [this.parseAddSub()];
    while (this.peek().type === "comma") {
      this.advance();
      args.push(this.parseAddSub());
    }
    return args;
  }

  private parseAtom(): number {
    const t = this.peek();
    switch (t.type) {
      case "num":
        this.advance();
        return parseFloat(t.value);
      case "ident": {
        this.advance();
        if (this.peek().type === "lbracket") {
          this.advance();
          const args = this.parseArgList();
          this.expect("rbracket");
          const fn = FUNCTIONS[t.value];
          if (!fn) throw new ExprEvalError(`Unknown function ${t.value}`);
          return fn(...args);
        }
        const constant = CONSTANTS[t.value];
        if (constant === undefined) throw new ExprEvalError(`Unknown symbol ${t.value}`);
        return constant;
      }
      case "lparen": {
        this.advance();
        const v = this.parseAddSub();
        this.expect("rparen");
        return v;
      }
      default:
        throw new ExprEvalError(`Unexpected token "${t.value || t.type}"`);
    }
  }
}

/** Evaluate a WL-shaped linear syntax arithmetic expression to a number. */
export function evaluateArithmetic(input: string): number {
  const tokens = tokenize(input.trim());
  return new Parser(tokens).parseTop();
}

/** Format a number as a compact, non-noisy result string for display. */
export function formatNumber(value: number): string {
  if (!isFinite(value)) return value > 0 ? "Infinity" : "-Infinity";
  if (Number.isInteger(value)) return value.toString();
  const rounded = Math.round(value * 1e10) / 1e10;
  return rounded.toString();
}
