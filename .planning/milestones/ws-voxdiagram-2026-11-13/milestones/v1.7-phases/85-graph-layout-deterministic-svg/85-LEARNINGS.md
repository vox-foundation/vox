---
phase: 85
phase_name: "graph-layout-deterministic-svg"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 34
  lessons: 12
  patterns: 9
  surprises: 5
missing_artifacts:
  - "UAT.md"
---

# Phase 85 Learnings: graph-layout-deterministic-svg

## Decisions

### Property and unit tests for text live in the sibling src/layout/text/tes...
Property and unit tests for text live in the sibling src/layout/text/tests.rs (as src/lexer.rs does), keeping text.rs at 134 non-blank lines

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 85-01-SUMMARY.md

---

### <desc> and the 24-char endpoint labels are cut to at most the stated len...
<desc> and the 24-char endpoint labels are cut to at most the stated length INCLUDING the ellipsis (23 or 299 chars plus U+2026)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 85-01-SUMMARY.md

---

### Cylinder is one rect (rx = half width, ry = 8) plus a vd-node-rim path, ...
Cylinder is one rect (rx = half width, ry = 8) plus a vd-node-rim path, so every outline stays a single rect or polygon for the 85-04 checker

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 85-01-SUMMARY.md

---

### Tracer (Task 1)
Tracer (Task 1): `svg::render` on `graph { node a: "A"  node b: "B"  a -> b }` gives `00-minimal.svg` byte for byte: `role="img"`, `aria-labelledby="vd-title vd-desc"`, `viewBox="0 0 96 168"`, no width/height, `<desc>` `2 nodes, 1 edge: A → B`, arrow marker, every paint a `var(--vd-*,fallback)` pair from `THEME_VARS`, no `<style>`/`foreignObject`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 85-01-SUMMARY.md

---

### Fail-closed paths
Fail-closed paths: bad `id_prefix` -> `InvalidIdPrefix`; 101 boxes or 201 edges -> `TooLarge` before layout; dangling endpoint or duplicate node id -> `InvalidIr("edge 0: unknown endpoint")`; engine panic / NaN / missing coordinate -> `Layout(..)` (tested through `Engine` doubles because dagre itself never panics).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 85-01-SUMMARY.md

---

### Expansion (Task 2)
Expansion (Task 2): `wrap` (explicit `\n`, 28 cells, words split only on space/tab/CR, hard break only for an over-long word, zero-width followers stay with their base), the full width model (ASCII Arimo table, U+00A0-017F 600, wide 1000, emoji/VS16 1300, zero-width 0) and `node_size` for rect, rounded, diamond (corner-containment rule), cylinder, start, end; `shapes.rs` draws them.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 85-01-SUMMARY.md

---

### Dependencies
Dependencies: `Cargo.lock` diff is +14/-0 (one `dagre 0.1.1` package, three vox-diagram dependency lines); `cargo hakari generate --diff` printed nothing (rc 0); `cargo deny check licenses bans sources` -> bans ok, licenses ok, sources ok.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 85-01-SUMMARY.md

---

### See frontmatter key-decisions
See frontmatter `key-decisions`. Also: `LayoutInput` carries no `containers` field yet (85-03 declares `LContainer` and adds it); `input::build` refuses a labelled edge (`edge N: labels are not drawn yet`) or any container (`container 0: ...`) with `InvalidIr` until 85-02 / 85-03 land, as the plan prescribes, so nothing is silently dropped.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 85-01-SUMMARY.md

---

