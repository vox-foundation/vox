---
phase: 85-graph-layout-deterministic-svg
fixed_at: 2026-10-17T00:00:00Z
review_path: .planning/workstreams/voxdiagram/phases/85-graph-layout-deterministic-svg/85-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 85: Code Review Fix Report

**Fixed at:** 2026-10-17
**Source review:** .planning/workstreams/voxdiagram/phases/85-graph-layout-deterministic-svg/85-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 5 (WR-01 to WR-05; Info findings out of scope)
- Fixed: 5
- Skipped: 0

**Verification location:** the existing isolated worktree `/Users/brbrainerd/dev/vox-voxdiagram` (branch `milestone/v1.7-voxdiagram`). No second worktree was created: the caller scoped work to this directory and a nested worktree would have been rooted in the main checkout. Commits are on the branch directly. Gates: `INSTA_UPDATE=no cargo nextest run -p vox-diagram` (464 passed, 2 skipped), `cargo clippy -p vox-diagram --all-targets -- -D warnings`, `RUSTDOCFLAGS='-D warnings' cargo doc -p vox-diagram --no-deps`, `cargo fmt -p vox-diagram`, and `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/voxdiagram-layout-go-no-go-2026.md` all clean.

## Fixed Issues

### WR-01: Edge-label boxes are not verified against container titles

**Files modified:** `crates/vox-diagram/src/layout/verify.rs`, `crates/vox-diagram/src/layout/verify/tests.rs` (tests moved out of `verify.rs` to stay under 400 lines), `crates/vox-diagram/tests/svg_geometry_props.rs`, `crates/vox-diagram/tests/fixtures/svg/corpus/13-...-l110.svg`
**Commit:** e69861f4b (shared with WR-05)
**Status:** fixed: requires human verification (verifier logic)
**Applied fix:** New `labels()` rule in the verifier with `Violation::LabelInTitleBand(edge, container)`, `LabelOnNode(edge, node)` and `LabelOnLabel(edge, edge)`; any interior overlap escalates the separation rung and fails closed after the third. Test first: the SVG read-back oracle was extended (see WR-05) and, against the unfixed verifier, failed with "edge label 0 ... covers title band of a0". Unit tests for each new violation were written before the variants existed (compile-red), then pass.

Corpus result: all 30 graphs still render (`corpus_graphs_all_render`). One graph is affected, not failing: top-10 diagram 1 (`13-deep-research-waves-...-l110`) now settles on rung 2 instead of rung 1, because its first-rung layout had edge labels on the title band (the soft defect the go/no-go doc had recorded). Its golden changed (viewBox 1346.5 x 2070 to 1541 x 2550) and was re-blessed with the blessing test alone. The other 29 graphs pass on rung 1. The generated-graph fail-closed rate is now 4 to 8% (5 to 10 of 128 over four runs), not "about 4%"; the doc is updated.

### WR-02: `clip_diamond` overflows i64 on coordinates the seam accepts

**Files modified:** `crates/vox-diagram/src/layout/mod.rs`, `crates/vox-diagram/src/layout/tests.rs`, `crates/vox-diagram/src/layout/tests/coords.rs` (new, holds the regression tests)
**Commit:** cdeff1390
**Applied fix:** `MAX_COORD_PX = 1.0e5` replaces the 1e7 bound in `quantize`; `dx*w*h` stays near 2.4e16, well inside i64. Test first: an engine double returning a 5e6 px diamond with an edge point at 9e6 px reproduced the debug panic at `paths.rs:88` ("attempt to multiply with overflow"); it now returns `Err(Layout("non-finite coordinate"))`. A second test renders a 9e4 px diamond with an end point at 9.9e4 px (just under the bound) and asserts `Ok`. The `quantize` unit tests now cover 1e5, -1e5 and 99,999.

### WR-03: Render caps do not bound render time; doc figure too low

