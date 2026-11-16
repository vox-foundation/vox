---
phase: 90-corpus-migration-gate-tightening
reviewed: 2026-10-27T09:10:00Z
depth: deep
files_reviewed: 43
files_reviewed_list:
  - crates/vox-diagram/src/layout/dagre_impl.rs
  - crates/vox-diagram/src/layout/text.rs
  - crates/vox-diagram/src/layout/text/tests.rs
  - crates/vox-diagram/src/layout/tests.rs
  - crates/vox-diagram/src/layout/tests/containers.rs
  - crates/vox-diagram/src/layout/tests/feeds.rs
  - crates/vox-diagram/src/layout/finish.rs
  - crates/vox-diagram/src/layout/verify.rs
  - crates/vox-diagram/src/layout/mod.rs
  - crates/vox-diagram/src/svg/paths.rs
  - crates/vox-diagram/tests/migration_fidelity.rs
  - crates/vox-diagram/tests/freeze_corpus.rs
  - crates/vox-diagram/tests/svg_corpus.rs
  - crates/vox-diagram/tests/fixtures/import-cases/13-rust-crate-imports-stripped.mmd
  - crates/vox-diagram/tests/fixtures/import-cases-roundtrip/13-rust-crate-imports-stripped.vdg
  - crates/vox-diagram/tests/fixtures/import-cases-roundtrip/13-rust-crate-imports-stripped.export.mmd
  - crates/vox-diagram/tests/fixtures/import-cases-roundtrip/13-rust-crate-imports-stripped.sets.json
  - docs-astro/tests/lib/diagrams.ts
  - docs-astro/tests/migration-contact-sheet.spec.ts
  - docs-astro/tests/vox-diagram.spec.ts
  - contracts/documentation/legacy-mermaid-baseline.v1.yaml
  - docs/src/adr/012-internal-web-ir-strategy.md
  - docs/src/architecture/ai-first-fixtures-research-2026.md
  - docs/src/architecture/as-any-casts-reduction-strategy-research-2026.md
  - docs/src/architecture/axis-harness-river-design-2026.md
  - docs/src/architecture/deep-research-full-surface-audit-and-roadmap-2026.md
  - docs/src/architecture/deep-research-waves-and-competitive-analysis-research-2026.md
  - docs/src/architecture/gamification-ludus-review-findings-2026.md
  - docs/src/architecture/graphify-python-free-findings-2026.md
  - docs/src/architecture/mcp-vox-language-exposure.md
  - docs/src/architecture/scientia-automated-research-historical-extension-research-2026.md
  - docs/src/architecture/tauri-convergence-migration-plan-2026.md
  - docs/src/architecture/unified-workstream-execution-plan-2026.md
  - docs/src/architecture/vox-compiler-architecture-research-2026.md
  - docs/src/architecture/vox-gui-browser-support-2026.md
  - docs/src/architecture/vox-react-backend-interop-audit-2026.md
  - docs/src/ci/deploy-contract.md
  - docs/src/explanation/expl-durable-execution.md
  - docs/src/how-to/how-to-rust-crate-imports.md
  - docs/src/reference/mens-external-tech-options.md
  - docs/src/reference/mens-measurement-gap-analysis.md
  - docs/src/reference/populi.md
  - docs/src/reference/vox-diagram.md
findings:
  critical: 1
  warning: 6
  info: 4
  total: 11
status: issues_found
---

# Phase 90: Code Review Report

**Reviewed:** 2026-10-27T09:10:00Z
**Depth:** deep
**Files Reviewed:** 43 source, test and doc files (`git diff 455c2ffc7..HEAD`). Planning files, `*.generated.md`, reports, lockfiles and SVG/vdg goldens were excluded; goldens were sampled.
**Status:** issues_found

## Summary

