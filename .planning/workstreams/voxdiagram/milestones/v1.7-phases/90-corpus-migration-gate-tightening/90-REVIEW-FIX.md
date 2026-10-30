---
phase: 90-corpus-migration-gate-tightening
fixed_at: 2026-10-27T12:00:00Z
review_path: .planning/workstreams/voxdiagram/phases/90-corpus-migration-gate-tightening/90-REVIEW.md
iteration: 1
findings_in_scope: 7
fixed: 7
skipped: 0
status: all_fixed
---

# Phase 90: Code Review Fix Report

**Fixed at:** 2026-10-27
**Source review:** .planning/workstreams/voxdiagram/phases/90-corpus-migration-gate-tightening/90-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 7 (the blocker CR-01 and all six warnings)
- Fixed: 7
- Skipped: 0
- Info: IN-04 is resolved as a side effect of WR-01. IN-01, IN-02 and IN-03 are not trivial and were left alone.

Worktrees were disabled (`workflow.use_worktrees: false`). Every fix and every verification ran in the main checkout `/Users/brbrainerd/dev/vox-voxdiagram` on `milestone/v1.7-voxdiagram`. The review itself was committed first as `aa5c5de3f`.

## Fixed Issues

### CR-01: An edge passes through a non-endpoint node and lands on node corners

**Files modified:**
- `crates/vox-diagram/src/layout/ports.rs` and `ports/tests.rs` (new)
- `src/layout/{mod,verify}.rs`, `verify/tests.rs`, `src/layout/tests.rs`
- `tests/support/clearance.rs` (new), `tests/support/mod.rs`, `tests/svg_corpus.rs`, `tests/svg_geometry_props.rs`
- Goldens: 10 under `svg/cases` and 10 under `svg/corpus`

**Commit:** e247c112e

