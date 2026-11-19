---
phase: 89
phase_name: "docs-astro-integration"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 16
  lessons: 9
  patterns: 4
  surprises: 5
missing_artifacts:
  - "UAT.md"
---

# Phase 89 Learnings: docs-astro-integration

## Decisions

### Diagram dedupe tracks tried keys, so a failing source is attributed to (...
Diagram dedupe tracks tried keys, so a failing source is attributed to (and reported once from) the first file holding it

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 89-01-SUMMARY.md

---

### Invalid fences are reported only through fence_issues; a second diagram-...
Invalid fences are reported only through fence_issues; a second diagram-render line is emitted only for valid-but-unrenderable fences

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 89-01-SUMMARY.md

---

### Temp manifest file is a hidden dotfile in the target directory, removed ...
Temp manifest file is a hidden dotfile in the target directory, removed on any write/rename error

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 89-01-SUMMARY.md

---

### render-diagrams walks docs/src (archive skipped, .md and .mdx), dedupes ...
`--render-diagrams` walks `docs/src` (archive skipped, `.md` and `.mdx`), dedupes by `fence::normalize`, renders each source once with the default `vd` prefix and writes `{schema_version:1, diagrams:[{source,svg}]}` sorted and byte-identical (sha256 equal across two runs from the repo root). Hard fence findings print `docs/src/<file>:<line>:<col>: <code>: <message>`, render failures print `diagram-render`, exit is 1, and the valid diagrams are still written.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 89-01-SUMMARY.md

---

### The built dist/reference/vox-diagram/index.html has exactly 2 class="vd-...
The built `dist/reference/vox-diagram/index.html` has exactly 2 `class="vd-figure"` figures (the page has 2 fences), ids `vd-reference-vox-diagram-1-*` and `-2-*`, 0 `<pre>`-wrapped figures, and 2 source blocks.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 89-01-SUMMARY.md

---

### crates/vox-diagram/tests/fixtures/fence-keys.json (23 hand-authored Comm...
`crates/vox-diagram/tests/fixtures/fence-keys.json` (23 hand-authored CommonMark cases) passes against the Rust key on the first run; plan 89-03 runs the same corpus through remark.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 89-01-SUMMARY.md

---

### docs-deploy.yml (push) and docs-quality.yml (push + pull_request) trigge...
`docs-deploy.yml` (push) and `docs-quality.yml` (push + pull_request) trigger on `crates/vox-diagram/**` and run `Pre-render VoxDiagram fences` from the repo root before `Build Starlight`; no concurrency/permissions/cache/timeout line was touched.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 89-01-SUMMARY.md

---

### Sequence call sites reach the floor through a private hpad (W=i64 wrappe...
Sequence call sites reach the floor through a private `hpad` (W=i64 wrapper over pad_x, saturating at the Half range) so there is still exactly one copy of the formula

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 89-02-SUMMARY.md

---

### start/end nodes keep their fixed 40 px per side (exceeds the floor under...
start/end nodes keep their fixed 40 px per side (exceeds the floor under the label cap)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 89-02-SUMMARY.md

---

### WIDE_FONT_PCT = 12 and pad_x(tw, base) = max(base, ceil(tw*12/200)) in l...
`WIDE_FONT_PCT = 12` and `pad_x(tw, base) = max(base, ceil(tw*12/200))` in `layout/text.rs` (integer only, negative width clamped, saturating).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 89-02-SUMMARY.md

---

### Call sites: node_size (rect, rounded, cylinder, diamond; actors via node...
Call sites: `node_size` (rect, rounded, cylinder, diamond; actors via `node_size`), `edge_label`, container `strut_w`, `seq::label_box`, `lanes::note_w`. `verify.rs` unchanged (its node rule is a lower bound).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 89-02-SUMMARY.md

---

### Go/no-go record
Go/no-go record: new "Padding floor (Phase 89)" bullet; the Verdana sentence now says 89-RESEARCH measured overflow on 4 px-padded edge and message labels (up to ~7 px per side).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 89-02-SUMMARY.md

---

### Dev refresh uses refreshContent() in astro:server:setup, not deleting As...
Dev refresh uses refreshContent() in astro:server:setup, not deleting Astro's data store (the planned D-16 mechanism was wrong, see Deviations)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 89-03-SUMMARY.md

---

### customSelectors for svg.vd-diagram is kept as planned although llms-full...
customSelectors for svg.vd-diagram is kept as planned although llms-full.txt is already free of SVG text without it (mutation-proven, see below)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 89-03-SUMMARY.md

---

### Pages are discovered from dist/ (figureRoutes), so Phase 90's migrated p...
Pages are discovered from dist/ (`figureRoutes`), so Phase 90's migrated pages join the checks with no spec edit

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 89-04-SUMMARY.md

---

