# Phase 84: Mermaid Emitter & Strict Importer - Research

**Researched:** 2026-10-16
**Domain:** Rust Mermaid export/import inside `crates/vox-diagram` (L2), verified against a pinned real Mermaid parser run from Node in `docs-astro`
**Confidence:** HIGH (every encoding rule, oracle entry point and corpus fact below was executed this session against `mermaid@11.17.2` and `mermaid@12.2.0`; the MEDIUM items are the open decisions in Open Questions)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Mermaid export shape**
- Ids: keep the original id when safe (ASCII alnum/underscore, not a Mermaid keyword such as `end`/`graph`/`subgraph`, not starting with `o`/`x` that could merge with an edge); otherwise emit `n<declaration-index>`. Deterministic.
- Containers → `subgraph id ["Label"]` … `end`, nesting preserved; container direction emitted only when set.
- Shapes: rect `["…"]`, rounded `("…")`, diamond `{"…"}`, cylinder `[("…")]`, start `(("…"))`, end `((("…")))`.
- Labels always double-quoted; `"` → `#quot;`, newline → `<br>`, `<` `>` `&` entity-encoded. Edge labels use `-->|"…"|`.

**Importer behaviour**
- Mermaid subgraph nodes are global; VoxDiagram containers are scoped → importer rewrites cross-scope references to dotted paths (`backend.gateway`) so semantics are preserved exactly.
- stateDiagram-v2: `[*]` → `start`/`end` nodes, transitions → labelled edges, self-loops kept (folds into `graph`).
- Any unsupported construct fails the whole block closed with a by-line `unsupported` list (`{line, construct}`); nothing is silently dropped.
- Styling (`classDef`, `style`, `linkStyle`, `class`, `:::`, `click`) is unsupported by default; an explicit `drop_styling` import option drops them and lists every dropped line in the report.
- Documented leniencies: unquoted labels, `&` fan-out and chain expansion, `<br>` → `\n`.

**Oracle, API & corpus**
- Oracle: exact-pinned `mermaid` devDependency in `docs-astro` (committed lockfile; CI uses `--frozen-lockfile`), exercised by a `node --test` suite; research selects the Node-side parse entry point (jsdom vs `@mermaid-js/parser`) and how to extract node/edge sets independently of the Rust importer.
- API: `vox_diagram::mermaid::export(&Ir) -> String`; `vox_diagram::mermaid::import(&str, ImportOpts) -> ImportReport { source: Option<String> /* canonical fmt output */, unsupported: Vec<Unsupported { line, construct }>, dropped: Vec<…> }`.
- Corpus: freeze copies of every live `docs/src` mermaid fence (archive excluded) at committed HEAD into `crates/vox-diagram/tests/fixtures/corpus/`; tests never read `docs/`. The Gantt block must report unsupported, not be mangled.
- ROADMAP SC1/SC3 updated (2026-10-16): no `validate_and_repair_mermaid` clause; live counts instead of 37/1/2.

**Carried from Phase 83**
- Do not depend on the uncommitted `mermaid.rs` validator in the other checkout.
- No HashMap/HashSet in `src/`; byte-deterministic output; tests first (tdd-guard file-level); files < 400 non-blank lines; no new crate edges.
- Known gap from 83-07: an edge from outside a Mermaid subgraph to a node inside it currently becomes `undeclared-node` in the lenient Mermaid-ism path — the importer's dotted-path rewrite resolves this for imports.

### Claude's Discretion
- Internal module split (`mermaid/export.rs`, `mermaid/import/*.rs`), exact keyword list, error/`unsupported` construct naming, fixture file layout.

### Deferred Ideas (OUT OF SCOPE)
- Migrating `docs/superpowers/` fences; Gantt kind.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REND-06 | Any diagram exports to Mermaid that a real (pinned) Mermaid parser accepts (the sole oracle — no dependency on the uncommitted `mermaid.rs` validator) | Pinned oracle + Node entry point (Standard Stack, Code Example 1); the empirically derived encoding rules that made 20,000 hostile graph cases and 6,000 hostile sequence cases parse and round-trip (Exporter Rules); fixture contract so the Node test sees real Rust output (Architecture) |
| MIG-01 | Import a Mermaid flowchart/graph/state/sequence block; unsupported constructs fail closed with a by-line `unsupported` list | Supported/unsupported subset tables, id/label strictness rules measured against the real parser, corpus census (Importer Rules, Corpus Inventory) |
| MIG-02 | Import verified by node/edge-set equality through an import → Mermaid-export round trip over the corpus | Canonical-set definition proven on 58 corpus round trips, three-way equality design, the two corpus blocks that cannot take part (Corpus Inventory, Round-trip Design) |
</phase_requirements>

## Summary

Run the oracle in plain Node with a **jsdom shim loaded before `mermaid` is imported**; without it `mermaid.parse()` throws `TypeError: DOMPurify.addHook is not a function`. `@mermaid-js/parser` is not an option (it is the Langium parser for the newer diagram types; flowchart, state and sequence still use Mermaid's own jison parsers inside the `mermaid` package). Acceptance is the public `mermaid.parse()`; node/edge sets come from the diagram DB via `mermaid.mermaidAPI.getDiagramFromText(src)` → `db.getVertices()/getEdges()/getSubGraphs()` (flowchart), `db.getStates()/getRelations()` (state), `db.getActors()/getMessages()` (sequence). That path is semi-internal, so the exact pin is the guard. Pin **`mermaid` 11.17.2** (the line the site's `mermaid@11` CDN loader resolves to, above the 11.16.1 CVE fix) and **`jsdom` 30.0.1**; both behave identically to `mermaid@12.2.0` on every probe below.

The Rust side needs **no new dependencies and no new crate edges**. Recommended shape: `mermaid::export`/`export_sequence` driven by one shared escaping module; a standalone line-oriented `mermaid::import` (the Phase 83 Mermaid-ism lexer/parser is DSL-lexer-state-bound and loses the information the importer must report, so reuse only `check`/`fmt`, `limits`, `RESERVED`, `Direction`, `Shape`, `SequenceIr`). Rust and Node connect through **committed fixture files**, never through a subprocess: Rust golden tests write/compare `*.export.mmd`, `*.sets.json` and hostile exports under `crates/vox-diagram/tests/fixtures/`; `docs-astro/tests/unit/mermaid-oracle.test.mjs` reads them (the repo already reads repo-root files from these tests via `REPO_ROOT`). The hosted job that runs `node --test "tests/unit/*.test.mjs"` is `docs-quality.yml`, which does **not** currently trigger on `crates/vox-diagram/**`; add the fixture path to both its trigger lists.

Four findings change the plan versus the locked context and roadmap and need a decision (see Open Questions): (1) corpus block `how-to-rust-crate-imports.md:53` is **rejected by the real Mermaid parser** (a quoted label that starts with a backtick), so MIG-02 cannot hold for it and ROADMAP SC3 ("Gantt is the only exception") is short by one block; (2) `IrContainer` has **no direction field**, so "container direction emitted only when set" is vacuous and the importer must decide what to do with the two `direction TB` lines inside subgraphs in the corpus; (3) Phase 84's sequence import has **no DSL to print to** (Phase 86 owns the sequence DSL and `Ir::Sequence`), so `ImportReport` needs a place for a `SequenceIr`; (4) the live HEAD corpus has **one** `sequenceDiagram`, not two.

