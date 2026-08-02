// Translates LaTeX produced by a MathLive math-field into OpenMat's
// WL-shaped linear syntax (see ARCHITECTURE.md, "App-to-kernel contract").
//
// Approach: tokenize the LaTeX subset we support, parse it into a small
// expression tree, then pretty-print that tree as linear syntax with the
// minimum parentheses needed to preserve meaning. Going through a tree
// (rather than doing text substitution) is what lets nested cases, like a
// fraction inside a function argument, come out correctly.
//
// Coverage: numbers, symbols, + - * / ^, \frac, \sqrt, implicit
// multiplication by juxtaposition, function application in both notations
// (\sin(x) -> Sin[x], Plot[...] -> Plot[...]), typed-brace lists
// (\lbrace x,0,10\rbrace -> {x, 0, 10}), rules (\to -> ->), primes as
// derivatives (x''(t) -> x''[t]), integrals and sums with bounds, and
// Mathematica's =/== split (Set when the left side is assignable, Equal
// otherwise).
//
// Symbol convention matches Mathematica: consecutive letters form ONE
// symbol (xy is the symbol xy, not x*y); juxtaposition across a boundary
// (2x, a\,b, a b) is implicit multiplication.

type TokenType =
  | "num"
  | "ident"
  | "func"
  | "frac"
  | "sqrt"
  | "prime"
  | "lparen"
  | "rparen"
  | "lbrace"   // structural LaTeX group: ^{...}, \frac{...}{...}
  | "rbrace"
  | "lbrack"   // typed [ -> WL function call
  | "rbrack"
  | "llist"    // typed { (\lbrace) -> WL list
  | "rlist"
  | "plus"
  | "minus"
  | "star"
  | "slash"
  | "caret"
  | "under"
  | "equals"
  | "eqeq"
  | "arrow"
  | "int"
  | "sum"
  | "dd"       // \differentialD (MathLive's integral differential)
  | "comma"
  | "eof";

interface Token {
  type: TokenType;
  value: string;
  primeCount?: number;
}

// LaTeX command name -> WL function name. Doubles as the normalizer for
// lowercase heads typed as plain letters ("sin[x]" -> Sin[x]).
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
  min: "Min",
  max: "Max",
  abs: "Abs",
};

// WL heads that may be applied with parentheses as well as brackets
// (users coming from math notation type Plot(...), Sin(x)). A bare
// multi-letter symbol NOT in this set followed by ( is multiplication,
// matching Mathematica, where only [ ] is application syntax.
const KNOWN_HEADS = new Set([
  "Sin", "Cos", "Tan", "Cot", "Sec", "Csc",
  "ArcSin", "ArcCos", "ArcTan", "Sinh", "Cosh", "Tanh",
  "Log", "Exp", "Sqrt", "Abs", "N",
  "Plot", "ListPlot", "NDSolve", "DSolve", "Solve", "NSolve",
  "D", "Dt", "Integrate", "NIntegrate", "Sum", "Product",
  "Limit", "Series", "Expand", "Factor", "Simplify", "FullSimplify",
  "Table", "Range", "Min", "Max", "Floor", "Ceiling", "Round",
  "Mod", "GCD", "LCM", "Manipulate",
]);

