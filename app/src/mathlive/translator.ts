// Translates LaTeX produced by a MathLive math-field into OpenMat's
// WL-shaped linear syntax (see ARCHITECTURE.md, "App-to-kernel contract").
//
// Approach: tokenize the LaTeX subset we support, parse it into a small
// expression tree, then pretty-print that tree as linear syntax with the
// minimum parentheses needed to preserve meaning. Going through a tree
// (rather than doing text substitution) is what lets nested cases, like a
// fraction inside a function argument, come out correctly.
//
// Coverage: numbers, single-letter symbols, + - * / ^, \frac, implicit
// multiplication by juxtaposition, function application (\sin(x) -> Sin[x]),
// primes as derivatives (x''(t) -> x''[t]), equality (=  -> ==). No
// subscripts: the demo scope is subscript-free.

type TokenType =
  | "num"
  | "ident"
  | "func"
  | "frac"
  | "sqrt"
  | "prime"
  | "lparen"
  | "rparen"
  | "lbrace"
  | "rbrace"
  | "plus"
  | "minus"
  | "star"
  | "slash"
  | "caret"
  | "equals"
  | "comma"
  | "eof";

interface Token {
  type: TokenType;
  value: string;
  primeCount?: number;
}

// LaTeX command name -> WL function name.
const KNOWN_FUNCTIONS: Record<string, string> = {
  sin: "Sin",
  cos: "Cos",
  tan: "Tan",
  cot: "Cot",
  sec: "Sec",
  csc: "Csc",
  arcsin: "ArcSin",
  arccos: "ArcCos",
  arctan: "ArcTan",
  sinh: "Sinh",
  cosh: "Cosh",
  tanh: "Tanh",
  log: "Log",
  ln: "Log",
  exp: "Exp",
};

// LaTeX symbol command -> WL constant/symbol name.
const KNOWN_SYMBOLS: Record<string, string> = {
  pi: "Pi",
  infty: "Infinity",
  alpha: "Alpha",
  beta: "Beta",
  gamma: "Gamma",
  delta: "Delta",
  theta: "Theta",
  lambda: "Lambda",
  mu: "Mu",
  omega: "Omega",
  phi: "Phi",
  sigma: "Sigma",
};

const SPACE_COMMANDS = new Set(["left", "right", ",", ";", "!", "quad", "qquad", " "]);

class TranslatorError extends Error {}

function tokenize(latex: string): Token[] {
  const tokens: Token[] = [];
  let i = 0;
  const n = latex.length;

  const readCommand = (): string => {
    // i is at the backslash.
    i++;
    let start = i;
    if (i < n && /[a-zA-Z]/.test(latex[i])) {
      while (i < n && /[a-zA-Z]/.test(latex[i])) i++;
      return latex.slice(start, i);
    }
    // Single non-letter command, e.g. \, or \!
    if (i < n) {
      i++;
      return latex.slice(start, i);
    }
    return "";
  };

  while (i < n) {
    const c = latex[i];

    if (/\s/.test(c)) {
      i++;
      continue;
    }

    if (c === "\\") {
      const startI = i;
      const cmd = readCommand();
      if (SPACE_COMMANDS.has(cmd)) continue;
      if (cmd === "prime") {
        tokens.push({ type: "prime", value: "'", primeCount: 1 });
        continue;
      }
      if (cmd === "frac") {
        tokens.push({ type: "frac", value: cmd });
        continue;
      }
      if (cmd === "sqrt") {
        tokens.push({ type: "sqrt", value: cmd });
        continue;
      }
      if (cmd === "cdot" || cmd === "times") {
        tokens.push({ type: "star", value: "*" });
        continue;
      }
      const lower = cmd.toLowerCase();
      if (lower in KNOWN_FUNCTIONS) {
        tokens.push({ type: "func", value: KNOWN_FUNCTIONS[lower] });
        continue;
      }
      if (lower in KNOWN_SYMBOLS) {
        tokens.push({ type: "ident", value: KNOWN_SYMBOLS[lower] });
        continue;
      }
      throw new TranslatorError(`Unsupported LaTeX command \\${cmd || latex.slice(startI, i)}`);
    }

    if (c === "'") {
      tokens.push({ type: "prime", value: "'", primeCount: 1 });
      i++;
      continue;
    }
    if (c === "′") {
      // Unicode prime.
      tokens.push({ type: "prime", value: "'", primeCount: 1 });
      i++;
      continue;
    }
    if (c === "″") {
      // Unicode double prime.
      tokens.push({ type: "prime", value: "''", primeCount: 2 });
      i++;
      continue;
    }

    if (/[0-9]/.test(c)) {
      let start = i;
      while (i < n && /[0-9]/.test(latex[i])) i++;
      if (latex[i] === "." && /[0-9]/.test(latex[i + 1] ?? "")) {
        i++;
        while (i < n && /[0-9]/.test(latex[i])) i++;
      }
      tokens.push({ type: "num", value: latex.slice(start, i) });
      continue;
    }

    if (/[a-zA-Z]/.test(c)) {
      // Each letter is its own symbol: adjacent letters are implicit
      // multiplication, matching WL/math convention (no multi-letter bare
      // identifiers in this scope).
      tokens.push({ type: "ident", value: c });
      i++;
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
      case "{":
        tokens.push({ type: "lbrace", value: c });
        i++;
        continue;
      case "}":
        tokens.push({ type: "rbrace", value: c });
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
      case "=":
        tokens.push({ type: "equals", value: c });
        i++;
        continue;
      case ",":
        tokens.push({ type: "comma", value: c });
        i++;
        continue;
      default:
        throw new TranslatorError(`Unexpected character "${c}" in input`);
    }
  }

  tokens.push({ type: "eof", value: "" });
  return tokens;
}