**Primary recommendation:** Build the sanitizer (`mermaid/escape.rs`) and the Node oracle harness first and prove them against each other on a committed hostile fixture set before writing any importer code; then write the importer test-first against the frozen corpus.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Mermaid export (escaping, id policy, subgraph nesting) | Rust crate `vox-diagram` (L2, pure) | — | Pure string-in/string-out; no I/O; deterministic |
| Strict Mermaid import + `unsupported` report | Rust crate `vox-diagram` (L2, pure) | — | Same crate as the IR/`check`/`fmt` it feeds; Phase 88 `from_mermaid` and the CLI reuse it |
| Real-parser acceptance + independent node/edge sets | Node test tier (`docs-astro` `node --test`, CI only) | — | Only a JS runtime can run the real Mermaid jison parsers; never shipped, never called from Rust |
| Corpus freeze | Rust crate test fixtures | — | `tests` must not read `docs/`; frozen copies are the contract |
| Rust ↔ Node hand-off | Committed fixture files | CI path filter | Keeps Rust shards free of Node; the Rust golden test is the hard gate, the Node job is the oracle |
| CI hosting | `docs-quality.yml` (existing `node --test` step) | `ci.yml` nextest shards (Rust half) | The only workflow that installs `docs-astro` deps and runs `tests/unit` |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `mermaid` (npm, devDependency of `docs-astro`) | **11.17.2** exact, published 2026-08-25 [VERIFIED: npm registry `time`] | Sole oracle: `mermaid.parse()` acceptance + diagram DB for sets | The real parser; 11.17.2 is the newest 11.x (what the site's `mermaid@11` CDN import resolves to); ≥11.16.1 (CVE-2026-50159 fix) [CITED: mend.io/vulnerability-database/CVE-2026-50159]; engines not set on 11.x, 12.x needs Node ≥22.12 |
| `jsdom` (npm, devDependency) | **30.0.1** exact, published 2026-07-29 [VERIFIED: npm registry `time`] | Provides `window`/`document` so Mermaid's DOMPurify initialises under Node | Standard DOM shim; 30.x engines `^22.22.2 \|\| ^24.15.0 \|\| >=26.0.0`; CI runs Node 24 (latest 24.22 on the registry) [VERIFIED: npm registry]. Deliberately NOT 30.1.3 (published 2026-10-15, 1 day old) |
| `node:test` + `node:assert/strict` | Node 24 built-in | Test runner | Existing convention in `docs-astro/tests/unit/*.test.mjs` |
| Rust: none new | — | — | `serde` (already a dependency) gives `Serialize` for `ImportReport`; `serde_json` is already a dev-dependency for fixture JSON [VERIFIED: crates/vox-diagram/Cargo.toml] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `insta` 1.48.0 (existing dev-dep) | — | Reviewed snapshots for importer reports, catalog rows | Rust-only assertions. **Not** for the files Node reads: `assert_binary_snapshot!` is experimental and writes metadata sidecars [VERIFIED: insta-1.48.0 src/macros.rs:390-431] |
| `proptest` (existing dev-dep) | — | Arbitrary-IR sanitizer invariants; `import(export(ir)) ≅ ir` | Rust-only structural properties |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `mermaid` 11.17.2 | `mermaid` 12.2.0 (published 2026-10-13, node ≥22.12) | Identical results on every probe here (hostile labels, ids, corpus, 20k fuzz). New major 3 weeks old and not what the site loads; take it only if Phase 89 moves the site loader to 12 |
| `@mermaid-js/parser` | — | Cannot parse flowchart/state/sequence; verified by the oracle working only via the `mermaid` package; listed `@mermaid-js/parser` 2.0.1 is a transitive dep of `mermaid` already [VERIFIED: `pnpm licenses`] |
| jsdom shim | Stub `dompurify` through a Node module hook | Saves ~35 packages but sanitises nothing, so label text would no longer be what Mermaid really produces (e.g. `<br/>` normalisation, entity serialisation). Keep the real DOMPurify |
| Hand-written Rust importer | `merman-core` | Rejected in STACK.md (~20 new crates); unchanged |
| Rust test shells out to Node | — | Rust shards in `ci.yml` have no `pnpm install`; the committed-fixture hand-off avoids it |

**Installation (from `docs-astro/`):**
```bash
pnpm add --save-exact -D mermaid@11.17.2 jsdom@30.0.1
```
Verified in a scratch copy of `docs-astro` (`package.json`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`, `patches/`) with the CI pnpm major (11.28.6): the lockfile diff is **+1150 lines, 0 deletions** (purely additive, no existing resolution moves), `pnpm install --frozen-lockfile` passes, and there is no `ERR_PNPM_IGNORED_BUILDS` (no dependency has a build script, so the existing `allowBuilds` list in `pnpm-workspace.yaml` needs no change) [VERIFIED: scratch run].

**Version verification:** `npm view mermaid dist-tags` → `latest: 12.2.0`, `backport: 10.9.8`; `npm view mermaid time` → 11.16.1 = 2026-08-04, 11.17.2 = 2026-08-25, 12.0.0 = 2026-09-10 [VERIFIED: npm registry, 2026-10-16].

## Package Legitimacy Audit

`gsd_run query package-legitimacy check --ecosystem npm mermaid jsdom` returned:

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| mermaid | npm | multi-year; chosen pin is 7 weeks old (seam flagged the *latest* tag, 12.2.0, 3 days old) | ~18.8M/wk | github.com/mermaid-js/mermaid | SUS (`too-new`, from the latest tag's publish date) | Flagged — planner adds a `checkpoint:human-verify` before the install task. The pinned 11.17.2 is the version already loaded by the live site's CDN import and the repo's own research cites its CVE line |
| jsdom | npm | multi-year; chosen pin is 11 weeks old (latest 30.1.3 is 1 day old) | ~114M/wk | github.com/jsdom/jsdom | SUS (`too-new`, same reason) | Flagged — same checkpoint. Pin an older patch (30.0.1), not the latest |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** `mermaid`, `jsdom` (reason `too-new` on the latest tag only; `postinstall: null` for both). Planner inserts one `checkpoint:human-verify` before the `pnpm add` task.

Both names were first seen in repo-owned research (`STACK.md`, `PITFALLS.md` name `mermaid` as the oracle; mermaid's own docs use jsdom for Node) and `pnpm licenses` over the resolved tree shows only MIT/ISC/BSD/Apache-2.0/MPL-2.0 (dompurify is `(MPL-2.0 OR Apache-2.0)`)/MIT-0/CC0/Unlicense/BlueOak-1.0.0 — there is **no npm licence gate** in this repo (`deny.toml` is cargo-only) [VERIFIED: scratch `pnpm licenses list`, `deny.toml`].

## Architecture Patterns

### System Architecture Diagram

```
 ┌──────────── Rust (hosted `tests` shards, no Node) ─────────────┐        ┌──── Node (docs-quality job, after `pnpm build`) ────┐
 │                                                                  │        │                                                      │
 │ tests/fixtures/corpus/NN-*.mmd  (33 frozen blocks)               │        │ tests/lib/mermaid-oracle.mjs                         │
 │        │                                                         │        │   jsdom shim → import('mermaid')                     │
 │        ▼                                                         │        │   accepts(src)  = mermaid.parse                      │
 │ mermaid::import(src, opts) ──► ImportReport                      │        │   canon(src)    = getDiagramFromText → db →          │
 │        │  unsupported[] ──► (gantt, backtick block)              │        │                   decode+normalise → canonical sets  │
 │        │  source (DSL)  ──► check().ir ──┐                       │        │                                                      │
 │        │  sequence IR ───────────────────┤                       │        │ tests/unit/mermaid-oracle.test.mjs                   │
 │        ▼                                 ▼                       │        │   for each corpus block:                             │
 │ mermaid::export / export_sequence ──► NN-*.export.mmd  ─ golden ─┼─files─►│     canon(original) == sets.json   (importer direct) │
 │ canonical_sets(ir) ───────────────────► NN-*.sets.json ─ golden ─┼─files─►│     canon(export)   == sets.json   (exporter valid)  │
 │                                                                  │        │     canon(original) == canon(export)  (MIG-02)       │
 │ hostile IR JSON ─► export ─► hostile/*.mmd, *.sets.json ─ golden ┼─files─►│   hostile: accepts + sets equal (REND-06)            │
 │ proptest: arbitrary IR ─► sanitizer invariants (no Node)         │        │   expectation lists (gantt, rejected block) asserted │
 └──────────────────────────────────────────────────────────────────┘        └──────────────────────────────────────────────────────┘
   golden compare/bless: UPDATE_GOLDEN=1 (repo convention)           CI trigger: add crates/vox-diagram/tests/fixtures/** to docs-quality.yml
```

### Recommended Project Structure
```
crates/vox-diagram/src/mermaid/
├── mod.rs            # pub use; ImportOpts, ImportReport, Unsupported, Dropped (+ unit tests in tests.rs)
├── escape.rs         # the ONLY place that encodes labels/ids/text for Mermaid (graph + sequence)
├── export.rs         # export(&Ir) -> String (graph)
├── export_seq.rs     # export_sequence(&SequenceIr) -> String   (NOT named *ir.rs, see Pitfall 12)
└── import/
    ├── mod.rs        # dispatch on header, collect Unsupported, run check+fmt on the result
    ├── lines.rs      # line scanner: header, subgraph/end, edges, node forms, label rules
    ├── flow.rs       # flowchart/graph model → DSL text (global ids → scoped dotted paths)
    ├── state.rs      # stateDiagram-v2 → graph model
    └── seq.rs        # sequenceDiagram → SequenceIr
crates/vox-diagram/tests/
├── mermaid_export.rs         # sanitizer invariants, hostile goldens, proptest
├── mermaid_import.rs         # catalogue: every leniency, every unsupported construct, line numbers
├── mermaid_corpus.rs         # freeze check + round-trip goldens
├── freeze_corpus.rs          # #[ignore] one-shot freezer using fence::extract
└── fixtures/{corpus,corpus-roundtrip,export-hostile}/
docs-astro/tests/lib/mermaid-oracle.mjs
docs-astro/tests/unit/mermaid-oracle.test.mjs
```
Follow the Phase 83 pattern for test placement (`mod.rs` + sibling `tests.rs`, e.g. `lexer/tests.rs`) so the file-level tdd-guard and the 400-non-blank-line cap both hold.

### Pattern 1: Committed-fixture hand-off (Rust writes, Node reads)
**What:** Rust golden tests compare generated output to committed files (`UPDATE_GOLDEN=1 cargo nextest run -p vox-diagram …` rewrites them — the repo's existing convention in `vox-kb-facets` and `vox-cli-ci`). Node reads the same files from `REPO_ROOT/crates/vox-diagram/tests/fixtures/…`.
**When to use:** always for this phase; never spawn Node from a Rust test or cargo from a Node test.
**Why not insta binary snapshots:** experimental API with sidecar metadata; plain files are what Node needs.

### Pattern 2: Three-way equality (what makes the oracle "independent")
For each corpus block `B`: `C(B)` = canonical sets from the **real parser** on the original; `R(B)` = canonical sets computed **in Rust from the imported IR** and written to `NN.sets.json`; `C(E)` = canonical sets from the real parser on the Rust-exported text. Node asserts `C(B) == R(B)` (importer correct, no exporter involved) and `C(E) == C(B)` (exporter valid and lossless; this is MIG-02 verbatim). Without `R(B)`, an importer bug and a mirror-image exporter bug cancel out.

### Pattern 3: Canonical set definition (proven on 58 corpus round trips)
```
node key  = JSON [decodedLabel, shape, containerTitlePath[]]     shape ∈ rect|rounded|diamond|cylinder|start|end
edge      = JSON [nodeKey(from), nodeKey(to), decodedEdgeLabel]
sets      = { direction, nodes: sorted(nodeKeys), edges: sorted(edges), containers: sorted(titlePaths) }
```
Compare **sorted multisets**, not id-indexed lists: ids are renamed by the exporter, and the corpus contains three nodes with identical label (block 12 `Write Notes to Virtual Filesystem` ×3), which the multiset form handles. Container membership is part of the key, so a wrong dotted-path rewrite fails the test. Mermaid shapes map `square→rect, round→rounded, diamond, cylinder, circle→start, doublecircle→end`; implicit nodes (no bracket) have `shape` undefined and are `rect` with label = id. Container title path comes from walking `getSubGraphs()` (a subgraph lists nested subgraph ids among its `nodes`). For sequence diagrams compare **ordered** event lists (order is semantic) with actors keyed by index.

### Anti-Patterns to Avoid
- **Id-indexed set comparison for flowcharts:** Mermaid's vertex order depends on emission order (nodes declared inside `subgraph` blocks move), so positional comparison fails for a correct export.
- **Treating `mermaid.parse()` success as set equality:** parse acceptance (REND-06) and set equality (MIG-02) are separate assertions.
- **Computing `R(B)` by running the Node canonicaliser logic through Rust-side string tricks:** write the Rust canonicaliser independently from the IR; that independence is the point.
- **A Rust test that reads `docs/`:** forbidden by the context; only the `#[ignore]` freezer may, and it is run by hand once.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Markdown fence extraction for the freezer | A line scanner | `vox_diagram::fence::extract(md, "mermaid")` [VERIFIED: src/fence.rs, `pub fn extract(md: &str, info: &str) -> Vec<Fence>`] | Built on `pulldown-cmark`; the same scanner Phases 87/90 will use, so the frozen set equals what the gate sees |
| DSL string escaping (`\\`, `\"`, `\n`) in generated source | A second escaper | Make `print::label`'s logic `pub(crate)` or build an `ast::Document` and call `print::to_source` [VERIFIED: print.rs:59-70, `pub(crate) fn to_source`] | One canonical spelling; `fmt()` then validates it |
| Validating the importer's output | Ad hoc checks | `fmt(&raw)` (returns `None` iff `check` has an Error, print.rs:15-20) and `check(&src).ir` | Guarantees `ImportReport.source` is canonical and valid, and gives the IR for the exporter |
| HTML entity decoding in the oracle helper | A full HTML decoder | The 8-line decoder in Code Example 1 (fails loudly on an unknown entity) | The set of entities Mermaid/DOMPurify can emit here is closed: `quot lt gt amp apos nbsp`, `&#N;`, `&#xH;` |
| Node-side Mermaid parsing | A regex flowchart reader | `mermaid.mermaidAPI.getDiagramFromText` | The point of an oracle is independence from our parsing assumptions |

**Key insight:** Mermaid mutates text before and after lexing (entity placeholders, `style…:#…;` semicolon stripping, `wrap:` prefix stripping, `%%{` directives, DOMPurify serialisation). Every one of those bit a naive emitter in testing; the rules below are the closed list.

## Corpus Inventory (committed HEAD `7e8ccdbcc`, `docs/src`, `archive/` excluded)

**33 fences / 21 files** (matches REQUIREMENTS.md; `git grep` of `^\s*(```|~~~)\s*mermaid` and a fence-aware scan agree). The context's "live counts" and STATE.md's older "41/26, 2 sequence" differ: HEAD has **30 flowchart/graph, 1 stateDiagram-v2, 1 sequenceDiagram, 1 gantt** [VERIFIED: scan + header tally: `flowchart LR` 13, `flowchart TD` 11, `graph TD` 5, `flowchart TB` 1, `gantt` 1, `sequenceDiagram` 1, `stateDiagram-v2` 1]. Phase 86's SC3 says "2 corpus sequence blocks": at HEAD there is one.

Index = proposed fixture prefix (sorted by path then line). `line` = 1-based opening-fence line.

| # | file (under docs/src) | line | type | notes |
|---|------|------|------|-------|
| 00 | adr/012-internal-web-ir-strategy.md | 262 | flowchart LR | unquoted labels |
| 01 | architecture/ai-first-fixtures-research-2026.md | 147 | flowchart TD | quoted labels |
| 02 | architecture/as-any-casts-reduction-strategy-research-2026.md | 116 | graph TD | unquoted labels with `&` and `<br />`; `1.`-prefixed |
| 03 | architecture/axis-harness-river-design-2026.md | 83 | flowchart LR | 4 subgraphs, 11 edges all cross-scope, 7 `<br>` labels |
| 04 | architecture/deep-research-full-surface-audit-and-roadmap-2026.md | 46 | flowchart TD | `{}` diamond, `-- Hit -->` edge text |
| 05 | …same | 217 | flowchart LR | 1 subgraph; unquoted label containing `->`; `decide()` |
| 06 | …same | 238 | flowchart TD | 2 subgraphs, `direction TB` inside each, 4 cross-scope edges, implicit node `UserQuery` |
| 07 | …same | 340 | flowchart TD | `A & B & C --> D` fan-in |
| 08 | …same | 368 | flowchart TD | two `&` fan-in lines |
| 09 | …same | 407 | flowchart LR | 3 subgraphs, 3 cross-scope edges |
| 10 | …same | 485 | **gantt** | must report unsupported |
| 11 | architecture/deep-research-waves-and-competitive-analysis-research-2026.md | 45 | flowchart TD | unquoted edge text `Yes (Iter < Max)` (oracle returns `&lt;`) |
| 12 | …same | 76 | flowchart TD | `&` fan-in; 3 identical labels |
| 13 | …same | 110 | flowchart TD | 1 subgraph, 3 cross-scope edges, quoted `-- "Tier 0: …" -->`, node first mentioned outside before its subgraph |
| 14 | …same | 211 | flowchart LR | |
| 15 | architecture/gamification-ludus-review-findings-2026.md | 128 | graph TD | |
| 16 | architecture/graphify-python-free-findings-2026.md | 57 | graph TD | |
| 17 | architecture/mcp-vox-language-exposure.md | 21 | flowchart LR | quoted label with `@` |
| 18 | architecture/scientia-automated-research-historical-extension-research-2026.md | 50 | flowchart TB | 2 subgraphs, all 7 edges cross-scope, unquoted titles `subgraph eras [Historical eras]` |
| 19 | …same | 112 | **sequenceDiagram** | `participant X as Y` ×8, `->>` ×9, `Note over RS,SC:` |
| 20 | architecture/tauri-convergence-migration-plan-2026.md | 117 | flowchart LR | |
| 21 | architecture/unified-workstream-execution-plan-2026.md | 45 | flowchart TD | quoted labels with `->`, `<`, `&` (`920 -> <100`) |
| 22 | architecture/vox-compiler-architecture-research-2026.md | 19 | flowchart LR | 3 subgraphs, **chains** `Source --> lex --> parse --> ast`, 4 cross-scope edges |
| 23 | architecture/vox-gui-browser-support-2026.md | 33 | flowchart LR | chain |
| 24 | …same | 50 | flowchart LR | three chains |
| 25 | …same | 92 | flowchart LR | chain |
| 26 | architecture/vox-react-backend-interop-audit-2026.md | 51 | flowchart LR | |
| 27 | ci/deploy-contract.md | 14 | graph TD | `-->\|text\|` form, `Poll & verify` |
| 28 | explanation/expl-durable-execution.md | 35 | graph TD | `{}` diamond, `-- No -->` |
| 29 | how-to/how-to-rust-crate-imports.md | 53 | flowchart LR | **rejected by the real parser** (`A["\`.vox\` source"]`) |
| 30 | reference/mens-external-tech-options.md | 280 | flowchart TD | |
| 31 | reference/mens-measurement-gap-analysis.md | 284 | flowchart TD | |
| 32 | reference/populi.md | 211 | **stateDiagram-v2** | `[*]`, self-loop `Leased --> Leased`, `lease expiry (swept)` |

**Construct census over the 29 oracle-accepted flowcharts** [VERIFIED: oracle run + line scan]: 249 nodes, 237 edges (all `arrow_point`/`normal`: **no `---`, `-.->`, `==>`**), 16 subgraphs (all depth 1, none nested, none empty), shapes only rect (239), diamond (9) and one implicit node; 23 labelled edges (forms `-- t -->` ≈20 lines, `-->|t|` 3 lines); `&` fan-in on 4 lines; chains on 7 lines; 12 labels containing `<br>` (11 lines); 2 `direction TB` lines inside subgraphs; 6 blocks have cross-scope edges (32 edges total: 03, 06, 09, 13, 18, 22). **Zero** `classDef`/`style`/`linkStyle`/`class`/`:::`/`click`, zero `%%` comments, zero `;`, zero cylinder/start/end shapes, zero edges whose endpoint is a subgraph id, zero ids outside the DSL id charset or equal to a DSL reserved word, zero id shared between a node and a subgraph. Longest label 86 characters (cap is 200); largest block 18 nodes.

**Expected importer outcomes:** 31 blocks import (29 flowchart + 1 state → graph source; 1 sequence → `SequenceIr`); **2 report `unsupported`**: block 10 (gantt: header) and block 29 (markdown-string label the real parser rejects, see Open Question 1). No other block should hit `unsupported`; if one does, the importer is stricter than the corpus allows.

## Importer Rules (measured against the real parser)

Standalone, line-oriented, O(n); never abort at the first problem — collect every unsupported line, then fail the block (`source: None`) if any exist. Line numbers are 1-based **within the block body** (line 1 = header); callers add the fence offset.

| Mermaid input | Rule | Evidence |
|---|---|---|
| Headers `graph|flowchart` + `TD|TB|LR|RL|BT` | Direction map `TD,TB→down, LR→right, RL→left, BT→up`. Other headers (`flowchart-elk`, `gantt`, `classDiagram`…) → one `unsupported` on line 1 | oracle direction values |
| `subgraph id [Title]` / `subgraph id["Title"]` / `subgraph id` | Container; no title → label = id; `subgraph "Title"` (no id) → unsupported | stats over corpus |
| `direction X` inside a subgraph | If equal to the diagram direction (after `TD≡TB`) → accept, list in `dropped` (`subgraph-direction`); else `unsupported`. IR has no container direction | corpus 06:3,12; `IrContainer` has no such field |
| Node forms | `id`, `id[…]` rect, `id(…)` rounded, `id{…}` diamond, `id[(…)]` cylinder, `id((…))` start, `id(((…)))` end. Others (`([…])`, `[[…]]`, `{{…}}`, `>…]`, `[/…/]`, `@{…}`) → `unsupported` (`shape`) | oracle shape names: square/round/diamond/cylinder/circle/doublecircle |
| Unquoted label text | Accept iff it contains none of `( ) [ ] { } \| " @` and no tag-like `<` (`<` followed by an ASCII letter, `/`, `!`, `?`). Everything else in the set `& ' \` % ; : # \ / . , ! ? = + * ~ $ ^ ->  -->  %%` and non-ASCII is literal | `[x?y]`,`(x?y)`,`{x?y}` probe over 38 characters: only `( ) [ ] { } \| " @` raise a parse error; `<` swallows the rest of the text |
| Quoted label `"…"` | Anything except `"`; `#quot; #lt; #gt; #amp; #NN;` decode; a label that **starts** with a backtick is Mermaid's markdown-string opener → `unsupported` (`markdown-string`) | oracle: `A["\`.vox\` source"]` rejected on 11.17.2 and 12.2.0, while `["AST \`X\`"]` is fine |
| `<br>`, `<br/>`, `<br />` | → `\n`, spaces around kept verbatim (lossless; oracle keeps them too) | corpus 02, 03 |
| Edge arrow | Only `-->`. `---`, `-.->`, `==>`, `--o`, `--x`, `<-->`, `~~~` → `unsupported` (`edge-type:<tok>`) | corpus uses only `-->` |
| Edge text | `-->\|text\|`, `-- text -->`, `-- "text" -->`; text decoded like labels | corpus 04, 13, 27 |
| `A & B --> C & D`, `A --> B --> C` | Expand to explicit edges (documented leniency). Cap expansion (83 used 100/stmt) → `unsupported` beyond it | corpus 07, 08, 12, 22–25 |
| `style classDef class linkStyle click callback accTitle accDescr :::` | `unsupported` per line; with `drop_styling` → listed in `dropped`, block still imports [VERIFIED: lexer/mermaid.rs:10-19 `HEADS`] | CONTEXT |
| `%%` comment, trailing `;`, `\n`-less blank | `%%` and `;` → `unsupported` by default (no corpus use; strict). Blank lines ignored | discretion, see Open Question 5 |
| Re-declaration of a node with a different label/shape | `unsupported` (`redeclared-node`) — Mermaid is last-wins, so silently importing would pick one | FEATURES.md notes last-wins |
| Edge endpoint that is a subgraph id; node id equal to a subgraph id | `unsupported` (`edge-to-subgraph`, `id-clash`); DSL rejects container endpoints | none in corpus |
| Ids outside `[A-Za-z_][A-Za-z0-9_-]*` or in the DSL `RESERVED` list | `unsupported` (`invalid-id`/`reserved-id`) — corpus has none; do not rename silently [VERIFIED: resolve.rs:322-329 `["graph","sequence","direction","node","container","shape"]`] | |
| State: `stateDiagram-v2`, `A --> B`, `A --> B: text`, `[*] --> A`, `A --> [*]` | `[*]` source → one `start` node, `[*]` target → one `end` node (label text `Start`/`End`, ids `start`/`end`); self-loops kept. `state "x" as y`, `A : desc`, composites, `<<fork>>`, notes, `--`, `direction` → `unsupported` | corpus 32; oracle gives `root_start`/`root_end` |
| Sequence: `sequenceDiagram`, `participant X [as Label]`, `X->>Y: t`, `X-->>Y: t`, `Note over X[,Y]: t`; implicit participants | → `SequenceIr`. `actor`, other arrows (`->`, `-->`, `-x`, `-)`), `+`/`-` activation, `activate`, `loop/alt/opt/par/critical/break/rect/end`, `autonumber`, `title`, `box`, `create/destroy`, `Note left/right of`, `links` → `unsupported` | corpus 19 |
| Over-size input | Reuse `limits::MAX_SOURCE_BYTES` gate before scanning; labels > `MAX_LABEL_CHARS` (200) / nodes > 500 / edges > 1000 / depth > 4 → `unsupported` with the Mermaid line | limits.rs:4-14 |

**Scope rewrite (the 83-07 known gap):** assign each node to the **first-closed subgraph whose body mentions it** — this is exactly Mermaid's `makeUniq` behaviour (inner subgraphs close first and win; a node mentioned in a later sibling is removed from it) [VERIFIED: oracle: `crosssg` probe gave `s=[q]`, `t=[]`]. Mentions include edge endpoints, not only declarations (corpus 13: `Wave1` is first used outside, then inside `MultiWaveEngine`). Write each node's DSL declaration inside its container; write each edge in the **deepest container holding both endpoints**, endpoints as paths relative to that scope (bare when directly in scope, `child.node` otherwise); this is also valid with every edge at root using full paths. Edge placement is a readability choice (Open Question 6); both give identical IR and identical sets.

## Exporter Rules (all executed against the real parser)

**Total over arbitrary `Ir` values.** `Ir`/`SequenceIr` derive `Deserialize`, and sequence IR has no parser until Phase 86, so the exporter must tolerate IRs that `check()` would never produce (empty labels, control characters, dangling edge endpoints). `export` returns `String`, so dangling endpoints can only be emitted as implicit Mermaid nodes; document it.

### Output grammar (one statement per line; this is the injection guard)
```
flowchart TD                              # Down→TD (corpus majority), Right→LR, Up→BT, Left→RL
  n0["Label"]  /  n1("Label")  /  n2{"Label"}  /  n3[("Label")]  /  n4(("Label"))  /  n5((("Label")))
  subgraph c0 ["Title"]
    …nodes and nested subgraphs…
  end
  a --> b            or    a -->|"edge label"| b
```
Emit declarations (nodes top-level first, then containers recursively, nodes before child containers) and **all edges last**. A property test must assert: number of output lines == 1 header + nodes + 2×containers + edges for any IR (a label can never smuggle a second statement).

### Id policy (locked decision, plus the traps found)
Keep `local_id` iff ALL hold, else `n<declaration-index>` (index over nodes then containers, deterministic):
- matches `[A-Za-z_][A-Za-z0-9_]*` (ASCII; hyphen is excluded by the lock although Mermaid accepts it);
- `to_ascii_lowercase()` is not in the keyword set **`end graph flowchart subgraph style class classdef linkstyle click default interpolate call href direction`** — measured: `end graph flowchart subgraph style class classDef linkStyle` fail everywhere; `default` and `interpolate` fail as subgraph ids; **`click` as a subgraph id is silently dropped** (parse succeeds, the subgraph vanishes); `call`/`href` fail in edges (`direction` accepted but excluded for safety). Upper/mixed case (`End`, `END`) is accepted, so lowercase comparison is a safe superset;
- does not start with `o`/`x` (locked). Measured: with edges always written `a --> b` (spaces) `o`, `x`, `ox`, `xo`, `oa`, `xa` all parse, so the lock only costs readability; relaxing it needs a user decision;
- does not match `^n[0-9]+$` (it could collide with a generated fallback) and is not already used by an earlier node **or container** (Mermaid ids are one global namespace across nodes and subgraphs; scoped DSL ids repeat across containers, so use `local_id` only if globally unique);
- does not contain the substring `style` or `classDef` (see colon guard below; cheaper to rename than to reason about).

### Text encoding (graph labels, subgraph titles, edge labels)
Applied per character; all of these were required by a failing case in testing:

| Input | Output | Why (measured) |
|---|---|---|
| `&` `"` `<` `>` | `#amp;` `#quot;` `#lt;` `#gt;` | raw `<…>` is HTML-sanitised away; raw `"` ends the string |
| `#` | `#35;` | literal `#quot;` in a label would otherwise decode to a quote |
| `` ` `` | `#96;` | a label (or edge label) **starting** with a backtick opens a markdown string and fails to lex |
| newline | `<br>` | rendered as a line break; raw newline would end the statement |
| `:` when the label contains `style` or `classDef` | `#58;` | Mermaid's `encodeEntities` runs `/style.*:\S*#.*;/` and `/classDef.*:\S*#.*;/` over the whole text and deletes the last `;`, turning `#quot;` into `#quot` [VERIFIED: mermaid@11.17.2 dist/chunks/mermaid.core/chunk-75Z2AOVW.mjs:521-538] |
| `%` when the label contains `%%` | `#37;` | `%%{` begins a directive and breaks the parse |
| U+0000–U+001F, U+007F, U+2028, U+2029 | `#<decimal>;` | defensive; Mermaid accepts raw controls but they corrupt SVG downstream |
| U+FB02 `ﬂ` and U+00B6 `¶` | `#64258;` / `#182;` | Mermaid's private entity placeholders are `ﬂ°`/`¶ß`; a literal pair would be decoded as `&`/`;` at render time |
| empty or whitespace-only label | `" "` (one space) | `n0[""]`, `subgraph s [""]` and `-->\|""\|` are parse errors; `[" "]` is accepted and yields an empty label |
| edge label `Some("")` | omit the label | `-->\|""\|` is a parse error and an empty label equals no label |

`|` needs no escape inside a quoted edge label (`-->|"a|b"|` parses and returns `a|b`). Mermaid trims label whitespace, so expect `label.trim()` on the way back. Mermaid also serialises some labels as HTML (`&amp;`, `&lt;`) and others raw (see Pitfall 6), so the oracle normaliser decodes entities in all cases.

### Sequence export
```
sequenceDiagram
  participant p0 as Alias text          # id kept iff safe and not a sequence keyword, else p<idx>
  p0->>p1: message          # dashed: p1-->>p0: …      empty text: `p0->>p1:` (no trailing space)
  Note over p0,p1: text     # one actor: Note over p0: text
```
Same encoder as graph labels **plus**: `;` → `#59;` (a raw `;` is a parse error in messages, notes and aliases), and text whose trimmed form matches `^:?(no)?wrap:` must have its first character numerically encoded (`#119;rap:`, `#110;owrap:`, `#58;wrap:`) because Mermaid strips that prefix and sets `wrap` [VERIFIED: mermaid@11.17.2 dist/chunks/mermaid.core/sequenceDiagram-WJ2MYXX4.mjs:1443-1444]. Raw `#` truncates the message (`a # b` → `a`), covered by `#35;`. Keep ids as opaque `p<idx>` by default: actor ids that are keywords (`end`) fail to parse. Message types in the DB: 0 solid, 1 dashed, 2 note.

### Mermaid size limits the exporter cannot fix
Mermaid's defaults reject more than **500 edges** (`Edge limit exceeded. 500 edges found, but the limit is 500`) while the DSL allows 1000; `maxTextSize` is 50,000 characters [VERIFIED: oracle probe at 500/501/1000 edges]. Document on `export`; the oracle must run with default config (do not raise limits in the helper).

### Reference implementation of the rules (JavaScript, executed; port to Rust `escape.rs`)
```js
// Passed: 20,000 random graph cases (labels, titles, edge labels), 6,000 sequence cases, 58 corpus round trips,
// on mermaid 11.17.2 and 12.2.0 (5,000 / 58 on 12.2.0).
export function encLabel(s, guard = /style|classDef/.test(s), pct = s.includes('%%')) {
  let out = '';
  for (const ch of s) {
    const cp = ch.codePointAt(0);
    if (ch === '&') out += '#amp;'; else if (ch === '"') out += '#quot;'; else if (ch === '<') out += '#lt;'; else if (ch === '>') out += '#gt;';
    else if (ch === '%' && pct) out += '#37;'; else if (ch === ':' && guard) out += '#58;';
    else if (ch === '`') out += '#96;'; else if (ch === '#') out += '#35;'; else if (ch === '\n') out += '<br>';
    else if (cp < 0x20 || cp === 0x7f || cp === 0x2028 || cp === 0x2029 || cp === 0xfb02 || cp === 0xb6) out += `#${cp};`;
    else out += ch;
  }
  return out.trim() === '' ? ' ' : out;
}
```

## Round-trip Design and Fixture Contract

Files (all committed; Rust golden tests own them; `UPDATE_GOLDEN=1` blesses):

| Path | Written by | Read by |
|---|---|---|
| `fixtures/corpus/NN-<slug>.mmd` | one-shot `#[ignore]` freezer (`fence::extract`), then committed | Rust importer tests, Node |
| `fixtures/corpus-roundtrip/NN-<slug>.vdg` | `import(..).source` | Rust (re-`check`), humans |
| `fixtures/corpus-roundtrip/NN-<slug>.export.mmd` | `export(check(source).ir)` / `export_sequence` | Node |
| `fixtures/corpus-roundtrip/NN-<slug>.sets.json` | Rust canonicaliser over the imported IR | Node (`R(B)`) |
| `fixtures/corpus-roundtrip/unsupported.json` | `[{file, unsupported:[{line,construct}]}]` for blocks 10 and 29 | Rust golden, Node (expectation lists) |
| `fixtures/export-hostile/NN-<name>.ir.json` + `.export.mmd` + `.sets.json` | curated hostile IRs (below) | Node |

Freeze with `fence::extract` and record in a `corpus/MANIFEST.json` the source path and open line of each block (plus HEAD sha in the commit message). Assert in `mermaid_corpus.rs` that the directory holds exactly 33 files. The freezer is the only code allowed to read `docs/`, is `#[ignore]`, and is a Rust test (not a `.sh`/`.py`), which satisfies "VoxScript-first" for glue.

Hostile export set to commit (every row failed a naive emitter or is named in SC1): ids `end End graph subgraph style class click default interpolate call href o x ox n0 n1(dup) a-b _` and duplicate local ids in different containers; labels: quotes, `#quot;`, `&amp;`, `<script>`, `<img onerror>`, `<br>`, backtick first/last/middle, `style: x #quot;` (colon guard), `%%{init}`, `100%`, `;`, `|`, brackets, `-->`, `---`, `end`, empty, whitespace-only, tab, `U+2028`, `ﬂ°`, `¶ß`, emoji/CJK, 200 characters, a label that tries statement injection (`"] ; click a call alert(1) ["`); all six shapes; empty container; 4-deep nesting; self-loop; sequence: `;`, `#`, `wrap:`, `:nowrap:`, empty text, `end` actor id, self-message, single-actor note.

Node-side canonicaliser inputs: `securityLevel: 'strict'`, default config, `mermaid.parse` for acceptance, `getDiagramFromText` for sets.

## Common Pitfalls

### Pitfall 1: Mermaid import without a DOM
**What goes wrong:** `TypeError: DOMPurify.addHook is not a function` at first parse [VERIFIED: executed on 11.17.2].
**How to avoid:** load the jsdom shim first, then `await import('mermaid')` (static import hoists above the shim). **Warning signs:** works in browser, fails in `node --test`.

### Pitfall 2: Reading label text straight from the DB
**What goes wrong:** the DB returns Mermaid's placeholder form (`ﬂ°quot¶ß`, `ﬂ°°96¶ß`), HTML-escaped text (`&amp;`, `&lt;`, `&nbsp;`) for some labels and raw text for others, and `<br>` (not `\n`) for breaks.
**How to avoid:** the helper decodes br-tags first, then placeholders, then HTML entities, then trims; unknown entity → throw.

### Pitfall 3: The `style…:#…;` semicolon eater
**What goes wrong:** a label such as `style: say #quot;hi#quot;` loses its final `;` and decodes wrong; in 4,000 fuzz cases 83 failed for this and `%%{` alone.
**How to avoid:** colon guard (`#58;`) per label containing `style`/`classDef`; rename ids containing those substrings.

### Pitfall 4: Backtick-first labels
**What goes wrong:** `A["\`x"]` is a markdown-string opener and is a lexical error. Real in the corpus (block 29).
**How to avoid:** encode every backtick (`#96;`) when exporting; reject the form when importing.

### Pitfall 5: `end`/`click` ids and silent drops
**What goes wrong:** most keywords error loudly; `click` as a subgraph id parses and **drops the subgraph**. The id list above is empirical, not documentation-derived.
**How to avoid:** keyword set + the oracle's container-count assertion on every hostile fixture (set equality catches silent loss).

### Pitfall 6: Comparing text byte-for-byte across parses
**What goes wrong:** the same label comes back `Docs Debt Burn-Down (920 -&gt; &lt;100)` from the original and `… -> <100)` after export.
**How to avoid:** compare only after the normaliser, on both sides.

### Pitfall 7: Mermaid's node-membership semantics
**What goes wrong:** a node mentioned in two subgraphs belongs only to the first-closed one; a node first used outside a subgraph and later inside belongs to the subgraph.
**How to avoid:** implement membership as "first-closed subgraph that mentions it"; keep container paths in the node key so a mistake fails MIG-02.

### Pitfall 8: Wrong subgraph-direction handling
**What goes wrong:** dropping `direction TB` inside subgraphs silently (corpus 06) violates "nothing dropped silently"; erroring would fail a corpus block.
**How to avoid:** accept only when equal to the diagram direction and list it in `dropped`.

### Pitfall 9: `Some("")` edge labels and empty node labels
Parse errors on the Mermaid side (see Text encoding). The DSL rejects empty labels (`EMPTY_LABEL` "A quoted label with no text", codes.rs) but hand-built IRs and Phase 86 can still carry them.

### Pitfall 10: CI never runs the Node half
`docs-quality.yml` triggers on `docs/**`, `docs-astro/**`, `crates/vox-doc-pipeline/**`, `crates/vox-cli/**`, `crates/vox-compiler/**`, `crates/vox-runtime/**`, `contracts/**`… **not** `crates/vox-diagram/**` [VERIFIED: docs-quality.yml:3-54]. A Rust-only exporter change would never run the oracle. Add `crates/vox-diagram/tests/fixtures/**` to both `on.push.paths` and `on.pull_request.paths`; `docs-astro/tests/unit/workflow-paths.test.mjs` only asserts coverage of mounted docs, so it will not object. Docs-quality is path-filtered, so it cannot be the required check; the Rust golden test is the hard gate that fixtures are current.

### Pitfall 11: Dependabot bumping the oracle
`/docs-astro` is on a weekly grouped npm entry [VERIFIED: .github/dependabot.yml:129-139]; an exact pin still gets bump PRs. A bump that changes parse behaviour will fail the oracle test, which is the desired signal; do not add an ignore unless it becomes noisy.

### Pitfall 12: tdd-guard / determinism-guard traps
The determinism guard fails any `src/**` file containing the hash-collection type names, and fails a file whose path **ends with `ir.rs`** if it contains the ordered-map type (determinism_guard.rs:22-43). `sequence_ir.rs` and `ir.rs` already end that way; name new files `export_seq.rs`, `seq.rs`, not `…_ir.rs`. New `pub fn` in files ≥30 non-blank lines need a same-file test (follow the `mod tests;` sibling-file pattern from Phase 83). Keep files < 400 non-blank lines.

### Pitfall 13: Do not trust the ids the DB returns for subgraphs
`getVertices()` lists a subgraph id only when it is used as an edge endpoint; subgraphs live in `getSubGraphs()`, whose `nodes` order is reversed and includes nested subgraph ids. Sort, and derive nesting from the lists.

## Code Examples

### Example 1: Node oracle helper (executed; `docs-astro/tests/lib/mermaid-oracle.mjs`)
```js
// Source: prototype run against mermaid@11.17.2 / 12.2.0 with jsdom@30.0.1, this session.
import { JSDOM } from 'jsdom';
const { window } = new JSDOM('<!doctype html><html><body></body></html>');
globalThis.window = window;
globalThis.document = window.document;
for (const k of ['Element', 'HTMLElement', 'SVGElement', 'Node', 'DocumentFragment', 'DOMParser', 'XMLSerializer']) {
  if (!(k in globalThis)) globalThis[k] = window[k];
}
if (!globalThis.navigator) Object.defineProperty(globalThis, 'navigator', { value: window.navigator, configurable: true });
const { default: mermaid } = await import('mermaid');          // must come after the shim
mermaid.initialize({ startOnLoad: false, securityLevel: 'strict' });

export async function accepts(src) { try { await mermaid.parse(src); return true; } catch { return false; } }

const NAMED = { quot: '"', lt: '<', gt: '>', amp: '&', apos: "'", nbsp: String.fromCharCode(0xa0) };
export function decodeLabel(s) {
  let t = s.replace(/<br\s*\/?>/gi, '\n');                                        // real br tags first
  t = t.replaceAll('ﬂ°°', '&#').replaceAll('ﬂ°', '&').replaceAll('¶ß', ';');      // Mermaid entity placeholders
  t = t.replace(/&(#x[0-9a-f]+|#[0-9]+|[a-z]+);/gi, (m, g) => {
    if (g[0] === '#') return String.fromCodePoint(g[1].toLowerCase() === 'x' ? parseInt(g.slice(2), 16) : parseInt(g.slice(1), 10));
    if (g in NAMED) return NAMED[g];
    throw new Error(`unknown entity ${m}`);
  });
  return t.trim();
}

const SHAPE = { square: 'rect', round: 'rounded', diamond: 'diamond', cylinder: 'cylinder', circle: 'start', doublecircle: 'end', undefined: 'rect', null: 'rect' };
const DIR = { TB: 'down', TD: 'down', LR: 'right', RL: 'left', BT: 'up' };

export async function canonFlow(src) {
  const d = await mermaid.mermaidAPI.getDiagramFromText(src);   // semi-internal API: the exact pin is the guard
  const db = d.db, parentOf = new Map(), titleOf = new Map();
  for (const s of db.getSubGraphs()) { titleOf.set(s.id, decodeLabel(s.title)); for (const n of s.nodes) parentOf.set(n, s.id); }
  const path = (id) => { const out = []; for (let c = id; c; c = parentOf.get(c)) out.unshift(titleOf.get(c)); return out; };
  const key = new Map();
  const nodes = [...db.getVertices().values()].filter((v) => !titleOf.has(v.id)).map((v) => {
    const k = JSON.stringify([decodeLabel(v.text), SHAPE[v.type], path(parentOf.get(v.id))]); key.set(v.id, k); return k;
  }).sort();
  const edges = db.getEdges().map((e) => JSON.stringify([key.get(e.start), key.get(e.end), decodeLabel(e.text || '')])).sort();
  return { direction: DIR[db.getDirection()], nodes, edges, containers: [...titleOf.keys()].map((id) => JSON.stringify(path(id))).sort() };
}
```
State: `db.getStates()` (ids `root_start`/`root_end` for `[*]`, description in `descriptions[0]`) and `db.getRelations()` (`{id1,id2,relationTitle}`) mapped into the same key shape with start/end labels blanked on both sides. Sequence: `db.getActors()` (`description` = alias) and `db.getMessages()` (`type` 0/1/2, notes carry `placement`).

### Example 2: Rust API sketch (`crates/vox-diagram/src/mermaid/mod.rs`)
```rust
/// Export a graph to Mermaid. Total over any `Ir`; see module docs for Mermaid's 500-edge default limit.
pub fn export(ir: &Ir) -> String;
pub fn export_sequence(ir: &SequenceIr) -> String;      // Phase 86 routes Ir::Sequence here

#[derive(Debug, Clone, Default)]
pub struct ImportOpts { pub drop_styling: bool }

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Unsupported { pub line: u32, pub construct: String }   // line is 1-based within the block

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ImportReport {
    pub source: Option<String>,          // canonical `fmt` output; graph/state only, None if unsupported non-empty
    pub sequence: Option<SequenceIr>,    // sequence blocks (no DSL until Phase 86) — see Open Question 3
    pub unsupported: Vec<Unsupported>,
    pub dropped: Vec<Unsupported>,       // drop_styling lines and redundant subgraph directions
}
pub fn import(block: &str, opts: ImportOpts) -> ImportReport;
```
Build the DSL text (or an `ast::Document`), then `fmt(&raw)`; a `None` from `fmt` is an importer bug, surface it as a single `unsupported` entry rather than panicking (`unwrap_used`/`expect_used`/`panic` are denied outside tests, lib.rs:#![cfg_attr(not(test), deny(...))]).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `mermaid.parse()` in Node | still needs a DOM shim for DOMPurify | persists in 11.17.2 and 12.2.0 | jsdom shim required; no pure-Node entry exists for flowchart/state/sequence |
| Mermaid 11.x | 12.0.0 released 2026-09-10 (12.2.0 on 2026-10-13) | 2026-09/10 | Same probe results; adopt only with the site loader (Phase 89 coordination) |
| unpinned `mermaid@11` CDN on the site | exact-pinned devDependency for the oracle | this phase | the oracle is reproducible; the live loader is still Phase 89's concern |

**Deprecated/outdated:** `STACK.md`'s "No new npm deps" was about the pre-render path and still holds for the production site; the oracle is test-only. `PITFALLS.md` Pitfall 8 recommended `mermaid.parse()` over the exporter corpus in `docs-astro` — confirmed and made concrete here.

## Runtime State Inventory

Not a rename/refactor/migration phase; omitted.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The site's `mermaid@11` CDN import resolves to 11.17.2 today (latest 11.x on npm) | Standard Stack | Low: only affects the "matches the site" rationale; oracle is exact-pinned either way |
| A2 | GitHub's bundled Mermaid behaves like 11.17.2/12.2.0 for the exporter's output | Exporter Rules | Medium: GitHub renders exports in PR bodies; version moves independently (PITFALLS #8). Not testable here |
| A3 | Node 24 on `ubuntu-latest` via `actions/setup-node@v7` `node-version: 24` resolves to ≥24.15 (jsdom 30 engine floor) | Standard Stack | Low: registry shows 24.22; a stale cached runner would fail install with an engines warning/error |
| A4 | `docs-quality` stays non-required (path-filtered), so the Rust golden test is the only hard gate | Pitfall 10 | Medium: if a required check should cover the oracle, a path-independent job is needed (not in scope) |
| A5 | Making `print::label` logic `pub(crate)` (or building an `ast::Document`) is acceptable to the Phase 83 file-size/arch rules | Don't Hand-Roll | Low |
| A6 | Phase 86 will add `Ir::Sequence` and its DSL printer and can consume `ImportReport.sequence` | Open Question 3 | Medium: if 86 prefers `Ir::Sequence` now, the field changes shape |

## Open Questions

1. **Corpus block 29 (`how-to/how-to-rust-crate-imports.md:53`) is invalid Mermaid.**
   - What we know: `A["\`.vox\` source"]` fails to lex on 11.17.2 and 12.2.0 (a quoted label beginning with a backtick starts a markdown string); the site's CDN renderer therefore cannot draw this diagram today. `["AST \`X\`"]` (backtick not first) is accepted.
   - What's unclear: whether SC3 should read "Gantt and the one block the pinned parser rejects are the only exceptions", and what Phase 90 does with block 29.
   - Recommendation: importer reports `unsupported` (`markdown-string`) for it; record block 29 as an expected oracle-reject fixture (Node asserts both the Rust unsupported entry and the oracle rejection, so a future fix upstream flags the expectation); Phase 90 repairs the source block (drop the leading backtick styling) before importing. **Needs user acknowledgement** because it amends ROADMAP SC3 and, if instead the block were allowlisted, MIG-04 says any allowlist entry beyond the Gantt needs sign-off.
2. **Container direction.** `IrContainer` has `id, local_id, label, parent, depth, span` and no direction (ir.rs:81-95), and the DSL rejects `direction` in a container (`DIRECTION_IN_CONTAINER`). The context's "container direction emitted only when set" therefore emits nothing today.
   - Recommendation: keep the export rule as a no-op comment in code; importer accepts a subgraph `direction` equal to the diagram direction (list in `dropped`), `unsupported` otherwise (corpus 06 has two equal ones).
3. **Where a sequence import lands.** Phase 84 has `SequenceIr` types but no sequence DSL and no `Ir::Sequence`; `ImportReport.source` cannot hold it.
   - Recommendation: add `sequence: Option<SequenceIr>` to `ImportReport` (this phase) and leave `Ir` untouched; Phase 86 prints it to DSL. Alternative: add `Ir::Sequence` now (touches Phase 86's declared "additively" line and every exhaustive match).
4. **Start/end labels for state imports.** DSL labels cannot be empty. Recommendation: `Start` and `End`, ids `start` and `end` (the exporter then exercises its `end`→`n<idx>` fallback on a real corpus block). Oracle comparison blanks terminal labels on both sides.
5. **Comments and semicolons.** `%%` lines and trailing `;` have no corpus use. Recommendation: `unsupported` (strict, YAGNI). Alternative: list `%%` lines in `dropped` as comments. Pick one and test it.
6. **Edge placement in the generated DSL** (deepest common container vs always root with full paths). Recommendation: deepest common container (readable migrated diffs in Phase 90); root-only is the smaller diff if time is tight. Sets are identical.
7. **`o`/`x` id lock.** Verified unnecessary with spaced edges; honoured because locked. Relaxing it keeps more readable ids in exports. User decision only.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Node.js | oracle tests | ✓ | local v26.11.1; CI pins 24 (registry latest 24.22.0) | — |
| pnpm | `docs-astro` install | ✓ | local 12.11.2; CI `version: 11` (verified with 11.28.6) | — |
| network / npm registry | `pnpm add` | ✓ | — | — |
| cargo + nextest (Rust half) | tests | ✓ (Phase 83 ran it) | per `rust-toolchain.toml` | plain `cargo test -p vox-diagram` |
| `vox` binary for `vox ci crate-api-sync` | regenerating `docs/src/api/vox-diagram.generated.md` after `pub mod mermaid` | stale at last Phase 83 run | rebuild `cargo build -p vox-cli` (~24 s) | — |
| Playwright / browsers | not needed | — | — | — |

**Missing dependencies with no fallback:** none. **Missing with fallback:** none.

## Validation Architecture

`workflow.nyquist_validation` is absent from `.planning/config.json` → enabled.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust: `cargo nextest` / `cargo test` with `insta` 1.48 + `proptest` (existing); Node: built-in `node --test` |
| Config file | none new; Node tests discovered by CI glob `tests/unit/*.test.mjs` |
| Quick run command | `cargo nextest run -p vox-diagram mermaid` and `cd docs-astro && node --test tests/unit/mermaid-oracle.test.mjs` |
| Full suite command | `cargo nextest run -p vox-diagram` and `cd docs-astro && node --test "tests/unit/*.test.mjs"` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| REND-06 | Hostile graph and sequence exports are accepted by the pinned parser and their canonical sets equal the IR-derived sets | oracle (Node) | `cd docs-astro && node --test tests/unit/mermaid-oracle.test.mjs` | ❌ Wave 0 |
| REND-06 | Exporter sanitizer invariants for arbitrary IRs (one statement per line, no raw `" < > & \` #` in labels, ids unique/safe/non-keyword, `end`/`click`/`o`/`x` ids renamed) | proptest + unit | `cargo nextest run -p vox-diagram --test mermaid_export` | ❌ Wave 0 |
| REND-06 | Export goldens match committed `.export.mmd` | golden | same test target; bless with `UPDATE_GOLDEN=1` | ❌ Wave 0 |
| MIG-01 | Every documented leniency imports (unquoted labels, `&`, chains, `<br>`) and yields the expected DSL | unit catalogue | `cargo nextest run -p vox-diagram --test mermaid_import` | ❌ Wave 0 |
| MIG-01 | Every unsupported construct fails closed with the exact `{line, construct}` list; `drop_styling` lists dropped lines; gantt and block 29 are `unsupported` with `source: None` | unit catalogue | same | ❌ Wave 0 |
| MIG-01 | Imported `source` is `fmt`-stable and `check`-valid for all 31 importable corpus blocks | integration | `cargo nextest run -p vox-diagram --test mermaid_corpus` | ❌ Wave 0 |
| MIG-02 | `canon(original) == sets.json` and `canon(export) == canon(original)` over the 31 blocks | oracle (Node) | `cd docs-astro && node --test tests/unit/mermaid-oracle.test.mjs` | ❌ Wave 0 |
| MIG-02 | Rust-only fixpoint: `import(export(ir)).ir ≅ ir` modulo ids for generated IRs | proptest | `cargo nextest run -p vox-diagram --test mermaid_export` | ❌ Wave 0 |
| MIG-02 | Corpus freeze is complete: exactly 33 `.mmd` fixtures, manifest rows match | unit | `cargo nextest run -p vox-diagram --test mermaid_corpus` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo check -p vox-diagram` plus the new test target for the task (lean local verification; CI is the gate). Node tasks: the single `node --test` file.
- **Per wave merge:** `cargo nextest run -p vox-diagram` + `cargo clippy -p vox-diagram --all-targets -- -D warnings` + `node --test "tests/unit/*.test.mjs"` in `docs-astro`.
- **Phase gate:** hosted CI green (`tests` shards, `docs-quality`), `RUSTDOCFLAGS='-D warnings' cargo doc -p vox-diagram --no-deps`, `vox ci crate-api-sync` clean.
- **Mutation proofs (repo rule, with before/after presence checks):** (a) drop the `"`→`#quot;` branch in `escape.rs` → both the Rust sanitizer test and the Node hostile test must fail; (b) delete the colon guard → the `style: …` hostile fixture must fail in Node; (c) make the importer skip an `unsupported` push for `classDef` → the catalogue test must fail; (d) corrupt the dotted-path rewrite → MIG-02 container-path keys must fail.

### Wave 0 Gaps
- [ ] `docs-astro/package.json` + `pnpm-lock.yaml`: `mermaid@11.17.2`, `jsdom@30.0.1` (after the human-verify checkpoint)
- [ ] `docs-astro/tests/lib/mermaid-oracle.mjs`, `docs-astro/tests/unit/mermaid-oracle.test.mjs`
- [ ] `.github/workflows/docs-quality.yml`: add `crates/vox-diagram/tests/fixtures/**` to `push` and `pull_request` paths
- [ ] `crates/vox-diagram/tests/{mermaid_export,mermaid_import,mermaid_corpus,freeze_corpus}.rs` and `tests/fixtures/{corpus,corpus-roundtrip,export-hostile}/`
- [ ] Golden helper (`UPDATE_GOLDEN=1`) in `tests/support/`

## Security Domain

`security_enforcement` is not set in config → enabled.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | yes | importer is a parser of untrusted text: size gate first (`MAX_SOURCE_BYTES`), O(n) line scanner, caps, no recursion on input depth, `deny(unwrap_used, expect_used, panic)`, adversarial test (long lines, 10k edges, nested `subgraph`, `&`/chain blow-up cap) bounded in time |
| V6 Cryptography | no | no hashing needed (sequence-index ids only; do not add `blake3`/`sha2`) |
| V10/V12 output encoding | yes | one escaping module for all Mermaid output; structural line-count invariant proves no statement injection |

### Known Threat Patterns for this stack
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Label breaks out of its string and injects Mermaid statements (`click … call`, `style …`, `href`) | Tampering | Always-quoted labels, `"`→`#quot;`, newline→`<br>`, statement-count property test; the exporter never emits `click`/`style`/`href` |
| Entity smuggling (`#quot;` literal, `ﬂ°` placeholders) | Tampering | Encode `#`, U+FB02, U+00B6 |
| Mermaid-side CSS/markup injection from diagram-supplied ids/classes (CVE-2026-50159, fixed 11.16.1; CVE-2026-41159, fixed 11.15.0 [CITED: PITFALLS.md §12, mend.io]) | Tampering / EoP | Oracle pinned ≥ 11.16.1; exporter never emits user text as class names or ids that are not `[A-Za-z0-9_]` |
| Importer DoS (quadratic scans, expansion blow-up) | DoS | Phase 83 already found a quadratic lexer scan with an adversarial test; give the importer the same 10 s-bounded adversarial suite and the 100-edge expansion cap |
| Supply chain via new npm packages (`mermaid`, `jsdom` and ~110 transitives) | Tampering | Exact pins, committed lockfile, `--frozen-lockfile`, pnpm 11 build-script block (`allowBuilds` unchanged, none needed), test-only (never imported by `astro build`), jsdom created without `runScripts` or network resources, `securityLevel: 'strict'`; `deploy-cloudflare` installs docs deps but the Cloudflare token is only in the final step's `env` |
| Reading tombstoned content | Policy | The freezer skips `docs/src/archive/`; tests never read `docs/` |

## Project Constraints (from CLAUDE.md)

- Test-first: every new `pub fn` in a Rust file of ≥30 non-blank lines needs a same-file test (`mod tests;` sibling pattern is the Phase 83 precedent); failing test first.
- No new crate dependency edges; no new Rust dependencies are needed (do not touch `crate-edges.allow.v1.json`, `fan-in-snapshot.v1.json`, `Cargo.toml`/`Cargo.lock`).
- Files < 400 non-blank lines; no `HashMap`/`HashSet` in `src/`; byte-deterministic output; no `blake3`/`sha2` imports.
- `cargo` through the build broker (plain `cargo` on PATH); locally run only `cargo check -p vox-diagram` and the new test targets; hosted CI is the gate (no workspace clippy locally); background long builds.
- VoxScript-first: no new `.sh`/`.py`/`.ps1` glue. The corpus freezer is a Rust `#[ignore]` test. Node tests follow the existing `.mjs` convention in `docs-astro/tests` (the TypeScript-only rule does not apply; biome only lints `docs-astro/src/**`).
- Docs: `docs/src/api/vox-diagram.generated.md` is generated (`vox ci crate-api-sync --write`) when `pub mod mermaid` lands; add a `where-things-live.md` concept row for Mermaid export/import; any new `docs/src` page needs the full frontmatter. Never edit `docs/src/archive/`.
- Hosted-CI discipline: batch commits, draft PR; do not push to `main`.
- Do not use the uncommitted `mermaid.rs` validator in the other checkout; do not read `docs/src/archive/`.

## Suggested Plan Decomposition (for the planner)

1. **Wave 1 (parallel):** (a) `checkpoint:human-verify` then add `mermaid`/`jsdom` pins, lockfile, oracle helper + test skeleton, `docs-quality.yml` path edit; (b) `mermaid/escape.rs` + graph `export` tracer with sanitizer unit/proptest and the hostile fixtures; (c) corpus freezer + 33 frozen fixtures + manifest.
2. **Wave 2:** sequence export; importer flowchart (lines, scope rewrite, unsupported catalogue, `drop_styling`); importer state; importer sequence (`SequenceIr`).
3. **Wave 3:** round-trip goldens (`.vdg`, `.export.mmd`, `.sets.json`, `unsupported.json`) and the three-way Node assertions; mutation proofs; governance (`crate-api-sync`, `where-things-live.md`, rustdoc, clippy for the touched crate).

## Sources

### Primary (HIGH confidence)
- Executed this session in `/private/tmp/claude-501/-Users-brbrainerd-dev-vox/1578e102-b2d3-4922-b18e-a965ec6b6147/scratchpad` (never in the repo): `mermaid@11.17.2` and `@12.2.0` with `jsdom@30.0.1`/`30.1.3`: public `parse()`, `mermaidAPI.getDiagramFromText`, the 33-block corpus, id/keyword probes, label probes (46 hand cases, 20k graph fuzz, 6k sequence fuzz), 58 corpus round trips with a JS mimic of the emitter, state round trip, 500/501/1000-edge limit probe.
- `node_modules/mermaid/dist/chunks/mermaid.core/chunk-75Z2AOVW.mjs:521-538` (`encodeEntities`) and `sequenceDiagram-WJ2MYXX4.mjs:1443-1444` (`wrap:` stripping), read in full.
- Repo files read this session: `crates/vox-diagram/src/{ir,sequence_ir,fence,check,print,resolve,limits,codes,lib,span,ast}.rs`, `lexer/mermaid.rs:1-40`, `tests/determinism_guard.rs`, `grammar-card.txt`, `.github/workflows/{docs-quality,docs-deploy}.yml`, `.github/dependabot.yml`, `docs-astro/{package.json,pnpm-workspace.yaml}`, `docs-astro/tests/unit/workflow-paths.test.mjs`, `deny.toml`, `biome.json`, `docs/src/ci/affected-crate-selective-ci.md`, Phase 83 summary/verification, milestone research.
- npm registry (`npm view mermaid|jsdom|node`), `pnpm licenses list`, `gsd_run query package-legitimacy check`.

### Secondary (MEDIUM confidence)
- Mend vulnerability database entry for CVE-2026-50159 (fixed in 10.9.8 and 11.16.1; a search summary, not the advisory text): https://www.mend.io/vulnerability-database/CVE-2026-50159

### Tertiary (LOW confidence)
- GitHub's embedded Mermaid version (unverified; A2).

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — installed, executed, frozen-lockfile verified with the CI pnpm major.
- Exporter rules: HIGH — each rule traces to a failing case; 20k+6k fuzz pass on two Mermaid majors. Residual: the rules are validated through a JS mimic, so the Rust port needs its own property test (specified).
- Importer rules: MEDIUM-HIGH — label/shape/id strictness and membership semantics measured; the exact `unsupported` naming and the open decisions are discretionary.
- Pitfalls/CI wiring: HIGH — read from the workflow files.

**Research date:** 2026-10-16
**Valid until:** 2026-11-15 (Mermaid ships minors every 1–3 weeks; re-run the hostile probes if the pin moves)
