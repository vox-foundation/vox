# Requirements: Vox — v1.7 VoxDiagram (workstream `voxdiagram`)

**Defined:** 2026-10-16
**Core Value:** The compiler, orchestrator, runtime, knowledge base, and public docs site stay continuously verified and build-green. For this milestone: agents and humans author diagrams that cannot render-crash, and every live docs diagram renders deterministically.

**Recorded user decisions (2026-10-16, in chat):**
- Scope: IR + emitters (no compiler keyword / LSP / Axis GUI); text DSL in ```` ```vox-diagram ```` fences; migrate the full live corpus; layout choice delegated to research (→ `dagre`).
- **Crate edges pre-authorized:** exactly `vox-doc-pipeline → vox-diagram` and `vox-orchestrator-mcp → vox-diagram`. The CLI reaches the crate via a `vox-doc-pipeline` re-export — no third edge. Any other edge still needs fresh approval.
- Gantt (and any unconvertible block) stays Mermaid on the ratchet allowlist.
- In v1.7: sequence kind, `vox diagram` CLI, LLM authoring harness, machine-applicable fixes.

## v1.7 Requirements

### Language & Diagnostics (DGM)

- [ ] **DGM-01**: Author can write a `graph` diagram with `direction`, quoted `node` labels (with escapes), `container` brace nesting, `a -> b` / `a -> b: "label"` edges, dotted refs, shapes `rect`/`rounded`/`diamond`/`cylinder`/`start`/`end`, self-loops, and `//` comments
- [ ] **DGM-02**: Author can write a `sequence` diagram with actors, ordered messages, dashed returns, and `note over`
- [ ] **DGM-03**: A single check reports every error in a diagram (parser recovery), each with a stable `diagram/*` code, severity, and exact source span
- [ ] **DGM-04**: Diagnostics carry machine-applicable `fixes[]` (`{span, replacement}`) that an agent can apply without re-prompting
- [x] **DGM-05**: Author can canonically format a diagram (idempotent `fmt`)
- [x] **DGM-06**: Oversized or over-nested input and edges targeting a container are rejected with span-precise errors (no silent mis-layout, no hang)

### Rendering (REND)

- [ ] **REND-01**: A `graph` diagram renders to SVG via `dagre` layout behind a one-file adapter, with containers, edge labels, cycles, and self-loops; the 10 largest corpus diagrams pass a go/no-go spike (fallback `dugong`)
- [ ] **REND-02**: SVG output is byte-identical across repeated runs and across macOS and Linux (no `HashMap` order, no transcendental floats, no embedded versions/timestamps)
- [ ] **REND-03**: SVG is safe and accessible: all labels escaped by one writer, generated per-diagram id prefixes, no `<style>`/`foreignObject`/user-derived attributes, `<title>`/`<desc>` from the IR, text as `<text>`; the escape guard is proven by mutation
- [ ] **REND-04**: One inline SVG themes correctly in light and dark mode via CSS variables mapped to Starlight tokens
- [ ] **REND-05**: A `sequence` diagram renders to SVG with width-aware lanes
- [ ] **REND-06**: Any diagram exports to Mermaid that a real (pinned) Mermaid parser accepts (the sole oracle — no dependency on the uncommitted `mermaid.rs` validator)

### Import & Migration (MIG)

- [ ] **MIG-01**: Maintainer can import a Mermaid flowchart/graph/state/sequence block to VoxDiagram; unsupported constructs fail closed with a by-line `unsupported` list (nothing silently dropped)
- [ ] **MIG-02**: Import is verified by node/edge-set equality through an import → Mermaid-export round trip over the corpus
- [ ] **MIG-03**: All convertible ```` ```mermaid ```` fences in `docs/src` (33 fences / 21 files at committed HEAD 2026-10-16, plus any landed before migration; `archive/` excluded and untouched) are migrated to ```` ```vox-diagram ````, reviewed in small batches with a contact sheet
- [ ] **MIG-04**: After migration the legacy-Mermaid baseline is tightened to the allowlist (the Gantt, plus any block MIG-01 cannot convert)

### Docs Pipeline (DOCS)