// --- AST ---

type Node =
  | { kind: "num"; value: string }
  | { kind: "sym"; name: string }
  | { kind: "neg"; operand: Node }
  | { kind: "bin"; op: "+" | "-" | "*" | "/" | "^" | "=="; left: Node; right: Node; explicitMul?: boolean }
  | { kind: "call"; name: string; args: Node[]; primeCount: number };

class Parser {
  private tokens: Token[];
  private pos = 0;

  constructor(tokens: Token[]) {
    this.tokens = tokens;
  }

  private peek(offset = 0): Token {
    return this.tokens[Math.min(this.pos + offset, this.tokens.length - 1)];
  }

  private advance(): Token {
    const t = this.tokens[this.pos];
    if (this.pos < this.tokens.length - 1) this.pos++;
    return t;
  }

  private expect(type: TokenType): Token {
    const t = this.peek();
    if (t.type !== type) {
      throw new TranslatorError(`Expected ${type} but found ${t.type === "eof" ? "end of input" : `"${t.value}"`}`);
    }
    return this.advance();
  }

  parseTop(): Node {
    const node = this.parseEquation();
    if (this.peek().type !== "eof") {
      throw new TranslatorError(`Unexpected trailing input near "${this.peek().value}"`);
    }
    return node;
  }

  private parseEquation(): Node {
    const left = this.parseAddSub();
    if (this.peek().type === "equals") {
      this.advance();
      const right = this.parseAddSub();
      return { kind: "bin", op: "==", left, right };
    }
    return left;
  }

  private parseAddSub(): Node {
    let left = this.parseMulDiv();
    while (this.peek().type === "plus" || this.peek().type === "minus") {
      const op = this.advance().type === "plus" ? "+" : "-";
      const right = this.parseMulDiv();
      left = { kind: "bin", op, left, right };
    }
    return left;
  }

  private startsFactor(): boolean {
    const t = this.peek().type;
    return (
      t === "num" ||
      t === "ident" ||
      t === "func" ||
      t === "frac" ||
      t === "sqrt" ||
      t === "lparen" ||
      t === "lbrace"
    );
  }

  private parseMulDiv(): Node {
    let left = this.parseUnary();
    while (true) {
      if (this.peek().type === "star") {
        this.advance();
        const right = this.parseUnary();
        left = { kind: "bin", op: "*", left, right, explicitMul: true };
      } else if (this.peek().type === "slash") {
        this.advance();
        const right = this.parseUnary();
        left = { kind: "bin", op: "/", left, right };
      } else if (this.startsFactor()) {
        const right = this.parseUnary();
        left = { kind: "bin", op: "*", left, right, explicitMul: false };
      } else {
        break;
      }
    }
    return left;
  }

  // Unary minus binds looser than power, so "-x^2" parses as "-(x^2)",
  // matching WL and standard math convention.
  private parseUnary(): Node {
    if (this.peek().type === "minus") {
      this.advance();
      return { kind: "neg", operand: this.parseUnary() };
    }
    if (this.peek().type === "plus") {
      this.advance();
      return this.parseUnary();
    }
    return this.parsePow();
  }

  private parsePow(): Node {
    const base = this.parsePostfix();
    if (this.peek().type === "caret") {
      this.advance();
      let exponent: Node;
      if (this.peek().type === "lbrace") {
        this.advance();
        exponent = this.parseAddSub();
        this.expect("rbrace");
      } else {
        exponent = this.parseUnary();
      }
      return { kind: "bin", op: "^", left: base, right: exponent };
    }
    return base;
  }

  private collectPrimes(): number {
    let count = 0;
    while (true) {
      if (this.peek().type === "prime") {
        count += this.advance().primeCount ?? 1;
        continue;
      }
      if (this.peek().type === "caret" && this.peek(1).type === "lbrace" && this.isPrimeBlock(this.pos + 2)) {
        this.advance(); // caret
        this.advance(); // lbrace
        while (this.peek().type === "prime") {
          count += this.advance().primeCount ?? 1;
        }
        this.expect("rbrace");
        continue;
      }
      break;
    }
    return count;
  }