**Files modified:** `docs/src/architecture/voxdiagram-layout-go-no-go-2026.md`, `crates/vox-diagram/src/svg/mod.rs` (doc comment on `render`)
**Commit:** 116e6156e
**Status:** fixed (documentation only; no regression test, because a wall-clock assertion would be flaky)
**Applied fix:** Measured with a temporary probe test (deleted, not committed) calling the public `render` at exactly 100 nodes plus containers and 200 edges. Release, loaded machine (load average about 35): random edges 0.23 s; chain plus 100 long-span edges 1.05 to 1.06 s; chain plus 100 full-span labelled edges 3.4 to 4.1 s; 50 four-deep containers with 50 nodes 0.70 to 0.73 s; 40 containers with 60 nodes and labelled edges 1.4 to 1.5 s. The test (unoptimised) profile: 0.29, 1.31, 5.56, 0.84, 1.73 s. The doc now quotes about 4 s as the worst case at the caps, and `svg::render` documents that it is synchronous, has no timeout and must run off the request path.

**Caps NOT changed (user decision).** The 100/200 caps are met in the sense that nothing exceeds about 4 s release / 6 s test profile, but that is 20x the old figure. If a latency target is set, the levers are lower `MAX_RENDER_EDGES` or a rank-span (dummy-node) budget; I did not apply either.

### WR-04: Doc claims all graphics reach 3:1; the container frame does not

**Files modified:** `crates/vox-diagram/src/svg/mod.rs`, `crates/vox-diagram/tests/svg_theme.rs`, `crates/vox-diagram/tests/support/theme.rs`, `tests/fixtures/svg/cases/{20-container,21-nested}.svg`, `tests/fixtures/svg/corpus/{03,06,13}-*.svg`, `.planning/.../85-CONTACT-SHEET.html` (regenerated), go/no-go doc (in the WR-03 commit)
**Commit:** f2b6e06f4
**Applied fix:** Chose the palette fix, not the "decorative" justification: the frame is the only visual cue that nodes are grouped, so it is an informative graphic (WCAG 1.4.11). `contrast_holds` now asserts `--vd-container-stroke` against `--vd-bg` and `--vd-container-fill`; it failed first (dark 1.51 against the background). Fallback `#d0d7de` (1.45) becomes `#848d97` (3.37 on white, 3.16 on the container fill); the Starlight candidate map moves container stroke from gray-5 to gray-3 (dark 4.47 and 5.24, light 6.07 and 7.02). The five affected goldens were updated by replacing only that hex string and pass byte-exact, so no bless run was needed. The contact sheet was regenerated with its ignored one-shot generator. Visual effect: container frames are now as strong as node outlines; Phase 89 may want a thinner stroke or a dash if that reads too heavy.

### WR-05: `geometry_holds` can pass with zero successful renders; oracle lacks rules

**Files modified:** `crates/vox-diagram/tests/svg_geometry_props.rs`
**Commit:** e69861f4b (shared with WR-01)
**Applied fix:** A `Tally` counts rejected and drawn renders and the test asserts `drawn >= 96` of 128. Mutation check: forcing `verify` to always return an error now fails the test with "only 0 of 128 cases rendered (128 rejected)"; the original was restored. Observed drawn counts 118 to 123 (floor 96 is more than 9 standard deviations away at a 6% rate). The oracle now reads back, independently of the verifier: sibling containers disjoint, foreign nodes clear of a frame, nested containers and own nodes below the title band (band height computed from the drawn title lines), and edge-label boxes clear of nodes, title bands and other labels.

## Skipped Issues

None.

## Notes for the caller

- WR-01 and WR-05 share one commit because the oracle and the floor live in the same test function.
- Commit `f2b6e06f4` also holds the regenerated contact sheet, a planning artifact.
- REVIEW-FIX.md is deliberately not committed. `.planning/active-workstream` and `milestone.lock` were already modified or untracked before this run and were left alone.
- No Cargo.toml or Cargo.lock edits; no proptest-regressions files were created.

---

_Fixed: 2026-10-17_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
