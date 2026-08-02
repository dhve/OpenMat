# OpenMat Core Language & Symbolic Engine Spec

Scope: the kernel. Everything-is-an-expression, evaluation, pattern matching, the CAS layer, core data structures, functional programming constructs, and exact numerics. Numerics/data/visualization and the notebook front end are covered by other specs and are only mentioned here where they touch the kernel (e.g. packed arrays, precision tracking).

Comparison systems referenced throughout: **Mathics3** (mathics-core, Python, the most complete open WL clone), **Symja** (matheclipse/symja_android_library, Java, strongest CAS depth), **expreduce** (Go, small but clean pattern-matching core), **SymPy** (Python, mature CAS but not WL-shaped), **Maxima** (Lisp, oldest and most mathematically complete open CAS), **GiNaC** (C++ CAS library).

---

## 1. Wolfram Language Fundamentals

### 1.1 Everything-is-an-expression model

**What it is:** Every piece of WL code and data, `2+2`, `{1,2,3}`, `Plot[...]`, a symbol, a string, is the same tree structure: `Head[arg1, arg2, ...]`. `1+2` is really `Plus[1,2]`, a list is `List[1,2,3]`, even `Hold[x]` is an expression with head `Hold`. There is no separate "code" vs "data" type, no separate "AST" vs "value" distinction. This uniformity is what makes symbolic manipulation, metaprogramming, and pattern matching over code all the same operation.

**Core use cases:**
- A user writes `f[x_] := x^2 + 1` and expects `FullForm[f[a]]` to show `Plus[1, Power[a, 2]]`, and `Head[f[a]]` (post-evaluation) to be `Plus`.
- A user writes `expr = Hold[1 + 2]; Head[expr]` and gets `Hold`: they expect to be able to inspect and rewrite unevaluated code as data.
- A user does `Cases[bigExpr, _Integer, Infinity]` to pull every integer literal out of an arbitrarily nested expression tree, treating code as a tree to query.
- A user builds an expression programmatically: `Apply[Plus, {1,2,3}]` or `f @@ {a,b,c}` and expects it to construct `f[a,b,c]` and evaluate it exactly as if typed.
- A user expects `2` and `2.0` to be different expressions (`Integer` vs `Real` head in `FullForm`) even though they compare equal numerically.

**Implementation difficulty:** Easy to get a naive version working (a generic `Expr{Head, Args}` tree with a print/parse layer); hard to get it fast and memory-efficient. The tricky parts are: making atomic numeric expressions (machine reals, big integers) not pay full tree-node overhead, and making `FullForm`/box-form round-trip cleanly with the pretty-printed forms (infix `+`, superscript powers, etc.) which is a nontrivial parser/printer layered on top of a very small kernel grammar.

**Existing OSS building blocks:** Every clone (Mathics3, Symja, expreduce) implements this same tree exactly this way; it's the one design decision none of them deviate from. Mathics3's `mathics.core.expression.Expression` and Symja's `IExpr`/`IAST` hierarchy are directly studyable reference implementations. SymPy deviates (its `Basic` tree is similar but SymPy is not homoiconic with its own surface syntax the way WL is: SymPy expressions are Python objects, not a self-hosting language), so it's a weaker template for this specific piece.

**MVP priority:** Must-have. This is the foundation; nothing else in the spec works without it.

---

### 1.2 Evaluation semantics: the evaluation loop, up/down values

**What it is:** WL evaluates expressions by repeatedly rewriting: evaluate the head, evaluate each argument (leftmost-innermost, with exceptions for held arguments), then look for a rule that matches the whole expression and apply it, and repeat until the expression stops changing. Rules attached to a symbol come in two flavors relative to where that symbol sits in an expression: **downvalues** (`f[x_] := ...`, rules that fire when the symbol is the head, i.e. "look down into the expression") and **upvalues** (`x /: f[x_, y_] := ...`, rules attached to an argument symbol that fire when that symbol appears as a subexpression argument of some other head, i.e. "look up from the argument"). This lets a user extend how existing functions behave on their own new types without touching the existing function's own definition.

**Core use cases:**
- User defines `f[x_] := x^2` (a downvalue on `f`) and expects `f[3]` to evaluate to `9`, and `?f` / `DownValues[f]` to show the rule.
- User defines a custom type `Circle[r]` and wants `Circle[3] + Circle[4]` to combine radii; they attach an **upvalue** with `Circle /: Plus[Circle[r1_], Circle[r2_]] := Circle[r1+r2]` so `Plus`'s own generic downvalue-driven evaluation defers to the type-specific rule without modifying `Plus`.
- User expects assignment order matters for overlapping patterns: more specific rules (`f[0] := "zero"` defined after `f[x_] := "generic"`) still get tried first because WL auto-orders downvalues by specificity, not definition order.
- User writes recursive definitions (`fib[n_] := fib[n-1] + fib[n-2]; fib[0] = 0; fib[1] = 1`) and expects the evaluator to keep rewriting until a fixed point (all base cases hit), and to detect/report `$IterationLimit`/`$RecursionLimit` blowups rather than hang forever.
- User inspects `Trace[f[3]]` or `TracePrint` to see the exact sequence of rewrite steps the evaluator took: a debugging use case that requires the evaluator to be introspectable, not just a black box.

**Implementation difficulty:** Medium for a basic fixed-point rewriter with downvalues only; hard once you add upvalues, ownvalues, subvalues, correct evaluation-order edge cases (e.g. `N[Pi]` vs `Pi // N`), and full `Trace`-style introspection. The genuinely hard part is matching Wolfram's exact rule-ordering algorithm (most-specific-pattern-first heuristics) closely enough that real-world code depending on definition order behaves the same.

**Existing OSS building blocks:** Mathics3's `mathics.core.definitions.Definitions` class explicitly separates own/down/up/sub-values and its evaluator (`mathics.core.evaluation`) is a directly readable reference for the fixed-point loop, iteration limits, and `Trace`. Symja's `Rule`/`RulesData` per-symbol structure does the same split. expreduce's evaluator (`expreduce/evaluation.go`) is the smallest and most legible full implementation of this loop if you want to read one thing end to end.

**MVP priority:** Must-have (downvalues, the core loop, iteration limits). Upvalues/subvalues are should-have; real code uses them for operator overloading on custom types, but a credible MVP can ship with downvalues only and add up/subvalues in a fast-follow.

---

### 1.3 Hold attributes (HoldAll, HoldFirst, HoldRest, SequenceHold, etc.)

