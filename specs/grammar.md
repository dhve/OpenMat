# OpenMat surface grammar and box conversion, v0.2

Normative for M1. This documents what the implemented parser
(crates/openmat-core) and the MathLive translator
(app/src/mathlive/translator.ts) accept and produce. Conformance fixtures
live in specs/fixtures/ and run in both test suites; a change to this
document requires a version bump and fixture updates.

v0.2 adds pattern surface syntax, `(* ... *)` comments, user definitions
(`Set`/`SetDelayed`/`Clear`), the comparison operators, the `/@` (Map)
operator, and the named constants `Pi`/`E`. Everything from v0.1 (specs
history: specs/fixtures/grammar-v0.1.txt) still holds unchanged; this file
supersedes specs/grammar.md v0.1 rather than living alongside it.

## 1. Lexical grammar

- Whitespace separates tokens and is otherwise insignificant, except where
  called out below: adjacency (no whitespace) is significant around pattern
  syntax (section 1.2).
- Symbols: ASCII letter or `$`, followed by alphanumerics or `$`. Unlike
  v0.1, `_` is no longer an identifier character at all, matching real
  Wolfram Language: `x_` now lexes as two tokens (a symbol and a pattern
  marker), not one identifier. `$` stays valid throughout, matching WL's
  context-mark convention.
- Numeric literals: integer (`123`) and real (`1.5`, `0.5`, `1e10`,
  `2.5e-3`). No `` 2`10 `` precision marks, no base notation.
- Strings: double-quoted.
- Comments: `(* ... *)`, and they nest: `(* outer (* inner *) still a
  comment *)` is one comment, closed only by its outer `*)`. Comments are
  stripped by the lexer before the parser ever sees a token, so they may
  appear anywhere whitespace may (including inside an argument list, between
  operators, or wrapping the whole input) and never affect the parsed
  expression.
- Operators and delimiters: `+ - * / ^ == != < > <= >= -> := = ' /@ ( ) [ ]
  { } ,`

### 1.1 Pattern tokens

A run of one, two, or three underscores lexes as a single `Blank` token
(not as identifier characters): `_`, `__`, `___`. Four or more consecutive
underscores is a lex error. See section 4 for what each count means.

### 1.2 Adjacency around patterns

`x_` (no space) is a named pattern; `x _` (space before the underscore) is
implicit multiplication of `x` and a bare `_` pattern, following the same
"any two adjacent primaries multiply" rule as `2 x`. Adjacency is likewise
what attaches a type restriction: `x_Integer` (no space anywhere) is one
pattern, `x_ Integer` is a named pattern followed by implicit
multiplication by the symbol `Integer`. The parser tracks this via token
byte positions, not a lexer-level fusion of the tokens.

## 2. Precedence and associativity

From loosest to tightest binding, matching the recursive-descent chain in
the parser:

| Level | Forms | Associativity |
|---|---|---|
| assign | `a = b` (Set), `a := b` (SetDelayed) | right |
| rule | `a -> b` (Rule) | right |
| map | `f /@ a` (Map) | right |
| relational | `a == b`, `a != b`, `a < b`, `a > b`, `a <= b`, `a >= b` | non-chaining |
| additive | `a + b`, `a - b` (Plus; subtraction is Plus with Times[-1, ...]) | left |
| multiplicative | `a * b`, `a / b`, implicit multiplication (Times; division is Times with Power[..., -1]) | left |
| unary | `-a` | prefix |
| power | `a ^ b` (Power, right associative) | right |
| postfix | `f[args]` application, `'` derivative primes | left |
| primary | number, symbol, string, pattern (`_`, `x_`, ...), `{...}` list, `(...)` grouping | |

Consequences fixed by fixtures: `-x^2` parses as `Times[-1, Power[x, 2]]`;
`a/b/c` is `(a/b)/c`; `2^3^2` is `2^(3^2)`; `a = b = 5` is `Set[a, Set[b,
5]]` (assignment is right associative, so chained assignment reads as
"assign the whole rest of the line").

The relational operators do not chain against each other or against `==`:
each `parse_equal` call consumes one comparison and, per this pass, treats
a second comparison at the same level as a fresh one built on the previous
result (`a < b < c` is `Less[Less[a,b], c]`, not the three-way inequality
real Wolfram Language builds) — a known simplification, not the WL
`Inequality` semantics.

## 3. Implicit multiplication

Two adjacent primary-or-tighter expressions multiply: `2 x`, `c x[t]`,
`3 Sin[t]`, `2(x+1)`, `x _` (a symbol next to a bare pattern, section 1.2).
Implicit multiplication binds at the multiplicative level. A symbol
immediately followed by `[` is function application, never multiplication.

