# OpenMat Spec: Numerics, Data, and Visualization

Scope: numerical computing, plotting/graphics, data import and manipulation, statistics and ML, and a brief pass on image/sound/graph processing. This is the "what does a scientist actually do all day" layer. It sits on top of whatever core language/CAS the other spec covers, and underneath whatever notebook front-end the third spec covers.

Format per feature: what it is and why it matters, core use cases with concrete input/output, implementation difficulty, OSS building blocks, MVP priority.

---

## 1. Numerical computing

### 1.1 Machine and arbitrary precision numerics (N, precision/accuracy model)

**What it is.** `N[expr]` and `N[expr, digits]` numericalize any expression. Under the hood, Mathematica tracks two different numeric representations: fast fixed-width machine doubles (53 bits, ~15.95 decimal digits), and arbitrary-precision "bignums" tagged with a live precision value that propagates through arithmetic (significance arithmetic). This second system is the part that is genuinely unusual: `N[Pi, 50]` doesn't just print 50 digits, it computes a number that Mathematica itself considers accurate to roughly 50 digits, and every downstream operation degrades that precision honestly as error accumulates (catastrophic cancellation reduces reported precision instead of silently returning garbage digits).

**Core use cases:**
- `N[Pi, 100]` - get 100 correct digits of a constant, used constantly in demos and in verifying formulas.
- `N[Sqrt[2] - 1.4142135623730951]` - machine-precision subtraction that silently loses most digits; Mathematica's arbitrary-precision version of the same computation reports reduced precision instead of a wrong-looking answer.
- `SetPrecision[x, 30]` and `Precision[x]` / `Accuracy[x]` - inspect and control precision explicitly, common in numerical analysis coursework and in validating that an iterative algorithm hasn't degraded.
- Mixed-precision expressions: `1.5 + 2\`\`30` (a machine number plus a 30-digit number) automatically resolves to the lower of the two, matching how a numerical analyst reasons about error propagation.
- High-precision special functions: `N[Zeta[1/2 + 14 I], 40]`, used in research contexts (number theory, physics) where standard double precision floating point is insufficient.

**Implementation difficulty: hard.** Machine-precision doubles are trivial (wrap IEEE 754, i.e. the host language's native float). Arbitrary-precision bignum arithmetic is medium (GMP/MPFR solve the raw bignum problem completely). The genuinely hard part is significance arithmetic: propagating a "this many digits are trustworthy" tag through every arithmetic op, every special function, every linear algebra routine, in a way that is not wildly conservative (interval arithmetic massively over- or under-estimates error in practice) and not wildly optimistic (naive tracking gives false confidence). Wolfram's own implementation is a proprietary, decades-refined heuristic, not a formally provable system, and outside researchers have written papers critiquing its edge cases. Full fidelity to Mathematica's exact model is effectively research-grade; a "good enough" approximate model (track precision, degrade on cancellation, don't chase every edge case) is hard-but-tractable.

**OSS building blocks:** MPFR (arbitrary-precision floats with correct rounding) + MPFI or a custom precision-tracking wrapper on top of MPFR closes most of the raw-computation gap. GMP underneath MPFR. Python's `mpmath` is the closest existing OSS analog and already implements adaptive precision (it bumps working precision and checks digit stability), which is a good reference architecture, though it does not do full significance-tracking through arbitrary expressions the way Mathematica does. Realistically: wrap MPFR for the number type, and implement precision propagation only for elementary arithmetic and a curated set of special functions first; treat "significance arithmetic through arbitrary composed functions" as a stretch goal, not a v1 requirement.

**Priority: must-have** (machine precision + basic arbitrary precision via MPFR) for MVP; **later** for full significance-arithmetic fidelity. Most day-to-day numeric work (a plot, a numerical solve, a stats fit) only needs machine precision. Arbitrary precision is a must-have as a feature (users expect `N[x, 50]` to work) but the exact error-propagation semantics can be simplified initially.

---

### 1.2 Numerical linear algebra

**What it is.** `LinearSolve`, `Eigenvalues`/`Eigenvectors`, `SingularValueDecomposition`, `Det`, `Inverse`, `PseudoInverse`, `Norm`, `LeastSquares`, `MatrixRank`, sparse and structured matrix support (`SparseArray`, banded/Hermitian/positive-definite hints). This is the workhorse under almost everything else (fitting, ODE solving, stats).

**Core use cases:**
- `LinearSolve[m, b]` for a dense or sparse linear system, expected to auto-detect and dispatch to the right algorithm (LU for square dense, least-squares for overdetermined, sparse iterative for large sparse).
- `Eigensystem[m]` for a symmetric/Hermitian matrix in a physics or engineering class, expecting real eigenvalues sorted sensibly.
- `SingularValueDecomposition[m]` for PCA-style dimensionality work or ill-conditioned system diagnosis.
- `SparseArray` construction from a list of rules for large, mostly-zero systems (finite element matrices, graph Laplacians), with automatic sparse-aware solving.
- `LinearSolve[m, b, Method -> "Krylov"]` style explicit method selection for large-scale iterative solves.

