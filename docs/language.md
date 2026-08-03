# The OpenMat Language

OpenMat speaks a Wolfram-Language-shaped notation: square brackets apply functions, curly braces build lists, `=` assigns, `==` states equations, and capitalized names are built-ins. If you know Mathematica, you already know how to type here. This page documents exactly what the v0.01 kernel understands, so you can use OpenMat entirely by hand, no natural language features required.

Everything on this page is real kernel behavior, verified against the shipped build.

## The notebook

- **Shift+Enter** evaluates a cell. **Enter** moves to the next cell (or creates one at the end).
- Evaluated cells get Mathematica-style `In[n]:=` and `Out[n]=` labels. Errors consume an `In` number, exactly as in Mathematica.
- Definitions evaluate silently: `f[x_] := x^2` produces no `Out` line, matching Mathematica's `Null` convention.
- Click a cell's right-edge bracket to collapse it to its input line; click again to expand.
- Cell styles: **Alt+1** Title, **Alt+4** Section, **Alt+7** Text, **Alt+9** back to Input. The toolbar buttons do the same.
- **Save** and **Open** in the header store the whole notebook as a file.
- The kernel session persists across cells: what you define stays defined until `Clear` or a restart.

## Typing math

Input cells are 2D math fields. You can type Mathematica-style linear syntax directly and it comes out right:

- `Plot[Sin[x], {x, 0, 10}]` works typed literally: square brackets stay calls, braces stay lists.
- Consecutive letters form one symbol, as in Mathematica: `xy` is the symbol `xy`, not `x*y`. Multiplication needs a boundary: `2x`, `2 Sin[x]`, `a*b`.
- `^` starts an exponent (the caret then moves you into the superscript; type the exponent and arrow right to come back down).
- `=` with a plain symbol on the left assigns; with anything else on the left it becomes the equation `==`. Typing `==` always means Equal.
- Primes are derivatives in equations: `x''[t]` is the second derivative of `x`.
- Hover a math cell for the keyboard icon: a full math symbol keyboard (Greek letters, roots, fractions, operators). Typed integrals like `∫ x^2 dx` translate to `Integrate[x^2, x]`.
- Known function names typed with parentheses are accepted too: `Sin(x)` and `Plot(...)` normalize to bracket form.

## Numbers

The kernel keeps exact values exact and only goes numeric when you ask.

```
In[1]:= 2^10
Out[1]= 1024

In[2]:= Sqrt[8]
Out[2]= Sqrt[8]

In[3]:= N[Sqrt[2]]
Out[3]= 1.414213562373

In[4]:= 0.1 + 0.2
Out[4]= 0.3
```

- `N[expr]` forces numeric evaluation. Constants `Pi`, `E`, and `Degree` resolve under `N`; `Infinity` is recognized symbolically.
- Floating point noise is cleaned to 12 significant digits, so `0.1 + 0.2` prints `0.3`.
- Exact fractions like `1/3` stay symbolic rather than becoming decimals. Rational arithmetic is not folded yet: `1/3 + 1/6` stays as written until you apply `N`.

## Variables and functions

```
In[1]:= a = 5
Out[1]= 5

In[2]:= a^2 + 1
Out[2]= 26

In[3]:= f[x_] := x^2 + 1

In[4]:= f[3]
Out[4]= 10

In[5]:= g[x_, y_] := x*y

In[6]:= g[2, 5]
Out[6]= 10

In[7]:= Clear[a]
```

- `=` (Set) evaluates the right side once and assigns. `:=` (SetDelayed) stores the definition and evaluates at each use, silently.
- `x_` is a blank pattern: it matches any expression and binds it to `x`. Multiple pattern arguments work.
- Definitions are visible everywhere, including inside `Plot` and `NDSolve`: define `f[x_] := x^2` in one cell and `Plot[f[x], {x, 0, 2}]` in the next just works.
- `Clear[name]` removes a definition.

## Lists

```
In[1]:= {2, 3, 5, 7}
Out[1]= {2, 3, 5, 7}

In[2]:= Table[n^2, {n, 1, 5}]
Out[2]= {1, 4, 9, 16, 25}

In[3]:= Range[5]
Out[3]= {1, 2, 3, 4, 5}

In[4]:= Length[{1, 2, 3}]
Out[4]= 3

In[5]:= Map[f, {1, 2}]
Out[5]= {f[1], f[2]}
```

Arithmetic does not yet thread over lists: `{1, 2} + {10, 20}` stays unevaluated. Use `Table` to build element-wise results.

## Algebra

```
In[1]:= Expand[(x + 1)^2]
Out[1]= 1 + 2*x + x^2

In[2]:= Factor[x^2 - 1]
Out[2]= (-1 + x)*(1 + x)

In[3]:= Solve[x^2 - 5*x + 6 == 0, x]
Out[3]= {{x -> 2}, {x -> 3}}

In[4]:= Solve[x^2 - 2 == 0, x]
Out[4]= {{x -> -Sqrt[2]}, {x -> Sqrt[2]}}
```

