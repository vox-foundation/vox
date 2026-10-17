---
phase: 85-graph-layout-deterministic-svg
verified: 2026-10-17T00:00:00Z
status: passed
score: 5/5 must-haves verified
covered_files:
  - ".planning/workstreams/voxdiagram/phases/85-graph-layout-deterministic-svg/85-01-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/85-graph-layout-deterministic-svg/85-01-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/85-graph-layout-deterministic-svg/85-02-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/85-graph-layout-deterministic-svg/85-02-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/85-graph-layout-deterministic-svg/85-03-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/85-graph-layout-deterministic-svg/85-03-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/85-graph-layout-deterministic-svg/85-04-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/85-graph-layout-deterministic-svg/85-04-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/85-graph-layout-deterministic-svg/85-05-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/85-graph-layout-deterministic-svg/85-05-SUMMARY.md"
  - "crates/vox-diagram/Cargo.toml"
  - "crates/vox-diagram/src/layout/dagre_impl.rs"
  - "crates/vox-diagram/src/layout/finish.rs"
  - "crates/vox-diagram/src/layout/input.rs"
  - "crates/vox-diagram/src/layout/mod.rs"
  - "crates/vox-diagram/src/layout/text.rs"
  - "crates/vox-diagram/src/layout/verify.rs"
  - "crates/vox-diagram/src/limits.rs"
  - "crates/vox-diagram/src/svg/mod.rs"
  - "crates/vox-diagram/src/svg/paths.rs"
  - "crates/vox-diagram/src/svg/shapes.rs"
  - "crates/vox-diagram/src/svg/writer.rs"
  - "crates/vox-diagram/tests/determinism_guard.rs"
  - "crates/vox-diagram/tests/svg_corpus.rs"
  - "crates/vox-diagram/tests/svg_determinism.rs"
  - "crates/vox-diagram/tests/svg_geometry_props.rs"
  - "crates/vox-diagram/tests/svg_hostile.rs"
  - "crates/vox-diagram/tests/svg_theme.rs"
  - "docs/src/architecture/voxdiagram-layout-go-no-go-2026.md"
covered_digest: "v3:sha256:1a47fb483ae098ee8f115dab1544ca633c1201049470090f90f57a5c5d44ff65"
behavior_unverified: 0
overrides_applied: 0
re_verification: false
---

# Phase 85: Graph Layout & Deterministic SVG Verification Report