## 4. Pattern surface syntax

Patterns are ordinary expressions, exactly as in Wolfram Language's own
`FullForm`, and print back in the same compact surface form they were
parsed from:

| Surface | `FullForm` | Meaning |
|---|---|---|
| `_` | `Blank[]` | matches anything |
| `_Integer` | `Blank[Integer]` | matches anything with head `Integer` |
| `x_` | `Pattern[x, Blank[]]` | matches anything, binds it to `x` |
| `x_Integer` | `Pattern[x, Blank[Integer]]` | matches an `Integer`, binds it to `x` |
| `__` | `BlankSequence[]` | matches one or more expressions (parse-only, see below) |
| `___` | `BlankNullSequence[]` | matches zero or more expressions (parse-only, see below) |
| `x__`, `x___` | `Pattern[x, BlankSequence[]]`, `Pattern[x, BlankNullSequence[]]` | named sequence patterns (parse-only) |

`BlankSequence`/`BlankNullSequence` are parse-only in this pass: they build
the right `Expr`, and print/round-trip correctly, but
`crate::pattern::pattern_match` does not yet match a sequence against
multiple actual arguments (it only handles `Blank`, named patterns, and
literal structural matching; see the module doc in `pattern.rs`).

## 5. User definitions

`lhs = rhs` (`Set`) and `lhs := rhs` (`SetDelayed`) are ordinary
expressions usable anywhere an expression is expected (matching WL), not a
separate statement form. Two shapes on the left matter to the evaluator:

- A bare symbol (`a = 5`): an **OwnValue**. Evaluating the symbol later
  evaluates to the bound value.
- A call whose head is a symbol (`f[x_] := x^2`, `f[x_] = x^2`): a
  **DownValue** on that symbol. Evaluating a later call `f[3]` matches `3`
  against the stored argument patterns and substitutes into the right-hand
  side. `Set` evaluates the right-hand side once, at definition time;
  `SetDelayed` stores it unevaluated and evaluates it fresh on every
  application, so a `SetDelayed` body can depend on OwnValues that change
  later (`c = 2; f[x_] := c*x; f[10]` gives `20`; after `c = 3`, `f[10]`
  gives `30`). `Set` returns the assigned value; `SetDelayed` returns
  `Null`.

Redefining a function with the exact same argument pattern replaces the
rule in place rather than adding a second one. Definitions are otherwise
tried in the order they were made, first match wins: unlike real Wolfram
Language, rules are not reordered by pattern specificity.

`Clear[f]` removes both the OwnValue and any DownValues on file for `f`
(`Clear`'s argument is held, so `Clear[f]` never evaluates `f` itself).

Definitions live on the `Evaluator` instance that made them (see
`eval.rs`'s module doc for why this is a `RefCell`-backed table rather than
requiring `&mut self`), not globally: a fresh `Evaluator::new()` starts
with no definitions. A caller that wants `f[x_] := x^2` from one input to
be visible to `f[3]` in a later input must reuse the same `Evaluator`
across both `eval` calls.

## 6. Structural builtins

Evaluated eagerly wherever their shape matches (numeric folding uses `f64`
comparison for the comparison operators; everything else stays exact where
the input is exact):

- `Table[expr, {i, n}]` and `Table[expr, {i, a, b}]`: instantiate `expr`
  with `i` bound to each integer from `1` (or `a`) through `n` (or `b`)
  inclusive, evaluating each instance. `Table` holds all its arguments, so
  neither `expr` nor the iterator spec is evaluated before the loop
  variable is substituted in (an existing binding for a symbol named the
  same as the loop variable never leaks in).
- `Range[n]` and `Range[a, b]`: the integer list `1, ..., n` or `a, ...,
  b`.
- `Map[f, expr]` and the infix `f /@ expr`: apply `f` to each argument of
  `expr` (level 1), keeping `expr`'s own head (`Map[f, {a,b}]` is `{f[a],
  f[b]}`; `Map[f, g[a,b]]` is `g[f[a], f[b]]`).
- `Length[expr]`: the number of arguments of `expr` (0 for an atom).
- `First[expr]`, `Rest[expr]`: the first argument, and every argument after
  the first, keeping `expr`'s own head. Stay symbolic (unevaluated) on an
  empty or atomic `expr` rather than erroring.
- `Total[expr]`: the sum of `expr`'s arguments, via the same numeric
  folding `Plus` gets during ordinary evaluation.
- `Less`, `Greater`, `LessEqual`, `GreaterEqual` (`< > <= >=`), `Unequal`
  (`!=`): fold to the symbols `True`/`False` when both sides are numeric
  (`Integer` or `Real`); stay symbolic otherwise. `Equal` (`==`) is
  unchanged from v0.1 and does not fold (it stays symbolic even for numeric
  arguments, since `openmat-kernel`'s NDSolve dispatch relies on `==` never
  reducing away before it extracts an equation's two sides).
- `If[cond, then, else]`: evaluates `cond`; if it is exactly `True` or
  `False`, evaluates and returns the matching branch only (the other branch
  is never evaluated). Otherwise returns `If[cond, then, else]` with `cond`
  evaluated but both branches still held.

## 7. Constants

`Pi` and `E` are ordinary symbols with no numeric value on their own (`Pi`
prints as `Pi`, `Pi + 1` stays `1 + Pi`). `N[expr]` treats them specially,
substituting `Pi` and `E` with their `f64` values (`std::f64::consts::PI`
and `std::f64::consts::E`) alongside its usual `Integer` to `Real`
conversion, so `N[Pi]` is `3.14159...` and `N[E]` is `2.71828...`. A small
table of exact trig identities at `Pi` (`Sin[Pi] = 0`, `Cos[Pi] = -1`,
`Tan[Pi] = 0`) is checked before the general numeric builtin library, so
these fold even without `N`. Nothing else about `Pi`/`E` is special-cased:
`Exp[1]` stays symbolic (`Exp` has no `Pi`/`E`-specific rule), matching how
`Exp[3]` stays symbolic in v0.1.

## 8. Derivatives

Postfix primes on a symbol before application: `x'[t]` is
`Derivative[1][x][t]`, `x''[t]` is `Derivative[2][x][t]`. Primes without
application (`x'`) produce `Derivative[n][x]`.