**What it is:** By default every argument to every function is evaluated before the function itself runs. Attributes attached to a symbol (`HoldFirst`, `HoldRest`, `HoldAll`, `HoldAllComplete`) suppress evaluation of some or all arguments before the function body sees them. This is what makes `Set`/`SetDelayed` (`=`/`:=`), `If`, `While`, `Module`, and `Function` possible at all: `x = 5` cannot evaluate `x` first (there'd be nothing to assign to), and `SetDelayed`'s right-hand side must stay unevaluated until call time.

**Core use cases:**
- `SetDelayed` (`:=`) has `HoldAll` on `SetDelayed` itself so `f[x_] := x^2` stores the unevaluated pattern and body rather than trying to evaluate `x^2` with `x` unbound at definition time.
- `If[cond, then, else]` has `HoldRest` so only `cond` is evaluated up front; `then`/`else` stay unevaluated until the branch is chosen: required for correctness (you can't evaluate a branch with side effects or infinite recursion before knowing you need it).
- A user writes a custom control-structure-like function, e.g. `SetAttributes[myWhile, HoldAll]; myWhile[cond_, body_] := ...`, and expects to be able to re-implement `While`-like constructs in library code, not just built-ins.
- A user calls `Hold[1+1]` and gets back the unevaluated expression `Hold[1 + 1]`, then strips the wrapper with `ReleaseHold` when ready: the general escape hatch for holding anything on demand without declaring an attribute.
- `Attributes[f]` and `SetAttributes[f, HoldFirst]` are user-facing introspection/control: a user debugging why their function evaluates arguments "too early" or "too late" needs to read and set these directly.

**Implementation difficulty:** Medium. Conceptually simple (a per-symbol bitset of attributes checked before recursing into arguments), but it interacts with almost everything else: pattern matching against held expressions, `Evaluate` as an escape hatch inside held arguments, `HoldAllComplete` also suppressing `Sequence` splicing and `Unevaluated` wrapping. Getting the interaction between attributes and pattern matching exactly right (e.g. `HoldPattern` in rules) is the fiddly part, not the base mechanism.

**Existing OSS building blocks:** All three clones implement attribute-gated evaluation; Mathics3's `mathics.core.attributes` module plus checks in the evaluator is the clearest reference. This is a place where SymPy is not useful at all: Python's own evaluation model doesn't have an analogous concept (closest is `sympy.Lambda`/lazy evaluation, which is much weaker).

**MVP priority:** Must-have. Without `HoldAll`/`HoldFirst` you cannot correctly implement assignment, control flow, or scoping: this is load-bearing for section 5 as well.

---

### 1.4 Symbols and contexts (namespaces)

**What it is:** A WL symbol's full identity is `context\`name` (e.g. `System\`Plus`, `Global\`x`, `MyPackage\`PrivateHelper\``). Contexts are WL's namespace mechanism: they prevent name collisions between packages, control what's exported (`Begin`/`BeginPackage`/`End`/`EndPackage`, public vs `` `Private` `` contexts), and `$Context`/`$ContextPath` govern how unqualified names resolve.

**Core use cases:**
- A user loads two packages that both define a function called `Transform`; because each lives in its own context (`` PkgA`Transform `` vs `` PkgB`Transform ``), both coexist and the user disambiguates with the fully qualified name only when there's a genuine conflict on `$ContextPath`.
- A package author writes `BeginPackage["MyPkg`"]; publicFn::usage = "..."; Begin["`Private`"]; publicFn[x_] := helper[x]; helper[x_] := x^2; End[]; EndPackage[]` and expects `helper` to not leak into the global namespace, while `publicFn` does.
- A user types `Context[x]` or `Names["Global`*"]` to inspect what symbols exist and where they live, for debugging namespace pollution.
- A user expects symbols to be effectively global mutable variables scoped by context, not lexically scoped by default: `x = 5` in one cell affects `x` everywhere else that resolves to the same context, which is the source of a lot of beginner confusion and a real semantic to replicate faithfully (as opposed to "fixing" it into lexical scoping, which would break compatibility).

**Implementation difficulty:** Medium. The symbol table itself is a straightforward `(context, name) -> Symbol` hash map. The difficulty is entirely in getting `$Context`/`$ContextPath` resolution order, `Begin`/`BeginPackage` nesting, and shadowing warnings (`Symbol::shdw`) to match real behavior, since package authors depend on the exact resolution rules.

**Existing OSS building blocks:** Mathics3 implements contexts closely (`System\``, `Global\``, per-package contexts) and is the most faithful reference. Symja instead uses a flat/simplified namespace model in most usage (closer to a single global symbol table with some package support): less useful as a reference for this specific piece. This is a place where the OSS gap is real: none of the clones has heavily battle-tested multi-package context resolution because most usage of all three is single-notebook, single-context scripts.

**MVP priority:** Should-have. A flat global namespace (single implicit `Global\`` context plus a `System\`` for built-ins) is enough for an MVP and is what most interactive usage needs; full `BeginPackage`/`Private`-context package authoring can follow once there's a package ecosystem to serve.

---

## 2. Pattern Matching and Term Rewriting

This is the heart of the language: WL's own documentation says so, and every other section of this spec (function definition, `Simplify`, `Solve`'s internals, list processing) is pattern matching underneath. Get this right and a large fraction of the rest becomes "write more rules."

### 2.1 Blank patterns, named patterns, pattern tests, conditions

**What it is:** `_` (`Blank[]`) matches anything; `_h` matches anything with head `h`; `__` (`BlankSequence`) matches one or more expressions in a sequence position; `___` (`BlankNullSequence`) matches zero or more; `x_` binds the match to `x` for use on the right-hand side; `x_?test` (`PatternTest`) requires `test[x]` to return `True`; `x_ /; cond` (`Condition`) requires `cond` (which can reference `x`) to be `True`. These compose: `x_Integer?Positive` matches only positive integers and binds `x`.

**Core use cases:**
- `f[x_] := x^2`: the single most common pattern in the language, "match anything, bind it, use it."
- `f[x_, y_, z_] := ...` vs `f[x_, rest__] := ...`: fixed arity vs variadic ("head, then the rest") function heads, used constantly for recursive list processing (`first[{x_, ___}] := x`).
- `f[x_?NumericQ] := ...` to restrict a rule to only fire on numeric input, leaving symbolic input for a fallback/generic rule: the standard idiom for "compute numerically if you can, stay symbolic otherwise."
- `f[x_ /; x > 0] := ...` for conditions that need more than a single-argument predicate, e.g. `f[x_, y_ /; y > x] := ...` referencing an earlier-bound variable.
- `g[x_Integer]` vs `g[x_Real]` vs `g[x_List]` as informal multiple dispatch on runtime type: a user expects to define several `g[...]` overloads with different head-constrained patterns and have the right one selected automatically by specificity.

**Implementation difficulty:** Medium for `_`/`__`/`___` with head constraints and named bindings; hard for full correctness including backtracking search across multiple `__`/`___` in one expression (matching `f[a__, b__]` against `f[1,2,3]` requires trying every split point until one lets the rest of the pattern succeed) and interaction with `Orderless`/`Flat` attributed heads (matching against `Plus`/`Times` with commutative, associative structure is a much harder combinatorial search: this is the single hardest piece of the pattern matcher).

**Existing OSS building blocks:** This is the best-covered area in existing OSS. Symja's pattern matcher (`org.matheclipse.core.patternmatching`) handles orderless/flat matching for `Plus`/`Times` and is the most mature reference implementation available. Mathics3's `mathics.core.pattern` module is a clean, readable (if slower) implementation of the same ideas in Python. expreduce's matcher is smaller but its `matcher.go` is a good from-scratch read for the backtracking-with-continuations design pattern (it uses explicit match generators/iterators rather than ad hoc recursion, which is a cleaner architecture to copy than either of the other two). SymPy's `Wild` symbols are a much weaker analog, with no orderless/flat matching and no sequence patterns, so not a good template for this piece specifically.

**MVP priority:** Must-have, including `Orderless`/`Flat` matching for at least `Plus`/`Times`/`And`/`Or`: a huge fraction of realistic user code (anything doing algebra) silently depends on `x + y` matching the same as `y + x` in a rule, so skipping this makes the MVP feel broken rather than incomplete.

---

### 2.2 Rule application: Replace, ReplaceAll (`/.`), ReplaceRepeated (`//.`)

**What it is:** A `Rule` (`lhs -> rhs`) or `RuleDelayed` (`lhs :> rhs`) pairs a pattern with a replacement. `Replace[expr, rule]` tries the rule once at the top level (or at specified levels); `ReplaceAll[expr, rule]` (infix `/.`) applies the rule everywhere throughout the expression tree, once per subexpression; `ReplaceRepeated[expr, rule]` (infix `//.`) applies `ReplaceAll` repeatedly until the expression stops changing (a fixed point), which is how ad hoc term rewriting/simplification scripts are written without defining a persistent function.

**Core use cases:**
- `{a, b, c} /. b -> x`: the single most common one-off substitution idiom in the language, used interactively constantly.
- `expr /. {x -> 1, y -> 2}` applying multiple rules in one pass, first-match-wins per subexpression, and `expr /. {rule1, rule2} /. {rule3}` chaining separate rounds.
- `expr //. {Power[a_, 2] -> a*a}` expanding all squares into products repeatedly until no `Power[_,2]` remains anywhere in the tree, a canonical "flatten this structure by rewriting" use case.
- `Replace[expr, pattern -> replacement, {2}]` applying a rule only at a specific tree depth (level spec), used for "only touch the immediate children of the immediate children" transformations.
- `Cases[expr, pattern -> transform, Infinity]` and `Position[expr, pattern]`: the query-and-extract siblings of replace, used to find and pull out matching subexpressions rather than rewrite them in place.

**Implementation difficulty:** Medium once 2.1's matcher exists: `Replace`/`ReplaceAll`/`ReplaceRepeated` are mostly tree-traversal drivers around the core matcher, plus level-spec parsing (`{n}`, `{n1,n2}`, `Infinity`, `Heads->True`). The one real subtlety is `ReplaceRepeated`'s fixed-point / non-termination detection and matching Wolfram's exact traversal order (WL replaces bottom-up per subexpression in one `ReplaceAll` pass, which matters for rules whose LHS and RHS overlap in structure).

**Existing OSS building blocks:** Directly implemented in all three clones as thin layers over their respective matchers; Mathics3's `mathics.builtin.patterns` and Symja's `ReplaceAll`/`ReplaceRepeated` builtins are both good references. This is low-risk relative to 2.1: once you have the matcher, this layer is mechanical.

**MVP priority:** Must-have. `/.` and `//.` are used as often as `=` in real WL code; shipping the matcher without these would be pointless.

---

### 2.3 Function definition via patterns (`Set`, `SetDelayed`, multi-clause definitions, default values)

**What it is:** `f[x_] := body` is not a special "function definition" syntax distinct from pattern matching: it's exactly a `RuleDelayed` stored as a downvalue on `f`, and calling `f[3]` is exactly `ReplaceAll`-style rule application at the evaluator's rewrite step. Multiple `SetDelayed`s on the same head accumulate into an ordered list of rules (auto-sorted by specificity), giving pattern-matching-based multiple dispatch/overloading for free, plus optional patterns (`x_:default`) and named-argument-like patterns for default values.

**Core use cases:**
- `f[x_] := x^2` then later `f[0] = "special case"`: user expects both to coexist and the more specific one (`f[0]`, a literal) to be tried before the general pattern, regardless of definition order.
- `f[x_, y_:10] := x + y`: optional argument with a default, so `f[5]` gives `15` and `f[5, 2]` gives `7`, the standard way to fake keyword-argument-with-default ergonomics.
- `f[x_Integer] := ...` and `f[x_Real] := ...` and `f[x_List] := ...` defined separately, dispatching on runtime type as if it were operator/method overloading: this is the idiomatic WL substitute for a type system with methods.
- Recursive definitions with explicit base cases: `fact[0] = 1; fact[n_Integer?Positive] := n*fact[n-1]`: the user expects the literal `fact[0]` rule to short-circuit recursion, not get shadowed by the general pattern.
- `Clear[f]` / `f[x_] =.` to remove a specific rule or all rules on a symbol, and `?f`/`Definition[f]` to list every rule currently attached: introspection users expect to have on any function they or a package defined, built-in or not.

**Implementation difficulty:** Medium. Depends entirely on 1.2 (down/upvalues) and 2.1/2.2 (matching and replacement) already existing: at that point `SetDelayed` is "store this pattern+body pair in the symbol's rule list, keep the list sorted by specificity." The specificity-ordering heuristic (literal values before typed patterns before untyped `_`, more constraints before fewer) is the one piece that needs careful, faithful replication since real code relies on the exact tie-breaking order.

**Existing OSS building blocks:** Universally implemented (it's not optional: the clones can't run any test suite without it). Mathics3's rule-sorting logic and Symja's `PatternMatcherAndEvaluator` ordering are both worth reading; expreduce's simpler linear-scan-with-specificity-score approach is easier to port as a first cut and matches real behavior closely enough for an MVP.

**MVP priority:** Must-have, obviously: this is how users write any nontrivial program.

---

## 3. Symbolic Mathematics (CAS)

### 3.1 Algebraic simplification: Simplify, FullSimplify

**What it is:** `Simplify[expr]` searches over a fixed set of transformation rules (combined with `Together`, `Cancel`, `PowerExpand`, trig identities, etc.) for the "simplest" equivalent form by a complexity metric (default: leaf count), optionally under `Assumptions`. `FullSimplify` extends the rule set to special functions, more aggressive trig/log identities, and can call `Integrate`/other solvers as sub-steps, at much higher cost.

**Core use cases:**
- `Simplify[(x^2 - 1)/(x - 1)]` returning `x + 1`: cancel-and-reduce, the most common single use.
- `Simplify[Sin[x]^2 + Cos[x]^2]` returning `1`: trig identity simplification.
- `Simplify[expr, x > 0]`: assumption-conditioned simplification, e.g. `Simplify[Sqrt[x^2], x > 0]` giving `x` instead of `Abs[x]`.
- `FullSimplify[Gamma[x+1]/Gamma[x]]` returning `x`: special-function identity simplification that plain `Simplify` won't attempt.
- A user pastes a long, ugly expression from an intermediate computation and just runs `Simplify[%]` expecting "make this readable," a fuzzy, satisficing goal rather than a well-specified transformation: this is the use case that makes the feature hard to spec precisely and hard to fully clone.

**Implementation difficulty:** Research-grade for full fidelity, medium for a useful subset. Simplification is not one algorithm: it's a search over a large, hand-tuned rule library with a cost heuristic, refined over decades in Mathematica. A subset covering polynomial cancellation, basic trig/log/exponential identities, and `PowerExpand`-style rewriting is medium difficulty and covers most everyday use; matching Mathematica's exact output form (which of several equally-simple forms it picks) is not achievable and shouldn't be a goal: "an equally valid simpler form" should be the bar, not "the identical form."

**Existing OSS building blocks:** SymPy's `simplify()` is the strongest open reference for the search-with-cost-heuristic architecture (it also tries several strategies and picks the shortest result) even though it isn't WL-shaped on the surface. Symja has a `Simplify`/`FullSimplify` implementation directly modeled on Mathematica's, and is the closer template since it already speaks WL semantics. Maxima's `radcan`/`trigsimp`/`ratsimp` family is the most mathematically battle-tested set of individual simplification algorithms to draw on even though its architecture (many named specific simplifiers rather than one adaptive `Simplify`) differs from WL's.

**MVP priority:** Should-have for a basic `Simplify` (polynomial + trig/log/exp identities, assumption-aware); later for `FullSimplify`'s special-function and solver-backed tiers. A CAS that can't cancel `(x^2-1)/(x-1)` will feel broken, but the long tail of special-function identities is not blocking for credibility.

---

### 3.2 Calculus: D, Integrate, Limit, Series

**What it is:** `D` is symbolic differentiation (a pure syntax-directed rewrite, mechanical and total for elementary functions). `Integrate` is symbolic (indefinite/definite) integration, which is not total: no algorithm finds a closed form for every integrable-in-principle expression, and some integrals genuinely have no elementary closed form. `Limit` computes symbolic limits (including one-sided, at infinity, indeterminate forms via series expansion or L'Hopital-style rules). `Series` produces truncated power/Laurent series expansions around a point, returning a `SeriesData` object that supports further arithmetic.

**Core use cases:**
- `D[x^3 + Sin[x], x]` → `3x^2 + Cos[x]`: bread-and-butter differentiation, expected to be instant and always succeed for elementary functions, including multivariate (`D[f[x,y], x, y]`) and implicit-form derivatives.
- `Integrate[x^2, x]` → `x^3/3`, and `Integrate[Exp[-x^2], {x, -Infinity, Infinity}]` → `Sqrt[Pi]`: indefinite and definite integration; the definite case often requires different machinery (residues, special-function results) than finding an antiderivative.
- `Limit[Sin[x]/x, x -> 0]` → `1`: indeterminate-form limits, including at infinity and one-sided (`Limit[1/x, x -> 0, Direction -> "FromAbove"]`).
- `Series[Exp[x], {x, 0, 5}]` → truncated Taylor series with a `O[x]^6` error term, and a user expects `Series[f,{x,0,3}] + Series[g,{x,0,3}]` to arithmetic-compose correctly, order-tracking through the operation.
- `DSolve[y'[x] == y[x], y[x], x]` (see 3.3) as the composite use case that pulls together `D`, `Integrate`, and pattern-matched ODE-family recognition all at once.

**Implementation difficulty:** `D` is easy (a total, purely syntax-directed table of rewrite rules: every clone has this and it's essentially finished technology). `Limit` is medium-to-hard (needs series expansion machinery plus special-casing indeterminate forms). `Integrate` is hard-to-research-grade for a general solver: the Risch algorithm is decidable and complete for purely elementary/rational-exponential integrands but is a serious, rarely-fully-implemented algorithm on its own (branches for the transcendental and, worse, the algebraic case are notoriously hard); rule-based systems (Rubi, 6000+ hand-curated rules) get broad practical coverage without algorithmic completeness. `Series` is medium: mechanical once you have `D` and Taylor-coefficient bookkeeping, but truncation-order arithmetic (knowing how many terms of a product you can trust) has real edge cases.

**Existing OSS building blocks:** For `Integrate`, **Rubi** (rulebasedintegration.org, 6000+ rules, originally Mathematica-native) is the single most valuable existing asset for an OSS clone: it's already rule-based and pattern-driven, the same paradigm this whole spec is built on, and SymPy has already partially ported it (`sympy/integrals/rubi`) which is worth studying for the porting approach and gotchas. `SymbolicIntegration.jl` (2025) is a newer hybrid combining a real Risch implementation with ~3400 Rubi-derived rules and is the most modern reference for how to combine both strategies. Maxima ships a mature, independent Risch-family integrator (`integrate`) built over decades: a good second reference/cross-check for correctness. For `D`/`Series`/`Limit`, SymPy's `diff`/`series`/`limit` (the `Gruntz` algorithm for limits specifically) are solid, well-tested references; Symja implements all four directly in WL-shaped form and is the best single template to port from since output conventions already match.

**MVP priority:** `D`: must-have (it's cheap and everything above it depends on it). `Series`: should-have. `Limit`: should-have (basic cases; full Gruntz-algorithm generality later). `Integrate`: should-have for a rule-based subset covering common textbook forms (polynomials, basic trig/exp/log, substitution-friendly forms via a ported Rubi subset): treat a *general, complete* symbolic integrator as later/research-grade and out of MVP scope; users should get "no closed form found" gracefully rather than the MVP silently being wrong.

---

### 3.3 Differential equations: DSolve

**What it is:** `DSolve` finds closed-form (exact, symbolic) solutions to ordinary and some partial differential equations by pattern-matching the ODE against known solvable families (separable, linear first-order, constant-coefficient linear higher-order, Bernoulli, exact equations, etc.) and applying the family-specific solution method, falling back through the list until one matches or none do.

**Core use cases:**
- `DSolve[y'[x] == y[x], y[x], x]` → `y[x] -> C[1] E^x`, the canonical first-order linear case, with an undetermined constant the user is expected to pin down from an initial condition next.
- `DSolve[{y''[x] + y[x] == 0, y[0] == 0, y'[0] == 1}, y[x], x]` → `y[x] -> Sin[x]`: initial-value problems, where a user expects the constants to already be solved for when boundary conditions are supplied.
- `DSolve[y'[x] == y[x]^2, y[x], x]`: nonlinear but separable, exercising the "try known solvable families in order" dispatch rather than a single universal algorithm.
- A user expects a graceful, honest failure (returning the input unevaluated, or a message) for a DE that has no elementary closed form, rather than a wrong or nonsensical answer.
- `DSolve[...]` producing solutions in terms of special functions (Bessel, hypergeometric) for equations in those recognized families, tying directly into 3.7.

**Implementation difficulty:** Research-grade for broad coverage; medium for a useful common-case subset. Like `Integrate`, this is a "library of solvable special cases plus a dispatcher" problem, not one clean algorithm: coverage is a long tail with diminishing returns per family added.

**Existing OSS building blocks:** SymPy's `dsolve` has one of the more complete open family-classifier-and-solver setups (`sympy/solvers/ode`) and is the strongest reference available. Maxima's `ode2`/`desolve` are mature and independently useful for cross-checking. No clone (Mathics3/Symja/expreduce) has DSolve depth close to real Mathematica; this is a genuine gap across the whole OSS landscape, not just something OpenMat would be behind on relative to existing clones.

**MVP priority:** Later. First-order linear/separable and constant-coefficient linear ODEs (a small, well-defined family list) could be should-have if calculus credibility matters early, but full `DSolve` breadth is not MVP-blocking: it's a deep, open-ended feature users will forgive being partial far more readily than they'll forgive `Solve` or `D` being partial.

---

### 3.4 Equation solving: Solve, Reduce, NSolve

**What it is:** `Solve[eqns, vars]` finds symbolic solutions to systems of equations (polynomial systems via Gröbner bases, and pattern-recognized special forms elsewhere), returning a list of replacement rules. `Reduce` is strictly more general and more honest: it handles equations *and* inequalities *and* mixed real/complex domains and returns a fully quantifier-reduced logical formula describing the entire solution set (including case splits, e.g. "if `a != 0` then ... else ..."), rather than just a rule list: `Solve` is essentially a friendlier front end over a subset of what `Reduce` can do. `NSolve` gives numeric (not exact-symbolic) solutions, typically via `Solve` symbolically failing over to root-finding, or by numeric polynomial root isolation directly.

**Core use cases:**
- `Solve[x^2 - 5x + 6 == 0, x]` → `{{x -> 2}, {x -> 3}}`: the everyday single-variable polynomial case, expected to give every root, exactly, as a list of rules ready to `/.`-substitute back in.
- `Solve[{x + y == 3, x - y == 1}, {x, y}]` → linear system solving, expected to be fast and exact via linear algebra rather than general polynomial machinery.
- `Reduce[x^2 + y^2 == 1 && x > 0, {x, y}]`: a case where the user explicitly wants the full parametrized/quantified description of a solution set (here, real algebraic conditions on `x` and `y` jointly), not just "a" solution.
- `Solve[eqn, x]` on a quintic or higher polynomial with no radical solution: a user expects either `Root[...]` objects (implicit algebraic-number representation) or an honest statement that no closed form exists, not silence or an incorrect numeric-looking answer.
- `NSolve[x^5 - x - 1 == 0, x]` → numeric approximations of all five roots (including complex ones), used when the user wants numbers, not exact radicals/`Root` objects.

**Implementation difficulty:** Medium for single-variable polynomials and linear systems (well-known closed-form and linear-algebra algorithms). Hard for general polynomial systems, which require Gröbner basis computation (Buchberger's algorithm or F4/F5): algorithmically well-understood but a serious implementation project with real performance engineering involved (term orders, S-polynomial reduction efficiency). Research-grade for full `Reduce` generality (real quantifier elimination via cylindrical algebraic decomposition is one of the hardest widely-implemented pieces of any CAS).

**Existing OSS building blocks:** SymPy's `solveset`/`solve` and `sympy.polys` (which includes a Gröbner basis implementation) are the strongest open reference for the polynomial-system-solving core. For Gröbner bases specifically, standalone libraries exist outside the Python/WL world too (e.g. Singular, Macaulay2) worth studying for algorithm choice even if not directly portable. `Reduce`-level real quantifier elimination has essentially no complete open implementation anywhere accessible: this is a place where even Mathematica's own implementation is one of its most differentiated, hard-won pieces of technology; QEPCAD is the closest independent open research implementation of CAD-based QE and is worth knowing about but is not production-grade tooling to build on.

**MVP priority:** `Solve` for single-variable polynomials and linear systems: must-have. `Solve` for general polynomial systems via Gröbner bases: should-have. `NSolve`: should-have (numeric root-finding is much easier than exact and gives a fallback when exact `Solve` can't find a closed form). `Reduce`, especially the inequality/quantifier-elimination generality, is later; flag it clearly as out of MVP scope, since a partial `Reduce` that silently mishandles quantifiers is worse than not shipping it.

---

### 3.5 Polynomial algebra

**What it is:** The workhorse layer underneath most of section 3: `Expand`, `Factor`, `Together`, `Apart`, `Cancel`, `PolynomialGCD`, `PolynomialQuotient`/`Remainder`, `Collect`, `Coefficient`, `Resultant`, `Discriminant`. Multivariate polynomial arithmetic with exact (rational/integer) coefficients, plus factoring over the integers/rationals (and optionally algebraic extensions).

**Core use cases:**
- `Expand[(x+y)^3]` → fully distributed polynomial; `Factor[x^2 - y^2]` → `(x-y)(x+y)`: the two most common, most load-bearing operations, expected to be instant even on moderately large expressions.
- `Together[1/x + 1/y]` → single fraction `(x+y)/(x y)`; `Apart[...]` the inverse, partial-fraction decomposition: used constantly as a pre-processing step before `Simplify`, `Integrate`, or `Limit`.
- `PolynomialGCD[x^2-1, x^2-x-2]` → `x+1`: used both directly and internally by `Cancel`/`Together`.
- `Collect[expr, x]` → group an expression as a polynomial in `x` with the other variables' contributions as coefficients: used to read off structure from a messy expanded expression.
- `Factor[x^4 - 1]` needing to factor into irreducibles over the rationals (`(x-1)(x+1)(x^2+1)`), which requires real factoring algorithms (not just difference-of-squares pattern matching) to be correct in general.

**Implementation difficulty:** Medium for `Expand`/`Collect`/`Together`/GCD (well-understood, mechanical algorithms; multivariate GCD via a good representation is the main engineering task). Hard for general multivariate polynomial factoring over the integers, which requires real number-theoretic algorithms (Berlekamp/Zassenhaus or similar for univariate, then lifting to multivariate). This is well-trodden classical CAS ground but not trivial to implement correctly and efficiently.

**Existing OSS building blocks:** This is the best-covered area of the entire spec in open source. SymPy's `sympy.polys` module is a genuinely complete, well-tested, actively maintained polynomial arithmetic and factorization library and is directly reusable as a reference implementation (algorithm-for-algorithm) even though its Python API isn't WL-shaped. FLINT (C library, fast, used by SageMath) is the performance-oriented reference if speed matters more than readability. Symja wraps a Java library (JAS - Java Algebra System) for this layer rather than reimplementing it, which is itself a useful precedent: this may be the one area where OpenMat should bind an existing battle-tested library rather than write a new implementation.

**MVP priority:** Must-have (`Expand`, `Factor` for univariate and simple multivariate, `Together`, `Cancel`, `PolynomialGCD`, `Collect`). Everything else in section 3 leans on this layer being solid, so it should be prioritized ahead of, not alongside, `Simplify`/`Integrate`/`Solve`.

---

### 3.6 Linear algebra over symbolic entries

**What it is:** Matrix and vector operations (`Det`, `Inverse`, `Eigenvalues`, `Eigenvectors`, `MatrixRank`, `NullSpace`, `RowReduce`, `LinearSolve`) that work correctly with symbolic (not just numeric) entries: e.g. computing the exact eigenvalues of a 3x3 matrix with symbolic parameters as entries, or row-reducing a matrix of expressions. This is distinct from the numerics agent's area, which covers numeric linear algebra (LU/QR decompositions, large-scale numeric eigenvalue algorithms, performance): this section is specifically about exactness and symbolic entries.

**Core use cases:**
- `Det[{{a, b}, {c, d}}]` → `a d - b c`: symbolic determinant, expected to be an exact expanded polynomial in the entries, not a numeric approximation.
- `Inverse[{{1, 2}, {3, 4}}]` → exact rational-entry inverse matrix (not floating point), and `Inverse[{{a,b},{c,d}}]` → the general symbolic formula with `1/(a d - b c)` factored out.
- `Eigenvalues[{{2, 0}, {0, 3}}]` → `{3, 2}` exactly for nice matrices; `Eigenvalues[symbolicMatrix]` expected to return exact algebraic expressions (possibly `Root` objects) for the characteristic polynomial's roots, falling back gracefully to numeric-only when no closed form exists (degree 5+).
- `RowReduce[matrix]` / `LinearSolve[A, b]` on a matrix with exact rational or symbolic entries, expected to give exact results usable in further exact computation, contrasted with the numeric agent's floating-point-focused `LinearSolve`.
- `NullSpace`/`MatrixRank` on a symbolic-parameter matrix used to determine, exactly, for which parameter values a system is singular: a genuinely CAS-specific (not numeric-linear-algebra) use case.

**Implementation difficulty:** Medium. The linear algebra algorithms themselves (Gaussian elimination, cofactor/Bareiss determinant expansion, characteristic polynomial via Faddeev-LeVerrier) are standard and not hard to implement correctly. The real difficulty is that every arithmetic operation inside them needs to run over the exact/symbolic number tower (section 6) rather than floats, and needs `Simplify` calls at the right points to keep intermediate symbolic expressions from exploding in size (fraction-free/Bareiss-style elimination exists specifically to control this blowup and is worth using over naive Gaussian elimination).

**Existing OSS building blocks:** SymPy's `sympy.matrices` handles symbolic entries throughout and is a strong, directly studyable reference including its use of Bareiss elimination for exactness-preserving determinant/inverse computation. Symja's matrix operations are directly WL-shaped and a good porting template. This is a comparatively low-risk area since the math is classical and well covered by both references.

**MVP priority:** Should-have. `Det`, `Inverse`, `LinearSolve`, `RowReduce` for exact/symbolic small-to-medium matrices should ship reasonably early since linear algebra underpins solving linear systems (3.4) and other math areas; `Eigenvalues`/`Eigenvectors` for the general symbolic case (versus small/nice matrices) can be later given it inherits all of polynomial root-finding's difficulty (3.4).

---

### 3.7 Special functions

**What it is:** The large catalog of named mathematical functions beyond elementary ones: Gamma, Beta, Bessel (`BesselJ`, `BesselY`, etc.), error functions (`Erf`, `Erfc`), orthogonal polynomials (`LegendreP`, `ChebyshevT`, `HermiteH`), hypergeometric functions (`Hypergeometric2F1`), elliptic integrals, zeta/polylog, each with symbolic identities, derivatives, series expansions, and special-value evaluation (`Gamma[1/2] == Sqrt[Pi]`), plus arbitrary-precision numeric evaluation.

**Core use cases:**
- `Gamma[5]` → `24` (exact integer for integer input), `Gamma[1/2]` → `Sqrt[Pi]` (exact closed form at known special points), `Gamma[1.5]` → high-precision numeric value: a user expects the same function to give exact results where a closed form is known and fall back to numeric evaluation otherwise.
- `D[BesselJ[n, x], x]` returning the correct derivative identity in terms of other Bessel functions: special functions need to participate fully in calculus (section 3.2), not just be numerically evaluable leaves.
- `Series[Erf[x], {x, 0, 5}]`: series expansion of a special function around a point, used in asymptotic analysis.
- A physics/engineering user evaluates `BesselJ[0, 2.5]` or `LegendreP[3, x]` numerically to arbitrary precision as part of a larger numeric pipeline: correctness and precision-tracking (section 6.3) matter as much as symbolic identity coverage here.
- `FunctionExpand[Gamma[n+1]]` for integer `n` rewriting to `n!`, and similar targeted identity rewrites a user invokes explicitly when `Simplify`/`FullSimplify` don't go far enough on their own.

**Implementation difficulty:** Hard, mainly because of breadth, not depth per function. Each special function individually is well-documented classical mathematics (Abramowitz & Stegun / DLMF have the formulas), so implementing any single one (symbolic identities + numeric evaluation algorithm, e.g. continued fractions or series with the right convergence region) is medium difficulty; the difficulty is that there are dozens of these, each needing its own derivative rules, series rules, special-value table, and numeric evaluation algorithm, so total effort scales with catalog size.

**Existing OSS building blocks:** mpmath (which underlies SymPy's numeric evaluation) is the strongest open reference for arbitrary-precision numeric special-function evaluation: it implements a very wide catalog correctly and is directly usable as a numeric backend rather than needing to be re-derived. SymPy's `sympy.functions.special` module covers symbolic identities/derivatives for a good subset. The DLMF (Digital Library of Mathematical Functions, NIST, free/open reference) is the authoritative formula source to build the identity tables from regardless of which code is studied.

**MVP priority:** Should-have for a small core set (Gamma, Erf, Bessel J/Y, a few orthogonal polynomial families) since they show up constantly in calculus/probability contexts; later for full catalog breadth (elliptic integrals, generalized hypergeometric, number-theoretic functions like `MoebiusMu`) since coverage gaps here degrade gracefully (function stays symbolic/unevaluated) rather than giving wrong answers.

---

### 3.8 Assumptions system

**What it is:** `Assumptions` (used via the `Assumptions` option to many functions, or globally via `$Assumptions`, or locally via `Assuming[...]`) lets a user assert facts about symbols (`x > 0`, `n ∈ Integers`, `x ∈ Reals`) that `Simplify`, `Integrate`, `Limit`, `Refine`, and `Reduce` use to pick correct branches and simplifications that wouldn't be valid unconditionally (e.g. `Sqrt[x^2] -> x` only holds if `x >= 0`).

**Core use cases:**
- `Simplify[Sqrt[x^2], x > 0]` → `x` (vs. the unconditional, correct-in-general `Abs[x]`): the canonical example, showing up in nearly every tutorial on the feature.
- `Assuming[x > 0, Integrate[Exp[-x] , {x, 0, Infinity}]]` (or similar): an assumption scoping a whole block of computation rather than being passed function-by-function, so several calls share one assumption context.
- `Refine[Abs[x], x > 0]` → `x`: `Refine` specifically as the "apply assumptions and simplify, but only using the given fact, don't search further" lighter-weight sibling of `Simplify[..., assumptions]`.
- `Reduce[x^2 == 4 && x ∈ Integers, x]`: domain assumptions (`Integers`, `Reals`, `Complexes`) feeding directly into `Reduce`'s branch selection.
- A user asserts `n \[Element] Integers && n >= 0` and expects `Sin[n Pi]` to simplify to `0`: assumptions composing with special-value/periodicity identities, not just sign/domain gating.

**Implementation difficulty:** Medium for basic sign/domain assumptions feeding a handful of consuming functions (`Simplify`, `PowerExpand`, `Refine`); hard for assumptions to compose correctly and consistently across every consuming function without contradictions or silent incorrect simplification, since the "database" of asserted facts needs a real (if small) inference layer (e.g. `x > 0` should imply `x != 0` should imply certain sqrt/log branch choices without each being separately special-cased).

**Existing OSS building blocks:** SymPy's assumptions module (`sympy.assumptions`, the `Q` predicate system with `ask()`/`refine()`) is the strongest and most directly comparable open implementation: it's a genuinely general-purpose logical inference system over predicates like `Q.positive`, `Q.integer`, `Q.real`, with a SAT-based backend for consistency checking, and its `refine()` function is architecturally the closest open analog to WL's `Refine`. This is a good area to study and adapt from SymPy fairly directly, more so than from either Mathics3 or Symja, whose assumptions support is comparatively thin.

**MVP priority:** Should-have (basic positivity/realness/integer-domain assumptions feeding `Simplify` and `Refine`): not needed for a first cut where `Simplify` handles only unconditionally-true rewrites, but needed soon after since a meaningful fraction of real symbolic math work depends on domain assumptions to get non-trivial-but-correct simplifications at all.

---

## 4. Core Data Structures

### 4.1 List

**What it is:** `List[a,b,c]` (`{a,b,c}`) is WL's universal ordered-collection type: arrays, tuples, sets, matrices (lists of lists), and sequences are all just lists, with no separate array/tuple/vector types at the language level (though `Association`/`SparseArray`/packed arrays exist as optimized or semantically distinct alternatives layered on top).

**Core use cases:**
- `{1, 2, 3}` literal construction; `list[[2]]` (`Part`) indexing (1-based, and negative indices count from the end); `list[[2;;4]]` span slicing.
- `Length[list]`, `Append`/`Prepend`, `Join[l1, l2]`, `Flatten[nestedList]`, `Sort[list]`, `Union`/`Intersection`/`Complement` (set-like operations on lists), `Reverse`.
- `{{1,2},{3,4}}` used directly as a 2x2 matrix: a user expects `Dot` (`.`), `Transpose`, `Det`, etc. to just work on nested lists without a separate "matrix" constructor.
- `Table[i^2, {i, 1, 10}]` generating a list programmatically (ties directly to section 5.4): this is how most lists in real code actually get built, not literal typing.
- Pattern matching against list structure directly: `{x_, y_} := ...` or `{first_, rest___} := ...` for head/tail-style recursive list processing, tying section 4.1 straight back into section 2.

**Implementation difficulty:** Easy. A list is the same generic `Expression` tree node from section 1.1 with head `List`; the only real design decision is the underlying storage (growable array vs persistent/functional structure) and how `Part`/slicing map onto it, plus getting `Flat`/`Orderless`-style attribute behavior irrelevant here (List has neither) out of the way.

**Existing OSS building blocks:** Every clone implements this identically and trivially, since it's the same generic expression node. Nothing special to borrow beyond what section 1.1 already covers.

**MVP priority:** Must-have, day one.

---

### 4.2 Association

**What it is:** `Association[k1 -> v1, k2 -> v2, ...]` (`<|k1 -> v1, k2 -> v2|>`) is WL's ordered hash map / dictionary type: keys can be any expression (not just strings), lookup is by `assoc[key]` or `assoc[[Key[key]]]`, and it preserves insertion order for iteration while giving hash-map-speed lookup.

**Core use cases:**
- `<|"a" -> 1, "b" -> 2|>["a"]` → `1`: direct key lookup, the primary operation.
- `KeyValueMap[f, assoc]`, `Keys[assoc]`, `Values[assoc]`, `AssociationMap[f, list]`: the standard traversal/construction idioms, mirroring `Map` but for key-value pairs.
- `Merge[{assoc1, assoc2}, Total]`: combining associations with a conflict-resolution function for duplicate keys, a common data-wrangling idiom.
- `assoc["newKey"] = value` mutating/inserting in place, and `KeyDrop`/`KeyTake` for filtering keys: a user expects both functional (non-mutating) and mutating-assignment styles to both work naturally.
- Nested nested nested associations used as ad hoc JSON-like records (`<|"name" -> "x", "tags" -> {"a","b"}|>`), since `Association` is WL's structural answer to "parse this JSON and let me query it," feeding directly into `Dataset` (4.3).

**Implementation difficulty:** Easy to medium. Conceptually a hash map, so the base implementation is easy; the medium part is making it a first-class expression that participates correctly in pattern matching (`KeyValuePattern[...]` matching against associations), `Map`/functional operations applying to values not keys by convention, and preserving WL's specific semantics around duplicate keys (last write wins, order preserved from first occurrence) and `Normal`/round-tripping to a list of rules.

**Existing OSS building blocks:** Mathics3 has `Association` support with reasonable fidelity to core operations. Symja implements `Association` as well. Python's own `dict` (insertion-ordered since 3.7) is a nearly perfect underlying primitive to build this on if the host implementation language is Python; for other host languages an ordered hash map data structure is the direct equivalent to reach for.

**MVP priority:** Must-have. Modern WL code (post ~2014) leans on `Association` heavily as the default structured-data type; shipping without it would feel like a pre-2014 dialect.

---

### 4.3 Dataset (structure only)

**What it is:** `Dataset[...]` wraps a list of associations (or nested association/list structures) and gives it a special interactive display plus a query language via `[...]`-chaining (`dataset[Select[cond]][GroupBy[key]][...]`) that works uniformly whether the underlying data is a list of lists, list of associations, or association of associations. Note: the data-science *workflow* around `Dataset` (large-data ingestion, `SemanticImportString`, statistics) belongs to the numerics/data agent's spec: this section covers only the structural/query-language piece, which is a core-language concern because it's really a generalized `Map`/`Select`/`GroupBy` operating over an inferred nested schema.

**Core use cases:**
- `Dataset[{<|"a"->1,"b"->2|>, <|"a"->3,"b"->4|>}]` displayed as a formatted table: the basic wrap-list-of-records use case.
- `ds[Select[#a > 1 &]]`: filtering, chained the same way regardless of whether `ds` wraps flat lists or associations.
- `ds[All, "a"]`: column projection, pulling one field out of every record, the tabular-data-frame-like access pattern.
- `ds[GroupBy["category"], Length]`: grouping and aggregating, composing several query operations in one chained expression.
- A user expects `Normal[dataset]` to recover the plain nested list/association structure underneath at any point, i.e. `Dataset` is a thin queryable wrapper, not a separate opaque data format.

**Implementation difficulty:** Medium. The core trick is that `dataset[op1][op2][...]` is really just currying `op1`/`op2` (which are themselves ordinary functions like `Select`/`GroupBy`) over the wrapped data and re-wrapping the result, plus schema inference for how to render a mixed list/association/list-of-association nested structure as a sensible table. The hard-in-a-different-spec part (efficient large-data backing, out-of-core operation) belongs to the numerics/data spec, not here.

**Existing OSS building blocks:** Neither Mathics3 nor Symja has a real `Dataset` implementation as of current releases: this is a genuine, current OSS gap, not just an incompleteness. Architecturally, the closest open analog to study is less "another CAS" and more query-chaining data libraries generally (e.g. how pandas or dplyr structure chained/lazy operations), for the interaction design rather than for reusable code, since none of them share WL's "operate transparently over list-of-list vs list-of-association vs association-of-association" polymorphism.

**MVP priority:** Later. It's a nice-to-have convenience layer over `Select`/`GroupBy`/`Map` which must exist anyway (section 5); shipping those first and adding the `Dataset` wrapper/chaining sugar afterward is the efficient order, and there's no existing OSS reference implementation to lean on so it's inherently more original work per feature-hour than most of this spec.

---

### 4.4 SparseArray

**What it is:** `SparseArray[...]` is a memory-efficient representation for arrays (typically matrices, but any rank) that are mostly a single "background" value (usually `0`), storing only the explicit non-default entries while supporting the same indexing/arithmetic interface as a dense array/list.

**Core use cases:**
- `SparseArray[{{1,1}->1, {2,2}->1, {3,3}->1}, {3,3}]` constructing a sparse identity-like matrix by explicit rules rather than typing out all 9 entries.
- `SparseArray[{i_, i_} -> 1, {n, n}]`: pattern-based bulk construction, generating a large sparse structure from a rule pattern instead of listing every entry, which only makes sense once pattern matching (section 2) exists.
- `Normal[sparseArr]` to convert to a dense list when needed, and the reverse `SparseArray[denseList]`: a user expects seamless round-tripping, treating sparse as a storage optimization, not a different type to program against.
- Linear algebra on sparse matrices (`sparseA . sparseB`, `LinearSolve[sparseA, b]`) that a user expects to actually exploit sparsity for performance on large systems, not just "work" while secretly densifying: though the performance-critical numeric-solver side of this belongs to the numerics/data agent's spec.
- `ArrayRules[sparseArr]` to inspect exactly which entries are explicitly stored versus defaulted, for debugging/understanding the sparse representation itself.

**Implementation difficulty:** Medium for a correct-but-unoptimized version (a dict-keyed-by-index-tuple plus a default value, with `Normal`/indexing/arithmetic layered on top); hard to make it genuinely fast for large-scale sparse linear algebra (proper compressed storage formats, sparse-aware solvers): but that performance-engineering half is arguably numerics-agent territory, not core-language territory. The core-language piece is mainly: getting `SparseArray` to be a legitimate `Expression` head that pattern matching, `Map`, and generic list-like functions all handle correctly via `Normal` fallback or sparse-aware fast paths.

**Existing OSS building blocks:** SciPy's sparse matrix formats (CSR/CSC/COO) are the standard, battle-tested reference for underlying storage schemes even though SciPy's API isn't WL-shaped. Neither Mathics3 nor Symja has deep `SparseArray` support; this is another area (like `Dataset`) where the OSS clones leave a real gap rather than offering a template.

**MVP priority:** Later. Dense lists cover the overwhelming majority of early use cases; `SparseArray` matters once users start doing larger-scale linear algebra/graph-adjacency-matrix style work, which is reasonably deferrable past an initial credible MVP.

---

### 4.5 Packed arrays

**What it is:** An internal (mostly invisible to the user) storage optimization: when a list is homogeneous in type (all machine-precision reals, or all machine integers) and rectangular in shape, the kernel silently stores it as a flat contiguous numeric buffer instead of a tree of boxed `Expression` nodes, giving large speed and memory wins for numeric list processing, while `Head`/`Part`/pattern matching etc. behave identically to an "unpacked" list from the user's point of view: the packing is meant to be a transparent performance layer, not a user-facing type. `Developer\`PackedArrayQ` lets a user check packing status, and certain operations silently "unpack" (fall back to boxed representation) which is a well-known real-world performance foot-gun in actual Mathematica usage.

**Core use cases:**
- `Range[1000000]` or `Table[N[i], {i, 1, 1000000}]` staying fast and memory-light because the kernel auto-packs the resulting homogeneous numeric list, versus a naive boxed-tree implementation choking on a million-node expression tree.
- A user runs `Total[bigNumericList]`/`Map[f, bigNumericList]` and expects near-native-array performance (this is really a numerics-agent-facing performance concern, but the packing mechanism itself is a core-language/evaluator concern since it changes the internal representation of `List` expressions).
- A user inserts one symbolic element into an otherwise-numeric list (`Append[packedList, x]`) and it silently "unpacks" to the general boxed representation: power users specifically test for and guard against this because it's a documented, common performance cliff in real Mathematica code.
- `Developer`PackedArrayQ[list]` and `Developer`ToPackedArray`/`Developer`FromPackedArray`` as the explicit introspection/control API for users who need to reason about or force packing state directly.
- Interop with the numerics agent's array/tensor operations, which typically require or strongly prefer packed input for performance: packed arrays are the connective tissue between "it's just a WL list" and "it's efficient enough to hand to a numeric kernel."

**Implementation difficulty:** Medium-to-hard. Not conceptually hard (it's a classic boxed-vs-unboxed representation optimization, well understood in language-runtime design generally), but it's invasive: every list-producing and list-consuming builtin needs a fast path that checks for and preserves/produces packed representation, or the optimization doesn't actually pay off anywhere real workloads touch it, and getting the unpack-triggering rules to match real Mathematica closely enough that ported performance-sensitive code behaves as expected is genuinely fiddly, low-glory work.

**Existing OSS building blocks:** This is an area where none of the clones (Mathics3, Symja, expreduce) implement anything resembling true auto-packing as a transparent optimization: Mathics3 in particular is well known to be much slower than real Mathematica on large numeric list workloads specifically because it lacks this. NumPy's ndarray is the right underlying storage primitive to reach for if the host language is Python (i.e., "a packed array in a Python-hosted OpenMat kernel should just literally be a NumPy array wearing a `List`-head expression costume"), but the auto-pack/auto-unpack *policy* layer on top has no existing open reference to copy: it would be original engineering work informed by, but not portable from, any existing project.

**MVP priority:** Should-have, earlier than its "just a perf optimization" framing suggests: without it, any OSS clone hits an early, visible performance wall on completely ordinary numeric list workloads (`Table`, `Range`, `Map` over a few hundred thousand reals), which is a common first impression for evaluators kicking the tires. Full fidelity to Mathematica's exact unpack-triggering edge cases is later; "big homogeneous numeric lists are fast" as a basic guarantee is should-have.

---

### 4.6 Strings and string patterns

**What it is:** `String` is an atomic expression type (not a list of characters) with its own function library (`StringJoin`, `StringLength`, `StringTake`, `StringSplit`, `StringReplace`) and its own parallel pattern-matching mini-language: `StringExpression` patterns (`__` inside strings means "any characters," `StartOfString`/`EndOfString`/`WordBoundary`, `DigitCharacter`/`LetterCharacter` character classes) that mirror the structural pattern matcher's vocabulary but operate over string contents, plus full regular-expression support (`RegularExpression["..."]`) as an escape hatch/interop layer.

**Core use cases:**
- `StringJoin["a", "b", "c"]` (or `"a" <> "b" <> "c"`), `StringLength`, `StringTake[s, 3]`, `ToUpperCase`/`ToLowerCase`: basic string manipulation, expected to be as ergonomic and complete as any general-purpose language's string library.
- `StringSplit["a,b,,c", ","]` and `StringReplace["hello world", "o" -> "0"]`: split/replace as the most common text-processing operations, with `StringReplace` explicitly reusing rule syntax (`->`) from section 2 rather than a separate API.
- `StringMatchQ["hello123", LetterCharacter.. ~~ DigitCharacter..]` (`~~` is `StringExpression`'s concatenation operator): structural string pattern matching using WL's own pattern vocabulary (`__`, character classes) instead of dropping into regex syntax.
- `StringCases[text, RegularExpression["[0-9]+"]]`: regex-based extraction when the WL-native pattern vocabulary isn't expressive enough, expected to support standard PCRE-like regex syntax as an interop/escape hatch.
- `StringTemplate["Hello, \`name\`!"][<|"name" -> "World"|>]`: template-based string construction from an association, a common code-generation/report-formatting idiom.

**Implementation difficulty:** Easy for the basic string function library (mechanical wrapping of the host language's own string operations). Medium for `StringExpression` pattern matching done properly, since it needs its own matcher (structurally similar to, but a distinct implementation from, section 2's expression pattern matcher) supporting sequence patterns, character classes, and named captures, then bridging cleanly to/from real regex for the `RegularExpression` escape hatch.

**Existing OSS building blocks:** The host language's own regex engine (Python `re`, Java `java.util.regex`, Go `regexp`) directly covers the `RegularExpression[...]` escape hatch with no real engineering needed beyond syntax translation. Mathics3 and Symja both implement a reasonable subset of `StringExpression`-style pattern matching already and are directly studyable references for how to map WL's pattern vocabulary onto strings without just being "always fall back to regex," which would miss cases like named-capture-into-pattern-variables that need to interoperate with `/.`/`ReplaceAll`.

**MVP priority:** Must-have for the basic string function library (join/length/take/split/replace/case conversion): string handling is baseline-expected in any language. `StringExpression` pattern matching: should-have (very commonly used in real scripts for text processing, but a `RegularExpression`-only fallback is a tolerable stopgap for an early MVP).

---

## 5. Functional and Structural Programming

### 5.1 Map, Apply, Fold, Nest family

**What it is:** The core higher-order-function vocabulary for transforming and iterating: `Map[f, list]` (`f /@ list`) applies `f` to every element; `Apply[f, expr]` (`f @@ expr`) replaces an expression's head; `Fold[f, init, list]` reduces a list to a single value by repeated pairwise combination; `Nest[f, x, n]` applies `f` to its own output `n` times; `NestWhile`/`FixedPoint` iterate until a condition/fixed point is reached. These compose fluidly with pure functions (5.2) and are the idiomatic replacement for explicit loops in most WL code.

**Core use cases:**
- `Map[f, {1,2,3}]` or `f /@ {1,2,3}` → `{f[1],f[2],f[3]}`: the single most common functional idiom in the language, used constantly and expected to be second nature/terse via the `/@` operator form.
- `Apply[Plus, {1,2,3}]` (`Plus @@ {1,2,3}`) → `6`, and more generally using `Apply` to change an expression's head, e.g. `List @@ (a+b+c)` → `{a,b,c}` pulling `Plus`'s arguments out into a list: a very common "get inside the expression" idiom tying back to section 1.1.
- `Fold[Plus, 0, {1,2,3,4}]` → `10`, and more interestingly `Fold[f, init, list]` used for genuine stateful-accumulator-style reductions (running totals, state machines) that a simple `Total`/`Map` can't express.
- `Nest[f, x, 5]` applying a function five times, and `NestList[f, x, 5]` returning every intermediate result as a list: used for simulating iterative processes (e.g. Newton's method steps, cellular automaton generations) directly and visibly.
- `FixedPoint[f, x]` iterating until `f[x] === x` (used, notably, as literally how `ReplaceRepeated` conceptually works): a user reaches for this whenever they want "keep applying this until it stops changing" without manually writing the loop/termination check.

**Implementation difficulty:** Easy. These are all direct, mechanical implementations once sections 1-2 (expressions, evaluation, pattern-based function calling) exist: `Map` is a loop calling `f` on each element and rebuilding a `List`, `Apply` is a head-swap, `Fold`/`Nest` are simple accumulator loops. The only real subtlety is getting the `Listability`/level-spec options right (`Map[f, expr, {2}]` mapping at a specific depth) and matching exact edge-case semantics (e.g. `Apply`'s level spec default of `{0}` vs `Map`'s `{1}`).

**Existing OSS building blocks:** Universally and completely implemented in all three clones: this is "finished technology" across the whole OSS landscape, nothing left to research. Python's own `map`/`functools.reduce` and Haskell/Lisp's fold/apply traditions are the general-CS analogs if implementing from scratch without reference to a WL clone at all.

**MVP priority:** Must-have, day one, alongside pattern matching: genuinely can't write idiomatic WL code without this family.

---

### 5.2 Pure (anonymous) functions

**What it is:** `Function[x, body]` or the terser `body &` with numbered slots (`#1`, `#2`, or `#` for the first/only argument) defines an anonymous function without naming it, used pervasively as the `f` argument to `Map`/`Select`/`Fold`/etc. rather than defining a named helper for every small transformation.

**Core use cases:**
- `Map[#^2 &, {1,2,3}]` → `{1,4,9}`: the single most common use, an inline transformation passed straight to a higher-order function, expected to be terse enough to type without breaking flow.
- `Select[list, # > 0 &]`: pure functions as predicates, used constantly with `Select`/`Cases`/`SortBy` etc.
- `Function[{x, y}, x + y]` or `(#1 + #2) &` for multi-argument pure functions, when the single-`#` shorthand isn't enough.
- `Function[x, x^2, Listable]` or attribute-aware pure functions, a less common but real use case where a user wants a pure function to also carry evaluation attributes.
- Nested pure functions and the well-known `#`-scoping gotcha: `Map[Map[#+1&, #]&, matrix]`: a user needs the inner and outer `#` to resolve to their respective enclosing `Function`, which is a real semantic (dynamic-scoping-like resolution to nearest enclosing `Function`) that needs to be implemented correctly, not just "the last one wins."

**Implementation difficulty:** Easy. `Function`/`&` desugars into essentially the same rule-application machinery as `SetDelayed` (section 2.3) minus the persistent-storage-on-a-symbol part: call it, substitute slots/named parameters into the body, evaluate. The one thing worth real care is correct `#`-slot scoping when `Function`s nest, since naive implementations get this wrong.

**Existing OSS building blocks:** Fully implemented everywhere (Mathics3, Symja, expreduce all support both `Function[...]` and `#...&` forms); no gaps to speak of. Lambda calculus / anonymous-function support in essentially any general-purpose language is the conceptual analog if building from first principles.

**MVP priority:** Must-have, day one, for the same reason as 5.1: `Map[f, list]` is much less useful without being able to write `f` inline as `#+1&`.

---

### 5.3 Listability

**What it is:** The `Listable` attribute, present on most mathematical built-ins (`Plus`, `Times`, `Sin`, `Sqrt`, etc.), makes a function automatically thread/distribute over lists (and nested lists, matching structure element-by-element) without the user having to call `Map` explicitly: `Sin[{1,2,3}]` just works and returns `{Sin[1],Sin[2],Sin[3]}`, and `{1,2}+{3,4}` gives `{4,6}` via the same mechanism applied to a two-argument built-in.

**Core use cases:**
- `Sin[{0, Pi/2, Pi}]` → element-wise application without any explicit `Map`: the most basic and most-relied-upon manifestation.
- `{1,2,3} + {10,20,30}` → `{11,22,33}`: arithmetic operators threading over lists automatically, which users coming from array-programming languages (NumPy, MATLAB) expect implicitly and users coming from math notation expect because it matches vector addition.
- `f[{1,2},{3,4}]` for a user-defined function `f` with `SetAttributes[f, Listable]`: listability as something ordinary users can opt their own functions into, not just a built-in-only mechanism.
- `{1,2,3} + 5` → `{6,7,8}`: scalar broadcasting against a list (one argument list, one argument scalar) as a special case of the same threading rule.
- A user calling a listable function on a *ragged* nested structure or on arguments of visibly mismatched list lengths and expecting a clear `Thread::tdlen`-style error rather than silent wrong behavior or a crash.

**Implementation difficulty:** Easy to medium. The base case (single-level threading over same-length lists, or list-vs-scalar) is easy: check the `Listable` attribute, and if present, either zip-map across matching-length list arguments or broadcast scalars against them before applying the function. Medium comes from getting multi-level/nested threading, mismatched-length error semantics, and the interaction with `Listable`-plus-other-attributes (e.g. `Listable` and `Orderless` both on the same head) exactly right, and from making sure it's efficient rather than route through a generic slow path for every arithmetic op (this is a major reason "packed arrays," section 4.5, exist: listability over packed numeric lists is where the performance actually matters).

**Existing OSS building blocks:** Universally implemented (it's not optional: basic arithmetic on lists is one of the first things any WL clone's test suite checks). Nothing distinctive to borrow beyond what any of the three clones already do; NumPy's broadcasting rules are a useful general mental model for the scalar-vs-list threading case even though WL's exact rules aren't identical to NumPy's.

**MVP priority:** Must-have, day one: `{1,2,3}+1` not working would be an immediate, glaring correctness gap for any WL-literate evaluator.

---

### 5.4 Table, Do, While

**What it is:** `Table[expr, {i, imin, imax}]` (and multi-index/nested forms) is the primary list-generation construct: a declarative "build a list by evaluating this expression for each value of the iterator" rather than an imperative loop with manual accumulation. `Do[expr, {i, imin, imax}]` is the imperative sibling for side-effecting iteration without building a list. `While[cond, body]` is the general imperative loop for when the iteration count isn't known up front. All three exist alongside the functional family (5.1) rather than being fully superseded by it: real WL code mixes both styles freely.

**Core use cases:**
- `Table[i^2, {i, 1, 10}]` → `{1,4,9,...,100}`: declarative list-building from a formula and a range, extremely common, arguably more idiomatic than `Map[#^2&, Range[10]]` for this exact case even though both work.
- `Table[i*j, {i,1,3}, {j,1,3}]`: nested/multi-index `Table` producing a matrix (list of lists) in one construct, a very common way to build small symbolic or numeric matrices.
- `Do[Print[i], {i, 1, 5}]`: pure side-effecting iteration (printing, mutating an external accumulator via `AppendTo`, etc.) where building and discarding a list (as `Table` would) is wasteful or semantically wrong.
- `While[x < 100, x = x*2]`: condition-driven looping when the number of iterations isn't known ahead of time, e.g. iterative numeric algorithms with a convergence check.
- `Table[f[x], {x, list}]` (iterating directly over an explicit list of values rather than a numeric range): a user expects the same `Table` syntax to generalize smoothly from "range of numbers" to "elements of any list."

**Implementation difficulty:** Easy. These desugar straightforwardly (`Table`/`Do` unroll the iterator spec into a loop that evaluates the body once per iteration, collecting results into a `List` for `Table` or discarding them for `Do`; `While` is a direct evaluate-condition/evaluate-body loop) once `HoldAll`-style held evaluation (section 1.3) and scoping (5.5, since the iterator variable needs to be properly localized) already exist.

**Existing OSS building blocks:** Fully and unremarkably implemented across all three clones; no gaps. General-purpose language `for`/`while` loops are the conceptual analog for anyone implementing from scratch.

**MVP priority:** Must-have, day one.

---

### 5.5 Scoping: Module, Block, With

**What it is:** Three distinct scoping constructs with genuinely different semantics, a common source of real user confusion worth being precise about. **`Module[{vars}, body]`** gives *lexical* scoping with fresh, uniquely-renamed local variables per call (implemented by literally generating a unique symbol name like `x$123` each invocation, so recursive/nested calls don't collide); it's the default choice for "I need a local variable." **`Block[{vars}, body]`** gives *dynamic* scoping: it temporarily rebinds an existing (often global) symbol's value for the duration of `body`, restoring it afterward, and the rebinding is visible to anything called from within `body` regardless of lexical nesting, which is why it gets used deliberately for things like temporarily changing `$RecursionLimit` or a global option during one computation. **`With[{x = val}, body]`** gives simple, non-reassignable lexical substitution (closer to a `let`/macro-expansion), replacing `x` with `val` literally in `body` at scoping time, most efficient and most restrictive of the three (no reassignment inside `body`).

**Core use cases:**
- `Module[{sum = 0}, Do[sum += i, {i, 1, 10}]; sum]`: the default "I need a local mutable variable and a temporary scratch computation" idiom, expected to behave like a local variable in any conventional language (no leakage, safe under recursion).
- `Block[{$RecursionLimit = 10000}, riskyRecursiveCall[]]`: dynamic rebinding of a global/system variable for the duration of one call, specifically relying on the callee (which may be defined elsewhere, unaware of the `Block`) seeing the temporarily-changed value: this is the use case that most sharply distinguishes `Block` from `Module` and trips up users who reach for the wrong one.
- `With[{n = 5}, n^2 + n]`: simple substitution for values that won't change within the body, preferred for performance and clarity when reassignment isn't needed (e.g. defining constants used in a formula).
- A user writing a recursive `Module`-scoped function and correctly expecting each call to get its own fresh locals (no shared-mutable-state bug between concurrent/recursive invocations): this is the property that specifically requires `Module`'s unique-renaming implementation, not just "a local variable that shadows."
- A user deliberately choosing `Block` over `Module` specifically to affect the behavior of *other, already-defined* functions during one call (e.g. temporarily setting a global numeric precision variable): a legitimately common, WL-idiomatic pattern that has no direct equivalent in most mainstream languages' scoping model, so it's a "explain the mental model, then implement it exactly" feature rather than "obviously implement it like X."

**Implementation difficulty:** Medium. `With` is easy (compile-time-style substitution). `Module` is medium: needs a fresh-unique-symbol-name generator per invocation and correct interaction with `HoldAll`-attributed callers so the localized names don't leak into held/unevaluated expressions incorrectly. `Block` is medium-to-tricky: needs genuine dynamic scoping (save old value, install new value, guarantee restoration even on error/non-local exit via `Throw`/`Catch` or abort), which is a different mechanism from `Module`'s lexical renaming and easy to accidentally implement as if it were "`Module` but for globals" (wrong: the distinguishing behavior is that callees see the rebinding, which a renaming-based implementation would not give you).

**Existing OSS building blocks:** Mathics3 and Symja both implement all three with reasonable fidelity to the lexical-vs-dynamic distinction; Mathics3's `mathics.builtin.evaluation` module (or equivalent) is a good reference specifically for how it separates `Module`'s renaming approach from `Block`'s save/restore approach in code, since conflating them is the most common correctness bug this feature invites. No mainstream general-purpose language has all three of these scoping disciplines built in side by side, so unlike most of this spec, there isn't a simple "here's the analogous feature in language X" shortcut: Common Lisp's `let` (lexical) vs special-variable dynamic binding is the closest classical-CS analog to `Module`-vs-`Block`, if that framing helps.

**MVP priority:** Must-have (`Module`, `With`) since local variables are basic hygiene for writing any nontrivial function body. `Block`: should-have: genuinely used in real code (especially around numeric precision and recursion-limit-style system variables) but a smaller fraction of everyday code depends on its specific dynamic-scoping behavior compared to `Module`/`With`.

---

## 6. Exact Numerics

### 6.1 Arbitrary-precision integers and rationals

**What it is:** WL integers have no fixed bit-width: `2^1000` is computed exactly, not overflowed or truncated, and rational numbers (`1/3`, `22/7`) are kept as exact numerator/denominator pairs rather than converted to floating point unless the user explicitly asks for a numeric approximation. This "stay exact unless told otherwise" default is one of the most fundamental and most immediately noticeable differences from general-purpose languages and from purely-numeric tools.

**Core use cases:**
- `2^1000` returning the full exact ~302-digit integer, not overflow or a floating-point approximation: the canonical "wait, it just does that?" first-impression feature for people new to the language.
- `1/3 + 1/6` → `1/2` exactly, not `0.5` or `0.4999999999999999`: exact rational arithmetic that stays exact through a chain of operations rather than degrading to floating point at the first division.
- `100!` (factorial) computed exactly as a 158-digit integer: big-integer arithmetic used constantly in number theory, combinatorics, and just as a stress test users run to confirm the system is "real."
- `GCD`/`LCM`/`Mod`/`PowerMod`/`FactorInteger` on large integers: number-theoretic operations that are only meaningful/correct if the underlying integer arithmetic is exact and arbitrary-precision to begin with.
- A user expects `1/3` to display as the fraction `1/3` by default (not `0.333...`) and to only convert to a decimal approximation when they explicitly call `N[1/3]`: the default-to-exact behavior is a user-facing contract, not just an internal implementation detail.

**Implementation difficulty:** Easy. This is genuinely solved technology: every mainstream language ecosystem has a mature arbitrary-precision integer library (GMP being the reference C implementation nearly everything else wraps or reimplements), and rationals are a thin numerator/denominator-pair-plus-GCD-reduction layer on top of that. This is the single easiest item in this entire spec.

**Existing OSS building blocks:** GMP (or a language-native equivalent: Python's built-in arbitrary-precision `int`, Java's `BigInteger`/`BigDecimal`) is the direct, complete, battle-tested building block; there is no reason to reimplement big-integer arithmetic from scratch. All three clones simply use their host language's existing big-integer support (Mathics3/Python's native ints, Symja/Java `BigInteger`, expreduce/Go's `math/big`) rather than rolling their own, which is clearly the correct approach for OpenMat too.

**MVP priority:** Must-have, day one, and effectively free given host-language support: there's no reason not to make this a foundational, non-negotiable part of the number tower from the very first version.

---

### 6.2 Exact irrationals and symbolic constants

**What it is:** `Pi`, `E`, `Sqrt[2]`, `GoldenRatio` and similar are kept as exact symbolic entities (not floating-point approximations) that participate in exact algebraic simplification (`Sqrt[2]*Sqrt[2]` → `2` exactly, `Sin[Pi]` → `0` exactly) and are only converted to a numeric approximation, to however many digits are requested, when `N[...]` is explicitly applied.

**Core use cases:**
- `Sqrt[2]^2` → `2` exactly (not `2.0000000000000004`-style floating point error): the headline behavior that distinguishes a real CAS from a calculator, and a very common first test users run.
- `N[Pi, 50]` → 50 digits of Pi computed on demand: arbitrary-precision numeric evaluation of a symbolic constant, decoupled from any fixed hardware float width.
- `Sin[Pi/6]` → `1/2` exactly, using known special-angle identities, rather than falling back to a floating-point sine evaluation: exact irrationals need to participate in the special-function identity system (section 3.7), not just be inert symbols.
- `Sqrt[2] + Sqrt[2]` → `2 Sqrt[2]`: exact irrationals combining algebraically (like terms collecting) the same way polynomial terms do, since under the hood `Sqrt[2]` is just `Power[2, 1/2]`, another ordinary expression subject to the same simplification machinery as everything else.
- A user compares `Pi == 3.14159...` (numeric) vs `Pi === Pi` (exact symbolic identity) and expects `SameQ`/`Equal` to behave correctly and predictably across the exact/approximate boundary: including the well-known subtlety that `Equal` between an exact and an approximate number does numeric comparison to available precision rather than claiming exact numbers "equal" an inherently-approximate float representation of themselves.

**Implementation difficulty:** Medium. Representing `Pi`/`E`/`Sqrt[2]` as ordinary symbolic expressions is easy (they're just symbols/expressions with special evaluation rules and a registered arbitrary-precision numeric evaluation algorithm attached, e.g. a fast Pi-digit algorithm). The medium-difficulty part is making sure the general expression-simplification machinery (section 3.1, `PowerExpand`, like-term collection) treats `Sqrt[2]` correctly as `2^(1/2)` for combining/canceling purposes without a pile of special cases specific to "irrational constants" as a category unto themselves: the right design keeps this uniform with ordinary polynomial/power simplification rather than bolting on a parallel system.

**Existing OSS building blocks:** mpmath again is the strongest reference for the arbitrary-precision numeric-evaluation side (computing Pi/E/algebraic numbers to arbitrary requested digit counts efficiently). SymPy's exact-irrational handling (`sympy.sqrt(2)`, its `Rational`/`Pow` interaction, and its `nsimplify` for going the other direction: recognizing that a float is probably `Sqrt[2]` in disguise) is the closest architectural analog for the symbolic side. All three WL clones handle `Pi`/`E`/`Sqrt` as exact symbolic expressions already, consistent with this approach.

**MVP priority:** Must-have. This is as core to "being a CAS" as pattern matching is to "being WL": a variant that silently floats `Pi` to a machine double the moment it's touched would fail the most basic credibility test with any Mathematica-literate evaluator.

---

### 6.3 Precision tracking (significance arithmetic)

**What it is:** WL tracks, per numeric value, how many digits are actually known to be meaningful (`Precision[x]`) and how many digits before/after the decimal point are meaningful in absolute terms (`Accuracy[x]`), and propagates this through arithmetic so that error introduced by limited-precision inputs is honestly reflected in outputs: this is fundamentally different from fixed-width IEEE floating point (where every `Real`, no matter its true accuracy, always reports 53 bits/~15-17 digits) and different from naive arbitrary-precision (which would need explicit, manual error/interval tracking bolted on separately). A machine-precision number (`2.5`) and an arbitrary-precision number with explicitly tracked precision (`` 2.5`20 `` meaning "20 significant digits, and I mean it") are genuinely different representations in WL, not the same float dressed up differently.

**Core use cases:**
- `N[Pi, 30]` → Pi to 30 significant digits, and the result carries `Precision[...] == 30`, i.e. the system knows and can report exactly how trustworthy that number is, not just how it's printed.
- `1.0000000000000000000000` (a literal typed with many digits) being interpreted as a *high-precision* number (WL infers precision from the number of digits typed when using the `` `## `` precision-mark syntax or from context), contrasted with `1.` (machine precision, ~16 digits): the same "one" meaning something different depending on how much precision was actually asserted.
- `a = 1.234560000`; `Precision[a]` reflecting that trailing zeros in a literal do count as asserted significant digits in WL's convention: a specific, easy-to-get-wrong parsing/precision-inference rule that real programs (and confused beginners) depend on being correct.
- Multiplying a low-precision number by a high-precision one and watching precision correctly degrade to the lower of the two (roughly), so a user doing a chain of arbitrary-precision arithmetic gets an honest, automatically-tracked estimate of how many digits of the final result are actually trustworthy, without manually doing interval/error analysis themselves.
- `SetPrecision[x, 50]` and `SetAccuracy[x, 50]` to explicitly assert/change the tracked precision of a value (as opposed to `N[x, 50]` which recomputes to that many digits): used when a user wants to explicitly declare a number's trustworthiness rather than derive it from a computation.

**Implementation difficulty:** Hard. Machine-precision arithmetic (IEEE double, mapped directly onto the host language's native float type) is easy and should be the fast-path default for ordinary `2.5`-style numeric work. The hard part is *arbitrary*-precision arithmetic with genuine significance tracking: every arithmetic operation needs to compute not just a numeric result but also a correctly-propagated precision/accuracy estimate for that result (Wolfram's model is closer to interval-arithmetic-derived error bounds than a simple "min of the two operands' precision" rule, particularly around cancellation: subtracting two close high-precision numbers correctly *loses* precision in the result, which a naive implementation easily gets wrong), and the arbitrary-precision arithmetic itself needs a real bignum-float library (mantissa+exponent+explicit precision, not just "more bits of a fixed-width float"). This is genuinely one of the most differentiated, least-copied pieces of real Mathematica; it's a documented area where even sophisticated users find edge cases surprising, and it's fair to call full-fidelity replication research-grade, while a reasonable simplified approximation (machine doubles by default, mpmath-style fixed-extra-precision arbitrary-precision as an opt-in tier, without exactly replicating Wolfram's specific error-propagation formulas) is medium difficulty and workable for an MVP.

**Existing OSS building blocks:** mpmath is the strongest available building block: it does real arbitrary-precision floating point with a settable working precision, though its default model (global working precision, "guard digits" for safety) is closer to conventional arbitrary-precision libraries (MPFR-style) than to WL's per-number significance-tracking model; it can be used as the underlying bignum-float engine while a thin precision-tracking layer is built on top to approximate WL's semantics, rather than expecting mpmath's own precision model to already match. Symja uses `apfloat` (a Java arbitrary-precision library) similarly, as an underlying engine rather than a semantics match. No existing OSS clone (Mathics3, Symja, expreduce) claims to fully replicate Wolfram's exact significance-arithmetic propagation rules: this is consistently the piece all of them are honest about being an approximation of, not a faithful clone of, and it should be treated the same way in this spec's honesty about scope.

**MVP priority:** Should-have in simplified form (machine precision as the fast default, a working arbitrary-precision mode via an mpmath-style engine with user-settable precision): must-have in the sense that *some* arbitrary-precision numeric mode is expected of any serious CAS, but full Wolfram-exact significance/accuracy propagation semantics are explicitly later/research-grade and should be scoped out of MVP with that stated honestly rather than half-implemented silently.

---

## MVP Slice

> **Superseded.** The authoritative MVP scope is now [m0-milestone.md](m0-milestone.md) (see issue #1), which merges this build order with the other two specs' slices into one milestone table. This section is kept for research context.

The minimal coherent subset an OSS core language needs to ship first to be credible with a Mathematica-literate evaluator, in rough build order:

1. **Expression kernel**: the universal `Head[args...]` tree (1.1), symbol table with a flat `Global`/`System` namespace (1.4, deferring full multi-package contexts), machine-precision reals plus arbitrary-precision integers/rationals from day one via a host bignum library (6.1).
2. **Evaluation loop**: fixed-point rewriting, downvalues, and `Hold`-family attributes (1.2, 1.3): without this nothing else runs.
3. **Pattern matcher**: `_`/`__`/`___`, named/typed/conditioned patterns, and critically `Orderless`/`Flat` matching for `Plus`/`Times` (2.1): this is the single highest-leverage, highest-risk piece; skipping `Orderless` matching makes basic algebra feel broken.
4. **Rule application and function definition**: `Replace`/`/.`/`//.` and `SetDelayed` with specificity-ordered multi-clause definitions (2.2, 2.3).
5. **Lists, Associations, basic strings**: the three data structures nearly all real code touches immediately (4.1, 4.2, 4.6's basic function library); `Dataset`/`SparseArray` deferred.
6. **Functional core**: `Map`/`Apply`/`Fold`/`Nest`, pure functions, listability, `Table`/`Do`/`While`, `Module`/`With`/`Block` (all of section 5): this is comparatively easy and low-risk, and it's what makes the language usable for real scripts once 1-4 exist.
7. **Polynomial layer**: `Expand`/`Factor`/`Together`/`Cancel`/`PolynomialGCD`/`Collect` (3.5): bind an existing library (SymPy's `sympy.polys` algorithms or a similar mature reference) rather than build from scratch; this underpins everything else in the CAS layer.
8. **Calculus and solving basics**: `D` (easy, total), basic `Simplify`, `Solve` for single-variable polynomials and linear systems, a rule-based `Integrate` subset seeded from a ported piece of Rubi (3.2, 3.1, 3.4): this is where "OpenMat" starts feeling like an actual CAS rather than a rewrite engine with algebra bolted on.
9. **Precision tracking, basic tier**: machine doubles as default, an mpmath-style arbitrary-precision opt-in mode, without chasing Wolfram's exact significance-propagation formulas (6.3).

**Biggest technical risks, ranked:**

1. **`Orderless`/`Flat` pattern matching** (2.1) is the one piece of the "must-have day one" list that is genuinely hard, not just effortful: it's a real combinatorial search problem, it's load-bearing for all of algebra, and getting it slow-but-correct is much easier than getting it fast-and-correct, so early performance complaints are likely regardless of care taken.
2. **`Integrate`/`Solve`/`Reduce` generality** is open-ended by nature: these are the areas where real Mathematica represents decades of accumulated special-case coverage (Rubi's 6000+ rules being the clearest evidence of how much of this is long-tail breadth, not depth), so the risk isn't "can we build a version," it's "stakeholders anchoring expectations on full-Mathematica coverage" when the honest MVP target is a well-chosen, clearly-scoped common-case subset with graceful, honest failure outside it.
3. **Packed arrays / performance** (4.5) is easy to deprioritize as "just an optimization" and then become the reason early demos feel sluggish on completely ordinary numeric workloads: this is a case where an OSS clone (Mathics3 is the cautionary example) can be functionally complete-ish yet feel unusably slow, hurting credibility more than a missing feature would.
4. **Significance arithmetic** (6.3) is the piece most likely to cause silent, hard-to-notice correctness drift if under-scoped without being explicit about it: shipping machine-precision-only and calling it "numerics" quietly is fine; shipping an arbitrary-precision mode that *looks* like it tracks precision but doesn't propagate it correctly is worse than not having the feature, because it produces plausible-looking wrong answers.
5. **Scope creep from `Simplify`/`FullSimplify`** (3.1): because "simplify this" is an open-ended, satisficing goal rather than a well-specified transformation, it's the single easiest area in this whole spec to sink unbounded engineering time into chasing Mathematica's exact output form, when "produces a valid, if not identical, simpler form" is both the achievable and the appropriate bar.