- `Solve` handles polynomial equations up through quadratics and returns exact roots as replacement rules.
- `Simplify` performs basic structural simplification. It does not yet know trigonometric identities: `Simplify[Sin[x]^2 + Cos[x]^2]` stays as written.

## Calculus

```
In[1]:= D[Sin[x]*x, x]
Out[1]= Cos[x]*x + Sin[x]

In[2]:= Integrate[x^2, x]
Out[2]= x^3/3

In[3]:= Integrate[2*Sin[2*Pi*x]^2, {x, 0, 1}]
Out[3]= 1.

In[4]:= Integrate[Exp[-x^2/2], {x, 0, 1}]
Out[4]= 0.855624391892
```

- `D[f, x]` differentiates with the product, quotient, chain, and power rules.
- `Integrate[f, x]` knows the standard table: powers, `1/x` to `Log`, `Sin`, `Cos`, `Exp`, linear substitutions, and polynomial expansion. Powers of sine and cosine reduce via double-angle rules, and products like `Sin[2*Pi*x]*Sin[Pi*x]` go through product-to-sum, which is what makes normalization and orthogonality integrals come out exact.
- Definite integrals evaluate the antiderivative difference. When no closed form exists but the bounds are numeric, the kernel falls back to numerical quadrature rather than giving up, which is why the Gaussian above just answers.

## Plotting

```
Plot[Sin[x], {x, 0, 10}]
Plot[{Sin[x], Cos[x]}, {x, 0, 2*Pi}]
Plot[Tan[x], {x, -4, 4}]
ListPlot[{1, 4, 9, 16, 25}]
ListPlot[{{1, 2}, {2, 4}, {3, 9}}]
```

- `Plot` samples adaptively: it refines where the curve bends and detects poles, so `Tan[x]` renders as separate branches instead of vertical lines through the asymptotes.
- Multiple expressions in a list get separate curves and a legend.
- `ListPlot` draws discrete points, from plain values (x becomes 1, 2, 3, ...) or `{x, y}` pairs.
- Session definitions resolve inside plots: `data = Table[n^2, {n, 1, 8}]` then `ListPlot[data]` works.

## Differential equations

```
NDSolve[{x'[t] == -x[t], x[0] == 1}, x, {t, 0, 5}]
NDSolve[{x''[t] + 0.3*x'[t] + Sin[x[t]] == 0, x[0] == 2, x'[0] == 0}, x, {t, 0, 20}]
```

- `NDSolve` solves scalar first and second order ODEs numerically and plots the trajectory.
- Write the equation with `==`, give initial conditions as `x[0] == value` (and `x'[0] == value` for second order), name the unknown function, and give the range `{t, t0, t1}`.
- Second order equations must be linear in the highest derivative (any textbook `x'' = f(t, x, x')` form qualifies).
- The desktop build integrates with SUNDIALS CVODE; the browser build uses a pure Rust Dormand-Prince RK5(4). Same input, same answer.
- Cells with a parameter slider (like the damped pendulum demo) re-solve live as the slider moves; the slider's symbol is bound to its current value at each tick.

## Built-in reference

| Area | Names |
|---|---|
| Trigonometric | `Sin` `Cos` `Tan` `Cot` `Sec` `Csc` `ArcSin` `ArcCos` `ArcTan` `Sinh` `Cosh` `Tanh` |
| Exponential | `Exp` `Log` `Sqrt` `Abs` |
| Numeric | `N` `Floor` `Ceiling` `Round` `Min` `Max` |
| Constants | `Pi` `E` `Degree` `Infinity` |
| Structure | `List` (`{...}`) `Table` `Range` `Length` `Map` |
| Definitions | `=` `:=` `Clear`, patterns `x_` |
| Algebra | `Expand` `Factor` `Simplify` `Solve` |
| Calculus | `D` `Integrate` (indefinite and definite) |
| Numerics and graphics | `Plot` `ListPlot` `NDSolve` |
| Relations and rules | `==` `->` |

## Where OpenMat differs from Mathematica today

Honest edges of the v0.01 kernel, so nothing surprises you:

- `Solve` stops at quadratics. Cubics and beyond stay unevaluated.
- `NDSolve` handles one scalar equation, first or second order. No coupled systems yet.
- `Simplify` is structural only; trig identities and radical simplification are not applied.
- No `%` output history, no `/.` (ReplaceAll) operator, no `Sum`/`Product` evaluation, no strings-and-formatting layer.
- Arithmetic does not thread over lists.
- Exact rational arithmetic is preserved but not folded (`1/3 + 1/6` stays symbolic; apply `N` for a decimal).
- Unknown functions are left symbolic rather than erroring, exactly as in Mathematica: `h[2]` evaluates to `h[2]` until `h` gets a definition.

Everything listed in the sections above works today, in the desktop app and in the browser build, on the same Rust kernel.
