---
phase: 85-graph-layout-deterministic-svg
plan: 04
subsystem: diagram-render
tags: [rust, svg, quick-xml, hostile, determinism, geometry, theme, contrast, mutation-proof]
status: complete

requires:
  - phase: 85-03
    provides: "Containers, geometry verifier, deterministic SVG writer with the single escaper"
provides:
  - "tests/support/svgcheck.rs: quick-xml allowlist checker that also reads geometry and decoded text back out (independent of the renderer's verifier)"
  - "tests/support/svgnum.rs: half-pixel number, path and polygon parsing split out of svgcheck (size limit)"
  - "tests/support/theme.rs: Starlight 0.38.3 DARK/LIGHT tokens, STARLIGHT_MAP, contrast, pane_css(Theme) for the 85-05 contact sheet"
  - "svg_hostile, svg_determinism, svg_geometry_props, svg_theme test binaries; three new source guards in determinism_guard"
affects: [85-05]

actuals:
  tokens: 13000   # chars/4 over the diff (52677 chars)
  tasks: 3
  commits: 3
plan_head_before: c857898c0e353165d8838af645fc9cb731046f59
plan_head_after: c23073ebc5e067fed8c31156c346532db74a6d40

tech-stack:
  added: []
  patterns:
    - "SVG oracle = quick-xml allowlist checker over the written bytes; decoded text compared with a `sanitize` restated in the test, never read from src"
    - "mutation proofs driven by a scratch script: save copy, presence count before/mutated/restored, run named binaries, restore, `git diff --quiet`"

key-files:
  created:
    - crates/vox-diagram/tests/support/svgcheck.rs
    - crates/vox-diagram/tests/support/svgnum.rs
    - crates/vox-diagram/tests/support/theme.rs
    - crates/vox-diagram/tests/svg_hostile.rs
    - crates/vox-diagram/tests/svg_determinism.rs
    - crates/vox-diagram/tests/svg_geometry_props.rs
    - crates/vox-diagram/tests/svg_theme.rs
  modified:
    - crates/vox-diagram/tests/support/mod.rs
    - crates/vox-diagram/tests/determinism_guard.rs

key-decisions:
  - "No `src/` change: every new test passed against the 85-01..03 code, so the writer needed no fix"
  - "`Svg` gained `root_children`, `ids` and `paints` (every style `prop:value`), and `Label` gained `class` and `owner`, so tests can tie a label to the shape it sits in"
  - "`theme::pane_css` takes a `Theme` enum and scopes to `.pane-dark` / `.pane-light`; `theme::contrast` takes `#hex` or `hsl(..)` strings"
  - "REND-03 is ticked (all its clauses are delivered and mutation-proven); REND-02 (macOS and Linux bytes), REND-04 (browser view of both themes) and REND-01 stay open for 85-05"

requirements-completed: [REND-03]

duration: ~75min (not clocked; estimated)
completed: 2026-10-17
---

# Phase 85 Plan 04: SVG Safety, Determinism and Theme Proofs Summary

**A quick-xml allowlist checker, independent of the writer, now vets every hostile label, edge label, title and `graph_cases()` IR; every arm of the single escaper is shown by mutation to matter; byte determinism and SC2 geometry are checked on the drawn SVG; and the candidate Starlight map is proven legible in dark, light and standalone.**

## Accomplishments

- Task 1 (REND-03): `svgcheck::check(svg, prefix)` rejects any element outside {svg,title,desc,defs,marker,path,rect,polygon,text,tspan,g}, any attribute or class outside the fixed sets, comments/CDATA/PI/DOCTYPE, duplicate or unprefixed ids, unresolved `url(#..)` and `aria-labelledby` tokens, entities other than amp/lt/gt/quot/#39 (raw attribute text and `Event::GeneralRef`), raw `>` and control chars in text (quick-xml accepts raw `]]>`), non-whitespace text outside `title`/`desc`/`tspan`, `url(` in a style and style properties outside {fill, stroke, stroke-width, font-family, font-size}. `svg_hostile` (7 tests): checker self-tests (20 rejection cases plus the 00-minimal golden and a second-root document), every `FRAGMENTS` entry (and `x`+f+`y`) as node label, edge label and `SvgOpts.title` with decoded text equal to the independent `sanitize`, all `graph_cases()` under prefixes `vd` and `h-1`, a 128-case property (1-8 picks of fragments or `any::<char>()`, 200 chars, node + labelled self-loop + title), `no_user_text_in_attributes` (marker `QzQ7` through node, container, both edges and title), and `structure_contract` (title then desc first, ids exactly `{prefix}-title/-desc/-arrow`).
- Task 2 (REND-02, SC2): `twenty_fresh_renders` (00-minimal, 10-edges, 21-nested and corpus block 13 imported through `mermaid::import`: 20 renders, one distinct string, all Ok), `generated_graphs_render_identically` (64 cases), `geometry_holds` (128 cases through `TestRunner`, geometry read back from the SVG; printed `5 of 128 cases were Err(Geometry)`; that count varies by seed and is not asserted), and the guards `determinism_guard_no_transcendental_floats`, `determinism_guard_no_clock_or_version`, `layout_seam_guard_dagre_only_in_adapter` (4 tests now in the binary).
- Task 3 (REND-04): `paints_are_theme_vars` (every fill/stroke/font-family in all 10 case goldens is `none` or a `THEME_VARS` pair; no colour literal outside a `var()` fallback in any attribute), `all_theme_vars_used` (nine names), `one_markup_for_both_themes`, `contrast_matches_known_ratios`, `contrast_holds`, the map-covers-variables check and `pane_css` check. Measured ratios (all above threshold): dark text 17.70 / 15.10, muted 10.01, graphics min 4.47 (edge or node-stroke on container-fill); light text 17.70 / 15.29, muted 11.74, graphics min 6.07; fallback text 15.80 / 14.84, muted 6.39, graphics min 6.00. No `THEME_VARS` or map change was needed.

