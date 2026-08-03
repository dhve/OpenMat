# OpenMat

## What it is

A fully open-source variant of Wolfram Mathematica: a WL-shaped symbolic kernel written in Rust, a Mathematica-style desktop notebook (Tauri), and the same kernel compiled to WebAssembly (530 KB) for the browser. Natural language is a first-class input: one sentence can generate a whole evaluated notebook.

## Unique mechanism

One Rust kernel runs everywhere (native and WASM), and the notebook writes itself from plain English: type `=` in a cell or use the docked language bar, and titles, prose, equations, sliders, and live plots materialize and evaluate in place. Works with the user's own Anthropic API key or a fully local Ollama model, so the whole stack can run offline.

## Audience

Students, educators, and developers who want Mathematica's notebook experience without the license: calculus and physics coursework, ODE exploration, quick symbolic computation. They know what Mathematica looks like; visual fidelity to that experience is part of the promise.

## Capabilities (truthful, current)

- Kernel: arithmetic, simplification, `D`, `Integrate` (indefinite + definite with trig rules and numeric fallback), `Solve` (through quadratics), `Expand`, `Factor`, `Simplify`, `Table`; persistent session (`a = 5`, `f[x_] := x^2` persist until `Clear`)
- `Plot`/`ListPlot` with adaptive sampling, legends, discontinuity splitting; `NDSolve` for scalar first/second-order ODEs; Manipulate sliders re-solve live on drag
- Notebook: 2D math input (MathLive), KaTeX output, In/Out numbering, collapsible right-edge cell brackets, Title/Section/Text cells, save/open, math symbol keyboards
- Natural language: `=` prefix cells and the docked bar; generates multi-cell notebooks with sliders
- Limits (do not overclaim): no coupled ODE systems, no parametric plots, Solve is quadratic-level, macOS Apple Silicon build only, unsigned

## Facts

- Repo: https://github.com/dhve/OpenMat
- Release: v0.01, dmg at https://github.com/dhve/OpenMat/releases/download/v0.01/OpenMat_0.1.0_aarch64.dmg
- License: dual MIT / Apache-2.0
- Site: https://openmat.tools
- Tagline: "The open-source, local-first, AI-native alternative to Mathematica."
- Positioning: the three pillars are open source (MIT/Apache, kernel included), local-first (kernel runs on the user's machine, no server, notebooks stay local), and AI-native (natural language as a built-in input mode, user's own key or fully local model). "AI-native" is brand language for marketing surfaces; inside the product the feature is still called natural language input.

## Brand commitments

The product's own visual world is the incumbent identity for every surface: white paper ground, black ink, Times-family serif for titles (the Mathematica notebook voice), Courier-family mono for In/Out labels, classic blue #3b5fa4 accent, burnt orange #c1652c reserved for natural language affordances, flat borderless cells marked by right-edge brackets. Surfaces extend this world; they do not invent a new one.

## Writing rules

Plain, direct sentences. Keep technical detail. Never use em or en dashes. No AI branding in product surfaces; the feature is called natural language input.
