# OpenMat Feature Interview

Live interview with Vedh and team on feature details, run against the open decisions in specs 01-03. Recorded round by round. Interviewer flags disagreements inline.

## Round 1: Strategic frame

**Q1. Primary wedge user for year one?**
Answer: **Students and educators.** Calculus/physics teaching, symbolic homework, interactive demos. Tolerates an incomplete CAS, plays to the reactivity story.

**Q2. Wolfram Language compatibility stance?**
Answer: **WL-shaped with core compatibility.** Same syntax and semantics for the core (patterns, Hold attributes, evaluation loop). No promise that arbitrary .nb files run. Keeps Rubi porting on the table without committing to 6000-function parity.

**Q3. Kernel implementation language?**
Answer: **Rust.** Chosen for packed-array and pattern-matcher performance, WASM/browser story, contributor signal. Accepted cost: slower initial velocity, numerics libraries (SUNDIALS etc.) need FFI bindings rather than coming free.

**Q4. Day-one undeniable demo?**
Answer: **All three eventually, sequenced ODE-first.** Order: (1) ODE-to-plot workflow ("this is killer"), (2) then work down the list (Manipulate slider + plot, symbolic homework). Explicit quality bar: nothing ships that reads as a cheap knockoff.

**Interviewer flag (resolved in round 2):** Tension between Q1 and Q4. The wedge is students/educators, but the ODE-to-plot demo was framed in the specs as the scientist's smoke test; the education spec ranked Manipulate reactivity as the highest-leverage feature for teaching. Resolved: unified demo (see round 2, Q1).

## Round 2: Architecture and licensing

**Q1 (follow-up on round 1 flag). Unify ODE-first with the education wedge?**
Answer: **Yes.** The flagship demo is a Manipulate-wrapped NDSolve: a slider drives a parameter (e.g. damping coefficient on a pendulum) and the solution curve re-solves and re-renders live. Hits the ODE-first instinct and the teaching wedge in one artifact, and forces the reactivity plumbing early, which the notebook spec says cannot be retrofitted cheaply.

**Q2. Front-end architecture?**
Answer: **Desktop app, firmly.** (Team asked what Tauri is; interviewer to explain and confirm the shell choice in round 3.)
**Interviewer flag (open):** Desktop-first sits in tension with the classroom wedge; school Chromebooks and zero-install were the argument for web. Proposed reconciliation in round 3: a desktop shell built on web tech, so the same UI can ship in the browser later without a rewrite.

**Q3. License?**
Answer: **Dual MIT + Apache-2.0** (the Rust ecosystem convention). Consequences accepted: Rubi (MIT) is portable; Mathics3 and Symja (GPL family) can be studied for design but their code cannot be copied.

**Q4. Team strengths and staffing?**
Answer: Team of five (Vedh, OmkumarSolanki, JTP75, Pizzawookiee, Raman18crypto) covers all four workstreams (Rust/systems, web front-end, numerics, CAS/math). Development approach: subagent-driven development, either continuing in this session or a dedicated build session. Execution plan to be settled at interview close.

## Round 3: Shell, solver, rendering, execution

**Q1. App shell (Tauri explained: Electron alternative in Rust; UI is web tech in the OS webview, backend is a Rust process, ~10MB binaries, kernel links in-process; same UI can later ship in a browser with a WASM kernel)?**
Answer: **Tauri.** Desktop now, browser path preserved for classrooms later, one UI codebase.

**Q2. ODE engine sourcing for NDSolve's first slice?**
Answer: **SUNDIALS from day one** (CVODE via FFI).
**Interviewer flag (resolved in round 4):** conflicts with the WASM/browser path that motivated Tauri; see round 4 Q1.

**Q3. Output rendering for the MVP?**
Answer: **Full 2D input too**: typeset output AND visual 2D input (click into a fraction and edit).
**Interviewer flag (resolved in round 4):** spec rates 2D input hard and should-have; see round 4 Q2.

**Q4. Path from interview to execution?**
Answer: **Build now, in this session**, subagent-driven. Roadmap and issues can follow from the build rather than precede it.

## Round 4: Disagreement follow-ups

**Q1 (interviewer pushback). SUNDIALS vs the WASM path: adopt a Solver trait with dual backends?**
Answer: **Yes, dual backend.** Solver trait from day one. SUNDIALS (CVODE) is the default engine in desktop builds; a pure-Rust backend (Dormand-Prince RK45, diffsol for stiff) compiles in for WASM builds. Both round 3 choices survive.

**Q2 (interviewer pushback). Stage 2D input behind the demo, or gate the demo on it?**
Answer: **Gate the demo on 2D input.** Team overruled the interviewer: the flagship demo does not ship until visual equation editing works end to end. Interviewer position recorded (staging was recommended to avoid the knockoff failure mode); team accepts the schedule risk for the higher first-impression ceiling. Mitigation: build 2D input on MathLive (MIT-licensed web component) rather than hand-rolling.

## Decisions summary

These decisions are authoritative and supersede any conflicting recommendation in specs 01-03 (see issue #1). The single MVP scope derived from them is [m0-milestone.md](m0-milestone.md).

| Decision | Call |
|---|---|
| Wedge user | Students and educators |
| WL compatibility | WL-shaped, core semantics compatible, no full-parity promise |
| Kernel language | Rust |
| Flagship demo | Manipulate-wrapped NDSolve: slider-driven pendulum damping, live re-solve, gated on 2D input |
| Shell | Tauri (React/TS web-tech UI, Rust kernel in-process) |
| ODE engine | Solver trait; SUNDIALS/CVODE default on desktop, pure-Rust RK45/diffsol for WASM |
| Math rendering | Typeset output plus MathLive-based 2D input, both demo-gating |
| License | Dual MIT + Apache-2.0 |
| Execution | Subagent-driven build, started same session as interview |
