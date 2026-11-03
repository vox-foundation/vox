# Phase 85: Graph Layout & Deterministic SVG - Research

**Researched:** 2026-10-16
**Domain:** pure-Rust layered graph layout (`dagre` 0.1.1) behind a one-file adapter, label-aware text sizing, deterministic injection-safe themeable SVG writer, inside `crates/vox-diagram` (L2)
**Confidence:** HIGH on the go/no-go, determinism and dependency mechanics (all executed this session); MEDIUM on font metrics outside Chromium/macOS and on the long tail of dagre compound-graph failures

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Layout engine & spike
- DEPENDENCY APPROVED by user in chat (2026-10-16): add `dagre = "=0.1.1"` (Apache-2.0, pure Rust) as a `vox-diagram` runtime dependency (Cargo.toml + Cargo.lock change; run `cargo hakari generate`; cargo-deny licence check on PR). Fallback `dugong 0.9.0` behind the same adapter if the spike fails. This is a third-party crate, not a workspace crate edge.
- One-file adapter `layout/dagre_impl.rs`; the rest of the crate talks to a small layout trait/struct.
- Spike: the 10 largest frozen-corpus diagrams (imported via the Phase 84 importer) rendered to committed SVG goldens plus an HTML contact sheet in the phase directory; go/no-go recorded in a decision doc (adopt dagre only if ≤2 of 10 need DSL tweaks, else dugong).
- Edge routing: dagre polyline points → SVG path with fixed-radius rounded corners using arithmetic only (no sin/cos/powf); arrowheads as generated markers.
- `catch_unwind` around the dagre call + caps reused from `limits.rs`; a caught panic becomes `RenderError`, never a crash.

#### Text sizing & font policy
- Static advance-width table (ASCII from a reference sans font) + `unicode-width` (wide/CJK = 2 em) with ~8% slack; SVG text `font-family: var(--vd-font, system-ui, sans-serif)`. Real-browser verification is Phase 89.
- Labels: respect explicit `\n`; soft-wrap at word boundaries past ~28 chars (fixed constant); break mid-word only when a single word exceeds the width.
- Coordinates quantized to 0.5 px with fixed decimal formatting (no exponent, no `-0`) → byte-stable goldens on macOS and Linux.

#### SVG contract & theming
- No `<style>`, no `foreignObject`, no user-derived attributes. Fixed generated class names (`vd-node`, `vd-edge`, `vd-container`, `vd-label`, …) plus inline `style` declarations using CSS variables with literal fallbacks (`fill:var(--vd-node-fill,#fff)`), so the SVG renders standalone; the site maps `--vd-*` → `--sl-*` in Phase 89. One SVG serves light and dark.
- Caller-supplied `SvgOpts.id_prefix` (validated `[a-z][a-z0-9-]*`), default `vd`; all ids (markers, title, desc) prefixed; no hashing.
- Accessibility: `role="img"`, `aria-labelledby` → `<title>` (diagram title or "Diagram") + `<desc>` IR summary ("N nodes, M edges: a → b …", truncated); text as `<text>`.
- API: `vox_diagram::svg::render(&Ir, &SvgOpts) -> Result<String, RenderError>`.
- Single escaping writer for all text/attribute output; escape guard mutation-proven.

#### Carried forward
- No HashMap/HashSet in `src/`; no transcendental floats; no versions/timestamps in output; tests first; files < 400 non-blank lines; crate-api-sync --write after pub API changes; no workspace crate edges.
- UI-SPEC generation skipped: this phase's visual contract is the theming decisions above; browser verification lands in Phase 89.

### Claude's Discretion
- Rank/node separation constants, marker geometry, desc truncation length, module layout, proptest generators, contact-sheet format.

### Deferred Ideas (OUT OF SCOPE)
- Edges to containers (anchor lowering); multiple layout engines.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REND-01 | `graph` renders to SVG via `dagre` behind a one-file adapter, with containers, edge labels, cycles, self-loops; 10 largest corpus diagrams pass a go/no-go spike (fallback `dugong`) | Spike ran: 10/10 clean, 0 DSL tweaks (§Spike Results). Adapter shape, the title-strut + separation-ladder + fail-closed `verify` that make containers safe, caps, `catch_unwind` seam |
| REND-02 | SVG byte-identical across runs and across macOS and Linux (no HashMap order, no transcendental floats, no versions/timestamps) | Executed: 20 separate processes identical; macOS arm64 = Linux arm64 = Linux x86_64 (QEMU) over 30 corpus + 2000 random SVGs (§Determinism). Half-pixel integer quantization design |
| REND-03 | SVG safe and accessible: one escaping writer, generated id prefixes, no `<style>`/`foreignObject`/user-derived attributes, `<title>`/`<desc>` from IR, escape guard proven by mutation | Writer design, escape table, quick-xml allowlist checker prototype and its measured blind spot (raw `]]>`), per-arm mutation recipe |
| REND-04 | One inline SVG themes in light and dark via CSS variables mapped to Starlight tokens | `var(--vd-*, fallback)` in `style` verified in Chromium in both themes; candidate `--vd-*`→`--sl-*` map with WCAG contrast computed from Starlight 0.38.3 token values (§Theming) |
</phase_requirements>

## Summary

**Go for `dagre` 0.1.1.** I built a throwaway prototype (copy of the crate + `dagre`, scratchpad only, nothing in the repo changed) that runs the real Phase 84 importer over the frozen corpus, lays out through `dagre`, verifies geometry, renders SVG, and screenshots it in headless Chromium. The 10 largest importable graphs all lay out with zero hard-invariant violations and zero quality defects, needing **0 DSL tweaks** (threshold was ≤2). Across all 30 importable corpus graphs the only blemish is one edge grazing a node (a known dagre property, not a DSL fix). `dugong` was not needed and was not exercised. Layout takes 0.2-5 ms per corpus diagram.

dagre alone is not enough, and three facts shape the plan. (1) dagre cannot size or title containers: a container's width/height come from border nodes in its *last rank*, it ignores titles, and in random compound stress graphs it can place a leaf outside its container or overlap a sibling (~0.1% of dense 30-node random graphs even before titles are considered, and 5-20% of random graphs once container titles are grown post hoc). A **title-strut leaf per container + a 3-rung deterministic separation ladder + a fail-closed geometry `verify`** took that to 0 failures in 10,000 corpus-density graphs and ~0.02% at 40 nodes (those return `RenderError`, never a bad SVG). (2) dagre never panics on bad input: NaN/inf sizes propagate silently, and a compound graph can return with `x` missing on a node, so the guard is "validate finite and present", with `catch_unwind` as insurance. (3) dagre has no work budget and is super-quadratic: 500 nodes/1000 edges (the parse caps) takes ~17-20 s, so **render caps must be lower than parse caps** (recommend 100 nodes / 200 edges).

Determinism is better than feared and is now measured, not argued: dagre's internal `HashMap`s are lookup-only (iteration uses insertion-order vectors), and its layout path contains no transcendental float (the only `sin`/`cos` are inside a `#[cfg(test)]` module). Output was byte-identical across 20 processes and across macOS arm64, Linux arm64 (Docker/Colima) and Linux x86_64 (QEMU), for 30 corpus SVGs and 2,000 random compound graphs. Text metrics: Arial/Helvetica-class widths match Chromium to 0.3%; 8% slack plus 14 px horizontal padding keeps labels inside boxes even under Verdana (worst 0.8 px overflow); it does not survive a monospace fallback, and the model under-measures emoji by 29%.

**Primary recommendation:** add `dagre = "=0.1.1"` with **default features on** (the crate does not compile with `default-features = false`), keep every `dagre` type inside `layout/dagre_impl.rs`, give the adapter a title strut per container, a deterministic 3-rung separation ladder and a fail-closed geometry verifier, cap rendering at 100 nodes / 200 edges, quantize to integer half-pixels at the seam, and write SVG through one `String` writer whose escape arms are each mutation-proven against a `quick-xml` allowlist checker plus an explicit "no raw `<`/`>` in text" assertion.

## Project Constraints (from CLAUDE.md)

- Test-first: every new `pub fn` in `crates/*/src/**` (file ≥30 non-blank lines) needs a `#[test]` in the same file (detector `skeleton/untested-pub-api`, pre-commit `tdd-guard`). Existing crate pattern: `#[cfg(test)] mod tests;` with a sibling `tests.rs`.
- No `HashMap`/`HashSet` anywhere under `crates/vox-diagram/src` (guard test `determinism_guard_no_hash_collections`; `BTreeMap` is allowed outside `ir.rs`) [VERIFIED: crates/vox-diagram/tests/determinism_guard.rs:32-41].
- Files < 400 non-blank lines; `vox-diagram` is `layer = 2, max_loc = 12_000` (warn) [VERIFIED: docs/src/architecture/layers.toml:188]; `src/` is 7,590 `wc -l` lines today [VERIFIED: wc -l over `crates/vox-diagram/src`], so Phase 85 + 86 fit but not with much slack.
- No workspace crate edges (third-party `dagre`/`unicode-width` are not edges). Never edit `crates/vox-diagram`-external baselines to admit anything.
- Crypto policy: do not import `blake3`/`sha2` here; ids are prefix + index, no hashing.
- Hosted CI is the gate; locally run only `cargo check -p vox-diagram` and `cargo nextest run -p vox-diagram`; background builds; never workspace clippy (memory: never block on cargo, GSD lean verification; mutation proofs stay).
- After pub API changes run `vox ci crate-api-sync --write`; add a `where-things-live.md` row for any new concept (Rule 12 parity).
- Authored docs under `docs/src/` need frontmatter + `<!-- kb:summary -->`; project research goes to `docs/src/architecture/`, not IDE memory (so the go/no-go decision doc belongs there, see §Open Questions).
- Do not use `.py`/`.sh`/`.js` glue; helpers are Rust tests or `.vox`. (My throwaway Node scripts live only in the scratchpad.)
- `docs/src/archive/` must not be read or touched.

