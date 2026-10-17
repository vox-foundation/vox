# Phase 86: Sequence Kind - Context

**Gathered:** 2026-10-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Inside `crates/vox-diagram`: the `sequence` DSL (lexer/parser/resolver/IR via `Ir::Sequence`), `check` and `fmt` parity with `graph` (all errors at once, stable codes, spans, machine-applicable fixes, canonical idempotent form), sequence SVG rendering through the Phase 85 safe writer with width-aware lanes, and the corpus sequence block round trip (import → DSL → re-parse → render → Mermaid export accepted by the real parser). No docs-pipeline/CLI/MCP wiring.

Requirements: DGM-02, REND-05.

</domain>

<decisions>
## Implementation Decisions

### Sequence DSL syntax
- Header `sequence { … }`; actors declared `actor api: "API Gateway"` (same `id: "label"` shape as `node`).
- Using an undeclared actor → `diagram/undeclared-actor` with a declare fix (consistent with graph's `undeclared-node`).
- Messages `a -> b: "request"`; dashed returns use the explicit keyword `reply b -> a: "200 OK"` so `-->` stays a Mermaid-ism error everywhere.
- Notes `note over a: "text"` and `note over a, b: "text"`.
- Self-messages allowed (`a -> a: "retry"`), rendered as a right-side loop.
- Mermaid-isms in sequence context (`->>`, `-->>`, `participant`, `Note over A:` unquoted) get lenient parse + `diagram/mermaid-syntax` with canonical rewrite fixes, mirroring Phase 83's graph behaviour.

### Sequence rendering & corpus
- Actor boxes on top only, dashed lifelines; each gap between adjacent lanes widens to fit the longest message label crossing it (80+ char messages neither overlap nor clip).
- Respect explicit `\n`; soft-wrap message labels only past 60 chars.
- `Ir::Sequence(SequenceIr)`; `check`/`fmt`/`svg::render`/`mermaid::export` dispatch on it; same single safe writer, theme vars, id prefix, title/desc; render caps actors ≤ 100, events ≤ 200 (new constants alongside the graph render caps).
- Corpus: 1 sequence block at committed HEAD — it must import → print DSL → re-parse to the same IR → render → re-export accepted by the pinned real Mermaid parser (ROADMAP SC3 corrected 2026-10-17; no `validate_and_repair_mermaid`).
- Phase 84's `ImportReport.sequence: Option<SequenceIr>` becomes printable DSL: import should now also fill `source` for sequence blocks (keep the `sequence` field for compatibility or remove it if nothing consumes it — planner decides, document it).

### Carried forward
- No HashMap/HashSet in src; byte-deterministic output; no transcendental floats; tests first; files < 400 non-blank lines; no new deps; no crate edges; crate-api-sync --write after pub API change; fail closed.

### Claude's Discretion
- Lane spacing constants, lifeline/arrow geometry, note box placement, code names for new diagnostics, module split.

</decisions>

<code_context>
## Existing Code Insights

- `sequence_ir.rs` (Phase 83 types), `mermaid::export_sequence` + sequence importer (Phase 84), graph lexer/parser/resolver/printer/fixes (Phase 83), `svg/` writer + theme + text sizing (Phase 85), frozen corpus sequence block under `tests/fixtures/corpus/`.

</code_context>

<specifics>
## Specific Ideas

Golden: the corpus sequence block rendered; property: fmt idempotence + parse(fmt(x)) for sequence; hostile labels through the sequence renderer.

</specifics>

<deferred>
## Deferred Ideas

- alt/loop/opt/par blocks, activations, autonumber (none in corpus).

</deferred>