**The migration itself is faithful.** I checked this independently, without relying on the SUMMARY claims. A script extracted every `vox-diagram` fence from the 21 docs, paired each with its MANIFEST entry, and compared it byte for byte with `corpus-roundtrip/<stem>.vdg`. Stem 29 was compared with `import-cases-roundtrip/13-rust-crate-imports-stripped.vdg`. All 32 are IDENTICAL, so every fence is the importer's output with no hand edits. The Gantt is the only Mermaid fence left, at deep-research-full-surface:530. `import-cases/13-rust-crate-imports-stripped.mmd` equals `tr -d '\140'` of the frozen block exactly. The baseline YAML holds exactly the Gantt entry. The Phase 84 canonical sets compare labels, shapes, container membership, edge labels and direction. No corpus fence uses dotted or thick edges or `classDef`/`style`, so nothing is lost outside those sets.

**The renderer is where the defects are.** I rendered all 33 corpus fences and the 23 `svg/cases` with `target/debug/vox diagram render` (built at HEAD) and measured the SVG geometry with a script:

- **Deferred class D-A ships a misleading figure.** On the scientia page, the "1960s-80s" edge cuts through the corner of the "2018-2023" box. It visually leaves from the same corner as that box's own edge. I rate this a BLOCKER (CR-01).
- **Deferred class D-B is a real asymmetry.** The gap between a container's title and its nodes is 39 to 70 px, against a fixed 8 px at the bottom. It comes from the engine's rank and node spacing, which no test measures (WR-01).
- **Fix f9ec97c97 misses nested containers.** I reproduced the staircase it was meant to remove when the outside edge feeds a nested child container (WR-02).
- **Fix e45074c5b has a side effect.** It also stops wrapping CJK text and other text with no spaces (WR-03).
- **Some tests are weaker than they look:** the proptest (WR-04), `corpus_words_are_never_split` (WR-05) and the now-dangerous freezer (WR-06).

**The scoped figure-id check is not too weak** for what it is meant to guard (IN-01).

Both deferred classes also break a rule the phase set for itself. CONTEXT says "A defect in the renderer is fixed in `crates/vox-diagram` with a red-first test, not worked around in the doc". It also lists "overlaps" as a defect. Plans 90-03, 90-04 and 90-05 recorded D-A and D-B as "observations" instead.

## Critical Issues

### CR-01: An edge passes through a non-endpoint node and lands on node corners, with no check for it (deferred D-A)

**Files:** `crates/vox-diagram/src/layout/verify.rs:1-6` (soft defects "deliberately not checked"), `crates/vox-diagram/src/layout/mod.rs:309-323` (`lay_out` rung selection), `crates/vox-diagram/src/layout/dagre_impl.rs:223-239` (dagre points taken as-is), `crates/vox-diagram/src/svg/paths.rs:92-116` (only diamonds get endpoint clipping).

**Evidence (live page `architecture/scientia-automated-research-historical-extension-research-2026`, stem 18):**

- Edge 0 (`E1 "1960s-80s" -> L2`) starts at (689, 144). That point is on the **left side** of its box (689..841 x 102..158), not the bottom. The first segment runs to (459, 187), a 230 px horizontal drop over 43 px. At x = 649 it is at y ≈ 151.5, so it cuts through the bottom-right corner of the "2018-2023" box (428..649 x 102..158) over about 36 px.
- Edge 4 (`E4 -> L3`) starts at (649, 152), which is exactly that corner. Two lines therefore leave the "2018-2023" corner, and the 1960s edge looks like it leaves from the wrong node.
- Edge 1 (`E2 -> L3`) cuts through the "2010s" box (881..1088) the same way.

This is a misreading of the data, not just a cosmetic flaw. That is why it is rated BLOCKER.

**How widespread it is (measured over the rendered SVG control points):**

| Measure | Corpus result | `svg/cases` result |
|---|---|---|
| Edge through a non-endpoint node's interior | 2, both in stem 18 | 1, in `21-nested` (edge 3 through node 0) |
| Edge within 4 px of a non-endpoint node | 3 | not measured |
| Endpoint within 12 px of a box corner | 33, across 15 stems | 10, at a 6 px tolerance |

