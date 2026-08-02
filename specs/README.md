# OpenMat Specs

Feature and use-case specs for OpenMat, a fully open-source variant of Wolfram Mathematica. Each spec covers what Mathematica ships, the core use cases behind each feature, implementation difficulty, existing OSS building blocks to reuse, and an MVP priority call.

1. [Core language and symbolic engine](01-core-language.md): expression model, evaluation loop, pattern matching and rewriting, CAS (Simplify, Integrate, Solve), core data structures, exact numerics.
2. [Numerics, data, and visualization](02-numerics-data-viz.md): NDSolve, NIntegrate, optimization, the Plot and Graphics families, Import/Export, tabular data, stats and ML.
3. [Notebook front-end and ecosystem](03-notebook-ecosystem.md): notebook document model, Manipulate/Dynamic reactivity, kernel architecture, documentation system, packages, knowledge base, competitive landscape.
4. [Feature interview](feature-interview.md): recorded team interview settling wedge user, compatibility stance, kernel language, shell, solver sourcing, rendering scope, license, and execution plan. Authoritative over conflicting spec recommendations.
5. [M0 milestone](m0-milestone.md): the single authoritative MVP scope (damped-pendulum path) with measurable acceptance criteria. Supersedes the three per-spec MVP slices.
6. [Grammar and box conversion](grammar.md): normative v0.1 surface syntax, precedence, diagnostics, and MathLive conversion rules, with conformance fixtures in [fixtures/](fixtures/).

## Cross-cutting takeaways

- The highest technical risk is Orderless/Flat pattern matching in the evaluator; everything else in the CAS depends on it. Symja and expreduce are the best references.
- The highest product risk is polish, not algorithms: adaptive plot sampling, default aesthetics, runnable docs, and a convincing Manipulate demo are what make Mathematica feel like Mathematica.
- Most numerics are a wrapping job over LAPACK, SUNDIALS, QUADPACK, and SciPy. Rubi's rule set is the most valuable asset to port for symbolic integration.
- One transport-neutral kernel service with adapters: a local in-process adapter for the Tauri app first, a Jupyter protocol adapter for external clients later (see ARCHITECTURE.md). Notebooks as genuine kernel expressions from day one, custom front end and document format.
- Deliberate descopes: full Reduce-style quantifier elimination, Wolfram-style significance arithmetic, general-geometry PDEs, native deep learning, and the full Wolfram Knowledgebase (use Wikidata plus small vendored datasets instead).
- The cautionary lesson from Mathics3: win a narrow slice convincingly instead of chasing full function-count parity.
