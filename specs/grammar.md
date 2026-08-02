# OpenMat surface grammar and box conversion, v0.1

Normative for M0. This documents what the implemented parser (crates/openmat-core) and the MathLive translator (app/src/mathlive/translator.ts) accept and produce. Conformance fixtures live in specs/fixtures/ and run in both test suites; a change to this document requires a version bump and fixture updates.

## 1. Lexical grammar

- Whitespace separates tokens and is otherwise insignificant.
- Symbols: ASCII letter, `_`, or `$`, followed by alphanumerics, `_`, or `$`. Known deviation from WL: `_` is currently an identifier character, so `x_` lexes as one symbol rather than `Pattern[x, Blank[]]`. Pattern surface syntax (`_`, `x_`, `_Head`, `__`, `___`) is scheduled for M1; until then patterns are built programmatically.
- Numeric literals: integer (`123`) and real (`1.5`, `0.5`). No exponent notation, no `` 2`10 `` precision marks, no base notation in v0.1.
- Strings: double-quoted.
- Comments: `(* ... *)` is NOT supported in v0.1; reserved for M1.
- Operators and delimiters: `+ - * / ^ == -> ' ( ) [ ] { } ,`

## 2. Precedence and associativity

From loosest to tightest binding, matching the recursive-descent chain in the parser:

| Level | Forms | Associativity |
|---|---|---|
| rule | `a -> b` (Rule) | right |
| equal | `a == b` (Equal) | non-chaining |
| additive | `a + b`, `a - b` (Plus; subtraction is Plus with Times[-1, ...]) | left |
| multiplicative | `a * b`, `a / b`, implicit multiplication (Times; division is Times with Power[..., -1]) | left |
| unary | `-a` | prefix |
| power | `a ^ b` (Power) | right |
| postfix | `f[args]` application, `'` derivative primes | left |
| primary | number, symbol, string, `{...}` list, `(...)` grouping | |

Consequences fixed by fixtures: `-x^2` parses as `Times[-1, Power[x, 2]]`; `a/b/c` is `(a/b)/c`; `2^3^2` is `2^(3^2)`.

## 3. Implicit multiplication

Two adjacent primary-or-tighter expressions multiply: `2 x`, `c x[t]`, `3 Sin[t]`, `2(x+1)`. Implicit multiplication binds at the multiplicative level. A symbol immediately followed by `[` is function application, never multiplication.

## 4. Derivatives

Postfix primes on a symbol before application: `x'[t]` is `Derivative[1][x][t]`, `x''[t]` is `Derivative[2][x][t]`. Primes without application (`x'`) produce `Derivative[n][x]`.

## 5. Diagnostics and recovery

- Parse errors report a message and a byte-offset position. Error shape is stable: the kernel returns them in the result's error field, never a crash.
- The parser does not error-recover in v0.1: first error wins.
- Editor rule (normative for the app): incomplete or invalid 2D input stays editable in the MathLive field and is never sent to the kernel as if it were a valid expression; evaluation of invalid input surfaces the position-tagged error in the output cell.

## 6. Box conversion: MathLive LaTeX to linear syntax

The translator accepts the LaTeX MathLive emits for the supported box model and produces linear syntax per sections 1 to 4. Supported: numbers, identifiers, `+ - * / ^`, `\frac{a}{b}` to `(a)/(b)`, `a^{b}` to `a^(b)`, primes, `=` and `==` to `==`, `\sin` etc to `Sin[...]` for the known function set (Sin, Cos, Tan, Exp, Log, Sqrt, Abs), parenthesized application converted to square brackets for known functions, `\left(\right)` grouping, `\pi` to Pi, `e` context-dependent to E, lists `\{...\}`. Anything outside this set is a translation error shown to the user, not silently dropped.

## 7. Box conversion: expression to LaTeX

`openmat_core::to_latex` renders: fractions for negative powers and Divide shapes, `x^{n}` superscripts, `\sin`-style operators for known functions, primes or `^{(n)}` for Derivative, `\sqrt`, `\left( \right)` where precedence requires, proper unary minus. Output must be KaTeX-renderable.

## 8. Conformance fixtures

- specs/fixtures/grammar-v0.1.txt: `input || canonical InputForm || LaTeX` triples, one per line, `#` comments. The openmat-core conformance test parses the input, checks the canonical printed form, re-parses the printed form and checks it evaluates to the same expression (round trip up to evaluation: the printer may regroup, e.g. `a/b/c` prints as `a/(b*c)`; canonicalization makes them equal), and checks the LaTeX rendering.
- The MathLive translator fixtures live in app/src/mathlive/translator.test.ts and must cover every supported box form in section 6.