Corner endpoints (12 px tolerance), by stem: 03:1, 06:1, 07:4, 08:1, 12:5, 13:2, 17:2, 18:2, 21:3, 22:2, 24:2, 26:2, 27:2, 31:2, 32:3. These figures sit on more than 10 live pages. The stem 13 "Run Sandboxed Compiler" edge ends at (405, 1468), on the top-right corner of node 17 ("Consolidated Knowledge Subgraph"). The earlier "~25 through a node" figure cannot be reproduced with a strict interior test. It matches the corner-endpoint count instead.

**Root cause (three parts):**

1. **dagre puts every long edge's first dummy point one half-rank away.** `makeSpaceForEdgeLabels` halves `ranksep` and doubles `minlen`. At rung 0 the first dummy is only `ranksep/2` = 30 px below the source rank. Brandes-Köpf x-assignment lines the dummy chain up with the target's column (in stem 18 the vertical runs at x = 451.5 / 883.5 / 1103 go straight down to the targets). So the whole horizontal offset is spent in that first 30-px half-rank. Fix f9ec97c97 made this more common: it adds a title rank between a feeder and the container it feeds, so every container-to-container edge is now a long edge with dummies.
2. **dagre's endpoint is where the ray from the node centre to the next point leaves the bounding box.** On a shallow first segment that ray leaves through the **side**, next to the corner. Then it keeps going through the neighbouring box in the same rank, which sits only `nodesep` (40 px) away. Nothing in `place` or `svg::paths` moves an endpoint off a corner; only diamonds are clipped.
3. **`verify.rs` ignores edges crossing nodes by design** (module doc, lines 2-3). `lay_out` never escalates rungs for it, and no corpus test measures it, so it shipped.

**Fix.** Add an integer port pass in `layout` after `place` (for example `finish::ports`) and before `verify`, so verification sees the final geometry:

```rust
// Top-down (mirror on y for Up, swap axes for Left/Right): exit through the side that faces the next rank,
// away from corners, then turn in the clear gap below the rank, never inside it.
let r = nodes[e.from];
let next = pts[1];
let inset = (r.w / 4).max(RADIUS_H);                       // keep ports off the corners
let x = next.0.clamp(r.x + inset, r.x + r.w - inset);
let rank_bottom = nodes.iter()
    .filter(|o| o.y < r.y + r.h && r.y < o.y + o.h)       // same rank = overlapping y-range
    .map(|o| o.y + o.h).max().unwrap_or(r.y + r.h);
if next.1 > rank_bottom {
    let turn = rank_bottom + (next.1 - rank_bottom) / 2;  // inside the node-free and label-free gap
    pts.splice(0..1, [(x, r.y + r.h), (x, turn)]);
}
// Same at the target end with the top side / rank_top.
```

Every node in the source rank ends at or above `rank_bottom`. Edge-label dummies sit at the half-rank, which starts `ranksep/2` lower. The vertical stub stays inside the source's own x-span, and the turn lies in a band that holds no node and no label. For stem 18, at least, the new path cannot hit a neighbour. Raising the rung alone would not do: at rung 1 (ranksep 90) stem 18 still crosses, by about 0.5 px. Rung 2 clears it but makes every gap taller, which makes D-B worse.

**Oracle that would have caught it.**

- Add `Violation::EdgeThroughNode(edge, node)` to `verify.rs` using the existing `crosses()`, checked against every node that is not an endpoint. Add `Violation::PortAtCorner(edge, node)` for an endpoint within `inset` of a rect corner.
- Wire both into `lay_out` as soft checks, like `title_crossing`: prefer a rung without them and accept the last one.
- Add a corpus test, `svg_corpus::corpus_edges_clear_foreign_nodes`, over `corpus_graphs()` and every `svg/cases` graph that asserts zero of each. It is red today on stems 18 and 21-nested.
- Prove the guard by mutation: with the port pass disabled, the test must fail on stem 18.

## Warnings

### WR-01: Container title band is the engine's spacing, not padding; nodes look pinned to the frame bottom (deferred D-B)