## Architectural Responsibility Map

Single-crate phase; "tier" here is the module that owns the capability (so the planner does not split ownership):

| Capability | Primary Owner | Secondary | Rationale |
|------------|---------------|-----------|-----------|
| IR validation (unknown refs, parent cycles, caps) | `svg::render` entry / `layout::validate_ir` | `limits.rs` | `Ir` is public and hand-buildable (`tests/support/hostile.rs::build` sets `depth: 1` for every container), so the renderer must not trust `depth`, `local_id` or ref integrity |
| Text measurement + wrapping | `layout/text.rs` | `unicode-width` | Needed before layout (node sizes) and after (title wrap), one calibration table |
| Layout (ranks, order, coordinates) | `layout/dagre_impl.rs` (only importer of `dagre`) | `layout/mod.rs` trait | Swap seam to `dugong`; everything else sees integer half-pixel structs |
| Container title room, separation ladder, geometry verification | `layout/mod.rs` + `layout/verify.rs` | - | Engine-independent: a `dugong` swap needs the same guarantees |
| Quantization (f64 → i32 half-px) | seam in `layout/mod.rs` | - | After this point no float exists, so no `-0`, no epsilon, exact property tests |
| SVG serialization, escaping, ids, theming vars | `svg/` (`writer.rs`, `shapes.rs`) | - | Single escape point; fixed attribute order |
| Contact sheet, goldens | test support (`#[ignore]`/env-gated) | phase dir | Evidence artifacts, not shipped API |
| Mapping `--vd-*` → `--sl-*` in site CSS | Phase 89 (docs-astro) | candidate map + contrast test here | CONTEXT locks this to Phase 89 |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `dagre` | `=0.1.1`, **default features** | compound layered layout | User-approved; only maintained pure-Rust compound+labels+variable-size engine; 0 DSL tweaks on the 10 largest corpus diagrams [VERIFIED: spike] |
| `unicode-width` | `0.2` (lock has 0.2.2) | classify wide / zero-width chars | Already in `Cargo.lock`; adds no crate [VERIFIED: lock diff in scratch copy] |
| `serde` | workspace | already a dep | unchanged |

### Supporting (dev-only)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `quick-xml` | workspace `0.41` | well-formedness + element/attribute allowlist checker in tests | REND-03 guard; already a workspace dep and already feature-unified in `workspace-hack`, so `hakari generate` is a no-op with it added [VERIFIED: scratch `cargo hakari generate` → "no changes detected"] |
| `proptest` | workspace (existing) | geometry/hostile properties | reuse `tests/support` S1 generator |
| golden helper `assert_golden` | in-repo | byte-exact `.svg` goldens, `UPDATE_GOLDEN=1` | reuse; do not use `insta` for SVG (its `.snap` header makes the files non-SVG and the contact sheet needs real `.svg`) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `dagre` | `dugong` 0.9.0 | Only if dagre regresses; has `WorkControl` budget (would fix the no-work-budget weakness). Not exercised this session |
| `quick-xml` in tests | hand-rolled tokenizer | No new dep either way; quick-xml is a real parser, but see its `]]>` blind spot below |
| string `SvgWriter` | `svg` crate | Never: attributes stored in `HashMap` [CITED: research/STACK.md §4] |

**Installation (edits, not run in the repo):**
```toml
# Cargo.toml [workspace.dependencies] (after strsim, line ~392)
dagre              = "=0.1.1"   # NOTE: default features are REQUIRED, see Pitfall 1
unicode-width      = "0.2"

# crates/vox-diagram/Cargo.toml
[dependencies]
dagre = { workspace = true }
unicode-width = { workspace = true }
[dev-dependencies]
quick-xml = { workspace = true }
```
Resulting `Cargo.lock` diff (reproduced in a scratch copy): one new `[[package]] dagre 0.1.1` (deps `log`, `serde`, `serde_json`, all already locked) plus three added dependency lines under `vox-diagram`. `crates/workspace-hack/Cargo.toml` is unchanged: `cargo hakari generate` → `info: no changes detected` [VERIFIED: scratch]. `cargo deny --workspace check licenses bans sources` → `bans ok, licenses ok, sources ok`; `Apache-2.0` is the first entry of the allowlist [VERIFIED: deny.toml:35-36].

**Version verification:** `dagre` 0.1.1 resolved from crates.io and built on `rust-toolchain.toml` 1.98.1 [VERIFIED: spike + `rust:1.98.1` image]; `build = false` in its manifest (no build script) [VERIFIED: dagre-0.1.1/Cargo.toml].

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `dagre` | crates.io | first published 2026-04-13 (~6 mo) | 387/wk | github.com/kookyleo/dagre-rs | SUS (`low-downloads`) | **Approved by user in chat 2026-10-16** (CONTEXT). Planner: cite that approval in the first task rather than add a new ask; if strict protocol is preferred, add a no-op `checkpoint:human-verify` that only confirms the recorded approval |
| `unicode-width` | crates.io | 11 yr | 14.8M/wk | github.com/unicode-rs/unicode-width | OK | Approved (already in lock) |
| `quick-xml` (dev) | crates.io | 10 yr | 10.8M/wk | github.com/tafia/quick-xml | OK | Approved (already a workspace dep) |
| `dugong` (fallback, not installed) | crates.io | 8 mo | 9.6k/wk | github.com/Latias94/merman | OK | Not added |

Results from `gsd-tools query package-legitimacy check --ecosystem crates` run this session. `dagre` mitigations verified here: no build script and no `postinstall` analogue, deps are `log`/`serde`/`serde_json` only (all already locked), pure Rust, licence passes `cargo-deny`, and I read the parts that decide behaviour (layout entry point and pipeline order, `types.rs`, the graph store, `position/{mod,bk}`, border segments, `intersect`). Residual risk is single-maintainer drift (STATE.md blocker), contained by the adapter seam and the `=` pin.

**Packages removed due to [SLOP]:** none. **Flagged [SUS]:** `dagre` (user-approved).

## Architecture Patterns

### System Architecture Diagram

```
 Ir (public, maybe hand-built)               SvgOpts { id_prefix, title }
        |                                            |
        v                                            v
 svg::render --> validate_ir ------ fail -----> RenderError::InvalidIr / BadIdPrefix / TooLarge
        |  (refs resolve, no parent cycles, caps: 100 nodes / 200 edges)
        v
 layout::text  (wrap <=28 cells, advance table x1.08, node sizes per shape)
        |
        v
 layout::solve(LayoutInput) -- ladder: rung1 (nodesep 40, ranksep 60, edgesep 70)
        |                         |           rung2 (50, 90, 120), rung3 (60, 140, 200)
        |                         v
        |             Engine::solve  (trait)  <--- test doubles: panic / NaN / missing x / overlap
        |                         |
        |                 dagre_impl.rs  [ONLY file importing dagre]
        |                   Graph{directed,multigraph,compound} + title strut leaf per container
        |                   catch_unwind(layout)  ->  read x/y/points  ->  finite + present check
        v
 quantize f64 -> i32 half-pixels   (after here: integers only)
        |
        v
 layout::verify (hard: node overlap, containment, finite/in-viewBox)  -- fail on all rungs --> RenderError::Geometry
        |
        v
 svg::writer  (one escape fn, one num fn, fixed attr order, ids = prefix + role)
        |
        v
 String  (LF, no timestamp/version, `var(--vd-*, fallback)` inline styles, <title>/<desc>)
```

### Recommended Project Structure
```
crates/vox-diagram/src/
├── layout/
│   ├── mod.rs        # LayoutInput/Output (i32 half-px), Engine trait, solve() + ladder, guarded() ~250
│   ├── dagre_impl.rs # the only `use dagre::` file: IR->Graph, strut, read-back, diamond clip ~220
│   ├── text.rs       # ADV table, measure, wrap, node_size per Shape ~200
│   └── verify.rs     # hard-invariant verifier (also used by proptests) ~150
├── svg/
│   ├── mod.rs        # pub fn render, SvgOpts, RenderError, desc/title ~220
│   ├── writer.rs     # esc(), num(), tiny Writer ~120
│   ├── shapes.rs     # node/container/edge-label drawing ~220
│   └── paths.rs      # rounded polyline ~80
└── limits.rs         # + MAX_RENDER_NODES = 100, MAX_RENDER_EDGES = 200
```
Layout total is roughly 1.5k lines against the 12k `max_loc` budget.

