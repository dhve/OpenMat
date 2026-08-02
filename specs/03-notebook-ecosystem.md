# OpenMat Spec 03: Notebook Front-End, Interactivity, and Ecosystem

Scope: the notebook document model, dynamic interactivity, front-end/kernel architecture, documentation system, ecosystem/deployment, the curated knowledge-base question, and the competitive landscape. This document stays out of core symbolic engine and numerics/visualization internals, which other specs own.

---

## 1. The notebook document model

**What it is and why it matters**
A Mathematica notebook is not a text file with output appended below it. It is itself a Wolfram Language expression: a nested tree of `Cell[...]` objects with styles, grouping, and metadata, stored as `Notebook[{cell, cell, ...}, options]`. Because the document is data in the same language the kernel evaluates, the front end can programmatically read, write, and restyle any part of a notebook, and notebooks can generate or modify other notebooks. This is the single biggest structural difference from Jupyter's `.ipynb` (a JSON blob the kernel never sees as a native data type) and it is what enables things like dynamically-generated reports, self-modifying tutorials, and slide shows built from the same document.

**Top use cases**
- Structuring a long derivation or report with Title/Section/Subsection/Text/Input/Output cells that auto-collapse into an outline, the way a Word doc outline works but computable.
- Mixing prose, math, and live code in one linear document for teaching or reports, then hiding all "Input" cells to present just narrative and results.
- Building "cell groups" that batch-evaluate as a unit (e.g. a section that sets up a problem, then shows the solution) and can be collapsed/closed like an accordion.
- Programmatically generating notebooks from code (`CreateDocument`, `NotebookPut`) e.g. auto-generating a report notebook per data batch, or a course generating one notebook per student.
- Round-tripping: selecting a cell and choosing "Show Expression" to see and edit its raw symbolic form, useful for scripting notebook transformations.

**Implementation difficulty**
- Cell types, grouping, basic input/output/text cells: **medium**. This is UI/data-model plumbing, well understood, comparable to building a rich-text/outliner editor (Notion, Obsidian).
- Notebook-as-expression (the front end and kernel sharing one data model, kernel can manipulate its own notebook) is **hard**. It requires the kernel's expression representation and the front end's document representation to genuinely be the same tree, not two synced-but-separate formats. Get this wrong (as most notebook clones do) and you lose the property that makes Mathematica notebooks special: everything downstream (styling, dynamic content, templates) depends on it.
- 2D typeset math input/output (fractions, integrals typed visually, not LaTeX-then-render): **hard**. This needs a real structural math editor with cursor navigation through nested boxes (numerator/denominator, sub/superscript, matrix cells), undo/redo at the box level, and a parser that converts the box structure to/from an internal expression (`FractionBox`, `SuperscriptBox`, etc.) losslessly.
- Free-form linguistic input (type English, Wolfram|Alpha-style parsing turns it into code): **research-grade**. This depends on a large curated NLP-to-symbolic pipeline tied to Wolfram|Alpha's backend; not something an OSS clone can replicate without years of curated grammar/ontology work, or without leaning on an LLM (a plausible modern substitute, but a different mechanism with different failure modes).

**OSS building blocks to reuse**
- ProseMirror or Lexical (rich structured-document editors with node/mark models) for the cell-tree/outline layer, since both support arbitrary nested block types and structural editing similar to what cell grouping needs.
- MathQuill or MathLive for structural 2D math input (both are mature, actively maintained, LaTeX-in/LaTeX-out, keyboard-navigable box editors); MathLive is the stronger pick today (accessibility, virtual keyboard, custom macros).
- KaTeX or MathJax for fast static math rendering of output.
- CodeMirror 6 for the plain-text code-input cell type and syntax highlighting of the OpenMat language.
- For "notebook as expression," there's no off-the-shelf answer; this has to be designed as part of the core language spec (coordinate with the CAS-engine agent so `Cell`/`Notebook` are first-class expression heads the evaluator understands).

**MVP priority**
- Cell types (Input/Output/Text/Section), linear notebook, save/load: **must-have**.
- Cell grouping/outlining, collapsible sections: **should-have**.
- Notebook-as-expression (kernel can read/write its own doc structurally): **should-have** for v1, but architecturally must be decided on day one even if the full feature ships later, because retrofitting it is very costly.
- Structural 2D math input: **must-have, release gate** (superseded by the feature interview, round 4: the flagship demo does not ship until MathLive-based structural input works end to end; the initial supported box model is limited to the forms the flagship demo needs: identifiers, numbers, arithmetic, fractions, powers, derivatives, function application, equations, lists, grouping).
- Free-form linguistic input: **later**, and probably reframed as "ask an LLM to write Wolfram-Language-like code," not a reimplementation of Wolfram|Alpha's NLP.

---

## 2. Dynamic interactivity: Manipulate and Dynamic