### verify padding rule for rectangular shapes is w >= tw + 4*PAD_X and h >=...
verify padding rule for rectangular shapes is w >= tw + 4*PAD_X and h >= th + 4*PAD_Y half-pixels (the nominal node_size padding, which every shape's node_size satisfies); diamond uses tw*h + th*w <= w*h

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 85-02-SUMMARY.md

---

### Edge label box is the wrapped text block plus 4 px each side; the engine...
Edge label box is the wrapped text block plus 4 px each side; the engine receives that box, the label rect is placed from dagre's centre read-back and the input size

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 85-02-SUMMARY.md

---

### The vd-edge-labels group is emitted only when at least one edge has a la...
The vd-edge-labels group is emitted only when at least one edge has a label, so the 85-01 goldens are byte-unchanged

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 85-02-SUMMARY.md

---

### shapes::label is shared (class and fill parameters) so edge labels reuse...
shapes::label is shared (class and fill parameters) so edge labels reuse the single tspan/baseline layout

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 85-02-SUMMARY.md

---

### Task 1: LEdge carries Option<LLabel>; the dagre adapter passes the label...
Task 1: `LEdge` carries `Option<LLabel>`; the dagre adapter passes the label box with `LabelPos::Center` and reads the label centre back; `place` quantizes it to a top-left `Rect` and includes label rects in the margin/viewBox bounds. The 85-01 labelled-edge refusal is gone (a labelled edge renders Ok; blank or whitespace labels become `None`). `svg::paths` has `rounded` (radius `min(8 px, leg1/2, leg2/2)`, legs by `i64::isqrt`, offsets by integer `d*r/len`, repeated points dropped, collinear points just get `L`), `clip_diamond` (`p = c + d*w*h / (2|dx|h + 2|dy|w)` in `i64`, a point on or inside the outline is untouched) and the edge and edge-label drawing. Draw order is edges, nodes, edge labels. Four goldens blessed and inspected (all path coordinates inside the viewBox, diamond endpoints on the outline, self-loop and parallel edges distinct).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 85-02-SUMMARY.md

---

### Task 2: layout::verify (exact integer checks, no epsilon, no label text ...
Task 2: `layout::verify` (exact integer checks, no epsilon, no label text in messages). `lay_out` loops `RUNGS` in order, `guarded` then `place` then `verify`, returns the first Ok, returns a `Layout` error at once, and returns `Geometry(<last violation>)` when all three rungs fail. Soft defects (edge through a non-endpoint node, label touching a node) are deliberately not checked.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 85-02-SUMMARY.md

---

### Container titles use a 16 px line pitch (the band is 10 px + 16 px per l...
Container titles use a 16 px line pitch (the band is 10 px + 16 px per line), so `shapes::label` now takes a `(class, fill)` pair and a line step; node and edge labels keep 18 px and their goldens are byte-unchanged

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 85-03-SUMMARY.md

---

### text::wrap_at(label, cells) added next to wrap; the title re-wrap tries ...
`text::wrap_at(label, cells)` added next to `wrap`; the title re-wrap tries 27 down to 1 cells per line until the widest line fits the final inner width (container width minus 20 px)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 85-03-SUMMARY.md

---

### The vd-containers group is emitted only when the diagram has containers,...
The vd-containers group is emitted only when the diagram has containers, so every earlier golden is byte-unchanged

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 85-03-SUMMARY.md

---

### A parent cycle and over-deep nesting share one InvalidIr message
A parent cycle and over-deep nesting share one InvalidIr message: the depth walk stops after MAX_DEPTH hops, which a cycle always exceeds

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 85-03-SUMMARY.md

---

### Labels over MAX_LABEL_CHARS (nodes, containers, edges) are InvalidIr bef...
Labels over MAX_LABEL_CHARS (nodes, containers, edges) are InvalidIr before layout; edge-label length is checked after endpoint resolution

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 85-03-SUMMARY.md

---

### Task 1: build resolves containers through a BTreeMap (unknown or duplica...
Task 1: `build` resolves containers through a `BTreeMap` (unknown or duplicate id is `InvalidIr`), derives depth from parent links, caps labels, and the 85-01 container refusal is gone. The dagre adapter adds `c{i}` containers and a `t{i}` strut leaf per container. `layout::finish::frame` pads the quantized engine rect and adds the title band; `place` includes container frames in the viewBox extent and the margin shift. `svg::shapes::write_container` draws the frame (`rx=6`, container fill/stroke variables) and the title centred in the band. Goldens `20-container` and `21-nested` blessed and inspected numerically: every node is inside its container rect, every nested container inside its parent, each child container starts below the parent's title band (Middle layer band ends at y=396, Innermost container starts at y=440), sibling containers (Outer/Sibling/Hollow/Lengthy) are disjoint, the 60-char title wrapped to two lines inside its frame, and the empty container is 297x174 px.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 85-03-SUMMARY.md

---

### Task 2: five containment checks on exact integer geometry (touching allo...
Task 2: five containment checks on exact integer geometry (touching allowed, ancestors followed through resolved parent indices with a bounded walk) plus containers in the view-box check. `ContainerAlwaysBad` double gives `Geometry(OutsideContainer(0))` after exactly three engine calls. `svg_errors` covers unknown node container, unknown parent, duplicate container id, two-container cycle, self-parent, five nested levels (4 is Ok), 201-char node/container/edge labels (200 is Ok), 60 nodes + 41 containers as `TooLarge { nodes: 101, .. }`, and an edge to a container id as `InvalidIr` ("edge 0").

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 85-03-SUMMARY.md

---

### hostile_graph_cases_render
`hostile_graph_cases_render`: every `graph_cases()` entry renders Ok. No finding to report.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 85-03-SUMMARY.md

---

### No src/ change
No `src/` change: every new test passed against the 85-01..03 code, so the writer needed no fix

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 85-04-SUMMARY.md

---

### Svg gained root_children, ids and paints (every style prop:value), and L...
`Svg` gained `root_children`, `ids` and `paints` (every style `prop:value`), and `Label` gained `class` and `owner`, so tests can tie a label to the shape it sits in

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 85-04-SUMMARY.md

---

### theme::pane_css takes a Theme enum and scopes to .pane-dark / .pane-ligh...
`theme::pane_css` takes a `Theme` enum and scopes to `.pane-dark` / `.pane-light`; `theme::contrast` takes `#hex` or `hsl(..)` strings

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 85-04-SUMMARY.md

---

### REND-03 is ticked (all its clauses are delivered and mutation-proven); R...
REND-03 is ticked (all its clauses are delivered and mutation-proven); REND-02 (macOS and Linux bytes), REND-04 (browser view of both themes) and REND-01 stay open for 85-05

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 85-04-SUMMARY.md

---

### Task 1 (REND-03)
Task 1 (REND-03): `svgcheck::check(svg, prefix)` rejects any element outside {svg,title,desc,defs,marker,path,rect,polygon,text,tspan,g}, any attribute or class outside the fixed sets, comments/CDATA/PI/DOCTYPE, duplicate or unprefixed ids, unresolved `url(#..)` and `aria-labelledby` tokens, entities other than amp/lt/gt/quot/#39 (raw attribute text and `Event::GeneralRef`), raw `>` and control chars in text (quick-xml accepts raw `]]>`), non-whitespace text outside `title`/`desc`/`tspan`, `url(` in a style and style properties outside {fill, stroke, stroke-width, font-family, font-size}. `svg_hostile` (7 tests): checker self-tests (20 rejection cases plus the 00-minimal golden and a second-root document), every `FRAGMENTS` entry (and `x`+f+`y`) as node label, edge label and `SvgOpts.title` with decoded text equal to the independent `sanitize`, all `graph_cases()` under prefixes `vd` and `h-1`, a 128-case property (1-8 picks of fragments or `any::<char>()`, 200 chars, node + labelled self-loop + title), `no_user_text_in_attributes` (marker `QzQ7` through node, container, both edges and title), and `structure_contract` (title then desc first, ids exactly `{prefix}-title/-desc/-arrow`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 85-04-SUMMARY.md

---

### Task 2 (REND-02, SC2)
Task 2 (REND-02, SC2): `twenty_fresh_renders` (00-minimal, 10-edges, 21-nested and corpus block 13 imported through `mermaid::import`: 20 renders, one distinct string, all Ok), `generated_graphs_render_identically` (64 cases), `geometry_holds` (128 cases through `TestRunner`, geometry read back from the SVG; printed `5 of 128 cases were Err(Geometry)`; that count varies by seed and is not asserted), and the guards `determinism_guard_no_transcendental_floats`, `determinism_guard_no_clock_or_version`, `layout_seam_guard_dagre_only_in_adapter` (4 tests now in the binary).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 85-04-SUMMARY.md

---

### Task 3 (REND-04)
Task 3 (REND-04): `paints_are_theme_vars` (every fill/stroke/font-family in all 10 case goldens is `none` or a `THEME_VARS` pair; no colour literal outside a `var()` fallback in any attribute), `all_theme_vars_used` (nine names), `one_markup_for_both_themes`, `contrast_matches_known_ratios`, `contrast_holds`, the map-covers-variables check and `pane_css` check. Measured ratios (all above threshold): dark text 17.70 / 15.10, muted 10.01, graphics min 4.47 (edge or node-stroke on container-fill); light text 17.70 / 15.29, muted 11.74, graphics min 6.07; fallback text 15.80 / 14.84, muted 6.39, graphics min 6.00. No `THEME_VARS` or map change was needed.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 85-04-SUMMARY.md

---

### GO on dagre 0.1.1
GO on dagre 0.1.1: 30 of 30 importable corpus graphs render Ok; 0 of 10 need a DSL tweak (threshold 2)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 85-05-SUMMARY.md

---

### Diagram 10 (edge crossing a container title) is a recorded residual risk...
Diagram 10 (edge crossing a container title) is a recorded residual risk, not a DSL tweak and not a renderer change here

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 85-05-SUMMARY.md

---

### Task 1: svg_corpus.rs with corpus_graphs_all_render (33 blocks split 30 ...
Task 1: `svg_corpus.rs` with `corpus_graphs_all_render` (33 blocks split 30 graphs / 1 sequence / 2 refused; each of the 30 checks, renders Ok and passes the independent allowlist checker), `corpus_top10_ranking` (frozen list, tenth scores 20, eleventh 19), `corpus_top10_goldens` (ten goldens, directory holds exactly those) and the ignored `contact_sheet` generator (asserts the dark SVG equals the light SVG after the id-prefix swap). The computed top 10 matched the frozen list and no graph returned `Err`, so nothing had to be stopped or reported. The sheet was generated twice: same SHA-1 `09e9ee01d81a1ad4a4e2b9624390992df3d6c925`, 171,257 bytes, 20 `class="vd-diagram"`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 85-05-SUMMARY.md

---

### Task 2: visual review, Linux comparison and the decision record (below).
Task 2: visual review, Linux comparison and the decision record (below).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 85-05-SUMMARY.md

---

### Task 3: where-things-live row, crate-api-sync --write, and the closing g...
Task 3: where-things-live row, `crate-api-sync --write`, and the closing gates.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 85-05-SUMMARY.md

## Lessons

### The first UPDATE_GOLDEN=1 run of Task 2 had shapes_case_has_one_polygon ...
The first `UPDATE_GOLDEN=1` run of Task 2 had `shapes_case_has_one_polygon` fail because nextest ran it in parallel with the blessing test; the rerun without the variable was green. Not a defect.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 85-01-SUMMARY.md

---

### A path can contain a zero-length L segment after a Q when two corners on...
A path can contain a zero-length `L` segment after a `Q` when two corners on a short leg meet at the same point (visible in the self-loop paths). It is valid SVG and deterministic; not suppressed to keep `rounded` small. Candidate tidy-up for a later plan if path size matters.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 85-02-SUMMARY.md

---

### Not measured
Not measured: a visual render of the two goldens (the shell allowlist blocks `qlmanage`); containment was checked from the SVG coordinates instead.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 85-03-SUMMARY.md

---

### The 21-nested layout in direction right is tall and sparse (1145x1247 px...
The 21-nested layout in direction `right` is tall and sparse (1145x1247 px) because dagre's compound LR layout stretches containers; valid under every invariant, recorded for the 85-05 contact sheet and decision doc.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 85-03-SUMMARY.md

---

### Generated S1 graphs fail closed (Err(Geometry)) in about 4% of cases (5 ...
Generated S1 graphs fail closed (`Err(Geometry)`) in about 4% of cases (5 of 128 in the recorded run, varying by seed). The contract allows it and the test prints the count; 85-05 should say so in the decision doc rather than claim every S1 graph renders.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 85-04-SUMMARY.md

---

### No visual check of the pane CSS (that is the 85-05 contact sheet)
No visual check of the pane CSS (that is the 85-05 contact sheet). `pane_css` is only string-tested here.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 85-04-SUMMARY.md

---

### The bash allowlist rewrote some sed/head calls on test paths; file reads...
The bash allowlist rewrote some `sed`/`head` calls on test paths; file reads were done with the Read tool instead.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 85-04-SUMMARY.md

---

### 85-04's note was honoured
85-04's note was honoured: the decision doc states the roughly 4% generated-graph `Geometry` fail-closed rate next to the corpus 30/30 result.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 85-05-SUMMARY.md

---

### Diagram 10's edge-through-title defect is the one thing a reader of the ...
Diagram 10's edge-through-title defect is the one thing a reader of the sheet will notice; it needs title-aware routing or a text halo, which is outside this plan (no renderer change was made to force a clean sheet).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 85-05-SUMMARY.md

---

### The bash allowlist blocks $VAR command names, sleep in the foreground an...
The bash allowlist blocks `$VAR` command names, `sleep` in the foreground and `bash -c`; long waits used a scratchpad wait script and plain literal commands.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 85-05-SUMMARY.md

---

### Hosted CI only
**Hosted CI only:** the macOS plus Linux byte-identity run on the milestone PR (I re-ran nothing on Linux; 85-05's Docker aarch64 and QEMU x86_64 results are recorded but not independently re-verified here, and QEMU is not real x86_64 hardware); the PR-level `crate-edges`, `vox-arch-check`, `hakari`, fmt and full ssot-drift gates.

**Context:** Verified in phase acceptance checks.
**Source:** 85-VERIFICATION.md

---

### Phase 89: real-browser mounting under the actual Starlight site, wide-fo...
**Phase 89:** real-browser mounting under the actual Starlight site, wide-font (Verdana/DejaVu) fit, and legibility policy for diagrams scaled below 60%.

**Context:** Verified in phase acceptance checks.
**Source:** 85-VERIFICATION.md

## Patterns

### engine seam + catch_unwind guard + quantize to integer half-pixels, no f...
engine seam + catch_unwind guard + quantize to integer half-pixels, no float after the seam

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 85-01-SUMMARY.md

---

### one writer: ordered (&'static str, &str) attribute slices, every value t...
one writer: ordered (&'static str, &str) attribute slices, every value through esc(.., true)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 85-01-SUMMARY.md

---

### every paint is var(--vd-NAME,FALLBACK) built from THEME_VARS
every paint is var(--vd-NAME,FALLBACK) built from THEME_VARS

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 85-01-SUMMARY.md

---

### ladder: guarded engine call, quantize, verify; first verified rung wins
ladder: guarded engine call, quantize, verify; first verified rung wins

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 85-02-SUMMARY.md

---

### edge geometry in i64 with isqrt and truncating integer division; no floa...
edge geometry in i64 with isqrt and truncating integer division; no float after quantize

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 85-02-SUMMARY.md

---

### container frame = quantized engine rect, padded and titled after quantiz...
container frame = quantized engine rect, padded and titled after quantize, then verified inside the same ladder

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 85-03-SUMMARY.md

---

### ancestor/depth walks bounded by a constant (MAX_DEPTH) or by the contain...
ancestor/depth walks bounded by a constant (MAX_DEPTH) or by the container count, so hand-built cyclic parents terminate

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 85-03-SUMMARY.md

---

### SVG oracle = quick-xml allowlist checker over the written bytes; decoded...
SVG oracle = quick-xml allowlist checker over the written bytes; decoded text compared with a `sanitize` restated in the test, never read from src

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 85-04-SUMMARY.md

---

### mutation proofs driven by a scratch script
mutation proofs driven by a scratch script: save copy, presence count before/mutated/restored, run named binaries, restore, `git diff --quiet`

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 85-04-SUMMARY.md

## Surprises

### Estimate variance on 85-01
Estimate variance on 85-01: planned 110000 tokens vs 15700 actual (0.14x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 85-01-PLAN.md, 85-01-SUMMARY.md

---

### Estimate variance on 85-02
Estimate variance on 85-02: planned 85000 tokens vs 10200 actual (0.12x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 85-02-PLAN.md, 85-02-SUMMARY.md

---

### Estimate variance on 85-03
Estimate variance on 85-03: planned 90000 tokens vs 13000 actual (0.14x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 85-03-PLAN.md, 85-03-SUMMARY.md

---

### Estimate variance on 85-04
Estimate variance on 85-04: planned 100000 tokens vs 13000 actual (0.13x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 85-04-PLAN.md, 85-04-SUMMARY.md

---

### Estimate variance on 85-05
Estimate variance on 85-05: planned 85000 tokens vs 14500 actual (0.17x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 85-05-PLAN.md, 85-05-SUMMARY.md

