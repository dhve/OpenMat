---
name: OpenMat
description: The open-source, local-first, AI-native alternative to Mathematica. One notebook skin for the app and every surface that speaks for it.
colors:
  paper: "#ffffff"
  ink: "#1a1a1a"
  ink-soft: "#545454"
  ink-faint: "#9a9a9a"
  rule: "#c9c9c9"
  rule-soft: "#e6e6e6"
  accent: "#3b5fa4"
  accent-pressed: "#33538f"
  accent-ink: "#ffffff"
  accent-soft: "#eaf0fa"
  code-wash: "#fafafa"
  code-wash-inline: "#f6f6f6"
  freeform: "#c1652c"
  plot-green: "#4f8f57"
  error-bg: "#fdecea"
  error-border: "#e3a49c"
  error-ink: "#a3291d"
typography:
  display:
    fontFamily: "'Times New Roman', Times, Georgia, serif"
    fontSize: "clamp(52px, 7.5vw, 84px)"
    fontWeight: 700
    lineHeight: 0.98
    letterSpacing: "-0.015em"
  headline:
    fontFamily: "'Times New Roman', Times, Georgia, serif"
    fontSize: "27px"
    fontWeight: 700
  title:
    fontFamily: "'Times New Roman', Times, Georgia, serif"
    fontSize: "17px"
    fontWeight: 700
  body:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Helvetica Neue', Helvetica, Arial, sans-serif"
    fontSize: "15px"
    fontWeight: 400
    lineHeight: 1.55
  label:
    fontFamily: "'Courier New', Courier, Menlo, monospace"
    fontSize: "12px"
    fontWeight: 400
  label-lg:
    fontFamily: "'Courier New', Courier, Menlo, monospace"
    fontSize: "12.5px"
  code:
    fontFamily: "'Courier New', Courier, Menlo, monospace"
    fontSize: "13.5px"
  code-lg:
    fontFamily: "'Courier New', Courier, Menlo, monospace"
    fontSize: "14px"
  wordmark:
    fontFamily: "'Times New Roman', Times, Georgia, serif"
    fontSize: "19px"
    fontWeight: 700
  docs-display:
    fontFamily: "'Times New Roman', Times, Georgia, serif"
    fontSize: "clamp(34px, 5vw, 46px)"
    fontWeight: 700
  title-lg:
    fontFamily: "'Times New Roman', Times, Georgia, serif"
    fontSize: "21px"
    fontWeight: 700
  lead-serif:
    fontFamily: "'Times New Roman', Times, Georgia, serif"
    fontSize: "clamp(20px, 2.4vw, 26px)"
  lead-sans:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Helvetica Neue', Helvetica, Arial, sans-serif"
    fontSize: "16.5px"
  body-sm:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Helvetica Neue', Helvetica, Arial, sans-serif"
    fontSize: "13.5px"
  body-md:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Helvetica Neue', Helvetica, Arial, sans-serif"
    fontSize: "14.5px"
  control:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Helvetica Neue', Helvetica, Arial, sans-serif"
    fontSize: "13px"
  control-sm:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Helvetica Neue', Helvetica, Arial, sans-serif"
    fontSize: "14px"
  micro:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Helvetica Neue', Helvetica, Arial, sans-serif"
    fontSize: "11px"
  micro-plus:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Helvetica Neue', Helvetica, Arial, sans-serif"
    fontSize: "15.5px"
rounded:
  sm: "3px"
  bar: "7px"
spacing:
  cell: "7px"
  block: "22px"
  section: "40px"
  column: "46px"
components:
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.accent-ink}"
    rounded: "{rounded.sm}"
    padding: "11px 22px"
  button-primary-hover:
    backgroundColor: "#33538f"
    textColor: "{colors.accent-ink}"
  button-secondary:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    rounded: "{rounded.sm}"
    padding: "11px 18px"
  chrome-button:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    rounded: "{rounded.sm}"
    padding: "5px 13px"
  freeform-marker:
    backgroundColor: "{colors.freeform}"
    textColor: "{colors.accent-ink}"
    rounded: "{rounded.sm}"
    size: "20px"
---

# Design System: OpenMat

