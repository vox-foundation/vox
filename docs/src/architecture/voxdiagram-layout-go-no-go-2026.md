---
title: "VoxDiagram Layout Go/No-Go: dagre 0.1.1 and the Deterministic SVG Contract"
description: "Records the decision to adopt dagre 0.1.1 behind a layout seam for VoxDiagram, with the corpus evidence, determinism results, font policy, render caps and residual risks."
category: "Architecture SSOTs"
topic: "doc-pipeline"
domain: "documentation"
authorship: "llm-assisted"
authorship_model: "claude-sonnet-5-5"
status: "current"
---

# VoxDiagram Layout Go/No-Go: dagre 0.1.1 and the Deterministic SVG Contract

<!-- kb:summary -->
Decision record for the VoxDiagram layout engine. Dagre 0.1.1 is adopted, pinned exactly and confined to one adapter file behind the layout seam, with dugong 0.9.0 as the fallback. The ten largest frozen corpus graphs and all 30 importable corpus graphs render with zero DSL tweaks against a threshold of two. The record also fixes the font-measurement policy, render caps, the SVG theming contract and the honest limits: about 4% of generated compound graphs fail closed, and an edge can cross a container title.
<!-- /kb:summary -->

## Decision

**GO.** `crates/vox-diagram` lays graphs out with `dagre` 0.1.1.

- The dependency is an exact pin (`=0.1.1`) with default features on; the crate does not compile with `default-features = false`.
- Every dagre type stays inside `crates/vox-diagram/src/layout/dagre_impl.rs`, behind the crate-internal `layout::Engine` seam. A source-guard test (`layout_seam_guard_dagre_only_in_adapter`) fails if any other file imports dagre.
- The fallback is `dugong` 0.9.0 behind the same seam. It was not exercised, and adopting it would add a dependency, which needs the project owner's sign-off. The go/no-go threshold was never approached, so it stays a fallback.

## Criterion and method

The input is the frozen Mermaid corpus under `crates/vox-diagram/tests/fixtures/corpus/` (33 blocks from `docs/src`). Each block goes through the Phase 84 importer (`mermaid::import`), then `vox_diagram::check`, then `svg::render` with default options, so the evidence comes from the shipped code path, not a prototype.

- 30 blocks import as graphs, one imports as a sequence diagram (Phase 86) and two are refused (`gantt`, markdown strings), exactly as the importer fixtures record.
- The ten largest graphs are ranked by nodes plus edges, descending, with the fixture stem ascending as tiebreak. The tenth scores 20 and the eleventh 19. `corpus_top10_ranking` asserts the list and both scores.
- A diagram **needs a DSL tweak** only if it returns `Err` or shows a hard defect that editing its DSL alone would fix. An edge grazing a node is a known dagre behaviour and does not count. The go threshold is at most 2 of 10; more than 2 means stop and ask about `dugong`.

The ten rendered diagrams are pinned as byte-exact goldens in [the corpus golden directory](../../../crates/vox-diagram/tests/fixtures/svg/corpus/). The light and dark contact sheet is committed at `.planning/workstreams/voxdiagram/phases/85-graph-layout-deterministic-svg/85-CONTACT-SHEET.html` (a planning artifact, so cited by path rather than linked).

## Results

All 30 importable graphs render `Ok` and pass an independent quick-xml allowlist check (`corpus_graphs_all_render`). Each of the ten was reviewed as a headless-Chromium screenshot of its light and dark pane. Scale is `min(100, 76000 / width)` percent for a 760 px column.

