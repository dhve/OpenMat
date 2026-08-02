# OpenMat Architecture

Decisions from specs/ and the recorded team interview (specs/feature-interview.md). The authoritative MVP scope is specs/m0-milestone.md; the surface grammar is specs/grammar.md. Where this document and an older spec section disagree, this document and the interview win (issue #1).

## Layout

- `crates/openmat-core`: expression kernel. Expr type, WL-subset parser (specs/grammar.md), evaluator with Hold attributes, basic pattern matching, LaTeX rendering.
- `crates/openmat-solve`: numeric ODE solving. `OdeSolver` trait; pure-Rust Dormand-Prince RK5(4) backend (always available, WASM-safe); SUNDIALS CVODE backend behind the `sundials` cargo feature (desktop default).
- `crates/openmat-kernel`: the kernel service. Owns evaluation semantics and result formatting. Everything above it is a transport adapter.
- `app/`: Tauri 2 desktop app. React + TypeScript UI: notebook cells, MathLive 2D input, KaTeX typeset output, SVG plots, Manipulate slider.

## Kernel service and transport adapters

The kernel facade (`openmat-kernel`) is transport-neutral. Adapters expose it without owning any evaluator state or result formatting:

- Local adapter (M0): the Tauri command layer in `app/src-tauri`, calling the kernel in-process for low latency.
- Jupyter protocol adapter (M3): the same kernel API over ZeroMQ for Jupyter clients, VS Code, remote and headless use.

Neither adapter interprets expressions or reformats results beyond serialization. Anything a transport needs (request IDs, cancellation) is part of the kernel API so every adapter gets identical semantics.

## Kernel API

Two entry points:

```
evaluate(input: string, request_id: u64) -> KernelResult
evaluate_with_bindings(input: string, bindings: {symbol: number}, request_id: u64) -> KernelResult
```

`KernelResult` is structured data, not presentation:

```
KernelResult {
  request_id: u64,          // echoed; transports enforce latest-result-wins
  status: "ok" | "error",   // terminal status, mutually exclusive
  input_form: string?,      // canonical InputForm of the evaluated expression (ok only)
  displays: Display[],      // derived presentations, zero or more
  messages: Message[],      // warnings/notes; may accompany ok
  error: KernelError?,      // set iff status == "error"
}
Display  = { kind: "latex", latex: string }
         | { kind: "plot", curves: {points: [number, number][], label: string?}[],
             x_range: [number, number], y_range: [number, number] }
Message  = { severity: "warning" | "note", text: string }
KernelError = { kind: "parse" | "eval" | "solve", message: string, position: number? }
```

LaTeX, plots, and plain text are derived representations; the expression itself (as `input_form`, and later a serialized AST) is the canonical payload. Error results carry no displays; messages may accompany success. Timing and cancellation metadata attach here when async evaluation lands (M3); within M0 evaluation is synchronous per request and the transport discards stale responses by `request_id`.

## Manipulate: typed bindings, not text substitution

Slider values are never substituted into source text. The app parses a cell once per edit, then re-evaluates through `evaluate_with_bindings` with the slider's parameter passed as a typed binding (e.g. `{c: 0.5}`). The kernel binds the symbol during evaluation. The transport layer assigns monotonically increasing request IDs, debounces/coalesces slider movement, and drops stale results so the newest binding always wins. Dependency tracking is cell-level in M0; sub-expression Dynamic is post-MVP.

## NDSolve input shape

WL-shaped linear syntax per specs/grammar.md, e.g.
`NDSolve[{x''[t] + c x'[t] + Sin[x[t]] == 0, x[0] == 2, x'[0] == 0}, x, {t, 0, 20}]`
with `c` supplied via bindings. MathLive input cells translate their LaTeX to linear syntax before calling the kernel (specs/grammar.md section 6).

## Flagship demo (gates M0)

Slider-driven damped pendulum: 2D-input equation cell, slider for damping coefficient c, NDSolve re-solves and the solution curve re-renders live, notebook saves and reopens. Ships only when 2D input works end to end (team decision, interview round 4). Acceptance criteria: specs/m0-milestone.md.

## Style rules

- No em or en dashes anywhere in code, comments, docs, or UI strings.
- Plain direct language in docs.
- Dual licensed MIT OR Apache-2.0; GPL-family code (Mathics3, Symja) must not be copied, only studied.