// LaTeX symbol command -> WL constant/symbol name.
const KNOWN_SYMBOLS: Record<string, string> = {
  pi: "Pi",
  infty: "Infinity",
  exponentialE: "E",
  imaginaryI: "I",
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

const SPACE_COMMANDS = new Set(["left", "right", ",", ";", "!", "quad", "qquad", " ", "mathrm", "text", "operatorname"]);

class TranslatorError extends Error {}

function tokenize(latex: string): Token[] {
  // MathLive's inline shortcuts can fire mid-word: typing "Sin" becomes
  // "S\in" (the element-of shortcut). Glue such commands back onto the
  // preceding letter so the identifier survives.
  const source = latex.replace(/([A-Za-z])\\in(?![a-zA-Z])/g, "$1in");

  const tokens: Token[] = [];
  let i = 0;
  const n = source.length;

  const readCommand = (): string => {
    // i is at the backslash.
    i++;
    let start = i;
    if (i < n && /[a-zA-Z]/.test(source[i])) {
      while (i < n && /[a-zA-Z]/.test(source[i])) i++;
      return source.slice(start, i);
    }
    // Single non-letter command, e.g. \, or \{
    if (i < n) {
      i++;
      return source.slice(start, i);
    }
    return "";
  };

  while (i < n) {
    const c = source[i];

    if (/\s/.test(c)) {
      i++;
      continue;
    }

    if (c === "\\") {
      const startI = i;
      const cmd = readCommand();
      if (SPACE_COMMANDS.has(cmd)) continue;
      if (cmd === "{") {
        tokens.push({ type: "llist", value: "{" });
        continue;
      }
      if (cmd === "}") {
        tokens.push({ type: "rlist", value: "}" });
        continue;
      }
      if (cmd === "lbrace") {
        tokens.push({ type: "llist", value: "{" });
        continue;
      }
      if (cmd === "rbrace") {
        tokens.push({ type: "rlist", value: "}" });
        continue;
      }
      if (cmd === "lbrack") {
        tokens.push({ type: "lbrack", value: "[" });
        continue;
      }
      if (cmd === "rbrack") {
        tokens.push({ type: "rbrack", value: "]" });
        continue;
      }
      if (cmd === "prime") {
        tokens.push({ type: "prime", value: "'", primeCount: 1 });
        continue;
      }
      if (cmd === "doubleprime") {
        tokens.push({ type: "prime", value: "''", primeCount: 2 });
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
      if (cmd === "to" || cmd === "rightarrow" || cmd === "Rightarrow") {
        tokens.push({ type: "arrow", value: "->" });
        continue;
      }
      if (cmd === "int") {
        tokens.push({ type: "int", value: cmd });
        continue;
      }
      if (cmd === "sum") {
        tokens.push({ type: "sum", value: cmd });
        continue;
      }
      if (cmd === "differentialD") {
        tokens.push({ type: "dd", value: "d" });
        continue;
      }
      const lower = cmd.toLowerCase();
      if (lower in KNOWN_FUNCTIONS) {
        tokens.push({ type: "func", value: KNOWN_FUNCTIONS[lower] });
        continue;
      }
      if (cmd in KNOWN_SYMBOLS) {
        tokens.push({ type: "ident", value: KNOWN_SYMBOLS[cmd] });
        continue;
      }
      if (lower in KNOWN_SYMBOLS) {
        tokens.push({ type: "ident", value: KNOWN_SYMBOLS[lower] });
        continue;
      }
      throw new TranslatorError(`Unsupported LaTeX command \\${cmd || source.slice(startI, i)}`);
    }

    if (c === "'") {
      tokens.push({ type: "prime", value: "'", primeCount: 1 });
      i++;
      continue;
    }
    if (c === "′") {
      tokens.push({ type: "prime", value: "'", primeCount: 1 });
      i++;
      continue;
    }
    if (c === "″") {
      tokens.push({ type: "prime", value: "''", primeCount: 2 });
      i++;
      continue;
    }

    if (/[0-9]/.test(c) || (c === "." && /[0-9]/.test(source[i + 1] ?? ""))) {
      let start = i;
      while (i < n && /[0-9]/.test(source[i])) i++;
      if (source[i] === "." && /[0-9]/.test(source[i + 1] ?? "")) {
        i++;
        while (i < n && /[0-9]/.test(source[i])) i++;
      }
      tokens.push({ type: "num", value: source.slice(start, i) });
      continue;
    }

    if (/[a-zA-Z]/.test(c)) {
      // Consecutive letters form ONE symbol, exactly as in Mathematica:
      // "Plot" is the head Plot, "xy" is the symbol xy. Implicit
      // multiplication needs a boundary (a digit, \, spacing, an operator).
      let start = i;
      while (i < n && /[a-zA-Z]/.test(source[i])) i++;
      tokens.push({ type: "ident", value: source.slice(start, i) });
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
      case "[":
        tokens.push({ type: "lbrack", value: c });
        i++;
        continue;
      case "]":
        tokens.push({ type: "rbrack", value: c });
        i++;
        continue;
      case "+":
        tokens.push({ type: "plus", value: c });
        i++;
        continue;
      case "-":
        if (source[i + 1] === ">") {
          tokens.push({ type: "arrow", value: "->" });
          i += 2;
          continue;
        }
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
      case "_":
        tokens.push({ type: "under", value: c });
        i++;
        continue;
      case "=":
        if (source[i + 1] === "=") {
          tokens.push({ type: "eqeq", value: "==" });
          i += 2;
          continue;
        }
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
  | { kind: "list"; items: Node[] }
  | { kind: "bin"; op: "+" | "-" | "*" | "/" | "^" | "==" | "->" | "="; left: Node; right: Node; explicitMul?: boolean }
  | { kind: "call"; name: string; args: Node[]; primeCount: number };

/** Set (a = 5, f[x] = ...) vs Equal (x^2 + y^2 = 4): a typed "=" is
 * Mathematica's Set only when the left side could actually take a
 * definition; for any other left side the user means an equation. */
function isAssignable(node: Node): boolean {
  return node.kind === "sym" || node.kind === "call";
}

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
    const node = this.parseSet();
    if (this.peek().type !== "eof") {
      throw new TranslatorError(`Unexpected trailing input near "${this.peek().value}"`);
    }
    return node;
  }

  // Loosest level, right-associative like WL: a = b = 5 is a = (b = 5).
  private parseSet(): Node {
    const left = this.parseRule();
    if (this.peek().type === "equals") {
      this.advance();
      const right = this.parseSet();
      return { kind: "bin", op: isAssignable(left) ? "=" : "==", left, right };
    }
    return left;
  }

  private parseRule(): Node {
    const left = this.parseEquation();
    if (this.peek().type === "arrow") {
      this.advance();
      const right = this.parseRule();
      return { kind: "bin", op: "->", left, right };
    }
    return left;
  }

  private parseEquation(): Node {
    let left = this.parseAddSub();
    while (this.peek().type === "eqeq") {
      this.advance();
      const right = this.parseAddSub();
      left = { kind: "bin", op: "==", left, right };
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
      t === "int" ||
      t === "sum" ||
      t === "lparen" ||
      t === "lbrace" ||
      t === "llist"
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

  /** Normalize a head typed as plain letters: lowercase aliases of known
   * functions become their WL names (sin -> Sin); anything else is kept
   * as typed, since WL symbols are case-sensitive. */
  private normalizeHead(name: string): string {
    return KNOWN_FUNCTIONS[name.toLowerCase()] && name === name.toLowerCase() ? KNOWN_FUNCTIONS[name.toLowerCase()] : name;
  }

  private parsePostfix(): Node {
    const node = this.parseAtom();

    if (node.kind === "sym") {
      const primeCount = this.collectPrimes();
      // Bracket application is always a call, exactly as in WL.
      if (this.peek().type === "lbrack") {
        this.advance();
        const args = this.peek().type === "rbrack" ? [] : this.parseArgList();
        this.expect("rbrack");
        return { kind: "call", name: this.normalizeHead(node.name), args, primeCount };
      }
      // Paren application only for single-letter symbols (f(x), x'(t)),
      // primed symbols (cx'(t) is the derivative of the function cx), and
      // known heads (Plot(...), Sin(x)); any other symbol followed by ( is
      // multiplication, as in Mathematica.
      if (
        this.peek().type === "lparen" &&
        (node.name.length === 1 || primeCount > 0 || KNOWN_HEADS.has(this.normalizeHead(node.name)))
      ) {
        this.advance();
        const args = this.peek().type === "rparen" ? [] : this.parseArgList();
        this.expect("rparen");
        return { kind: "call", name: this.normalizeHead(node.name), args, primeCount };
      }
      if (primeCount > 0) {
        // A bare symbol with primes and no call, e.g. just "x'". WL has no
        // clean bare-prime-no-args form, so keep it as a primed name.
        return { kind: "sym", name: node.name + "'".repeat(primeCount) };
      }
      return node;
    }

    return node;
  }

  private parseArgList(): Node[] {
    const args: Node[] = [this.parseSet()];
    while (this.peek().type === "comma") {
      this.advance();
      args.push(this.parseSet());
    }
    return args;
  }

  /** Parse one bound of an integral/sum: a braced group or a single
   * (possibly negated) atom. Deliberately NOT parseUnary: in "\int_0^1"
   * the ^1 is the upper bound, and a power-aware parse of the lower bound
   * would swallow it as 0^1. */
  private parseBound(): Node {
    if (this.peek().type === "lbrace") {
      this.advance();
      const inner = this.parseSet();
      this.expect("rbrace");
      return inner;
    }
    if (this.peek().type === "minus") {
      this.advance();
      return { kind: "neg", operand: this.parseAtom() };
    }
    return this.parseAtom();
  }

  /** \int_a^b body \differentialD x  ->  Integrate[body, {x, a, b}].
   * Without bounds: Integrate[body, x]. The differential may arrive as
   * MathLive's \differentialD token or as a plain "dx" identifier. */
  private parseIntegral(): Node {
    let lower: Node | undefined;
    let upper: Node | undefined;
    while (this.peek().type === "under" || this.peek().type === "caret") {
      if (this.advance().type === "under") lower = this.parseBound();
      else upper = this.parseBound();
    }

    let body: Node;
    let variable: string;
    if (this.peek().type === "dd") {
      // \int ... \differentialD x with an empty integrand means integrand 1.
      body = { kind: "num", value: "1" };
      this.advance();
      variable = this.expect("ident").value;
    } else {
      body = this.parseAddSub();
      if (this.peek().type === "dd") {
        this.advance();
        variable = this.expect("ident").value;
      } else {
        const stripped = stripTrailingDifferential(body);
        if (!stripped) {
          throw new TranslatorError("Integral is missing its differential (dx)");
        }
        [body, variable] = stripped;
      }
    }

    const args: Node[] =
      lower !== undefined && upper !== undefined
        ? [body, { kind: "list", items: [{ kind: "sym", name: variable }, lower, upper] }]
        : [body, { kind: "sym", name: variable }];
    return { kind: "call", name: "Integrate", args, primeCount: 0 };
  }

  /** \sum_{n=1}^{10} body -> Sum[body, {n, 1, 10}]. */
  private parseSum(): Node {
    let lower: Node | undefined;
    let upper: Node | undefined;
    while (this.peek().type === "under" || this.peek().type === "caret") {
      if (this.advance().type === "under") lower = this.parseBound();
      else upper = this.parseBound();
    }
    if (!lower || lower.kind !== "bin" || (lower.op !== "==" && lower.op !== "=") || lower.left.kind !== "sym" || !upper) {
      throw new TranslatorError("Sum needs bounds of the form n=1 below and a limit above");
    }
    const body = this.parseMulDiv();
    return {
      kind: "call",
      name: "Sum",
      args: [body, { kind: "list", items: [lower.left, lower.right, upper] }],
      primeCount: 0,
    };
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
        } else if (this.peek().type === "lbrack") {
          this.advance();
          args = this.parseArgList();
          this.expect("rbrack");
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
      case "int": {
        this.advance();
        return this.parseIntegral();
      }
      case "sum": {
        this.advance();
        return this.parseSum();
      }
      case "lparen": {
        this.advance();
        const inner = this.parseSet();
        this.expect("rparen");
        return inner;
      }
      case "lbrace": {
        this.advance();
        const inner = this.parseSet();
        this.expect("rbrace");
        return inner;
      }
      case "llist": {
        this.advance();
        if (this.peek().type === "rlist") {
          this.advance();
          return { kind: "list", items: [] };
        }
        const items: Node[] = [this.parseSet()];
        while (this.peek().type === "comma") {
          this.advance();
          items.push(this.parseSet());
        }
        this.expect("rlist");
        return { kind: "list", items };
      }
      default:
        throw new TranslatorError(`Unexpected token "${t.value || t.type}"`);
    }
  }
}

/** For an integral typed without \differentialD (plain "dx" letters, which
 * tokenize as the single symbol dx): find and remove the trailing d-prefixed
 * factor, returning the remaining body and the integration variable. */
function stripTrailingDifferential(node: Node): [Node, string] | null {
  if (node.kind === "sym" && node.name.length >= 2 && node.name.startsWith("d")) {
    return [{ kind: "num", value: "1" }, node.name.slice(1)];
  }
  if (node.kind === "bin" && node.op === "*") {
    const right = stripTrailingDifferential(node.right);
    if (right) {
      const [rest, variable] = right;
      if (rest.kind === "num" && rest.value === "1") return [node.left, variable];
      return [{ ...node, right: rest }, variable];
    }
  }
  if (node.kind === "neg") {
    const inner = stripTrailingDifferential(node.operand);
    if (inner) return [{ kind: "neg", operand: inner[0] }, inner[1]];
  }
  return null;
}

// --- Pretty printer ---

function nodePrec(node: Node): number {
  switch (node.kind) {
    case "num":
    case "sym":
    case "call":
    case "list":
      return 6;
    case "neg":
      return 4;
    case "bin":
      if (node.op === "^") return 5;
      if (node.op === "*" || node.op === "/") return 3;
      if (node.op === "+" || node.op === "-") return 2;
      if (node.op === "==") return 1;
      if (node.op === "->") return 0.8;
      return 0.5; // =
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
    case "list":
      return `{${node.items.map((item) => print(item, 0)).join(", ")}}`;
    case "call": {
      const args = node.args.map((a) => print(a, 0)).join(", ");
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
        case "->":
          return `${print(node.left, 1)} -> ${print(node.right, 1)}`;
        case "=":
          return `${print(node.left, 0.8)} = ${print(node.right, 0.5)}`;
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