| # | Fixture (corpus stem suffix) | Nodes / edges / containers | Dir | viewBox px | Scale | Result | DSL tweaks | Review notes |
|---|------------------------------|----------------------------|-----|-----------|-------|--------|-----------|--------------|
| 1 | `13-deep-research-waves-...-l110` | 18 / 20 / 1 | down | 1346.5 x 2070 | 56% | Ok | 0 | Labels inside shapes, container title inside its band, arrowheads visible in both panes. Two edge labels sit on the band's top border (soft). |
| 2 | `04-deep-research-full-surface-...-l46` | 17 / 16 / 0 | down | 534.5 x 2052 | 100% | Ok | 0 | Clean. A 31-character identifier in the last node is hard-broken mid-word by the 28-cell wrap (soft). |
| 3 | `21-unified-workstream-execution-plan-...-l45` | 14 / 17 / 0 | down | 1440.5 x 758 | 52% | Ok | 0 | Clean fan-out, no overlap. |
| 4 | `12-deep-research-waves-...-l76` | 13 / 16 / 0 | down | 1438 x 684 | 52% | Ok | 0 | Clean; the long supervisor-to-router edge and the loop-back edge are legible. |
| 5 | `26-vox-react-backend-interop-audit-...-l51` | 11 / 12 / 0 | right | 942.5 x 382 | 80% | Ok | 0 | Clean. |
| 6 | `03-axis-harness-river-design-...-l83` | 11 / 11 / 4 | right | 1024 x 999.5 | 74% | Ok | 0 | Four container titles inside their bands; cross-container edges pass through a sibling band, as the graph requires. |
| 7 | `01-ai-first-fixtures-research-...-l147` | 11 / 10 / 0 | down | 247.5 x 1194 | 100% | Ok | 0 | Clean. |
| 8 | `11-deep-research-waves-...-l45` | 10 / 11 / 0 | down | 559.5 x 1066 | 100% | Ok | 0 | Clean; loop-back edges and their labels are legible. |
| 9 | `20-tauri-convergence-migration-plan-...-l117` | 11 / 10 / 0 | right | 1312 x 262 | 57% | Ok | 0 | Clean. |
| 10 | `06-deep-research-full-surface-...-l238` | 12 / 8 / 2 | down | 1311.5 x 1162 | 57% | Ok | 0 | The edge from the outside `UserQuery` node into the first container runs through that container's title text. The title stays readable but is struck through (see Residual risks). |

**DSL tweaks: 0 of 10 (threshold 2).** Diagram 10 is the one visible defect. It is a routing limit of the renderer, not something a DSL edit should paper over, so it is recorded as a residual risk. Even counted as a tweak, the total is 1, which is still within the threshold. No diagram returned `Err`.

## Determinism

Output is deterministic by construction and by measurement.

- Dagre's `HashMap`s are lookup-only; every iteration walks insertion-ordered vectors, and its layout path calls no transcendental float function. A guard test (`determinism_guard_no_transcendental_floats`) keeps that true for the crate's own code.
- The adapter quantizes every coordinate to integer half-pixels at the seam. No float survives past it, which also protects against a future engine bump that changes a last bit.
- `twenty_fresh_renders` (four diagrams, 20 renders, one distinct string) and `generated_graphs_render_identically` cover in-process repetition across fresh dagre graphs. Nextest runs each test in its own process, which adds cross-process coverage.
- The goldens were blessed on macOS arm64.
- Linux comparison (non-gating; hosted CI on the milestone PR is the authoritative run): the `svg_corpus`, `svg_render` and `svg_determinism` binaries were run in `rust:1.98.1` against a standalone copy of the crate pinned to the locked dependency versions. aarch64 (Colima, native): **pass**, 21 tests including the ten corpus goldens. x86_64 (`--platform linux/amd64`, QEMU, `uname -m` printed `x86_64`): **pass**, the same 21 tests with the same goldens.
- The research spike also hashed the whole pipeline over 30 corpus graphs and 2,000 random compound graphs on macOS arm64, Linux aarch64 and Linux x86_64, with identical hashes. An assumption remains that QEMU's x86_64 float emulation matches real hardware.

## Font policy

Layout measures text itself, without a browser.

- Advance widths for printable ASCII come from Arimo Regular (metric-compatible with Arial and Helvetica, OFL), stored as a table of 95 integers per 1000 em. Browser widths for Arial-class stacks matched the table to 0.3% over 309 corpus label lines.
- Width is the table sum plus `unicode-width` cells for non-ASCII, with 8% slack and 14 px horizontal and 10 px vertical node padding. Lines wrap greedily at 28 cells; a longer word is hard-broken on a character boundary. This is why diagram 2 breaks a long identifier mid-word.
- The non-ASCII model is approximate: accented Latin and CJK are close, but arrows and some symbols are under-measured by up to 40%, and emoji by about 29%. The padding absorbs most of it, and wide labels can overflow their box.
- Verdana and DejaVu Sans are the wide stack: they run 12% wider than the table on corpus text (worst label +19.7% for DejaVu). With the slack and padding, the spike measured one overflow of 0.8 px across 144 labels under Verdana. **Phase 89 must test with Verdana or DejaVu as its wide stack.** Monospace fallbacks are not supported: they overflow badly (up to 30 px).
- Real-browser verification of fit is Phase 89's gate. Phase 85 only proves the contract in the renderer and in a screenshot review.