## 9. Diagnostics and recovery

- Parse errors report a message and a byte-offset position. Error shape is
  stable: the kernel returns them in the result's error field, never a
  crash.
- The parser does not error-recover in v0.2: first error wins.
- Editor rule (normative for the app): incomplete or invalid 2D input stays
  editable in the MathLive field and is never sent to the kernel as if it
  were a valid expression; evaluation of invalid input surfaces the
  position-tagged error in the output cell.

## 10. Box conversion: MathLive LaTeX to linear syntax

The translator accepts the LaTeX MathLive emits for the supported box model
and produces linear syntax per sections 1 to 3. Supported: numbers,
identifiers, `+ - * / ^`, `\frac{a}{b}` to `(a)/(b)`, `a^{b}` to `a^(b)`,
primes, `=` and `==` to `==`, `\sin` etc to `Sin[...]` for the known
function set (Sin, Cos, Tan, Exp, Log, Sqrt, Abs), parenthesized
application converted to square brackets for known functions,
`\left(\right)` grouping, `\pi` to Pi, `e` context-dependent to E, lists
`\{...\}`. Anything outside this set is a translation error shown to the
user, not silently dropped. The v0.2 additions (patterns, comments, `/@`,
`Set`/`SetDelayed`, the relational operators) have no 2D box form yet and
are linear-syntax-only until the translator picks them up.

## 11. Box conversion: expression to LaTeX

`openmat_core::to_latex` renders: fractions for negative powers and Divide
shapes, `x^{n}` superscripts, `\sin`-style operators for known functions,
primes or `^{(n)}` for Derivative, `\sqrt`, `\left( \right)` where
precedence requires, proper unary minus, `\pi` for the constant `Pi`
(regardless of capitalization, unlike the general Greek-letter-name
rendering used for ordinary symbols named after Greek letters). Patterns
render with a literal escaped underscore (`\_`, `\_\_`, `\_\_\_`), which
KaTeX renders as a literal underscore glyph; this is legible but not
polished 2D pattern notation, a known scope cut for this pass. Output must
be KaTeX-renderable.

## 12. Conformance fixtures

- specs/fixtures/grammar-v0.1.txt: the original v0.1 fixture set, still in
  force and still passing unchanged; new v0.2 surface does not appear
  there.
- specs/fixtures/grammar-v0.2.txt: fixtures for everything new in this
  version (patterns, comments, relational operators, `/@`, `Pi`,
  `Set`/`SetDelayed`, `If`). Same `input || canonical InputForm || LaTeX`
  format as v0.1.
- Both files are replayed by crates/openmat-core/tests/conformance.rs,
  which for each fixture line: parses the input, checks the canonical
  printed form, re-parses the printed form and checks it evaluates to the
  same expression as the original parse (round trip up to evaluation: the
  printer may regroup, e.g. `a/b/c` prints as `a/(b*c)`; canonicalization
  makes them equal), and checks the LaTeX rendering.
- The MathLive translator fixtures live in
  app/src/mathlive/translator.test.ts and must cover every supported box
  form in section 10; they do not yet cover v0.2 surface, which has no box
  form.
