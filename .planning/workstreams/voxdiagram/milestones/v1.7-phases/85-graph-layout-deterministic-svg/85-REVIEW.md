---
phase: 85-graph-layout-deterministic-svg
reviewed: 2026-10-17T00:00:00Z
depth: standard
files_reviewed: 31
files_reviewed_list:
  - crates/vox-diagram/Cargo.toml
  - crates/vox-diagram/src/layout/dagre_impl.rs
  - crates/vox-diagram/src/layout/finish.rs
  - crates/vox-diagram/src/layout/input.rs
  - crates/vox-diagram/src/layout/mod.rs
  - crates/vox-diagram/src/layout/tests.rs
  - crates/vox-diagram/src/layout/text.rs
  - crates/vox-diagram/src/layout/text/tests.rs
  - crates/vox-diagram/src/layout/verify.rs
  - crates/vox-diagram/src/lib.rs
  - crates/vox-diagram/src/limits.rs
  - crates/vox-diagram/src/svg/mod.rs
  - crates/vox-diagram/src/svg/paths.rs
  - crates/vox-diagram/src/svg/shapes.rs
  - crates/vox-diagram/src/svg/writer.rs
  - crates/vox-diagram/tests/determinism_guard.rs
  - crates/vox-diagram/tests/support/mod.rs
  - crates/vox-diagram/tests/support/svgcheck.rs
  - crates/vox-diagram/tests/support/svgnum.rs
  - crates/vox-diagram/tests/support/theme.rs
  - crates/vox-diagram/tests/svg_corpus.rs
  - crates/vox-diagram/tests/svg_determinism.rs
  - crates/vox-diagram/tests/svg_errors.rs
  - crates/vox-diagram/tests/svg_geometry_props.rs
  - crates/vox-diagram/tests/svg_hostile.rs
  - crates/vox-diagram/tests/svg_render.rs
  - crates/vox-diagram/tests/svg_theme.rs
  - docs/src/architecture/voxdiagram-layout-go-no-go-2026.md
  - docs/src/architecture/where-things-live.md
findings:
  critical: 0
  warning: 5
  info: 7
  total: 12
status: issues_found
---

# Phase 85: Code Review Report

**Reviewed:** 2026-10-17
**Depth:** standard (with scratch-copy probes, see below)
**Files Reviewed:** 31 (29 listed; `text/tests.rs` and `tests/support/*` read only for the parts the findings touch)
**Status:** issues_found

## Summary

The injection surface is sound. Every attribute and text value goes through one `esc`; no attribute is label-derived; `id_prefix` is validated before use; the controls, C1, bidi, U+2028/9 and U+FFFE/F set is covered. A 4,000-graph random fuzz through the public `render` (all shapes, directions, nested containers, self-loops, multi-edges, empty and CJK labels) produced no panic, no hang and one clean `Geometry` rejection. Render caps are applied before layout (`input::build` first). Integer paths in `paths.rs` and `finish.rs` are safe for every value the real dagre engine can produce.

The problems are in the "fail-closed, never a drawn overlap" contract and in the evidence for it:

- The geometry verifier does not cover edge-label boxes against container titles, and an opaque label background is then painted over the title text (measured, WR-01).
- The seam accepts coordinates (up to 1e7 px) that overflow the i64 math in `clip_diamond`, which runs after verification (WR-02, reproduced).
- The decision doc's render-time figure is about 10x too low for structured inputs at the cap (WR-03).
- A contrast claim in the doc is false for the container frame (WR-04).
- The headline geometry property test can pass with zero successful renders (WR-05).

No finding is a BLOCKER: none is reachable from the public API with the shipped dagre engine except WR-01 and WR-03, which are quality and robustness defects rather than data loss or security issues.

Probe method: scratch copy of the crate under `scratchpad/probe85/` (workspace untouched), release and debug builds, `cargo test --offline`. Results are cited per finding.

## Warnings

### WR-01: Edge-label boxes are not verified against container titles, and the opaque label background covers title text

