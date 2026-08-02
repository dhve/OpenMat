# M0: the damped-pendulum milestone

Version 0.1. This is the single authoritative MVP scope, replacing the three per-spec MVP slices (which remain as research context, marked superseded). Scope rule: a capability is in M0 only if the flagship demo path depends on it. Everything else moves to named later milestones.

The flagship demo, from the feature interview: open OpenMat, see the Damped Pendulum notebook, edit the equation in structural 2D input, drag the damping slider, watch the solution curve re-solve and re-render live, close the app, reopen it, and find the notebook as you left it.

## M0 capabilities and acceptance criteria

Each capability ships with executable checks. "Reference machine" is an Apple Silicon MacBook (the team's dev hardware) until CI defines one.

| # | Capability | Acceptance criteria |
|---|---|---|
| 1 | Structural 2D input for the supported box model (identifiers, numbers, arithmetic, fractions, powers, derivatives, function application, equations, lists, grouping) | Every form in the box model has a MathLive-to-linear-syntax conversion fixture; the vitest translator suite covers all of them and passes. Incomplete input stays editable and is never sent to the kernel as a valid expression. |
| 2 | Parse into the canonical expression model | Every form in grammar.md has parse, print, and round-trip fixtures in the conformance suite (openmat-core tests); parse errors carry position info and a stable shape. |
| 3 | Typed slider binding (no textual substitution) | The Manipulate parameter is bound by symbol through an explicit bindings argument to the kernel; the pendulum cell is parsed once per edit, not once per slider tick. Requests carry monotonically increasing IDs; stale results are discarded (latest-result-wins test). |
| 4 | Solve the supported ODE-IVP subset (scalar first and second order, linear in the highest derivative) | Harmonic oscillator within 1e-5 of analytic over [0, 20]; exponential decay within 1e-5 of analytic; both backends (CVODE, DP5(4)) agree to 1e-4; unsupported forms produce a clear typed error, not a crash. |
| 5 | Interpolated solution output | Output points are dense-output samples, evenly spaced over t_span, count independent of solver internal steps; verified by test. |
| 6 | Rendered 2D plot | 1-2-5 tick ladder fixture tests pass; curve is smooth at 400 points at cell width 640px; axes, gridlines, and legend render; no clipping of the padded y-range. |
| 7 | Reliable slider interaction | Drag end-to-end latency under 100 ms per re-solve on the reference machine for the pendulum at 400 points; debounced so intermediate positions coalesce; no result flicker from out-of-order responses (covered by the ID test in row 3). |
| 8 | Notebook save and reopen | Notebook serializes to a versioned format with a schema_version field; save-load-save round trip is byte-stable; reopening restores cells, slider value, and last outputs. Format documented before M1. |

Cross-cutting M0 budgets:

- Evaluator limits: iteration cap (4096), recursion cap, and a per-evaluation time budget of 5 s after which evaluation returns a typed timeout error.
- Error shapes: parser, evaluator, and solver errors are distinct typed variants at the kernel API; the UI renders each distinctly.
- Platform matrix: macOS arm64 is the M0 target; the pure-Rust solver path must also compile for wasm32 (build check only, no UI requirement).
- No em or en dashes in any shipped surface.

## Explicitly out of M0 (named milestones)

- M1 (language depth): Set/SetDelayed downvalues, Orderless/Flat pattern matching, Rational type, integer bignums, symbolic D.
- M2 (notebook depth): text/section cells with grouping, notebook-as-expression editing from the kernel, documentation viewer, cell-level reactive dependency graph beyond the single Manipulate cell.
- M3 (kernel service): Jupyter protocol adapter over the same kernel API, headless CLI runner, remote kernels.
- M4 (math breadth): Integrate via Rubi port, Solve, symbolic simplification depth, NIntegrate, Plot[] of arbitrary expressions with adaptive sampling and discontinuity detection.
- M5 (ecosystem): package mechanism, Python bridge, curated data functions.

Adaptive plot sampling note: M0 plots solver dense output only, so adaptive sampling and discontinuity fixtures (spec 02's flagged risk) belong to M4 with Plot[], not M0.
