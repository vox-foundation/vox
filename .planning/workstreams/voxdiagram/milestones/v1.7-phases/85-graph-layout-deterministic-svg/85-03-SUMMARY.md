---
phase: 85-graph-layout-deterministic-svg
plan: 03
subsystem: diagram-render
tags: [rust, dagre, svg, layout, containers, verify, fail-closed, hostile-ir, golden]
status: complete

requires:
  - phase: 85-02
    provides: "Engine seam, three-rung separation ladder, geometry verifier, edge drawing"
provides:
  - "layout::input: container resolution from parent links (derived depth, bounded walk, label cap, node cap including containers); IR depth and local_id never read"
  - "dagre adapter compound graph: container c{i}, invisible title strut t{i} sized (strut_w, 1 px), parents set for nodes, nested containers and struts"
  - "layout::finish: engine rect padded 8 px left/right/bottom, title band 10 px + 16 px per line on top, title re-wrapped to the final inner width"
  - "layout::verify: OutsideContainer, OutsideParent, SiblingOverlap, ForeignOverlap, TitleBand, plus containers inside the view box"
  - "svg: vd-containers group drawn first in (derived depth, IR order); vd-container rect and vd-container-label title"
  - "svg_errors test binary; goldens 20-container and 21-nested"
affects: [85-04, 85-05]

actuals:
  tokens: 13000   # chars/4 over the diff excluding fixtures (52060 chars)
  tasks: 2
  commits: 2
plan_head_before: d3b7b5624e5afab2ad81cc356381716541f1f4c5
plan_head_after: eb633f9eedb3f4bd6ce29e68cf8bbaeccb389e0c

tech-stack:
  added: []
  patterns:
    - "container frame = quantized engine rect, padded and titled after quantize, then verified inside the same ladder"
    - "ancestor/depth walks bounded by a constant (MAX_DEPTH) or by the container count, so hand-built cyclic parents terminate"

key-files:
  created:
    - crates/vox-diagram/src/layout/finish.rs
    - crates/vox-diagram/tests/svg_errors.rs
    - crates/vox-diagram/tests/fixtures/svg/cases/ (20-container, 21-nested, each .vdg and .svg)
  modified:
    - crates/vox-diagram/src/layout/{mod,input,dagre_impl,verify,text,tests}.rs
    - crates/vox-diagram/src/svg/{mod,shapes,paths}.rs
    - crates/vox-diagram/tests/svg_render.rs

key-decisions:
  - "Container titles use a 16 px line pitch (the band is 10 px + 16 px per line), so `shapes::label` now takes a `(class, fill)` pair and a line step; node and edge labels keep 18 px and their goldens are byte-unchanged"
  - "`text::wrap_at(label, cells)` added next to `wrap`; the title re-wrap tries 27 down to 1 cells per line until the widest line fits the final inner width (container width minus 20 px)"
  - "The vd-containers group is emitted only when the diagram has containers, so every earlier golden is byte-unchanged"
  - "A parent cycle and over-deep nesting share one InvalidIr message: the depth walk stops after MAX_DEPTH hops, which a cycle always exceeds"
  - "Labels over MAX_LABEL_CHARS (nodes, containers, edges) are InvalidIr before layout; edge-label length is checked after endpoint resolution"

requirements-completed: []

duration: 75min
completed: 2026-10-17
---

# Phase 85 Plan 03: Containers Summary

**Containers (nested, sibling, empty, long-titled, cross-edged in both directions) now render inside titled frames, built only from parent links the renderer resolved itself, and every containment rule fails closed into `RenderError::Geometry`; hostile hand-built IR is refused with `InvalidIr` or rendered, never a panic.**

## Accomplishments

- Task 1: `build` resolves containers through a `BTreeMap` (unknown or duplicate id is `InvalidIr`), derives depth from parent links, caps labels, and the 85-01 container refusal is gone. The dagre adapter adds `c{i}` containers and a `t{i}` strut leaf per container. `layout::finish::frame` pads the quantized engine rect and adds the title band; `place` includes container frames in the viewBox extent and the margin shift. `svg::shapes::write_container` draws the frame (`rx=6`, container fill/stroke variables) and the title centred in the band. Goldens `20-container` and `21-nested` blessed and inspected numerically: every node is inside its container rect, every nested container inside its parent, each child container starts below the parent's title band (Middle layer band ends at y=396, Innermost container starts at y=440), sibling containers (Outer/Sibling/Hollow/Lengthy) are disjoint, the 60-char title wrapped to two lines inside its frame, and the empty container is 297x174 px.
- Task 2: five containment checks on exact integer geometry (touching allowed, ancestors followed through resolved parent indices with a bounded walk) plus containers in the view-box check. `ContainerAlwaysBad` double gives `Geometry(OutsideContainer(0))` after exactly three engine calls. `svg_errors` covers unknown node container, unknown parent, duplicate container id, two-container cycle, self-parent, five nested levels (4 is Ok), 201-char node/container/edge labels (200 is Ok), 60 nodes + 41 containers as `TooLarge { nodes: 101, .. }`, and an edge to a container id as `InvalidIr` ("edge 0").
- `hostile_graph_cases_render`: every `graph_cases()` entry renders Ok. No finding to report.

