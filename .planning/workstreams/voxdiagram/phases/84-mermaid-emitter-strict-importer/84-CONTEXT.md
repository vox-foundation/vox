# Phase 84: Mermaid Emitter & Strict Importer - Context

**Gathered:** 2026-10-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Inside `crates/vox-diagram`: a sanitized Mermaid emitter for graph IR (and sequence IR types from Phase 83), and a strict, fail-closed Mermaid → VoxDiagram importer for flowchart/graph, stateDiagram-v2 and sequenceDiagram blocks, verified by import → export round-trip node/edge-set equality over a frozen copy of the live docs corpus, with a pinned real Mermaid parser (Node) as the oracle. No layout/SVG (85), no sequence DSL parser (86), no docs-pipeline/CLI/MCP wiring (87–89), no migration of docs (90).

Requirements: REND-06, MIG-01, MIG-02.

</domain>

<decisions>
## Implementation Decisions

### Mermaid export shape
- Ids: keep the original id when safe (ASCII alnum/underscore, not a Mermaid keyword such as `end`/`graph`/`subgraph`, not starting with `o`/`x` that could merge with an edge); otherwise emit `n<declaration-index>`. Deterministic.
- Containers → `subgraph id ["Label"]` … `end`, nesting preserved; container direction emitted only when set.
- Shapes: rect `["…"]`, rounded `("…")`, diamond `{"…"}`, cylinder `[("…")]`, start `(("…"))`, end `((("…")))`.
- Labels always double-quoted; `"` → `#quot;`, newline → `<br>`, `<` `>` `&` entity-encoded. Edge labels use `-->|"…"|`.

### Importer behaviour
- Mermaid subgraph nodes are global; VoxDiagram containers are scoped → importer rewrites cross-scope references to dotted paths (`backend.gateway`) so semantics are preserved exactly.
- stateDiagram-v2: `[*]` → `start`/`end` nodes, transitions → labelled edges, self-loops kept (folds into `graph`).
- Any unsupported construct fails the whole block closed with a by-line `unsupported` list (`{line, construct}`); nothing is silently dropped.
- Styling (`classDef`, `style`, `linkStyle`, `class`, `:::`, `click`) is unsupported by default; an explicit `drop_styling` import option drops them and lists every dropped line in the report.
- Documented leniencies: unquoted labels, `&` fan-out and chain expansion, `<br>` → `\n`.

### Oracle, API & corpus
- Oracle: exact-pinned `mermaid` devDependency in `docs-astro` (committed lockfile; CI uses `--frozen-lockfile`), exercised by a `node --test` suite; research selects the Node-side parse entry point (jsdom vs `@mermaid-js/parser`) and how to extract node/edge sets independently of the Rust importer.
- API: `vox_diagram::mermaid::export(&Ir) -> String`; `vox_diagram::mermaid::import(&str, ImportOpts) -> ImportReport { source: Option<String> /* canonical fmt output */, unsupported: Vec<Unsupported { line, construct }>, dropped: Vec<…> }`.
- Corpus: freeze copies of every live `docs/src` mermaid fence (archive excluded) at committed HEAD into `crates/vox-diagram/tests/fixtures/corpus/`; tests never read `docs/`. The Gantt block must report unsupported, not be mangled.
- ROADMAP SC1/SC3 updated (2026-10-16): no `validate_and_repair_mermaid` clause; live counts instead of 37/1/2.

### Carried from Phase 83
- Do not depend on the uncommitted `mermaid.rs` validator in the other checkout.
- No HashMap/HashSet in `src/`; byte-deterministic output; tests first (tdd-guard file-level); files < 400 non-blank lines; no new crate edges.
- Known gap from 83-07: an edge from outside a Mermaid subgraph to a node inside it currently becomes `undeclared-node` in the lenient Mermaid-ism path — the importer's dotted-path rewrite resolves this for imports.

### Claude's Discretion
- Internal module split (`mermaid/export.rs`, `mermaid/import/*.rs`), exact keyword list, error/`unsupported` construct naming, fixture file layout.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/vox-diagram`: `check`, `fmt`, `Ir`, `sequence_ir`, `fence::extract`, Mermaid-ism lexer/parser paths (`lexer/mermaid.rs`, `parser/mermaid.rs`, `resolve/mermaid.rs`) that already recognise many Mermaid tokens leniently.
- `docs-astro` already runs `node --test` suites (research to confirm location/pattern).

### Integration Points
- New `docs-astro` devDependency + lockfile change (sentinel path → full CI on PR).

</code_context>

<specifics>
## Specific Ideas

Round-trip equality: original block → oracle sets; import → VoxDiagram → export → oracle sets; compare node-label and edge (src,dst,label) multisets with id-renaming normalization.

</specifics>

<deferred>
## Deferred Ideas

- Migrating `docs/superpowers/` fences; Gantt kind.

</deferred>