**Files:** `crates/vox-diagram/src/layout/finish.rs:91-98,113` and `crates/vox-diagram/src/layout/tests/containers.rs:15,97`

**Issue.** I measured title bottom to first node top, and last node to frame bottom, in the rendered corpus:

| Stem | Direction | Title band | Bottom gap |
|---|---|---|---|
| 06 | top-down | 59 px (both frames) | 8 px |
| 18 | top-down | 59 px (both frames) | 8 px |
| 13 | top-down | 70 px | 8 px |
| 03, 09 | left-to-right | 39 px | 8 px |
| 03 container 2 | left-to-right | 70 px | 8 px |
| 22 | left-to-right | 39 / 10 / 44 px | 8 px |

The bottom gap is the designed `CONTAINER_PAD` of 8 px, which is 4.8 px at the 0.6 scale floor. That is why nodes look as if they sit on the frame edge. The "~4 px" seen on stem 13 is that floor, not a smaller pad. The band comes from the engine:

- **Top-down:** the title is a dagre node in its own rank, so it sits `ranksep/2 + ranksep/2` (60 px at rung 0, 90 or 140 px at later rungs) above the content. In stem 13 the rank is also tall because the Tier 2/3 label dummies share it.
- **Sideways:** the title is stacked above the first column with `nodesep` (40 px).

`finish::frames` takes `content ∪ title` as given. It only ever moves a title *up*, and only for sideways layouts (line 92), so the hole stays in the frame. The Phase 89 compactness oracle (`MAX_GAP`) measures the frame against a children box that *includes the title*, so a hole between the title and the content cannot be seen by it.

Severity is WARNING: nothing is misread. But it is on all seven container figures across 5 live pages, and it looks unfinished.

**Fix.** In `finish::frames`, after `content` is known, snap the title toward the content in every direction. Set `title.y = content.y - TITLE_GAP - title.h` (top-down and sideways; the mirror for bottom-up), with `TITLE_GAP` around 8 px. Keep the move only if `clear_of_edges(moved, edges)` passes and it does not overlap any label in `touching`. Otherwise fall back to the dagre position. Consider raising `CONTAINER_PAD` to 12 px so the bottom does not read as touching at the 0.6 floor.

**Oracle.** Add `containers::title_hugs_its_content` over `corpus_graphs()` and the container cases. For each container, assert `first_content_top - title_bottom <= 2 * CONTAINER_PAD` unless a recorded edge or label blocks the snap. Also assert the band minus the bottom pad is at most `TITLE_GAP + CONTAINER_PAD`. It is red today on 03, 06, 09, 13, 18 and 22.

### WR-02: f9ec97c97 misses a container whose outside feeder enters through a nested child, so the staircase is still there

**File:** `crates/vox-diagram/src/layout/dagre_impl.rs:57-59,74-89`

**Issue.** `firsts`, and so `feeders`, come only from `own` nodes (`container == Some(k)`). When an outside node feeds a node inside a *child* of `k`, three links exist: `title(k) -> title(child)` and `feeder -> title(child)`. Nothing orders `title(k)` after the feeder, so the outer title can share the feeder's rank. Reproduced at HEAD with:

```text
graph {
    container src: "Source stage" { node A: "Alpha source node"  node B: "Beta source node" }
    container outer: "Outer platform" { container inner: "Inner services" { node C: "Gamma service"  node D: "Delta service" } }
    src.A -> src.B
    src.B -> outer.inner.C
    src.A -> outer.inner.D
}
```

The result is the same staircase the fix was meant to remove:

- The "Source stage" frame is y 16..276, x 16..264.5.
- The "Outer platform" frame is y 232..520, starting at x 340.5. Its title (y ≈ 254) shares B's rank (230..268), so the two frames sit side by side and overlap along the flow.
- The outer frame also has a 112 px band above the inner frame.

The unit test (line 306, the `n3 -> n2` case) covers exactly this shape but asserts only `(n(3), t(1))`. It locks in the missing `(n(3), t(0))`. `feeds.rs` tests only flat containers. No live corpus figure is a top-down nested container, so this is latent. Hence WARNING, not BLOCKER.