## Overview

**Creative North Star: "The Evaluated Page"**

Every OpenMat surface is a Mathematica notebook page: white paper, black ink, typeset math, and the quiet furniture of a scientific document. The desktop app defined the world (`app/src/styles/theme.css` is the canonical token source), and the landing page proved that the same skin extends to marketing without borrowing anything from launch-page convention. There is no dark hero, no gradient, no card grid. The page header is the app's own chrome bar, the footer signs off as `Out[3]=`, and the feature list is a ruled reference card.

The system is flat, dense with meaning, and almost entirely monochrome. Two chromatic voices carry all the signal: the classic notebook blue (#3b5fa4) for labels, evaluation, selection, and the primary action; and burnt orange (#c1652c) for exactly one idea, natural language input. Structure is drawn, never floated: hairline rules and right-edge cell brackets mark extent, and nothing sits on a shadowed card.

**Key Characteristics:**
- Paper-white ground with a near-black ink scale; color is rare and semantic
- Times-family serif for display voice, Courier-family mono for In/Out machinery, system sans for prose
- Flat, borderless cells; the right-edge bracket is the unit of composition
- Hairline rules (1px, #c9c9c9 / #e6e6e6) do all separation work
- The orange `=` marker means "this is English" everywhere it appears

## Colors

A white-and-ink document palette with one working blue, one reserved orange, and a muted plot cycle.

### Primary
- **Notebook Blue** (#3b5fa4): the classic Mathematica label blue. Carries In[n]:=/Out[n]= labels (at 0.82 to 0.85 opacity), hovered and selected cell brackets, links, focus outlines, the primary download button, the first plot curve, and slider accent-color. Hover state of filled blue deepens to #33538f.
- **Blue Wash** (#eaf0fa): `--accent-soft`; selection background and the hover fill of quiet buttons. The only tinted surface in the system.

### Secondary
- **Freeform Orange** (#c1652c): the natural language color. Fills the `=` marker chip, tints the "interpreted as" triple-bar glyph, the freeform bar focus border, the Go button, and the typing caret on the site. Inside plot output it may also appear as the second curve of the plot cycle (blue, orange, green), which is its Mathematica-native role; outside plots, orange in chrome always means English input.

### Tertiary
- **Plot Green** (#4f8f57): third curve of the muted plot cycle. Plot output only.
- **Error trio** (#fdecea background, #e3a49c border, #a3291d ink): app-side evaluation error messages.

### Neutral
- **Paper** (#ffffff): the only surface color; page, chrome, raised bars are all the same white.
- **Ink** (#1a1a1a): primary text.
- **Soft Ink** (#545454): secondary prose, descriptions, readouts.
- **Faint Ink** (#9a9a9a in the app; the site raises it to #767676 where faint text carries content on white and must stay legible): placeholders, footnote-grade metadata.
- **Rule** (#c9c9c9) and **Soft Rule** (#e6e6e6): hairline borders, cell brackets at rest, plot gridlines, column dividers.

### Named Rules
**The Orange Means English Rule.** Burnt orange (#c1652c) is reserved for natural language affordances: the `=` marker, the interpretation line, the freeform bar's focus and Go button. Its single exception is native, not decorative: it is also the second color of the plot curve cycle inside rendered output. Never use it for generic emphasis, warnings, or a second brand accent.

**The One Wash Rule.** #eaf0fa is the only background tint. Everything else sits directly on white.

## Typography

**Display Font:** Times New Roman (Times, Georgia, serif)
**Body Font:** system UI sans (-apple-system, Helvetica Neue, Arial)
**Label/Mono Font:** Courier New (Courier, Menlo, monospace)

**Character:** The Mathematica notebook voice: bookish serif titles over utilitarian sans prose, with blue Courier labels doing the machine's bookkeeping. Nothing geometric, nothing branded; it reads like a typeset scientific document.

### Hierarchy
- **Display** (700, clamp(52px, 7.5vw, 84px), 0.98, -0.015em): the masthead wordmark-as-title. Site only.
- **Headline** (serif 700, 27px on the site, 28px as the app's Title cell): section-opening heads ("Drag it. The math answers.").
- **Title** (serif 700, 17px to 18px): subsection and reference-column heads; blue in the site's reference card, and the app's Section cell.
- **Body** (sans 400, 13.5px to 15px, 1.5 to 1.55): prose, facts, dd descriptions. Measure capped around 44 to 62ch.
- **Label** (mono 400, 12px to 12.5px, Notebook Blue at ~0.85 opacity): In[n]:=/Out[n]= labels, version tags, cta-note metadata, interpreted lines. Code itself is mono at 12.5px to 14px in ink.
- **Wordmark** (serif 700, 19px): the OpenMat name in app chrome headers.
- **Docs Display** (serif 700, clamp(34px, 5vw, 46px)): interior page titles (the language reference), deliberately quieter than the masthead.
- **Lead** (serif clamp(20px, 2.4vw, 26px) for taglines; sans 16.5px for freeform sentences).
- **Control** (sans 13px to 14.5px): chrome buttons, footers, install notes, reference-table body.
- **Micro** (sans 11px to 12px): SVG plot tick labels only.

### Named Rules
**The Blue Label Rule.** Every In[n]:=/Out[n]= label is Courier mono, 12px, Notebook Blue at reduced opacity, right of nothing and left of the content it numbers. Labels are furniture: user-select is off.

**The Serif Speaks, Sans Explains Rule.** Serif is reserved for titles, section heads, and italic math variables (the slider's *c*). Running prose is always the UI sans. Mono never carries prose.

## Layout

A single centered sheet, like the notebook column: the app notebook is max-width 860px with 26px side padding; the site widens the sheet to 1060px with the same 26px gutters. Sections stack vertically, each closed by a full-width 1px rule (#c9c9c9), with roughly 40px of padding above and below the seam. Grid is used sparingly and structurally: the masthead is a two-column 1.05fr/1fr split with a 46px gap, the reference card is four equal ruled columns, install is a 1fr/1fr pair. The live cell uses a 76px label column, echoing the app's 58px cell-label gutter.

Rhythm inside a section runs on small steps: 6 to 14px between related lines, 22 to 26px before a new block, 7px between cells in the app. Breakpoints observed: 900px (masthead stacks, refcard drops to two columns) and 560px (single column, chrome tag hidden, gutters tighten to 16px). The app chrome header is sticky at top; the app's freeform bar is fixed to the bottom over a white fade.

## Elevation & Depth

Flat by doctrine: the app theme literally sets `--shadow-cell: none`. Depth is conveyed by drawn structure (hairline rules, brackets, the plot frame) and by the ink scale, never by lift. The single exception is floating chrome: the docked freeform bar and its symbol palette carry `box-shadow: 0 3px 14px rgba(0, 0, 0, 0.09)` because they genuinely float over content.

### Shadow Vocabulary
- **floating-bar** (`box-shadow: 0 3px 14px rgba(0, 0, 0, 0.09)`): only for fixed elements layered over the page (the docked natural language bar).

### Named Rules
**The Flat Paper Rule.** Content never casts a shadow. If an element needs a boundary, draw a hairline; if it needs extent, give it a bracket. Shadows are permitted only on chrome that physically overlaps the page.

## Shapes

Rectilinear and near-square. The system radius is 3px (`--radius`), applied to buttons, chips, code blocks, and the `=` marker; the freeform bar rounds to 7px; nothing else curves except the 10px circular slider thumb dot in plot-adjacent UI. Borders are 1px hairlines. The signature silhouette is the right-edge cell bracket: a 7px-wide, three-sided 1.5px rule (top, right, bottom, radius 0 3px 3px 0) hugging the cell's right edge, stretched top-to-bottom of its cell. In the app it is interactive: rest state #c9c9c9, hover and selected turn Notebook Blue, selected thickens to 2px, collapsed to 2.5px. Brackets nest, a per-row bracket inside an outer group bracket, echoing Mathematica's cell-group grammar.

**The Bracket, Not the Card Rule.** A cell's extent is marked by its right-edge bracket, never by a box, background, or shadow around the content. Cells are transparent and flush against the paper.

## Components

### App Chrome / Page Header
Sticky white bar, 1px #e6e6e6 bottom rule, 9px 22px padding. Serif 700 19px wordmark left; right side holds a mono version tag and a row of quiet buttons. Reused verbatim as the site's page header.

### Buttons
- **Shape:** near-square (3px radius), 1px border always present.
- **Primary (CTA):** Notebook Blue fill, white text, 600 weight, 11px 22px. Hover deepens to #33538f. Chrome-scale variant is 13px type at 5px 13px.
- **Secondary / quiet:** white ground, #c9c9c9 border, ink text. Hover: border turns blue, fill turns Blue Wash (#eaf0fa). App toolbar buttons are the ghost version: transparent border until hover.
- **Freeform Go:** outlined in orange, orange text, fills orange on hover. Orange buttons exist only inside natural language affordances.
- **Focus:** `outline: 2px solid` in the element's accent (blue, or orange in freeform contexts), offset 2px.

### Cells (the signature component)
Flat transparent block, right-edge bracket, mono blue label in a fixed left gutter (58px app, 76px site live-cell), content beside it. Input rows, output rows, and text rows share the frame; a group bracket spans In + controls + Out.

### Freeform Marker and Bar
The 20px orange chip holding a bold mono `=` marks every natural language entry point: prefix of a freeform cell, docked bar, site demo. The interpreted line renders in 12.5px mono soft-ink prefixed by an orange triple-bar glyph (U+2261). The docked bar: white, 1px #c9c9c9 border, 7px radius, floating-bar shadow, border turns orange on focus-within.

### Code Blocks
Mono 12.5px on #fafafa, 1px #e6e6e6 border, 3px radius, 14px 16px padding; comments in faint ink. The only off-white fill in the system besides Blue Wash.

### Reference Card Columns
Equal columns separated by 1px #e6e6e6 vertical rules (horizontal rules when stacked). Blue serif 17px column heads; mono 12.5px terms (dt) with soft-ink sans descriptions (dd).

### Plot Output
SVG in the Out row: 1px ink frame at 0.55 opacity, #e6e6e6 gridlines, 11px sans tick labels in soft ink, 2.25px blue curve, dashed 1px orange envelope. The plot cycle is blue, burnt orange, green, all muted.

### Inputs / Fields
Borderless and transparent on the paper (freeform bar input, app text cells); focus is a border or outline shift to the contextual accent, never a glow. Placeholders in faint ink. Range sliders use `accent-color: var(--accent)`.

### Motion
Small and quick: 0.1s to 0.12s ease for border-color, background, and color state changes. The site's generated-notebook cascade lands cells with a 0.5s cubic-bezier(0.16, 1, 0.3, 1) rise, staggered 0.25s, playing once on scroll; the caret blinks at 1.1s steps(1). All decorative motion is disabled under prefers-reduced-motion.

## Do's and Don'ts

### Do:
- **Do** put every surface on pure white (#ffffff) with the ink scale; reach for Blue Wash (#eaf0fa) only as a hover or selection tint.
- **Do** mark cell-like content with the right-edge bracket (7px, 1.5px #c9c9c9 hairline, 0 3px 3px 0 radius) and a mono blue label.
- **Do** separate sections and columns with 1px hairline rules, #c9c9c9 for major seams, #e6e6e6 for minor ones.
- **Do** use serif 700 for every heading level and keep prose in the system sans at 13.5 to 15px.
- **Do** pair every orange element with a natural language meaning: the `=` chip, the interpreted line, the freeform focus state.
- **Do** honor prefers-reduced-motion for any animation beyond a state transition.

### Don't:
- **Don't** put cards, background panels, or shadows around content; `--shadow-cell` is none, and only floating chrome may cast the 0 3px 14px shadow.
- **Don't** use orange for emphasis, alerts, or branding outside natural language affordances and the plot curve cycle.
- **Don't** introduce a second surface color, gradients, or a dark section; the world is one continuous sheet of paper.
- **Don't** exceed the 3px system radius on content elements (7px is reserved for the docked bar; circles only for the slider thumb and insert-plus dot).
- **Don't** brand the language feature as AI in any surface; it is "natural language input", marked by the orange `=`.