  private isPrimeBlock(startPos: number): boolean {
    let p = startPos;
    let sawPrime = false;
    while (this.tokens[p] && this.tokens[p].type === "prime") {
      sawPrime = true;
      p++;
    }
    return sawPrime && this.tokens[p]?.type === "rbrace";
  }

  private parsePostfix(): Node {
    const node = this.parseAtom();

    if (node.kind === "sym") {
      const primeCount = this.collectPrimes();
      if (this.peek().type === "lparen") {
        this.advance();
        const args = this.parseArgList();
        this.expect("rparen");
        return { kind: "call", name: node.name, args, primeCount };
      }
      if (primeCount > 0) {
        // A bare symbol with primes and no call, e.g. just "x'". Treat as a
        // zero-argument-looking derivative marker kept as a symbol name for
        // simplicity: WL has no clean bare-prime-no-args form, so we render
        // it as a plain primed symbol name.
        return { kind: "sym", name: node.name + "'".repeat(primeCount) };
      }
      return node;
    }

    if (node.kind === "call") {
      // Function calls (from \sin etc.) do not take primes in this scope.
      return node;
    }

    return node;
  }

  private parseArgList(): Node[] {
    const args: Node[] = [this.parseAddSub()];
    while (this.peek().type === "comma") {
      this.advance();
      args.push(this.parseAddSub());
    }
    return args;
  }

  private parseAtom(): Node {
    const t = this.peek();
    switch (t.type) {
      case "num":
        this.advance();
        return { kind: "num", value: t.value };
      case "ident":
        this.advance();
        return { kind: "sym", name: t.value };
      case "func": {
        this.advance();
        let args: Node[];
        if (this.peek().type === "lparen") {
          this.advance();
          args = this.parseArgList();
          this.expect("rparen");
        } else if (this.peek().type === "lbrace") {
          this.advance();
          args = [this.parseAddSub()];
          this.expect("rbrace");
        } else {
          throw new TranslatorError(`Expected an argument after ${t.value}`);
        }
        return { kind: "call", name: t.value, args, primeCount: 0 };
      }
      case "frac": {
        this.advance();
        this.expect("lbrace");
        const num = this.parseAddSub();
        this.expect("rbrace");
        this.expect("lbrace");
        const den = this.parseAddSub();
        this.expect("rbrace");
        return { kind: "bin", op: "/", left: num, right: den };
      }
      case "sqrt": {
        this.advance();
        this.expect("lbrace");
        const radicand = this.parseAddSub();
        this.expect("rbrace");
        return { kind: "call", name: "Sqrt", args: [radicand], primeCount: 0 };
      }
      case "lparen": {
        this.advance();
        const inner = this.parseEquation();
        this.expect("rparen");
        return inner;
      }
      case "lbrace": {
        this.advance();
        const inner = this.parseEquation();
        this.expect("rbrace");
        return inner;
      }
      default:
        throw new TranslatorError(`Unexpected token "${t.value || t.type}"`);
    }
  }
}

// --- Pretty printer ---

function nodePrec(node: Node): number {
  switch (node.kind) {
    case "num":
    case "sym":
    case "call":
      return 6;
    case "neg":
      return 4;
    case "bin":
      if (node.op === "^") return 5;
      if (node.op === "*" || node.op === "/") return 3;
      if (node.op === "+" || node.op === "-") return 2;
      return 1; // ==
  }
}

function print(node: Node, minPrec: number): string {
  const inner = printInner(node);
  return nodePrec(node) < minPrec ? `(${inner})` : inner;
}

function printInner(node: Node): string {
  switch (node.kind) {
    case "num":
      return node.value;
    case "sym":
      return node.name;
    case "neg":
      return `-${print(node.operand, 4)}`;
    case "call": {
      const args = node.args.map((a) => print(a, 1)).join(", ");
      return `${node.name}${"'".repeat(node.primeCount)}[${args}]`;
    }
    case "bin": {
      switch (node.op) {
        case "+":
          return `${print(node.left, 2)} + ${print(node.right, 3)}`;
        case "-":
          return `${print(node.left, 2)} - ${print(node.right, 3)}`;
        case "*":
          return `${print(node.left, 3)}${node.explicitMul ? " * " : " "}${print(node.right, 4)}`;
        case "/":
          return `${print(node.left, 3)}/${print(node.right, 4)}`;
        case "^":
          return `${print(node.left, 6)}^${print(node.right, 5)}`;
        case "==":
          return `${print(node.left, 2)} == ${print(node.right, 2)}`;
      }
    }
  }
}

export class TranslatorParseError extends Error {}

/**
 * Translate LaTeX (as produced by a MathLive math-field) into OpenMat's
 * WL-shaped linear syntax. Throws TranslatorParseError on malformed input.
 */
export function translateLatexToWL(latex: string): string {
  const trimmed = latex.trim();
  if (trimmed === "") return "";
  try {
    const tokens = tokenize(trimmed);
    const tree = new Parser(tokens).parseTop();
    return printInner(tree);
  } catch (err) {
    if (err instanceof TranslatorError) {
      throw new TranslatorParseError(err.message);
    }
    throw err;
  }
}