**Applied fix:**
- **Port pass.** A new `layout::ports::route` runs in `place` after quantizing. It runs before the frames and before `verify`, so both see the final geometry. In the flow frame (mirrored for up, axes swapped for left and right), every edge end works like this:
  - It leaves the side that faces its next route point.
  - It sits at least 16 px from a corner.
  - It sits at least 12 px from the other ends on that side. Without this, two arrowheads into corpus 18's "Claim" node merged into one.
  - It turns halfway into the gap beyond the rank (the reviewer's design). That gap holds no node and, short of the next half rank, no label.
- **Line-of-sight straightening.** I added this because the reviewer's design alone looked worse on the figures. It spends the whole cross-axis offset inside a 30 px gap, so on corpus 03 several edges ran long and parallel just beside a frame border. Now each route is straightened between its anchors: the ports, the turns and its own label point. A shortcut is taken only when it stays at least 4 px clear of every node, title and other edge label. Collinear points are dropped.
- **Diamonds.** A diamond end keeps dagre's end, which `svg::paths` clips onto the outline, so its edges fan out. It is re-ported (within the middle half of its side) only when dagre's segment cuts a foreign node. Porting every diamond end to its vertex bundled all four Triage edges in corpus 13, so I backed that out.
- **Verifier.** It gains `Violation::EdgeThroughNode` (`node_crossing`), a **soft** rung preference beside `title_crossing`. `lay_out` prefers a rung with neither defect and otherwise draws the first valid rung, so a diagram never fails to render because of it.
  - `PortAtCorner` (`port_at_corner`) is a **test-only metric** (`#[cfg(test)]`). The ports already keep every end off the corners, and a wider rung would not move an end the ports leave alone. Escalating on it would only cost extra dagre runs and make the figure larger.
- **New SVG-level oracle** (`tests/support/clearance.rs`): it reads the drawn SVG and never consults the renderer.
  - `svg_corpus::corpus_edges_clear_foreign_nodes` checks the 30 corpus graphs and 11 `svg/cases` graphs. Red before: 20 graphs.
  - `geometry_holds` gains ceilings for edges through a node (2 of 128) and ends on a corner (4 of 128).
- **Unit tests:** ports in all four directions; side spreading; straightening; label anchors; diamonds; verify `node_crossing` and `port_at_corner`; the `lay_out` escalation double.
- **Mutations:**
  - Disabling the port pass and the rung preference fails the corpus oracle (20 graphs), the property run (17 edges through a node and 111 corner ends in 128 cases) and the escalation test.
  - Setting the spread gap to 0 fails both spread tests.

**Measured** (corner = an end within 12 px of a box corner on both axes; through = a segment through a node interior):

| Set | Through a node, before -> after | Ends on a corner, before -> after | Grazing (within 4 px), before -> after |
|---|---|---|---|
| Corpus (30 graphs) and `svg/cases` (11) | 3 edges (stem 18: 2, 21-nested: 1) -> 0 | 45 (30 corpus, 15 cases) -> 0 | 1 -> 0 |
| Property run, 1,024 generated graphs (3 runs after) | 110 cases -> 0 | 875 ends -> 0 | not measured |
| Property run, edges through a title | 6 -> 0 | | |

**Status:** fixed: requires human verification. This is a routing heuristic, and the line-of-sight straightening goes beyond the review's proposal. It changes the look of long edges: they become straight diagonals in place of dagre's aligned vertical chains. I judged the before/after renders by eye.

### WR-01: Container title band is the engine's spacing, not padding

**Files modified:** `crates/vox-diagram/src/layout/finish.rs`, `src/layout/tests.rs`, `src/layout/tests/titles.rs` (new); goldens `cases/{20-container,21-nested,22-stress-nested}.svg`, `corpus/{03,06,13}-*.svg`
**Commit:** 1b17a49c5
**Applied fix:**
- `finish::frames` now frames twice:
  1. First with the engine's titles.
  2. Then with each title snapped down to its content.
- **Where the snapped title goes:**
  - It sits `TITLE_GAP` (8 px) above the content under it.
  - When the content sits higher beside it, the title goes level with the content's top. This happens sideways, where the title tops one column, and it keeps the Phase 89 "title on top of its frame" guarantee.
  - It never moves above both the engine's spot and that top.
  - When the snapped spot would touch an edge, a label, a node or a frame not holding the container, the title backs off 2 px at a time towards the engine's spot.
- **Why the first frames are a safe stand-in:** a snap only shrinks a frame, so the first frames are a superset for frames not yet final. Every rule `verify` applies to title boxes is checked before a title moves.
- **New oracle `titles::a_title_hugs_its_content`.** It covers every corpus graph and the container cases, sideways ones both ways. Each title must be within 8 px of its content top, unless it cannot move 2 px lower without touching something.
  - Red before: only 5 of 35 titles hugged.
  - 03, 06, 09, 13, 18, 22 and the cases had holes of 11 to 71 px.
  - Now 35 of 35 hug.
- **Mutation:** disabling the snap drops it back to 5 of 35. The new `snap` unit test covers a free snap, an edge that blocks it, a fully blocked spot and the sideways case.
- **Measured** (title bottom to content top, px): corpus 03 40/40/71/40 -> 8; 06 60 -> 8; 09 40 -> 8; 13 71 -> 8; 18 60 -> 8; 22 40/11/45.5 -> 8; 21-nested 52/52/40/40/40 -> 8.
- `CONTAINER_PAD` was not raised to 12 px. The review only suggested it, and with the title hugging, the 8 px top and bottom pads now balance.

**Status:** fixed: requires human verification. It is a layout heuristic.

### WR-02: A container fed through a nested child keeps the staircase

**Files modified:** `crates/vox-diagram/src/layout/dagre_impl.rs`, `src/layout/tests/feeds.rs`
**Commit:** f12ec3bf8
**Applied fix:**
- Entry nodes of nested containers now count when `title_links` collects feeders. An outside node that feeds one of them ranks before the outer title too.
- The unit test now expects `(n3, t0)`.
- New red-first test `a_container_fed_through_a_nested_child_lies_after_its_source`. It uses the review's reproduction, in both down and up.
- **Mutation:** without the nested entries, both tests fail. The fed frame comes back at x 340.5 beside its source, which is the review's staircase.
- No corpus golden changed.

### WR-03: The 48-cell word cap turned off wrapping for CJK

**Files modified:** `crates/vox-diagram/src/layout/text.rs`, `text/tests.rs`, golden `cases/02-wrap.svg`
**Commit:** d663d2663
**Applied fix:**
- The 48-cell allowance now applies only to words made of characters at most one cell wide. A word holding a wide character breaks at the line width.
- The CJK assertion is restored (15 x U+65E5 -> 14 + 1), plus a mixed Latin/CJK case.
- 02-wrap re-blessed: the CJK node wraps at 14 characters, and the figure is 1621.5 -> 1454.5 px wide. The reference page figure 05 shows the Japanese node wrapped onto two lines.
- **Mutation:** forcing the allowance for every word fails both.

### WR-04: The wrap property's bound was 48 cells for every line

**Files modified:** `crates/vox-diagram/src/layout/text/tests.rs`
**Commit:** cfabd9bc0
**Applied fix:**
- A line must fit `WRAP_CELLS`, unless it is a single narrow word. That word may run to `WORD_CELLS`.
- **Mutation:** packing with `word_cap` instead of the line limit fails the property.

### WR-05: `corpus_words_are_never_split` matched against all SVG text

**Files modified:** `crates/vox-diagram/tests/svg_corpus.rs`
**Commit:** 58a7bcbad
**Applied fix:**
- Each node is checked against its own `vd-label` lines (by owner index), never against `<desc>`, a title or another node.
- Words are filtered at 48 terminal cells, not 48 chars.
- **Mutation:** a 28-cell word cap fails the test on corpus 04 (`update_research_session_status:`).

### WR-06: The corpus freezer was destructive

**Files modified:** `crates/vox-diagram/tests/freeze_corpus.rs`
**Commit:** 5e27d7c3e
**Applied fix:**
- The freezer now collects every fence first. Before touching anything, it refuses to write a MANIFEST with fewer fences than the one on disk.
- Every I/O error is reported; the old `let _` calls ignored them.
- Run against the repo, it now stops with "freezer would drop 32 of 33 frozen fences", and the corpus is untouched.
- Two new non-ignored tests run on a scratch tree: one for the refusal (MANIFEST and `.mmd` files unchanged) and one for a normal freeze.
- **Mutation:** removing the guard fails the refusal test.

## Info

- **IN-04 (resolved by WR-01):** on corpus 13, the "Tier 3: Multi-Wave" and "Tier 2: Standard Deep" labels now sit above the engine frame, not on its title row.
- **IN-01, IN-02, IN-03 (skipped, not trivial):** these are prose duplicate-id reporting, re-wrapping and centring titles over the frame, and a byte-equality fidelity assertion. Corpus 06 still shows the IN-02 off-centre two-line titles.

## Verification

All gates ran in the main checkout.

- `cargo nextest run -p vox-diagram`: 619 of 619 pass. The known load flake did not fire.
- `cargo clippy -p vox-diagram --all-targets -- -D warnings`: clean.
- `cargo fmt --check -p vox-diagram`: clean.
- **Fidelity report** (`migration_fidelity`, ignored, run explicitly): passes. 32 fences are migrated and the Gantt is kept.
- `cargo nextest run -p vox-doc-pipeline -E 'test(diagrams) | test(mermaid_gate) | test(render_manifest)'`: 36 of 36 pass.
- `pnpm diagrams`: 38 diagrams from 842 files. `pnpm build`: 844 pages.
- **Playwright** `tests/vox-diagram.spec.ts` and `tests/build-output/diagrams.spec.ts`: 211 of 211 pass.
  - Minimum stroke contrast is unchanged: dark 4.49, light 6.06.
  - Wide-stack fit: 95.8%.
- **Contact sheets.** All 32 migrated stems were regenerated (`docs-astro/tests/migration-contact-sheet.spec.ts`, out `target/vd-migration/review-fix/`, 32 passed). I read 18, 13, 06, 03 and 09, plus the six reference page figures (the review bundle), by eye:
  - **18:** every edge leaves a bottom face and enters a top face as a straight diagonal. Nothing passes through or grazes a box, and the arrowheads into "Claim" are distinct. Both titles sit right above their nodes.
  - **13:** the diamonds keep their outline fan. Rect ends enter vertically. The engine frame starts below the tier labels.
  - **06 and 09:** frames are compact, and the titles hug their content.
  - **03:** three compact columns. Edges leave side faces and run straight across the gaps; there are no long runs beside frame borders.
  - **Reference figures 01 to 06:** no regressions. Figure 03's back edge now leaves the diamond's side and enters "Validate" off its corner. In figure 05 the Japanese node wraps (WR-03).
- **Not verified:**
  - Hosted CI has not run.
  - On corpus 12, the back edge "Subagents Distill -> Lead Researcher" now leaves its node's top face and crosses the "Spawn -> Subagents" edge near that node. It reads, but it is busier than before.
  - The column screenshot of corpus 13 shows the site's sticky header over the figure. This is a capture artifact of a tall figure, not the renderer.

---

_Fixed: 2026-10-27_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