### Pattern 1: Engine seam with test doubles
**What:** `trait Engine { fn solve(&self, i: &LayoutInput) -> Result<RawLayout, LayoutError>; }`, implemented by `DagreEngine`; `render_with(&dyn Engine, ..)` is `pub(crate)`.
**When to use:** always; it is what makes `catch_unwind`, missing-`x`, NaN and overlapping-output paths unit-testable (dagre itself never panics on anything I could construct, so the guard cannot be tested through dagre).
**Example:** see Code Examples (`guarded`).

### Pattern 2: Title strut + separation ladder + verifier (container safety)
**What:** for each container add an invisible leaf child sized `(min(title_w + 20, 260), 1)`; after layout expand the container rect by `cont_pad = 8` on sides/bottom and by a title band `10 + 16 * title_lines` on top; wrap the title to the final width; run `verify`; on a hard violation retry with the next rung `[(nodesep, ranksep, edgesep)] = [(40,60,70), (50,90,120), (60,140,200)]`; if rung 3 fails return `RenderError::Geometry`.
**Why:** measured (10,000-seed fuzz tables in §Spike Results). Container width in dagre = distance between the *last-rank* border nodes, and `edgesep` is the gap between adjacent container borders, so widening separation is what stops sibling/foreign overlap.
**Cost:** one extra node per container in the dagre graph; position of the strut is ignored.

### Pattern 3: Quantize once, integers after
Convert every dagre `f64` to `i32` half-pixels at the seam (`(v * 2.0).round()` after a finite and `|v| < 1e7` check). `Placed` stores integers, `verify` and the writer do exact integer comparisons, and `num()` prints without ever formatting a float.

### Anti-Patterns to Avoid
- **Trusting dagre's container rect:** it ignores titles and can be wrong in dense cross-container graphs. Derive the final rect from dagre's, pad it, verify it.
- **`default-features = false` for dagre:** does not compile (Pitfall 1).
- **Asserting layout invariance under declaration reorder:** dagre uses insertion order for tie-breaks. Declaration order is intentionally significant; test determinism, not permutation invariance.
- **Making "edge passes through a node" a hard invariant:** 1/30 corpus graphs and ~50% of dense random graphs have one; dagre does not route around nodes between adjacent ranks. Track it as a soft metric.
- **Using the IR `depth` field or `local_id` in the renderer:** hostile/hand-built IR sets `depth: 1` everywhere; derive nesting from `parent` links with a cycle guard.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Layered layout, crossing minimization, network simplex, border segments | custom Sugiyama | `dagre` behind the seam | milestone-sized; defactor policy forbids forking 100+ lines |
| XML well-formedness in tests | regex/tokenizer | `quick-xml` (dev-dep) | real parser; add the explicit raw-`<`/`>` text assertion on top (it accepts raw `]]>`) |
| Width of CJK/zero-width chars | own Unicode tables | `unicode-width` | already locked |
| Float → decimal text | `format!("{}", f64)` | integer half-pixel `num()` | no exponent, no `-0`, no shortest-repr dependence |
| Golden compare | new snapshot framework | `assert_golden` in `tests/support/roundtrip.rs` | `UPDATE_GOLDEN=1`, byte-exact, already used for the corpus |
| Hostile label corpus | new fragment list | `tests/support/hostile.rs` `FRAGMENTS` + `graph_cases()` (hand-built IR with raw `\u{1}`, `\u{7f}`, `<script>`, `]]>`-adjacent text) | reuse; also exercises the "IR is not DSL-clean" path |
| Diagram generator | new proptest model | `tests/support/mod.rs` `model_graph()` (S1: depth ≤3, ≤30 items, arbitrary Unicode labels incl. `\n`/`\t`, edges as indices incl. self-loops) | already in the density band where the ladder gives 0 failures |

**Key insight:** the hard part is not the engine, it is the three things the engine does not do: size containers, fail loudly, and bound its own cost. Those are exactly the adapter's job.

## Spike Results (go/no-go evidence)

**Method** [VERIFIED: executed]: scratch copy of `crates/vox-diagram` (src only) + `dagre =0.1.1` + `unicode-width 0.2` + `quick-xml 0.41`; for every `corpus/*.mmd` run `mermaid::import(ImportOpts::default())` → `vox_diagram::check` → layout → geometry check → SVG → Chromium 1248 screenshots (light and dark). 33 blocks: 30 import as graphs, 1 sequence (Phase 86), gantt and markdown-string refused exactly as `unsupported.json` says. Final config: `FONT 14px`, line 18 px, node padding 14x10, wrap 28 cells, slack 8%, rung-1 separations above, strut + ladder.

