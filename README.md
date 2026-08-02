# OpenMat

A fully open-source variant of Wolfram Mathematica: a WL-shaped language kernel in Rust, a numeric solving layer, and a desktop notebook with structural 2D math input and live Manipulate-style interactivity.

Status: pre-M0, in active development. The first release gate is the flagship demo: open the Damped Pendulum notebook, edit the equation in 2D input, drag the damping slider, and watch the ODE re-solve and re-render live. See [specs/m0-milestone.md](specs/m0-milestone.md) for the authoritative scope and acceptance criteria.

## Layout

| Path | What it is |
|---|---|
| `crates/openmat-core` | Expression kernel: WL-subset parser, Hold-attribute evaluator, pattern matching, LaTeX rendering. Zero dependencies. |
| `crates/openmat-solve` | ODE solving: `OdeSolver` trait, pure-Rust Dormand-Prince RK5(4) (WASM-safe), SUNDIALS CVODE behind the default `sundials` feature. |
| `crates/openmat-kernel` | The kernel service: parses, evaluates, dispatches NDSolve, returns structured results. Transports are adapters over this API. |
| `app/` | Tauri 2 desktop notebook: React/TypeScript, MathLive 2D input, KaTeX output, SVG plots, Manipulate slider. |
| `specs/` | Research specs, the recorded feature interview, the M0 milestone, and the normative grammar with conformance fixtures. |

Architecture and contracts: [ARCHITECTURE.md](ARCHITECTURE.md). Grammar: [specs/grammar.md](specs/grammar.md).

## Building

Rust workspace (needs cmake for the vendored SUNDIALS build):

```bash
cargo test --workspace
```

Pure-Rust solver path (what the future browser build uses):

```bash
cargo test -p openmat-solve --no-default-features
```

App (needs Node 20+):

```bash
cd app && npm install && npm test -- --run && npm run build
```

Run the desktop app in dev mode:

```bash
cd app && npm run tauri dev
```

## Design ground rules

- WL-shaped, core-semantics compatible; no promise that arbitrary Mathematica notebooks run. Win a narrow slice convincingly.
- The kernel service is transport-neutral; the Tauri app and the future Jupyter adapter are thin transports.
- Polish is a feature: plot aesthetics, typeset quality, and docs are release criteria, not afterthoughts.
- GPL-family code (Mathics3, Symja) is studied, never copied.

History: the specs grew out of a recorded team interview and an external spec review ([discussion](https://chatgpt.com/share/6a6f6de4-81ac-83ea-9360-ac878ba80737), resolved in issue #1).

## License

Dual-licensed under MIT or Apache-2.0, at your option.