**Fix.** Compute entry nodes over every node `k` encloses: `inside(n)` instead of `container == Some(k)`, keeping the "no inside predecessor" rule. Then take feeders from those. Alternatively, also push `(far, Title(p))` for each ancestor `p` of `k` that does not enclose `far`. Update the unit test to expect `(n(3), t(0))`. Add a `feeds.rs` case with the source above as a red-first test: `outer.y >= src.y + src.h`.

### WR-03: The 48-cell word cap also turns off wrapping for CJK and other unspaced text

**File:** `crates/vox-diagram/src/layout/text.rs:126` (and the CJK test at `text/tests.rs:73-80`)

**Issue.** `word_cap = limit.max(WORD_CELLS)` treats any run without spaces as one "word". CJK sentences have no spaces. A Japanese or Chinese label of 15 to 24 characters (30 to 48 cells) used to wrap at 28 cells and now renders as one line up to 48 cells wide, about 1.7 times wider. Longer runs now break every 24 characters instead of 14. The rationale ("an identifier is never split, as in Mermaid") does not apply to CJK: Mermaid wraps those by width. The test was rewritten to pin the new behaviour (`half = WORD_CELLS / 2`) instead of protecting the old one. There is no CJK in the corpus today, so nothing live is affected.

**Fix.** Apply the 48-cell allowance only to words made entirely of 1-cell characters, for example `word.chars().all(|c| cell(c) == 1)`. Keep `limit` as the break width for words that contain wide characters. Restore the old CJK assertion (15 × U+65E5 → 14 + 1).

### WR-04: The proptest's line bound was loosened from 28 to 48 for every line, so packing regressions go unseen

**File:** `crates/vox-diagram/src/layout/text/tests.rs:213`

**Issue.** `prop_assert!(cells(l) <= WORD_CELLS)` now allows a 48-cell line made of several short words. Suppose a later change packed with `word_cap` instead of `limit` (the line `cells(&cur) + 1 + wc <= limit`). The property would still pass. Only `wrap_breaks_a_sentence_at_spaces` would catch it, for one fixed sentence.

**Fix.**

```rust
prop_assert!(cells(l) <= WRAP_CELLS || (!l.contains(' ') && cells(l) <= WORD_CELLS), "{l:?}");
```

### WR-05: `corpus_words_are_never_split` matches against every text in the SVG, `<desc>` and `<title>` included

**File:** `crates/vox-diagram/tests/svg_corpus.rs:103-129`

**Issue.** `shown` is a flat list of every word in `parsed.texts`. That includes the `desc` and `title` elements (`support/svgcheck.rs:270-274`), edge labels, container titles and other nodes. A word split in one node still passes if the same word appears whole anywhere else in the SVG, for example a repeated identifier or a short word echoed in `<desc>`. The filter also counts `chars()`, while the cap is in cells. The mutation proof in 90-01 showed the test fails for the corpus as it is today. It does not show that the check is tied to the node it reports.

**Fix.** Match per node: use `parsed.labels` filtered to `class == "vd-label"` and to the node's owner/index. Compare against that label's own lines, and filter on `cells(word) <= WORD_CELLS`.

### WR-06: The corpus freezer is now destructive whenever someone runs it, and only a doc comment guards it

**File:** `crates/vox-diagram/tests/freeze_corpus.rs:1-7,48-50`

**Issue.** The new comment says "Do not re-run the freezer mid-migration". After Phase 90 the hazard is permanent. A run deletes every `corpus/*.mmd` (line 50) and re-freezes only the Gantt. That removes the 32 Mermaid originals that the Node oracle, `mermaid_roundtrip` and `migration_fidelity` depend on, and the Mermaid source of the live figures exists nowhere else. Git can restore it, but the run itself fails silently: the `let _ =` calls ignore errors.