**File:** `crates/vox-diagram/src/layout/verify.rs:84-138` (no label rule), `crates/vox-diagram/src/svg/paths.rs:118-154` (label background drawn last)
**Issue:** `containment()` checks nodes, containers and title bands against each other but never an edge-label `Rect`. `write_edge_labels` paints each label with an opaque `--vd-bg` rect after the containers and nodes, so a label that lands in a title band erases part of the title. The contract in 85-CONTEXT.md is "never a drawn overlap". The go/no-go doc lists only "label touching a node" and "two edge labels on the band's top border" as soft defects and never says a label can cover title text.

Measured at the layout seam with the real `DagreEngine`: 600 random compound inputs (1 to 5 containers, up to 14 nodes, half the edges labelled, all four directions) all returned `Ok`. In 87 of them an edge-label box overlapped a title band, and in 45 it overlapped the actual title text block. Example, direction Down: label `Rect{x:3463,y:600,w:50,h:52}` (half-pixels) sits inside the title text block `Rect{x:3307,y:606,w:340,h:96}` of "A very long container title / that must wrap around / several lines for sure". The same harness found 0 label-over-node and 0 label-over-label overlaps, so dagre does reserve space for labels against nodes. The gap is specific to the title band, which dagre knows nothing about.

**Fix:** Add a rule to `containment` (or a sibling check) and escalate rungs on it like the other violations:
```rust
// Violation::LabelInTitleBand(edge, container)
for (ei, e) in placed.edges.iter().enumerate() {
    if let Some(l) = e.label {
        for (k, c) in placed.containers.iter().enumerate() {
            let band = Rect { h: title_band(c.title_lines.len()), ..c.rect };
            if overlap(l, band) { return Err(Violation::LabelInTitleBand(ei, k)); }
        }
    }
}
```
If failing on every rung is too costly (the rate is about 7% of labelled compound graphs), the alternative is drawing label backgrounds before the container titles. Either way, update the go/no-go residual risks to state the actual behaviour and rate.

### WR-02: `clip_diamond` overflows i64 on coordinates the seam accepts; it also runs after verification

**File:** `crates/vox-diagram/src/svg/paths.rs:80-89` and `crates/vox-diagram/src/layout/mod.rs:184-190`
**Issue:** `quantize` accepts `|v| < 1e7` px (up to 2e7 half-pixels) and says it "fails closed on a huge coordinate". `clip_diamond` computes `dx * w * h` in i64, which is about 1.6e22 at those bounds and overflows i64 (9.2e18). Two further problems follow:

- Debug and test builds panic with "attempt to multiply with overflow" at `paths.rs:88`. That code is outside `guarded()`, so the panic escapes `render()`.
- Release builds wrap silently. `clip` runs after `verify`, so the wrapped endpoint is never checked.

I reproduced both by putting a test engine behind `render_with`: one diamond node 5e6 px square, an edge point at 9e6 px. Debug panicked at `paths.rs:88`. Release returned `Ok` with the nonsense path `M2508408 2508408L2498273 2498273`.

With real dagre this is unreachable, since node sizes come from capped labels. The seam, however, is documented as untrusted-engine-safe ("tests substitute doubles that return garbage"), and the doc credits it with defending against "a future engine bump".

**Fix:** Tighten the bound to what the i64 arithmetic can take and make the clip total:
```rust
// layout/mod.rs
const MAX_COORD_PX: f64 = 1.0e5; // 2e5 half-px: dx*w*h <= ~8e15, safe in i64
if !v.is_finite() || v.abs() >= MAX_COORD_PX { return Err(LayoutError::NonFinite); }
```
Alternatively use `i128` in `clip_diamond`, or `checked_mul` with a fallback to returning `p` unchanged. Update the `huge` unit test (`1.0e8`) to also cover a value just under the new bound.

### WR-03: Render caps do not bound render time; the doc's 0.2 s figure is for sparse random graphs

**File:** `crates/vox-diagram/src/limits.rs:15-20`, `docs/src/architecture/voxdiagram-layout-go-no-go-2026.md` ("Render caps")
**Issue:** The doc says dagre is super-quadratic with no work budget, so "the render caps are the only protection", and gives about 0.2 s for 100 nodes and 200 edges. At exactly the caps, structured inputs are much slower. All numbers below are release builds on this machine; the debug column is an unoptimised dagre (opt-level 0), so it is an upper bound only. The workspace builds dependencies at opt-level 1.