**Implementation difficulty: easy-to-medium.** This is the single most "solved" area in scientific computing. Dense linear algebra is a thin, well-understood wrapper around LAPACK/BLAS. Sparse direct solvers (SuiteSparse: UMFPACK, CHOLMOD) and iterative solvers (Krylov methods via a library or a from-scratch conjugate-gradient/GMRES implementation) are also mature and off-the-shelf. The work is API design (matching Mathematica's auto-dispatch and symbolic-friendly interface) and integration, not algorithm research.

**OSS building blocks:** OpenBLAS or Intel MKL (dense BLAS/LAPACK kernels), Eigen (C++ header-only, good API to wrap), SuiteSparse (sparse direct), and any Krylov-method library or your own (a few hundred lines) for sparse iterative. NumPy/SciPy's `linalg` module is a proven reference for exactly this wrapping job and closes ~90% of the functional gap already; the remaining 10% is Mathematica-specific ergonomics (arbitrary precision matrices, symbolic-numeric mixing, automatic method dispatch).

**Priority: must-have.** Nothing else in this spec works without it.

---

### 1.3 NIntegrate

**What it is.** Numerical integration (definite integrals, multidimensional, improper, oscillatory, singular) via an extensible "integration strategy + integration rule" architecture. Deterministic adaptive strategies (`GlobalAdaptive`, `LocalAdaptive`) subdivide the domain and refine where the integrand misbehaves; specialized rules handle singularities (variable transformations like the double-exponential/IMT method) and oscillatory integrands (`"Oscillatory"` method using extrapolation or level-index methods); Monte Carlo and quasi-Monte Carlo strategies handle high-dimensional integrals.

**Core use cases:**
- `NIntegrate[Exp[-x^2], {x, -Infinity, Infinity}]` - the default case, expected to just work with no method tuning, matching the exact answer to `Sqrt[Pi]` closely.
- `NIntegrate[f[x], {x, 0, 1}, Method -> "LocalAdaptive"]` when the default strategy fails near a singularity, e.g. `1/Sqrt[x]` near 0.
- Multidimensional integrals: `NIntegrate[f[x, y], {x, 0, 1}, {y, 0, x}]` over a triangular region, common in physics/engineering coursework.
- Oscillatory integrands: `NIntegrate[Sin[100 x] f[x], {x, 0, 10}]`, where naive adaptive quadrature fails and a dedicated oscillatory method is required.
- Diagnosing convergence: users expect informative warnings ("failed to converge to prescribed accuracy") rather than a silently wrong number, which is a real usability differentiator versus calling `scipy.integrate.quad` blind.

**Implementation difficulty: medium.** Standard 1D adaptive Gauss-Kronrod quadrature is a solved, well-documented problem (this is exactly what QUADPACK/`scipy.integrate.quad` does). Multidimensional adaptive integration, singularity-handling transformations, and oscillatory-integrand methods are each individually medium difficulty but there are many of them, and the value Mathematica adds is the automatic strategy selection and graceful degradation/diagnostics, which requires real engineering polish, not new algorithms.

**OSS building blocks:** QUADPACK (the classic Fortran library, also the engine behind SciPy's `quad`) covers 1D adaptive quadrature completely. Cubature libraries (e.g. Steven Johnson's `cubature` library) cover multidimensional adaptive integration. GSL (GNU Scientific Library) has Monte Carlo and QAWO oscillatory routines. Realistic plan: wrap QUADPACK + cubature + a Monte Carlo/QMC sampler, and put engineering effort into method auto-selection and error reporting rather than writing new quadrature rules.

**Priority: must-have** for 1D and basic multidimensional; **should-have** for oscillatory/singular specialty methods (can start as "manually pick a method" rather than full auto-detection).

---

### 1.4 NDSolve (ODEs/PDEs, events, method framework)

**What it is.** The general numerical differential equation solver: IVPs, BVPs, DAEs, and a subset of PDEs, all through one function with an object-oriented "Method" plug-in framework (each method is a Step-function object; NDSolve handles step-size control, event detection, and output generically around whatever method is plugged in). PDEs go through method-of-lines: spatial discretization (finite difference or finite element via `"MethodOfLines"` / the FEM package) reduces the PDE to a large ODE/DAE system, which is then handed to the same ODE machinery.

**Core use cases:**
- `NDSolve[{y'[t] == -y[t], y[0] == 1}, y, {t, 0, 10}]` - basic IVP, returns an `InterpolatingFunction` that behaves like a continuous function, pretty-plots immediately with `Plot[y[t] /. sol, {t, 0, 10}]`. This "solution as a callable, plottable object" ergonomic is the single biggest UX win over raw solver output arrays.
- Stiff systems (chemical kinetics, circuit equations): `NDSolve` auto-detects stiffness and switches to BDF/Gear-type methods without the user specifying anything, which is a real differentiator from naively calling an explicit RK4.
- Event detection: `NDSolve[{..., WhenEvent[y[t] == 0, "StopIntegration"]}, ...]` for bouncing-ball / threshold-crossing simulations, extremely common in teaching (physics, biology, control systems).
- Simple PDEs: `NDSolve[{D[u[t,x],t] == D[u[t,x],{x,2}], ...}, u, {t,0,1}, {x,0,1}]` (1D heat equation), expected to "just work" for textbook-level problems without the user hand-building a discretization.
- Parametric/sensitivity sweeps: solving the same system for a family of parameter values and comparing trajectories, common in modeling coursework.

**Implementation difficulty: hard for full generality, medium for the 80% case.** Robust adaptive-step explicit RK (Dormand-Prince/RK45) and implicit BDF methods for stiff ODEs are well-trodden ground with mature open libraries. What's hard: (1) the unified interface across ODE/DAE/PDE that infers problem type from the input equations, (2) automatic stiffness detection and method switching, (3) robust, general event location within adaptive steps, (4) PDE method-of-lines with automatic mesh generation for arbitrary geometries (this last one edges into research-grade for anything beyond 1D/rectangular 2D domains). A credible OSS clone should nail ODEs+events well and treat general PDE solving as a much later, narrower feature (e.g. just 1D/2D rectangular finite-difference).

**OSS building blocks:** SUNDIALS (CVODE for stiff/nonstiff ODEs via Adams/BDF, IDA for DAEs, ARKODE for flexible one-step/IMEX methods) is a mature, LLNL-maintained, permissively-licensed C library and is the single best building block here, it closes most of the ODE/DAE gap directly. Julia's `DifferentialEquations.jl` (SciML) is not license-compatible to wrap directly but is the best architectural reference for how to structure a method-plugin system and event handling; its docs/benchmarks are worth studying. For PDEs, there is no equivalent single drop-in; FEniCS/deal.II (finite element) or a hand-rolled finite-difference method-of-lines are the realistic paths, and this is where scope should be kept narrow initially.

**Priority: must-have** for ODE IVPs with basic stiffness handling and events (this is what 90% of users touch). **Should-have** for DAEs and simple 1D/2D PDEs. **Later** for BVPs, general-geometry PDEs, and delay differential equations.

---

### 1.5 FindRoot / FindMinimum / NMinimize (and NMaximize)

**What it is.** `FindRoot` solves nonlinear equations locally (Newton/secant-family methods); `FindMinimum`/`FindMaximum` do local nonlinear optimization (gradient-based: quasi-Newton, conjugate gradient, Levenberg-Marquardt for least-squares residuals; also derivative-free Nelder-Mead as a fallback); `NMinimize`/`NMaximize` target global optimization over a bounded or constrained region using direct-search methods (Nelder-Mead, differential evolution, simulated annealing, random search) followed by local polishing.

**Core use cases:**
- `FindRoot[Cos[x] == x, {x, 1}]` - single nonlinear equation, one starting point, expected to converge fast via Newton's method with automatic (symbolic or numeric) derivative computation.
- `FindRoot[{eq1, eq2}, {x, x0}, {y, y0}]` - systems of nonlinear equations, common in engineering/circuit problems.
- `FindMinimum[f[x, y], {{x, x0}, {y, y0}}]` - local optimization from a guess, e.g. fitting a physical model by hand or minimizing an energy functional.
- `NMinimize[{f[x], constraints}, {x, y}]` - global optimization with no starting guess needed, over a bounded box or general constraints; this "just give me the global min, I don't know where to start" ergonomic is a major usability draw versus raw `scipy.optimize` where the user must pick an algorithm and starting point.
- Constrained optimization mixing equalities/inequalities: `NMinimize[{cost, g[x] <= 0, h[x] == 0}, vars]`, common in operations-research-flavored coursework.

**Implementation difficulty: medium.** Every individual algorithm (Newton, BFGS/L-BFGS, Levenberg-Marquardt, Nelder-Mead, differential evolution, simulated annealing) is well-documented and has multiple OSS implementations. The Mathematica-specific value-add is: automatic differentiation or symbolic derivatives feeding the local methods (removing the need for the user to supply a Jacobian), automatic method selection based on problem structure (smooth vs. nonsmooth, least-squares residual form triggers Levenberg-Marquardt automatically), and reasonable defaults for global search budget/convergence tolerance. None of this is research-grade; it's careful engineering and good defaults.

**OSS building blocks:** SciPy's `optimize` module (`minimize`, `root`, `differential_evolution`, `dual_annealing`) is a near-complete functional analog and covers almost all of the needed algorithms already; NLopt is another strong, permissively-licensed library covering both local and global methods with a clean C API. IPOPT (interior point, for larger constrained NLP) covers the constrained-optimization end. The gap to close is mostly UX/dispatch layered on top of these, not new solvers.

**Priority: must-have** for FindRoot and FindMinimum (local, unconstrained/box-constrained). **Should-have** for NMinimize global methods and general nonlinear constraints.

---

### 1.6 Interpolation

**What it is.** `Interpolation[data]` builds a continuous, differentiable `InterpolatingFunction` from discrete data (1D or multidimensional, structured or scattered), used both directly and as the return type of `NDSolve`. Also `ListInterpolation`, spline fitting (`BSplineFunction`), and smoothing variants.

**Core use cases:**
- `Interpolation[{{1,2},{2,3},{3,5},{4,4}}]` - fit a smooth curve through data points (default cubic spline), then use it as a plain function: `f[2.5]`, `Plot[f[x], {x,1,4}]`, `D[f[x], x]`, `NIntegrate[f[x], {x,1,4}]`. The fact that the result composes transparently with plotting, calculus, and further numerics is the key UX point, it's not just curve-fitting, it's "turn data into a first-class function."
- 2D/3D scattered-data interpolation for experimental or simulation grid data: `Interpolation[{{{x1,y1},z1}, ...}]`.
- Order control: `Interpolation[data, InterpolationOrder -> 1]` for piecewise-linear when the underlying data is noisy and a smooth spline would overfit/oscillate (Runge phenomenon).
- Being the invisible plumbing under `NDSolve` output, this needs to be solid and fast since it's on the hot path of `Plot[sol[t], ...]` after every solve.

**Implementation difficulty: easy-to-medium.** 1D cubic spline and piecewise polynomial interpolation is genuinely easy and extremely well understood. Multidimensional scattered-data interpolation (as opposed to grid data) is medium, typically via Delaunay triangulation + local polynomial fits, or radial basis functions.

**OSS building blocks:** SciPy's `interpolate` module (`CubicSpline`, `griddata`, `RBFInterpolator`) is a complete, provenreference implementation covering essentially the full scope needed. GSL also has solid 1D spline routines. This is a low-risk, mostly-wrapping task.

**Priority: must-have.** Cheap to build, high leverage (needed by NDSolve output, data plotting, and general numeric work).

---

### 1.7 Fourier / signal processing

**What it is.** `Fourier`/`InverseFourier` (discrete Fourier transform with configurable convention/normalization), `FourierTransform` (symbolic/analytic, actually core-language territory), plus signal processing on top: `ListConvolve`, `LowpassFilter`, `HighpassFilter`, spectrogram-style analysis, `PeriodogramArray`.

**Core use cases:**
- `Fourier[data]` on a numeric list to inspect frequency content, e.g. analyzing a noisy time series or audio clip, expecting a clean, correctly-normalized complex spectrum.
- `ListConvolve[kernel, data]` for filtering or smoothing a signal or image row/column.
- `LowpassFilter[data, cutoff]` / `HighpassFilter` for quick denoising without manually designing a filter, a "batteries included" convenience over raw FFT-based tools.
- Spectrogram-style time-frequency views for audio or vibration data, common in signal-processing coursework and engineering diagnostics.
- Interplay with `Audio`/`Sound` objects: import an audio file, `Fourier` its samples, plot the spectrum, this end-to-end "load, transform, plot" pipeline in three lines is a differentiator over needing separate libraries for I/O, transform, and plotting.

**Implementation difficulty: easy-to-medium.** The FFT itself is a fully solved, extremely well-optimized problem. Mathematica's twist is the configurable `FourierParameters` (different fields/communities use different normalization conventions for the DFT, and Mathematica lets you match any of them), which is trivial to add as a parameter. Filter design functions are medium (need reasonable default filter orders/windows) but well-documented DSP territory.

**OSS building blocks:** FFTW (the standard, extremely fast, permissively-licensed-enough C FFT library; note GPL license needs checking against OpenMat's license choice, PocketFFT is a friendlier-licensed alternative used by NumPy/SciPy now) closes the core transform completely. SciPy's `signal` module gives filter design, convolution, and periodogram tools essentially for free.

**Priority: should-have.** Important for signal-processing-adjacent users but narrower audience than plotting/linear algebra/ODEs; can trail the must-haves.

---

## 2. Visualization

### 2.1 Plot / Plot3D / ParametricPlot / ContourPlot / DensityPlot family

**What it is.** Function-based plotting: give an expression and a variable range, get a plot. `Plot` (2D curves), `Plot3D` (surfaces), `ParametricPlot`/`ParametricPlot3D` (parametric curves/surfaces), `ContourPlot`/`ContourPlot3D` (level sets), `DensityPlot` (2D scalar field as color), `RegionPlot` (inequality regions). These all share an adaptive sampling engine: the plotter evaluates the function at initial points, then recursively refines the sampling wherever the function is changing fast, curving, or discontinuous, rather than using a fixed grid.

**Core use cases:**
- `Plot[Sin[x] Exp[-x/5], {x, 0, 20}]` - the single most common first command a new user types; expected to produce a smooth, correctly-scaled, nicely-labeled curve with zero extra options.
- `Plot[{f[x], g[x]}, {x, a, b}]` - multiple curves overlaid, automatically color-differentiated and given a sensible default legend when asked.
- `Plot3D[Sin[x y], {x, -3, 3}, {y, -3, 3}]` - 3D surface with automatic mesh density, lighting, and viewpoint that looks reasonable without any styling.
- `ContourPlot[x^2 + y^2 == 4, {x, -3, 3}, {y, -3, 3}]` or implicit curve families, common for visualizing constraint regions or level sets in optimization/multivariable calculus contexts.
- Discontinuity and singularity handling: `Plot[Tan[x], {x, -Pi, Pi}]` is expected to NOT draw vertical lines connecting the asymptote branches, i.e. the adaptive sampler needs enough intelligence to detect a jump and break the curve, this specific case is a well-known "does this clone actually work" litmus test.

**Implementation difficulty: medium-to-hard.** Drawing a curve from sampled points is trivial. What's actually hard, and what makes Mathematica's plots feel "smart," is the adaptive refinement algorithm: deciding where to add more sample points (curvature-based refinement), detecting and handling discontinuities/singularities so they don't get connected by a spurious line, and choosing sensible default plot ranges when the function blows up or is undefined on part of the domain (`PlotRange -> Automatic` clipping outliers intelligently). This is squarely medium-to-hard engineering: no single algorithm to implement, but a long tail of heuristics that collectively produce the "just works" feel. Getting 80% of the visual quality (reasonable adaptive sampling, basic discontinuity detection via large-derivative heuristics) is medium; matching Mathematica's exact polish on pathological functions is a long tail.

**OSS building blocks:** No existing OSS plotting library does full adaptive function-sampling with discontinuity detection out of the box the way Mathematica does; matplotlib/plotly expect you to hand them sampled data arrays, the adaptive sampling is squarely on OpenMat to build. This is a genuine differentiator to invest in. The rendering/backend layer (turning a list of line segments/polygons into pixels or SVG/PNG) can lean on existing 2D vector graphics libraries (Cairo, Skia) or a browser canvas/SVG target, that part is well solved. Matplotlib's `matplotlib.pyplot` is worth studying for the plot-object model even though its default sampling is naive (uniform grid).

**Priority: must-have**, and arguably the single highest-leverage investment in this whole spec: this is the feature that makes or breaks the "feels like Mathematica" first impression.

---

### 2.2 ListPlot family (ListPlot, ListLinePlot, BarChart, Histogram, etc.)

**What it is.** Plotting from discrete data instead of symbolic functions: `ListPlot` (scatter), `ListLinePlot` (connected), `BarChart`, `Histogram`, `PieChart`, `BoxWhiskerChart`. This is the "I have a dataset, show it to me" counterpart to section 2.1's "I have a formula, show it to me."

**Core use cases:**
- `ListPlot[data]` on a raw list of numbers or `{x,y}` pairs, immediately usable with no data-frame ceremony.
- `Histogram[data]` with automatic binning (Mathematica uses a smart default bin-width heuristic, not a fixed bin count), the single most common exploratory-data-analysis command.
- `BarChart[values, ChartLabels -> labels]` for categorical comparisons, expected to auto-color and auto-space bars sensibly.
- `ListPlot[data1, data2, ...]` or `ListPlot[{data1, data2}]` overlaying multiple series with automatic color/marker differentiation and joint axis scaling.
- `BoxWhiskerChart` / error-bar variants for showing distributions or uncertainty, common in lab-report-style scientific use.

**Implementation difficulty: easy-to-medium.** Much easier than 2.1 because there's no adaptive sampling problem, the data already exists. The main work is: (a) good default binning/aggregation heuristics (Histogram's default bin width is itself a small research topic, Freedman-Diaconis-style rules), (b) consistent, shared styling/color-cycling logic with the rest of the plotting system so a `ListPlot` and a `Plot` look like they belong to the same family.

**OSS building blocks:** Directly comparable to matplotlib/plotly/Vega-Lite territory, this is the best-covered part of the whole spec by existing OSS. Wrapping a Vega-Lite-style declarative spec or building directly on a Cairo/Skia/SVG backend (reusing the section 2.1 renderer) gets most of the way there quickly.

**Priority: must-have.**

---

### 2.3 Graphics / Graphics3D primitive language

**What it is.** The low-level declarative graphics language underneath every high-level plot: `Graphics[{Red, Disk[{0,0}, 1], Line[{{0,0},{1,1}}], Text["label", {2,2}]}]`. Primitives (Point, Line, Polygon, Circle, Disk, Rectangle, Arrow, Text, BezierCurve, and their 3D analogs Sphere, Cuboid, Cylinder, Cone, Tube), directives (color, thickness, opacity, dashing), and the fact that every high-level plot function is really just "compute some primitives, wrap in Graphics" under the hood.

**Core use cases:**
- Building custom diagrams by hand: `Graphics[{Circle[], Line[{{-1,0},{1,0}}]}]` for a geometry illustration, extremely common in teaching materials and math exposition.
- Composing custom visualizations that aren't a built-in chart type, e.g. hand-drawn network diagrams, custom annotated figures for a paper.
- `Show[plot1, plot2]` and `Show[Plot[...], Graphics[{extra primitives}]]` to overlay custom annotations (arrows, labels, shaded regions) onto an automatically generated plot, an extremely common workflow for annotating a result.
- 3D scenes: `Graphics3D[{Sphere[{0,0,0}], Cuboid[{1,1,1}]}]` for simple 3D illustrations, with automatic reasonable lighting/camera defaults.
- Direct manipulation from other functions: `Plot` internally returns a `Graphics` expression, and users routinely poke at it (`Cases[plot, Line[pts_] :> pts, Infinity]`) to extract underlying coordinate data, this "graphics is just an inspectable symbolic expression" property is architecturally important and touches the core-language spec (out of scope here beyond noting the dependency).

**Implementation difficulty: medium.** The primitive set itself is not hard to render (this is standard 2D/3D vector graphics, directly analogous to SVG/Canvas or a basic 3D scene graph). The medium difficulty is in (a) making Graphics expressions symbolic/inspectable the way Mathematica expressions are (a core-language integration point), (b) sensible automatic layout defaults (padding, aspect ratio, 3D default camera/lighting) so hand-built primitives look reasonably good without manual tuning, matching the polish bar set by section 2.1.

**OSS building blocks:** For 2D: SVG or Cairo as a direct target, this is a well-solved rendering problem. For 3D: a modest WebGL/OpenGL scene-graph renderer, or lean on an existing library (three.js if targeting a web/notebook front-end, which is a reasonable bet given most modern OSS scientific notebooks are browser-based). This is squarely "well-understood graphics engineering," not research.

**Priority: must-have** as the foundation (everything in 2.1/2.2 sits on top of it), but it can be built incrementally, start with 2D primitives sufficient to support Plot/ListPlot, add 3D and the full primitive catalog after.

---

### 2.4 Styling, theming, legends, and "why Mathematica plots look good by default"

**What it is.** `PlotTheme` (a small set of named presets like `"Scientific"`, `"Detailed"`, `"Business"` that each set a bundle of underlying options: color palette, font, gridlines, frame style at once), automatic color-cycling across multiple series using a curated palette, automatic legends (`PlotLegends -> Automatic`), and a general design philosophy where every visual choice (default font, default color palette, default aspect ratio, default padding/margins) was chosen once by Wolfram's design team to look good together, so a user who never touches a styling option still gets a coherent, publication-reasonable figure. This is genuinely a big part of Mathematica's reputation and a place where naive OSS clones (a bare matplotlib default plot) visibly fall short.

**Core use cases:**
- Never specifying any styling and getting a plot that looks intentional: right aspect ratio, non-garish default blue curve, light gridlines, readable default font size, sensible tick spacing, this is the zero-effort baseline every other feature is compared against.
- `Plot[{f,g,h}, {x,a,b}, PlotLegends -> "Expressions"]` - one option turns on a legend that auto-labels each curve with its formula, a very well-loved convenience feature.
- `PlotTheme -> "Scientific"` to switch to a paper-ready look (serif-ish fonts, muted palette, tighter frame) in one keystroke instead of setting ten options by hand.
- Consistent color palettes across an entire notebook (using `$PlotTheme` or a stylesheet) so a report or paper has visual consistency without per-plot micromanagement.
- Overriding one piece (`PlotStyle -> Red`) while leaving everything else at its good default, the theme system is layered so explicit options always win over theme defaults.

**Implementation difficulty: medium, but high-value and underrated.** No individual piece is hard (color palettes and font choices are just data), but this is a design/curation problem as much as an engineering one: it requires someone with actual visual design taste to pick a default palette (colorblind-safe, good on both light/dark), default typography, default spacing/padding rules, and encode them as the fallback for every plot type consistently. This is exactly the kind of area where OSS projects chronically underinvest (see matplotlib's famously dated pre-2.0 defaults, fixed only when the "viridis + better defaults" overhaul shipped) and where it visibly pays off, this is a common "small wow" people compare when evaluating an OSS Mathematica clone.

**OSS building blocks:** No existing library to wrap for this, it's a design deliverable, not a coding one. Good references for what "good defaults" look like in the wild: Vega-Lite/Observable Plot's design defaults, ggplot2's theme system architecture (a genuinely good analog for the layered-theme concept, ggplot2's `theme_bw()`/`theme_minimal()` etc. map closely to Mathematica's `PlotTheme`), and Makie.jl's newer default theme (praised for looking modern out of the box). Recommend explicitly budgeting design time/a design pass here rather than treating it as incidental to the plotting engine.

**Priority: must-have.** This is disproportionately important to perceived quality relative to its engineering cost; a good default palette and theme system is cheap compared to the adaptive-sampling engine in 2.1 but arguably just as visible to users.

---

### 2.5 Image export (rasterization, vector export, formats)

**What it is.** Every graphic needs to leave the notebook: `Export["plot.png", g]`, `Export["figure.pdf", g]`, `Export["diagram.svg", g]`, with DPI/size control for print-quality output, and format-appropriate rendering (vector formats keep curves as curves, not rasterized).

**Core use cases:**
- `Export["fig1.pdf", plot]` for inclusion in a LaTeX paper, expected to produce a clean vector PDF at the right bounding box, no extra whitespace.
- `Export["chart.png", plot, ImageResolution -> 300]` for a Word doc or slide deck, print-quality raster output.
- Batch exporting a set of plots from a loop/table, a common workflow when generating many similar figures (e.g. one per parameter value).
- Round-tripping: exporting then re-importing an image for further processing (ties into section 3.1 and section 5.1).

**Implementation difficulty: easy.** Rendering to PNG/SVG/PDF is a completely solved problem once the Graphics primitive renderer (2.3) exists, it's a matter of choosing the right backend targets (a vector graphics library that supports multiple output formats, e.g. Cairo supports PNG/PDF/SVG/PS from one drawing API) and getting DPI/bounding-box handling right.

**OSS building blocks:** Cairo (or Skia) natively supports exporting the same drawing calls to PNG, PDF, and SVG, this is close to a solved problem once the renderer exists. resvg or similar for SVG-specific needs.

**Priority: must-have**, but nearly free once 2.3 exists.

---

## 3. Data handling

### 3.1 Import/Export and the format ecosystem

**What it is.** `Import["file.ext"]` auto-detects format from extension (or content) and returns an appropriate native structure (a list of lists for CSV, nested associations for JSON, an `Image` for a PNG, `Audio`/`Sound` for a WAV, a `Graphics` for an SVG). `Export` does the reverse. Mathematica supports several hundred formats and subformats spanning tabular, image, audio, video, scientific (FITS, NetCDF, HDF5, DICOM), 3D geometry, GIS, and document formats, all through one uniform function pair.

**Core use cases:**
- `Import["data.csv"]` and immediately having a usable list-of-lists or, with `"Dataset"`, a queryable `Dataset` object with headers as keys.
- `Import["data.json"]` returning nested associations that mirror the JSON structure directly, no separate parsing step or schema needed.
- `Import["photo.jpg"]` returning a first-class `Image` object usable directly in image-processing functions (ties to section 5.1) and directly displayable/plottable.
- Scientific formats: `Import["spectrum.fits"]`, `Import["data.nc"]` (NetCDF), common in astronomy/climate/physics research workflows where the ability to just open the domain's native format without a separate library is a major convenience.
- `Export["out.xlsx", dataset]` writing a computed result straight to a spreadsheet a non-technical collaborator can open, a very common "hand off the result" workflow.

**Implementation difficulty: medium overall, easy per-format.** No single format is hard (CSV/JSON/XLSX parsing/writing are all solved problems with mature libraries in every ecosystem), the difficulty is breadth: replicating "hundreds of formats" is a long tail of integration work, not depth. A pragmatic OSS scope should prioritize by actual usage frequency, not attempt format-parity with Mathematica on day one.

**OSS building blocks:** This is one of the strongest areas for OSS leverage. CSV: trivial/any language's standard tooling. JSON: same. XLSX: `openpyxl`-equivalent or a small XLSX reader/writer library. Images: any standard image codec library (libpng, libjpeg-turbo, or a bundled image crate/library) covers the common raster formats. Scientific formats: cfitsio (FITS), netcdf-c/HDF5 libraries are mature, permissively-licensed C libraries built exactly for this. Realistically, wrapping 15-20 high-frequency formats (CSV, TSV, JSON, XLSX, PNG/JPG/GIF/BMP, WAV/MP3, PDF-read-only, HDF5, plain text) covers the large majority of real usage; the "hundreds of formats" long tail is a should-have/later concern.

**Priority: must-have** for CSV/JSON/XLSX/common images; **should-have** for scientific formats and audio; **later** for the long tail (GIS formats, CAD formats, exotic scientific formats, proprietary document formats).

---

### 3.2 Dataset / Tabular query workflows (GroupBy, aggregation, joins)

**What it is.** Two overlapping systems in modern Mathematica: `Dataset` (a generic wrapper over nested association/list data supporting a functional query language via `Query`, `GroupBy`, `Select`, etc., that generalizes beyond flat tables to hierarchical/ragged data) and the newer `Tabular` type (introduced to give a more conventional, column-typed, pandas/polars-like dataframe with dedicated functions like `AggregateRows`, better suited to strictly rectangular data and reportedly faster for that case). A real OpenMat needs the rectangular-dataframe case to be excellent (that's 95% of real usage) and can treat the fully general hierarchical `Dataset`/`Query` system as a stretch feature.

**Core use cases:**
- `data = Import["sales.csv", "Dataset"]` then `data[Select[#Revenue > 1000 &]]` or `data[GroupBy["Region"], Mean, "Revenue"]` - filter, group, and aggregate in a chained, readable pipeline, this is the bread-and-butter data-analysis workflow.
- Column selection and transformation: `data[All, {"Name", "Revenue"}]`, `data[All, <|"RevenuePerUnit" -> #Revenue/#Units &|>]`.
- Aggregation with multiple summary stats per group: total, mean, count by category, the single most common "give me a summary table" ask.
- Joins across two tables on a key column, common when combining a lookup table with a main dataset.
- Displaying nicely: a `Dataset`/`Tabular` object renders as a formatted, scrollable table in the notebook automatically, no separate print-formatting step, this display integration matters as much as the query semantics for perceived quality.

**Implementation difficulty: medium.** The rectangular-dataframe case (a `Tabular`-equivalent) is medium: column-oriented storage, typed columns, group-by/aggregate/join/filter operations are all well-trodden dataframe-engine territory with strong prior art to copy from. The fully general `Dataset`/`Query` system over arbitrary nested/ragged data is harder (medium-to-hard) because it needs to generalize cleanly over irregular structures, and it leans heavily on the core symbolic language's pattern-matching, making it a genuine cross-cutting concern with the CAS-engine spec.

**OSS building blocks:** This is another very strong OSS-leverage area. Polars (Rust, Arrow-based, fast, permissively licensed) is an excellent architectural and even literal-library candidate to build the `Tabular`-equivalent on top of or bind directly. Pandas is the more "battle-tested API surface" reference if familiarity to Python users matters. Apache Arrow as the underlying columnar memory format is worth adopting outright for interop with the rest of the data-science world (zero-copy handoff to/from numpy/pandas/polars in a bridging story, see section 4.6). Building the rectangular case directly on Polars (via bindings, if OpenMat's host language allows) could close most of this gap almost immediately, leaving OpenMat's job mostly as API-surface/display-integration design to match Mathematica's ergonomics.

**Priority: must-have** for the rectangular case (import, filter, group, aggregate, join, nice display); **should-have/later** for the fully general hierarchical `Dataset`/`Query` semantics over ragged data.

---

### 3.3 Units (Quantity)

**What it is.** `Quantity[value, unit]` makes physical units first-class: arithmetic automatically checks dimensional consistency, `UnitConvert` converts between compatible units (including to SI base units by default), and units flow through plotting, statistics, and general computation rather than being stripped out as bare numbers.

**Core use cases:**
- `Quantity[5, "Meters"] + Quantity[3, "Feet"]` - mixed-unit arithmetic that just works and returns a sensible unit, while `Quantity[5,"Meters"] + Quantity[3,"Seconds"]` throws a clear dimensional-mismatch error, this "catches your unit bugs for you" property is the core value proposition, especially loved in engineering/physics teaching contexts.
- `UnitConvert[Quantity[100, "Kilometers"/"Hours"], "Miles"/"Hours"]` for everyday conversions, extremely common as a quick calculator use case independent of any larger program.
- Attaching units to a dataset column and having downstream stats/plots respect and label them automatically (e.g. axis labels that include the unit).
- Symbolic dimensional analysis: checking that a derived formula is dimensionally consistent before trusting a numeric result, a real workflow in physics coursework.

**Implementation difficulty: easy-to-medium.** The unit-conversion-table part is easy (data entry: define a set of base dimensions and conversion factors, this is a static data problem, not an algorithmic one). Medium comes from making arithmetic dimension-aware everywhere consistently (every arithmetic operator, comparison, and relevant function needs a Quantity-aware code path or a clean generic dispatch mechanism), which is more a core-language integration question than a numerics-specific one.

**OSS building blocks:** `pint` (Python) is a very strong, mature, permissively-licensed reference implementation, both its unit-definition data files and its dimensional-analysis approach are directly reusable as a model (and its unit definition text files could plausibly be adapted/ported wholesale, license permitting, as a huge shortcut over hand-authoring hundreds of unit conversions). `units` (the classic Unix tool) and its database is another good data source for conversion factors.

**Priority: should-have.** Loved by engineering/physics users and relatively cheap once the conversion-factor database exists, but not on the critical path the way plotting or linear algebra is; safe to sequence after the must-haves.

---

### 3.4 Dates and times

**What it is.** `DateObject`, `DateList`, arithmetic on dates (`DateObject[...] + Quantity[3, "Days"]`), formatting/parsing (`DateString`, flexible free-form date parsing from strings), time zones, and `TimeSeries` for date-indexed data with resampling/alignment operations.

**Core use cases:**
- `DateObject["2024-03-15"]` parsing a wide variety of date string formats without the user specifying a format string every time, and displaying nicely.
- Date arithmetic: `DateObject[{2024,1,1}] + Quantity[45, "Days"]`, or computing the difference between two dates in a chosen unit, common in scheduling/finance-adjacent scripts.
- `TimeSeries` construction from a list of `{date, value}` pairs, with `TimeSeriesResample`, moving averages, and alignment against another series with different sampling, common in any time-indexed dataset (financial, sensor, weather).
- Plotting: `DateListPlot` handles date-typed x-axes with sensible automatic tick formatting (year/month/day granularity chosen based on the span), avoiding the classic "plot dates as raw numbers" pitfall.
- Time zone-aware arithmetic and conversion, relevant for any data pipeline combining sources from different regions.

**Implementation difficulty: easy-to-medium.** Calendar arithmetic itself is a solved, if fiddly, problem (leap years, month-length edge cases, time zone/DST rules) with mature libraries in every ecosystem. The medium part is free-form date string parsing (inferring format from ambiguous input) and integrating date types cleanly into the plotting/stats pipeline so `DateListPlot` and `TimeSeries` operations feel first-class rather than bolted on.

**OSS building blocks:** The IANA time zone database (tzdata) is the standard, universally-used source of truth for time zone rules, adopt directly. Any mature date/time library in the host language (e.g. `chrono` in Rust, `java.time`-style libraries, `dateutil`-equivalent for flexible parsing) covers the core arithmetic; `dateutil`'s free-form parser specifically is a good reference for the "guess the format" parsing UX.

**Priority: should-have.** Necessary for a complete data-handling story (especially for `TimeSeries`/finance/sensor use cases) but not as universally load-bearing as core numerics or plotting; reasonable to sequence just after the must-haves.

---

### 3.5 Geographic data and GeoGraphics basics

**What it is.** Symbolic geographic entities (`Entity["City", "Boston"]`-style knowledge-base lookups), geo computation (distance, area, geodesics), and `GeoGraphics`/`GeoListPlot`/`GeoRegionValuePlot` for rendering data on maps with automatic basemap tiles, projections, and geo-aware layout.

**Core use cases:**
- `GeoListPlot[{city1, city2, city3}]` plotting a set of named places on an automatically-fetched, reasonably-projected map, no manual basemap/projection setup.
- `GeoDistance[city1, city2]` computing real-world distance between two named places or coordinate pairs, using correct geodesic math (not flat-Euclidean approximation), a common one-off utility use case.
- Choropleth-style plots: `GeoRegionValuePlot[<|"California" -> 39, "Texas" -> 30, ...|>]` coloring regions (states/countries) by a data value, a very common data-journalism/reporting visualization.
- Free-form entity lookup: typing a place name and getting back a resolvable geo entity with associated data (population, coordinates, boundary polygon) without the user managing a separate geocoding API call, this "built-in knowledge base" aspect is genuinely hard to replicate and is really a core-language/knowledgebase concern more than a numerics one.

**Implementation difficulty: medium for the graphics/math, hard-to-out-of-scope for the knowledge base.** Map projections, geodesic distance calculations, and rendering points/regions on a basemap are medium, well-documented GIS-adjacent engineering with strong open libraries. The "type a city name and get rich structured knowledge back" experience depends on Wolfram's curated Knowledgebase (populations, boundaries, administrative hierarchies for every place on Earth), which is a data-acquisition and maintenance problem far bigger than any single spec area, arguably out of scope for this numerics/data/viz spec and more a "does OpenMat build or license a knowledgebase" strategic question for the project as a whole.

**OSS building blocks:** GDAL/OGR for geospatial data reading and projection math (the standard OSS GIS toolkit, extremely mature). Natural Earth (public domain vector map data: coastlines, country/state boundaries) is a strong free substitute basemap/boundary dataset. OpenStreetMap data and tile servers for basemap tiles (mind attribution/usage-policy requirements for tile serving at scale). GeoPandas/Shapely (Python) are good architectural references for the geometry+data integration pattern. For the entity knowledge base specifically: Wikidata is the best realistic OSS substitute for structured place facts (population, coordinates, administrative hierarchy), though integrating it well is real work.

**Priority: later.** Valuable but narrower in audience than the core numerics/plotting/dataframe work, and the knowledgebase dependency makes full fidelity a much bigger undertaking than the rendering math alone; sequence after the must-haves and should-haves elsewhere in this spec. A minimal "plot lat/long points on a basemap tile" capability could be should-have if cheap to bolt onto the existing Graphics renderer (2.3) plus a tile-fetching library.

---

## 4. Statistics and ML

### 4.1 Descriptive statistics

**What it is.** `Mean`, `Median`, `StandardDeviation`, `Variance`, `Quantile`, `Correlation`, `Covariance`, `Skewness`, `Kurtosis`, and their weighted/grouped variants, all operating uniformly on plain lists, `Dataset`/`Tabular` columns, or `WeightedData`.

**Core use cases:**
- `Mean[data]`, `StandardDeviation[data]` as immediate one-liners on a raw list, the most basic and most frequent stats operation.
- `Quantile[data, {0.25, 0.5, 0.75}]` for quartiles/percentiles, common in exploratory analysis and directly feeding `BoxWhiskerChart`.
- `Correlation[x, y]` / `CorrelationMatrix[dataset]` for a quick relationship check between variables before deeper modeling.
- Grouped descriptive stats: mean/std by category, tying directly into the `GroupBy`/`Tabular` aggregation workflow in section 3.2, this composition (stats functions as the aggregator inside a groupby) is the natural real-world usage pattern, not a standalone stats call.

**Implementation difficulty: easy.** All standard formulas, no algorithmic novelty, just needs numerically stable implementations (e.g. Welford's algorithm for variance to avoid catastrophic cancellation on large datasets) and consistent dispatch across the various container types (list, Tabular column, weighted data).

**OSS building blocks:** NumPy/SciPy `stats`, or any standard statistics library, cover this completely; effectively a full solve, this is not a place to spend design effort.

**Priority: must-have.** Cheap, foundational, expected by every user on day one.

---

### 4.2 Distributions framework (parametric distributions as first-class objects)

**What it is.** Distributions (`NormalDistribution[mu, sigma]`, `PoissonDistribution[lambda]`, `BinomialDistribution[n,p]`, and around 100+ others spanning continuous/discrete/multivariate/derived families) are symbolic objects, not just sampling functions. You can compute `PDF[dist, x]`, `CDF[dist, x]`, `Mean[dist]`, `Variance[dist]` symbolically or numerically, generate random variates (`RandomVariate[dist, n]`), and combine/transform distributions symbolically (`TransformedDistribution`, sums of independent random variables). This symbolic-first design (a distribution is an expression you compute properties of, not just a black-box sampler) is the key architectural idea to preserve, it's what lets the same object feed plotting, fitting, and hypothesis testing uniformly.

**Core use cases:**
- `dist = NormalDistribution[0, 1]; PDF[dist, x]` returning a symbolic formula usable in further calculus/plotting (`Plot[PDF[dist,x],{x,-4,4}]`), not just a numeric evaluator, this symbolic-formula return is a real point of pride/differentiation.
- `RandomVariate[dist, 1000]` for simulation/Monte Carlo work, expected to be fast and statistically correct (good underlying RNG and correct inverse-transform/rejection sampling per distribution family).
- `EstimatedDistribution[data, NormalDistribution[mu, sigma]]` - fit distribution parameters to observed data via MLE, a very common "does my data look normal, and with what parameters" workflow.
- Comparing empirical data to a fitted distribution visually: `Histogram[data, "PDF"]` overlaid with `Plot[PDF[fitted, x], {x, min, max}]`, extremely common exploratory step tying together sections 2.2 and 4.2.
- Derived/combined distributions: `TransformedDistribution[x + y, {x \[Distributed] dist1, y \[Distributed] dist2}]` for propagating uncertainty through a formula, a more advanced but real use case in engineering/uncertainty-quantification contexts.

**Implementation difficulty: medium for the common ~20-30 distributions with full PDF/CDF/moments/sampling; hard to fully replicate the "100+ distributions including exotic derived/multivariate ones with full symbolic manipulation" breadth.** Each individual distribution's PDF/CDF/quantile/sampling formulas are well-documented (any stats reference or SciPy source covers them). The volume is real work but not conceptually hard for standard families. What's hard is the fully general symbolic layer: `TransformedDistribution` needs to derive the distribution of an arbitrary function of random variables symbolically, and `EstimatedDistribution`/general MLE fitting for arbitrary user-defined distributions is genuinely nontrivial numerically. A pragmatic scope: nail the ~30 most-used named distributions (Normal, Uniform, Exponential, Poisson, Binomial, Gamma, Beta, Chi-Square, Student-t, F, Weibull, LogNormal, etc.) with full PDF/CDF/moments/sampling/fitting, and treat the symbolic-transform machinery and long tail of exotic distributions as later work.

**OSS building blocks:** SciPy's `stats` module is an extremely strong, near-complete reference and building block, it implements ~100 distributions with pdf/cdf/ppf/fit/rvs methods already, and is the single best "port this" target for the distribution-object layer. The main gap SciPy doesn't close is Mathematica's symbolic-first design (SciPy distributions are numeric objects/functions, not manipulable symbolic expressions), that symbolic wrapper layer is OpenMat-specific work, but the numeric core underneath it does not need to be reinvented.

**Priority: must-have** for the ~30 common named distributions with PDF/CDF/mean/variance/RandomVariate/basic fitting; **should-have** for less common named distributions; **later** for full symbolic transform/derived-distribution machinery.

---

### 4.3 Hypothesis testing

**What it is.** `TTest`, `ChiSquareTest`, `ANOVATest`, `KolmogorovSmirnovTest`, `CorrelationTest`, and similar functions that take data (and sometimes a null-hypothesis distribution or a second sample) and return a p-value or a full report (test statistic, p-value, degrees of freedom) depending on the requested `"ReportType"` .

**Core use cases:**
- `TTest[data1, data2]` for a quick two-sample comparison, the most common single hypothesis test in intro-stats/lab-report contexts, expected to return just a p-value by default but support a detailed report on request.
- `ChiSquareTest[observed, expected]` for categorical/count-data comparison, common in intro-stats coursework and quality-control contexts.
- `DistributionFitTest[data, dist]` (goodness-of-fit test, e.g. Kolmogorov-Smirnov or Anderson-Darling) to test whether data plausibly comes from an assumed distribution, ties directly to section 4.2's fitting workflow.
- `CorrelationTest[x, y]` for significance-testing an observed correlation, common alongside the descriptive `Correlation` call in 4.1.

**Implementation difficulty: easy-to-medium.** Test statistics are standard textbook formulas; the only real subtlety is correctly computing p-values from the right reference distribution (needs the distributions framework in 4.2 as a dependency) and handling edge cases (small samples, ties, unequal variances triggering Welch's correction, etc.) the way a careful stats package does.

**OSS building blocks:** SciPy `stats` again covers essentially all of the common tests directly (`ttest_ind`, `chi2_contingency`, `f_oneway`, `kstest`, `pearsonr` with p-values, etc.), this is close to a full solve once the distributions layer (4.2) exists to supply reference distributions.

**Priority: should-have.** Important for the scientist/student audience but sits naturally just after descriptive stats and distributions are in place; not a day-one blocker the way plotting or linear algebra is.

---

### 4.4 Model fitting (LinearModelFit, NonlinearModelFit)

**What it is.** `LinearModelFit[data, {1, x, x^2}, x]` and `NonlinearModelFit[data, model, params, x]` fit a specified functional form to data via least squares, returning a rich fitted-model object (not just coefficients): the object supports `Normal[fit]` (get the formula back), `fit["RSquared"]`, `fit["ParameterConfidenceIntervals"]`, `fit["BestFitParameters"]`, residual plots, and direct use as a callable function, plus prediction bands. This "fit returns a queryable object with built-in diagnostics" pattern, not just a coefficient array, is the recurring Mathematica UX theme across this whole spec (see also Interpolation in 1.6 and Distribution objects in 4.2).

**Core use cases:**
- `LinearModelFit[data, x, x]` for simple linear regression, immediately followed by `fit["RSquared"]` and `Plot[fit[x], {x, min, max}]` overlaid on the data, an extremely common two-line "fit and visualize" workflow.
- `NonlinearModelFit[data, a Exp[-b x] + c, {a, b, c}, x]` for curve fitting a specified nonlinear model (exponential decay, sigmoid, etc.) with automatic use of the section 1.5 optimization machinery under the hood.
- Multiple linear regression: `LinearModelFit[data, {x1, x2, x1 x2}, {x1, x2}]` including interaction terms, common in intro-stats/econometrics coursework.
- Diagnostics: `fit["ParameterTable"]` (coefficients, standard errors, t-stats, p-values in one formatted table, mirroring what R's `summary(lm(...))` or statsmodels gives) and residual plots to check fit quality, this diagnostic-table output is specifically what students/researchers expect and compare against R/statsmodels.

**Implementation difficulty: medium.** Linear least squares itself is easy (direct linear algebra, section 1.2). Nonlinear fitting is medium and leans directly on section 1.5's optimization machinery (Levenberg-Marquardt for the least-squares case). The real work is the rich result object: computing confidence intervals and standard errors correctly (needs the covariance matrix of the estimator, which needs the distributions framework for the relevant t/F distributions), and making the object composably callable/plottable/queryable the way Mathematica's fitted-model objects are, consistent with the "solution as first-class object" pattern from Interpolation and NDSolve.

**OSS building blocks:** `statsmodels` (Python) is the best direct reference, it already produces exactly this kind of rich fitted-model object with `.summary()`, confidence intervals, and diagnostics, and is a strong architectural template even if not directly wrapped. SciPy's `curve_fit` (built on Levenberg-Marquardt) covers the nonlinear numerics directly.

**Priority: must-have** for basic linear and nonlinear fitting with core diagnostics (R-squared, standard errors, residuals); **should-have** for the full diagnostic table/confidence-interval polish.

---

### 4.5 Classify / Predict

**What it is.** Fully automated supervised ML: `Classify[trainingData]` and `Predict[trainingData]` inspect the data, pick preprocessing, pick a model family (logistic regression, random forest, gradient boosting, nearest-neighbor, neural net, etc.) via internal cross-validation, tune hyperparameters, and return a ready-to-use `ClassifierFunction`/`PredictorFunction`, all with zero configuration required (though every choice is overridable). It also transparently handles mixed input types: numeric, categorical, text, image, or sound features in the same training set.

**Core use cases:**
- `c = Classify[{"cat.jpg" -> "cat", "dog.jpg" -> "dog", ...}]` then `c[newImage]` - image classification with no manual feature engineering or model selection, the flagship "automated data scientist" demo.
- `p = Predict[{{1,2,3} -> 10, {2,3,4} -> 15, ...}]` then `p[{3,4,5}]` for regression on tabular features, with `p[input, "Distribution"]` giving a full predictive distribution (uncertainty), not just a point estimate, this "give me uncertainty, not just a number" default is a specific and well-regarded differentiator.
- `Classify[trainingdata, Method -> "RandomForest"]` for a user who wants to override the automatic model choice, still getting the automated preprocessing/validation harness around whatever model is chosen.
- `ClassifierMeasurements[classifier, testdata]` for automatic evaluation (accuracy, confusion matrix, ROC curve) on held-out data, a very common "how good is my model" follow-up step.
- Feature importance / interpretability queries on the trained model, useful in applied/business analytics contexts.

**Implementation difficulty: research-grade for full automation-and-quality parity; easy-to-medium if scoped as a thin AutoML layer over an existing library.** The individual algorithms (logistic regression, random forest, gradient boosting, k-NN, basic neural nets) are all well-covered by existing OSS libraries, that part is not hard. What's hard, and where Wolfram invested real research effort, is the automated pipeline: robust type/preprocessing inference, automatic model selection via internal cross-validation across many candidate families, automatic hyperparameter tuning, and doing all of this reliably across wildly different data types (tabular, image, text, audio) with good default behavior on messy real-world data with minimal user input. Getting this genuinely as good as Mathematica's is a multi-year research/engineering investment; getting something serviceable (auto-select among 4-5 solid sklearn-equivalent models via cross-validation, basic preprocessing inference) is medium.

**OSS building blocks:** scikit-learn covers essentially every classical algorithm needed and is the obvious base to build an AutoML layer on top of. Existing AutoML projects (auto-sklearn, TPOT, FLAML) are directly relevant prior art for the "automatically pick model + hyperparameters" orchestration layer, worth studying architecture rather than necessarily wrapping directly. For deep learning /image / text, bridging to PyTorch (see 4.6) rather than reimplementing is clearly the right call.

**Priority: should-have, scoped narrow, not must-have for MVP.** A student/scientist's day-one credibility bar is numerics and plotting, not automated ML; a basic `Classify`/`Predict` wrapper over scikit-learn with simple cross-validated model selection is a strong should-have (it's a beloved, demo-friendly feature) but the full research-grade automation is explicitly a later/ongoing investment, not a launch blocker.

---

### 4.6 Build native vs. bridge to existing ML ecosystems: honest assessment

The Wolfram Language's ML story is impressive precisely because it's fully integrated (symbolic, uniform across data types, notebook-native) but that integration was a decade-plus of dedicated engineering investment by a company with deep pockets and a captive audience. For OpenMat, the honest recommendation is:

- **Do not attempt to build a competing deep learning framework from scratch.** PyTorch and JAX are extraordinarily mature, GPU-optimized, and have enormous ecosystems (pretrained models, tooling, community knowledge). Trying to reimplement autodiff + GPU kernels + a training loop framework natively would consume the entire OpenMat project's resources for a worse result than just bridging.
- **Do bridge**, with a real, low-friction interop layer: a `PythonEvaluate`/foreign-function story (or an Arrow-based zero-copy data bridge, see section 3.2) that lets a user hand a `Tabular`/array off to scikit-learn, PyTorch, or JAX and get a result back into OpenMat's native types cleanly. This is analogous to how Mathematica itself ships `ExternalEvaluate["Python", ...]` for exactly this reason, even Wolfram doesn't try to out-build the Python ML ecosystem for deep learning specifically.
- **Do build natively** the classical-ML and statistics layer (distributions, hypothesis tests, linear/nonlinear model fitting, and a scikit-learn-backed `Classify`/`Predict` convenience wrapper): this is squarely in "well-understood algorithms, needs good API design and integration polish" territory, achievable by a focused OSS team, and it's what makes the notebook feel cohesive for 90% of statistics/data-science use cases that aren't deep learning.
- **The dividing line**: anything that's fundamentally a numerical algorithm with a closed-form or classical iterative solution (regression, classical distributions, decision trees, k-means, PCA, basic neural nets even) is worth building/wrapping natively for a seamless experience. Anything that's "state of the art deep learning requiring GPU kernels, massive pretrained models, or an actively-evolving research field" (LLMs, diffusion models, modern CV backbones) should bridge to Python's ecosystem rather than compete with it. This mirrors how most working data scientists actually operate today even inside pure-Python workflows (classical sklearn natively, deep learning via a dedicated framework).

---

## 5. Image, sound, and graph processing (brief, secondary priority)

### 5.1 Image as a first-class expression

**What it is.** `Image` objects are symbolic expressions like everything else, an image can be passed to functions, pattern-matched, displayed inline in a notebook automatically, and manipulated with a large library of operations (`ImageResize`, `ColorConvert`, `ImageFilter`, `EdgeDetect`, morphological operations like `Dilation`/`Erosion`, `ImageCompose`) that compose the way list/expression operations do elsewhere in the language.

**Core use cases:**
- `img = Import["photo.jpg"]` then directly `ImageResize[img, 300]`, `ColorConvert[img, "Grayscale"]`, `EdgeDetect[img]`, chained naturally, with the result auto-displaying inline, no separate "show" step.
- Basic morphology (`Dilation`, `Erosion`, `Closing`, `Opening`) for cleaning up binary/segmented images, common in intro image-processing coursework.
- `ImageCompose`/cropping/padding for building composite figures or simple data augmentation.
- Extracting numeric data from an image (`ImageData[img]` as a numeric array) to feed into the numerics/stats machinery elsewhere in this spec, this bridge between "image" and "just an array of numbers" is architecturally important and mirrors the Quantity/Tabular pattern of "special type that degrades gracefully to plain data."

**Implementation difficulty: easy-to-medium.** Standard raster operations (resize, color conversion, basic filters, morphology) are well-documented, solved computer-vision-101 problems with mature libraries in every ecosystem. The main work is making `Image` a proper first-class citizen in the language/notebook display pipeline (a language-and-notebook integration concern, touching the other two specs) rather than the pixel-processing algorithms themselves.

**OSS building blocks:** OpenCV covers essentially the entire classical (non-deep-learning) image-processing operation set already. Simpler codec/manipulation needs can go through a lighter library (e.g. libvips for fast resize/convert, or a bundled image-processing crate/library) if OpenCV feels heavy. This is a strong-leverage wrapping job, not new algorithm work.

**Priority: should-have.** Secondary to core numerics/plotting/data as scoped by this task, but cheap to get a solid baseline (resize, convert, basic filters, morphology) once an image codec library is already in place for section 3.1's Import/Export.

---

### 5.2 Sound / Audio (very brief, later priority)

`Audio`/`Sound` objects with playback, basic transforms, and the Fourier/signal-processing tie-in from section 1.7. Lower priority than Image for a scientist/student audience; standard codec libraries (libsndfile, FFmpeg for broader format coverage) close most of the format/decode gap. **Priority: later.**

### 5.3 Graph objects and network analysis

**What it is.** `Graph[{1->2, 2->3, ...}]` as a first-class expression with automatic layout (`GraphPlot`/the default `Graph` display uses a force-directed or other layout algorithm chosen automatically based on graph size/structure) and a large library of graph algorithms: shortest paths, centrality measures (degree, betweenness, closeness, eigenvector, PageRank), community detection (`CommunityGraphPlot`, `FindGraphCommunities`), connectivity, coloring, and graph-theoretic properties (planarity, bipartiteness).

**Core use cases:**
- `Graph[{1<->2, 2<->3, 3<->1}]` immediately auto-displaying with a reasonable layout, no manual node positioning required, mirroring the "just works" philosophy of the Plot family.
- `FindShortestPath[g, start, end]` for routing/pathfinding style problems, common in both CS coursework and applied network analysis.
- Centrality measures (`BetweennessCentrality[g]`, `PageRankCentrality[g]`) for identifying important nodes in a social/citation/dependency network, a very common social-network-analysis and applied-data-science use case.
- `FindGraphCommunities[g]`/`CommunityGraphPlot[g]` for clustering a network into densely-connected subgroups and visualizing the result with communities visually separated, popular in social-network and biological-network analysis.
- Building a graph directly from data (e.g. an edge list column in a `Tabular`), tying network analysis back into the data-handling workflow in section 3.

**Implementation difficulty: medium.** Core graph algorithms (shortest path, centrality measures, connectivity, basic community detection like Louvain/label propagation) are all textbook, well-documented, and have mature reference implementations. Automatic, good-looking layout (force-directed layout that doesn't produce a tangled hairball for anything but the smallest graphs) is the part that takes real tuning to look as clean as Mathematica's default, similar in spirit to the plotting-aesthetics point in section 2.4.

**OSS building blocks:** NetworkX (Python) is a complete algorithmic reference covering essentially this entire feature list already, though its default layouts are notably less polished than Mathematica's. graph-tool or igraph (both have permissively-licensed C/C++ cores) are faster alternatives worth considering for the computational core if performance at scale matters. For layout specifically, Graphviz's algorithms (or a from-scratch force-directed implementation, which is genuinely not hard, a few hundred lines) plus some default-styling attention closes the aesthetics gap.

**Priority: should-have.** Valuable, well-covered by existing OSS algorithmically, and relatively cheap given NetworkX-equivalent libraries exist to lean on; sequence after the core must-haves in numerics/plotting/data.

---

## MVP slice: the minimal numerics + plotting subset for day-one credibility

A scientist or student's first ten minutes with OpenMat should support, without any missing piece breaking the flow:

1. **`N[expr, digits]`** with machine precision solid and arbitrary precision working via an MPFR wrapper (exact significance-arithmetic fidelity not required yet).
2. **Dense linear algebra**: `LinearSolve`, `Eigenvalues`, `SingularValueDecomposition`, backed directly by LAPACK/Eigen.
3. **`Plot`** with real adaptive sampling and discontinuity handling, plus **`ListPlot`/`Histogram`/`BarChart`**, all sharing one good-looking default theme (section 2.4's design investment). This is the highest-leverage single item in the whole spec: it is the first thing every new user tries, and a bad-looking or dumbly-sampled plot kills the "feels like Mathematica" impression instantly, no amount of numerics correctness compensates for an ugly or broken-looking plot.
4. **`NDSolve`** for ODE IVPs (nonstiff and basic stiff, via a SUNDIALS/CVODE wrapper) returning a callable `InterpolatingFunction` that plugs directly into `Plot`.
5. **`FindRoot`/`FindMinimum`** for local nonlinear solving, backed by SciPy-equivalent optimizers.
6. **`Interpolation`** turning data into a callable function.
7. **`Import`/`Export`** for CSV, JSON, and PNG at minimum, plus a `Tabular`-equivalent with `GroupBy`/aggregate/filter (built on or modeled after Polars) so "load a CSV, filter it, plot it" is a three-line, good-looking workflow end to end.
8. **Descriptive stats + the ~30 core distributions** (`Mean`, `StandardDeviation`, `NormalDistribution`, `RandomVariate`, `PDF`/`CDF`) plus basic `LinearModelFit`.

Everything else in this spec (NIntegrate specialty methods, PDEs, Quantity, dates, geo, Classify/Predict, image/graph processing) is real and should ship, but is should-have/later relative to this slice.

### Biggest technical risks

- **Plot aesthetics and adaptive sampling (2.1, 2.4) are underestimated by engineers and are the single biggest perception risk.** It is tempting to treat "call a plotting library with sampled points" as a solved problem and move on; it is not what makes Mathematica's plots distinctive. The adaptive-sampling algorithm (curvature-based refinement, discontinuity/singularity detection so `Plot[Tan[x], ...]` doesn't draw spurious vertical asymptote lines) and the default color palette/theme/typography are both real, non-trivial design-and-engineering investments, and skimping on either is the most likely way this project "looks like an open source clone" instead of "feels like Mathematica." Budget real design time here, not just plotting-library integration time.
- **NDSolve method selection and robustness (1.4) is a deep rabbit hole.** Getting a basic RK45/BDF ODE solver working is easy; matching Mathematica's automatic stiffness detection, reliable event location inside adaptive steps, and graceful handling of the many DAE/PDE edge cases users will inevitably throw at it is a long tail with real research-grade corners (general PDE geometry especially). The risk is scope creep: teams chase full NDSolve generality and never ship a rock-solid ODE-IVP-with-events core. Recommend explicitly capping v1 scope at ODE IVPs + basic stiffness + WhenEvent, and treating DAEs/PDEs as clearly separate, later milestones.
- **Significance arithmetic (1.1) is a precision-vs-effort trap.** Full fidelity to Mathematica's arbitrary-precision error tracking is genuinely research-grade and arguably not fully specified even in Mathematica's own documentation (it's a heuristic system refined over decades, not a proven algorithm). The risk is either under-investing (silently wrong high-precision results erode trust fast in a numerics-focused audience) or over-investing (chasing exact parity with an undocumented proprietary heuristic wastes months). Recommend a pragmatic middle: correct arbitrary-precision arithmetic via MPFR with straightforward precision propagation and clear documentation of where OpenMat's model is simpler than Mathematica's, rather than silently claiming full parity.
- **The Dataset/Tabular split is a real design fork, not just a naming detail.** Mathematica itself has two overlapping systems (general `Dataset`/`Query` over ragged hierarchical data, and the newer more conventional `Tabular`) for reasons that reflect real tension between generality and performance/ergonomics for the common rectangular case. OpenMat should deliberately choose to nail the rectangular `Tabular`-equivalent first (where Polars-equivalent prior art is strong) and treat full generality over ragged/hierarchical data, which leans on deep core-language pattern-matching integration, as an explicit later phase, rather than trying to build one system that's excellent at both from day one.
- **Classify/Predict and the ML story generally risks over-scoping.** The temptation is to chase Wolfram's "automated data scientist" reputation directly; the honest read (section 4.6) is that this specific feature area is where an OSS project should most deliberately under-build and bridge to Python's ML ecosystem instead, both because the ceiling (matching Wolfram's decade of AutoML research investment) is very high and because the audience overlap with users who'd rather just use scikit-learn/PyTorch directly is large. The risk is spending disproportionate engineering effort here relative to its MVP importance.