**What it is and why it matters**
`Dynamic[expr]` wraps any expression so the front end re-evaluates and re-displays it automatically whenever any symbol it reads changes, no explicit callback wiring required. `Manipulate[expr, {var, min, max}]` is a one-line macro that builds `Dynamic` content plus auto-generated sliders/controls bound to that content. The dependency tracking is automatic and fine-grained: each `Dynamic` region tracks exactly which variables it read on its last evaluation and only that region refreshes, not the whole notebook. This turns "build an interactive parameter-exploration UI" from a multi-file app (state store + view + event handlers) into a single expression, and it is widely cited as Mathematica's signature "wow" feature for classroom demos, live derivations, and quick model exploration.

**Top use cases**
- Classroom demo: `Manipulate[Plot[Sin[a x], {x,0,10}], {a,1,5}]` gives an instantly draggable-parameter plot, no app-building.
- Exploring a dataset or model: bind sliders to hyperparameters and watch a fit or plot update live, used constantly for "what if I change X" exploration during analysis.
- Building lightweight interactive tools (unit converters, quiz widgets, simple simulations) that can be shared as a single file or deployed to a browser via CDF/cloud embedding, no separate front-end app needed.
- Locator controls: drag a point on a graphic to interactively define geometry (e.g. dragging a point to redefine a triangle and watch its area recompute).
- 3D manipulation: rotate/orbit a 3D plot with the mouse, with `Dynamic` content (e.g. a cross-section) updating live as the viewpoint changes.