## Task Commits

1. **Task 1: Containers end to end** - `73273e59c` (feat)
2. **Task 2: Verifier checks, container doubles, hostile-IR validation** - `eb633f9ee` (feat)

## TDD record

- Task 1 RED: with the new `layout::input` tests, the `an_empty_container_is_at_least_its_title_strut_wide` layout test, the two new `svg_render` tests and the `.vdg` inputs added but no implementation, `cargo check -p vox-diagram --all-targets` failed with 12 errors on the lib test target (E0422/E0425/E0560/E0609: `LContainer`, `containers` field on `LayoutInput`/`Placed`/`RawLayout`). GREEN after implementation: 81 tests pass (`binary(svg_render) | test(/^layout::/) | test(/^svg::/)`). The golden blessing test ran alone with `UPDATE_GOLDEN=1`, no spurious failures this time; no existing golden changed.
- Task 2 RED: with the verifier tests, `ContainerAlwaysBad` test and `svg_errors.rs` added, the check failed with 5 errors (E0599: `OutsideContainer`, `OutsideParent`, `SiblingOverlap`, `ForeignOverlap`, `TitleBand` not found on `Violation`). GREEN: 76 tests pass.
- Mutation (guard proof, presence-checked): `grep -c 'containment(input, placed)?;'` 1 before, 0 mutated, 1 restored (restored file `diff`-identical to the saved copy). With the call removed, `cargo nextest run -p vox-diagram -E 'binary(svg_errors) | test(/^layout::/) | binary(svg_render)'` failed 6 tests (`a_leaf_outside_its_container_is_rejected`, `a_child_inside_the_title_band_is_rejected`, `a_container_outside_its_parent_is_rejected`, `sibling_containers_sharing_area_are_rejected`, `a_leaf_overlapping_a_container_it_does_not_belong_to_is_rejected`, `layout::tests::a_leaf_outside_its_container_on_every_rung_is_a_geometry_error`), 70 passed. Restored; green again.
- Final: `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(svg_render) | binary(svg_errors) | test(/^layout::/) | test(/^svg::/) | binary(determinism_guard)'` -> 98 passed; `cargo clippy -p vox-diagram --all-targets -- -D warnings` clean; `cargo fmt -p vox-diagram --check` clean.

## Deviations from Plan

None - plan executed as written. (`layout/verify.rs` ends at 397 non-blank lines, close to the 400 limit; the next plan that touches it should split the container checks into their own file first.)

## Issues Encountered

- Not measured: a visual render of the two goldens (the shell allowlist blocks `qlmanage`); containment was checked from the SVG coordinates instead.
- The 21-nested layout in direction `right` is tall and sparse (1145x1247 px) because dagre's compound LR layout stretches containers; valid under every invariant, recorded for the 85-05 contact sheet and decision doc.

## Known Stubs

None.

## Not done (belongs to later plans)

- Allowlist checker, theme/contrast tests, property suites: 85-04. Corpus spike and goldens, closing gates (`vox ci crate-api-sync`, `where-things-live.md`): 85-05.
- REND-01 is NOT ticked: the corpus-level proof (SC1/SC2 across live diagrams) lands in 85-05.

## Threat Flags

None. T-85-08 (bounded parent walk, cycle/unknown id/depth/dup id give `InvalidIr`, containers count toward the node cap), T-85-09 (five containment checks each with a failing unit test plus the `ContainerAlwaysBad` double and the skip-containment mutation above) and T-85-10 (label cap for node, container and edge labels, tested at 200/201) are mitigated. Container titles are written only through `shapes::label` -> `Writer::leaf` (the single escaper).

## Self-Check: PASSED

Verified present: `crates/vox-diagram/src/layout/finish.rs`, `crates/vox-diagram/tests/svg_errors.rs`, `20-container` and `21-nested` `.vdg` and `.svg`. Commits `73273e59c` and `eb633f9ee` are ancestors of HEAD. `grep -c set_parent dagre_impl.rs` = 3; `grep -c 'class="vd-container"' 21-nested.svg` = 6 = containers in `21-nested.vdg`; `grep -rl 'dagre::' src` prints only `layout/dagre_impl.rs`; `git diff --quiet 5247a56ea -- crates/vox-diagram/tests/support` exits 0; no Cargo.toml / Cargo.lock change; every touched file is under 400 non-blank lines (largest `layout/verify.rs`, 397).
