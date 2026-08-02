# OpenMat Architecture

Decisions from specs/ and the recorded team interview (specs/feature-interview.md).

## Layout

- `crates/openmat-core`: expression kernel. Expr type, WL-subset parser, evaluator with Hold attributes, basic pattern matching, LaTeX rendering of expressions.
- `crates/openmat-solve`: numeric ODE solving. `OdeSolver` trait; pure-Rust Dormand-Prince RK45 backend (always available, WASM-safe); SUNDIALS CVODE backend behind the `sundials` cargo feature (desktop default).
- `crates/openmat-kernel`: facade tying core and solve together. Parses input, evaluates, dispatches NDSolve to a solver, returns render-ready results. This is the only crate the app talks to.
- `app/`: Tauri 2 desktop app. React + TypeScript UI: notebook cells, MathLive 2D input, KaTeX typeset output, SVG plots, Manipulate slider.

## App-to-kernel contract

The UI calls one Tauri command:

```
evaluate(input: string) -> EvalResult

EvalResult {
  latex: string,            // typeset form of the result expression
  plot?: {                  // present when the result is plottable
    curves: { points: [number, number][], label?: string }[],
    x_range: [number, number],
    y_range: [number, number],
  },
  error?: string,           // parse or evaluation failure, human readable
}
```

Input is WL-shaped linear syntax, e.g.
`NDSolve[{x''[t] + c x'[t] + Sin[x[t]] == 0, x[0] == 2, x'[0] == 0}, x, {t, 0, 20}]`.
The Manipulate slider re-issues `evaluate` with the parameter substituted. MathLive input cells translate their LaTeX to linear syntax before calling `evaluate`.

Until integration, the UI mocks `evaluate` behind the same TypeScript interface.

## Flagship demo (gates the MVP)

Slider-driven damped pendulum: 2D-input equation cell, slider for damping coefficient c, NDSolve re-solves and the solution curve re-renders live. Ships only when 2D input works end to end (team decision, round 4).

## Style rules

- No em or en dashes anywhere in code, comments, docs, or UI strings.
- Plain direct language in docs.
- Dual licensed MIT OR Apache-2.0; GPL-family code (Mathics3, Symja) must not be copied, only studied.