**How the reactive model compares**
- Jupyter's `ipywidgets.interact` is the closest analog: it auto-generates controls from a function's arguments and re-runs the function on change. It is comparably terse for the simple case but the dependency model is coarser (the whole decorated function reruns; there isn't fine-grained sub-expression tracking the way `Dynamic` has it, and mixing multiple independently-updating dynamic regions in one output is harder).
- Pluto.jl (Julia) is architecturally the more interesting comparison: it does static dependency-graph analysis of the whole notebook (not just one interactive region) so any cell that depends on a changed variable anywhere in the notebook re-runs automatically, and this reactivity is also how its `@bind` sliders work. This is actually closer in spirit to `Dynamic`'s "track dependencies, refresh only what changed" model than ipywidgets is, but it operates at the cell/variable level across the whole notebook rather than at the sub-expression level within a single `Dynamic` box.
- Observable notebooks use the same dataflow-graph idea in JavaScript: cells are dataflow nodes, and the runtime topologically re-evaluates dependents when an input cell changes.
- The distinctive part of Wolfram's design is that `Dynamic` works at arbitrary granularity anywhere in an expression, not just at the cell level, and composes: you can have many independent `Dynamic` regions inside one cell's output, each tracking its own dependencies, updating independently and cheaply. That is more powerful than both the ipywidgets model (function-level) and the Pluto/Observable model (cell-level), though it is also why it is harder to implement correctly (it needs the front end's rendering tree to itself carry live dependency edges, not just the notebook's cell graph).

**Implementation difficulty**
- A Pluto/Observable-style cell-level reactive graph (static analysis of variable reads/writes per cell, topological re-execution): **medium**. This is now a well-trodden design with reference implementations (Pluto.jl's `ExpressionExplorer` + `PlutoDependencyExplorer` is open source and readable).
- Fine-grained `Dynamic`-in-an-expression tracking (multiple independent live regions inside one output, not just whole-cell reactivity): **hard**. Needs runtime dependency tracking (not just static analysis) because in a dynamic language what a piece of code reads can depend on control flow, and it needs the rendering layer to know how to patch just the changed sub-region.
- Manipulate itself (auto-generating sliders/controls from a variable-spec syntax and wiring them to a Dynamic body): **medium**, once Dynamic exists underneath, since it is mostly UI generation and control-widget code.
- Controls: sliders/checkboxes/dropdowns are **easy**; locators (draggable points on a graphic, with hit-testing and coordinate mapping) are **medium**; smooth 3D rotation with live dynamic content updating during the drag (not just after mouse-up) is **medium-hard**, mainly a performance problem (need to keep frame rate acceptable while re-evaluating).
- Animation (`Animate`, play/pause/loop controls that step a variable over time): **easy** once Manipulate/Dynamic exist, it is a timer driving the same update mechanism.

**OSS building blocks to reuse**
- Pluto.jl's `ExpressionExplorer`/reactivity engine as the reference design for static dependency analysis (Julia, but the algorithm ports); this is the most directly reusable prior art for "automatic dependency tracking without an app framework."
- A signals/reactive-state library (Solid.js, Preact Signals, or a custom fine-grained reactive core) for the `Dynamic`-region-level tracking in the front end, since this is exactly the problem client-side reactive UI frameworks solve.
- Three.js for 3D plot rendering and rotation controls.
- Standard web form controls plus a canvas/SVG hit-testing layer for locators.

**MVP priority**
- Cell-level reactivity (Pluto-style): **must-have**. It is the foundation and is achievable with known techniques.
- `Manipulate`-equivalent with sliders bound to plots: **must-have**. This is the single highest-leverage teaching/demo feature and the one users will judge the whole project by.
- Fine-grained sub-expression `Dynamic` (multiple independent live regions per cell): **should-have**, ship after coarse cell-level reactivity works.
- Locators, 3D live-rotation updates, animation: **should-have**.
- Full parity with advanced `Manipulate` options (custom control layout, `TrackingFunction`, action-delay tuning): **later**.

---

## 3. Front-end/kernel architecture

**What it is and why it matters**
Mathematica strictly separates the front end (the notebook UI, a big C++/Qt-like application) from the kernel (the language evaluator) and connects them over WSTP (formerly MathLink), a binary/text wire protocol for passing expressions and requests back and forth. This separation is why one front end can drive multiple kernels (local, remote, grid), why headless kernel use (scripts, servers) works without any UI, and why third-party programs can talk to the kernel at all. For an OSS project this is the single most consequential architecture decision, because it determines what protocol other tools, editors, and services can plug into.

**Top use cases**
- Headless batch/server use: run the kernel with no front end at all for CI jobs, cron scripts, or web APIs (`wolframscript`, `math -script`).
- Multiple front ends against one kernel style: same evaluator power available from a desktop notebook, a Wolfram Cloud web notebook, and a text console.
- Remote/parallel kernels: front end on a laptop, kernel(s) running on a cluster or cloud machine, transparently over the same protocol (this is what `LaunchKernels`/parallel computing and grid Mathematica are built on).
- Third-party integration: other languages and tools embed a Mathematica kernel as a computation engine via the WSTP C API, without needing to reimplement any math.

**What a modern OSS equivalent should use**
- Adopt the **Jupyter kernel protocol** (ZeroMQ transport, JSON messages over shell/iopub/stdin channels, plus the `comm` sub-protocol for custom widget-like bidirectional messages) as the wire protocol, rather than inventing a new one. Reasoning: it is open, documented, has mature client and server libraries in every major language, and immediately makes an OpenMat kernel usable from JupyterLab, VS Code's notebook UI, nteract, and any other Jupyter-protocol client with zero extra work. Inventing a WSTP-alike from scratch would buy nothing (WSTP's main virtue, native binary expression transfer, matters less when your kernel is likely implemented in Python/JS/a JIT anyway) and would forfeit the entire existing Jupyter tooling ecosystem.
- The `comm` channel is the piece to lean on hardest for the interactivity story (section 2): it is exactly the mechanism ipywidgets uses for bidirectional kernel<->front-end state sync, and it is flexible enough to carry OpenMat's `Dynamic`/reactive updates too, so a custom OpenMat front end and a plain JupyterLab session could both drive the same kernel, with the custom front end getting the fuller `Dynamic`/notebook-as-expression experience and JupyterLab getting a reduced-but-functional one.
- Still build a **custom, purpose-built front end** on top of that protocol (not just "use Jupyter's classic notebook UI"), because cell grouping, 2D math input, and notebook-as-expression are UI/document-model features Jupyter's own front end does not have and its `.ipynb` format cannot naturally represent. Jupyter protocol for the wire, custom front end for the document model and interactivity layer, is the right split.
- Keep the kernel able to run fully headless (script mode) from day one, since that is cheap once the protocol exists and unlocks CI/server/CLI use immediately.

**Implementation difficulty**
- Standing up a Jupyter-protocol-compliant kernel wrapper around a custom evaluator: **easy-medium**. This is a well-documented integration task (see ipykernel, xeus-based kernels, and dozens of language kernels already built this way).
- A custom front end that speaks Jupyter protocol but renders OpenMat's richer document/cell model: **medium-hard**, this is most of the actual front-end engineering effort in the whole project.
- Multi-kernel / remote-kernel support: **medium**, this mostly falls out of the protocol choice (Jupyter already supports connecting to remote kernel gateways).
- A WSTP-equivalent low-level binary protocol for embedding-in-other-languages use cases: **later/skip**; cover that need instead via a documented HTTP/WebSocket API and language-specific client libraries, which is cheaper to build and matches how most modern tools integrate anyway.

**OSS building blocks to reuse**
- `jupyter_client`/the Jupyter messaging spec directly (protocol definition, reference implementations in Python).
- ZeroMQ (pyzmq, zeromq.js, etc.) for transport.
- xeus (C++ framework for building Jupyter kernels in native languages) if the evaluator ends up native/compiled rather than Python-hosted.
- JupyterLab's extension APIs as a fallback distribution channel (ship an OpenMat JupyterLab extension for users who want partial functionality inside existing Jupyter installs, even while the flagship experience is the custom front end).

**MVP priority**
- Jupyter-protocol-speaking kernel + headless script mode: **must-have**.
- Custom front end talking that protocol: **must-have** (this is the product).
- `comm`-based sync for interactivity: **must-have**, needed for section 2 to work at all.
- Remote/multi-kernel support: **should-have**.
- JupyterLab extension as an alternate thin client: **later**, nice adoption lever once the core exists.

---

## 4. Documentation system

**What it is and why it matters**
Every built-in Wolfram Language function has a "reference page": a standardized document with the function's syntax forms, a plain-language description, argument details, options, and, critically, worked examples that are live, runnable, and editable inside the docs themselves (copy an example cell into your own notebook and it just runs). This consistency plus runnability is repeatedly cited as the reason Wolfram documentation is considered the best in the industry: users do not read API docs and then go write code in a separate window, they read docs that already are working code. For an OSS project, mediocre docs are one of the most common reasons a technically-good alternative fails to get adopted, so this is not a "nice to have" polish item.

**Top use cases**
- Look up a function you half-remember, get the signature, options, and a two-line example you can paste and run immediately, without leaving the notebook.
- Discover related functionality via "See Also" and guide-page links (browsing from one function to a whole topic area, e.g. from `Plot` to the "Plotting" guide page listing every related function).
- Learn a topic top-down via tutorial pages that read like a mini-textbook chapter and link back into the reference pages for the functions used.
- Copy a worked example wholesale as a starting template for your own code, relying on the fact that it is guaranteed to actually run as shown.
- In-notebook contextual help: hover or use a keyboard shortcut on a function name while coding to get an inline summary without a context switch.

**What an OSS variant needs**
- A documentation format that stores examples as real, executable notebook cells (not markdown code blocks that silently rot), with a CI job that re-runs every example on every release and fails the build if an example errors or its output changes unexpectedly. This is the mechanism that keeps docs trustworthy over time and is arguably more important long-term than the initial writing effort.
- Consistent structural template per function: signature line(s), one-sentence description, "Details," "Examples: Basic Usage," "Options," "See Also," "Related Guides." Predictability lets users pattern-match across the whole doc set instead of parsing prose each time.
- Docs addressable and renderable both inside the notebook front end (so `?FunctionName` or a help pane shows the real page, live and runnable) and as a static website (for search engines, non-notebook browsing, linking from GitHub issues, etc.) generated from the same source.
- Full-text and fuzzy search across function names, descriptions, and even example content, fast enough to use as a "just start typing" launcher.

**Implementation difficulty**
- The doc template/renderer itself (structured page with sections, static site generation): **easy-medium**, this is a solved problem (Docusaurus, mkdocs, Sphinx all do this class of thing).
- Making examples "live" inside the notebook front end (an example cell in the docs is a real evaluable cell, not a screenshot or a static code block): **medium**, needs the docs viewer to be built on the same cell-rendering component as the main notebook, which argues for building the front end's cell renderer as a reusable component from day one.
- CI-verified examples (every example actually executes correctly on every build): **medium**, mostly an engineering-discipline problem, requires a stable test harness and is worth doing early since it is much more expensive to retrofit onto thousands of already-written examples later.
- Achieving Wolfram's actual depth and volume (thousands of functions x several examples each, all curated and cross-linked) is **hard**, not because any one page is technically hard, but because it is a massive, ongoing content-authoring effort, arguably comparable in scope to writing the functions themselves. This is a place where an OSS project can realistically fall behind for years; docs quality should be treated as a first-class, continuously-funded workstream, not a launch checkbox.

**OSS building blocks to reuse**
- Docusaurus or Starlight (Astro) for the static-site half of doc rendering, both support MDX/custom components which can embed the same live-cell React/web component used in the notebook.
- The notebook's own cell-rendering component, reused inside the docs site so "live runnable example" is literally the same code path as a notebook cell, not a reimplementation.
- A CI example-runner script (send each example's input to a headless kernel over the Jupyter protocol from section 3, diff the output) is straightforward to build in-house; no existing tool does the Mathematica-specific "runnable doc example" pattern off the shelf, though Jupyter's `nbval`/doctest-style tools are a useful reference.
- Typesense or Meilisearch for fast fuzzy full-text search over the doc corpus.

**MVP priority**
- Structured reference pages with runnable examples, rendered in the front end (`?FunctionName` opens a real page): **must-have**. Ship this thin (fewer functions, but each one done to the full standard) rather than broad-and-shallow.
- CI verification that examples still run/produce expected output: **must-have**, set this up before the doc corpus grows, not after.
- Static doc website generated from the same source: **should-have**.
- Guide pages / topic browsing, tutorial-style long-form docs, full-text search: **should-have**.
- Community-contributed doc examples / a "did this answer your question" feedback loop: **later**.

---

## 5. Ecosystem and deployment

**What it is and why it matters**
Beyond the core language, Wolfram built a full distribution and deployment stack: paclets (versioned installable packages, roughly Mathematica's npm/PyPI unit) served from a central Paclet Repository; the Wolfram Function Repository (community-submitted single functions, lower friction than a full paclet); `CloudDeploy`/`APIFunction` for turning any expression into a hosted web API or interactive web form with one call; `wolframscript` for CLI/script execution; and bridges out to other ecosystems (`ExternalEvaluate` for calling Python/R/Julia/etc. from inside the language, `LibraryLink` for calling compiled C, historically `J/Link` for Java). This is what makes the language useful beyond a closed sandbox: install other people's code, publish your own, ship a computation as a web service, and interoperate with the rest of the software world.

**Top use cases**
- Install a community package for a specialized domain (e.g. a chemistry toolkit) with one line, the way `pip install` or `npm install` works, instead of manually vendoring files.
- Publish a single useful function to a public repository so anyone can call it by name without installing anything, lowering the bar for sharing small utilities (this is exactly the Wolfram Function Repository's pitch and it meaningfully increased community code-sharing versus the heavier full-paclet path).
- Turn a written computation into a shareable web app or API in minutes (`CloudDeploy[APIFunction[...]]`) without writing any web framework code, useful for quick internal tools, teaching demos, or REST endpoints wrapping a model.
- Run a script headlessly from a shell or cron job (`wolframscript -file foo.wls`), for automation and CI use.
- Call out to Python for a library that only exists there (e.g. a specific ML framework) without leaving the notebook, keeping OpenMat as the orchestration layer rather than forcing a full rewrite.
- Call fast compiled C code for a performance-critical inner loop via a native extension mechanism, without embedding a whole foreign build toolchain in the main language.

**Implementation difficulty**
- A package/module system with versioning and a lockfile-style dependency resolution: **medium**, this is well-trodden ground (npm, pip/uv, Cargo are all reference designs); the main work is picking one and not reinventing package-manager mistakes that other ecosystems already fixed.
- A central hosted repository (packages) with a submission/review pipeline: **medium**, mostly infrastructure and moderation policy, not novel engineering; can bootstrap on GitHub + a generated index before building bespoke hosting.
- A "Function Repository"-style low-friction single-function sharing mechanism: **easy-medium** once the package system exists, it is a thin, more constrained variant of the same publish flow.
- `CloudDeploy`/`APIFunction`-equivalent (one call turns a function into a hosted HTTP endpoint): **medium**, needs a hosting/runtime backend (turn the expression into a request handler, manage auth/URLs/scaling) but the request/response shape itself is simple; the main cost is operating the hosting service, not the language feature.
- `wolframscript`-equivalent CLI: **easy**, a thin wrapper once headless kernel mode exists (section 3).
- `ExternalEvaluate`-equivalent bridges to Python/R/Julia: **medium** per language, this is subprocess/protocol management (spawn the other language's runtime, marshal values across, keep a persistent session); Python is the highest-value target and should come first, worth noting Wolfram's own implementation already leans on exactly this subprocess-plus-marshaling pattern.
- `LibraryLink`-equivalent (call compiled C/native code from the language, e.g. for performance-critical extensions): **hard**, needs a stable ABI/FFI story, memory-safety discipline at the boundary, and cross-platform build tooling; likely lower priority than the scripting-language bridges since it serves a narrower "extension author" audience rather than everyday users.

**OSS building blocks to reuse**
- npm/Cargo/uv as design references for the package manager (semver, lockfiles, registries); consider directly building on an existing registry technology (e.g. a package.json-like manifest plus a lightweight custom index) rather than inventing new tooling.
- GitHub (or a GitHub-backed index) as the initial "repository" backend, exactly how many young language ecosystems (Deno's early module registry, Homebrew) bootstrapped before building dedicated infrastructure.
- Standard web frameworks (any HTTP server library in the kernel's implementation language) plus a container/serverless backend (Cloud Run, Fly.io, etc.) for the CloudDeploy-equivalent, rather than building custom cloud infrastructure from scratch.
- Existing subprocess/FFI bridge patterns: Python's `subprocess`+JSON-RPC style bridges, or reticulate (R-to-Python) as a design reference for `ExternalEvaluate`-style bridges.
- A stable C FFI layer (many languages already have one, e.g. Python's `ctypes`/`cffi`, Node's N-API) as the base to build a `LibraryLink`-equivalent on rather than designing a new ABI.

**MVP priority**
- Basic package/module install-and-import mechanism: **must-have**, users cannot build an ecosystem without it.
- `wolframscript`-equivalent CLI and headless execution: **must-have**, cheap and unlocks automation/CI use immediately.
- `ExternalEvaluate`-equivalent Python bridge: **should-have**, high leverage (unlocks the entire Python numerical/ML ecosystem as an escape hatch) relative to its cost.
- Central package repository with a real submission/review pipeline: **should-have**, can start as "a curated list of GitHub repos" and formalize later.
- Function-Repository-style single-function sharing: **later**.
- CloudDeploy/APIFunction hosted-API story: **later**, valuable but a distinct product (hosting infrastructure) that can follow once the language and package system are solid.
- LibraryLink-equivalent native-code FFI: **later**, needed eventually for a serious numerics story but not for an MVP that leans on the Python bridge instead.

---

## 6. The knowledge-base question

**What it is and why it matters**
Mathematica ships curated, load-on-demand datasets accessible as simple function calls (`CountryData["France","Population"]`, `ChemicalData[...]`, `WordData[...]`, and dozens more), all ultimately backed by the same curated Wolfram Knowledgebase that powers Wolfram|Alpha. This is a genuine "wow" feature (ask a question, get a real, computable, cited answer, no API key or scraping) and it is also the single hardest thing to replicate, because it is not really a software feature: it is decades of Wolfram Research staff curating, cleaning, structuring, and continuously updating hundreds of gigabytes of licensed and compiled data across dozens of domains.

**Honest assessment**
An OSS project cannot realistically build an equivalent to the Wolfram Knowledgebase. That is not a scoping problem solvable with more engineers on a normal OSS timeline, it is a data-licensing-and-curation operation the size of a small company, sustained over 15+ years, much of it from licensed commercial data sources that are not freely redistributable. Attempting to match it head-on would be a multi-year distraction from the parts of the project that are actually achievable (language, notebook, interactivity).

The better strategy is to build a thin, honest, swappable **data-connector layer**, not a knowledgebase:
- Wire up a small number of well-structured open data functions (`CountryData`, maybe `ElementData`, `CurrencyData`-style FX rates) backed by genuinely open sources: **Wikidata** (via its SPARQL endpoint, broad coverage across countries, chemicals, people, works, huge and actively maintained by a large volunteer community) is the strongest single option; supplement with domain-specific open datasets where they exist and are well maintained (e.g. periodic-table/element data, which is small, stable, and easy to vendor directly rather than querying live; open FX-rate APIs for currency).
- Be upfront in docs and marketing that this is a "connect to open data" layer, not a "built-in omniscient knowledgebase," and that data quality/coverage/freshness will vary by domain and by upstream source, unlike Wolfram's single curated pipeline.
- Design the data-function API (`XData["Name","Property"]`-style calls) to be source-agnostic under the hood, so a given function's backing data source can be swapped or a local cache added later without breaking user code, and so a user or organization could plug in their own data source for a given domain if the default open one is not good enough.
- Treat Wolfram|Alpha-style natural-language question answering as explicitly out of scope for the same reason as free-form linguistic input in section 1: it depends on the same curated backend, and the realistic modern substitute is routing to an LLM, which is a different mechanism (probabilistic, not curated/citable) and should be presented that way, not disguised as a knowledgebase lookup.

**Top use cases (for the descoped, connector-based version)**
- Quick geography/demographics lookups for teaching examples (population, capital, flag, area) via Wikidata-backed `CountryData`.
- Chemical/physical constants lookups (element properties, basic molecular data) from small vendored open datasets rather than a live query, since this data is stable and doesn't need daily freshness.
- Unit conversion and physical constants (a much smaller, well-defined, easily-open-sourceable dataset, e.g. CODATA / NIST published constants) as an early, achievable win that delivers real value without needing the full curated-knowledgebase ambition.
- Basic linguistic data (e.g. word frequency, synonyms) via an open lexical resource (WordNet-derived data) for a lightweight `WordData`-equivalent.

**Implementation difficulty**
- Small, stable, vendorable datasets (element properties, physical constants, WordNet-style lexical data): **easy**, these can be shipped as static bundled data with no live dependency at all.
- Wikidata-backed live queries (`CountryData`-equivalent via SPARQL): **medium**, the query mechanics are well documented, but building a clean, stable, well-typed function API on top of Wikidata's much messier and more general schema (and handling rate limits/caching) takes real design work.
- Anything approaching the breadth of the actual Wolfram Knowledgebase (dozens of domains, continuously updated, licensed commercial sources): **research-grade/infeasible** for an OSS project at typical scale; explicitly descope this rather than half-attempt it.
- Wolfram|Alpha-style free-form question answering: **research-grade**, and arguably not the right problem to solve the same way Wolfram solved it; an LLM-routing approach is a different, more tractable, but fundamentally different-in-character feature.

**OSS building blocks to reuse**
- Wikidata's public SPARQL endpoint and the Wikidata Query Service as the primary live-data backend.
- Static open datasets: CODATA physical constants, IUPAC/NIST element data, Princeton WordNet, Natural Earth (geography/country boundaries), all freely redistributable and small enough to vendor.
- A caching layer (even a simple local SQLite cache of recent Wikidata query results) to avoid hammering the public endpoint and to keep functions fast and available offline after first use.

**MVP priority**
- Physical constants and unit conversion (static, vendored, small, high value): **must-have**, this one is cheap and genuinely useful for almost every technical-computing user.
- Element/chemical basic property data (static, vendored): **should-have**.
- Wikidata-backed `CountryData`-equivalent: **should-have**, good demo value, moderate build cost.
- Broader curated-knowledgebase ambition (ChemicalData depth, WordData depth, anything requiring licensed commercial data): **explicitly descoped**, communicate this clearly rather than let users discover the gap by surprise.
- Wolfram|Alpha-style NL question answering: **later**, and likely delivered as "ask an LLM" rather than a knowledgebase feature, clearly labeled as such.

---

## 7. Competitive landscape summary

| Project | What it already delivers | Where it falls short of Mathematica |
|---|---|---|
| **Mathics3** | An actual open-source Wolfram Language kernel (Python), parses and evaluates a real subset of WL syntax, has a Django-based web front end with MathML output and Three.js graphics, growing built-in function coverage (has been steadily adding ~100+ built-ins per cycle). Closest thing to "OpenMat already exists" today. | Front end is a basic web notebook, not a rich cell-grouping/2D-math-input/notebook-as-expression experience. No `Manipulate`/`Dynamic` reactive model to speak of. Function coverage, performance, and documentation quality are far behind. Small maintainer base, slow velocity relative to the surface area of the language it's cloning. |
| **Jupyter + SymPy (+ SciPy/NumPy/matplotlib)** | The dominant open notebook ecosystem, huge tooling and extension base, SymPy is a solid pure-Python CAS, massive numerics/plotting ecosystem around it. | No unified language: SymPy, NumPy, and plotting are separate libraries with separate APIs and mental models, not one coherent symbolic-first language. No `Manipulate`-equivalent one-liner (ipywidgets is lower-level and coarser-grained). No cell-grouping/notebook-as-expression document model, `.ipynb` is just JSON. No unified, Mathematica-quality documentation across the whole stack (docs are per-library and vary wildly in quality). |
| **SageMath** | The most ambitious existing open "Mathematica-scale" umbrella: unifies SymPy, Maxima, PARI/GP, GAP, FLINT and more under one Python-based interface, explicitly founded to be a free Mathematica/Maple/Magma alternative, strong in number theory and algebra especially. Runs in Jupyter. | Still fundamentally a Python library collection wearing a unifying interface, not a from-the-ground-up single coherent symbolic language; heavier install (bundles many C/Fortran libraries), less polished interactive front end, documentation and knowledgebase depth both behind Mathematica. Reasonable "80% there" for research math users, weaker as a general computing/teaching environment. |
| **Julia + Pluto.jl** | Best-in-class open reactive-notebook *mechanism*: true reactive dependency-graph execution (arguably closer to the spirit of `Dynamic` than anything else on this list), reached a stable 1.0 milestone, clean guarantee that visible code fully determines program state. Julia itself is a strong, fast, general numerical language. | Not a symbolic-math-first language (Julia's symbolic ecosystem, Symbolics.jl etc., is younger and less complete than Mathematica's core). No unified curated-documentation-with-runnable-examples system at Mathematica's depth. No knowledgebase story at all. Reactivity engine is the standout, everything else is "good general-purpose scientific computing," not "Mathematica clone." |

**Where the open gap actually is**
No existing project combines all three of: (1) a real symbolic-math-first language core, (2) a genuinely reactive notebook front end with a `Manipulate`-caliber one-liner interactivity story, and (3) documentation good enough that people trust and enjoy using it. Mathics3 has (1) partially and neither (2) nor (3) at Mathematica's level. SageMath/Jupyter+SymPy have breadth of numeric/scientific tooling but not (1) as a unified language or (3). Pluto/Julia have the best version of (2)'s underlying mechanism but not (1) or a knowledgebase-adjacent story. This is the gap OpenMat can credibly target: not out-doing Mathematica on knowledgebase depth or 40 years of built-in function coverage, but being the first OSS project to combine a coherent symbolic language, best-in-class notebook reactivity, and documentation quality high enough that people choose it over Mathematica for teaching and prototyping specifically, and choose it over Jupyter+SciPy for anyone who wants a more integrated language rather than a library pile.

**Lessons from why Mathics has not displaced Mathematica**
- Coverage and depth compound: Mathematica has had a large, well-funded team adding functions, fixing edge cases, and polishing docs continuously for decades. A volunteer project chasing full parity on that surface area will always be behind and will burn morale trying. The lesson for OpenMat: do not attempt full API-surface parity as a goal; pick a smaller, coherent, excellent core and let it grow organically, the way early-stage successful OSS languages (Julia, Rust) did rather than the way "clone everything" projects tend to stall.
- The front end matters as much as the language. Mathics's kernel is a genuinely credible WL implementation, but its front end has never come close to notebook-quality UX, and that alone keeps it a curiosity rather than a daily driver even for users who would tolerate an incomplete function library. The lesson: front-end polish is not secondary work to do "once the language is done," it needs comparable investment from the start, which is exactly why this spec exists as a separate track.
- Documentation debt is silent but fatal. A project can have a working evaluator and a working notebook and still fail to get adopted if users cannot self-serve answers to "how do I do X," because they'll bounce to Mathematica's superior docs (or to Python) rather than dig through source. Budget for docs as a permanent, funded workstream, not a pre-launch checklist item.
- Small, funded, narrow projects with a real product wedge (Pluto.jl is the best example here) can beat "clone everything" efforts on the specific dimension they focus on, even with far fewer resources than Wolfram Research. OpenMat's best shot is the same pattern: win decisively on reactivity + notebook UX + docs quality for a well-defined initial audience (education, quick prototyping), rather than trying to be a complete Mathematica replacement on day one.

---

## MVP slice

> **Superseded.** The authoritative MVP scope is now [m0-milestone.md](m0-milestone.md), which resolves this section against the feature interview and the other two specs (see issue #1). Two changes of note: structural 2D input moved from skip to release gate, and the kernel is specified as one transport-neutral service with a local in-process adapter first and a Jupyter protocol adapter as a later milestone. This section is kept for research context.

The smallest front end that is worth shipping and that a real user would choose to open twice:

1. **Notebook editor**: linear sequence of Input/Output/Text/Section cells, evaluate-cell-with-shift-enter, save/load a native notebook format that is a serialized expression tree (not ad hoc JSON), basic cell grouping/outlining.
2. **Kernel connection over the Jupyter protocol**: a headless-capable kernel, `comm`-channel wired up from day one (not bolted on later) because interactivity depends on it.
3. **Cell-level reactivity**: Pluto-style static dependency analysis so cells automatically re-run when something they depend on changes, this is the reactivity foundation everything else in section 2 sits on.
4. **One killer interactivity primitive**: a working `Manipulate`-equivalent (slider-bound live plot/expression) even if it only supports the common cases at first (numeric sliders, one or two variables, plot/graphics output). This is the single highest-leverage demo feature for both credibility and teaching use cases, ship it early and make it good even if other things are thin.
5. **A documentation viewer built from the same cell-rendering component as the notebook**, covering a small, curated set of functions to the full standard (runnable examples, CI-verified) rather than a large set done poorly.
6. **A CLI/script runner** (headless kernel execution) for automation and CI use, cheap to add once the kernel exists.
7. **A basic package/import mechanism** and a Python bridge (`ExternalEvaluate`-equivalent), so users are never fully blocked by missing built-ins.
8. Skip for MVP: free-form linguistic input, any knowledgebase beyond static physical-constants/unit data, CloudDeploy/hosted-API story, LibraryLink/native FFI, locators and 3D live-rotation (ship these once the reactivity core is solid).

**Architecture recommendation: own notebook document model, Jupyter kernel protocol underneath, not "just build a Jupyter notebook extension."**

Reasoning: the wire protocol and the document model are separable decisions, and they should get different answers. On the protocol side, there is no upside to inventing something new (see section 3); adopt the Jupyter protocol so the kernel is immediately usable from existing tools and so the project inherits a mature client/server ecosystem for free. On the document model side, the opposite is true: Jupyter's `.ipynb`/classic-notebook model cannot represent cell grouping, notebook-as-expression, or fine-grained `Dynamic` regions without serious contortion, because it was never designed for a document that is itself a manipulable expression in the same language the kernel runs. Building only a JupyterLab extension would cap OpenMat at "a somewhat nicer Jupyter," never at "a Mathematica-caliber notebook," which defeats the point of this whole track. So: build a custom front end and document format, but make the kernel and its `comm` channel Jupyter-protocol-compatible, so JupyterLab/VS Code users can still connect to an OpenMat kernel today (adoption/distribution lever) even while the flagship experience lives in the custom front end.

**Biggest product risks**
- **Scope creep toward full Mathematica parity.** The single most likely failure mode, based on Mathics3's history, is trying to match Wolfram's function count and knowledgebase breadth instead of winning decisively on a narrower slice (reactivity, notebook UX, docs quality). Guard against this explicitly in planning, not just in this document.
- **Underinvesting in documentation until "later."** Docs are the thing users judge trust and quality by fastest, and they are also the thing most likely to get deprioritized under deadline pressure because they don't feel like "core engineering." Fund it from day one.
- **Getting the notebook-as-expression architecture wrong early.** This is a foundational data-model decision (cells and notebooks as real expressions the kernel understands, not a separate JSON format the kernel is blind to) that is very expensive to retrofit. Decide and lock this in before writing much front-end code, even if the full feature set built on top of it ships incrementally.
- **The Manipulate/Dynamic demo not actually feeling as good as Mathematica's.** This is the feature most likely to be judged by side-by-side comparison against the real thing, and a sluggish, coarse-grained, or bug-prone version will read as "cheap knockoff" rather than "credible alternative" even if the rest of the language is solid. Worth extra polish investment relative to its raw engineering cost.
- **Knowledgebase-envy scope creep.** Users coming from Mathematica will ask "why doesn't `CountryData` know X" and it will be tempting to keep chasing coverage. Set and communicate the descope from section 6 clearly and early so it does not become an open-ended, unwinnable commitment.