**Phase Goal:** A `graph` diagram renders to a deterministic, injection-safe, themeable inline SVG that holds up on the largest real diagrams, with layout risk retired by an explicit go/no-go.
**Verified:** 2026-10-17
**Status:** passed (with hosted-CI-only items listed below)
**Re-verification:** No, initial verification (after the code-review fix round, commits e69861f4b..841a841ba)

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Recorded go/no-go on the 10 largest corpus diagrams; dagre adopted only if at most 2 need tweaks; engine call in `catch_unwind` with node and edge caps | VERIFIED | `docs/src/architecture/voxdiagram-layout-go-no-go-2026.md` records GO, 0 of 10 tweaks (threshold 2), per-diagram table, dugong fallback. `layout/mod.rs:10,160` wraps the engine in `catch_unwind(AssertUnwindSafe(..))` mapped to `LayoutError::EnginePanic`; `layout/input.rs:95` enforces `MAX_RENDER_NODES=100` / `MAX_RENDER_EDGES=200` (`limits.rs:17,20`) before layout. `dagre_impl.rs` is the only file naming `dagre` (guard test `layout_seam_guard_dagre_only_in_adapter`, passing). Top-10 ranking frozen by `corpus_top10_ranking` (passing). Doc passes `vox-doc-pipeline --lint-only` (run here, exit 0). |
| 2 | Containers, edge labels, cycles, self-loops render with property-tested invariants (no node overlap, children in container, labels in boxes); contact sheet of the 10 | VERIFIED | `layout/verify.rs` is a hard-invariant verifier with three separation rungs and `RenderError::Geometry` on total failure; `svg_geometry_props::geometry_holds` reads the drawn SVG back independently (128 generated graphs, floor of 96 rendered so it cannot pass vacuously). `corpus_graphs_all_render`: 30 of 30 importable graphs `Ok`. `85-CONTACT-SHEET.html` exists (10 diagrams x light/dark panes, 20 `vd-diagram` blocks). Full suite: 464 passed, 2 skipped. |
| 3 | Byte-identical across runs and across macOS and Linux (goldens green on both; no HashMap order, transcendental floats, versions, timestamps) | VERIFIED locally; Linux/hosted run not re-executed here | `svg_determinism` (20 fresh renders, 64 generated graphs) and `determinism_guard` (no `.powf/.sin/...`, no clock/version read, in `layout/` and `svg/`) pass. A grep of `src/layout` and `src/svg` finds no `HashMap`, transcendental call, `Instant`, `SystemTime` or version read. Coordinates are integer half-pixels (`writer::num`). 85-05 records the goldens passing on Linux aarch64 (native Docker) and x86_64 (QEMU). I did not re-run Linux; hosted CI is the authoritative cross-platform run. |
| 4 | Hostile labels escaped by the single writer; no `<style>`/`foreignObject`/user-derived attributes; per-diagram id prefixes; IR-derived `<title>`/`<desc>`; text as `<text>`; removing escape fails the guard (mutation-proven) | VERIFIED | `svg/writer.rs` has the single `esc` (`& < >`, quotes in attributes, controls/C1/bidi/separators to U+FFFD); all attrs/text go through `Writer`. quick-xml allowlist checker (`tests/support/svgcheck.rs`) vets hostile fragments as node label, edge label and title plus `graph_cases()`. **Independent mutation run by me:** deleting the `'<'` arm of `esc` made `svg_hostile` fail 3 of 7 (`hostile_property`, `hostile_fragments`, `hostile_graph_cases`); file restored byte-identical (`git diff --quiet` clean). 85-04 SUMMARY records the other arms plus a whole-function bypass. No `<style` / `foreignObject` in `src/` or the corpus goldens. |
| 5 | Same single inline SVG legible in light and dark Starlight token sets via `--vd-*` mapped to `--sl-*`, no duplicated per-theme markup | VERIFIED at the SVG level | Every paint is `var(--vd-NAME,FALLBACK)` from the nine-entry `THEME_VARS` (`svg/mod.rs:75`); `svg_theme` tests `paints_are_theme_vars`, `all_theme_vars_used`, `contrast_holds` (text >= 4.5:1, edge/node/container strokes >= 3:1 under Starlight 0.38.3 dark, light and fallback palettes), `one_markup_for_both_themes` all pass. The contact sheet defines `.vd-light` / `.vd-dark` panes differing only in `--sl-*` values over identical markup. Real-site browser mounting is Phase 89 by roadmap design. |

**Score:** 5/5 truths verified (0 present-but-behavior-unverified).

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `crates/vox-diagram/src/svg/{mod,writer,paths,shapes}.rs` | VERIFIED | Substantive, wired; `svg::render` public API |
| `crates/vox-diagram/src/layout/{mod,dagre_impl,input,finish,text,verify}.rs` | VERIFIED | Seam, wrapping, verifier, container strut all present and tested |
| `crates/vox-diagram/tests/svg_corpus.rs` + 10 goldens in `tests/fixtures/svg/corpus/` | VERIFIED | Directory holds exactly 10 .svg; `corpus_top10_goldens` passes |
| `85-CONTACT-SHEET.html` | VERIFIED | Present, 20 diagram blocks, light/dark token panes |
| `docs/src/architecture/voxdiagram-layout-go-no-go-2026.md` | VERIFIED | GO recorded with evidence and residual risks; lint clean |
| `where-things-live.md` row; API page | VERIFIED | Row names `vox-diagram` incl. layout and SVG (line 113); Cargo description updated |

### Key Link Verification

| From | To | Status | Details |
|------|----|--------|---------|
| `svg::render` | `layout::lay_out` | WIRED | `build` caps -> rungs -> `verify` -> finish -> writer |
| `lay_out` | `DagreEngine` via `Engine` trait inside `catch_unwind` | WIRED | `mod.rs:160` |
| `Writer` | `esc` for every attr and text | WIRED | `writer.rs` `tag`/`leaf` |
| `THEME_VARS` | every paint | WIRED | `paint()` helper; `paints_are_theme_vars` |