## Task Commits

1. **Task 1: Allowlist checker, hostile suite, escape mutation proof** - `39bca9b0d` (test)
2. **Task 2: Determinism, geometry properties, source guards** - `bdf367ca8` (test)
3. **Task 3: Theme variables and contrast** - `c23073ebc` (test)

## TDD record

- Task 1 RED: `svg_hostile.rs` written first with `support::svgcheck` not declared; `cargo check -p vox-diagram --test svg_hostile` failed with E0432 (unresolved import `support::svgcheck`). GREEN needed two test-side fixes found by running: the two-roots self-test used the raw template (`@@` left in) and the mismatched-tag needle (`ill-formed`, quick-xml's own message). Then 7/7 pass.
- Task 2: the tests target existing code, so there is no behavioural RED; the first runs failed for test-side reasons (see Deviations) and the guards were proven by the mutations below.
- Task 3: tests written before `pub mod theme;` was declared; the compile-failure output was not captured (the grep for the error line printed nothing), so RED is not evidenced for this task. The tests exercise existing code and all passed on first compile after the module was declared.

## Mutation proofs (writer.rs `esc`; all restored, `git diff --quiet -- src/svg/writer.rs` clean after each)

Run: `cargo nextest run -p vox-diagram --lib --test svg_hostile -E 'binary(svg_hostile) | test(/^svg::writer::/)'` (14 tests). "Presence" counts the exact line (or, for the bypass, the inserted marker line) before / mutated / restored.

| Mutation | Presence | Failing tests (render-level guard first) | Message excerpt | Restored |
|---|---|---|---|---|
| delete the `&` arm | 1 / 0 / 1 | svg_hostile `hostile_fragments`, `hostile_property`, `hostile_graph_cases`; `svg::writer::tests::markup_characters` | `node label of "&amp;"` left != right; `entity &;` | yes |
| delete the `<` arm | 1 / 0 / 1 | svg_hostile `hostile_fragments`, `hostile_property`, `hostile_graph_cases`; writer `markup_characters`, `writer_keeps_attribute_order_and_escapes` | `element <script&gt;alert(1)</script&gt;</title> not allowed` | yes |
| delete the `>` arm | 1 / 0 / 1 | svg_hostile `hostile_fragments`, `hostile_property`, `hostile_graph_cases`; writer `markup_characters` | `raw > in text "script>alert(1)"` | yes |
| control/bidi/separator arm to `push(c)` | 1 / 0 / 1 | svg_hostile `hostile_fragments`, `hostile_property`, `hostile_graph_cases`; writer `controls_and_bidi_become_replacement` | `control char U+0000 in text "\0"`; `node label of "\u{2028}"` | yes |
| whole-function bypass (`return s.to_owned()` first; presence of the inserted marker 0 / 1 / 0) | 0 / 1 / 0 | svg_hostile `hostile_fragments`, `hostile_property`, `hostile_graph_cases`; writer `controls_and_bidi_become_replacement`, `markup_characters`, `quotes_only_in_attributes`, `writer_keeps_attribute_order_and_escapes` | 7 failures | yes |
| delete the `"` attribute arm | 1 / 0 / 1 | writer `quotes_only_in_attributes`, `writer_keeps_attribute_order_and_escapes` | left != right | yes |
| delete the `'` attribute arm | 1 / 0 / 1 | writer `quotes_only_in_attributes` | left != right | yes |

Every text arm and the bypass are caught by `svg_hostile`; the two attribute-only arms are caught by `svg::writer::tests` only (no label text reaches an attribute, which `no_user_text_in_attributes` asserts directly, so the render-level suite cannot see an attribute-quote arm).

Guard mutations (`cargo nextest run -p vox-diagram --test determinism_guard -E 'binary(determinism_guard)'`, `git diff --quiet -- crates/vox-diagram/src` clean afterwards):

| Mutation | Presence of inserted line | Failing test | Message |
|---|---|---|---|
| `#[cfg(test)] fn` with `2.0_f64.powf(2.0)` before `num` in `svg/writer.rs` | 0 / 1 / 0 | `determinism_guard_no_transcendental_floats` | `svg/writer.rs calls .powf(` |
| comment `// mutant: see dagre::Graph` in `layout/mod.rs` | 0 / 1 / 0 | `layout_seam_guard_dagre_only_in_adapter` | `left == right` failed (importers list gained `layout/mod.rs`) |
| comment `// mutant: Instant::now` in `svg/mod.rs` (extra, not assigned) | 0 / 1 / 0 | `determinism_guard_no_clock_or_version` | `svg/mod.rs reads Instant` |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - test oracle] Label centring tolerance of one half-pixel**
- **Found during:** Task 2 (`geometry_holds`, first run)
- **Issue:** a node 155 half-pixels wide (77.5 px) has no representable centre; the writer's `x + w/2` truncates, so the label anchor sits 0.25 px left of the exact centre (`x=376` in a box `x=299 w=155`). Exact equality, as the plan worded it, can never hold for odd sizes.
- **Fix:** horizontal and vertical centring are asserted within one half-pixel (0.25 px); a real offset (a pixel or more) still fails. No `src` change: the quantisation is inherent to the half-pixel grid.
- **Files modified:** crates/vox-diagram/tests/svg_geometry_props.rs
- **Commit:** bdf367ca8