## Containers and fail-closed behaviour

Dagre cannot size or title containers: a container's box comes from its border nodes and ignores the title. The adapter therefore adds a title strut leaf per container, then tries three separation rungs in order, nodesep/ranksep/edgesep of 40/60/70, 50/90/120 and 60/140/200. The first rung whose result passes the geometry verifier wins.

The verifier is exact integer arithmetic with no epsilon. It rejects a node overlap, a node outside its container, a foreign or sibling container overlap, a container outside its parent and a non-finite coordinate. If every rung fails, `RenderError::Geometry` is returned and no SVG is drawn. A panic or a missing coordinate from the engine is `RenderError::Layout`.

Measured limits, stated plainly:

- All 30 corpus graphs pass on the first rung.
- Generated compound graphs do not. In the 85-04 property test, about 4% of generated graphs fail closed with `Geometry` (5 of 128 in the recorded run; the count varies with the seed and is deliberately not asserted). The corpus result is 30 of 30, but a generated graph is not guaranteed to render.
- The research fuzz measured 0 failures in about 10,000 corpus-density graphs and rates from 0.02% (up to 40 nodes) to 0.7% (up to 100 nodes) for larger random ones. The 4% property-test rate comes from denser, more adversarial generation with long titles.
- Soft defects, an edge through a non-endpoint node or a label touching a node, are deliberately not checked.

## Render caps

`MAX_RENDER_NODES = 100` (nodes plus containers) and `MAX_RENDER_EDGES = 200` apply to rendering and are lower than the 500 / 1,000 parse caps. Dagre has no work budget and is super-quadratic: the spike measured about 0.2 s for 100 nodes and 200 edges, 3 s for 300 and 600, and 16.6 s for 500 and 1,000. Larger diagrams get `RenderError::TooLarge`. The ten corpus diagrams lay out in 0.8 to 4.5 ms each.

## SVG and theming contract

- One writer produces the SVG, and every text and attribute value goes through a single escaper. Output uses only `svg`, `title`, `desc`, `defs`, `marker`, `path`, `rect`, `polygon`, `text`, `tspan` and `g`, a fixed class set (`vd-*`) and inline styles limited to `fill`, `stroke`, `stroke-width`, `font-family` and `font-size`. No `<style>`, script, `foreignObject`, comment or user-derived id or class.
- Ids are `<prefix>-title`, `-desc` and `-arrow`; the prefix matches `[a-z][a-z0-9-]*` and is at most 32 characters, so several diagrams can share a page.
- Every colour is `var(--vd-NAME, fallback)` from the nine entries in `THEME_VARS` (text, muted, node fill, node stroke, edge, container fill, container stroke, background, font). Inline style outranks stylesheet rules, so a site themes diagrams only through the variables.
- Candidate map for Phase 89, proven in the contact sheet from one markup in two panes: text to `--sl-color-white`, muted to `--sl-color-gray-2`, node fill and background to `--sl-color-black`, node stroke and edge to `--sl-color-gray-3`, container fill to `--sl-color-gray-6`, container stroke to `--sl-color-gray-5`, font to `--sl-font-system`.
- WCAG contrast from the Starlight 0.38.3 token values: dark text 17.70 (container title 15.10), muted 10.01, graphics at least 4.47; light text 17.70 (title 15.29), muted 11.74, graphics at least 6.07; the fallback palette gives text 15.80, muted 6.39 and graphics at least 6.00. All text is at least 4.5 and all graphics at least 3, in both themes.

## Residual risks

- **Edge through a container title.** An edge from outside a container to a node near its top crosses the title strip (diagram 10). Fixing it needs title-aware routing, a text halo or a different title placement; it is a Phase 89 or later decision.
- **Single maintainer, no work budget.** Dagre 0.1.1 has a single maintainer and no cap on work, so the render caps are the only protection against slow inputs. The exact pin and the seam keep a swap to `dugong` local to one file.
- **Fail-closed rate.** About 4% of generated compound graphs return `Geometry`; callers must treat `RenderError` as a normal outcome and show the source or a message.
- **Edges grazing nodes** and soft mid-word breaks remain, as in Mermaid's own dagre output.
- **Tall and wide diagrams scale down.** Five of the ten render below 60% at a 760 px column; legibility policy belongs to Phase 89.
- **Research assumptions that still stand:** the Arimo-derived width table is treated as facts rather than font software, and QEMU matches real x86_64 floats. Every other premise is exercised by a test in the crate.