**Fix.** Make the freezer refuse to run when it would shrink the MANIFEST: `assert!(found.len() >= manifest_len, "freezer would drop N frozen fences; docs are migrated")`. Or retire it now that the corpus is closed, and update the comment to say so.

## Info

### IN-01: The scoped figure-id check is sound, but prose duplicate ids are now reported nowhere

**File:** `docs-astro/tests/vox-diagram.spec.ts:92-104`

**Not too weak.** It still flags any duplicate in which at least one element sits inside `.vd-figure`, whichever comes first in the document. That covers figure-to-figure and figure-to-prose collisions, and `closest()` works on SVG elements. The global `aria-labelledby` resolution check is unchanged, and the 90-02 mutation (an `h1` given the figure's title id) proves it fires.

**What was lost.**

1. The tauri page's 9 real duplicate anchors (`<a id="phase-N--…">` plus a heading slug with the same id) are a real anchor and accessibility defect, and no gate reports them now.
2. The check depends on the `vd-figure` class. A future pipeline that hoists shared `<defs>` (for example a marker sprite) outside the figure would not be checked.

**Suggestion.** Keep the scope. Log the prose duplicates as a non-blocking annotation, and file the tauri page anchors as a docs issue.

### IN-02: Container titles wrap at 28 cells whatever the frame width, and are not centred over the frame

**File:** `crates/vox-diagram/src/layout/input.rs:78`

Two examples in stem 06 (`b9/06-…-l238.png`):

- "Local Models: MENS / Ollama / Candle Metal / CUDA" wraps onto two lines inside a frame about 300 px wide.
- "OpenRouter / Frontier Cloud Models" wraps and sits over the left column, off the frame's centre. Its x is wherever dagre placed the title node.

Stem 05 shows the same thing (noted in 90-05). **Suggestion:** after `frames`, re-wrap the title to the frame's inner width and centre it on the frame. Ordering ranks still needs the dagre box.

### IN-03: The fidelity tool checks semantic equality, not "the fence is the importer's output"

**File:** `crates/vox-diagram/tests/migration_fidelity.rs:100-136`

`verify_migrated` compares canonical sets with the oracle-proven golden, plus an export and re-import round trip. That is the right fidelity oracle, but it would accept hand-renamed ids or a reordered fence. The "importer output" claim rests on the executors' manual `diff -q`. I re-ran that comparison for all 32 fences and they match. The test is `#[ignore]`d, so nothing runs it after the phase. **Suggestion:** if the guarantee should hold, add a byte-equality assertion against `corpus-roundtrip/<stem>.vdg` (the HAND table for stem 29). Otherwise leave as is.

### IN-04: Edge labels for edges entering a container are drawn inside its frame (stem 13)

**File:** `crates/vox-diagram/src/layout/finish.rs:99-112`

On the waves page, the "Tier 3: Multi-Wave" and "Tier 2: Standard Deep" labels sit inside the engine frame, on its title row. The `touching` loop grows the frame to take in any label that overlaps its padding. A reader can take them for part of the engine. This interacts with WR-01: snapping the title down would leave those labels in the band. **Suggestion:** after WR-01, test whether these labels can stay outside the frame.

---

## Verified clean

- **Fences:** 32 of 32 migrated fences are byte-identical to the importer goldens.
- **How-to import case:** the stem 29 block is exactly the frozen block with the backticks removed. Its `.vdg`, `.export.mmd` and sets are consistent, and only code formatting is lost, as CONTEXT allows.
- **Baseline and archive:** the baseline is reduced to the Gantt only, the reason is recorded beside the fence and on the reference page, and nothing changed under `docs/src/archive`.
- **Feeder links and cycles:** feedback cycles through outside nodes, and through two containers, do not invert the title. Probes `Q -> eng.A -> eng.B -> R -> eng.A` and `one.B -> two.C … two.D -> one.A` gave the title above its content in both.
- **Contact-sheet spec:** it fails closed on fence-count and figure-count mismatches. `innerHTML` is used only on a throwaway `about:blank` page with strict-mode Mermaid output.

_Reviewed: 2026-10-27T09:10:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