- [ ] **DOCS-01**: Doc lint rejects any *new* ```` ```mermaid ```` fence in `docs/src` (excluding `archive/`) via a shrink-only ratchet seeded at the fence count present at implementation time (33/21 at committed HEAD 2026-10-16), landed before migration
- [ ] **DOCS-02**: Doc lint validates every ```` ```vox-diagram ```` fence; the `vox` doctest runner never compiles it
- [ ] **DOCS-03**: The docs build pre-renders every ```` ```vox-diagram ```` fence to SVG (Rust pre-render → gitignored manifest) and inlines it via a remark plugin (MDX-safe), failing the build on a missing render
- [ ] **DOCS-04**: Rendered diagrams on voxlang.org pass Playwright checks: no duplicate ids, labels fit their boxes under two font stacks, contrast passes in both themes

### Agent & CLI Surface (TOOL)

- [ ] **TOOL-01**: Agent can call `vox_validate_diagram` and get diagnostics, fixes, `normalized_source`, `stats`, and optional `from_mermaid` conversion; registered through the operations catalog with all generated artifacts and GUI coverage (no new exemption)
- [ ] **TOOL-02**: Developer can run `vox diagram check|fmt|render|export|import` from the CLI
- [ ] **TOOL-03**: Agents receive a short (≤~600-token) VoxDiagram grammar card with examples in their authoring guidance, and AGENTS.md directs new docs diagrams to ```` ```vox-diagram ````

### Measurement (EVAL)

- [ ] **EVAL-01**: A paired NL→diagram harness (≥60 tasks, Wilson CIs, deterministic parser as oracle, models selected via `vox_actor_runtime::llm`, none pinned) reports VoxDiagram vs Mermaid first-pass validity and validity after the bounded repair loop
- [ ] **EVAL-02**: The advisory's unsourced error-rate table is annotated as unsourced and linked to the measured EVAL-01 result

## Future Requirements

- `diagram` declaration in vox-compiler + LSP
- Native Axis GUI rendering (chat, Code Atlas)
- Published JSON Schema for the diagram IR
- ER / class / Gantt kinds; edges to containers via anchor lowering
- Migrating `docs/superpowers/` fences

## Out of Scope

| Feature | Reason |
|---------|--------|
| `docs/src/archive/` migration | Tombstoned (AGENTS.md Archival Protocol) |
| Styling DSL, classDef, extra edge styles/shapes | 0 corpus uses; anti-feature |
| Client-side diagram rendering / committed SVG | Non-deterministic or regenerate-after-merge churn |
| Multiple layout engines, ELK/WASM | Toolchain invariant + licence; `dagre` seam is enough |
| LLM repair inside `vox_validate_diagram` | Tool stays deterministic; repair loop lives in the facade caller |
| Removing `mermaid.rs` / `vox_validate_mermaid` | Agents still emit Mermaid in chat/PR bodies; it's also the emitter oracle |
| Citing the advisory's error-rate numbers | Unsourced and refuted by research |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| DGM-01 | Phase 83 | Pending |
| DGM-02 | Phase 86 | Pending |
| DGM-03 | Phase 83 | Pending |
| DGM-04 | Phase 83 | Pending |
| DGM-05 | Phase 83 | Complete |
| DGM-06 | Phase 83 | Complete |
| REND-01 | Phase 85 | Pending |
| REND-02 | Phase 85 | Pending |
| REND-03 | Phase 85 | Pending |
| REND-04 | Phase 85 | Pending |
| REND-05 | Phase 86 | Pending |
| REND-06 | Phase 84 | Pending |
| MIG-01 | Phase 84 | Pending |
| MIG-02 | Phase 84 | Pending |
| MIG-03 | Phase 90 | Pending |
| MIG-04 | Phase 90 | Pending |
| DOCS-01 | Phase 87 | Pending |
| DOCS-02 | Phase 87 | Pending |
| DOCS-03 | Phase 89 | Pending |
| DOCS-04 | Phase 89 | Pending |
| TOOL-01 | Phase 88 | Pending |
| TOOL-02 | Phase 88 | Pending |
| TOOL-03 | Phase 88 | Pending |
| EVAL-01 | Phase 91 | Pending |
| EVAL-02 | Phase 91 | Pending |

**Coverage:** v1.7 requirements: 25 total — mapped: 25 — unmapped: 0 ✓

Cross-cutting note: DGM-03/04/05 are owned by Phase 83 (graph kind); Phase 86 re-asserts them for the `sequence` kind. REND-06/MIG-01/MIG-02 are owned by Phase 84 (both kinds at IR level via the Phase 83 sequence IR types); Phase 86 re-verifies the sequence round trip through the DSL. Phase 90 re-runs DOCS-04's Playwright suite over the migrated corpus.

---
*Requirements defined: 2026-10-16*
