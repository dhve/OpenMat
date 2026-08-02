// Essential subset of OpenMat's supported linear syntax, embedded as a
// constant so the LLM system prompt (see ./systemPrompt.ts) always carries
// it, rather than depending on the model already knowing OpenMat. Derived
// from specs/grammar.md, the normative grammar reference; keep this in
// sync if that document's supported forms change.
export const GRAMMAR_SUMMARY = `Supported functions (apply with square brackets, e.g. Sin[x]):
  Sin  Cos  Tan  Exp  Log  Sqrt  Abs
  D[expr, var]                      derivative of expr with respect to var
  Integrate[expr, var]              indefinite integral
  Integrate[expr, {var, a, b}]      definite integral of expr from a to b
  Solve[eqn, var]                   solve an equation for var
  Expand[expr]                      expand or distribute an expression
  Table[expr, {i, imin, imax}]      build a list by evaluating expr for each i
  Range[n]                          the list {1, 2, ..., n}
  Map[f, list]                      apply f to every element of list
  NDSolve[{eqns}, y, {t, t0, t1}]   numerically solve a differential equation
  Plot[expr, {x, a, b}]             plot expr over x from a to b
  ListPlot[data]                    plot a list of numbers or {x, y} pairs

Operators: + - * / ^ == -> := =
  ==  is symbolic equality (used inside Solve, NDSolve equations, etc.)
  ->  is a replacement rule
  :=  is a delayed assignment
  =   is an immediate assignment

Patterns: x_ is a blank pattern that matches anything, bound to the name x.
Lists: {a, b, c}.
Derivatives: x'[t] is a first derivative of x with respect to t, x''[t] a second derivative.

Do not use any function, operator, or form outside this list. If the request
cannot be expressed with exactly these, use the closest supported form
rather than inventing new syntax.`;