### Data-Flow Trace

Not applicable in the UI sense; the renderer is a pure function of the IR. Corpus goldens come from `mermaid::import` -> `check` -> `svg::render` (the shipped path), so the data is real.

### Behavioral Spot-Checks (run by me, in `/Users/brbrainerd/dev/vox-voxdiagram`)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full crate suite | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 --no-fail-fast` | 464 run, 464 passed, 2 skipped | PASS |
| Clippy | `cargo clippy -p vox-diagram --all-targets -- -D warnings` | clean | PASS |
| Rustdoc | `RUSTDOCFLAGS='-D warnings' cargo doc -p vox-diagram --no-deps` | clean | PASS |
| Escape mutation | delete `'<'` arm in `esc`, run `svg_hostile` | 3 failed, restored clean | PASS (guard works) |
| Doc lint | `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/voxdiagram-layout-go-no-go-2026.md` | no hard errors | PASS |

### Probe Execution

No phase-declared probes; Step 7c not applicable.

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|-------------|--------|----------|
| REND-01 (dagre behind one-file adapter; containers, edge labels, cycles, self-loops; top-10 go/no-go) | 85-01..05 | SATISFIED | Truths 1, 2 |
| REND-02 (byte-identical across runs and macOS/Linux) | 85-01, 02, 04, 05 | SATISFIED locally; Linux claim rests on 85-05 Docker record, hosted CI pending | Truth 3 |
| REND-03 (single escaper, id prefixes, no style/foreignObject, title/desc, mutation-proven) | 85-01, 04, 05 (completed in 85-04) | SATISFIED | Truth 4 |
| REND-04 (one SVG themes via CSS variables mapped to Starlight tokens) | 85-01, 04, 05 | SATISFIED at SVG level | Truth 5 |

All four IDs appear in PLAN frontmatter and in REQUIREMENTS.md (checked, traceability table "Complete"). No orphaned Phase 85 requirements (REND-05 is Phase 86, REND-06 Phase 84).

### Anti-Patterns Found

No `TBD`/`FIXME`/`XXX` debt markers surfaced in the phase's source; no stubs. Known, documented limits (not blockers):

| Item | Severity | Impact |
|------|----------|--------|
| Fail-closed rate: about 4-8% of generated compound graphs return `RenderError::Geometry`; corpus is 30/30 | Info | Documented in the go/no-go doc; callers must treat `RenderError` as normal |
| Edge can cross a container title (top-10 diagram 10, "residual risk") | Warning | Cosmetic (title struck through); recorded, deferred to Phase 89 or later |
| Worst-case render about 4 s at the 100/200 caps, synchronous, no timeout | Warning | Documented in doc and `render` rustdoc; callers must run off the request path |
| Container frame stroke now heavier (WR-04) | Info | Phase 89 may thin or dash it |

### Human Verification Required

None required for the phase gate. Items that stay open by design:

- **Hosted CI only:** the macOS plus Linux byte-identity run on the milestone PR (I re-ran nothing on Linux; 85-05's Docker aarch64 and QEMU x86_64 results are recorded but not independently re-verified here, and QEMU is not real x86_64 hardware); the PR-level `crate-edges`, `vox-arch-check`, `hakari`, fmt and full ssot-drift gates.
- **Phase 89:** real-browser mounting under the actual Starlight site, wide-font (Verdana/DejaVu) fit, and legibility policy for diagrams scaled below 60%.

### Gaps Summary

No gaps. Notes for bookkeeping only: the ROADMAP phase list line "- [ ] Phase 85" is still unchecked although the detail section shows 5/5 plans executed and REQUIREMENTS marks REND-01..04 complete; tick it when closing the phase. `85-REVIEW-FIX.md` and this report are uncommitted by instruction. The code-review fix round (WR-01..05) is reflected in the code, goldens and doc; the 85-05 SUMMARY's older figures (460 tests, "29 of 30 on rung 1") predate it, and the doc and current suite (464) are the current truth.

---

_Verified: 2026-10-17_
_Verifier: Claude (gsd-verifier)_