### The review bundle holds two PNGs per figure and mode
The review bundle holds two PNGs per figure and mode: as shipped (0.6 floor, scroll-clipped) and `-natural` (full diagram lifted over the page chrome), because the shipped capture of a wide figure shows only its visible part

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 89-04-SUMMARY.md

## Lessons

### 1. [Rule 1 - Plan error] Built-HTML language-vox-diagram acceptance cann...
**1. [Rule 1 - Plan error] Built-HTML `language-vox-diagram` acceptance cannot be met literally**

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 89-01-SUMMARY.md

---

### Found during
**Found during:** Task 1 verification.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 89-01-SUMMARY.md

---

### Issue: Expressive Code re-processes the <pre><code class="language-vox-d...
**Issue:** Expressive Code re-processes the `<pre><code class="language-vox-diagram">` inside the figure into a framed block carrying `data-language="vox-diagram"`; the `language-vox-diagram` class is restored only by starlight-llms-txt's `improveExpressiveCodeHandling` when building `llms-full.txt` (89-RESEARCH Pitfall 5/6), never in the site HTML.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 89-01-SUMMARY.md

---

### Fix: Verified the equivalent facts on the built page: data-language="vox...
**Fix:** Verified the equivalent facts on the built page: `data-language="vox-diagram"` count 2 and `class="vd-source"` count 2. The `llms-full.txt` selector work belongs to plan 89-03.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 89-01-SUMMARY.md

---

### [Minor] lanes.rs uses hpad( rather than a literal pad_x(
**[Minor] `lanes.rs` uses `hpad(` rather than a literal `pad_x(`**: the plan's acceptance grep (`pad_x(` >= 1 in lanes.rs) is not literally met; `hpad` is a 3-line `W`-to-`Half` wrapper in `seq.rs` that calls `pad_x`, which is the plan's own instruction ("convert at the call"). One copy of the formula remains.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 89-02-SUMMARY.md

---

### [Process] Scratchpad log names collide across sessions; one stale log wa...
**[Process]** Scratchpad log names collide across sessions; one stale log was briefly read as mine. Re-ran with unique names; no results were affected.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 89-02-SUMMARY.md

---

### None to the plan's logic
None to the plan's logic. Additions: natural-size screenshots (`*-natural.png`) beside the shipped ones, the fit test logs the widest fill ratio, and the review capture hides the site chrome (`header, nav, aside`) so the lifted figure is not overlaid.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 89-04-SUMMARY.md

---

### Left-to-right container-title layout
**Left-to-right container-title layout.** Verifier judgement: defect, fix before Phase 90. Look at corpus 03 (`HarnessRiverPane` / `l`, staircase layout, bundled edges) against its pre-`07b66a573` render, then choose a gap-closure plan in 89 or a pre-step in 90.

**Context:** Verified in phase acceptance checks.
**Source:** 89-VERIFICATION.md

---

### Hosted docs-quality run
**Hosted docs-quality run.** The pre-render, build, apt fonts plus Chromium, and VoxDiagram browser steps must be green within 30 minutes.

**Context:** Verified in phase acceptance checks.
**Source:** 89-VERIFICATION.md

## Patterns

### Rust items in the new module are private/pub(crate); no public API or cr...
Rust items in the new module are private/pub(crate); no public API or crate edge changed

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 89-01-SUMMARY.md

---

### Pre-render in Rust, trust-but-parse in JS
Pre-render in Rust, trust-but-parse in JS: fail-closed SVG-to-hast parser for the writer grammar only

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 89-01-SUMMARY.md

---

### Retype the mdast node to a custom type (vdFigure) so no <pre> wrapper is...
Retype the mdast node to a custom type (vdFigure) so no <pre> wrapper is emitted

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 89-01-SUMMARY.md

---

### Manifest keyed by normalized fence source; atomic temp-file + rename wri...
Manifest keyed by normalized fence source; atomic temp-file + rename write

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 89-01-SUMMARY.md

## Surprises

### Estimate variance on 89-01
Estimate variance on 89-01: planned 100000 tokens vs 9700 actual (0.10x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 89-01-PLAN.md, 89-01-SUMMARY.md

---

### 1. [Rule 1 - Plan error] Built-HTML language-vox-diagram acceptance cann...
**1. [Rule 1 - Plan error] Built-HTML `language-vox-diagram` acceptance cannot be met literally**

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 89-01-SUMMARY.md

---

### Estimate variance on 89-02
Estimate variance on 89-02: planned 70000 tokens vs 6500 actual (0.09x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 89-02-PLAN.md, 89-02-SUMMARY.md

---

### Estimate variance on 89-03
Estimate variance on 89-03: planned 95000 tokens vs 7500 actual (0.08x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 89-03-PLAN.md, 89-03-SUMMARY.md

---

### Estimate variance on 89-04
Estimate variance on 89-04: planned 95000 tokens vs 14000 actual (0.15x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 89-04-PLAN.md, 89-04-SUMMARY.md