| Input (100 nodes, 200 edges unless noted) | Release | Debug (opt-level 0 dagre) |
|---|---|---|
| random edges | 0.15 s | 1.3 s |
| chain plus 100 long-span edges | 1.3 s | not measured |
| chain plus 100 full-span edges, labelled | 2.4 s | 33 s |
| 50 containers (4-deep chains), 50 nodes | 3.5 s | 30 s |
| 40 containers, 60 nodes, labelled edges | 2.1 s | 19 s |

Long edges create one dummy node per crossed rank, which is the super-quadratic part, and `lay_out` repeats the whole solve on each of three rungs when verification fails, so the worst case is up to 3x these figures. A caller that renders on a request thread, such as Phases 87 to 89, has no timeout. Hosted nextest debug runs of such a graph could also hit the per-test timeout.

**Fix:** Correct the doc to quote a worst-case structured figure (about 3.5 s release at the cap) next to the random-graph figure. Either lower `MAX_RENDER_EDGES`/`MAX_RENDER_NODES`, or bound dummy-node growth, for example by rejecting when `edges * (estimated rank span)` exceeds a budget. At minimum, document the hazard on `svg::render` so callers run it off the request path.

### WR-04: The doc claims every graphic has at least 3:1 contrast; the container frame does not, and the test omits that pair

**File:** `crates/vox-diagram/tests/svg_theme.rs:104-125`, `docs/src/architecture/voxdiagram-layout-go-no-go-2026.md` ("SVG and theming contract")
**Issue:** The doc states "all graphics at least 3 in both themes". `assert_pairs` checks `--vd-edge` and `--vd-node-stroke` against `--vd-bg` and `--vd-container-fill`, but never `--vd-container-stroke` against its fill or against `--vd-bg`. The fallback pair is `#d0d7de` on `#f6f8fa`/`#fff`, which is about 1.3 to 1.4:1 by the standard WCAG formula. The container frame is the only visual cue that nodes are grouped, so it is an informative graphic (WCAG 1.4.11), not decoration. The doc's contrast figures are therefore a selective subset presented as complete, and the test cannot fail for this pair.

**Fix:** Either add `("--vd-container-stroke", "--vd-container-fill")` and `("--vd-container-stroke", "--vd-bg")` to `graphic` and darken the fallback and the Starlight mapping, or state in the doc that the container frame is deliberately a low-contrast decorative boundary and that grouping is conveyed by the title.

### WR-05: `geometry_holds` can pass with zero successful renders, and its oracle lacks the sibling, foreign and title-band rules

**File:** `crates/vox-diagram/tests/svg_geometry_props.rs:182-219`
**Issue:** Every `Err(RenderError::Geometry)` is counted and then ignored. The test never asserts a minimum number of `Ok` renders, so a verifier regression that rejects everything leaves it green; it only prints "N of 128 were Err". Separately, the SVG read-back oracle checks node/node overlap, node in container, container in parent and the viewBox. It does not check sibling-container overlap, foreign node vs container overlap, children vs title band, or label vs anything. The verifier's own rules for those cases (`SiblingOverlap`, `ForeignOverlap`, `TitleBand`) are therefore tested only by unit tests on hand-built `Placed` values, not independently against drawn output. WR-01 would have been caught by an independent label-vs-title read-back.

**Fix:**
```rust
assert!(errs.get() <= 128 / 4, "too many Geometry rejections: {}", errs.get()); // and count Ok > 0
```
Add read-back checks for sibling containers (same parent, disjoint), nodes outside a container not overlapping its frame, children below the title band (`first_baseline` plus lines), and edge-label boxes versus node and container-title boxes.

## Info

### IN-01: The view-box rule is tautological, and container titles and engine node sizes are not verified

**File:** `crates/vox-diagram/src/layout/mod.rs:273-314`, `crates/vox-diagram/src/layout/verify.rs:154-170`
**Issue:**
- `place` derives width and height from the extents of every node, container, label and edge point. `verify`'s `OutsideView` check therefore cannot fail on the `lay_out` path. It is exercised only by unit tests on hand-built `Placed`. A single stray edge point at 9.9e6 px produces an `Ok` render with a 20-million-px viewBox rather than a rejection.
- `fits()` checks node text against the engine-reported box, but there is no equivalent for container titles (`fit_title` falls back to one cell per line without a check), and there is no check that the engine returned the size it was given. With dagre these never trigger (0 of 600 title overflows in the probe), but the verifier is described as engine-independent.

