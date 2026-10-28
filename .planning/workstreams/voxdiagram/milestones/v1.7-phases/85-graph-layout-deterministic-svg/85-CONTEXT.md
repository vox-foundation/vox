# Phase 85: Graph Layout & Deterministic SVG - Context

**Gathered:** 2026-10-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Inside `crates/vox-diagram`: a layout seam (`layout/`) backed by the `dagre` crate behind a one-file adapter, label-aware text sizing, and a deterministic, injection-safe, themeable SVG writer for the `graph` kind, with a recorded go/no-go spike on the 10 largest frozen-corpus diagrams. No sequence rendering (86), no docs-pipeline/astro wiring (87/89), no CLI/MCP (88).

Requirements: REND-01, REND-02, REND-03, REND-04.

</domain>

<decisions>
## Implementation Decisions

### Layout engine & spike
- DEPENDENCY APPROVED by user in chat (2026-10-16): add `dagre = "=0.1.1"` (Apache-2.0, pure Rust) as a `vox-diagram` runtime dependency (Cargo.toml + Cargo.lock change; run `cargo hakari generate`; cargo-deny licence check on PR). Fallback `dugong 0.9.0` behind the same adapter if the spike fails. This is a third-party crate, not a workspace crate edge.
- One-file adapter `layout/dagre_impl.rs`; the rest of the crate talks to a small layout trait/struct.
- Spike: the 10 largest frozen-corpus diagrams (imported via the Phase 84 importer) rendered to committed SVG goldens plus an HTML contact sheet in the phase directory; go/no-go recorded in a decision doc (adopt dagre only if ≤2 of 10 need DSL tweaks, else dugong).
- Edge routing: dagre polyline points → SVG path with fixed-radius rounded corners using arithmetic only (no sin/cos/powf); arrowheads as generated markers.
- `catch_unwind` around the dagre call + caps reused from `limits.rs`; a caught panic becomes `RenderError`, never a crash.

### Text sizing & font policy
- Static advance-width table (ASCII from a reference sans font) + `unicode-width` (wide/CJK = 2 em) with ~8% slack; SVG text `font-family: var(--vd-font, system-ui, sans-serif)`. Real-browser verification is Phase 89.
- Labels: respect explicit `\n`; soft-wrap at word boundaries past ~28 chars (fixed constant); break mid-word only when a single word exceeds the width.
- Coordinates quantized to 0.5 px with fixed decimal formatting (no exponent, no `-0`) → byte-stable goldens on macOS and Linux.

### SVG contract & theming
- No `<style>`, no `foreignObject`, no user-derived attributes. Fixed generated class names (`vd-node`, `vd-edge`, `vd-container`, `vd-label`, …) plus inline `style` declarations using CSS variables with literal fallbacks (`fill:var(--vd-node-fill,#fff)`), so the SVG renders standalone; the site maps `--vd-*` → `--sl-*` in Phase 89. One SVG serves light and dark.
- Caller-supplied `SvgOpts.id_prefix` (validated `[a-z][a-z0-9-]*`), default `vd`; all ids (markers, title, desc) prefixed; no hashing.
- Accessibility: `role="img"`, `aria-labelledby` → `<title>` (diagram title or "Diagram") + `<desc>` IR summary ("N nodes, M edges: a → b …", truncated); text as `<text>`.
- API: `vox_diagram::svg::render(&Ir, &SvgOpts) -> Result<String, RenderError>`.
- Single escaping writer for all text/attribute output; escape guard mutation-proven.

### Post-research decisions (user, 2026-10-17)
- Spike result: GO for dagre 0.1.1 (0/10 largest corpus diagrams needed DSL tweaks; byte-identical across macOS arm64, Linux arm64, Linux x86_64). Use dagre with DEFAULT features (`default-features = false` does not compile).
- Render caps 100 nodes / 200 edges as new constants in `limits.rs` (parse caps unchanged); larger diagrams still parse/check but `render` returns `RenderError::TooLarge`.
- Container handling: invisible title-strut leaf per container + 3-rung deterministic separation ladder + fail-closed geometry verifier; residual failures return `RenderError::Geometry` (never a drawn overlap).
- Finite/present coordinate check on every dagre output (dagre fails silently, never panics); `catch_unwind` still wraps the engine call, tested via an `Engine` test double.
- Decision record lives at `docs/src/architecture/voxdiagram-layout-go-no-go-2026.md` (full frontmatter + kb:summary per AGENTS.md); it links the committed SVG goldens and the contact sheet.
- Claude's discretion accepted: "wide = 2" means 2 half-em cells (unicode-width semantics); `<desc>` uses labels; `<title>` comes from `SvgOpts.title` (GraphIr has no title); ASCII widths from Arimo Regular (OFL) as numeric data with a provenance comment.

### Carried forward
- No HashMap/HashSet in `src/`; no transcendental floats; no versions/timestamps in output; tests first; files < 400 non-blank lines; crate-api-sync --write after pub API changes; no workspace crate edges.
- UI-SPEC generation skipped: this phase's visual contract is the theming decisions above; browser verification lands in Phase 89.

### Claude's Discretion
- Rank/node separation constants, marker geometry, desc truncation length, module layout, proptest generators, contact-sheet format.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `vox_diagram::{check, Ir}`, `limits.rs`, `mermaid::import` (to turn frozen corpus into DSL/IR), frozen corpus fixtures under `tests/fixtures/corpus/`, Phase 83/84 test support generators (`tests/support/`).

### Integration Points
- `Cargo.toml` (vox-diagram) + `Cargo.lock` + `crates/workspace-hack` (hakari) — sentinel paths, full CI on PR.

</code_context>

<specifics>
## Specific Ideas

Property invariants: no overlapping node boxes, children inside container bounds, labels inside their boxes, all coordinates finite and within the viewBox.

</specifics>

<deferred>
## Deferred Ideas

- Edges to containers (anchor lowering); multiple layout engines.

</deferred>