**Top 10 by nodes+edges** (ties broken by fixture name; cutoff is clean: #10 has 20, #11 has 19):

| # | fixture | nodes/edges/containers | dir | size px | scale at 760 px column | layout ms | hard | quality |
|---|---------|------------------------|-----|---------|------------------------|-----------|------|---------|
| 0 | 13-deep-research-waves…-l110 | 18/20/1 | down | 1353x2062 | 0.56 | 4.5 | 0 | 0 |
| 1 | 04-deep-research-full-surface…-l46 | 17/16/0 | down | 534x2050 | 1.00 | 1.3 | 0 | 0 |
| 2 | 21-unified-workstream-execution-plan-l45 | 14/17/0 | down | 1439x758 | 0.53 | 1.7 | 0 | 0 |
| 3 | 12-deep-research-waves…-l76 | 13/16/0 | down | 1436x682 | 0.53 | 1.4 | 0 | 0 |
| 4 | 26-vox-react-backend-interop-audit-l51 | 11/12/0 | right | 941x382 | 0.81 | 1.0 | 0 | 0 |
| 5 | 03-axis-harness-river-design-l83 | 11/11/4 | right | 1079x1054 | 0.70 | 3.7 | 0 | 0 |
| 6 | 01-ai-first-fixtures-research-l147 | 11/10/0 | down | 247x1194 | 1.00 | 0.8 | 0 | 0 |
| 7 | 11-deep-research-waves…-l45 | 10/11/0 | down | 561x1060 | 1.00 | 1.5 | 0 | 0 |
| 8 | 20-tauri-convergence-migration-plan-l117 | 11/10/0 | right | 1310x262 | 0.58 | 1.0 | 0 | 0 |
| 9 | 06-deep-research-full-surface…-l238 | 12/8/2 | down | 1359x1162 | 0.56 | 2.4 | 0 | 0 |

**DSL tweaks needed: 0 of 10** (threshold ≤2) → **adopt `dagre`**. Over all 30 graphs: 0 hard violations; one soft defect (`18-scientia…-l50`: edge `eras.E4 -> vox.L3` grazes a sibling node; also the five `eras` nodes are permuted by crossing minimization, which is the same behaviour Mermaid's dagre shows). I eyeballed `03-axis-harness` (4 containers, LR; light and dark), `13-…-l110` (labelled decision diamonds + container), `06-…-l238` (cross-container edge, floating node) and `21-…` (fan-out) as screenshots: clean routing, no label/edge collisions, container titles inside boxes (the first three views were taken before the final separation constants, `21-…` after; the machine checks above ran on the final config for all 30).

**Container handling experiments** (fuzz: random compound graphs, 10,000 seeds, ≤20 nodes, edges ≤ nodes, up to 5 containers depth 3, labels from a 12-word soup so titles are long; "hard violation" = node overlap / node outside container / foreign overlap / sibling overlap / container outside parent / non-finite):

| Adapter variant | graphs with a hard violation |
|-----------------|------------------------------|
| dagre rect, title grown/wrapped post hoc, separations (40,60,40) | 2,020 (20%) |
| + title strut, (40,60,40) | 507 (5.1%) |
| strut, (40,60,70) | 69 (0.7%) |
| strut, (40,100,70) | 35 (0.35%) |
| **strut + 3-rung ladder** | **0 of 9,998** |
| strut + ladder, ≤40 nodes, 10,000 seeds | 2 of 9,989 (0.02%) |
| strut + ladder, ≤60 nodes, 4,000 seeds | 6 of 3,990 (0.15%) |
| strut + ladder, ≤100 nodes, 4,000 seeds | 28 of 3,984 (0.7%) |
| strut + ladder, 30 nodes, edges up to 2x nodes (stress) | 7 of 9,993 (0.07%) |

(Rung escalations happened for 0.35-0.7% of graphs; the first rung suffices for all 30 corpus graphs.) The residual failures are real dagre limits: a leaf placed outside its container (e.g. the generated graph `seed667`: 6 nodes, 3 sibling containers, every edge crossing containers). They must become `RenderError::Geometry`, not a drawn overlap. **Executor note:** the exact failing graphs depend on the executor's metrics, so the fail-closed regression test should use the `Engine` test double (hand-built overlapping output), plus a seed search against the real adapter if a natural example is wanted.

**Other dagre behaviours measured:**
- Zero layout errors, zero panics, zero NaN in 80,000+ random graphs and in: empty graph, container-only, single node, self-loop only, self-loop + edge in all four directions, 2-cycle with labels, 3 parallel labelled edges, 4-cycle, disconnected components, isolated nodes, container with cross edges in both directions, 4-deep nest with one leaf, empty child container, hostile labels. All came out hard-clean (the "empty container only" case is 190x127: the title strut sets its width).
- **dagre never panics on garbage input**: node width `NaN`, `inf`, negative, `1e300`, zero, label width `NaN` all return normally; `NaN` propagates into positions (`inf` width → `x = NaN`). `catch_unwind` has nothing to catch on any input I could build; the real guard is a finite/present check on every output coordinate.
- **Silent incomplete output**: a 100-node graph with 10 interleaved containers and random cross edges returned with a leaf having `rank`/`order` but `x == None` (no panic, no error). The adapter must treat `None` x/y as `RenderError::Layout`. In the fuzz domain above it never occurred.
- **Time is super-quadratic** (release; dev profile with deps at opt-level 1, as the workspace sets, is within 15%): random 50n/100e 46 ms; 100/200 208 ms; 150/300 326 ms; 200/400 ~1 s; 300/600 3.1 s; **500/1000 16.6 s (19.6 s dev)**. Pathological chain + skip-edges: 100n/197e 1.4 s, **150n/297e 3.3 s** (dev). Containers help (300n/450e in 10 containers: 0.8 s). 500-node chain: 43 ms.
- Output size: 10 corpus SVGs = 94 KB total (max 15.6 KB); all 30 = 184 KB.

## Determinism (REND-02)

**Source audit** [VERIFIED: read dagre-0.1.1 src]: `Graph` keeps `HashMap`s for lookup but every iteration API (`nodes()`, `edges()`, `children()`, `in_edges()`, `out_edges()`, `predecessors()`, `successors()`, `neighbors()`) walks insertion-order `Vec`s. The only `HashMap::values()` iteration is in `remove_node` (removing incident edges), whose end state is order-independent. `graph/alg.rs` (the `HashMap`-heavy module) is not called by the layout path. Float ops in the layout path: `+ - * /`, `abs`, `min`, `max`, `sqrt` (only `intersect_ellipse`, which `assign_node_intersects` does not use: it uses `intersect_rect`). `sin`/`cos` appear only at `intersect.rs:253-254`, inside a `#[test]`. No `par_iter`, no `thread`, no `sort_unstable`, no time or randomness.

**Executed evidence** (config: strut + ladder, 30 corpus graphs and 2,000 random graphs; hash = FNV-1a over all SVG bytes):

| Run | Result |
|-----|--------|
| 20 separate macOS processes (fresh `HashMap` seeds), earlier separation constants | one distinct hash for the corpus and one for the random set |
| macOS, final constants, 3 processes | corpus `c97515a5cf279993`, random set `a18ed4f062b1dc64` |
| Linux aarch64 (`rust:1.98.1` in Colima/Docker), final constants | identical both hashes |
| Linux x86_64 (`--platform linux/amd64`, QEMU, `uname -m` = `x86_64`), earlier constants | identical to macOS for both hashes (`931ea7fc…` / `11779b67…`) |
| Linux x86_64, final constants | identical both hashes (`c97515a5cf279993`, `a18ed4f062b1dc64`) |

So macOS = Linux aarch64 = Linux x86_64 byte-for-byte for the whole pipeline. CI is `ubuntu-latest` [VERIFIED: .github/workflows/ci.yml:25,165,213,300,341]; a Windows job is compile-only and advisory. Residual assumption: QEMU's x86_64 float emulation matches real x86_64 hardware (Rust does not contract `a*b+c` into FMA, so I expect so) [ASSUMED].

**Design consequences:** keep the quantization anyway (defence in depth against a future dagre/dugong bump that changes a last bit); declare the `dagre` pin exact; commit goldens from macOS and let hosted Linux compare; `.gitattributes` already has `* text=auto eol=lf` so `.svg`/`.html` goldens are LF everywhere [VERIFIED: .gitattributes:2]. In-process repetition is a meaningful test here because every `dagre::Graph` instance builds fresh `HashMap`s (different `RandomState` per instance): run the same IR through 20 fresh graphs and compare bytes. Nextest runs each test in its own process, which adds cross-process coverage for free.

**Optional local CI-parity recipe** (not gating; the repo is under `$HOME`, which Colima mounts): `docker run --rm -v "$PWD":/w -w /w -e CARGO_TARGET_DIR=/tmp/t rust:1.98.1 cargo test -p vox-diagram --test svg_goldens` for aarch64, add `--platform linux/amd64` for x86_64. Both worked for the spike (copied via `tar`, because the scratchpad is outside the mount).

## Text Measurement (decision 3)

**Table source.** ASCII advance widths per 1000 em from **Arimo Regular** (`google/fonts` `ofl/arimo/Arimo[wght].ttf`, METADATA `license: "OFL"`, metric-compatible with Arial/Helvetica), read with `ttf-parser` in the spike. Cross-check against Adobe's Helvetica AFM: 93 of 95 printable ASCII widths identical; the two differences are `'` (AFM maps 39 to `quoteright` 222, Arimo `quotesingle` 191) and `` ` `` (222 vs 333). Use Arimo's (browser behaviour). Over every label in the corpus the Arimo and Helvetica totals differ by 124 of 4,338,126 units. The table is 95 integers; embed as `const ADV: [u16; 95]` with a provenance comment (font, version 1.341, OFL, "advance widths only, no glyph data"). Legal sign-off on "numeric advance widths are facts, not font software" is an assumption (A1); the Adobe AFM carries an Adobe copyright header, so Arimo/OFL is the cleaner source.

The 95 values (chars 32..=126): `278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556×10 (digits), 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584` (full literal in Code Examples).

**Browser-vs-table ratio** [VERIFIED: Playwright Chromium 1248, macOS, 309 ASCII label lines from the corpus, `getComputedTextLength` at 14 px]:

| Font stack | mean | p95 | max | lines >8% over table |
|------------|------|-----|-----|----------------------|
| `sans-serif` / Helvetica / Arial | 0.997 | 1.000 | 1.001 | 0 |
| `system-ui` (SF on this Mac) | 1.030 | 1.057 | 1.098 | 3 |
| Helvetica Neue | 1.001 | 1.023 | 1.037 | 0 |
| Tahoma / Trebuchet MS | 0.979 / 0.990 | 1.007 / 1.045 | 1.054 / 1.100 | 0 / 2 |
| **Verdana (stand-in for DejaVu-class fallbacks)** | **1.123** | 1.161 | 1.196 | 296 |
| Menlo (monospace stress) | 1.261 | 1.407 | 1.576 | 298 |

Direct TTF measurement of **DejaVu Sans** (the Linux default fallback): +12.1% corpus-weighted, worst single label +19.7% vs Arimo [VERIFIED: spike `fonts` bin]. So 8% slack covers the system stack on Apple/Windows/Android-class fonts but not DejaVu/Verdana on average. It still holds because padding absorbs the difference: text is centred, so each side overflows by `0.5·(r−1.08)·w`; with `PAD_X = 14` that is safe up to `w ≈ 28/(r−1.08)` = 240 px at r = 1.197, and the wrap constant bounds `w` at about 210 px. **Measured** fit check (text `getBBox()` vs its shape, 144 labels of the 10 spike diagrams): `system-ui` 0 overflows, Arial 0, Trebuchet 0, **Verdana 1 (worst 0.8 px)**, Menlo 77 (worst 30 px). Recommendation: keep 8% slack and `PAD_X = 14`, `PAD_Y = 10`; Phase 89's two-font-stack Playwright check will be the real gate and should use Verdana/DejaVu as its wide stack. Do not promise monospace fallbacks.

**Non-ASCII** (Chromium, 14 px; "em" = width/14):

| Char class | measured | naive model (narrow 0.6 em, wide 1.0 em) | verdict |
|------------|----------|------------------------------------------|---------|
| Latin-1/Ext-A accents (é ü) | 0.56-0.63 | 0.60 | ok |
| `·` | 0.29-0.36 | 0.60 | over-reserves, fine |
| `→ — ✓ ≥ ½` | 0.62-1.00 | 0.60 | **under** by up to 40% |
| Cyrillic capital Ж | 0.97-1.01 | 0.60 | **under** 40% |
| CJK (日 本語) | 0.99-1.02 per char | 1.00 per char (2 cells) | ok |
| Hangul 한 | 0.87 | 1.00 | ok |
| Emoji (👍 🚀 🇯🇵 ZWJ family ❤️ ⚠️) | **1.29** per emoji | 1.00 (or 0.6) | **under 29%** |

Recommended model: ASCII table; U+00A0-U+017F = 0.6 em; other narrow non-ASCII = 1.0 em; wide (`unicode-width` 2 cells) = 0.5 em per cell, i.e. 1.0 em per CJK char; any char in U+1F000-U+1FFFF or followed by U+FE0F = 1.3 em; zero-width = 0; treat `\t` as a space. ZWJ sequences count each component (over-estimate, safe).

**CONTEXT ambiguity to confirm (A2):** "wide/CJK = 2 em" cannot mean 2 em per character: measured CJK is 1.0 em per character, so I read it as "2 cells at 0.5 em". Taking it literally would draw CJK boxes twice too wide.

**Wrapping algorithm** (matches the locked decision; prototype in Code Examples): split on explicit `\n`; within a line split on spaces and greedily fill to 28 cells (cells = sum of `unicode-width`); a single word longer than 28 cells is hard-broken on char boundaries (never inside a zero-width follower); empty tokens collapse (matches SVG whitespace collapsing, so measurement stays conservative); container titles wrap by measured pixel width of the final container.

## SVG Writer Design (decision 4)

**Escaping (single `esc(&str, attr: bool)`):** always `& → &amp;`, `< → &lt;`, `> → &gt;` (this alone makes `]]>` impossible in text, so no CDATA is ever used); additionally in attributes `" → &quot;`, `' → &#39;`; replace XML-1.0-illegal and C1 controls (U+0000-0008, 000B, 000C, 000E-001F, 007F-009F, FFFE, FFFF) with U+FFFD (the DSL lexer already forbids them, but `Ir` is public); also replace bidi controls (U+061C, 200E/F, 202A-202E, 2066-2069) and U+2028/2029 for defence in depth. Attributes carrying user text: none (ids, classes, numbers, fixed strings only). User text appears only in `<text>/<tspan>` content and `<title>/<desc>` content.

**Ids:** `{prefix}-title`, `{prefix}-desc`, `{prefix}-arrow`; prefix validated `[a-z][a-z0-9-]*` (add a length cap, e.g. 32) → `RenderError::InvalidIdPrefix`; default `vd`. `aria-labelledby="{p}-title {p}-desc"`, `marker-end="url(#{p}-arrow)"`. No user-derived id, class, href or style.

**`<title>`:** `IrGraph` has no title field [VERIFIED: ir.rs:50-60 `GraphIr { direction, nodes, containers, edges }`], so the "diagram title" must be `SvgOpts.title: Option<String>` (caller supplies, e.g. page heading); default `"Diagram"`.

**`<desc>`:** `"{N} nodes, {M} edges: a → b; …"`, truncated on a char boundary with `…`. Recommendation: use labels (first line, whitespace collapsed, each capped ~24 chars) rather than dotted ids: screen readers read ids like `LocalModels.L2` badly. CONTEXT's "a → b" is ambiguous (A3); ids worked in the spike.

**Theming:** inline `style="fill:var(--vd-node-fill,#fff);stroke:…"`. `var()` inside `style` is verified in Chromium for standalone and inline SVG, and a custom-property override on an ancestor recolours the same markup (light and dark screenshots from one file). Presentation *attributes* with `var()` were not tested and are not needed. Inline `style` outranks any stylesheet rule on the fixed classes, so the site can theme **only** through `--vd-*` variables, not by restyling classes (fine, and it is why the contract uses variables). Fixed variable set (suggested): `--vd-text, --vd-muted, --vd-node-fill, --vd-node-stroke, --vd-edge, --vd-container-fill, --vd-container-stroke, --vd-bg, --vd-font`. Optional size win: because `fill`/`stroke`/`font-*` are inherited properties, put the declarations once on a wrapping `<g style=…>` (nodes, edges, labels) instead of on every element (~50% smaller SVG); still no `<style>`.

**Fallback palette contrast** (computed): text `#1f2328` on `#fff` 15.8:1; edge/muted `#57606a` on `#fff` 6.39:1, on container fill `#f6f8fa` 6.00:1.

**Shapes and geometry:** `rect` (rx 3), `rounded` (rx 10), `diamond` (polygon, sized `W = 2(tw+14)`, `H = 2(th+10)`, min 96x64; **clip edge endpoints to the diamond outline** with `s = |dx|/(w/2) + |dy|/(h/2); p = c + d/s`, since dagre ends edges on the bounding rect), `cylinder` (two arcs, `ry = 8`, `+16` height), `start`/`end` (pill rx = h/2; `end` stroke 3). Edge path: points → `M … L … Q corner` with fixed radius 8, `r = min(8, l1/2, l2/2)`; `sqrt` only (IEEE-exact); write `x*x` not `powi`. Marker: one `<marker id="{p}-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="8" markerHeight="8" orient="auto">` with a triangle path; arrow fill `var(--vd-edge, …)` resolves from the SVG's ancestors (verified in the dark render). Label text: `text-anchor="middle"`, explicit `<tspan x dy>` lines, baseline `y = cy - (n-1)·9 + 5`; no `dominant-baseline`. Edge-label background rect uses `var(--vd-bg, #fff)` so it hides the line behind the label. Draw order: containers by nesting depth (derive from parent links), edges, nodes, edge labels.

**viewBox / sizing:** `viewBox="0 0 W H"` from the union bounding box of everything incl. container title bands, +16 px margin, shifted so the minimum is the margin; emit no `width`/`height` (the page decides). Beware: a 2062 px tall diagram scales up to the column width; at 760 px, 11 of the 30 corpus diagrams have an effective scale below 0.6 (down to 0.41 for `18-scientia…`, 0.49 for a 1566x106 LR chain). Legibility policy is Phase 89's, but the planner should know the numbers.

## Theming and Starlight tokens (REND-04)

Starlight 0.38.3 defines `--sl-color-white … black` on `:root` (dark) and inverts them under `:root[data-theme='light']` [VERIFIED: docs-astro/node_modules/@astrojs/starlight/style/props.css:5-12,123-134]. `--sl-color-white` is the strongest foreground in both themes and `--sl-color-black` is the page background in both. Candidate map (lives in the contact sheet CSS and the Phase 89 site CSS; the crate itself stays token-agnostic):

```css
.vox-diagram, .vd-sheet {
  --vd-text: var(--sl-color-white);
  --vd-muted: var(--sl-color-gray-2);
  --vd-node-fill: var(--sl-color-black);
  --vd-node-stroke: var(--sl-color-gray-3);
  --vd-edge: var(--sl-color-gray-3);
  --vd-container-fill: var(--sl-color-gray-6);
  --vd-container-stroke: var(--sl-color-gray-5);
  --vd-bg: var(--sl-color-black);
  --vd-font: var(--sl-font-system);
}
```
WCAG contrast computed from the literal hsl token values (`contrast.mjs`, scratchpad): **dark** text on node fill 17.70:1, container title on container fill 8.54:1, edge/node stroke on page bg 5.24:1 and on container fill 4.47:1, container border 1.51:1 (decorative); **light** 17.70, 10.14, 7.02, 6.07, container border 1.77:1. All text ≥4.5 and all edges/node strokes ≥3 in both themes. `--sl-color-gray-7` exists only in the light block, so avoid it. Because Phase 85 has no Astro, make SC5 testable in Rust: a test that (a) asserts every colour in the SVG is a `var(--vd-*, fallback)` (no literal paint outside a fallback) and the variable set equals the documented const, and (b) recomputes the contrast table above from two `const` token sets copied from Starlight (hsl→sRGB→luminance, ~40 lines) and asserts the thresholds, plus the contact sheet showing both themes from the same inline SVG.

## Property-Test Strategy and Contact Sheet (decision 5)

**Contract to test:** `render(ir) = Ok(svg)` implies all invariants; `Err` is allowed only as `Layout | Geometry | TooLarge | InvalidIr` and never from the generator's *guaranteed* band (≤20 nodes, edges ≤ nodes: 0 failures in 9,998). Do not assert "never Err" over S1's full range (≤30 items, 0.02-0.07% Err).

Hard invariants (checked on the quantized integer geometry, exact, no epsilon): all values finite/in range; no two leaf rects overlap; every leaf lies inside its container rect; every container lies inside its parent; sibling containers do not overlap; no leaf overlaps a non-ancestor container; container title band is free of the container's children; every node label's measured text fits its box (diamond: half-extents); every edge point, label rect, node and container is inside the viewBox. Soft metrics (record, do not fail): edge passes through a non-endpoint node, edge label overlaps a node or another label.

**Generators:** `model_graph()` S1 → `noisy_source`/`print` → `check` → `Ir` (S1 guarantees validity), 128 cases (the crate default); a second strategy for hand-built hostile IR (`FRAGMENTS`, `graph_cases()`); a "large" strategy only for time-bound/`TooLarge` tests. Determinism property: render twice and 20 fresh in-process layouts → identical bytes.

**SVG-level properties** (`quick-xml` checker, prototype verified): well-formed; element set ⊆ `{svg,title,desc,defs,marker,path,rect,polygon,text,tspan,g}`; attribute set ⊆ a fixed list; no `<style>`, `foreignObject`, `script`, comment, CDATA, PI; ids unique; every `url(#x)` and `aria-labelledby` token resolves; `style` values contain no `url(`, `@import`, `expression`, `javascript`; entities ⊆ `{amp, lt, gt, quot, #39}`. quick-xml 0.41 API notes: whole-text entities arrive as separate `Event::GeneralRef` events (name via `.decode()`), and `Attribute::unescape_value` is deprecated in favour of `normalized_value(quick_xml::XmlVersion::Implicit1_0)`.

**Contact sheet:** an env-gated test (`CONTACT_SHEET_OUT=<path>`, otherwise `#[ignore]`) that reads the committed `.svg` goldens and writes one static HTML file: for each of the 10 spike diagrams a header (counts, size, hard/quality) and the same inline SVG twice, once in a light pane and once in a dark pane whose only difference is the `--vd-*` custom properties from the Starlight map above. No timestamps, no versions, so it is reproducible; commit it to the phase directory. I produced a working prototype of exactly this (`contact.html`, dual panes, in the scratchpad). Goldens: `tests/fixtures/svg/<stem>.svg` through `assert_golden`.

## Common Pitfalls

### Pitfall 1: `dagre` does not compile without its default features
**What goes wrong:** `dagre = { version = "0.1.1", default-features = false }` (the recommendation in `research/STACK.md` §2 and §Installation) fails with `error[E0432]: unresolved import serde` / `cannot find attribute serde` in `src/graph/json.rs`.
**Why:** `pub mod json;` in `dagre/src/graph/mod.rs:7` is not behind `#[cfg(feature = "json")]`; the feature only toggles the optional `serde`/`serde_json` deps [VERIFIED: failing build in spike; dagre Cargo.toml `[features] default = ["json"]`].
**How to avoid:** use default features. No cost: `serde`/`serde_json` are already in the lock.
**Warning signs:** the first `cargo check` after adding the dep.

### Pitfall 2: trusting dagre's container box
**What goes wrong:** container narrower than its title (860 of 3,000 random graphs, and `03-axis-harness`'s "UI" container in the corpus), leaf outside its container, sibling containers or foreign nodes overlapping.
**Why:** compound size = last-rank border distance; titles are unknown to dagre; `edgesep` is the border-to-border gap. Growing a container after layout makes overlaps (10-20% of random graphs).
**How to avoid:** strut + ladder + verify (Pattern 2). **Warning signs:** `verify` firing on a corpus graph.

### Pitfall 3: silent failure instead of a panic
**What goes wrong:** the layout returns with `x` or `y` `None`, or `NaN` coordinates, and the writer prints `NaN` or skips a node.
**How to avoid:** the seam reads every coordinate through a checked accessor (`finite`, present, `|v| < 1e7`) and returns `RenderError::Layout`; test with an `Engine` double that returns `NaN`/missing. Keep `catch_unwind` (the workspace does not set `panic = "abort"` and explicitly relies on unwinding [VERIFIED: Cargo.toml:453-459]; precedent `vox-vcs` `guarded!` at `crates/vox-vcs/src/jj_actor.rs:194`), but test it only through the double.

### Pitfall 4: parse caps are not render caps
`MAX_NODES = 500`, `MAX_EDGES = 1_000` [VERIFIED: limits.rs:6,8] are parse limits; laying out that much takes ~17-20 s and can pin an MCP worker or a nextest slot. **Avoid:** new `MAX_RENDER_NODES = 100`, `MAX_RENDER_EDGES = 200` (worst measured at that size: 1.4 s adversarial dev-profile, 0.2 s random; corpus max is 18/20), returning `RenderError::TooLarge { nodes, edges }` before any layout. CONTEXT says "caps reused from limits.rs": add the two constants there so they stay named and shared.

### Pitfall 5: escape guard that passes against a broken escape
`quick-xml` happily accepts a raw `]]>` and a raw `>` in text (it only complains about `<`/`&`). A guard test built on "parses OK" therefore passes with the `>` arm deleted. **Avoid:** the guard must also assert, per text/attribute node, that none of `< > "` and no control char appear unescaped (iterate `Event::Text` slices), and the mutation proof must remove each arm separately: all of `esc` (caught by "element not allowed: script", measured), only `<`, only `>`, only `&`, only `"`/`'` (attribute context), only the control-char arm. Use the repo's presence-check protocol (84-05 SUMMARY): `grep -c` the arm before, mutate, run the one test binary, expect red, restore, `grep -c` after, `git diff --quiet -- crates/vox-diagram/src`.

### Pitfall 6: using `Ir` fields the DSL guarantees but hand-built IR does not
`depth`, `local_id`, unique ids, resolvable `parent`/`container`/`from`/`to`, acyclic parents. **Avoid:** `validate_ir` first (cap + ref + cycle checks, O(n)); derive depth from parent links; never index maps with `[]` on IR strings (the spike's `cidx[p]` would panic outside `catch_unwind`; the crate denies `clippy::unwrap_used/expect_used/panic` outside tests [VERIFIED: lib.rs `deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)`]).

### Pitfall 7: font fallback wider than the table
See Text Measurement: slack 8% is not enough for DejaVu/Verdana on average, padding saves it, monospace fails, emoji under-measure by 29%. **Avoid:** the non-ASCII model above; keep slack and padding single constants; Phase 89 must test a wide stack.

### Pitfall 8: edge "passes through a node" treated as a bug to fix
It is dagre behaviour for long edges between adjacent ranks beside wide nodes; the corpus has one grazing case. **Avoid:** soft metric; do not tune the DSL or add obstacle routing in this phase.

### Pitfall 9: parallel phases and `Cargo.lock`
84/85/87 run in parallel; `Cargo.toml`/`Cargo.lock` are CI sentinels, so this PR runs the **full-workspace suite** (~21 min sharded) [CITED: docs/src/ci/affected-crate-selective-ci.md "Sentinels"; CLAUDE.md "CI Contract"]. **Avoid:** one owner plan declares all three dependency edits (as 83-01 did), land them once; on a `Cargo.lock` merge conflict regenerate with `cargo metadata` rather than hand-merging.

### Pitfall 10: tall/wide diagrams scale poorly in the content column
See viewBox note. Not a Phase 85 failure, but record the per-diagram scale in the decision doc.

## Code Examples

All prototyped and executed in the scratchpad spike (compiled on 1.98.1; adapt, do not paste: add `Result` plumbing, `#[must_use]`, tests, and avoid `unwrap`/`[]` indexing per the crate lints).

### Seam types (integers after the boundary)
```rust
// layout/mod.rs
pub(crate) type Half = i32; // 1 unit = 0.5 px
pub(crate) struct Rect { pub x: Half, pub y: Half, pub w: Half, pub h: Half }

pub(crate) struct LayoutInput {
    pub direction: crate::ir::Direction,
    pub nodes: Vec<LNode>,          // index == IR node order; size in half-px
    pub containers: Vec<LContainer>, // parent: Option<usize>, title_w
    pub edges: Vec<LEdge>,           // from, to, label: Option<(w,h)>
    pub sep: Separations,            // nodesep, ranksep, edgesep, in px
}
pub(crate) trait Engine {
    fn solve(&self, i: &LayoutInput) -> Result<RawLayout, LayoutError>;
}
/// Never lets a panic escape; the closure is what tests replace with `|| panic!()`.
pub(crate) fn guarded<T>(f: impl FnOnce() -> T) -> Result<T, LayoutError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).map_err(|_| LayoutError::EnginePanic)
}
```

### dagre adapter core (`layout/dagre_impl.rs`)
```rust
use dagre::graph::{Graph, GraphOptions};
use dagre::layout::types::LabelPos;          // NOT re-exported at the crate root
use dagre::{EdgeLabel, LayoutOptions, NodeLabel, RankDir};

let mut g: Graph<NodeLabel, EdgeLabel> =
    Graph::with_options(GraphOptions { directed: true, multigraph: true, compound: true });
for i in 0..containers.len() { g.set_node(format!("c{i}"), Some(NodeLabel::default())); }
for (i, n) in nodes.iter().enumerate() {
    g.set_node(format!("n{i}"), Some(NodeLabel { width: n.w, height: n.h, ..NodeLabel::default() }));
}
for (i, c) in containers.iter().enumerate() {            // title strut: forces width >= title
    g.set_node(format!("t{i}"), Some(NodeLabel { width: c.title_w, height: 1.0, ..NodeLabel::default() }));
    g.set_parent(&format!("t{i}"), Some(&format!("c{i}")));
    if let Some(p) = c.parent { g.set_parent(&format!("c{i}"), Some(&format!("c{p}"))); }
}
// node -> container: g.set_parent("n{i}", Some("c{k}")) ; edges (name makes parallel edges legal):
g.set_edge(format!("n{f}"), format!("n{t}"),
    Some(EdgeLabel { width: lw, height: lh, labelpos: LabelPos::Center, ..EdgeLabel::default() }),
    Some(&format!("e{i}")));
let opts = LayoutOptions { rankdir, nodesep, ranksep, edgesep, marginx: 16.0, marginy: 16.0, ..Default::default() };
guarded(|| dagre::layout(&mut g, Some(opts)))?;
// read-back: node center x/y + width/height (containers get computed width/height);
// edge points via g.edge(v, w, Some(name)).points, label center el.x/el.y; every Option and f64 checked.
```
Facts: node ids are `String`; `layout` returns `()`; `Graph::set_edge(v, w, label, name)`; self-loops return 5 points to the right of the node in the (pre-rotation) layout frame and work in all four directions; reversed (cycle) edges come back oriented from → to; `LayoutOptions::tie_keep_first` (default false = dagre.js 3.x) only matters for byte parity with Mermaid's older dagre; leave it.

### Half-pixel number formatting (no float is ever formatted)
```rust
pub(crate) fn num(h: Half) -> String {           // h is already quantized
    let (neg, a) = (h < 0, h.unsigned_abs());
    let body = if a % 2 == 0 { format!("{}", a / 2) } else { format!("{}.5", a / 2) };
    if neg { format!("-{body}") } else { body }   // a == 0 cannot be negative
}
fn quantize(v: f64) -> Result<Half, LayoutError> {
    if !v.is_finite() || v.abs() >= 1.0e7 { return Err(LayoutError::NonFinite); }
    Ok((v * 2.0).round() as Half)
}
```

### Escape + the allowlist guard (REND-03)
```rust
pub(crate) fn esc(s: &str, attr: bool) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"), '<' => o.push_str("&lt;"), '>' => o.push_str("&gt;"),
            '"' if attr => o.push_str("&quot;"), '\'' if attr => o.push_str("&#39;"),
            '\u{0}'..='\u{8}' | '\u{b}' | '\u{c}' | '\u{e}'..='\u{1f}' | '\u{7f}'..='\u{9f}'
            | '\u{fffe}' | '\u{ffff}' | '\u{61c}' | '\u{200e}' | '\u{200f}'
            | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{2028}' | '\u{2029}' => o.push('\u{fffd}'),
            c => o.push(c),
        }
    }
    o
}
// test: Reader::from_str(svg); on Event::Text(t): assert!(!t.contains('>') && !t.contains("]]>"));
//       on Start/Empty: element in ALLOW, every attribute key in ALLOW_ATTR, id unique,
//       attr values via a.normalized_value(XmlVersion::Implicit1_0); Event::GeneralRef(g): name in {amp,lt,gt,quot,#39}.
```
Measured: hostile fixture passes; deleting `esc` is caught (`element not allowed: script`); deleting only the `]]&gt;`→`]]>` arm is **not** caught by quick-xml alone (hence the explicit text assertion).

### Wrapping (cells), measuring, node size
```rust
const FONT_PX: f64 = 14.0; const LINE_H: f64 = 18.0; const SLACK: f64 = 0.08;
const WRAP_CELLS: usize = 28; const PAD_X: f64 = 14.0; const PAD_Y: f64 = 10.0;
fn text_w(s: &str) -> f64 { s.chars().map(adv1000).sum::<u32>() as f64 * FONT_PX / 1000.0 * (1.0 + SLACK) }
// adv1000: ASCII -> ADV[c-32]; U+00A0..=U+017F -> 600; zero-width -> 0; wide (width 2) -> 500 per cell;
//          U+1F000..=U+1FFFF (or +FE0F) -> 1300; \t -> space; other narrow non-ASCII -> 1000.
// node_size: rect/rounded (tw+28, th+20) min 64x38; diamond (2(tw+14), 2(th+10)) min 96x64;
//            cylinder (tw+28, th+20+16) min 64x54; start/end (tw+40, th+20) min 72x38.
```
(`ADV` literal: the 95 numbers listed in Text Measurement, chars 32..=126.)

### Ladder + verify
```rust
for (nodesep, ranksep, edgesep) in [(40.0, 60.0, 70.0), (50.0, 90.0, 120.0), (60.0, 140.0, 200.0)] {
    let raw = engine.solve(&input.with_sep(nodesep, ranksep, edgesep))?;   // Err(Layout) is final: no retry
    let placed = quantize_and_finish_containers(raw)?;                      // pad, title band, wrap title
    if verify(ir, &placed).is_ok() { return Ok(placed); }
}
Err(RenderError::Geometry(/* first violation of the last rung */))
```

### Corpus test (live path, not the `.vdg` goldens)
`for each tests/fixtures/corpus/*.mmd: mermaid::import → check → Ir::Graph`; rank by `nodes+edges` descending with fixture-name tiebreak; assert the chosen 10 equal a frozen list; `assert_golden(tests/fixtures/svg/<stem>.svg, &render(...)?)`. The `corpus-roundtrip/*.vdg` files are the importer's own output and can be a cross-check.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Mermaid dagre-d3-es / dagre.js 0.8.5 (keeps first tied sweep) | `dagre` 0.1.1 tracks dagre.js 3.0.1-pre (keeps last) via `tie_keep_first: false` | 2026 | layouts can differ from today's Mermaid renders; `tie_keep_first: true` opts back, untested here |
| `svg` crate builder | `String` writer, fixed order | STACK.md | determinism |
| client-side Mermaid + CDN | pre-rendered inline SVG | milestone | Phase 89 |

**Deprecated/outdated:** `quick-xml` `Attribute::unescape_value` (use `normalized_value`); `research/STACK.md`'s `default-features = false` for dagre (wrong, see Pitfall 1).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Embedding Arimo-derived numeric advance widths (no glyph data) needs no licence action beyond a provenance comment (OFL covers font software; widths are facts) | Text Measurement | licence objection; fallback: Adobe core-14 AFM header terms, or derive from DejaVu (Bitstream Vera licence) |
| A2 | CONTEXT "wide/CJK = 2 em" means 2 half-em cells (1.0 em per CJK char), not 2 em | Text Measurement | boxes for CJK labels 2x too wide if read literally |
| A3 | `<desc>` should list labels, not dotted ids | SVG Writer | cosmetic/a11y quality only |
| A4 | QEMU x86_64 float emulation equals real x86_64 hardware for this code | Determinism | hosted-Linux golden mismatch on first CI run (cheap to see, fix is quantization/golden refresh) |
| A5 | `var()` in inline `style` behaves like Chromium in Safari and Firefox | SVG Writer | Phase 89 browser matrix may find a gap; only Chromium 1248/macOS was run |
| A6 | Render caps of 100 nodes / 200 edges are acceptable product limits | Pitfall 4 | an author's real diagram rejected; relax only with a measured budget |
| A7 | The fuzz generator (12-word label soup, random edges) is harder than real docs diagrams | Spike Results | residual `Geometry` failures could be more frequent on real input than 0.02% (corpus: 0 of 30) |
| A8 | The go/no-go decision doc belongs in `docs/src/architecture/` (project rule: research there) | Open Questions | doc placement/lint churn |

## Open Questions

1. **Where does the go/no-go decision doc live and what is its name?**
   - Known: CONTEXT says "decision doc"; CLAUDE.md says project research goes to `docs/src/architecture/` with frontmatter and `<!-- kb:summary -->`.
   - Unclear: phase directory vs `docs/src/architecture/`.
   - Recommendation: write `docs/src/architecture/voxdiagram-layout-go-no-go-2026.md` (frontmatter, lint with `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/<file>.md`), and keep the contact sheet + SVG goldens as the phase-directory/fixture evidence it links to.
2. **Render caps (A6).** 100/200 is my recommendation; the product owner may want 150/300 (0.3 s random, 3.3 s adversarial in dev).
3. **`desc` content (A3) and `SvgOpts.title`.** Confirm labels vs ids and that the caller supplies the title.
4. **Effective-scale policy** (11 of 30 diagrams <0.6 at 760 px): belongs to Phase 89/90, but it may motivate a tighter wrap or `nodesep` now. I would not change the locked 28-cell wrap in this phase.
5. **Whether `render` should return `Err(Geometry)` or a best-effort SVG** for the ~0.02% residual. Recommendation: `Err` (fail closed; the author tweaks the DSL); it matches "diagrams that cannot render-crash" and keeps the property contract clean.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | build/test | yes | 1.98.1 (`rust-toolchain.toml`) | - |
| `cargo-nextest` | test command | yes | in `~/.cargo/bin` | `cargo test -p vox-diagram` |
| `cargo-hakari` / `cargo-deny` | dep mechanics | yes | 0.9.39 / 0.20.2 | CI runs both |
| Docker (Colima) | optional Linux parity run | yes | Docker 29.9, Colima aarch64, amd64 via QEMU | hosted CI is the real gate |
| Chromium via docs-astro `playwright-core 1.63.0` | contact-sheet screenshots, fit checks (my scratch scripts) | yes | Chromium 1248 cached | none needed for the crate tests |
| `vox` binary (`vox ci crate-api-sync`) | API-page regen | built by earlier phases at `target/debug/vox` (per 84-05) | - | `cargo run -p vox-cli -- ci crate-api-sync --write` |
| Node ≥ 20 | existing Phase 84 oracle only | yes (v26.11.1) | - | n/a for Phase 85 |

Missing with no fallback: none. Note for executors: the Bash tool in this environment runs through a shell allowlist that rejects arbitrary binaries by name (e.g. a binary literally named `run`); invoke spike-like tools via `cargo run`/`cargo nextest`.

## Validation Architecture

(`workflow.nyquist_validation` is absent from `.planning/config.json` → enabled.)

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo-nextest` over `cargo test` binaries; `proptest` (workspace); byte-golden helper `assert_golden`; `quick-xml` (new dev-dep) |
| Config file | `.config/nextest.toml` (profiles `default`, `ci`, `ci-gate`) |
| Quick run command | `cargo nextest run -p vox-diagram -E 'binary(svg_goldens) | binary(svg_hostile) | binary(layout_failclosed)'` |
| Full suite command | `INSTA_UPDATE=no cargo nextest run -p vox-diagram` (340 pass / 1 skipped before this phase) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| REND-01 | top-10 corpus graphs lay out, render, match goldens, 0 hard violations; containers/labels/cycles/self-loops cases | integration + golden | `cargo nextest run -p vox-diagram -E 'binary(layout_corpus_spike)'` | Wave 0 |
| REND-01 | `catch_unwind`, NaN, missing-x, overlap, caps → `RenderError` never a panic | unit (engine double) | `cargo nextest run -p vox-diagram -E 'binary(layout_failclosed)'` | Wave 0 |
| REND-01 | geometry invariants hold for generated graphs; `Err` only in allowed kinds | property | `cargo nextest run -p vox-diagram -E 'binary(layout_geometry_props)'` | Wave 0 |
| REND-02 | 20 fresh in-process renders identical; goldens byte-equal on macOS and hosted Linux | determinism + golden | `cargo nextest run -p vox-diagram -E 'binary(svg_determinism) | binary(svg_goldens)'` | Wave 0 |
| REND-02 | no `HashMap`/`HashSet` in `src/`; no transcendental calls in layout/svg code | guard | `cargo nextest run -p vox-diagram -E 'test(determinism_guard)'` (extend needles: `.sin(`, `.cos(`, `.powf(`, `.exp(`, `.ln(`, `.atan`) | extend existing |
| REND-03 | hostile labels (`FRAGMENTS`, `graph_cases()`, `]]>`, `<script>`, quotes, `&`, controls) produce allowlisted well-formed SVG; no `<style>`/`foreignObject`/user attrs; ids unique; title/desc present | property + fixture | `cargo nextest run -p vox-diagram -E 'binary(svg_hostile)'` | Wave 0 |
| REND-03 | escape guard mutation-proven (each arm) | mutation recipe | presence-check → mutate `esc` arm → `cargo nextest run -p vox-diagram -E 'binary(svg_hostile)'` must fail → restore → presence-check → `git diff --quiet -- crates/vox-diagram/src` | procedure, recorded in SUMMARY |
| REND-04 | only `var(--vd-*, fallback)` paints; variable set matches const; contrast thresholds hold for both Starlight token sets; contact sheet shows both themes | unit + artifact | `cargo nextest run -p vox-diagram -E 'binary(svg_theme)'`; `CONTACT_SHEET_OUT=<phase-dir>/85-CONTACT-SHEET.html cargo test -p vox-diagram --test layout_corpus_spike -- --ignored contact_sheet` | Wave 0 |
| closing | API page, arch/edge gates, fmt, clippy, rustdoc | gates | `cargo fmt -p vox-diagram --check`; `cargo clippy -p vox-diagram --all-targets -- -D warnings`; `RUSTDOCFLAGS='-D warnings' cargo doc -p vox-diagram --no-deps`; `vox ci crate-api-sync --write` then check; `cargo run -p vox-arch-check`; `vox ci crate-edges` | existing commands |

### Sampling Rate
- **Per task commit:** `cargo check -p vox-diagram` + the one new test binary (background it; never block on cargo).
- **Per wave merge:** `INSTA_UPDATE=no cargo nextest run -p vox-diagram`.
- **Phase gate:** full suite green, clippy/fmt/doc clean, `crate-api-sync` clean; hosted CI (full workspace because `Cargo.toml`/`Cargo.lock` are sentinels) green on Linux against macOS-blessed goldens before `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `tests/layout_corpus_spike.rs` (+ `tests/fixtures/svg/*.svg`) covers REND-01/02; blessing: `UPDATE_GOLDEN=1 cargo nextest run -p vox-diagram -E 'binary(layout_corpus_spike)'`
- [ ] `tests/layout_failclosed.rs` engine doubles (REND-01)
- [ ] `tests/layout_geometry_props.rs`, `tests/svg_determinism.rs`, `tests/svg_hostile.rs`, `tests/svg_theme.rs`
- [ ] `tests/support/svgcheck.rs` (quick-xml allowlist checker) and a geometry-verifier re-export from `layout::verify` for tests
- [ ] dependency edits (`Cargo.toml`, `crates/vox-diagram/Cargo.toml`, `Cargo.lock`) owned by the tracer plan only

## Security Domain

`security_enforcement` is not set in config (absent = enabled).

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | - |
| V3 Session Management | no | - |
| V4 Access Control | no | - |
| V5 Input Validation / Output Encoding | **yes** | single escape writer; `validate_ir`; id-prefix validation; render caps; allowlist checker in tests |
| V6 Cryptography | no | no hashing here (policy: ids are prefix + role) |
| V12/V14 Resource limits / config | yes | `MAX_RENDER_*`, finite-coordinate checks, `catch_unwind`, no `panic = "abort"` |

### Known Threat Patterns for inline-SVG generation
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Label injects `</text><script>`, `foreignObject`, `onload=` | Tampering / XSS | escape `& < >` in text, `& < > " '` in attributes; labels never in attributes; allowlist checker + per-arm mutation proof |
| `]]>` / controls / bidi produce malformed or spoofed text | Tampering | `>` always escaped; controls and bidi → U+FFFD |
| User text becomes id/class/href/style (CSS injection, cf. Mermaid CVE-2026-50159) | Tampering | no user-derived attribute at all; ids = validated prefix + fixed role |
| `<style>` leaks page-wide | Tampering | no `<style>`; inline `style` with fixed property names |
| Duplicate ids across inline SVGs hijack `url(#)` | Spoofing | caller-unique prefix, validated; page-level duplicate-id test is Phase 89 |
| Adversarial size/shape hangs the renderer (MCP path) | DoS | render caps 100/200; super-quadratic cost measured; no wall-clock budget (non-deterministic) |
| Garbage IR (cycles, dangling refs) panics outside the guard | DoS | `validate_ir` before layout; no indexing panics (lints deny `unwrap`/`expect`/`panic`) |

## Sources

### Primary (HIGH confidence)
- Executed spike and experiments in the scratchpad `p85/` (crate copy + dagre; `run`, `fuzz`, `edge`, `scale`, `adv`, `pan`, `fonts`, `xmlcheck` bins; `shot/view/measure/measure2/fit/contrast` Node scripts; Docker aarch64 and amd64 runs; scratch repo copy for hakari/deny).
- `~/.cargo/registry/src/*/dagre-0.1.1` (full source read: `lib.rs`, `layout/{mod,types,position/{mod,bk},add_border_segments,intersect}.rs`, `graph/{mod,alg}.rs`, `Cargo.toml`, `README.md`).
- In-repo, read this session: `crates/vox-diagram/{Cargo.toml, src/{lib,limits,ir,sequence_ir,mermaid/mod}.rs, tests/{determinism_guard,support/{mod,hostile,roundtrip}}.rs, grammar-card.txt}`, `deny.toml`, `.config/hakari.toml`, root `Cargo.toml`, `docs/src/architecture/layers.toml`, `docs/src/ci/affected-crate-selective-ci.md`, `.github/workflows/ci.yml`, Starlight `props.css` (0.38.3), phase 84 `VERIFICATION`/`05-SUMMARY`, milestone `STACK.md`/`PITFALLS.md`/`REQUIREMENTS.md`/`ROADMAP.md`/`STATE.md`.
- `gsd-tools query package-legitimacy check` (crates ecosystem) for `dagre`, `unicode-width`, `dugong`, `quick-xml`.

### Secondary (MEDIUM confidence)
- `research/STACK.md` candidate matrix (dugong/rust-sugiyama/layout-rs/elkrs claims not re-verified this session).

### Tertiary (LOW confidence)
- Safari/Firefox behaviour of `var()` in inline SVG `style` (not run).

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH, built, locked, deny-checked, hakari no-op reproduced.
- Architecture: HIGH for the adapter pattern (fuzzed 100k+ graphs), MEDIUM for residual compound-graph failures beyond 40 nodes.
- Pitfalls: HIGH for 1-6 (each reproduced), MEDIUM for 7 (one OS/browser).
- Determinism: HIGH (three platforms, 2,030 SVGs), minus A4.

**Research date:** 2026-10-16
**Valid until:** 2026-11-15 (30 days; `dagre` is pinned, so the main drift risk is Starlight token changes and Mermaid-parity expectations)