**Fix:** Cap total canvas extent (for example 100k px) as a `Violation::TooBig`. Add a `TitleOverflow` rule using `measure` against `rect.w - 2 * TITLE_INSET`.

### IN-02: The default `id_prefix` collides when several diagrams share a page

**File:** `crates/vox-diagram/src/svg/mod.rs:26-33`
**Issue:** `SvgOpts::default()` uses the fixed prefix `vd`. Two default-rendered diagrams inline in one HTML document share `vd-title`, `vd-desc` and `vd-arrow`. `aria-labelledby` then resolves to the first diagram's title and description for both, so assistive tech announces the wrong diagram. The doc advertises that "several diagrams can share a page", which is only true if every caller sets a unique prefix.

**Fix:** Document on `SvgOpts` that `id_prefix` must be unique per page, or drop `Default` for the prefix so callers must choose.

### IN-03: Empty title and empty first label line give empty accessible text

**File:** `crates/vox-diagram/src/svg/mod.rs:147-152, 207-210`
**Issue:**
- `opts.title = Some("")` emits an empty `<title>`; it should fall back to "Diagram" like `None`.
- `short()` takes only `label.split('\n').next()`. A node label that starts with a newline yields an empty string, so `<desc>` reads " → ".

**Fix:** Use `.filter(|t| !t.trim().is_empty())` on the title. In `short`, take the first non-blank line.

### IN-04: `catch_unwind` containment is conditional on the build target

**File:** `crates/vox-diagram/src/layout/mod.rs:177-181`
**Issue:** The comment correctly notes the workspace keeps unwinding. `rust-toolchain.toml` also installs `wasm32-wasip1`, where Rust aborts on panic, and any consumer that sets `panic = "abort"` loses the guard. The panic hook also prints the dagre panic message to stderr on every contained panic, which is noise for an MCP or LSP host. Stack overflow and infinite loops are not containable either way.

**Fix:** Note the wasm and abort caveat on `guarded`. Optionally add a work budget or rank-span precheck (see WR-03) so the guard is not the only defence.

### IN-05: The determinism claims in the doc are slightly overstated

**File:** `docs/src/architecture/voxdiagram-layout-go-no-go-2026.md` ("Determinism", kb summary)
**Issue:** "Every iteration walks insertion-ordered vectors" is not literally true. `dagre::graph::Graph::remove_node` iterates `in_e.values()` and `out_e.values()` of a `HashMap` (graph/mod.rs:329-335). The effect looks order-independent because the ordered vectors are updated by key, and the measured 20-render and proptest checks support that. The doc should say "no iteration order reaches the output" rather than "all iteration is ordered". Also, the kb summary and "Containers" text credit the verifier with rejecting "a non-finite coordinate", but that rejection happens in `quantize` and `place`, before `verify`.

**Fix:** Reword the two statements.

### IN-06: Source guards read files with `unwrap_or_default`

**File:** `crates/vox-diagram/tests/determinism_guard.rs:37, 67, 92, 117`
**Issue:** An unreadable source file reads as an empty string and passes every needle. The `files.len() > 5` and `checked >= 8` floors limit the damage but do not cover one bad file.

**Fix:** Replace `unwrap_or_default()` with a panic on `Err`, matching the pattern used elsewhere in the suite.

### IN-07: `hard_break` can emit a leading empty chunk for a wide character at limit 1

**File:** `crates/vox-diagram/src/layout/text.rs:66-80`
**Issue:** With `limit = 1` and a 2-cell character first, `used + w > limit` is already true at `used == 0`, so a new chunk is pushed and the first chunk stays `""`. `wrap_at` then emits an empty title line, which wastes one title band row. This is only reachable through `fit_title` falling all the way to one cell per line, which needs a container narrower than one CJK glyph, so it is latent only.

**Fix:** Guard the push with `!chunks.last().is_some_and(String::is_empty)`, or only start a new chunk when `used > 0`.

---

_Reviewed: 2026-10-17_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