**2. [Rule 1 - test oracle] "n * 18 px + 20 px not above the box height" applies to node labels only**
- **Found during:** Task 2
- **Issue:** an edge-label background is 26 px high for one line (18 px of text plus 4 px each side), so the node padding of 20 px does not apply to it.
- **Fix:** the pad is 20 px for `vd-label` and 0 for `vd-edge-label` (text still has to fit).
- **Commit:** bdf367ca8

**3. [Rule 1 - test needle] `use dagre` matched prose**
- **Found during:** Task 2
- **Issue:** `src/limits.rs` says "because dagre has no work budget"; "because dagre" contains the plan's needle `use dagre`, so the guard found two importers.
- **Fix:** `use dagre` / `pub use dagre` count only at the start of a trimmed line; `dagre::` anywhere still counts. The mutation above still trips it.
- **Commit:** bdf367ca8

**4. [Rule 3 - size limit] svgcheck split**
- `svgcheck.rs` reached 455 non-blank lines; number, path and polygon parsing plus `Shape` moved to `support/svgnum.rs` (svgcheck now 383 non-blank, svgnum 76), re-exporting `Shape` so `support::svgcheck::Shape` still works. Commit 39bca9b0d.

**5. [Rule 1 - clippy] `manual_is_multiple_of` in svgnum.rs** found by the final `cargo clippy -p vox-diagram --all-targets -- -D warnings`; fixed in c23073ebc (clean afterwards).

Exploratory `svg_hostile.proptest-regressions` produced by the mutation runs was deleted, not committed.

## Issues Encountered

- Generated S1 graphs fail closed (`Err(Geometry)`) in about 4% of cases (5 of 128 in the recorded run, varying by seed). The contract allows it and the test prints the count; 85-05 should say so in the decision doc rather than claim every S1 graph renders.
- No visual check of the pane CSS (that is the 85-05 contact sheet). `pane_css` is only string-tested here.
- The bash allowlist rewrote some `sed`/`head` calls on test paths; file reads were done with the Read tool instead.

## Known Stubs

None.

## Not done (belongs to later plans)

- Corpus top-10 goldens, the contact sheet, Linux golden comparison, go/no-go decision doc, API page, `where-things-live`, closing gates: 85-05.
- REND-01, REND-02 (macOS and Linux bytes) and REND-04 (browser view) are NOT ticked.

## Threat Flags

None. T-85-11 (checker on every fragment, graph case and a 128-case property, plus per-arm mutation proof), T-85-12 (`no_user_text_in_attributes`, attribute and style-property allowlists), T-85-13 (unique prefixed ids, every reference resolved, two prefixes tested) and T-85-14 (in-process, generated-graph and source-guard determinism, all guards mutation-checked) are mitigated as planned.

## Self-Check: PASSED

Verified present: `tests/support/{svgcheck,svgnum,theme}.rs`, `tests/{svg_hostile,svg_determinism,svg_geometry_props,svg_theme}.rs`. Commits `39bca9b0d`, `bdf367ca8`, `c23073ebc` are ancestors of HEAD. `INSTA_UPDATE=no cargo nextest run -p vox-diagram -E 'binary(svg_hostile) | binary(svg_determinism) | binary(svg_geometry_props) | binary(svg_theme) | binary(determinism_guard) | test(/^svg::/)'` -> 42 passed; `cargo clippy -p vox-diagram --all-targets -- -D warnings` clean; `cargo fmt -p vox-diagram --check` clean; `git diff --quiet c857898c0 -- crates/vox-diagram/src` exits 0; no Cargo.toml / Cargo.lock change; `grep -c` acceptance counts: `pub mod svgcheck;` 1, `GeneralRef` >= 1, `with_cases(128)` 1 (hostile and geometry), `with_cases(64)` 1, three new guard fns 3, `pub mod theme;` 1, `STARLIGHT_MAP` 5, gray-6 dark token 2; largest new file 383 non-blank lines.
