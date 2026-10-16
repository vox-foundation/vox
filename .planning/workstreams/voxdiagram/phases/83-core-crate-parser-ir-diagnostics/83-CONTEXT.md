# Phase 83: Core Crate - Parser, IR & Diagnostics - Context

**Gathered:** 2026-10-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Ship the `vox-diagram` L2 crate's front half for the `graph` kind: lexer, error-recovering parser, typed AST → resolved IR (serde), stable `diagram/*` diagnostics with machine-applicable fixes, a canonical idempotent formatter, input caps, and the `fence` scanner module — plus all crate registrations (layers.toml L2 row, where-things-live rows, hakari, crate docs, temporary `orphan_exempt`). `sequence` IR *types* only. No layout, SVG, Mermaid emit/import, consumers, CLI or MCP (Phases 84–89).

Requirements: DGM-01, DGM-03, DGM-04, DGM-05, DGM-06.

</domain>

<decisions>
## Implementation Decisions

### DSL surface syntax
- Header: `graph { direction: right ... }`; direction values `down` | `right` | `up` | `left` (plain words, no Mermaid `TD/LR` tokens).
- Node declaration: `node api: "API Gateway"` with optional attribute block `{ shape: cylinder }`. Shapes: `rect` (default), `rounded`, `diamond`, `cylinder`, `start`, `end`.
- Edges: `a -> b` and `a -> b: "label"`; self-loops allowed; `//` line comments. All labels are double-quoted strings with escapes (`\"`, `\\`, `\n`).
- Edge to an undeclared id is an error `diagram/undeclared-node` with a fix inserting `node x: "x"` (no implicit declaration).
- Containers: `container backend: "Orchestrator" { ... }`; ids resolve in the innermost container first, outer scopes after; dotted refs `backend.gateway` address nested ids from outside.

### Diagnostics & fixes
- Flat, stable kebab codes `diagram/<name>` (e.g. `diagram/unterminated-string`, `diagram/undeclared-node`, `diagram/container-endpoint`); codes are never renamed, only added.
- Error recovery re-syncs at statement boundaries (newline or `}`); all independent errors reported; cap 100 diagnostics.
- `fixes[]` (`{span, replacement}`) only on machine-applicable remedies (quote a label, close a brace, declare a node, Mermaid-ism rewrite) — applying them must always be safe.
- Mermaid-isms (`-->`, `A[Label]`, `subgraph … end`, etc.) parse leniently and yield `diagram/mermaid-syntax` with the canonical rewrite as a fix — the primary mitigation for LLMs with no training data on the DSL.

### Limits, IR & formatting
- Caps (each a `diagram/limit-*` error, span-precise): 64 KiB source, 500 nodes, 1,000 edges, container depth 4, label 200 chars. Edge endpoint that is a container → `diagram/container-endpoint`. Adversarial input must terminate in bounded time with no panic.
- IR: serde, `Vec`s in declaration order (no `HashMap`, no sorted maps), spans as byte offsets + line/col; JSON byte-identical across runs.
- `fmt`: 4-space indent, one statement per line, declaration order preserved (never sorted), `//` comments kept attached to their statement; `fmt(fmt(x)) == fmt(x)` and `parse(fmt(x)) == parse(x)` (proptest).
- `sequence` IR types (actors, messages incl. dashed return, `note over`) defined now, no parser (Phase 86).

### Claude's Discretion
- Internal module layout under the 500-line file cap; lexer token set; exact diagnostic message wording; `max_loc` value in layers.toml; proptest generators.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/vox-similarity` — L2 pure crate precedent (`layers.toml` row `layer = 2, max_loc = 6_000`).
- `crates/vox-doc-pipeline/src/pipeline/mermaid.rs` — existing fence-aware `extract_mermaid_blocks`; the new `fence` module generalizes it (consumer swap happens in Phase 87).
- Workspace already has `insta`, `proptest`, `serde`, `unicode-width` (lockfile).

### Established Patterns
- New crate checklist (research/ARCHITECTURE.md): root `Cargo.toml` member, `layers.toml` `[crates]` row, `where-things-live.md` rows (Rule 12 parity), `workspace-hack` dep via `cargo hakari generate`, `orphan_exempt` until first consumer, generated API docs.
- Test-first: every `pub fn` has a test in the same file (tdd-guard pre-commit).
- No direct `blake3`/`sha2` (crypto policy); no `miette` in the library.

### Integration Points
- None yet (no consumer edges this phase). Consumers arrive in Phase 87 (`vox-doc-pipeline`) and 88 (`vox-orchestrator-mcp`), both pre-authorized 2026-10-16.

</code_context>

<specifics>
## Specific Ideas

Canonical example (from the advisory, adapted to the decided syntax):

```text
graph {
    direction: down

    node client: "Axis Desktop Client"

    container backend: "Orchestrator Host" {
        node gateway: "HTTP / WebSocket Gateway"
        node engine: "Agent Actor Runtime"
        node db: "Turso / Codex SQLite" { shape: cylinder }

        gateway -> engine: "Dispatches turns"
        engine -> db: "Queries & Mutations"
    }

    client -> backend.gateway: "TLS / WebSocket"
}
```

Draft a ≤~600-token grammar card with examples (finalized in Phase 88).

</specifics>

<deferred>
## Deferred Ideas

- Edges to containers via hidden-anchor lowering (v2).
- Published JSON Schema for the IR (v2).

</deferred>
